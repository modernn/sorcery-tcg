use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn minion(airborne: bool, ranged: bool) -> Value {
    json!({
        "airborne": airborne,
        "attack": 3,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "ranged": ranged,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn scenario_manifest(seed: u32, attacker: &Value, responder: &Value) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "airborne-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-airborne-rules-v1",
        },
        "cards": {
            "north-airborne-attacker": attacker,
            "north-avatar": avatar,
            "north-site": site,
            "south-avatar": avatar,
            "south-responder": responder,
            "south-site": site,
        },
        "decks": {
            "north": {
                "atlas": ["north-site", "north-site", "north-site", "north-site"],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-airborne-attacker", "north-airborne-attacker",
                    "north-airborne-attacker", "north-airborne-attacker",
                    "north-airborne-attacker", "north-airborne-attacker",
                    "north-airborne-attacker", "north-airborne-attacker"
                ],
            },
            "south": {
                "atlas": ["south-site", "south-site", "south-site", "south-site"],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-responder", "south-responder", "south-responder", "south-responder",
                    "south-responder", "south-responder", "south-responder", "south-responder"
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical synthetic manifest")
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

struct AttackSetup {
    attacker_id: String,
    responder_id: String,
    session: Session,
}

fn attack_checkpoint(seed: u32, attacker: &Value, responder: &Value) -> AttackSetup {
    let manifest = scenario_manifest(seed, attacker, responder);
    let mut session = Session::new(&manifest).expect("valid Airborne scenario");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-airborne-attacker"
            && descriptor["cell"] == "C4"
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
            && descriptor["from"]["cell"] == "C4"
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
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-responder"
            && descriptor["cell"] == "C2"
    });
    let responder_id = summon["cardInstanceId"]
        .as_str()
        .expect("responder identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["from"]["cell"] == "C3"
            && descriptor["to"]["cell"] == "C2"
    });

    AttackSetup {
        attacker_id,
        responder_id,
        session,
    }
}

fn minion_is_attack_target(session: &Session, instance_id: &str) -> bool {
    session
        .legal_actions()
        .expect("attack targets")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "declare-attack"
                && action.descriptor["target"]["kind"] == "minion"
                && action.descriptor["target"]["instanceId"] == instance_id
        })
}

fn interceptor_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("Intercept actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "intercept")
        .map(|action| {
            action.descriptor["unitInstanceId"]
                .as_str()
                .expect("interceptor identity")
                .to_owned()
        })
        .collect()
}

fn movement_paths(session: &Session, instance_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("movement actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .map(|action| {
            action.descriptor["path"]
                .as_array()
                .expect("movement path")
                .iter()
                .map(|location| location["cell"].as_str().expect("path cell"))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect()
}

fn prepare_diagonal_site(session: &mut Session) {
    if state(session)["phase"] == "intercept" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B3"
    });
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the direct Airborne scenario keeps its ordered rule proof together"
)]
fn rule_catalog_0100_airborne_moves_diagonally_and_restricts_attacks_and_intercept() {
    let ground = minion(false, false);
    let airborne = minion(true, false);
    let ranged = minion(false, true);
    let mut disabled_airborne = minion(true, false);
    disabled_airborne["genesisDisableSelfUntilDamaged"] = json!(true);

    let ranged_attack = attack_checkpoint(118, &ranged, &airborne);
    assert!(!minion_is_attack_target(
        &ranged_attack.session,
        &ranged_attack.responder_id
    ));
    assert_exact_replay(&ranged_attack.session);

    let mut airborne_attack = attack_checkpoint(119, &airborne, &airborne);
    assert!(minion_is_attack_target(
        &airborne_attack.session,
        &airborne_attack.responder_id
    ));
    accept_where(&mut airborne_attack.session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(state(&airborne_attack.session)["phase"], "intercept");
    assert_eq!(
        interceptor_ids(&airborne_attack.session),
        [airborne_attack.responder_id.clone()]
    );
    assert_exact_replay(&airborne_attack.session);

    let mut ground_response = attack_checkpoint(120, &airborne, &ground);
    assert!(minion_is_attack_target(
        &ground_response.session,
        &ground_response.responder_id
    ));
    let (_, declined) = accept_where(&mut ground_response.session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(state(&ground_response.session)["phase"], "main");
    assert_eq!(declined.events[0].payload["interceptWindowOpened"], false);
    prepare_diagonal_site(&mut ground_response.session);
    assert_eq!(
        movement_paths(&ground_response.session, &ground_response.attacker_id),
        ["C2,B3", "C2,C1", "C2,C3", "C2"]
    );
    let (movement, receipt) = accept_where(&mut ground_response.session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == ground_response.attacker_id
            && descriptor["path"]
                == json!([
                    { "cell": "C2", "region": "surface" },
                    { "cell": "B3", "region": "surface" },
                ])
    });
    assert_eq!(
        (
            receipt.events[0].event_type.as_str(),
            &receipt.events[0].payload
        ),
        (
            "move-and-attack-activated",
            &json!({
                "from": movement["from"].clone(),
                "path": movement["path"].clone(),
                "seat": "north",
                "steps": 1,
                "to": movement["to"].clone(),
                "unitInstanceId": ground_response.attacker_id,
            }),
        )
    );
    assert_eq!(
        state(&ground_response.session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == ground_response.attacker_id)
            .expect("Airborne attacker")["location"],
        "B3"
    );
    assert_exact_replay(&ground_response.session);

    let mut ranged_response = attack_checkpoint(121, &airborne, &ranged);
    accept_where(&mut ranged_response.session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(state(&ranged_response.session)["phase"], "intercept");
    assert_eq!(
        interceptor_ids(&ranged_response.session),
        [ranged_response.responder_id.clone()]
    );
    assert_exact_replay(&ranged_response.session);

    let mut ground_movement = attack_checkpoint(122, &ground, &ground);
    accept_where(&mut ground_movement.session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    prepare_diagonal_site(&mut ground_movement.session);
    assert_eq!(
        movement_paths(&ground_movement.session, &ground_movement.attacker_id),
        ["C2,C1", "C2,C3", "C2"]
    );
    assert_exact_replay(&ground_movement.session);

    let disabled_response = attack_checkpoint(124, &ground, &disabled_airborne);
    let disabled_responder = state(&disabled_response.session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == disabled_response.responder_id)
        .expect("Disabled Airborne responder")
        .clone();
    assert_eq!(disabled_responder["disabledUntilDamaged"], true);
    assert!(minion_is_attack_target(
        &disabled_response.session,
        &disabled_response.responder_id
    ));
    assert_exact_replay(&disabled_response.session);

    assert_airborne_defends_along_a_diagonal_path();
}

#[expect(
    clippy::too_many_lines,
    reason = "the direct Defend scenario keeps its turn sequence and assertions together"
)]
fn assert_airborne_defends_along_a_diagonal_path() {
    let airborne = minion(true, false);
    let mut setup = attack_checkpoint(123, &airborne, &airborne);
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "close-intercept"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B3"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    });

    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.responder_id
            && descriptor["path"]
                == json!([
                    { "cell": "C2", "region": "surface" },
                    { "cell": "B3", "region": "surface" },
                ])
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    });

    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.attacker_id
            && descriptor["path"] == json!([{ "cell": "C2", "region": "surface" }])
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "site"
    });
    let expected_path = json!([
        { "cell": "B3", "region": "surface" },
        { "cell": "C2", "region": "surface" },
    ]);
    let (defend, joined) = accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == setup.responder_id
            && descriptor["path"] == expected_path
    });
    assert_eq!(
        defend,
        json!({
            "from": { "cell": "B3", "region": "surface" },
            "kind": "defend",
            "path": expected_path,
            "to": { "cell": "C2", "region": "surface" },
            "unitInstanceId": setup.responder_id,
        })
    );
    assert_eq!(
        joined
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["defender-joined", "original-target-removed"]
    );
    assert_eq!(
        joined.events[0].payload,
        json!({
            "from": { "cell": "B3", "region": "surface" },
            "instanceId": setup.responder_id,
            "path": [
                { "cell": "B3", "region": "surface" },
                { "cell": "C2", "region": "surface" },
            ],
            "seat": "south",
            "steps": 1,
            "to": { "cell": "C2", "region": "surface" },
        })
    );
    let final_state = state(&setup.session);
    let responder = final_state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == setup.responder_id)
        .expect("Airborne defender");
    assert_eq!(
        (responder["location"].clone(), responder["tapped"].clone()),
        (json!("C2"), json!(true))
    );
    assert_exact_replay(&setup.session);
}
