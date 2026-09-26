//! Admission proofs for composable carried-artifact unit-strike modifiers.

use serde_json::json;
use sorcery_engine::facts::{ArtifactEffect, BearerUnitStrike, CardFacts, parse_card_definition};

fn artifact(fact: serde_json::Value) -> serde_json::Value {
    let mut card = json!({
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
    });
    card.as_object_mut()
        .expect("artifact object")
        .insert("bearerUnitStrike".to_owned(), fact);
    card
}

fn artifact_facts(card: &serde_json::Value) -> sorcery_engine::facts::ArtifactFacts {
    let CardFacts::Artifact(facts) =
        parse_card_definition("bearer-strike", card).expect("valid bearer unit-strike facts")
    else {
        panic!("expected artifact facts");
    };
    facts
}

#[test]
fn bearer_unit_strike_accepts_each_composable_modifier_and_passive_fallback() {
    let damage = artifact_facts(&artifact(json!({"damageBonus": 3})));
    assert_eq!(damage.effect, ArtifactEffect::PassiveModifiers);
    assert_eq!(
        damage.bearer_unit_strike,
        Some(BearerUnitStrike {
            damage_bonus: 3,
            first_strike: false,
            destroy_after_strike: false,
        })
    );

    let first = artifact_facts(&artifact(json!({"firstStrike": true})));
    assert_eq!(
        first.bearer_unit_strike,
        Some(BearerUnitStrike {
            damage_bonus: 0,
            first_strike: true,
            destroy_after_strike: false,
        })
    );

    let all = artifact_facts(&artifact(json!({
        "damageBonus": 1,
        "firstStrike": true,
        "destroyAfterStrike": true
    })));
    assert_eq!(all.effect, ArtifactEffect::PassiveModifiers);
    let strike = all.bearer_unit_strike.expect("all bearer modifiers");
    assert_eq!(strike.damage_bonus, 1);
    assert!(strike.first_strike);
    assert!(strike.destroy_after_strike);
}

#[test]
fn bearer_unit_strike_is_orthogonal_to_existing_legacy_effects() {
    let card = {
        let mut card = artifact(json!({"damageBonus": 1, "firstStrike": true}));
        card["grantsBearerPower"] = json!(2);
        card
    };
    let facts = artifact_facts(&card);
    assert_eq!(facts.effect, ArtifactEffect::GrantsBearerPowerTwo);
    assert!(facts.bearer_unit_strike.is_some());

    let card = {
        let mut card = artifact(json!({"destroyAfterStrike": true}));
        card["nearbyStrikesAgainstUnitsDealDoubleDamage"] = json!(true);
        card
    };
    let facts = artifact_facts(&card);
    assert_eq!(
        facts.effect,
        ArtifactEffect::NearbyStrikesAgainstUnitsDealDoubleDamage
    );
    assert!(facts.bearer_unit_strike.unwrap().destroy_after_strike);
}

#[test]
fn bearer_unit_strike_rejects_empty_unknown_false_null_and_malformed_values() {
    let invalid = [
        (json!({}), "at least one bearer unit-strike modifier"),
        (json!({"unknown": true}), "is unsupported"),
        (json!({"firstStrike": false}), "must be true when defined"),
        (
            json!({"destroyAfterStrike": null}),
            "must be true when defined",
        ),
        (json!({"damageBonus": 0}), "must be between 1"),
        (
            json!({"damageBonus": "1"}),
            "must be a supported safe integer",
        ),
        (json!(true), "must be an object"),
        (json!(null), "must be an object"),
    ];
    for (value, expected) in invalid {
        let error = parse_card_definition("invalid-bearer-strike", &artifact(value))
            .expect_err("invalid bearer unit-strike facts admitted")
            .to_string();
        assert!(
            error.contains(expected),
            "expected {expected:?}, got {error}"
        );
    }
}
