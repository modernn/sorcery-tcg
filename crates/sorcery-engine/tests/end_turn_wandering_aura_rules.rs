//! Direct proofs for end-of-each-turn wandering Auras (RULE-CATALOG-0262–0263)
//! and resolve-end-turn-aura-move withheld during trigger-order
//! (RULE-CATALOG-1156).
//!
//! Official cards such as Wildfire conjure atop a single nearby site. At the
//! end of each turn, each unit there takes damage, then the Aura must move to
//! an adjacent location it has not visited. When no unvisited adjacent cell
//! remains, it dispels.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn aura() -> Value {
    json!({
        "atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent": 3,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 10,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn encoded_manifest(fixture: &str, south_minion: &Value) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-aura": aura(),
            "north-avatar": avatar(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": south_minion.clone(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 12],
                "avatar": "north-avatar",
                "spellbook": vec!["north-aura"; 12],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 12],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest() -> String {
    encoded_manifest("end-turn-wandering-aura", &minion())
}

fn deathrite_manifest() -> String {
    encoded_manifest("end-turn-wandering-aura-deathrite", &deathrite_minion())
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn aura_id(session: &Session) -> Value {
    state(session)["realm"]["auras"][0]["instanceId"].clone()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
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

fn move_wildfire(session: &mut Session, cell: &str) -> Receipt {
    assert_eq!(state(session)["phase"], "end-turn-aura");
    let legal = session.legal_actions().expect("wandering Aura moves");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "resolve-end-turn-aura-move"
                && action.descriptor.get("cells").is_some()
        }),
        "wandering Aura moves are mandatory"
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-end-turn-aura-move" && descriptor["cells"] == json!([cell])
    })
    .1
}

fn draw_then_end(session: &mut Session) -> Receipt {
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn").1
}

fn summon_deathrite_at_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })
    .0["cardInstanceId"]
        .as_str()
        .expect("Deathrite identity")
        .to_owned()
}

#[test]
fn rule_catalog_0262_wandering_aura_damages_each_unit_then_must_move() {
    let mut session = Session::new(&manifest()).expect("valid wandering Aura session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    assert!(state(&session)["realm"].get("immobileAreas").is_none());
    let source_id = aura_id(&session);
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"].clone();
    let ended = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn").1;
    assert!(ended.events.iter().any(|event| {
        event.event_type == "aura-end-turn-damage-allocated"
            && event.payload["amount"] == 3
            && event.payload["targetInstanceId"] == avatar_id
            && event.payload["sourceInstanceId"] == source_id
    }));
    assert_eq!(
        ended
            .events
            .iter()
            .filter(|event| event.event_type == "aura-end-turn-damage-allocated")
            .count(),
        1
    );
    let moved = move_wildfire(&mut session, "C3");
    assert!(moved.events.iter().any(|event| {
        event.event_type == "aura-moved"
            && event.payload["cells"] == json!(["C3"])
            && event.payload["instanceId"] == source_id
    }));
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_eq!(after["players"]["north"]["avatar"]["life"], 17);
    assert_eq!(after["realm"]["auras"][0]["cells"], json!(["C3"]));
    assert_eq!(
        after["realm"]["auras"][0]["visitedCells"],
        json!(["C3", "C4"])
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0263_far_sites_are_illegal_and_each_turn_can_box_the_aura_out() {
    let mut session = Session::new(&manifest()).expect("valid wandering Aura session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    let legal_cells = session
        .legal_actions()
        .expect("nearby conjure actions")
        .into_iter()
        .filter_map(|action| {
            (action.descriptor["kind"] == "cast-aura"
                && action.descriptor["cardId"] == "north-aura")
                .then(|| action.descriptor["cells"][0].clone())
        })
        .collect::<Vec<_>>();
    assert!(legal_cells.contains(&json!("C4")));
    assert!(!legal_cells.contains(&json!("C1")));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    move_wildfire(&mut session, "C3");
    let south_trigger = draw_then_end(&mut session);
    assert!(
        south_trigger
            .events
            .iter()
            .any(|event| event.event_type == "aura-end-turn-triggered"),
        "the Aura triggers at the end of each turn"
    );
    move_wildfire(&mut session, "B3");
    for cell in ["A3", "A4", "B4"] {
        draw_then_end(&mut session);
        move_wildfire(&mut session, cell);
    }
    let dispelled = draw_then_end(&mut session);
    assert!(
        dispelled
            .events
            .iter()
            .any(|event| event.event_type == "aura-dispelled")
    );
    assert!(
        dispelled
            .events
            .iter()
            .all(|event| event.event_type != "aura-moved")
    );
    let after = state(&session);
    assert!(after["realm"].get("auras").is_none());
    assert_eq!(after["phase"], "draw");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1156_resolve_end_turn_aura_move_withheld_during_pending_deathrite_order() {
    let mut session = Session::new(&deathrite_manifest()).expect("valid Deathrite wandering Aura");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
    let mut deathrite_ids = [
        summon_deathrite_at_c4(&mut session),
        summon_deathrite_at_c4(&mut session),
    ];
    deathrite_ids.sort_unstable();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    let ended = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn").1;
    assert!(
        ended
            .events
            .iter()
            .any(|event| event.event_type == "aura-end-turn-triggered")
    );
    let paused = state(&session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-end-turn-aura-move"),
        "trigger-order must issue no resolve-end-turn-aura-move"
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(&session);
    assert_eq!(resumed["phase"], "end-turn-aura");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    let source_id = aura_id(&session);
    let moved = move_wildfire(&mut session, "C3");
    assert!(moved.events.iter().any(|event| {
        event.event_type == "aura-moved"
            && event.payload["cells"] == json!(["C3"])
            && event.payload["instanceId"] == source_id
    }));
    assert_eq!(state(&session)["phase"], "draw");
    assert_exact_replay(&session);
}
