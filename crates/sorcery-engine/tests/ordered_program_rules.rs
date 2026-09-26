//! Public API proofs for an authored ordered Magic program.
//!
//! The fixture is synthetic: draw-card, choose any allied unit, then grant
//! Movement +1 for this turn. It deliberately exercises the draw choice as a
//! separate action before the later unit choice.

#[path = "common/mod.rs"]
mod common;
use common::{modifier_rows, modifier_sources};

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

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
    json!({"cardType": "site", "elements": ["earth"]})
}

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
    })
}

fn ordered_magic(legacy_flag: bool) -> Value {
    let mut card = json!({
        "cardType": "magic",
        "effectProgram": {
            "effects": [
                {"op": "draw-card"},
                {
                    "op": "choose-unit",
                    "kind": null,
                    "relation": "anywhere",
                    "alliedOnly": true,
                },
                {
                    "op": "grant-this-turn",
                    "recipients": "chosen",
                    "modifier": "movement",
                    "amount": 1,
                },
            ],
        },
        "manaCost": 0,
        "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
    });
    if legacy_flag {
        card["grantMovementOneToAllyThisTurnThenDrawSpell"] = json!(true);
    }
    card
}

fn finish_manifest(mut manifest: Value) -> String {
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("synthetic manifest identity"));
    canonical_json(&manifest).expect("canonical synthetic manifest")
}

fn manifest(seed: u32, empty_atlas: bool, legacy_flag: bool) -> String {
    let atlas = if empty_atlas {
        vec!["north-site"; 3]
    } else {
        vec!["north-site"; 5]
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({"fixture": "ordered-program"}))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ordered-program-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-program": ordered_magic(legacy_flag),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": ally(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": atlas,
                "avatar": "north-avatar",
                "spellbook": vec!["north-program"; 8],
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
    }))
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid ordered-program session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn cast_program(session: &mut Session) -> (String, Receipt) {
    let (descriptor, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-program"
    });
    (
        descriptor["cardInstanceId"]
            .as_str()
            .expect("program source identity")
            .to_owned(),
        receipt,
    )
}

fn choose_draw(session: &mut Session, source_id: &str, zone: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "choose-ability-draw"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor["zone"] == zone
    });
    receipt
}

fn choose_avatar(session: &mut Session, source_id: &str) -> Receipt {
    let avatar_id = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar identity")
        .to_owned();
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "choose-ability"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["instanceId"] == avatar_id
    });
    receipt
}

fn assert_checkpoint_and_replay(session: &Session) {
    let clone = session.clone();
    assert_eq!(
        clone.replay_value().unwrap(),
        session.replay_value().unwrap()
    );

    let checkpoint = create_game_checkpoint(session).expect("ordered-program checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let restored =
        resume_game_checkpoint(&parse_game_checkpoint(&serialized).expect("parsed checkpoint"))
            .expect("restored checkpoint");
    assert_eq!(
        restored.replay_value().unwrap(),
        session.replay_value().unwrap()
    );

    let action_ids = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect::<Vec<_>>();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
    assert_eq!(replayed.transcript(), session.transcript());
}

fn run_successful_program(seed: u32, zone: &str) {
    let mut session = opening_main(&manifest(seed, false, false));
    let (source_id, cast) = cast_program(&mut session);
    assert_eq!(event_types(&cast), ["magic-cast"]);
    let mut zones = session
        .legal_actions()
        .expect("draw choices")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "choose-ability-draw"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .map(|action| {
            action.descriptor["zone"]
                .as_str()
                .expect("draw choice zone")
                .to_owned()
        })
        .collect::<Vec<_>>();
    zones.sort();
    assert_eq!(zones, ["atlas", "spellbook"]);

    assert_checkpoint_and_replay(&session);
    let draw = choose_draw(&mut session, &source_id, zone);
    assert!(event_types(&draw).iter().any(|event| {
        (*event == "site-drawn" && zone == "atlas")
            || (*event == "spell-drawn" && zone == "spellbook")
    }));
    assert!(
        !modifier_rows(&state(&session)["players"]["north"]["avatar"], "movement")
            .iter()
            .any(|row| row["sourceInstanceId"] == source_id)
    );

    assert_checkpoint_and_replay(&session);
    let grant = choose_avatar(&mut session, &source_id);
    assert_eq!(
        event_types(&grant),
        [
            "ability-choice-committed",
            "movement-granted",
            "magic-resolved"
        ]
    );
    let rows = modifier_rows(&state(&session)["players"]["north"]["avatar"], "movement");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["amount"], 1);
    assert_eq!(
        modifier_sources(&state(&session)["players"]["north"]["avatar"], "movement"),
        json!([source_id])
    );
    assert_checkpoint_and_replay(&session);

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(event_types(&ended).contains(&"movement-expired"));
    assert!(modifier_rows(&state(&session)["players"]["north"]["avatar"], "movement").is_empty());
}

#[test]
fn ordered_program_draws_atlas_before_choosing_avatar_and_granting_movement() {
    run_successful_program(4201, "atlas");
}

#[test]
fn ordered_program_draws_spellbook_before_choosing_avatar_and_granting_movement() {
    run_successful_program(4202, "spellbook");
}

#[test]
fn ordered_program_empty_chosen_deck_ends_before_later_grant() {
    let mut session = opening_main(&manifest(4203, true, false));
    let (source_id, _) = cast_program(&mut session);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "choose-ability-draw"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor["zone"] == "atlas"
    });
    assert!(event_types(&receipt).contains(&"game-ended"));
    assert_eq!(
        event_types(&receipt)
            .iter()
            .filter(|kind| **kind == "magic-resolved")
            .count(),
        1
    );
    assert_eq!(state(&session)["phase"], "terminal");
    assert_checkpoint_and_replay(&session);
    assert!(modifier_rows(&state(&session)["players"]["north"]["avatar"], "movement").is_empty());
    assert!(
        session
            .legal_actions()
            .expect("terminal actions")
            .is_empty()
    );
}

#[test]
fn ordered_program_rejects_mixed_legacy_flag() {
    assert!(Session::new(&manifest(4204, false, true)).is_err());
}

#[test]
fn ordered_program_rollouts_are_exercised_and_identical_across_workers() {
    use sorcery_engine::batch::{BatchJob, MAX_GAME_ACTIONS, run_game_batch};
    use sorcery_engine::game::Game;
    use sorcery_engine::policy::{
        baseline_policy_snapshot, parse_policy_snapshot, serialize_policy_snapshot,
    };
    use sorcery_engine::simulator::{replay_selected, run_game};

    let manifests = [manifest(4210, false, false), manifest(4211, false, false)];
    let game = Game::from_manifest_json(&manifests[0]).unwrap();
    let baseline =
        baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
            .unwrap();
    let mut policy: Value =
        serde_json::from_str(&serialize_policy_snapshot(&baseline).unwrap()).unwrap();
    // This probe casts the synthetic program instead of ending the turn before it.
    policy["selector"]["featurePriority"]
        .as_array_mut()
        .unwrap()
        .swap(7, 8);
    policy.as_object_mut().unwrap().remove("policyId");
    policy["policyId"] = json!(identity_hash(&policy).unwrap());
    let policy = parse_policy_snapshot(&canonical_json(&policy).unwrap()).unwrap();
    let rollout = run_game(game, &policy, &policy, MAX_GAME_ACTIONS).unwrap();
    let replayed = replay_selected(&manifests[0], &rollout).unwrap();
    assert!(
        replayed
            .transcript()
            .iter()
            .flat_map(|receipt| &receipt.events)
            .any(|event| event.event_type == "movement-granted")
    );
    let jobs: Vec<_> = manifests
        .iter()
        .map(|manifest_json| BatchJob {
            manifest_json,
            north_deck_id: policy.deck_id(),
            north_policy: &policy,
            south_deck_id: policy.deck_id(),
            south_policy: &policy,
        })
        .collect();
    let serial = run_game_batch(&jobs, 1).unwrap();
    let parallel = run_game_batch(&jobs, 2).unwrap();
    assert_eq!(serial, parallel);
    assert!(serial.iter().all(|result| result.report.replay_verified));
}
