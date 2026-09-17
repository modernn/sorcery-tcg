//! Direct proofs for mill-spell Magic (RULE-CATALOG-0627–0628).
//!
//! Mill-spell Magic offers only both Avatars and puts the printed number of
//! the chosen player's top Spellbook cards into that owner's cemetery. An
//! empty library is a paid no-op, never a deck-out.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn mill_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "millSpells": 2,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn mill_spells_manifest(seed: u32, south_spellbook: usize) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "mill-spells-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-mill-spells-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-mill": mill_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-mill"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; south_spellbook],
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
    let mut session = Session::new(encoded).expect("valid mill-spells session");
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

fn mill_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("mill actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-mill"
        })
        .filter_map(|action| {
            let target = action.descriptor.get("target")?;
            Some((
                target["kind"].as_str()?.to_owned(),
                target["seat"].as_str()?.to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn south_library(session: &Session) -> Vec<Value> {
    state(session)["players"]["south"]["spellbook"]
        .as_array()
        .expect("south library")
        .clone()
}

#[test]
fn rule_catalog_0627_mill_spells_puts_opponent_library_cards_in_the_cemetery() {
    let encoded = mill_spells_manifest(627, 6);
    let mut session = opening_main(&encoded);
    let before = south_library(&session);
    let expected: Vec<_> = before.iter().take(2).cloned().collect();
    assert_eq!(expected.len(), 2);
    assert_eq!(
        mill_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mill"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "spell-discarded",
            "spell-discarded",
            "magic-resolved"
        ]
    );
    let discarded: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "spell-discarded")
        .collect();
    assert_eq!(
        discarded[0].payload["instanceId"],
        expected[0]["instanceId"]
    );
    assert_eq!(discarded[0].payload["cardId"], "south-minion");
    assert_eq!(discarded[0].payload["seat"], "south");
    assert_eq!(
        discarded[0].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert_eq!(
        discarded[1].payload["instanceId"],
        expected[1]["instanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "game-ended"
                || event.event_type == "spell-drawn"
                || event.event_type == "damage-dealt")
    );

    let after = state(&session);
    assert_eq!(
        after["players"]["south"]["spellbook"]
            .as_array()
            .expect("remaining")
            .len(),
        1
    );
    let cemetery = after["players"]["south"]["cemetery"]
        .as_array()
        .expect("south cemetery");
    for card in &expected {
        assert!(
            cemetery
                .iter()
                .any(|entry| entry["instanceId"] == card["instanceId"])
        );
    }
    let south_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(south_view["players"]["south"]["spellbookCount"], 1);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("mill-spells checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized mill-spells");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed mill-spells");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed mill-spells session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0628_mill_spells_is_a_paid_noop_on_an_empty_library() {
    let encoded = mill_spells_manifest(628, 3);
    let mut session = opening_main(&encoded);
    assert_eq!(south_library(&session).len(), 0);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mill"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "spell-discarded" || event.event_type == "game-ended")
    );
    let after = state(&session);
    assert_eq!(after["players"]["south"]["spellbook"], json!([]));
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}
