use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::game::GameError;
use sorcery_engine::session::{Session, SessionError, StepResult};

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
fn unsupported_ranged_extensions_should_fail_closed() {
    for (seed, field, extra) in [
        (
            192,
            "mayStepAfterRangedStrike",
            json!({ "mayStepAfterRangedStrike": true, "ranged": true }),
        ),
        (
            193,
            "mayRangedStrikeOnceDuringBasicMovement",
            json!({ "mayRangedStrikeOnceDuringBasicMovement": true, "ranged": true }),
        ),
    ] {
        let encoded = manifest(
            seed,
            &minion(extra),
            &minion(json!({})),
            &minion(json!({})),
            &site(json!({ "isTower": true })),
            6,
        );
        let error = Session::new(&encoded).expect_err("unsupported Ranged extension");
        match error {
            SessionError::Game(GameError::UnsupportedManifestFact(actual)) => {
                assert_eq!(actual, field);
            }
            other => panic!("expected unsupported {field}, received {other}"),
        }
    }
}
