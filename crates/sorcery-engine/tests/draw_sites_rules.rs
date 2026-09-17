//! Direct proofs for targetless draw-site Magic (RULE-CATALOG-0647–0648).
//!
//! Draw-site Magic pays, draws the printed number of hidden Atlas cards in
//! deck order, and enters its owner's cemetery. Drawing from an empty Atlas
//! loses immediately after any partial draws.

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

fn draw_site() -> Value {
    json!({
        "cardType": "magic",
        "drawSites": 2,
        "manaCost": 1,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_sites_manifest(seed: u32, atlas_count: usize) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "draw-sites-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-draw-sites-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_site(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; atlas_count],
                "avatar": "north-avatar",
                "spellbook": vec!["north-draw"; 6],
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
    let mut session = Session::new(encoded).expect("valid draw-sites session");
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

#[test]
fn rule_catalog_0647_draw_sites_magic_draws_hidden_atlas_cards() {
    let encoded = draw_sites_manifest(647, 6);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let expected: Vec<_> = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .iter()
        .take(2)
        .map(|card| card["instanceId"].clone())
        .collect();
    assert_eq!(expected.len(), 2);
    let south_before = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_before["players"]["north"]["hand"]["atlas"], 2);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    });
    let spell_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("draw Magic identity")
        .to_owned();
    assert!(descriptor.get("target").is_none());
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-drawn", "site-drawn", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], spell_id);
    let after = state(&session);
    let hand = after["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand");
    assert_eq!(hand.len(), 4);
    for instance_id in &expected {
        assert!(
            hand.iter().any(|card| &card["instanceId"] == instance_id),
            "the drawn identities must enter the hidden Atlas hand"
        );
    }
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    let south_after = session
        .public_view(Seat::South)
        .expect("South public view after the draws");
    assert_eq!(
        south_after["players"]["north"]["hand"]["atlas"], 4,
        "the opponent sees only the new Atlas hand count"
    );
    assert_eq!(south_after["players"]["north"]["atlasCount"], 1);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("draw-sites checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized draw-sites");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed draw-sites");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed draw-sites session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0648_draw_sites_magic_exhausts_then_loses_on_empty_library() {
    for remaining in 0..=1 {
        let encoded = draw_sites_manifest(
            648 + u32::try_from(remaining).expect("small remaining count"),
            3 + remaining,
        );
        let mut session = opening_main(&encoded);
        let before = state(&session);
        let expected = before["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .clone();
        assert_eq!(expected.len(), remaining);
        let (_, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
        });
        let kinds = event_types(&receipt);
        assert_eq!(kinds.first(), Some(&"magic-cast"));
        assert_eq!(
            kinds.iter().filter(|kind| **kind == "site-drawn").count(),
            expected.len()
        );
        assert_eq!(kinds.last(), Some(&"game-ended"));
        assert!(kinds.contains(&"magic-resolved"));
        let after = state(&session);
        assert_eq!(after["players"]["north"]["atlas"], json!([]));
        assert_eq!(after["terminal"]["reason"], "deck_empty");
        assert_eq!(after["terminal"]["loser"], "north");
        for card in expected {
            assert!(
                after["players"]["north"]["hand"]["atlas"]
                    .as_array()
                    .expect("north Atlas hand")
                    .contains(&card)
            );
        }
        assert_exact_replay(&session);
    }
}
