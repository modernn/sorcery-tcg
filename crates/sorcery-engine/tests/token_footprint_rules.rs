//! Direct proofs for 2×2 token occupancy (RULE-CATALOG-0371–0372).

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn square_token() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "occupiesSquareArea": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    })
}

fn attacker_minion() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "charge": true,
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn genesis_site(token_id: &str) -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisPayOneManaToSummonToken": token_id,
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-footprint-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-footprint-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-gate": genesis_site("square-token"),
            "south-attacker": attacker_minion(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
            "square-token": square_token(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-gate"; 12],
                "avatar": "north-avatar",
                "spellbook": vec!["south-dummy"; 12],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-attacker"; 24],
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
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
        });
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

fn end_and_draw_zone(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn play_genesis_site(session: &mut Session, cell: &str, choice: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == cell
            && descriptor["genesisTokenChoice"] == choice
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn realm_unit<'a>(value: &'a Value, instance_id: &str) -> Option<&'a Value> {
    value["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
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

fn opening() -> Session {
    let mut session = Session::new(&manifest(371)).expect("token footprint candidate");
    keep(&mut session);
    keep(&mut session);
    session
}

fn summon_square_token(session: &mut Session) -> (String, Receipt) {
    play_genesis_site(session, "C4", "decline");
    end_and_draw_zone(session, "spellbook");
    play_site(session, "C1");
    end_and_draw_zone(session, "atlas");
    play_genesis_site(session, "B4", "decline");
    end_and_draw_zone(session, "spellbook");
    end_and_draw_zone(session, "atlas");
    play_genesis_site(session, "C3", "decline");
    end_and_draw_zone(session, "spellbook");
    end_and_draw_zone(session, "atlas");
    let (_, paid) = accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "B3"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    let token_id = paid.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity")
        .to_owned();
    (token_id, paid)
}

#[test]
fn rule_catalog_0371_square_token_occupies_its_whole_footprint() {
    let mut session = opening();
    let (token_id, paid) = summon_square_token(&mut session);
    assert_eq!(
        paid.events[1].payload["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    let current = state(&session);
    let placed = realm_unit(&current, &token_id).expect("square token");
    assert_eq!(
        (
            &placed["location"],
            &placed["region"],
            &placed["occupiedCells"]
        ),
        (
            &json!("B3"),
            &json!("surface"),
            &json!(["B3", "B4", "C3", "C4"])
        )
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0372_square_token_banishes_instead_of_entering_a_cemetery() {
    let mut session = opening();
    let (token_id, _) = summon_square_token(&mut session);
    end_and_draw_zone(&mut session, "spellbook");
    play_site(&mut session, "C2");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-attacker"
            && descriptor["cell"] == "C2"
    });
    let attacker_id = summoned["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == token_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let token_exits: Vec<_> = fight
        .events
        .iter()
        .filter(|event| event.payload["instanceId"] == token_id)
        .map(|event| event.event_type.as_str())
        .filter(|event_type| matches!(*event_type, "minion-died" | "minion-banished"))
        .collect();
    assert_eq!(token_exits, ["minion-died", "minion-banished"]);
    assert!(
        state(&session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != token_id)
    );
    assert_exact_replay(&session);
}
