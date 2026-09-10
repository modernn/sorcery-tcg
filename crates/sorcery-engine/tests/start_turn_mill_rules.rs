//! Direct proofs for start-of-controller-turn library mill (RULE-CATALOG-0292–0295).
//!
//! Official minions can mill the controller's Spellbook or Atlas at the start
//! of that player's turn. The trigger shares the Start Phase window with other
//! start-of-turn sources, reuses the shared mill helper, and remains a no-op
//! when the library is empty. Empty mill is never a deck-out.

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

fn miller(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra miller facts must be an object");
    };
    value.as_object_mut().expect("miller facts").extend(extra);
    value
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

fn manifest(seed: u32, mill: Value, north_atlas: &[&str], north_spellbook: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-mill" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-mill-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-miller": miller(mill),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": north_atlas,
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

fn after_north_second_start(
    seed: u32,
    mill: Value,
    north_atlas: &[&str],
    north_spellbook: &[&str],
) -> Session {
    let mut session =
        Session::new(&manifest(seed, mill, north_atlas, north_spellbook)).expect("valid mill session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-miller"
            && descriptor["cell"] == "C4"
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
fn rule_catalog_0292_start_turn_mill_spells_puts_the_top_spell_in_the_cemetery() {
    let mut session = after_north_second_start(
        292,
        json!({ "atStartOfControllerTurnMillSpells": 1 }),
        &["north-site"; 6],
        &["north-miller"; 6],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-miller")
        .expect("miller")["instanceId"]
        .clone();
    let milled_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("next spell")["instanceId"]
        .clone();
    let library_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    let cemetery_before = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let triggers: Vec<_> = session
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].descriptor["sourceInstanceId"], source_id);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["spell-discarded"]);
    assert_eq!(receipt.events[0].payload["seat"], "north");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert_eq!(receipt.events[0].payload["instanceId"], milled_id);
    assert_eq!(receipt.events[0].payload["cardId"], "north-miller");
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        library_before - 1
    );
    let cemetery = after["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery");
    assert_eq!(cemetery.len(), cemetery_before + 1);
    assert!(cemetery.iter().any(|card| card["instanceId"] == milled_id));
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("start-turn mill checkpoint");
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

#[test]
fn rule_catalog_0293_start_turn_mill_spells_is_a_no_op_on_an_empty_library() {
    let mut session = after_north_second_start(
        293,
        json!({ "atStartOfControllerTurnMillSpells": 1 }),
        &["north-site"; 6],
        &["north-miller"; 3],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    assert_eq!(state(&session)["players"]["north"]["spellbook"], json!([]));
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-miller")
        .expect("miller")["instanceId"]
        .clone();
    let cemetery_before = state(&session)["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), [] as [&str; 0]);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        cemetery_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0294_start_turn_mill_sites_puts_the_top_site_in_the_cemetery() {
    let mut session = after_north_second_start(
        294,
        json!({ "atStartOfControllerTurnMillSites": 1 }),
        &["north-site"; 6],
        &["north-miller"; 6],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-miller")
        .expect("miller")["instanceId"]
        .clone();
    let milled_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .first()
        .expect("next site")["instanceId"]
        .clone();
    let library_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["site-discarded"]);
    assert_eq!(receipt.events[0].payload["instanceId"], milled_id);
    assert_eq!(receipt.events[0].payload["cardId"], "north-site");
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        library_before - 1
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == milled_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0295_start_turn_mill_sites_is_a_no_op_on_an_empty_atlas() {
    let mut session = after_north_second_start(
        295,
        json!({ "atStartOfControllerTurnMillSites": 1 }),
        &["north-site"; 3],
        &["north-miller"; 6],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    assert_eq!(state(&session)["players"]["north"]["atlas"], json!([]));
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-miller")
        .expect("miller")["instanceId"]
        .clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), [] as [&str; 0]);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}
