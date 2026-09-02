use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

struct RangedSetup {
    far_target_id: Option<String>,
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

fn site(extra: Value) -> Value {
    let mut value = json!({ "cardType": "site", "elements": ["earth"] });
    let Value::Object(extra) = extra else {
        panic!("extra site facts must be an object");
    };
    value.as_object_mut().expect("site facts").extend(extra);
    value
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

fn manifest(
    seed: u32,
    shooter: &Value,
    near: &Value,
    far: &Value,
    north_site: &Value,
    south_atlas_len: usize,
) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-shooter": shooter,
        "north-site": north_site,
        "south-avatar": avatar(),
        "south-far": far,
        "south-filler": minion(json!({})),
        "south-near": near,
        "south-site": site(json!({})),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ranged-projectile-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ranged-projectile-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-shooter"; 3],
            },
            "south": {
                "atlas": vec!["south-site"; south_atlas_len],
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

fn power_loss_manifest(
    seed: u32,
    fixture: &str,
    cards: &Value,
    north_spellbook: [&str; 3],
    south_spellbook: [&str; 3],
    north_atlas_len: usize,
) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": fixture,
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; north_atlas_len],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 5],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
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
    let checkpoint = create_game_checkpoint(session).expect("Ranged checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn ranged_actions(session: &Session, shooter_id: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("Ranged actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "shoot-projectile"
                && action.descriptor["shooterInstanceId"] == shooter_id
        })
        .map(|action| action.descriptor)
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "the public scenario dimensions stay explicit at each direct proof call site"
)]
fn prepare_ranged(
    seed: u32,
    shooter: &Value,
    near: &Value,
    far: &Value,
    north_site: &Value,
    near_cell: Option<&str>,
    far_cell: Option<&str>,
    south_atlas_len: usize,
) -> RangedSetup {
    let encoded = manifest(seed, shooter, near, far, north_site, south_atlas_len);
    let mut session = Session::new(&encoded).expect("valid ordinary Ranged scenario");
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
    assert!(ranged_actions(&session, &shooter_id).is_empty());
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
    let far_target_id = far_cell.map(|cell| {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-far"
                && descriptor["cell"] == cell
        });
        summon["cardInstanceId"]
            .as_str()
            .expect("far target identity")
            .to_owned()
    });
    let near_target_id = near_cell.map(|cell| {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-near"
                && descriptor["cell"] == cell
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
    RangedSetup {
        far_target_id,
        near_target_id,
        session,
        shooter_id,
    }
}

fn fire_south(session: &mut Session, shooter_id: &str, target_id: &str) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["direction"] == "south"
            && descriptor["shooterInstanceId"] == shooter_id
            && descriptor["hit"]["instanceId"] == target_id
    })
    .1
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct ray proof compares default, bonus, stacked, and mixed action ordering"
)]
fn ranged_rays_should_use_site_range_skip_stealth_and_stop_at_first_visible_stack() {
    let ranged_and_fixed = minion(json!({
        "ranged": true,
        "tapToShootProjectileDamage": 1,
    }));
    let stealth = minion(json!({ "stealth": true, "summonToAnySite": true }));
    let visible = minion(json!({ "summonToAnySite": true }));

    let default_range = prepare_ranged(
        181,
        &ranged_and_fixed,
        &stealth,
        &visible,
        &site(json!({})),
        Some("C3"),
        Some("C2"),
        6,
    );
    let actions = ranged_actions(&default_range.session, &default_range.shooter_id);
    assert_eq!(
        actions
            .iter()
            .map(|descriptor| descriptor["direction"].clone())
            .collect::<Vec<_>>(),
        ["east", "north", "south", "west"]
    );
    assert!(actions.iter().all(|descriptor| descriptor["hit"].is_null()));
    assert_eq!(
        actions[2]["path"],
        json!([
            { "cell": "C4", "region": "surface" },
            { "cell": "C3", "region": "surface" },
        ])
    );

    let bonus_range = prepare_ranged(
        182,
        &ranged_and_fixed,
        &stealth,
        &visible,
        &site(json!({ "rangedUnitsHereRangeBonus": 1 })),
        Some("C3"),
        Some("C2"),
        6,
    );
    let far_id = bonus_range
        .far_target_id
        .as_ref()
        .expect("far visible target");
    let south = ranged_actions(&bonus_range.session, &bonus_range.shooter_id)
        .into_iter()
        .find(|descriptor| descriptor["direction"] == "south")
        .expect("south Ranged ray");
    assert_eq!(south["hit"]["instanceId"], *far_id);
    assert_eq!(
        south["path"],
        json!([
            { "cell": "C4", "region": "surface" },
            { "cell": "C3", "region": "surface" },
            { "cell": "C2", "region": "surface" },
        ])
    );
    assert_checkpoint_round_trip(&bonus_range.session);

    let stacked = prepare_ranged(
        183,
        &ranged_and_fixed,
        &visible,
        &visible,
        &site(json!({ "rangedUnitsHereRangeBonus": 1 })),
        Some("C3"),
        Some("C3"),
        6,
    );
    let stack_actions = ranged_actions(&stacked.session, &stacked.shooter_id);
    let south_hits = stack_actions
        .iter()
        .filter(|descriptor| descriptor["direction"] == "south")
        .collect::<Vec<_>>();
    assert_eq!(south_hits.len(), 2);
    assert!(south_hits.iter().all(|descriptor| {
        descriptor["path"]
            == json!([
                { "cell": "C4", "region": "surface" },
                { "cell": "C3", "region": "surface" },
            ])
    }));
    let ids = south_hits
        .iter()
        .map(|descriptor| {
            descriptor["hit"]["instanceId"]
                .as_str()
                .expect("target identity")
        })
        .collect::<Vec<_>>();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    let projectile_kinds = stacked
        .session
        .legal_actions()
        .expect("mixed projectile actions")
        .into_iter()
        .filter_map(|action| {
            matches!(
                action.descriptor["kind"].as_str(),
                Some("shoot-damage-projectile" | "shoot-projectile")
            )
            .then(|| action.descriptor["kind"].clone())
        })
        .collect::<Vec<_>>();
    assert_eq!(projectile_kinds.len(), 10);
    assert!(
        projectile_kinds
            .chunks_exact(2)
            .all(|pair| { pair == ["shoot-damage-projectile", "shoot-projectile"] })
    );
    assert_exact_replay(&stacked.session);
}

#[test]
fn ranged_issuance_should_require_an_enabled_ready_minion() {
    let ready = prepare_ranged(
        184,
        &minion(json!({ "ranged": true })),
        &minion(json!({})),
        &minion(json!({ "summonToAnySite": true })),
        &site(json!({})),
        None,
        None,
        6,
    );
    let mut missed = ready.session;
    let (_, receipt) = accept_where(&mut missed, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["direction"] == "east"
            && descriptor["hit"].is_null()
    });
    assert_eq!(event_types(&receipt), ["projectile-shot"]);
    assert_eq!(unit(&state(&missed), &ready.shooter_id)["tapped"], true);
    assert!(ranged_actions(&missed, &ready.shooter_id).is_empty());
    assert_exact_replay(&missed);

    let disabled = prepare_ranged(
        185,
        &minion(json!({
            "genesisDisableSelfUntilDamaged": true,
            "ranged": true,
        })),
        &minion(json!({})),
        &minion(json!({})),
        &site(json!({})),
        None,
        None,
        6,
    );
    assert_eq!(
        unit(&state(&disabled.session), &disabled.shooter_id)["disabledUntilDamaged"],
        true
    );
    assert!(ranged_actions(&disabled.session, &disabled.shooter_id).is_empty());
    assert_exact_replay(&disabled.session);
}

#[test]
fn ranged_damage_should_keep_current_power_unit_source_without_return_strike() {
    let setup = prepare_ranged(
        186,
        &minion(json!({ "attack": 4, "defense": 1, "ranged": true })),
        &minion(json!({
            "attack": 100,
            "preventsDamageFromUnitsWithPowerAtLeast": 3,
            "summonToAnySite": true,
        })),
        &minion(json!({})),
        &site(json!({})),
        Some("C3"),
        None,
        6,
    );
    let target_id = setup.near_target_id.as_ref().expect("prevention target");
    let mut session = setup.session;
    let receipt = fire_south(&mut session, &setup.shooter_id, target_id);
    assert_eq!(
        event_types(&receipt),
        ["projectile-shot", "strike-damage-allocated", "damage-dealt"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 4,
            "strikerInstanceId": setup.shooter_id,
            "targetInstanceId": target_id,
        })
    );
    assert_eq!(
        receipt.events[2].payload,
        json!({
            "accumulated": 0,
            "amount": 0,
            "attemptedAmount": 4,
            "direct": true,
            "instanceId": target_id,
            "prevented": true,
            "seat": "south",
        })
    );
    let position = state(&session);
    assert_eq!(unit(&position, target_id)["damage"], 0);
    assert_eq!(unit(&position, &setup.shooter_id)["damage"], 0);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct strike proof keeps Ward, death, Lethal, Deathrite, and terminal ordering together"
)]
fn ranged_strikes_should_apply_ward_lethal_deathrites_and_terminal_results() {
    let warded = prepare_ranged(
        187,
        &minion(json!({ "attack": 4, "ranged": true })),
        &minion(json!({ "defense": 1, "summonToAnySite": true, "ward": true })),
        &minion(json!({})),
        &site(json!({})),
        Some("C3"),
        None,
        6,
    );
    let warded_id = warded.near_target_id.as_ref().expect("Ward target");
    let mut warded_session = warded.session;
    let receipt = fire_south(&mut warded_session, &warded.shooter_id, warded_id);
    assert_eq!(
        event_types(&receipt),
        [
            "projectile-shot",
            "strike-damage-allocated",
            "damage-dealt",
            "ward-broken",
        ]
    );
    let position = state(&warded_session);
    assert_eq!(unit(&position, warded_id)["damage"], 0);
    assert_eq!(unit(&position, warded_id)["warded"], false);
    assert_eq!(unit(&position, &warded.shooter_id)["damage"], 0);
    assert_exact_replay(&warded_session);

    for (seed, shooter, target) in [
        (
            188,
            minion(json!({ "attack": 4, "ranged": true })),
            minion(json!({ "defense": 4, "summonToAnySite": true })),
        ),
        (
            189,
            minion(json!({ "lethal": true, "ranged": true })),
            minion(json!({ "defense": 10, "summonToAnySite": true })),
        ),
    ] {
        let dead = prepare_ranged(
            seed,
            &shooter,
            &target,
            &minion(json!({})),
            &site(json!({})),
            Some("C3"),
            None,
            6,
        );
        let target_id = dead.near_target_id.as_ref().expect("death target");
        let mut session = dead.session;
        let receipt = fire_south(&mut session, &dead.shooter_id, target_id);
        assert_eq!(
            event_types(&receipt),
            [
                "projectile-shot",
                "strike-damage-allocated",
                "damage-dealt",
                "minion-died",
            ]
        );
        assert!(
            state(&session)["players"]["south"]["cemetery"]
                .as_array()
                .expect("South cemetery")
                .iter()
                .any(|card| card["instanceId"] == *target_id)
        );
        assert_exact_replay(&session);
    }

    let deck_out = prepare_ranged(
        190,
        &minion(json!({ "attack": 1, "ranged": true })),
        &minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
        &minion(json!({})),
        &site(json!({})),
        Some("C3"),
        None,
        5,
    );
    let target_id = deck_out.near_target_id.as_ref().expect("Deathrite target");
    let mut terminal = deck_out.session;
    let receipt = fire_south(&mut terminal, &deck_out.shooter_id, target_id);
    assert_eq!(
        event_types(&receipt),
        [
            "projectile-shot",
            "strike-damage-allocated",
            "damage-dealt",
            "minion-died",
            "game-ended",
        ]
    );
    assert_eq!(
        state(&terminal)["terminal"],
        json!({
            "loser": "south",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "north",
        })
    );
    assert!(
        terminal
            .legal_actions()
            .expect("terminal actions")
            .is_empty()
    );
    assert_exact_replay(&terminal);
}

#[test]
fn ranged_hit_should_offer_one_replayable_optional_step() {
    let setup = prepare_ranged(
        192,
        &minion(json!({
            "mayStepAfterRangedStrike": true,
            "ranged": true,
        })),
        &minion(json!({ "defense": 1, "summonToAnySite": true })),
        &minion(json!({})),
        &site(json!({})),
        Some("C3"),
        None,
        6,
    );
    let target_id = setup.near_target_id.as_ref().expect("step target");
    let mut session = setup.session;
    fire_south(&mut session, &setup.shooter_id, target_id);
    let position = state(&session);
    assert_eq!(position["phase"], "ranged-step");
    assert_eq!(
        position["pendingRangedStep"]["sourceInstanceId"],
        setup.shooter_id
    );
    let actions = session.legal_actions().expect("Ranged step actions");
    assert_eq!(actions[0].descriptor["choice"], "decline");
    assert_eq!(actions[1].descriptor["choice"], "step");
    assert_eq!(actions[1].descriptor["to"]["cell"], "C3");
    assert_checkpoint_round_trip(&session);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-ranged-step" && descriptor["choice"] == "step"
    });
    assert_eq!(event_types(&receipt), ["unit-stepped"]);
    assert_eq!(unit(&state(&session), &setup.shooter_id)["location"], "C3");
    assert_eq!(state(&session)["phase"], "main");
    assert!(state(&session)["pendingRangedStep"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn empty_ranged_projectile_should_not_offer_a_step() {
    let setup = prepare_ranged(
        196,
        &minion(json!({
            "mayStepAfterRangedStrike": true,
            "ranged": true,
        })),
        &minion(json!({})),
        &minion(json!({})),
        &site(json!({})),
        None,
        None,
        6,
    );
    let mut session = setup.session;
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["shooterInstanceId"] == setup.shooter_id
            && descriptor["direction"] == "east"
            && descriptor["hit"].is_null()
    });
    assert_eq!(event_types(&receipt), ["projectile-shot"]);
    let position = state(&session);
    assert_eq!(position["phase"], "main");
    assert!(position.get("pendingRangedStep").is_none());
    assert_exact_replay(&session);
}

#[test]
fn ranged_deathrites_should_clear_a_dead_shooters_pending_step() {
    let setup = prepare_ranged(
        195,
        &minion(json!({
            "defense": 1,
            "mayStepAfterRangedStrike": true,
            "ranged": true,
        })),
        &minion(json!({
            "deathriteDamageEachUnitHere": 1,
            "defense": 1,
            "summonToAnySite": true,
        })),
        &minion(json!({})),
        &site(json!({})),
        Some("C4"),
        None,
        6,
    );
    let target_id = setup.near_target_id.as_ref().expect("Deathrite target");
    let mut session = setup.session;
    fire_south(&mut session, &setup.shooter_id, target_id);
    let position = state(&session);
    assert_eq!(position["phase"], "main");
    assert!(position.get("pendingRangedStep").is_none());
    assert!(
        position["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["instanceId"] != setup.shooter_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn immobile_ranged_minion_should_not_offer_a_post_strike_step() {
    let setup = prepare_ranged(
        197,
        &minion(json!({
            "immobile": true,
            "mayStepAfterRangedStrike": true,
            "ranged": true,
        })),
        &minion(json!({ "summonToAnySite": true })),
        &minion(json!({})),
        &site(json!({})),
        Some("C3"),
        None,
        6,
    );
    let target_id = setup.near_target_id.as_ref().expect("immobile target");
    let mut session = setup.session;
    fire_south(&mut session, &setup.shooter_id, target_id);
    let position = state(&session);
    assert_eq!(position["phase"], "main");
    assert!(position.get("pendingRangedStep").is_none());
    assert_exact_replay(&session);
}

#[test]
fn ranged_minion_should_pause_basic_movement_for_one_strike() {
    let setup = prepare_ranged(
        193,
        &minion(json!({
            "mayRangedStrikeOnceDuringBasicMovement": true,
            "movementBonus": 1,
            "ranged": true,
        })),
        &minion(json!({ "defense": 5, "summonToAnySite": true })),
        &minion(json!({})),
        &site(json!({})),
        Some("C2"),
        None,
        6,
    );
    let target_id = setup.near_target_id.as_ref().expect("movement target");
    let mut session = setup.session;
    let (_, started) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "C3", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    });
    assert_eq!(event_types(&started), ["basic-movement-started"]);
    assert_eq!(state(&session)["phase"], "movement");
    assert_eq!(unit(&state(&session), &setup.shooter_id)["location"], "C4");
    assert_eq!(
        session
            .legal_actions()
            .expect("movement actions")
            .iter()
            .map(|action| action.descriptor["kind"].as_str().expect("action kind"))
            .collect::<Vec<_>>(),
        [
            "shoot-projectile",
            "shoot-projectile",
            "shoot-projectile",
            "shoot-projectile",
            "continue-basic-movement",
        ]
    );
    let (_, continued) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    assert_eq!(event_types(&continued), ["basic-movement-continued"]);
    assert_eq!(unit(&state(&session), &setup.shooter_id)["location"], "C3");
    fire_south(&mut session, &setup.shooter_id, target_id);
    assert_eq!(state(&session)["phase"], "movement");
    assert!(ranged_actions(&session, &setup.shooter_id).is_empty());
    let (_, continued) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    assert_eq!(event_types(&continued), ["basic-movement-continued"]);
    assert_eq!(unit(&state(&session), &setup.shooter_id)["location"], "C2");
    let (_, finished) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    assert_eq!(event_types(&finished), ["move-and-attack-activated"]);
    assert_eq!(state(&session)["phase"], "attack");
    assert!(state(&session)["pendingBasicMovement"].is_null());
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves incremental Defend, one Ranged shot, and replay together"
)]
fn ranged_minion_should_pause_defend_movement_for_one_strike() {
    let ranged = minion(json!({
        "mayRangedStrikeOnceDuringBasicMovement": true,
        "ranged": true,
    }));
    let encoded = manifest(
        194,
        &ranged,
        &minion(json!({ "summonToAnySite": true })),
        &minion(json!({})),
        &site(json!({})),
        6,
    );
    let mut session = Session::new(&encoded).expect("valid Defend Ranged movement scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (first, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-shooter"
            && descriptor["cell"] == "C4"
    });
    let defender_id = first["cardInstanceId"]
        .as_str()
        .expect("defender identity")
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
    let (target, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-shooter"
            && descriptor["cardInstanceId"] != defender_id
            && descriptor["cell"] == "C3"
    });
    let target_id = target["cardInstanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (attacker, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-near"
            && descriptor["cell"] == "C2"
    });
    let attacker_id = attacker["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["instanceId"] == target_id
    });
    let (_, started) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == defender_id
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "C3", "region": "surface" },
                ])
    });
    assert_eq!(event_types(&started), ["basic-movement-started"]);
    let (_, shot) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["shooterInstanceId"] == defender_id
            && descriptor["direction"] == "east"
    });
    assert_eq!(event_types(&shot), ["projectile-shot"]);
    assert!(ranged_actions(&session, &defender_id).is_empty());
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    let (_, finished) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    assert_eq!(event_types(&finished), ["defender-joined"]);
    assert_eq!(state(&session)["phase"], "defend");
    assert!(
        state(&session)["pendingCombat"]["defenders"]
            .as_array()
            .expect("defenders")
            .iter()
            .any(|defender| defender["instanceId"] == defender_id)
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves terminal cleanup after ordered Deathrites interrupt movement"
)]
fn ordered_deathrites_ending_a_game_should_clear_incremental_ranged_movement() {
    let cards = json!({
        "north-avatar": avatar(),
        "north-mover": minion(json!({
            "lethal": true,
            "mayRangedStrikeOnceDuringBasicMovement": true,
            "movementBonus": 1,
            "ranged": true,
        })),
        "north-shooter": minion(json!({ "ranged": true })),
        "north-site": site(json!({ "rangedUnitsHereRangeBonus": 1 })),
        "south-avatar": avatar(),
        "south-aura": minion(json!({
            "defense": 1,
            "otherNearbyAlliesPowerBonus": 1,
            "summonToAnySite": true,
        })),
        "south-deathrite-a": minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
        "south-deathrite-b": minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
        "south-site": site(json!({})),
    });
    let encoded = power_loss_manifest(
        198,
        "synthetic-ranged-movement-terminal-deathrites-v1",
        &cards,
        ["north-shooter", "north-shooter", "north-mover"],
        ["south-aura", "south-deathrite-a", "south-deathrite-b"],
        6,
    );
    let mut session = Session::new(&encoded).expect("valid terminal movement scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let mut shooters = Vec::new();
    for _ in 0..2 {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-shooter"
                && descriptor["cell"] == "C4"
        });
        shooters.push(
            summon["cardInstanceId"]
                .as_str()
                .expect("shooter identity")
                .to_owned(),
        );
    }
    let (mover, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-mover"
            && descriptor["cell"] == "C4"
    });
    let mover_id = mover["cardInstanceId"]
        .as_str()
        .expect("mover identity")
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
    let mut south_ids = Vec::new();
    for card_id in ["south-aura", "south-deathrite-a", "south-deathrite-b"] {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == card_id
                && descriptor["cell"] == "C2"
        });
        south_ids.push(
            summon["cardInstanceId"]
                .as_str()
                .expect("South minion identity")
                .to_owned(),
        );
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });

    fire_south(&mut session, &shooters[0], &south_ids[1]);
    fire_south(&mut session, &shooters[1], &south_ids[2]);
    assert_eq!(unit(&state(&session), &south_ids[1])["damage"], 1);
    assert_eq!(unit(&state(&session), &south_ids[2])["damage"], 1);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == mover_id
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "C3", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    assert_eq!(unit(&state(&session), &mover_id)["cardId"], "north-mover");
    assert_eq!(
        unit(&state(&session), &south_ids[0])["cardId"],
        "south-aura"
    );
    let shot = fire_south(&mut session, &mover_id, &south_ids[0]);
    assert_eq!(
        event_types(&shot),
        ["projectile-shot", "strike-damage-allocated", "damage-dealt",]
    );
    let ordered = state(&session);
    assert_eq!(ordered["phase"], "deathrite-order");
    assert_eq!(ordered["decisionSeat"], "south");
    assert_eq!(
        ordered["pendingBasicMovement"]["sourceInstanceId"],
        mover_id
    );
    assert!(ordered.get("pendingRangedStep").is_none());
    assert_checkpoint_round_trip(&session);

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "deathrite-order-committed",
            "minion-died",
            "minion-died",
            "minion-died",
            "game-ended",
        ]
    );
    let terminal = state(&session);
    assert_eq!(
        terminal["terminal"],
        json!({
            "loser": "south",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "north",
        })
    );
    assert!(terminal.get("pendingBasicMovement").is_none());
    assert!(terminal.get("pendingRangedStep").is_none());
    assert!(
        session
            .legal_actions()
            .expect("terminal actions")
            .is_empty()
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves derived-power death and Deathrite resume on a movement edge"
)]
fn incremental_movement_power_loss_should_resolve_deathrite_and_resume() {
    let cards = json!({
        "north-ally": minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
        })),
        "north-avatar": avatar(),
        "north-mover": minion(json!({
            "mayRangedStrikeOnceDuringBasicMovement": true,
            "movementBonus": 1,
            "otherNearbyAlliesPowerBonus": 1,
            "ranged": true,
        })),
        "north-damager": minion(json!({ "genesisDamageEachOtherUnitHere": 1 })),
        "north-site": site(json!({})),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(json!({})),
    });
    let encoded = power_loss_manifest(
        199,
        "synthetic-ranged-movement-power-loss-v1",
        &cards,
        ["north-mover", "north-ally", "north-damager"],
        ["south-filler", "south-filler", "south-filler"],
        6,
    );
    let mut session = Session::new(&encoded).expect("valid movement power-loss scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let mut north_ids = Vec::new();
    for card_id in ["north-mover", "north-ally"] {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == card_id
                && descriptor["cell"] == "C4"
        });
        north_ids.push(
            summon["cardInstanceId"]
                .as_str()
                .expect("North minion identity")
                .to_owned(),
        );
    }
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

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-damager"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(unit(&state(&session), &north_ids[1])["damage"], 1);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_ids[0]
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "C3", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    });
    let (_, first_edge) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    assert_eq!(event_types(&first_edge), ["basic-movement-continued"]);
    assert_eq!(unit(&state(&session), &north_ids[1])["damage"], 1);
    let (_, second_edge) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    assert_eq!(
        event_types(&second_edge),
        ["basic-movement-continued", "site-drawn", "minion-died"]
    );
    let resumed = state(&session);
    assert_eq!(resumed["phase"], "movement");
    assert_eq!(resumed["decisionSeat"], "north");
    assert_eq!(resumed["pendingBasicMovement"]["pathIndex"], 2);
    assert!(
        resumed["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == north_ids[1])
    );
    assert_checkpoint_round_trip(&session);
    let (_, finished) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "continue-basic-movement"
    });
    assert_eq!(event_types(&finished), ["move-and-attack-activated"]);
    assert_eq!(state(&session)["phase"], "attack");
    assert!(state(&session)["pendingBasicMovement"].is_null());
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves post-strike queue timing through ordered derived deaths"
)]
fn ranged_step_should_not_queue_after_its_shooter_loses_derived_defense() {
    let cards = json!({
        "north-aura": minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "otherNearbyAlliesPowerBonus": 1,
        })),
        "north-avatar": avatar(),
        "north-damager": minion(json!({ "genesisDamageEachOtherUnitHere": 1 })),
        "north-shooter": minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "mayStepAfterRangedStrike": true,
            "ranged": true,
        })),
        "north-site": site(json!({})),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(json!({})),
    });
    let encoded = power_loss_manifest(
        200,
        "synthetic-ranged-step-derived-death-v1",
        &cards,
        ["north-shooter", "north-aura", "north-damager"],
        ["south-filler", "south-filler", "south-filler"],
        7,
    );
    let mut session = Session::new(&encoded).expect("valid derived-death Ranged scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (shooter, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-shooter"
            && descriptor["cell"] == "C4"
    });
    let shooter_id = shooter["cardInstanceId"]
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
    let (aura, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cell"] == "C3"
    });
    let aura_id = aura["cardInstanceId"]
        .as_str()
        .expect("aura identity")
        .to_owned();
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-damager"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(unit(&state(&session), &shooter_id)["damage"], 1);

    let shot = fire_south(&mut session, &shooter_id, &aura_id);
    assert_eq!(
        event_types(&shot),
        ["projectile-shot", "strike-damage-allocated", "damage-dealt"]
    );
    let ordered = state(&session);
    assert_eq!(ordered["phase"], "deathrite-order");
    assert_eq!(ordered["decisionSeat"], "north");
    assert!(ordered.get("pendingRangedStep").is_none());
    assert!(
        ordered["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != shooter_id && unit["instanceId"] != aura_id)
    );
    assert_eq!(
        ordered["pendingDeathrites"]["batches"][0]["activeRemaining"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_checkpoint_round_trip(&session);

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
            "minion-died",
        ]
    );
    let resumed = state(&session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed.get("pendingRangedStep").is_none());
    assert_eq!(
        resumed["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .filter(|card| card["instanceId"] == shooter_id || card["instanceId"] == aura_id)
            .count(),
        2
    );
    assert_exact_replay(&session);
}
