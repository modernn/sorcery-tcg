use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::game::GameError;
use sorcery_engine::session::{Session, SessionError, StepResult};

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

#[test]
fn ready_minion_should_tap_to_shoot_fixed_damage_at_first_visible_unit() {
    let setup = prepare_projectile(
        82,
        &minion(json!({
            "attack": 1,
            "stealth": true,
            "tapToShootProjectileDamage": 4,
            "ward": true,
        })),
        &minion(json!({ "stealth": true, "summonToAnySite": true })),
        &minion(json!({
            "preventsDamageFromUnitsWithPowerAtLeast": 3,
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
fn disabled_stealth_should_be_visible_but_disabled_shooter_cannot_fire() {
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
    assert_eq!(unit(&before, near_target_id)["stealthed"], true);
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
fn current_power_should_classify_fixed_projectile_damage_for_prevention() {
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
fn fixed_projectile_should_apply_ward_lethal_and_ordinary_death() {
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
fn non_surface_projectile_manifest_should_fail_closed() {
    let encoded = manifest(
        89,
        &minion(json!({
            "burrowing": true,
            "mustBeCastBurrowed": true,
            "tapToShootProjectileDamage": 1,
        })),
        &minion(json!({})),
        &minion(json!({})),
    );
    let error = Session::new(&encoded).expect_err("non-surface positions remain unsupported");
    match error {
        SessionError::Game(GameError::UnsupportedManifestFact(field)) => {
            assert_eq!(field, "mustBeCastBurrowed");
        }
        other => panic!("expected unsupported cast region, received {other}"),
    }
}
