//! Direct proofs for Deathrite Spellbook draw (RULE-CATALOG-0296–0297).
//!
//! Official minions can draw a hidden spell for their controller when they
//! die. The draw reuses the shared private-draw helper, resolves before the
//! corpse enters the cemetery, redacts the identity from the opponent, and
//! loses immediately when the Spellbook is empty.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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

fn source() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSpells": true,
        "defense": 1,
        "diesAtEndOfControllerTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn manifest(seed: u32, north_spellbook: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "deathrite-draw-spells" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-deathrite-draw-spells-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": source(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
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

fn after_north_ready_to_end(seed: u32, north_spellbook: &[&str]) -> Session {
    let mut session =
        Session::new(&manifest(seed, north_spellbook)).expect("valid Deathrite session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    session
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
fn rule_catalog_0296_deathrite_draw_spells_draws_a_hidden_spell_before_cemetery_entry() {
    let mut session = after_north_ready_to_end(296, &["north-source"; 6]);
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("Deathrite source")["instanceId"]
        .clone();
    let drawn_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("next spell")["instanceId"]
        .clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let library_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    let south_before = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_before["players"]["north"]["hand"]["spellbook"],
        hand_before
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let types = event_types(&receipt);
    let drawn = types
        .iter()
        .position(|kind| *kind == "spell-drawn")
        .expect("Deathrite spell draw");
    let died = types
        .iter()
        .position(|kind| *kind == "minion-died")
        .expect("Ignited death");
    assert!(drawn < died);
    assert_eq!(receipt.events[drawn].payload["seat"], "north");
    assert_eq!(receipt.events[drawn].payload["sourceInstanceId"], source_id);
    assert!(receipt.events[drawn].payload.get("instanceId").is_none());
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_eq!(after["terminal"]["status"], "active");
    let hand = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand");
    assert_eq!(hand.len(), hand_before + 1);
    assert!(hand.iter().any(|card| card["instanceId"] == drawn_id));
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        library_before - 1
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == source_id)
    );
    let south_after = session
        .public_view(Seat::South)
        .expect("South public view after the draw");
    assert_eq!(
        south_after["players"]["north"]["hand"]["spellbook"],
        hand_before + 1
    );
    let south_json = south_after.to_string();
    assert!(
        !south_json.contains(drawn_id.as_str().expect("drawn identity")),
        "the opponent must not see the drawn identity"
    );
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("Deathrite spell-draw checkpoint");
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
fn rule_catalog_0297_deathrite_draw_spells_decks_out_on_an_empty_library() {
    let mut session = after_north_ready_to_end(297, &["north-source"; 3]);
    assert_eq!(state(&session)["players"]["north"]["spellbook"], json!([]));
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "spell-drawn")
    );
    assert!(
        receipt.events.iter().any(
            |event| event.event_type == "game-ended" && event.payload["reason"] == "deck_empty"
        )
    );
    let after = state(&session);
    assert_eq!(after["phase"], "terminal");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_eq!(after["terminal"]["loser"], "north");
    assert_eq!(after["terminal"]["status"], "finished");
    assert_exact_replay(&session);
}
