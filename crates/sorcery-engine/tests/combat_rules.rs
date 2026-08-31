use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

struct AttackSetup {
    attacker_instance_id: String,
    session: Session,
    target_instance_id: String,
}

fn scenario_manifest(seed: u32, minion_defense: u64, avatar_life: u64) -> String {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid checked-in TypeScript parity fixture");
    let manifest_json = fixture["games"]
        .as_array()
        .and_then(|games| games.iter().find(|game| game["seed"] == 31))
        .and_then(|game| game["manifestJson"].as_str())
        .expect("seed-31 canonical manifest JSON");
    let mut manifest: Value = serde_json::from_str(manifest_json).expect("manifest value");
    let body = manifest.as_object_mut().expect("manifest object");
    body.remove("manifestId").expect("manifest identity");
    body.insert("seed".to_owned(), json!(seed));
    for card in body["cards"]
        .as_object_mut()
        .expect("manifest cards")
        .values_mut()
    {
        match card["cardType"].as_str() {
            Some("avatar") => card["life"] = json!(avatar_life),
            Some("minion") => card["defense"] = json!(minion_defense),
            _ => {}
        }
    }
    let manifest_id = identity_hash(&manifest).expect("scenario manifest identity");
    manifest["manifestId"] = json!(manifest_id);
    canonical_json(&manifest).expect("canonical scenario manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step");
    let StepResult::Accepted(receipt) = result else {
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

fn north_attacks_at_c2(seed: u32, minion_defense: u64, avatar_life: u64) -> AttackSetup {
    let manifest = scenario_manifest(seed, minion_defense, avatar_life);
    let mut session = Session::new(&manifest).expect("valid scenario session");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let attacker_instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("attacker instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
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
            && descriptor["unitInstanceId"] == attacker_instance_id
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
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C2"
    });
    let target_instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("target instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_instance_id
            && descriptor["from"]["cell"] == "C3"
            && descriptor["to"]["cell"] == "C2"
    });

    AttackSetup {
        attacker_instance_id,
        session,
        target_instance_id,
    }
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn event_values(receipt: &Receipt) -> Vec<Value> {
    receipt
        .events
        .iter()
        .map(|event| json!({ "payload": event.payload, "type": event.event_type }))
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn surviving_minion_damage_should_persist_until_end_phase() {
    let AttackSetup {
        attacker_instance_id,
        mut session,
        target_instance_id,
    } = north_attacks_at_c2(61, 2, 20);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_instance_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });

    assert_eq!(
        event_values(&fight),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": true },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "attackerInstanceId": attacker_instance_id,
                    "combatantInstanceIds": [target_instance_id],
                },
                "type": "fight-started",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "strikerInstanceId": attacker_instance_id,
                    "targetInstanceId": target_instance_id,
                },
                "type": "strike-damage-allocated",
            }),
            json!({
                "payload": {
                    "accumulated": 1,
                    "amount": 1,
                    "direct": true,
                    "instanceId": attacker_instance_id,
                    "seat": "north",
                },
                "type": "damage-dealt",
            }),
            json!({
                "payload": {
                    "accumulated": 1,
                    "amount": 1,
                    "direct": true,
                    "instanceId": target_instance_id,
                    "seat": "south",
                },
                "type": "damage-dealt",
            }),
        ]
    );
    let damaged = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| unit["location"] == "C2")
        .map(|unit| unit["damage"].as_u64().expect("unit damage"))
        .collect::<Vec<_>>();
    assert_eq!(damaged, [1, 1]);

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_values(&ended),
        vec![
            json!({
                "payload": { "seat": "north", "turnNumber": 5 },
                "type": "turn-ended",
            }),
            json!({
                "payload": { "drawSkipped": false, "seat": "south", "turnNumber": 6 },
                "type": "turn-started",
            }),
        ]
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["damage"] == 0)
    );
    assert_exact_replay(&session);
}

#[test]
fn later_undefended_site_strikes_should_not_deliver_deaths_door_death_blows() {
    let AttackSetup {
        attacker_instance_id,
        mut session,
        ..
    } = north_attacks_at_c2(71, 1, 1);
    let site_instance_id = state(&session)["realm"]["sites"]["C2"]["instanceId"]
        .as_str()
        .expect("C2 site instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_instance_id
    });
    let (_, first_strike) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });
    assert_eq!(
        event_values(&first_strike),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": false },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "attackerInstanceId": attacker_instance_id,
                    "cell": "C2",
                    "siteInstanceId": site_instance_id,
                },
                "type": "undefended-site-struck",
            }),
            json!({
                "payload": { "amount": 1, "life": 0, "seat": "south" },
                "type": "avatar-life-lost",
            }),
            json!({
                "payload": { "seat": "south", "turnNumber": 5 },
                "type": "avatar-reached-deaths-door",
            }),
        ]
    );
    let first_state = state(&session);
    assert_eq!(first_state["players"]["south"]["avatar"]["life"], 0);
    assert_eq!(
        first_state["players"]["south"]["avatar"]["deathDoorTurn"],
        5
    );
    assert_eq!(first_state["terminal"], json!({ "status": "active" }));

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_instance_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_instance_id
    });
    let (_, later_strike) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });

    assert_eq!(
        event_values(&later_strike),
        vec![
            json!({
                "payload": { "defenderCount": 0, "originalTargetParticipates": false },
                "type": "defend-window-closed",
            }),
            json!({
                "payload": {
                    "amount": 1,
                    "attackerInstanceId": attacker_instance_id,
                    "cell": "C2",
                    "siteInstanceId": site_instance_id,
                },
                "type": "undefended-site-struck",
            }),
        ]
    );
    let later_state = state(&session);
    assert_eq!(later_state["players"]["south"]["avatar"]["life"], 0);
    assert_eq!(later_state["terminal"], json!({ "status": "active" }));
    assert_exact_replay(&session);
}
