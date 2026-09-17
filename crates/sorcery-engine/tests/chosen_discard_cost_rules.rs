//! Direct proofs for player-chosen additional Magic discard
//! (RULE-CATALOG-0653–0654).
//!
//! A chosen-discard cost is a Storyline choice among every other Atlas or
//! Spellbook hand card. It cannot select the spell being cast, pays
//! `card-discarded` into the owner's cemetery before the cast is announced,
//! then the companion effect resolves. An empty other-hand issues no cast.

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

fn fodder() -> Value {
    json!({
        "cardType": "magic",
        "healController": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn cost_spell() -> Value {
    json!({
        "cardType": "magic",
        "discardCardAsAdditionalCost": true,
        "drawSites": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn chosen_discard_manifest(seed: u32, north_spellbook: &[&str]) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chosen-discard-cost" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chosen-discard-cost-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cost": cost_spell(),
            "north-fodder": fodder(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
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
    let mut session = Session::new(encoded).expect("valid chosen-discard session");
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

fn north_hand_ids(snapshot: &Value, zone: &str) -> Vec<String> {
    snapshot["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("hand identity")
                .to_owned()
        })
        .collect()
}

fn north_spell_card_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn seed_with(north_spellbook: &[&str], required: &[&str], start: u32) -> String {
    (start..start + 256)
        .map(|seed| chosen_discard_manifest(seed, north_spellbook))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                let hand = north_spell_card_ids(&state(&preview));
                required.iter().all(|id| hand.iter().any(|card| card == id))
            })
        })
        .expect("bounded seed with required opening cards")
}

fn discard_cost_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("chosen-discard actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-cost"
        })
        .filter_map(|action| {
            action.descriptor["discardCardInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn offers_cost(session: &Session) -> bool {
    !discard_cost_ids(session).is_empty()
}

fn cast_all_fodder(session: &mut Session) {
    while session
        .legal_actions()
        .expect("fodder actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fodder"
        })
    {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fodder"
        });
    }
}

fn hand_card(snapshot: &Value, zone: &str, card_id: &str) -> String {
    snapshot["players"]["north"]["hand"][zone]
        .as_array()
        .expect("hand zone")
        .iter()
        .find(|card| card["cardId"] == card_id)
        .expect("required hand card")["instanceId"]
        .as_str()
        .expect("identity")
        .to_owned()
}

#[test]
fn rule_catalog_0653_chosen_discard_cost_discards_another_spell_then_resolves() {
    let encoded = seed_with(
        &["north-cost", "north-fodder", "north-fodder"],
        &["north-cost", "north-fodder"],
        653,
    );
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let cost_id = hand_card(&before, "spellbook", "north-cost");
    let fodder_id = hand_card(&before, "spellbook", "north-fodder");
    let atlas_ids = north_hand_ids(&before, "atlas");
    let drawn_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas library")
        .first()
        .expect("next site")["instanceId"]
        .clone();
    let discard_ids = discard_cost_ids(&session);
    assert!(!discard_ids.is_empty());
    assert!(discard_ids.iter().all(|id| id != &cost_id));
    assert!(discard_ids.iter().any(|id| id == &fodder_id));
    assert!(atlas_ids.iter().all(|id| discard_ids.contains(id)));
    let south_observation = session.observe(Seat::South);
    let atlas_before = atlas_ids.len();
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cost"
            && descriptor["discardCardInstanceId"] == fodder_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(receipt.events[0].payload["cardId"], "north-fodder");
    assert_eq!(receipt.events[0].payload["instanceId"], fodder_id);
    assert_eq!(receipt.events[0].payload["owner"], "north");
    assert_eq!(receipt.events[0].payload["seat"], "north");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], cost_id);
    assert_eq!(receipt.events[0].payload["zone"], "spellbook");
    assert_eq!(descriptor["discardCardInstanceId"], fodder_id);
    assert_eq!(
        receipt.events[1].payload["discardCardInstanceId"],
        fodder_id
    );
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], cost_id);
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == fodder_id)
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .iter()
            .any(|card| card["instanceId"] == drawn_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        atlas_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .iter()
            .all(|card| card["instanceId"] != fodder_id && card["instanceId"] != cost_id)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_view["players"]["north"]["hand"]["atlas"],
        atlas_before + 1
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("chosen-discard checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized chosen-discard");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed chosen-discard");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed chosen-discard session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0654_chosen_discard_cost_is_unoffered_without_another_hand_card() {
    let encoded = seed_with(
        &[
            "north-cost",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
        ],
        &["north-cost"],
        654,
    );
    let mut session = opening_main(&encoded);
    assert!(offers_cost(&session), "Atlas leftovers still pay the cost");
    cast_all_fodder(&mut session);
    assert!(offers_cost(&session));
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        if state(&session)["players"]["south"]["domainEstablished"].as_bool() != Some(true) {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
        cast_all_fodder(&mut session);
    }
    let after = state(&session);
    assert_eq!(north_hand_ids(&after, "atlas").len(), 0);
    assert_eq!(north_spell_card_ids(&after), ["north-cost".to_owned()]);
    assert!(
        !offers_cost(&session),
        "an empty other-hand must issue no chosen-discard cast"
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}
