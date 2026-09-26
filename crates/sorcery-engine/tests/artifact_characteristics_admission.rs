//! Admission proofs for retained artifact characteristics.

use serde_json::json;
use sorcery_engine::deck::Rarity;
use sorcery_engine::facts::{CardFacts, Element, parse_card_definition};

fn artifact() -> serde_json::Value {
    json!({
        "cardType": "artifact",
        "elements": ["earth", "air"],
        "grantsBearerPower": 2,
        "manaCost": 3,
        "rarity": "ordinary",
        "subtypes": ["Device", "Weapon"],
        "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0}
    })
}

#[test]
fn artifact_characteristics_are_preserved_in_facts() {
    let CardFacts::Artifact(facts) = parse_card_definition("printed-artifact", &artifact())
        .expect("valid artifact characteristics")
    else {
        panic!("expected artifact facts");
    };
    assert_eq!(
        facts
            .elements
            .map(|elements| elements.iter().collect::<Vec<_>>()),
        Some(vec![Element::Earth, Element::Air])
    );
    assert_eq!(facts.rarity, Some(Rarity::Ordinary));
    assert_eq!(
        facts.subtypes.as_deref(),
        Some(["Device".to_owned(), "Weapon".to_owned()].as_slice())
    );
}

#[test]
fn artifact_characteristics_reject_null_unknown_and_noncanonical_values() {
    for rarity in [json!(null), json!("mythic"), json!(4)] {
        let mut card = artifact();
        card["rarity"] = rarity;
        assert!(parse_card_definition("invalid-rarity", &card).is_err());
    }

    let mut duplicate_elements = artifact();
    duplicate_elements["elements"] = json!(["earth", "earth"]);
    assert!(parse_card_definition("duplicate-elements", &duplicate_elements).is_err());

    let mut unsorted_subtypes = artifact();
    unsorted_subtypes["subtypes"] = json!(["Weapon", "Device"]);
    assert!(parse_card_definition("unsorted-subtypes", &unsorted_subtypes).is_err());
}

#[test]
fn artifact_characteristics_are_optional_for_legacy_facts() {
    let card = json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": null,
        "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        "token": true
    });
    let CardFacts::Artifact(facts) =
        parse_card_definition("legacy-token", &card).expect("legacy token remains valid")
    else {
        panic!("expected artifact facts");
    };
    assert_eq!(facts.elements, None);
    assert_eq!(facts.rarity, None);
    assert_eq!(facts.subtypes, None);
}
