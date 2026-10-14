//! Canonical optional Site characteristics remain independent of legacy abilities.
use serde_json::{Value, json};
use sorcery_engine::facts::{CardFacts, parse_card_definition};

#[test]
fn site_subtypes_preserve_absent_empty_and_independent_tower_qualification() {
    for names in [
        None,
        Some(json!([])),
        Some(json!(["River"])),
        Some(json!(["River", "Tower"])),
    ] {
        for legacy in [false, true] {
            let mut card = json!({"cardType":"site","elements":["air"]});
            if let Some(names) = &names {
                card["subtypes"] = names.clone();
            }
            if legacy {
                card["isTower"] = json!(true);
            }
            let CardFacts::Site(facts) = parse_card_definition("synthetic-site", &card).unwrap()
            else {
                panic!("site facts")
            };
            assert_eq!(facts.subtypes.as_ref().map(|v| json!(v)), names);
            assert_eq!(facts.is_tower, legacy);
        }
    }
}

#[test]
fn site_subtypes_reject_noncanonical_lists_without_relaxing_tower_flags() {
    for names in [
        json!(null),
        json!("Tower"),
        json!([1]),
        json!(["Tower", "River"]),
        json!(["River", "River"]),
        json!([""]),
        json!([" Tower"]),
        json!(["Tower\n"]),
        json!(["A\u{7f}B"]),
        json!(["é".repeat(33)]),
        json!((0..17).map(|i| format!("T{i:02}")).collect::<Vec<_>>()),
    ] {
        let card = json!({"cardType":"site","elements":[],"subtypes":names});
        assert!(
            parse_card_definition("invalid-site", &card).is_err(),
            "{card}"
        );
    }
    for flag in [Value::Null, json!(false), json!(1), json!("true")] {
        assert!(
            parse_card_definition(
                "invalid-flag",
                &json!({"cardType":"site","elements":[],"subtypes":["Tower"],"isTower":flag})
            )
            .is_err()
        );
    }
}
