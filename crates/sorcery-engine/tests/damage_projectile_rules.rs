use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

struct ProjectileSetup {
    far_target_id: String,
    near_target_id: Option<String>,
    session: Session,
    shooter_id: String,
}

fn avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    })
}

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn double_mask() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": 0,
        "nearbyStrikesAgainstUnitsDealDoubleDamage": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn manifest(seed: u32, shooter: &Value, near: &Value, far: &Value) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-shooter": shooter,
        "north-site": site(),
        "south-avatar": avatar(),
        "south-far": far,
        "south-filler": minion(json!({})),
        "south-near": near,
        "south-site": site(),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "damage-projectile-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-damage-projectile-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-shooter"; 3],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": ["south-near", "south-far", "south-filler"],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine-issued action must be accepted");
    };
    (descriptor, receipt)
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn unit<'a>(position: &'a Value, instance_id: &str) -> &'a Value {
    position["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected unit")
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn projectile_amount(receipt: &Receipt, target_id: &str) -> i64 {
    receipt
        .events
        .iter()
        .find(|event| {
            event.event_type == "projectile-damage-allocated"
                && event.payload["targetInstanceId"] == target_id
        })
        .expect("projectile allocation")
        .payload["amount"]
        .as_i64()
        .expect("projectile amount")
}

fn unit_id(session: &Session, card_id: &str) -> String {
    state(session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("expected unit")["instanceId"]
        .as_str()
        .expect("unit identity")
        .to_owned()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("projectile checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn projectile_actions(session: &Session, shooter_id: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("projectile actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "shoot-damage-projectile"
                && action.descriptor["shooterInstanceId"] == shooter_id
        })
        .map(|action| action.descriptor)
        .collect()
}

fn prepare_projectile(
    seed: u32,
    shooter: &Value,
    near: &Value,
    far: &Value,
    summon_near: bool,
) -> ProjectileSetup {
    let encoded = manifest(seed, shooter, near, far);
    let mut session = Session::new(&encoded).expect("valid fixed-damage projectile scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-shooter"
            && descriptor["cell"] == "C4"
    });
    let shooter_id = summon["cardInstanceId"]
        .as_str()
        .expect("shooter identity")
        .to_owned();
    assert!(projectile_actions(&session, &shooter_id).is_empty());
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C2"
    });
    let far_target_id = summon["cardInstanceId"]
        .as_str()
        .expect("far target identity")
        .to_owned();
    let near_target_id = summon_near.then(|| {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-near"
                && descriptor["cell"] == "C3"
        });
        summon["cardInstanceId"]
            .as_str()
            .expect("near target identity")
            .to_owned()
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    ProjectileSetup {
        far_target_id,
        near_target_id,
        session,
        shooter_id,
    }
}

fn fire_south(session: &mut Session, shooter_id: &str, target_id: &str) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "shoot-damage-projectile"
            && descriptor["direction"] == "south"
            && descriptor["shooterInstanceId"] == shooter_id
            && descriptor["hit"]["instanceId"] == target_id
    })
    .1
}

/// Shooter on C4 with empty intermediate cells so the south ray reaches the enemy Avatar on C1.
fn prepare_long_range_projectile(seed: u32) -> (Session, String, String) {
    let encoded = manifest(
        seed,
        &minion(json!({ "tapToShootProjectileDamage": 3 })),
        &minion(json!({})),
        &minion(json!({})),
    );
    let mut session = Session::new(&encoded).expect("valid long-range fixed projectile scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-shooter"
            && descriptor["cell"] == "C4"
    });
    let shooter_id = summon["cardInstanceId"]
        .as_str()
        .expect("shooter identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let south_avatar_id = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    (session, shooter_id, south_avatar_id)
}

#[test]
fn rule_catalog_0781_ready_minion_taps_to_shoot_fixed_damage_at_first_visible_unit() {
    let setup = prepare_projectile(
        82,
        &minion(json!({
            "attack": 1,
            "stealth": true,
            "tapToShootProjectileDamage": 4,
            "ward": true,
        })),
        &minion(json!({ "stealth": true, "summonToAnySite": true })),
        // The 1/5 shooter has power 3; its fixed 4 damage must not set source power.
        &minion(json!({
            "preventsDamageFromUnitsWithPowerAtLeast": 4,
            "summonToAnySite": true,
        })),
        true,
    );
    let near_target_id = setup.near_target_id.as_ref().expect("near Stealth target");
    let actions = projectile_actions(&setup.session, &setup.shooter_id);
    assert_eq!(
        actions
            .iter()
            .map(|descriptor| descriptor["direction"].clone())
            .collect::<Vec<_>>(),
        ["east", "north", "south", "west"]
    );
    assert_eq!(actions[0]["hit"], Value::Null);
    assert_eq!(actions[1]["hit"], Value::Null);
    assert_eq!(actions[3]["hit"], Value::Null);
    assert_eq!(actions[2]["hit"]["instanceId"], setup.far_target_id);
    assert_ne!(actions[2]["hit"]["instanceId"], *near_target_id);
    assert_eq!(
        actions[2]["path"]
            .as_array()
            .expect("south ray")
            .iter()
            .map(|location| location["cell"].as_str().expect("ray cell"))
            .collect::<Vec<_>>(),
        ["C4", "C3", "C2"]
    );
    assert_checkpoint_round_trip(&setup.session);

    let mut missed = setup.session.clone();
    let (_, miss) = accept_where(&mut missed, |descriptor| {
        descriptor["kind"] == "shoot-damage-projectile"
            && descriptor["direction"] == "east"
            && descriptor["hit"].is_null()
    });
    assert_eq!(event_types(&miss), ["projectile-shot", "stealth-lost"]);
    assert_eq!(
        miss.events[0].payload["path"],
        json!([{ "cell": "C4", "region": "surface" }])
    );
    let missed_state = state(&missed);
    let missed_shooter = unit(&missed_state, &setup.shooter_id);
    assert_eq!(missed_shooter["tapped"], true);
    assert_eq!(missed_shooter["stealthed"], false);
    assert_eq!(missed_shooter["warded"], true);
    assert!(projectile_actions(&missed, &setup.shooter_id).is_empty());
    assert_exact_replay(&missed);

    let mut hit = setup.session;
    let receipt = fire_south(&mut hit, &setup.shooter_id, &setup.far_target_id);
    assert_eq!(
        event_types(&receipt),
        [
            "projectile-shot",
            "stealth-lost",
            "projectile-damage-allocated",
            "damage-dealt",
        ]
    );
    assert_eq!(
        receipt.events[2].payload,
        json!({
            "amount": 4,
            "sourceInstanceId": setup.shooter_id,
            "targetInstanceId": setup.far_target_id,
        })
    );
    let hit_state = state(&hit);
    assert_eq!(unit(&hit_state, &setup.far_target_id)["damage"], 4);
    let hit_shooter = unit(&hit_state, &setup.shooter_id);
    assert_eq!(hit_shooter["tapped"], true);
    assert_eq!(hit_shooter["stealthed"], false);
    assert_eq!(hit_shooter["warded"], true);
    assert!(projectile_actions(&hit, &setup.shooter_id).is_empty());
    assert_exact_replay(&hit);
}

#[test]
fn rule_catalog_0746_disabled_stealth_is_visible_but_disabled_shooter_cannot_fire() {
    let visible = prepare_projectile(
        83,
        &minion(json!({ "tapToShootProjectileDamage": 1 })),
        &minion(json!({
            "stealth": true,
            "summonToAnySite": true,
            "waterbound": true,
        })),
        &minion(json!({ "summonToAnySite": true })),
        true,
    );
    let near_target_id = visible
        .near_target_id
        .as_ref()
        .expect("disabled Stealth target");
    let south = projectile_actions(&visible.session, &visible.shooter_id)
        .into_iter()
        .find(|descriptor| descriptor["direction"] == "south")
        .expect("south projectile");
    assert_eq!(south["hit"]["instanceId"], *near_target_id);
    assert_eq!(
        south["path"]
            .as_array()
            .expect("short ray")
            .iter()
            .map(|location| location["cell"].as_str().expect("ray cell"))
            .collect::<Vec<_>>(),
        ["C4", "C3"]
    );
    let before = state(&visible.session);
    assert_eq!(unit(&before, near_target_id)["stealthed"], false);
    let mut fired = visible.session;
    fire_south(&mut fired, &visible.shooter_id, near_target_id);
    assert_eq!(unit(&state(&fired), near_target_id)["damage"], 1);
    assert_exact_replay(&fired);

    let disabled = prepare_projectile(
        84,
        &minion(json!({
            "genesisDisableSelfUntilDamaged": true,
            "tapToShootProjectileDamage": 1,
        })),
        &minion(json!({})),
        &minion(json!({ "summonToAnySite": true })),
        false,
    );
    assert_eq!(
        unit(&state(&disabled.session), &disabled.shooter_id)["disabledUntilDamaged"],
        true
    );
    assert!(projectile_actions(&disabled.session, &disabled.shooter_id).is_empty());
    assert_exact_replay(&disabled.session);
}

#[test]
fn rule_catalog_0747_fixed_projectile_uses_current_power_for_prevention_threshold() {
    let setup = prepare_projectile(
        85,
        &minion(json!({ "attack": 4, "tapToShootProjectileDamage": 1 })),
        &minion(json!({})),
        &minion(json!({
            "preventsDamageFromUnitsWithPowerAtLeast": 3,
            "summonToAnySite": true,
        })),
        false,
    );
    let mut session = setup.session;
    let receipt = fire_south(&mut session, &setup.shooter_id, &setup.far_target_id);
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(
        receipt.events[2].payload,
        json!({
            "accumulated": 0,
            "amount": 0,
            "attemptedAmount": 1,
            "direct": true,
            "instanceId": setup.far_target_id,
            "prevented": true,
            "seat": "south",
        })
    );
    assert_eq!(unit(&state(&session), &setup.far_target_id)["damage"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0748_fixed_projectile_applies_ward_lethal_and_ordinary_death() {
    let warded = prepare_projectile(
        86,
        &minion(json!({ "tapToShootProjectileDamage": 4 })),
        &minion(json!({})),
        &minion(json!({ "defense": 1, "summonToAnySite": true, "ward": true })),
        false,
    );
    let mut warded_session = warded.session;
    let warded_receipt = fire_south(
        &mut warded_session,
        &warded.shooter_id,
        &warded.far_target_id,
    );
    assert_eq!(
        event_types(&warded_receipt),
        [
            "projectile-shot",
            "projectile-damage-allocated",
            "damage-dealt",
            "ward-broken",
        ]
    );
    let warded_state = state(&warded_session);
    assert_eq!(unit(&warded_state, &warded.far_target_id)["damage"], 0);
    assert_eq!(unit(&warded_state, &warded.far_target_id)["warded"], false);
    assert_exact_replay(&warded_session);

    for (seed, shooter, target) in [
        (
            87,
            minion(json!({ "tapToShootProjectileDamage": 4 })),
            minion(json!({ "defense": 4, "summonToAnySite": true })),
        ),
        (
            88,
            minion(json!({ "lethal": true, "tapToShootProjectileDamage": 1 })),
            minion(json!({ "defense": 10, "summonToAnySite": true })),
        ),
    ] {
        let dead = prepare_projectile(seed, &shooter, &minion(json!({})), &target, false);
        let mut dead_session = dead.session;
        let receipt = fire_south(&mut dead_session, &dead.shooter_id, &dead.far_target_id);
        assert_eq!(
            event_types(&receipt),
            [
                "projectile-shot",
                "projectile-damage-allocated",
                "damage-dealt",
                "minion-died",
            ]
        );
        assert!(
            state(&dead_session)["players"]["south"]["cemetery"]
                .as_array()
                .expect("South cemetery")
                .iter()
                .any(|card| card["instanceId"] == dead.far_target_id)
        );
        assert_exact_replay(&dead_session);
    }
}

#[test]
fn rule_catalog_0911_fixed_projectile_ray_extends_past_ranged_two_step_cap() {
    let (session, shooter_id, south_avatar_id) = prepare_long_range_projectile(89);
    let south = projectile_actions(&session, &shooter_id)
        .into_iter()
        .find(|descriptor| descriptor["direction"] == "south")
        .expect("south fixed-damage ray");
    assert_eq!(south["hit"]["instanceId"], south_avatar_id);
    assert_eq!(south["hit"]["kind"], "avatar");
    assert_eq!(south["hit"]["seat"], "south");
    assert_eq!(
        south["path"]
            .as_array()
            .expect("long south ray")
            .iter()
            .map(|location| location["cell"].as_str().expect("ray cell"))
            .collect::<Vec<_>>(),
        ["C4", "C3", "C2", "C1"],
        "fixed projectiles are not capped at Ranged's two measured steps"
    );

    let mut session = session;
    let receipt = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-damage-projectile"
            && descriptor["direction"] == "south"
            && descriptor["shooterInstanceId"] == shooter_id.as_str()
            && descriptor["hit"]["instanceId"] == south_avatar_id.as_str()
    })
    .1;
    assert_eq!(
        event_types(&receipt),
        [
            "projectile-shot",
            "projectile-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
        ]
    );
    assert_eq!(receipt.events[1].payload["amount"], 3);
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 17);
    assert_exact_replay(&session);
}

fn mask_fixed_projectile_manifest() -> String {
    (1..=4096)
        .map(|seed| {
            let cards = json!({
                "north-avatar": avatar(),
                "north-shooter": minion(json!({ "tapToShootProjectileDamage": 1 })),
                "north-site": site(),
                "south-avatar": avatar(),
                "south-mask": double_mask(),
                "south-minion": minion(json!({ "defense": 2, "summonToAnySite": true })),
                "south-site": site(),
            });
            let mut value = json!({
                "authority": {
                    "contentHash": identity_hash(&json!({
                        "fixture": "synthetic-nearby-fixed-projectile-double-strike-v1"
                    }))
                    .expect("synthetic authority identity"),
                    "mode": "synthetic",
                    "revisionId": "synthetic-nearby-fixed-projectile-double-strike-v1",
                },
                "cards": cards,
                "decks": {
                    "north": {
                        "atlas": vec!["north-site"; 6],
                        "avatar": "north-avatar",
                        "spellbook": vec!["north-shooter"; 6],
                    },
                    "south": {
                        "atlas": vec!["south-site"; 6],
                        "avatar": "south-avatar",
                        "spellbook": [
                            "south-mask",
                            "south-minion",
                            "south-minion",
                            "south-mask",
                            "south-minion",
                            "south-minion",
                        ],
                    },
                },
                "engineVersion": "sorcery-core-v1",
                "firstSeat": "north",
                "schemaVersion": 1,
                "seed": seed,
            });
            value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
            canonical_json(&value).expect("canonical synthetic manifest")
        })
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("opening candidate"));
            let hand = opening["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("south opening spellbook");
            hand.iter().any(|card| card["cardId"] == "south-mask")
                && hand.iter().any(|card| card["cardId"] == "south-minion")
        })
        .expect("bounded seed opening with a Mask and a south minion")
}

fn cast_south_mask(session: &mut Session, carry_on_minion: bool) {
    if carry_on_minion {
        let bearer_id = unit_id(session, "south-minion");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "south-mask"
                && descriptor["bearer"]["instanceId"] == bearer_id
        });
    } else {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "south-mask"
                && descriptor["cell"] == "C1"
                && descriptor["bearer"].is_null()
        });
    }
}

fn after_fixed_projectile_mask_ready(carry_mask: bool) -> Session {
    let mut session = Session::new(&mask_fixed_projectile_manifest())
        .expect("valid fixed-projectile mask session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-shooter"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    });
    cast_south_mask(&mut session, carry_mask);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    session
}

#[test]
fn fixed_damage_projectile_is_not_a_strike_even_near_a_strike_doubler() {
    for carry_mask in [true, false] {
        let mut session = after_fixed_projectile_mask_ready(carry_mask);
        let shooter_id = unit_id(&session, "north-shooter");
        let target_id = unit_id(&session, "south-minion");
        let receipt = fire_south(&mut session, &shooter_id, &target_id);
        assert_eq!(projectile_amount(&receipt, &target_id), 1);
        let after = state(&session);
        let target = after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == target_id)
            .expect("fixed damage leaves the 2-defense minion alive");
        assert_eq!(target["damage"], 1);
        assert_exact_replay(&session);
    }
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

#[test]
fn rule_catalog_0990_fixed_projectile_deathrite_draws_for_minion_controller_on_kill() {
    let setup = prepare_projectile(
        90,
        &minion(json!({ "tapToShootProjectileDamage": 4 })),
        &minion(json!({})),
        &minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
        false,
    );
    let before = state(&setup.session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let mut session = setup.session;
    let receipt = fire_south(&mut session, &setup.shooter_id, &setup.far_target_id);
    assert_eq!(
        event_types(&receipt),
        [
            "projectile-shot",
            "projectile-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
        ]
    );
    assert_eq!(
        receipt.events[1].payload["targetInstanceId"],
        setup.far_target_id
    );
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], setup.far_target_id);
    let finished = state(&session);
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == setup.far_target_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}

fn try_accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
}

fn rain() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_shoot_damage_manifest(seed: u32) -> String {
    let fixture = "damage-shoot-deathrite-withheld";
    let cards = json!({
        "north-avatar": avatar(),
        "north-rain": rain(),
        "north-shooter": minion(json!({ "tapToShootProjectileDamage": 3 })),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-minion": minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
        "south-site": site(),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-shooter",
                    "north-rain",
                    "north-rain",
                    "north-shooter",
                    "north-rain",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn north_has_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
}

struct PendingDeathriteShootDamageSetup {
    deathrite_ids: [String; 2],
    session: Session,
    shooter_id: String,
}

fn try_pending_deathrite_with_ready_damage_shooter(
    encoded: &str,
) -> Option<PendingDeathriteShootDamageSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let shooter = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-shooter"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let shooter_id = shooter.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if projectile_actions(&session, &shooter_id).is_empty() {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteShootDamageSetup {
        deathrite_ids,
        session,
        shooter_id,
    })
}

fn deathrite_shoot_damage_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_shoot_damage_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_damage_shooter(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with a ready fixed-damage projectile minion on the board",
        )
}

#[test]
fn rule_catalog_1134_shoot_damage_projectile_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_shoot_damage_seed_with(1134);
    let mut setup = try_pending_deathrite_with_ready_damage_shooter(&encoded)
        .expect("complete shoot-damage-projectile Deathrite withheld setup");
    let shooter_id = setup.shooter_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert_eq!(unit(&paused, &shooter_id)["cardId"], "north-shooter");
    assert!(projectile_actions(session, &shooter_id).is_empty());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "shoot-damage-projectile")
    );

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);
    assert_checkpoint_round_trip(session);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(unit(&resumed, &shooter_id)["cardId"], "north-shooter");
    assert!(!projectile_actions(session, &shooter_id).is_empty());

    let (_, shot) = accept_where(session, |descriptor| {
        descriptor["kind"] == "shoot-damage-projectile"
            && descriptor["shooterInstanceId"] == shooter_id
    });
    assert_eq!(event_types(&shot)[0], "projectile-shot");
    assert_exact_replay(session);
}
