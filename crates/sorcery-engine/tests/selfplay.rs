use std::collections::BTreeMap;

use serde_json::{Value, json};
use sorcery_engine::batch::BatchClassification;
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::deck::{
    CandidateDeck, CardCatalogEntry, CardCount, CardType, DeckCost, DeckValidation, FormatContext,
    OfficialCardMapping, PriceKey, PriceScope, PriceSnapshot, PrintingPrice, Rarity, price_deck,
    validate_deck,
};
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::selfplay::{
    DeckComparisonCandidate, SelfPlayCampaign, SelfPlayPair, compare_decks,
    create_selfplay_campaign_checkpoint, parse_selfplay_campaign_checkpoint,
    resume_selfplay_campaign, serialize_selfplay_campaign_checkpoint, train_and_promote,
};
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

fn rehash_checkpoint(mut checkpoint: Value) -> String {
    let object = checkpoint
        .as_object_mut()
        .expect("campaign checkpoint object");
    object.remove("checkpointId");
    let checkpoint_id = identity_hash(&Value::Object(object.clone())).expect("checkpoint identity");
    checkpoint["checkpointId"] = json!(checkpoint_id);
    canonical_json(&checkpoint).expect("canonical checkpoint")
}

fn mutate_manifest(manifest: &str, mutate: impl FnOnce(&mut Value)) -> String {
    let mut manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    mutate(&mut manifest);
    manifest_with_id(manifest)
}

fn claim_private_local(manifest: &str) -> String {
    mutate_manifest(manifest, |manifest| {
        manifest["authority"]["mode"] = json!("private-local");
    })
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

fn equivalent_candidate_variant(manifests: &(String, String)) -> (String, String) {
    let north = mutate_manifest(&manifests.0, |manifest| {
        let original_id = manifest["decks"]["north"]["spellbook"][0]
            .as_str()
            .expect("first North spell")
            .to_owned();
        let variant_id = format!("{original_id}-variant");
        manifest["cards"][&variant_id] = manifest["cards"][&original_id].clone();
        manifest["decks"]["north"]["spellbook"][0] = json!(variant_id);
    });
    let south = mutate_manifest(&north, |manifest| {
        let north_deck = manifest["decks"]["north"].clone();
        manifest["decks"]["north"] = manifest["decks"]["south"].clone();
        manifest["decks"]["south"] = north_deck;
    });
    (north, south)
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

fn one_pair<'a>(
    manifests: &'a (String, String),
    seed: u32,
    opponent: &'a PolicySnapshot,
    opponent_deck: &'a DeckValidation,
) -> [SelfPlayPair<'a>; 1] {
    [pair(
        &manifests.0,
        &manifests.1,
        seed,
        opponent,
        opponent_deck,
    )]
}

fn comparison_costs(
    baseline: &DeckValidation,
    variant: &DeckValidation,
) -> (DeckCost, DeckCost, DeckCost) {
    let mut definitions = BTreeMap::new();
    for deck in [baseline.deck(), variant.deck()] {
        definitions.insert(deck.avatar.clone(), (CardType::Avatar, None));
        definitions.extend(deck.atlas.iter().map(|row| {
            (
                row.card_id.clone(),
                (CardType::Site, Some(Rarity::Ordinary)),
            )
        }));
        definitions.extend(deck.spellbook.iter().map(|row| {
            (
                row.card_id.clone(),
                (CardType::Minion, Some(Rarity::Ordinary)),
            )
        }));
    }
    let mut cards = Vec::with_capacity(definitions.len());
    let mut prices = Vec::with_capacity(definitions.len() * 2);
    for (stable_id, (card_type, rarity)) in definitions {
        let official_card_id = format!("official:{stable_id}");
        cards.push(CardCatalogEntry {
            stable_id: stable_id.clone(),
            card_type,
            rarity,
            engine_supported: true,
            official_mapping: OfficialCardMapping::Exact(official_card_id.clone()),
            token: false,
        });
        for source in ["market:test", "market:other"] {
            prices.push(PrintingPrice {
                key: PriceKey {
                    official_card_id: official_card_id.clone(),
                    printing_id: format!("printing:{stable_id}:{source}"),
                    variant: "standard".to_owned(),
                    condition: "near-mint".to_owned(),
                    currency: "USD".to_owned(),
                    source: source.to_owned(),
                },
                unit_price_cents: u64::from(!stable_id.ends_with("-variant")) * 100,
            });
        }
    }
    let snapshot = PriceSnapshot::new("snapshot:test".to_owned(), prices)
        .expect("valid shared price snapshot");
    let scope = |source: &str| PriceScope {
        variant: "standard".to_owned(),
        condition: "near-mint".to_owned(),
        currency: "USD".to_owned(),
        source: source.to_owned(),
    };
    (
        price_deck(baseline, &cards, &snapshot, &scope("market:test"))
            .expect("baseline exact cost"),
        price_deck(variant, &cards, &snapshot, &scope("market:test")).expect("variant exact cost"),
        price_deck(variant, &cards, &snapshot, &scope("market:other"))
            .expect("alternate-scope exact cost"),
    )
}

fn assert_comparison_error(candidates: &[DeckComparisonCandidate<'_>], expected: &str) {
    assert_eq!(
        compare_decks(candidates, 10_000, 1_000)
            .expect_err("invalid comparison must fail")
            .to_string(),
        expected
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end deck comparison proof keeps its shared fixtures explicit"
)]
fn deck_comparison_should_use_fixed_seat_pairs_budget_and_cost_tie_break() {
    let manifests = paired_manifests(44, 1);
    let original_id = serde_json::from_str::<Value>(&manifests.0).expect("manifest JSON")["decks"]
        ["north"]["spellbook"][0]
        .as_str()
        .expect("first North spell")
        .to_owned();
    let variant_manifests = equivalent_candidate_variant(&manifests);
    let baseline_deck = validated_manifest_deck(&manifests.0, "north");
    let variant_deck = validated_manifest_deck(&variant_manifests.0, "north");
    let opponent_deck = validated_manifest_deck(&manifests.0, "south");
    let authority = authority_hash(&manifests.0);
    let baseline_policy = policy(
        &authority,
        baseline_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let variant_policy = policy(
        &authority,
        variant_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let opponent = policy(
        &authority,
        opponent_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let baseline_pairs = one_pair(&manifests, 44, &opponent, &opponent_deck);
    let variant_pairs = one_pair(&variant_manifests, 44, &opponent, &opponent_deck);
    let (baseline_cost, variant_cost, alternate_scope_cost) =
        comparison_costs(&baseline_deck, &variant_deck);
    let baseline = DeckComparisonCandidate {
        cost: &baseline_cost,
        deck: &baseline_deck,
        pairs: &baseline_pairs,
        policy: &baseline_policy,
    };
    let variant = DeckComparisonCandidate {
        cost: &variant_cost,
        deck: &variant_deck,
        pairs: &variant_pairs,
        policy: &variant_policy,
    };

    let forward =
        compare_decks(&[baseline, variant], 10_000, 1_000).expect("budgeted deck comparison");
    let reversed = compare_decks(&[variant, baseline], 10_000, 1_000)
        .expect("input-order-independent deck comparison");

    assert_eq!(forward, reversed);
    assert_eq!(forward.standings.len(), 2);
    assert_eq!(forward.standings[0].deck_id, *variant_deck.deck_id());
    assert_eq!(forward.standings[0].cost_cents, 9_000);
    assert_eq!(forward.selected_score, forward.standings[0].score);

    let changed_variant_manifests = (
        mutate_manifest(&variant_manifests.0, |manifest| {
            manifest["firstSeat"] = json!("south");
        }),
        mutate_manifest(&variant_manifests.1, |manifest| {
            manifest["firstSeat"] = json!("south");
        }),
    );
    let changed_pairs = one_pair(&changed_variant_manifests, 44, &opponent, &opponent_deck);
    let changed = DeckComparisonCandidate {
        pairs: &changed_pairs,
        ..variant
    };
    assert_comparison_error(
        &[baseline, changed],
        "deck comparison requires the same ordered scenario, seed, and opponent suite",
    );

    let mut changed_features = BASELINE_FEATURES;
    changed_features.swap(0, 1);
    let changed_policy = policy(
        &authority,
        variant_deck.deck_id().as_str(),
        changed_features,
    );
    let changed = DeckComparisonCandidate {
        policy: &changed_policy,
        ..variant
    };
    assert_comparison_error(
        &[baseline, changed],
        "deck comparison requires one fixed policy strategy",
    );

    let changed_card_manifests = (
        mutate_manifest(&variant_manifests.0, |manifest| {
            manifest["cards"][&original_id]["cost"] = json!(9);
        }),
        mutate_manifest(&variant_manifests.1, |manifest| {
            manifest["cards"][&original_id]["cost"] = json!(9);
        }),
    );
    let changed_card_pairs = one_pair(&changed_card_manifests, 44, &opponent, &opponent_deck);
    let changed_card = DeckComparisonCandidate {
        pairs: &changed_card_pairs,
        ..variant
    };
    assert_comparison_error(
        &[baseline, changed_card],
        "deck comparison requires the same ordered scenario, seed, and opponent suite",
    );

    let alternate_scope = DeckComparisonCandidate {
        cost: &alternate_scope_cost,
        ..variant
    };
    assert_comparison_error(
        &[baseline, alternate_scope],
        "deck comparison requires one price snapshot and market scope",
    );
}

#[test]
fn campaign_checkpoint_should_round_trip_and_require_the_validated_deck() {
    let manifests = paired_manifests(30, 20);
    let candidate_deck = validated_manifest_deck(&manifests.0, "north");
    let opponent_deck = validated_manifest_deck(&manifests.0, "south");
    let champion = policy(
        &authority_hash(&manifests.0),
        candidate_deck.deck_id().as_str(),
        BASELINE_FEATURES,
    );
    let campaign =
        SelfPlayCampaign::new(champion.clone(), candidate_deck.clone(), 500).expect("campaign");
    let checkpoint = create_selfplay_campaign_checkpoint(&campaign).expect("campaign checkpoint");
    let serialized =
        serialize_selfplay_campaign_checkpoint(&checkpoint).expect("canonical checkpoint");
    let parsed = parse_selfplay_campaign_checkpoint(&serialized).expect("parsed checkpoint");
    let resumed =
        resume_selfplay_campaign(&parsed, candidate_deck.clone()).expect("resumed campaign");

    assert_eq!(parsed.checkpoint_id(), checkpoint.checkpoint_id());
    assert_eq!(resumed.champion(), &champion);
    assert_eq!(resumed.max_actions(), 500);
    assert_eq!(resumed.promotion_attempts(), 0);
    assert!(!resumed.is_finalized());
    assert!(!serialized.contains("pendingOperation"));
    assert!(resume_selfplay_campaign(&parsed, opponent_deck).is_err());
    assert!(parse_selfplay_campaign_checkpoint(&format!(" {serialized}")).is_err());
    assert!(
        parse_selfplay_campaign_checkpoint(&serialized.replacen(
            '{',
            "{\"kind\":\"duplicate\",",
            1
        ))
        .is_err()
    );
    assert!(
        parse_selfplay_campaign_checkpoint(&serialized.replace(
            "\"kind\":\"sorcery-self-play-campaign-checkpoint\"",
            "\"kind\":\"changed\""
        ))
        .is_err()
    );

    let mut impossible: Value = serde_json::from_str(&serialized).expect("checkpoint JSON");
    impossible["promotionAttempts"] = json!(1);
    impossible["promotionPortfolio"] = json!([{
        "cell": {
            "subgroup": "mirror",
            "opponentPolicyId": champion.policy_id(),
            "opponentDeckId": candidate_deck.deck_id(),
            "scenarioId": identity_hash(&json!("scenario")).expect("scenario identity"),
        },
        "count": 128,
    }]);
    impossible["usedDevelopmentSeeds"] = json!((0_u32..21).collect::<Vec<_>>());
    assert!(parse_selfplay_campaign_checkpoint(&rehash_checkpoint(impossible)).is_err());

    let mut blank_subgroup: Value = serde_json::from_str(&serialized).expect("checkpoint JSON");
    blank_subgroup["promotionAttempts"] = json!(1);
    blank_subgroup["promotionPortfolio"] = json!([{
        "cell": {
            "subgroup": " ",
            "opponentPolicyId": champion.policy_id(),
            "opponentDeckId": candidate_deck.deck_id(),
            "scenarioId": identity_hash(&json!("scenario")).expect("scenario identity"),
        },
        "count": 20,
    }]);
    blank_subgroup["usedDevelopmentSeeds"] = json!((0_u32..21).collect::<Vec<_>>());
    assert!(parse_selfplay_campaign_checkpoint(&rehash_checkpoint(blank_subgroup)).is_err());
}

#[test]
fn pending_generation_checkpoint_should_require_the_exact_ordered_suite() {
    let training_manifests = paired_manifests(40, 1);
    let promotion_manifests = (100..120)
        .map(|seed| (seed, paired_manifests(seed, 1)))
        .collect::<Vec<_>>();
    let candidate_deck = validated_manifest_deck(&training_manifests.0, "north");
    let opponent_deck = validated_manifest_deck(&training_manifests.0, "south");
    let authority = authority_hash(&training_manifests.0);
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
    let training = one_pair(&training_manifests, 40, &opponent, &opponent_deck);
    let promotion = promotion_manifests
        .iter()
        .map(|(seed, manifests)| pair(&manifests.0, &manifests.1, *seed, &opponent, &opponent_deck))
        .collect::<Vec<_>>();
    let mut campaign =
        SelfPlayCampaign::new(champion, candidate_deck.clone(), 1_000).expect("campaign");
    campaign
        .reserve_generation(&training, &promotion)
        .expect("reserved generation");
    let checkpoint = create_selfplay_campaign_checkpoint(&campaign).expect("pending checkpoint");
    let serialized =
        serialize_selfplay_campaign_checkpoint(&checkpoint).expect("serialized checkpoint");
    let mut resumed = resume_selfplay_campaign(
        &parse_selfplay_campaign_checkpoint(&serialized).expect("parsed pending checkpoint"),
        candidate_deck,
    )
    .expect("resumed pending generation");
    let mut reordered = promotion.clone();
    reordered.swap(0, 1);

    assert!(resumed.complete_generation(&training, &reordered).is_err());
    assert_eq!(
        serialize_selfplay_campaign_checkpoint(
            &create_selfplay_campaign_checkpoint(&resumed).expect("retained pending checkpoint")
        )
        .expect("serialized retained checkpoint"),
        serialized
    );
}

#[test]
fn heldout_tie_should_keep_the_replay_verified_champion_deterministically() {
    let (training_north, training_south) = paired_manifests(30, 20);
    let (heldout_north, heldout_south) = paired_manifests(31, 20);
    let training_north = claim_private_local(&training_north);
    let training_south = claim_private_local(&training_south);
    let heldout_north = claim_private_local(&heldout_north);
    let heldout_south = claim_private_local(&heldout_south);
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
    assert_eq!(
        first.classification,
        BatchClassification::UnrankedPartialRulesUnverifiedAuthority
    );
    assert_eq!(
        serde_json::to_value(first.classification).expect("classification JSON"),
        "unranked_partial_rules_unverified_authority"
    );
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
    let mut relabeled = valid[0];
    relabeled.subgroup = "relabeled";
    let duplicate_matchup = [valid[0], relabeled];
    let not_swapped = [pair(&north, &north, 30, &opponent, &opponent_deck)];
    let wrong_deck = [pair(&north, &south, 30, &opponent, &candidate_deck)];
    let wrong_seed = [pair(&north, &south, 31, &opponent, &opponent_deck)];
    let malformed = SelfPlayPair {
        candidate_as_north_manifest_json: "{",
        candidate_as_south_manifest_json: "{",
        ..valid[0]
    };
    let oversized = vec![malformed; 129];

    assert!(train_and_promote(&champion, &candidate_deck, &[], &valid, 500).is_err());
    assert!(train_and_promote(&champion, &candidate_deck, &not_swapped, &valid, 500).is_err());
    assert!(train_and_promote(&champion, &candidate_deck, &wrong_deck, &valid, 500).is_err());
    assert!(train_and_promote(&champion, &candidate_deck, &wrong_seed, &valid, 500).is_err());
    assert!(train_and_promote(&champion, &candidate_deck, &valid, &valid, 500).is_err());
    assert!(
        train_and_promote(&champion, &candidate_deck, &duplicate_matchup, &valid, 500).is_err()
    );
    assert_eq!(
        train_and_promote(&champion, &candidate_deck, &valid, &oversized, 500)
            .expect_err("oversized suite")
            .to_string(),
        "self-play suites must contain 1-128 seat-swapped pairs"
    );
    assert!(SelfPlayCampaign::new(champion.clone(), candidate_deck.clone(), 0).is_err());
    let mut campaign =
        SelfPlayCampaign::new(champion.clone(), candidate_deck.clone(), 500).expect("campaign");
    assert_eq!(campaign.max_actions(), 500);
    assert_eq!(campaign.promotion_attempts(), 0);
    assert_eq!(
        campaign
            .run_generation(&valid, &oversized)
            .expect_err("oversized campaign suite")
            .to_string(),
        "self-play promotion suites must contain 20-128 unique-seed seat pairs"
    );
    let child = champion.neighbors().expect("neighbors").remove(0);
    assert!(SelfPlayCampaign::new(child, candidate_deck, 500).is_err());
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
        manifest["cards"]["north-spell-1"]["voidwalk"] = json!(true);
    });
    let unsupported_south = mutate_manifest(&south, |manifest| {
        manifest["cards"]["north-spell-1"]["voidwalk"] = json!(true);
    });
    let unsupported = [pair(
        &unsupported_north,
        &unsupported_south,
        30,
        &opponent,
        &opponent_deck,
    )];
    assert!(train_and_promote(&champion, &candidate_deck, &unsupported, &heldout, 500).is_err());

    let unsupported_north = mutate_manifest(&north, |manifest| {
        manifest["cards"]["north-spell-1"]["mustBeCastToOuterColumn"] = json!(true);
    });
    let unsupported_south = mutate_manifest(&south, |manifest| {
        manifest["cards"]["north-spell-1"]["mustBeCastToOuterColumn"] = json!(true);
    });
    let unsupported = [pair(
        &unsupported_north,
        &unsupported_south,
        30,
        &opponent,
        &opponent_deck,
    )];
    assert_eq!(
        train_and_promote(&champion, &candidate_deck, &unsupported, &heldout, 500)
            .expect_err("self-play must reject incomplete facts")
            .to_string(),
        "manifest fact is not yet supported by Rust: mustBeCastToOuterColumn"
    );

    let stealth_north = mutate_manifest(&north, |manifest| {
        manifest["cards"]["north-spell-1"]["stealth"] = json!(true);
    });
    let stealth_south = mutate_manifest(&south, |manifest| {
        manifest["cards"]["north-spell-1"]["stealth"] = json!(true);
    });
    let stealth = [pair(
        &stealth_north,
        &stealth_south,
        30,
        &opponent,
        &opponent_deck,
    )];
    assert!(train_and_promote(&champion, &candidate_deck, &stealth, &heldout, 500).is_ok());
}

#[test]
fn underpowered_gain_should_replay_but_not_promote_or_start_a_campaign() {
    let training_manifests = paired_manifests(40, 1);
    let heldout_manifests = paired_manifests(41, 1);
    let audit_manifests = paired_manifests(42, 1);
    let candidate_deck = validated_manifest_deck(&training_manifests.0, "north");
    let opponent_deck = validated_manifest_deck(&training_manifests.0, "south");
    let authority = authority_hash(&training_manifests.0);
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
    let training = one_pair(&training_manifests, 40, &opponent, &opponent_deck);
    let heldout = one_pair(&heldout_manifests, 41, &opponent, &opponent_deck);
    let audit = one_pair(&audit_manifests, 42, &opponent, &opponent_deck);
    let result = train_and_promote(&champion, &candidate_deck, &training, &heldout, 1_000)
        .expect("replay-gated comparison");
    let mut campaign =
        SelfPlayCampaign::new(champion.clone(), candidate_deck.clone(), 1_000).expect("campaign");

    assert!(!result.promoted, "{result:#?}");
    assert_eq!(
        result.classification,
        BatchClassification::UnrankedPartialRulesUnverifiedAuthority
    );
    assert_eq!(result.policy, champion);
    assert!(
        result.nominee_heldout.half_points() > result.champion_heldout.half_points(),
        "{result:#?}"
    );
    assert!(campaign.run_generation(&training, &heldout).is_err());
    assert!(campaign.run_generation(&training, &heldout).is_err());
    assert!(campaign.final_audit(&audit).is_err());
    assert!(!campaign.is_finalized());
    assert_eq!(campaign.champion(), &champion);
    assert_eq!(campaign.lineage().collect::<Vec<_>>().len(), 1);
}

#[test]
#[ignore = "release-only production-sized self-play acceptance gate"]
#[expect(
    clippy::too_many_lines,
    reason = "one acceptance proof covers generation and audit reservation across restart"
)]
fn production_sized_campaign_should_promote_and_seal_reproducibly() {
    let training_manifests = paired_manifests(40, 1);
    let promotion_manifests = (100..120)
        .map(|seed| (seed, paired_manifests(seed, 1)))
        .collect::<Vec<_>>();
    let audit_manifests = (200..220)
        .map(|seed| (seed, paired_manifests(seed, 1)))
        .collect::<Vec<_>>();
    let candidate_deck = validated_manifest_deck(&training_manifests.0, "north");
    let opponent_deck = validated_manifest_deck(&training_manifests.0, "south");
    let authority = authority_hash(&training_manifests.0);
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
    let training = one_pair(&training_manifests, 40, &opponent, &opponent_deck);
    let promotion = promotion_manifests
        .iter()
        .map(|(seed, manifests)| pair(&manifests.0, &manifests.1, *seed, &opponent, &opponent_deck))
        .collect::<Vec<_>>();
    let audit = audit_manifests
        .iter()
        .map(|(seed, manifests)| pair(&manifests.0, &manifests.1, *seed, &opponent, &opponent_deck))
        .collect::<Vec<_>>();
    let mut first = SelfPlayCampaign::new(champion.clone(), candidate_deck.clone(), 1_000)
        .expect("first campaign");
    let mut second = SelfPlayCampaign::new(champion.clone(), candidate_deck.clone(), 1_000)
        .expect("second campaign");

    first
        .reserve_generation(&training, &promotion)
        .expect("reserved first production-sized promotion");
    let pending_generation =
        create_selfplay_campaign_checkpoint(&first).expect("pending generation checkpoint");
    first = resume_selfplay_campaign(&pending_generation, candidate_deck.clone())
        .expect("resumed pending generation");
    let mut wrong_promotion = promotion.clone();
    wrong_promotion.swap(0, 1);
    assert!(
        first
            .complete_generation(&training, &wrong_promotion)
            .is_err()
    );
    let first_promotion = first
        .complete_generation(&training, &promotion)
        .expect("completed first production-sized promotion");
    let second_promotion = second
        .run_generation(&training, &promotion)
        .expect("repeated production-sized promotion");
    assert!(first_promotion.promoted, "{first_promotion:#?}");
    assert_eq!(first_promotion, second_promotion);

    let first_checkpoint =
        create_selfplay_campaign_checkpoint(&first).expect("first campaign checkpoint");
    let second_checkpoint =
        create_selfplay_campaign_checkpoint(&second).expect("second campaign checkpoint");
    let first_serialized = serialize_selfplay_campaign_checkpoint(&first_checkpoint)
        .expect("first serialized checkpoint");
    assert_eq!(
        first_serialized,
        serialize_selfplay_campaign_checkpoint(&second_checkpoint)
            .expect("second serialized checkpoint")
    );
    first = resume_selfplay_campaign(
        &parse_selfplay_campaign_checkpoint(&first_serialized).expect("parsed campaign checkpoint"),
        candidate_deck.clone(),
    )
    .expect("resumed first campaign");
    assert!(first.run_generation(&training, &promotion).is_err());

    first
        .reserve_final_audit(&audit)
        .expect("reserved first fresh final audit");
    let pending_audit =
        create_selfplay_campaign_checkpoint(&first).expect("pending final-audit checkpoint");
    first = resume_selfplay_campaign(&pending_audit, candidate_deck.clone())
        .expect("resumed pending final audit");
    let first_audit = first
        .complete_final_audit(&audit)
        .expect("completed first fresh final audit");
    let second_audit = second
        .final_audit(&audit)
        .expect("repeated fresh final audit");
    assert_eq!(first_audit, second_audit);
    assert!(first_audit.score.half_points() > first_audit.baseline_score.half_points());
    assert_eq!(first.champion(), second.champion());
    assert_eq!(first.lineage().collect::<Vec<_>>().len(), 2);
    assert!(first.is_finalized());
    assert!(second.is_finalized());
    assert_eq!(first.promotion_attempts(), 1);
    assert!(first.final_audit(&audit).is_err());
    let sealed = create_selfplay_campaign_checkpoint(&first).expect("sealed checkpoint");
    let mut sealed = resume_selfplay_campaign(&sealed, candidate_deck).expect("sealed campaign");
    assert!(sealed.is_finalized());
    assert!(sealed.final_audit(&audit).is_err());
}
