use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode, Seat};
use sorcery_engine::game::Game;
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::simulator::{replay_selected, run_game};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const HASH_A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn baseline_policy() -> PolicySnapshot {
    let mut body = json!({
        "authorityHash": HASH_A,
        "deckId": HASH_B,
        "engineVersion": "sorcery-core-v1",
        "generation": 0,
        "observationVersion": "seat-observation-v1",
        "schemaVersion": 1,
        "selector": {
            "atlasReserve": 3,
            "featurePriority": [
                "keep-mulligan",
                "play-site",
                "summon-minion",
                "preferred-draw",
                "powered-movement",
                "beneficial-tactic",
                "move-toward-enemy",
                "end-turn",
                "canonical-fallback"
            ]
        },
        "tieBreak": "canonical-action-order-v1"
    });
    body["policyId"] = json!(identity_hash(&body).expect("policy identity"));
    parse_policy_snapshot(&canonical_json(&body).expect("canonical policy"))
        .expect("valid baseline policy")
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
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn event_values(receipts: &[&Receipt]) -> Vec<Value> {
    receipts
        .iter()
        .flat_map(|receipt| &receipt.events)
        .map(|event| json!({ "payload": event.payload, "type": event.event_type }))
        .collect()
}

fn exact_replay(session: &Session) -> Session {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    Session::replay(session.manifest_json(), &action_ids).expect("exact authoritative replay")
}

fn canonical_manifest(mut manifest: Value) -> String {
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId")
        .expect("manifest identity");
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical manifest")
}

#[test]
fn deterministic_policy_completes_movement_fight_terminal_replay() {
    let manifest = synthetic_demo_manifest_json(31).expect("synthetic manifest");
    let policy = baseline_policy();
    let rollout = run_game(
        Game::from_manifest_json(&manifest).expect("valid game"),
        &policy,
        &policy,
        400,
    )
    .expect("deterministic rollout");
    let replay = replay_selected(&manifest, &rollout).expect("authoritative selected replay");
    let event_types: Vec<&str> = replay
        .transcript()
        .iter()
        .flat_map(|receipt| &receipt.events)
        .map(|event| event.event_type.as_str())
        .collect();

    assert!(
        rollout.is_terminal()
            && rollout.outcome().is_some()
            && replay.outcome() == rollout.outcome()
            && replay.verify_replay().expect("verified replay")
            && event_types.contains(&"move-and-attack-activated")
            && event_types.contains(&"fight-started"),
        "policy must move, fight, terminate, and replay exactly"
    );
}

#[test]
fn forged_stale_and_wrong_seat_requests_preserve_authoritative_game() {
    let manifest = synthetic_demo_manifest_json(31).expect("synthetic manifest");
    let mut session = Session::new(&manifest).expect("valid session");
    let action = session.legal_actions().expect("opening actions")[0].clone();
    let before = (
        session.replay_value().expect("initial replay value"),
        session.state_hash().expect("initial state hash"),
        session
            .initial_random_draws_hash()
            .expect("initial random draws hash"),
    );
    let requests = [
        (
            ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version + 1,
            },
            RejectionCode::StaleVersion,
        ),
        (
            ActionRequest {
                action_id: action.action_id.to_string(),
                seat: Seat::South,
                state_version: action.state_version,
            },
            RejectionCode::WrongSeat,
        ),
        (
            ActionRequest {
                action_id: "forged-spatial-action".to_owned(),
                seat: action.seat,
                state_version: action.state_version,
            },
            RejectionCode::UnknownAction,
        ),
    ];

    for (request, expected) in requests {
        assert!(matches!(
            session.step(request).expect("stable rejection"),
            StepResult::Rejected(rejection) if rejection.code == expected
        ));
    }

    assert_eq!(
        (
            session.replay_value().expect("unchanged replay value"),
            session.state_hash().expect("unchanged state hash"),
            session
                .initial_random_draws_hash()
                .expect("unchanged random draws hash"),
            session.transcript().len(),
        ),
        (before.0, before.1, before.2, 0)
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct rule proof keeps the staged movement and strike transcript together"
)]
fn move_and_attack_stages_movement_before_undefended_site_strike() {
    let manifest = synthetic_demo_manifest_json(47).expect("synthetic manifest");
    let mut session = Session::new(&manifest).expect("valid session");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let attacker_id = summon["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (movement, moved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["from"]["cell"] == "C3"
            && descriptor["to"]["cell"] == "C2"
    });
    let staged = state(&session);
    let staged_attacker = staged["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == attacker_id)
        .expect("staged attacker");
    assert_eq!(
        (
            staged["phase"].clone(),
            staged["decisionSeat"].clone(),
            staged_attacker["location"].clone(),
            staged_attacker["tapped"].clone(),
        ),
        (json!("attack"), json!("north"), json!("C2"), json!(true))
    );

    let site_id = staged["realm"]["sites"]["C2"]["instanceId"]
        .as_str()
        .expect("enemy site identity")
        .to_owned();
    let (declared, declaration) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_id
    });
    let (_, strike) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });

    assert_eq!(
        event_values(&[&moved, &declaration, &strike]),
        vec![
            json!({
                "payload": {
                    "from": movement["from"].clone(),
                    "path": movement["path"].clone(),
                    "seat": "north",
                    "steps": 1,
                    "to": movement["to"].clone(),
                    "unitInstanceId": attacker_id,
                },
                "type": "move-and-attack-activated",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": attacker_id,
                    "cell": "C2",
                    "seat": "north",
                    "target": declared["target"].clone(),
                },
                "type": "attack-declared",
            }),
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": false },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "attackerInstanceId": attacker_id,
                    "cell": "C2",
                    "siteInstanceId": site_id,
                },
                "type": "undefended-site-struck",
            }),
            json!({
                "payload": { "amount": 1, "life": 19, "seat": "south" },
                "type": "avatar-life-lost",
            }),
        ]
    );
    let finished = state(&session);
    assert_eq!(
        (
            finished["phase"].clone(),
            finished["decisionSeat"].clone(),
            finished["pendingCombat"].clone(),
        ),
        (json!("main"), json!("north"), Value::Null)
    );
    let replayed = exact_replay(&session);
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
}

#[test]
fn manifest_fact_and_deck_scope_invalid_shapes_fail_closed() {
    let manifest = synthetic_demo_manifest_json(59).expect("synthetic manifest");
    let valid: Value = serde_json::from_str(&manifest).expect("manifest value");
    let mut invalid = Vec::new();

    let mut unknown_manifest_field = valid.clone();
    unknown_manifest_field["futureManifestField"] = json!(true);
    invalid.push((
        "unknown manifest field",
        unknown_manifest_field,
        "unknown field",
    ));

    let mut unknown_fact = valid.clone();
    unknown_fact["cards"]["north-spell-1"]["futureRule"] = json!(true);
    invalid.push(("unknown card fact", unknown_fact, "futureRule"));

    let mut wrong_zone = valid.clone();
    wrong_zone["decks"]["north"]["atlas"][0] = json!("north-spell-1");
    invalid.push((
        "minion in Atlas",
        wrong_zone,
        "atlas must contain only sites",
    ));

    let mut unreferenced = valid;
    unreferenced["cards"]["unreferenced-site"] =
        json!({ "cardType": "site", "elements": ["earth"] });
    invalid.push((
        "unreferenced definition",
        unreferenced,
        "exactly the deck-referenced definitions",
    ));

    for (name, manifest, expected) in invalid {
        let error = Game::from_manifest_json(&canonical_manifest(manifest)).expect_err(name);
        assert!(
            error.to_string().contains(expected),
            "{name}: expected {expected:?}, found {error}"
        );
    }
}
