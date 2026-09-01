use std::collections::BTreeMap;

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::deck::{
    CandidateDeck, CardCatalogEntry, CardCount, CardType, DeckValidation, FormatContext,
    OfficialCardMapping, Rarity, validate_deck,
};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::selfplay::{SelfPlayPair, train_and_promote};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const BASELINE_FEATURES: [&str; 9] = [
    "keep-mulligan",
    "play-site",
    "summon-minion",
    "preferred-draw",
    "powered-movement",
    "beneficial-tactic",
    "move-toward-enemy",
    "end-turn",
    "canonical-fallback",
];

fn authority_hash(manifest: &str) -> String {
    serde_json::from_str::<Value>(manifest).expect("manifest JSON")["authority"]["contentHash"]
        .as_str()
        .expect("authority content hash")
        .to_owned()
}

fn policy(authority_hash: &str, deck_id: &str, feature_priority: [&str; 9]) -> PolicySnapshot {
    let mut body = json!({
        "authorityHash": authority_hash,
        "deckId": deck_id,
        "engineVersion": "sorcery-core-v1",
        "generation": 0,
        "observationVersion": "seat-observation-v1",
        "schemaVersion": 1,
        "selector": {
            "atlasReserve": 3,
            "featurePriority": feature_priority,
        },
        "tieBreak": "canonical-action-order-v1"
    });
    body["policyId"] = json!(identity_hash(&body).expect("policy identity"));
    parse_policy_snapshot(&canonical_json(&body).expect("canonical policy")).expect("valid policy")
}

fn manifest_with_id(mut manifest: Value) -> String {
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    let manifest_id = identity_hash(&manifest).expect("manifest identity");
    manifest["manifestId"] = json!(manifest_id);
    canonical_json(&manifest).expect("canonical manifest")
}

fn mutate_manifest(manifest: &str, mutate: impl FnOnce(&mut Value)) -> String {
    let mut manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    mutate(&mut manifest);
    manifest_with_id(manifest)
}

fn paired_manifests(seed: u32, avatar_life: u8) -> (String, String) {
    let mut north: Value =
        serde_json::from_str(&synthetic_demo_manifest_json(seed).expect("synthetic manifest"))
            .expect("manifest JSON");
    for seat in ["north", "south"] {
        let spellbook = north["decks"][seat]["spellbook"]
            .as_array_mut()
            .expect("spellbook");
        spellbook.extend(spellbook[..10].to_vec());
        north["cards"][format!("{seat}-avatar")]["life"] = json!(avatar_life);
    }
    let north = manifest_with_id(north);
    let mut south: Value = serde_json::from_str(&north).expect("candidate North manifest");
    let north_deck = south["decks"]["north"].clone();
    south["decks"]["north"] = south["decks"]["south"].clone();
    south["decks"]["south"] = north_deck;
    (north, manifest_with_id(south))
}

fn counted(ids: &[Value]) -> Vec<CardCount> {
    let mut counts = BTreeMap::<String, u32>::new();
    for id in ids {
        *counts
            .entry(id.as_str().expect("card ID").to_owned())
            .or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(card_id, copies)| CardCount { card_id, copies })
        .collect()
}

fn validated_manifest_deck(manifest: &str, seat: &str) -> DeckValidation {
    let manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    let deck = &manifest["decks"][seat];
    let candidate = CandidateDeck {
        avatar: deck["avatar"].as_str().expect("avatar ID").to_owned(),
        atlas: counted(deck["atlas"].as_array().expect("Atlas")),
        spellbook: counted(deck["spellbook"].as_array().expect("Spellbook")),
    };
    let mut catalog = vec![CardCatalogEntry {
        stable_id: candidate.avatar.clone(),
        card_type: CardType::Avatar,
        rarity: None,
        engine_supported: true,
        official_mapping: OfficialCardMapping::Unavailable,
        token: false,
    }];
    catalog.extend(candidate.atlas.iter().map(|row| CardCatalogEntry {
        stable_id: row.card_id.clone(),
        card_type: CardType::Site,
        rarity: Some(Rarity::Ordinary),
        engine_supported: true,
        official_mapping: OfficialCardMapping::Unavailable,
        token: false,
    }));
    catalog.extend(candidate.spellbook.iter().map(|row| CardCatalogEntry {
        stable_id: row.card_id.clone(),
        card_type: CardType::Minion,
        rarity: Some(Rarity::Ordinary),
        engine_supported: true,
        official_mapping: OfficialCardMapping::Unavailable,
        token: false,
    }));
    validate_deck(candidate, &catalog, FormatContext::constructed()).expect("valid assigned deck")
}

fn pair<'a>(
    north: &'a str,
    south: &'a str,
    seed: u32,
    opponent: &'a PolicySnapshot,
    opponent_deck: &'a DeckValidation,
) -> SelfPlayPair<'a> {
    SelfPlayPair {
        seed,
        subgroup: "mirror",
        candidate_as_north_manifest_json: north,
        candidate_as_south_manifest_json: south,
        opponent,
        opponent_deck,
    }
}

#[test]
fn heldout_tie_should_keep_the_replay_verified_champion_deterministically() {
    let (training_north, training_south) = paired_manifests(30, 20);
    let (heldout_north, heldout_south) = paired_manifests(31, 20);
    let candidate_deck = validated_manifest_deck(&training_north, "north");
    let opponent_deck = validated_manifest_deck(&training_north, "south");
    let authority = authority_hash(&heldout_north);
    let champion = policy(
        &authority,
        candidate_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let opponent = policy(
        &authority,
        opponent_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let training = [pair(
        &training_north,
        &training_south,
        30,
        &opponent,
        &opponent_deck,
    )];
    let heldout = [pair(
        &heldout_north,
        &heldout_south,
        31,
        &opponent,
        &opponent_deck,
    )];

    let first = train_and_promote(&champion, &candidate_deck, &training, &heldout, 500)
        .expect("first promotion cycle");
    let second = train_and_promote(&champion, &candidate_deck, &training, &heldout, 500)
        .expect("repeat promotion cycle");

    assert_eq!(first, second);
    assert!(!first.promoted);
    assert_eq!(first.policy, champion);
    assert_eq!(first.champion_heldout.games(), 2);
    assert_eq!(first.nominee_heldout.games(), 2);
    assert_eq!(
        first.champion_heldout.half_points(),
        first.nominee_heldout.half_points()
    );
}

#[test]
fn suite_should_reject_non_swaps_wrong_bindings_and_reused_seeds() {
    let (north, south) = paired_manifests(30, 20);
    let candidate_deck = validated_manifest_deck(&north, "north");
    let opponent_deck = validated_manifest_deck(&north, "south");
    let authority = authority_hash(&north);
    let champion = policy(
        &authority,
        candidate_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let opponent = policy(
        &authority,
        opponent_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let valid = [pair(&north, &south, 30, &opponent, &opponent_deck)];
    let not_swapped = [pair(&north, &north, 30, &opponent, &opponent_deck)];
    let wrong_deck = [pair(&north, &south, 30, &opponent, &candidate_deck)];
    let wrong_seed = [pair(&north, &south, 31, &opponent, &opponent_deck)];

    assert!(train_and_promote(&champion, &candidate_deck, &[], &valid, 500).is_err());
    assert!(train_and_promote(&champion, &candidate_deck, &not_swapped, &valid, 500).is_err());
    assert!(train_and_promote(&champion, &candidate_deck, &wrong_deck, &valid, 500).is_err());
    assert!(train_and_promote(&champion, &candidate_deck, &wrong_seed, &valid, 500).is_err());
    assert!(train_and_promote(&champion, &candidate_deck, &valid, &valid, 500).is_err());
}

#[test]
fn pair_should_reject_scenario_changes_composition_mismatch_and_unsupported_facts() {
    let (north, south) = paired_manifests(30, 20);
    let candidate_deck = validated_manifest_deck(&north, "north");
    let opponent_deck = validated_manifest_deck(&north, "south");
    let authority = authority_hash(&north);
    let champion = policy(
        &authority,
        candidate_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let opponent = policy(
        &authority,
        opponent_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let heldout_north = paired_manifests(31, 20).0;
    let heldout_south = paired_manifests(31, 20).1;
    let heldout = [pair(
        &heldout_north,
        &heldout_south,
        31,
        &opponent,
        &opponent_deck,
    )];

    let changed_scenario = mutate_manifest(&south, |manifest| {
        manifest["cards"]["north-spell-1"]["attack"] = json!(2);
    });
    let changed_scenario = [pair(
        &north,
        &changed_scenario,
        30,
        &opponent,
        &opponent_deck,
    )];
    assert!(
        train_and_promote(&champion, &candidate_deck, &changed_scenario, &heldout, 500).is_err()
    );

    let mismatched_north = mutate_manifest(&north, |manifest| {
        manifest["decks"]["north"]["spellbook"][0] = json!("north-spell-2");
    });
    let mismatched_south = mutate_manifest(&south, |manifest| {
        manifest["decks"]["south"]["spellbook"][0] = json!("north-spell-2");
    });
    let mismatched = [pair(
        &mismatched_north,
        &mismatched_south,
        30,
        &opponent,
        &opponent_deck,
    )];
    assert!(train_and_promote(&champion, &candidate_deck, &mismatched, &heldout, 500).is_err());

    let unsupported_north = mutate_manifest(&north, |manifest| {
        manifest["cards"]["north-spell-1"]["occupiesSquareArea"] = json!(2);
    });
    let unsupported_south = mutate_manifest(&south, |manifest| {
        manifest["cards"]["north-spell-1"]["occupiesSquareArea"] = json!(2);
    });
    let unsupported = [pair(
        &unsupported_north,
        &unsupported_south,
        30,
        &opponent,
        &opponent_deck,
    )];
    assert!(train_and_promote(&champion, &candidate_deck, &unsupported, &heldout, 500).is_err());
}

#[test]
fn adjacent_priority_gain_should_promote_and_replay_from_both_seats() {
    let (training_north, training_south) = paired_manifests(40, 1);
    let (heldout_north, heldout_south) = paired_manifests(41, 1);
    let candidate_deck = validated_manifest_deck(&training_north, "north");
    let opponent_deck = validated_manifest_deck(&training_north, "south");
    let authority = authority_hash(&training_north);
    let passive_features = [
        "keep-mulligan",
        "play-site",
        "summon-minion",
        "preferred-draw",
        "powered-movement",
        "beneficial-tactic",
        "end-turn",
        "move-toward-enemy",
        "canonical-fallback",
    ];
    let champion = policy(
        &authority,
        candidate_deck.deck_id().as_str(),
        passive_features,
    );
    let opponent = policy(
        &authority,
        opponent_deck.deck_id().as_str(),
        passive_features,
    );
    let training = [pair(
        &training_north,
        &training_south,
        40,
        &opponent,
        &opponent_deck,
    )];
    let heldout = [pair(
        &heldout_north,
        &heldout_south,
        41,
        &opponent,
        &opponent_deck,
    )];

    let result = train_and_promote(&champion, &candidate_deck, &training, &heldout, 1_000)
        .expect("positive promotion cycle");

    assert!(result.promoted, "{result:#?}");
    assert_eq!(result.policy.parent_policy_id(), Some(champion.policy_id()));
    assert!(
        result.nominee_heldout.half_points() > result.champion_heldout.half_points(),
        "{result:#?}"
    );
}
