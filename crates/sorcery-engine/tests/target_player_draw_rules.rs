//! Direct proofs for target-player Spellbook draw (RULE-CATALOG-0286–0287).
//!
//! Official Magic can make a chosen player draw from that player's Spellbook.
//! Drawn identities stay private. An empty library is a deck-out, not a paid
//! no-op — the opposite of mill.

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
    json!({ "cardType": "site", "elements": ["earth"] })
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

fn draw_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "targetPlayerDrawsSpells": 1,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32, south_spellbook: usize) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "target-player-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-target-player-draw-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-draw"; 6],
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid target-player-draw session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn draw_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("draw actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-draw"
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

#[test]
fn rule_catalog_0286_target_player_draw_puts_opponent_library_card_in_hand() {
    let encoded = manifest(286, 6);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let library = before["players"]["south"]["spellbook"]
        .as_array()
        .expect("south library");
    assert_eq!(library.len(), 3);
    let drawn_id = library[0]["instanceId"].clone();
    assert_eq!(
        before["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("south hand")
            .len(),
        3
    );
    assert_eq!(
        draw_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-draw"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    let drawn = &receipt.events[1];
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(
        drawn.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(drawn.payload.get("cardId").is_none());
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(drawn_id.as_str().expect("drawn identity"))
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "game-ended" || event.event_type == "spell-discarded")
    );

    let after = state(&session);
    assert_eq!(
        after["players"]["south"]["spellbook"]
            .as_array()
            .expect("remaining")
            .len(),
        2
    );
    let hand = after["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("south hand");
    assert_eq!(hand.len(), 4);
    assert!(hand.iter().any(|card| card["instanceId"] == drawn_id));
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 4);
    assert_eq!(north_view["players"]["south"]["spellbookCount"], 2);
    assert!(
        !serde_json::to_string(&north_view)
            .expect("view JSON")
            .contains(drawn_id.as_str().expect("drawn identity"))
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("target-player-draw checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized draw");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed draw");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed draw session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0287_target_player_draw_decks_out_an_empty_library() {
    let encoded = manifest(287, 3);
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["south"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-draw"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "magic-resolved", "game-ended"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = receipt
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "south");
    assert_eq!(ended.payload["winner"], "north");
    let after = state(&session);
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_eq!(after["terminal"]["loser"], "south");
    assert_eq!(after["terminal"]["winner"], "north");
    assert_exact_replay(&session);
}
