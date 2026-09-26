//! Token cost and deck-admission proofs for authored token programs.

use serde_json::{Value, json};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::facts::{CardFacts, Element, ElementSet, parse_card_definition};
use sorcery_engine::game::Game;
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::selfplay_manifest_with;

fn token_minion(mana_cost: &Value) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": mana_cost,
        "token": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn normal_minion(mana_cost: &Value) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": mana_cost,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn summon_magic(token_id: &str) -> Value {
    json!({
        "cardType": "magic",
        "effectProgram": { "effects": [{
            "op": "summon-token",
            "token": token_id,
            "count": 1,
            "destination": "source",
        }] },
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn retain_north_spell_one(manifest: &mut Value) {
    manifest["cards"]
        .as_object_mut()
        .expect("cards")
        .retain(|card_id, _| !card_id.starts_with("north-spell-") || card_id == "north-spell-1");
    manifest["decks"]["north"]["spellbook"] = json!(vec!["north-spell-1"; 50]);
}

fn summon_manifest(seed: u32) -> String {
    selfplay_manifest_with(seed, |manifest| {
        retain_north_spell_one(manifest);
        manifest["cards"]["north-spell-1"] = summon_magic("null-token");
        manifest["cards"]["null-token"] = token_minion(&Value::Null);
    })
}

fn token_deck_manifest(seed: u32) -> String {
    selfplay_manifest_with(seed, |manifest| {
        manifest["cards"]
            .as_object_mut()
            .expect("cards")
            .retain(|card_id, _| !card_id.starts_with("north-spell-"));
        manifest["cards"]["null-token"] = token_minion(&Value::Null);
        manifest["decks"]["north"]["spellbook"] = json!(vec!["null-token"; 50]);
    })
}

fn normal_zero_manifest(seed: u32) -> String {
    selfplay_manifest_with(seed, |manifest| {
        retain_north_spell_one(manifest);
        manifest["cards"]["north-spell-1"] = normal_minion(&json!(0));
    })
}

fn invalid_cost_manifest(seed: u32, missing: bool) -> String {
    selfplay_manifest_with(seed, |manifest| {
        retain_north_spell_one(manifest);
        let card = &mut manifest["cards"]["north-spell-1"];
        card["cardType"] = json!("minion");
        card.as_object_mut().expect("minion card").remove("token");
        if missing {
            card.as_object_mut()
                .expect("minion card")
                .remove("manaCost");
        } else {
            card["manaCost"] = Value::Null;
        }
    })
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
        panic!("engine-issued action was rejected");
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

fn opening_main(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid token characteristic manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay state")["state"].clone()
}

fn assert_checkpoint_and_replay(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("checkpoint");
    let encoded = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let restored =
        resume_game_checkpoint(&parse_game_checkpoint(&encoded).expect("parsed checkpoint"))
            .expect("restored checkpoint");
    assert_eq!(
        restored.replay_value().expect("restored replay"),
        session.replay_value().expect("replay")
    );

    let action_ids = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect::<Vec<_>>();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed state"),
        session.replay_value().expect("state")
    );
    assert_eq!(replayed.transcript(), session.transcript());
}

#[test]
fn null_cost_token_dependency_enters_with_zero_mana_paid() {
    let mut session = opening_main(&summon_manifest(7201));
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-spell-1"
    });
    let summoned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-summoned")
        .expect("token entry");
    assert_eq!(summoned.payload["cardId"], "null-token");
    assert_eq!(summoned.payload["token"], true);
    assert_eq!(summoned.payload["manaPaid"], 0);

    let token_id = summoned.payload["instanceId"]
        .as_str()
        .expect("token identity");
    let after = state(&session);
    let token = after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == token_id)
        .expect("entered token");
    assert_eq!(token["source"], "token");
    assert_checkpoint_and_replay(&session);
}

#[test]
fn token_cost_variants_are_admitted_or_rejected_by_card_characteristics() {
    Game::from_manifest_json(&summon_manifest(7202)).expect("null-cost token dependency admitted");

    let token_deck = token_deck_manifest(7203);
    let error = Game::from_manifest_json(&token_deck).expect_err("token spellbook card admitted");
    assert!(
        error
            .to_string()
            .contains("spellbook references an unsupported token spell")
    );

    for (seed, missing) in [(7204, false), (7205, true)] {
        let error = Game::from_manifest_json(&invalid_cost_manifest(seed, missing))
            .expect_err("non-token minion requires a printed cost");
        assert!(error.to_string().contains("manaCost"), "{error}");
    }
}

#[test]
fn explicit_zero_cost_normal_minion_remains_castable() {
    let mut session = opening_main(&normal_zero_manifest(7206));
    assert!(
        session
            .legal_actions()
            .expect("legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-spell-1"
                    && action.descriptor["cell"] == "C4"
            })
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-spell-1"
            && descriptor["cell"] == "C4"
    });
    let summoned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-summoned")
        .expect("normal minion entry");
    assert_eq!(summoned.payload["cardId"], "north-spell-1");
    assert_ne!(summoned.payload["token"], true);
    assert_checkpoint_and_replay(&session);
}

#[test]
fn minion_identity_retains_water_elements_and_derives_undead_from_subtypes() {
    let mut frog = normal_minion(&json!(0));
    frog["elements"] = json!(["water"]);
    frog["subtypes"] = json!(["Frog", "Undead"]);
    let CardFacts::Minion(facts) =
        parse_card_definition("synthetic-water-frog", &frog).expect("water Frog facts")
    else {
        panic!("expected minion facts");
    };

    assert_eq!(facts.elements, Some(ElementSet::only(Element::Water)));
    assert_eq!(facts.thresholds.canonical(), [0, 0, 0, 0]);
    assert!(facts.undead);
    assert!(!facts.mortal);
    assert!(!facts.demon);
    assert_eq!(
        facts.subtypes.as_deref().map(<[String]>::to_vec),
        Some(vec!["Frog".to_owned(), "Undead".to_owned()])
    );
}

#[test]
fn missing_and_explicit_empty_identity_metadata_remain_distinct() {
    let CardFacts::Minion(unspecified) =
        parse_card_definition("unspecified-identity", &normal_minion(&json!(0)))
            .expect("unspecified identity")
    else {
        panic!("expected minion facts");
    };
    assert_eq!(unspecified.elements, None);
    assert_eq!(unspecified.subtypes, None);

    let mut empty = normal_minion(&json!(0));
    empty["elements"] = json!([]);
    empty["subtypes"] = json!([]);
    let CardFacts::Minion(explicit_empty) =
        parse_card_definition("empty-identity", &empty).expect("explicit empty identity")
    else {
        panic!("expected minion facts");
    };
    assert_eq!(explicit_empty.elements, Some(ElementSet::empty()));
    assert!(
        explicit_empty
            .subtypes
            .as_deref()
            .is_some_and(<[String]>::is_empty)
    );
}

#[test]
fn identity_lists_require_canonical_unique_order_and_agree_with_legacy_flags() {
    for elements in [json!(["water", "water"]), json!(["water", "earth"])] {
        let mut card = normal_minion(&json!(0));
        card["elements"] = elements;
        let error = parse_card_definition("bad-elements", &card).expect_err("invalid elements");
        assert!(
            error
                .to_string()
                .contains("must contain unique elements in canonical order"),
            "{error}"
        );
    }

    for subtypes in [json!(["Undead", "Undead"]), json!(["Undead", "Frog"])] {
        let mut card = normal_minion(&json!(0));
        card["subtypes"] = subtypes;
        let error = parse_card_definition("bad-subtypes", &card).expect_err("invalid subtypes");
        assert!(
            error
                .to_string()
                .contains("must contain unique subtypes in sorted order"),
            "{error}"
        );
    }

    for subtype in ["\u{feff}Beast", "Beast\u{feff}"] {
        let mut card = normal_minion(&json!(0));
        card["subtypes"] = json!([subtype]);
        let error = parse_card_definition("untrimmed-subtype", &card)
            .expect_err("subtype edge whitespace must match the JSON boundary");
        assert!(error.to_string().contains("subtypes"), "{error}");
    }

    let mut contradictory = normal_minion(&json!(0));
    contradictory["subtypes"] = json!(["Beast"]);
    contradictory["undead"] = json!(true);
    let error = parse_card_definition("contradictory-undead", &contradictory)
        .expect_err("contradictory legacy undead flag");
    assert!(
        error
            .to_string()
            .contains("must agree with explicit subtypes"),
        "{error}"
    );
}
