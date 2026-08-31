//! Public synthetic manifest used by parity tests and release benchmarks.

use serde_json::{Map, Value, json};

use crate::canonical::{CanonicalError, canonical_json, identity_hash};

const SYNTHETIC_AUTHORITY_HASH: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";

/// Builds the canonical, entirely synthetic two-player demo manifest.
///
/// # Errors
///
/// Returns [`CanonicalError`] if the generated manifest cannot be canonicalized.
pub fn synthetic_demo_manifest_json(seed: u32) -> Result<String, CanonicalError> {
    let decks = json!({
        "north": demo_deck("north"),
        "south": demo_deck("south"),
    });
    let mut cards = Map::new();
    add_demo_cards(&mut cards, "north");
    add_demo_cards(&mut cards, "south");
    let mut manifest = json!({
        "authority": {
            "contentHash": SYNTHETIC_AUTHORITY_HASH,
            "mode": "synthetic",
            "revisionId": "synthetic-setup-fixture-v1",
        },
        "cards": cards,
        "decks": decks,
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    let manifest_id = identity_hash(&manifest)?;
    manifest["manifestId"] = json!(manifest_id);
    canonical_json(&manifest)
}

fn demo_deck(prefix: &str) -> Value {
    json!({
        "atlas": numbered_ids(prefix, "site", 30),
        "avatar": format!("{prefix}-avatar"),
        "spellbook": numbered_ids(prefix, "spell", 50),
    })
}

fn numbered_ids(prefix: &str, zone: &str, count: u8) -> Vec<String> {
    (1..=count)
        .map(|ordinal| format!("{prefix}-{zone}-{ordinal}"))
        .collect()
}

fn add_demo_cards(cards: &mut Map<String, Value>, prefix: &str) {
    cards.insert(
        format!("{prefix}-avatar"),
        json!({
            "attack": 1,
            "cardType": "avatar",
            "defense": 1,
            "drawSpell": false,
            "life": 20,
        }),
    );
    for ordinal in 1..=30 {
        cards.insert(
            format!("{prefix}-site-{ordinal}"),
            json!({"cardType": "site", "elements": ["earth"]}),
        );
    }
    for ordinal in 1..=50 {
        let mut thresholds = Map::new();
        thresholds.insert("air".to_owned(), json!(0));
        thresholds.insert("earth".to_owned(), json!(1));
        thresholds.insert("fire".to_owned(), json!(0));
        thresholds.insert("water".to_owned(), json!(0));
        cards.insert(
            format!("{prefix}-spell-{ordinal}"),
            json!({
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 1,
                "thresholds": thresholds,
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::synthetic_demo_manifest_json;
    use crate::game::Game;

    #[test]
    fn synthetic_manifest_should_be_canonical_and_accepted() {
        let manifest = synthetic_demo_manifest_json(31).expect("synthetic manifest");
        Game::from_manifest_json(&manifest).expect("accepted synthetic manifest");
    }
}
