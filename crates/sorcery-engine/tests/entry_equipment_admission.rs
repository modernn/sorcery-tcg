//! Admission proofs for bounded unsuppressible minion entry equipment facts.

use serde_json::json;
use sorcery_engine::facts::{CardFacts, parse_card_definition};

fn minion(extra: serde_json::Value) -> serde_json::Value {
    let mut card = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
    });
    let serde_json::Value::Object(extra) = extra else {
        panic!("minion extras");
    };
    card.as_object_mut().expect("minion object").extend(extra);
    card
}

#[test]
fn enters_carrying_is_absent_or_preserves_order_and_duplicates() {
    let CardFacts::Minion(empty) =
        parse_card_definition("plain-minion", &minion(json!({}))).expect("plain minion")
    else {
        panic!("expected minion facts");
    };
    assert!(empty.enters_carrying.is_empty());

    let CardFacts::Minion(facts) = parse_card_definition(
        "equipped-minion",
        &minion(json!({"entersCarrying": ["token-a", "token-a", "token-b"]})),
    )
    .expect("entry equipment") else {
        panic!("expected minion facts");
    };
    assert_eq!(facts.enters_carrying, ["token-a", "token-a", "token-b"]);
}

#[test]
fn enters_carrying_rejects_empty_oversized_and_malformed_lists() {
    let oversized = (0..33)
        .map(|index| json!(format!("token-{index}")))
        .collect::<Vec<_>>();
    for (value, expected) in [
        (json!([]), "nonempty array"),
        (json!(oversized), "at most 32"),
        (json!([1]), "must be a card ID"),
        (json!([""]), "1-256 UTF-16 code units"),
        (json!([null]), "must be a card ID"),
        (json!(true), "nonempty array"),
    ] {
        let error = parse_card_definition(
            "invalid-entry-equipment",
            &minion(json!({"entersCarrying": value})),
        )
        .expect_err("invalid entry equipment admitted")
        .to_string();
        assert!(
            error.contains(expected),
            "expected {expected:?}, got {error}"
        );
    }
}

#[test]
fn enters_carrying_is_rejected_on_nonminions_by_existing_field_admission() {
    let artifact = json!({
        "cardType": "artifact",
        "entersCarrying": ["token-a"],
        "manaCost": 0,
        "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
    });
    let error = parse_card_definition("wrong-entry-equipment-kind", &artifact)
        .expect_err("non-minion entry equipment admitted")
        .to_string();
    assert!(error.contains("is unsupported"), "got {error}");
}

#[test]
fn entry_equipment_dependencies_require_carriable_artifact_tokens() {
    use sorcery_engine::session::Session;
    use sorcery_engine::synthetic::selfplay_manifest_with;

    let manifest = |token: serde_json::Value| {
        selfplay_manifest_with(9121, |m| {
            m["cards"]
                .as_object_mut()
                .unwrap()
                .retain(|id, _| !id.contains("-spell-"));
            m["cards"]["equipment"] = token;
            m["cards"]["bearer"] = minion(json!({"entersCarrying":["equipment"]}));
            for seat in ["north", "south"] {
                m["decks"][seat]["spellbook"] = json!(vec!["bearer"; 10]);
            }
        })
    };
    let token = json!({"cardType":"artifact", "token":true, "manaCost":null,
        "thresholds":{"air":0,"earth":0,"fire":0,"water":0}, "grantsBearerPower":2});
    Session::new(&manifest(token.clone())).expect("carriable artifact token dependency");
    let mut ordinary = token.clone();
    ordinary.as_object_mut().unwrap().remove("token");
    ordinary["manaCost"] = json!(0);
    let mut immovable = token;
    immovable["cannotBeCarried"] = json!(true);
    for bad in [ordinary, immovable, minion(json!({"token":true}))] {
        assert!(Session::new(&manifest(bad)).is_err());
    }
}
