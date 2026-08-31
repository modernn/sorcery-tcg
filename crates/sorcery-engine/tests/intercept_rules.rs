use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 1,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn scenario_manifest(seed: u32, responder: &Value) -> String {
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
            "contentHash": identity_hash(&json!({ "fixture": "intercept-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-intercept-rules-v1",
        },
        "cards": {
            "north-attacker": minion(),
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
                    "north-attacker", "north-attacker", "north-attacker", "north-attacker",
                    "north-attacker", "north-attacker", "north-attacker", "north-attacker"
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

fn exact_replay(session: &Session) {
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
    distant_responder_id: String,
    session: Session,
    target_id: String,
}

fn attack_checkpoint(seed: u32, responder: &Value) -> AttackSetup {
    let manifest = scenario_manifest(seed, responder);
    let mut session = Session::new(&manifest).expect("valid Intercept scenario");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-attacker"
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
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-responder"
            && descriptor["cell"] == "C1"
    });
    let distant_responder_id = summon["cardInstanceId"]
        .as_str()
        .expect("distant responder identity")
        .to_owned();
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
    assert_eq!(state(&session)["phase"], "main");
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
    let target_id = summon["cardInstanceId"]
        .as_str()
        .expect("co-located target identity")
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
        distant_responder_id,
        session,
        target_id,
    }
}

fn site_id(session: &Session) -> String {
    state(session)["realm"]["sites"]["C2"]["instanceId"]
        .as_str()
        .expect("C2 site identity")
        .to_owned()
}

#[test]
fn cannot_defend_should_block_movement_but_allow_stationary_defend_and_intercept() {
    let mut responder = minion();
    responder["cannotDefend"] = json!(true);
    let setup = attack_checkpoint(50, &responder);

    let mut moving = setup.session.clone();
    accept_where(&mut moving, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.target_id
    });
    assert!(
        moving
            .legal_actions()
            .expect("Defend actions")
            .into_iter()
            .all(|action| {
                action.descriptor["kind"] != "defend"
                    || action.descriptor["unitInstanceId"] != setup.distant_responder_id
            })
    );

    let mut stationary = setup.session.clone();
    let site_id = site_id(&stationary);
    accept_where(&mut stationary, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_id
    });
    let (stationary_defend, _) = accept_where(&mut stationary, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == setup.target_id
            && descriptor["path"]
                .as_array()
                .is_some_and(|path| path.len() == 1)
    });

    let mut intercept = setup.session;
    accept_where(&mut intercept, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let (intercept_action, _) = accept_where(&mut intercept, |descriptor| {
        descriptor["kind"] == "intercept" && descriptor["unitInstanceId"] == setup.target_id
    });

    assert_eq!(
        (stationary_defend["path"].clone(), intercept_action),
        (
            json!([{ "cell": "C2", "region": "surface" }]),
            json!({ "kind": "intercept", "unitInstanceId": setup.target_id }),
        )
    );
    exact_replay(&moving);
    exact_replay(&stationary);
    exact_replay(&intercept);
}

#[test]
fn immobile_should_defend_and_attack_in_place_without_moving() {
    let mut immobile = minion();
    immobile["connectsTopBottom"] = json!(true);
    immobile["immobile"] = json!(true);
    immobile["movementBonus"] = json!(2);
    let setup = attack_checkpoint(146, &immobile);

    let mut moving_defend = setup.session.clone();
    accept_where(&mut moving_defend, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.target_id
    });
    assert!(
        moving_defend
            .legal_actions()
            .expect("Defend actions")
            .into_iter()
            .all(|action| {
                action.descriptor["kind"] != "defend"
                    || action.descriptor["unitInstanceId"] != setup.distant_responder_id
            })
    );

    let mut stationary_defend = setup.session.clone();
    let site_id = site_id(&stationary_defend);
    accept_where(&mut stationary_defend, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_id
    });
    let (defend, _) = accept_where(&mut stationary_defend, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == setup.target_id
            && descriptor["path"]
                .as_array()
                .is_some_and(|path| path.len() == 1)
    });

    let mut local_attack = setup.session;
    accept_where(&mut local_attack, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut local_attack, |descriptor| {
        descriptor["kind"] == "close-intercept"
    });
    accept_where(&mut local_attack, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut local_attack, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let movement_actions: Vec<Value> = local_attack
        .legal_actions()
        .expect("main actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == setup.target_id
        })
        .map(|action| action.descriptor)
        .collect();
    assert_eq!(movement_actions.len(), 1);
    let (movement, _) = accept_where(&mut local_attack, |descriptor| {
        descriptor == &movement_actions[0]
    });
    accept_where(&mut local_attack, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.attacker_id
    });

    assert_eq!(
        (defend["path"].clone(), movement["path"].clone()),
        (
            json!([{ "cell": "C2", "region": "surface" }]),
            json!([{ "cell": "C2", "region": "surface" }]),
        )
    );
    exact_replay(&moving_defend);
    exact_replay(&stationary_defend);
    exact_replay(&local_attack);
}

#[test]
fn cannot_defend_or_intercept_should_exclude_both_response_actions() {
    let mut prohibited = minion();
    prohibited["cannotDefendOrIntercept"] = json!(true);
    let setup = attack_checkpoint(112, &prohibited);

    let mut defend = setup.session.clone();
    let site_id = site_id(&defend);
    accept_where(&mut defend, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_id
    });
    assert!(
        defend
            .legal_actions()
            .expect("Defend actions")
            .into_iter()
            .all(|action| action.descriptor["kind"] != "defend")
    );

    let mut intercept = setup.session;
    let (_, declined) = accept_where(&mut intercept, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(
        (
            state(&intercept)["phase"].clone(),
            declined
                .events
                .iter()
                .map(|event| event.event_type.as_str())
                .collect::<Vec<_>>(),
        ),
        (json!("main"), vec!["attack-declined"])
    );
    exact_replay(&defend);
    exact_replay(&intercept);
}

#[test]
fn forward_only_should_issue_exact_forward_defend_path() {
    let mut phalanx = minion();
    phalanx["connectsTopBottom"] = json!(true);
    phalanx["movementBonus"] = json!(1);
    phalanx["movesOnlyForward"] = json!(true);
    let setup = attack_checkpoint(141, &phalanx);
    let mut session = setup.session;
    let site_id = site_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "site"
            && descriptor["target"]["instanceId"] == site_id
    });
    let (defend, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == setup.distant_responder_id
            && descriptor["path"]
                == json!([
                    { "cell": "C1", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    });

    assert_eq!(
        defend,
        json!({
            "from": { "cell": "C1", "region": "surface" },
            "kind": "defend",
            "path": [
                { "cell": "C1", "region": "surface" },
                { "cell": "C2", "region": "surface" },
            ],
            "to": { "cell": "C2", "region": "surface" },
            "unitInstanceId": setup.distant_responder_id,
        })
    );
    exact_replay(&session);
}

#[test]
fn decline_should_offer_only_colocated_ready_interceptor_and_close_into_fight() {
    let setup = attack_checkpoint(59, &minion());
    let mut session = setup.session;
    let (_, declined) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let interceptors: Vec<Value> = session
        .legal_actions()
        .expect("Intercept actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "intercept")
        .map(|action| action.descriptor)
        .collect();
    assert_eq!(
        (
            state(&session)["phase"].clone(),
            state(&session)["decisionSeat"].clone(),
            interceptors.clone(),
        ),
        (
            json!("intercept"),
            json!("south"),
            vec![json!({ "kind": "intercept", "unitInstanceId": setup.target_id })],
        )
    );
    let (_, joined) = accept_where(&mut session, |descriptor| descriptor == &interceptors[0]);
    let (_, closed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-intercept"
    });
    let final_state = state(&session);
    let living_ids: Vec<&str> = final_state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter_map(|unit| unit["instanceId"].as_str())
        .collect();
    let event_types: Vec<&str> = declined
        .events
        .iter()
        .chain(&joined.events)
        .chain(&closed.events)
        .map(|event| event.event_type.as_str())
        .collect();

    assert!(
        !living_ids.contains(&setup.attacker_id.as_str())
            && !living_ids.contains(&setup.target_id.as_str())
            && living_ids.contains(&setup.distant_responder_id.as_str())
            && !event_types.contains(&"attack-declared")
            && event_types.contains(&"attack-declined")
            && event_types.contains(&"interceptor-joined")
            && event_types.contains(&"intercept-window-closed")
            && event_types.contains(&"fight-started"),
        "only the co-located ready unit must Intercept and fight"
    );
    exact_replay(&session);
}
