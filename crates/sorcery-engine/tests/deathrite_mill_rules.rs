//! Direct proofs for Deathrite library mill (RULE-CATALOG-0300–0303).
//!
//! Official minions can mill a public library card when they die. The mill
//! reuses the shared mill helper, resolves before cemetery entry, and is a
//! no-op on an empty library. Unlike Deathrite draws, an empty mill never
//! decks the controller out.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn source(mill_spells: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "diesAtEndOfControllerTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if mill_spells {
        value["deathriteMillSpells"] = json!(true);
    } else {
        value["deathriteMillSites"] = json!(true);
    }
    value
}

fn manifest(seed: u32, mill_spells: bool, north_atlas: usize, north_spellbook: usize) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "deathrite-mill" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-deathrite-mill-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": source(mill_spells),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; north_atlas],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; north_spellbook],
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

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn after_north_ready_to_end(
    seed: u32,
    mill_spells: bool,
    north_atlas: usize,
    north_spellbook: usize,
) -> Session {
    let mut session = Session::new(&manifest(seed, mill_spells, north_atlas, north_spellbook))
        .expect("valid Deathrite mill session");
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

fn assert_checkpoint(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("Deathrite mill checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized mill");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed mill");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed mill session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

fn assert_deathrite_mill(session: &mut Session, zone: &str, event_type: &str, card_id: &str) {
    let before = state(session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("Deathrite source")["instanceId"]
        .clone();
    let library = before["players"]["north"][zone]
        .as_array()
        .expect("north library");
    let milled_id = library.first().expect("next library card")["instanceId"].clone();
    let library_before = library.len();
    let cemetery_before = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let (_, receipt) = accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    let types = event_types(&receipt);
    let milled = types
        .iter()
        .position(|kind| *kind == event_type)
        .expect("Deathrite mill");
    let died = types
        .iter()
        .position(|kind| *kind == "minion-died")
        .expect("Deathrite corpse");
    assert!(milled < died);
    assert_eq!(receipt.events[milled].payload["cardId"], card_id);
    assert_eq!(receipt.events[milled].payload["instanceId"], milled_id);
    assert_eq!(receipt.events[milled].payload["seat"], "north");
    assert_eq!(
        receipt.events[milled].payload["sourceInstanceId"],
        source_id
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "game-ended")
    );
    let after = state(session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["decisionSeat"], "south");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"][zone]
            .as_array()
            .expect("north library")
            .len(),
        library_before - 1
    );
    let cemetery = after["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery");
    assert_eq!(cemetery.len(), cemetery_before + 2);
    assert!(cemetery.iter().any(|card| card["instanceId"] == milled_id));
    assert!(cemetery.iter().any(|card| card["instanceId"] == source_id));
    assert_exact_replay(session);
    assert_checkpoint(session);
}

fn assert_deathrite_mill_empty(session: &mut Session, zone: &str, event_type: &str) {
    let before = state(session);
    assert_eq!(before["players"]["north"][zone], json!([]));
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("Deathrite source")["instanceId"]
        .clone();
    let cemetery_before = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let (_, receipt) = accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == event_type || event.event_type == "game-ended")
    );
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died")
    );
    let after = state(session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["decisionSeat"], "south");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(after["players"]["north"][zone], json!([]));
    let cemetery = after["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery");
    assert_eq!(cemetery.len(), cemetery_before + 1);
    assert!(cemetery.iter().any(|card| card["instanceId"] == source_id));
    assert_exact_replay(session);
    assert_checkpoint(session);
}

#[test]
fn rule_catalog_0300_deathrite_mill_spells_puts_the_top_spell_in_the_cemetery() {
    let mut session = after_north_ready_to_end(300, true, 6, 6);
    assert_deathrite_mill(&mut session, "spellbook", "spell-discarded", "north-source");
}

#[test]
fn rule_catalog_0301_deathrite_mill_spells_is_a_no_op_on_an_empty_library() {
    let mut session = after_north_ready_to_end(301, true, 6, 3);
    assert_deathrite_mill_empty(&mut session, "spellbook", "spell-discarded");
}

#[test]
fn rule_catalog_0302_deathrite_mill_sites_puts_the_top_site_in_the_cemetery() {
    let mut session = after_north_ready_to_end(302, false, 6, 6);
    assert_deathrite_mill(&mut session, "atlas", "site-discarded", "north-site");
}

#[test]
fn rule_catalog_0303_deathrite_mill_sites_is_a_no_op_on_an_empty_atlas() {
    let mut session = after_north_ready_to_end(303, false, 3, 6);
    assert_deathrite_mill_empty(&mut session, "atlas", "site-discarded");
}
