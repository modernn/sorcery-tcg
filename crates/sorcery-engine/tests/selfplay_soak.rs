use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::deck::{
    CandidateDeck, CardCatalogEntry, CardCount, CardType, DeckCost, DeckValidation, FormatContext,
    OfficialCardMapping, PriceKey, PriceScope, PriceSnapshot, PrintingPrice, Rarity, price_deck,
    validate_deck,
};
use sorcery_engine::game::Game;
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::selfplay::{
    DeckComparisonCandidate, DeckComparisonResult, SelfPlayPair, SelfPlayPairScore, compare_decks,
};
use sorcery_engine::simulator::{Rollout, replay_selected, search_root_actions};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const SEEDS: [u32; 4] = [31, 47, 59, 83];
const MAX_ACTIONS: usize = 512;
const MAX_ROOT_ACTIONS: usize = 4;
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
const PRESSURE_FEATURES: [&str; 9] = [
    "keep-mulligan",
    "play-site",
    "preferred-draw",
    "powered-movement",
    "beneficial-tactic",
    "move-toward-enemy",
    "summon-minion",
    "end-turn",
    "canonical-fallback",
];

struct SeatPair {
    candidate_north: String,
    candidate_south: String,
}

fn with_manifest_id(mut manifest: Value) -> String {
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical manifest")
}

fn constructed_manifest(seed: u32) -> String {
    let mut manifest: Value =
        serde_json::from_str(&synthetic_demo_manifest_json(seed).expect("synthetic manifest"))
            .expect("manifest JSON");
    for seat in ["north", "south"] {
        let spellbook = manifest["decks"][seat]["spellbook"]
            .as_array_mut()
            .expect("spellbook");
        spellbook.extend(spellbook[..10].to_vec());
    }
    with_manifest_id(manifest)
}

fn swapped_manifest(manifest: &str) -> String {
    let mut manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    let decks = manifest["decks"].as_object_mut().expect("manifest decks");
    let north = decks.remove("north").expect("North deck");
    let south = decks.remove("south").expect("South deck");
    decks.insert("north".to_owned(), south);
    decks.insert("south".to_owned(), north);
    with_manifest_id(manifest)
}

fn variant_manifest(manifest: &str) -> String {
    let mut manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    let original_id = manifest["decks"]["north"]["spellbook"][0]
        .as_str()
        .expect("first North spell")
        .to_owned();
    let variant_id = format!("{original_id}-variant");
    manifest["cards"][&variant_id] = manifest["cards"][&original_id].clone();
    manifest["decks"]["north"]["spellbook"][0] = json!(variant_id);
    with_manifest_id(manifest)
}

fn seat_pairs(variant: bool) -> Vec<SeatPair> {
    SEEDS
        .into_iter()
        .map(|seed| {
            let baseline = constructed_manifest(seed);
            let candidate_north = if variant {
                variant_manifest(&baseline)
            } else {
                baseline
            };
            let candidate_south = swapped_manifest(&candidate_north);
            SeatPair {
                candidate_north,
                candidate_south,
            }
        })
        .collect()
}

fn counted(cards: &[Value]) -> Vec<CardCount> {
    let mut counts = BTreeMap::<String, u32>::new();
    for card in cards {
        *counts
            .entry(card.as_str().expect("card ID").to_owned())
            .or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(card_id, copies)| CardCount { card_id, copies })
        .collect()
}

fn validated_deck(manifest: &str, seat: &str) -> (DeckValidation, Vec<CardCatalogEntry>) {
    let manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    let deck = &manifest["decks"][seat];
    let candidate = CandidateDeck {
        avatar: deck["avatar"].as_str().expect("avatar ID").to_owned(),
        atlas: counted(deck["atlas"].as_array().expect("Atlas")),
        spellbook: counted(deck["spellbook"].as_array().expect("Spellbook")),
    };
    let mut ids = Vec::with_capacity(1 + candidate.atlas.len() + candidate.spellbook.len());
    ids.push((candidate.avatar.clone(), CardType::Avatar, None));
    ids.extend(
        candidate
            .atlas
            .iter()
            .map(|row| (row.card_id.clone(), CardType::Site, Some(Rarity::Ordinary))),
    );
    ids.extend(candidate.spellbook.iter().map(|row| {
        (
            row.card_id.clone(),
            CardType::Minion,
            Some(Rarity::Ordinary),
        )
    }));
    let catalog = ids
        .into_iter()
        .map(|(stable_id, card_type, rarity)| CardCatalogEntry {
            official_mapping: OfficialCardMapping::Exact(stable_id.clone()),
            stable_id,
            card_type,
            rarity,
            engine_supported: true,
            token: false,
        })
        .collect::<Vec<_>>();
    let validation =
        validate_deck(candidate, &catalog, FormatContext::constructed()).expect("valid deck");
    assert!(validation.ranked_eligible(), "soak deck must be rankable");
    (validation, catalog)
}

fn priced_decks(
    baseline: &DeckValidation,
    baseline_catalog: &[CardCatalogEntry],
    variant: &DeckValidation,
    variant_catalog: &[CardCatalogEntry],
) -> (DeckCost, DeckCost) {
    let catalog = baseline_catalog
        .iter()
        .chain(variant_catalog)
        .map(|card| (card.stable_id.as_str(), card))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .cloned()
        .collect::<Vec<_>>();
    let scope = PriceScope {
        variant: "standard".to_owned(),
        condition: "near-mint".to_owned(),
        currency: "USD".to_owned(),
        source: "selfplay-soak".to_owned(),
    };
    let prices = catalog
        .iter()
        .map(|card| PrintingPrice {
            key: PriceKey {
                official_card_id: card.stable_id.clone(),
                printing_id: format!("printing:{}", card.stable_id),
                variant: scope.variant.clone(),
                condition: scope.condition.clone(),
                currency: scope.currency.clone(),
                source: scope.source.clone(),
            },
            unit_price_cents: 1,
        })
        .collect();
    let snapshot =
        PriceSnapshot::new("selfplay-soak-prices-v1".to_owned(), prices).expect("price snapshot");
    (
        price_deck(baseline, &catalog, &snapshot, &scope).expect("baseline price"),
        price_deck(variant, &catalog, &snapshot, &scope).expect("variant price"),
    )
}

fn policy(manifest: &str, deck_id: &IdentityHash, feature_priority: [&str; 9]) -> PolicySnapshot {
    let manifest: Value = serde_json::from_str(manifest).expect("manifest JSON");
    let mut value = json!({
        "authorityHash": manifest["authority"]["contentHash"],
        "deckId": deck_id,
        "engineVersion": "sorcery-core-v1",
        "generation": 0,
        "observationVersion": "seat-observation-v1",
        "schemaVersion": 1,
        "selector": {
            "atlasReserve": 3,
            "featurePriority": feature_priority,
        },
        "tieBreak": "canonical-action-order-v1",
    });
    value["policyId"] = json!(identity_hash(&value).expect("policy identity"));
    parse_policy_snapshot(&canonical_json(&value).expect("canonical policy")).expect("valid policy")
}

fn comparison_evidence(result: &DeckComparisonResult) -> Value {
    json!({
        "classification": result.classification,
        "selected": {
            "deckId": result.standings[0].deck_id,
            "games": result.selected_score.games(),
            "halfPoints": result.selected_score.half_points(),
            "pairs": result.selected_score.pairs().iter().map(|pair| json!({
                "halfPoints": pair.half_points(),
                "manifestIds": pair.manifest_ids(),
                "opponentPolicyId": pair.opponent_policy_id(),
                "seatHalfPoints": pair.seat_half_points(),
                "seed": pair.seed(),
                "subgroup": pair.subgroup(),
            })).collect::<Vec<_>>(),
        },
        "standings": result.standings.iter().map(|standing| json!({
            "costCents": standing.cost_cents,
            "deckId": standing.deck_id,
            "games": standing.score.games(),
            "halfPoints": standing.score.half_points(),
        })).collect::<Vec<_>>(),
    })
}

fn search_evidence(
    pairs: &[SeatPair],
    candidate_policy: &PolicySnapshot,
    opponent_policies: &[PolicySnapshot; 2],
) -> Value {
    let mut orientations = Vec::with_capacity(pairs.len() * 2);
    for (pair_index, pair) in pairs.iter().enumerate() {
        let opponent = &opponent_policies[pair_index % opponent_policies.len()];
        for (seat, manifest, north, south) in [
            (
                "north",
                pair.candidate_north.as_str(),
                candidate_policy,
                opponent,
            ),
            (
                "south",
                pair.candidate_south.as_str(),
                opponent,
                candidate_policy,
            ),
        ] {
            let game = Game::from_manifest_json(manifest).expect("search game");
            let rollouts = search_root_actions(&game, north, south, MAX_ACTIONS, MAX_ROOT_ACTIONS)
                .expect("bounded production search");
            assert!(!rollouts.is_empty(), "search root must have legal branches");
            assert!(
                rollouts.iter().all(Rollout::is_terminal),
                "every searched branch must terminate within the bound"
            );
            let selected = &rollouts[0];
            let replay =
                replay_selected(manifest, selected).expect("authoritative selected replay");
            assert!(replay.verify_replay().expect("replay verification"));
            assert_eq!(replay.outcome(), selected.outcome());
            orientations.push(json!({
                "actionIndices": selected.action_indices(),
                "branchCount": rollouts.len(),
                "manifestId": replay.manifest_id(),
                "outcome": format!("{:?}", selected.outcome()),
                "replay": replay.replay_value().expect("replay evidence"),
                "sessionHash": replay.session_hash().expect("session hash"),
                "stateHash": replay.state_hash().expect("state hash"),
                "transcriptHash": replay.transcript_hash().expect("transcript hash"),
                "candidateSeat": seat,
            }));
        }
    }
    json!(orientations)
}

fn run_soak(
    candidates: &[DeckComparisonCandidate<'_>],
    baseline_pairs: &[SeatPair],
    baseline_policy: &PolicySnapshot,
    variant_pairs: &[SeatPair],
    variant_policy: &PolicySnapshot,
    opponent_policies: &[PolicySnapshot; 2],
) -> (DeckComparisonResult, Value) {
    let comparison = compare_decks(candidates, 10_000, MAX_ACTIONS).expect("deck comparison");
    let (selected_pairs, selected_policy) =
        if comparison.standings[0].deck_id == *baseline_policy.deck_id() {
            (baseline_pairs, baseline_policy)
        } else {
            (variant_pairs, variant_policy)
        };
    let evidence = json!({
        "comparison": comparison_evidence(&comparison),
        "search": search_evidence(selected_pairs, selected_policy, opponent_policies),
    });
    (comparison, evidence)
}

fn assert_portfolio_contract(comparison: &DeckComparisonResult) {
    assert_eq!(comparison.selected_score.games(), 8);
    assert_eq!(comparison.selected_score.pairs().len(), SEEDS.len());
    assert_eq!(
        comparison
            .selected_score
            .pairs()
            .iter()
            .map(SelfPlayPairScore::seed)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(SEEDS)
    );
    assert_eq!(
        comparison
            .selected_score
            .pairs()
            .iter()
            .map(SelfPlayPairScore::opponent_policy_id)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
}

fn print_summary(
    comparison: &DeckComparisonResult,
    evidence: &Value,
    candidate_count: usize,
    opponent_policy_count: usize,
    first_elapsed_millis: u128,
    second_elapsed_millis: u128,
) {
    println!(
        "{}",
        canonical_json(&json!({
            "candidateCount": candidate_count,
            "determinismEvidenceHash": identity_hash(evidence).expect("evidence hash"),
            "firstRunMilliseconds": first_elapsed_millis,
            "maxActions": MAX_ACTIONS,
            "maxRootActions": MAX_ROOT_ACTIONS,
            "opponentPolicyCount": opponent_policy_count,
            "orientations": SEEDS.len() * 2,
            "secondRunMilliseconds": second_elapsed_millis,
            "seedCount": SEEDS.len(),
            "selectedDeckId": comparison.standings[0].deck_id,
            "selectedGames": comparison.selected_score.games(),
        }))
        .expect("canonical soak summary")
    );
}

fn selfplay_suite<'a>(
    pairs: &'a [SeatPair],
    opponent_policies: &'a [PolicySnapshot; 2],
    opponent_deck: &'a DeckValidation,
) -> Vec<SelfPlayPair<'a>> {
    pairs
        .iter()
        .enumerate()
        .map(|(index, pair)| SelfPlayPair {
            seed: SEEDS[index],
            subgroup: if index % 2 == 0 {
                "baseline-opponent"
            } else {
                "pressure-opponent"
            },
            candidate_as_north_manifest_json: &pair.candidate_north,
            candidate_as_south_manifest_json: &pair.candidate_south,
            opponent: &opponent_policies[index % opponent_policies.len()],
            opponent_deck,
        })
        .collect()
}

#[test]
#[ignore = "release-only clean-restart soak; run with cargo test --release --test selfplay_soak -- --ignored --nocapture"]
fn clean_restart_selfplay_search_should_be_bounded_and_byte_deterministic() {
    let baseline_pairs = seat_pairs(false);
    let variant_pairs = seat_pairs(true);
    let (baseline_deck, baseline_catalog) =
        validated_deck(&baseline_pairs[0].candidate_north, "north");
    let (variant_deck, variant_catalog) =
        validated_deck(&variant_pairs[0].candidate_north, "north");
    let (opponent_deck, _) = validated_deck(&baseline_pairs[0].candidate_north, "south");
    let baseline_policy = policy(
        &baseline_pairs[0].candidate_north,
        baseline_deck.deck_id(),
        BASELINE_FEATURES,
    );
    let variant_policy = policy(
        &variant_pairs[0].candidate_north,
        variant_deck.deck_id(),
        BASELINE_FEATURES,
    );
    let opponent_policies = [
        policy(
            &baseline_pairs[0].candidate_north,
            opponent_deck.deck_id(),
            BASELINE_FEATURES,
        ),
        policy(
            &baseline_pairs[0].candidate_north,
            opponent_deck.deck_id(),
            PRESSURE_FEATURES,
        ),
    ];
    let (baseline_cost, variant_cost) = priced_decks(
        &baseline_deck,
        &baseline_catalog,
        &variant_deck,
        &variant_catalog,
    );

    let baseline_suite = selfplay_suite(&baseline_pairs, &opponent_policies, &opponent_deck);
    let variant_suite = selfplay_suite(&variant_pairs, &opponent_policies, &opponent_deck);
    let candidates = [
        DeckComparisonCandidate {
            cost: &baseline_cost,
            deck: &baseline_deck,
            pairs: &baseline_suite,
            policy: &baseline_policy,
        },
        DeckComparisonCandidate {
            cost: &variant_cost,
            deck: &variant_deck,
            pairs: &variant_suite,
            policy: &variant_policy,
        },
    ];

    let first_started = Instant::now();
    let (first_comparison, first_evidence) = run_soak(
        &candidates,
        &baseline_pairs,
        &baseline_policy,
        &variant_pairs,
        &variant_policy,
        &opponent_policies,
    );
    let first_elapsed = first_started.elapsed();
    let second_started = Instant::now();
    let (second_comparison, second_evidence) = run_soak(
        &candidates,
        &baseline_pairs,
        &baseline_policy,
        &variant_pairs,
        &variant_policy,
        &opponent_policies,
    );
    let second_elapsed = second_started.elapsed();

    assert_eq!(first_comparison, second_comparison);
    let first_bytes = canonical_json(&first_evidence).expect("first canonical evidence");
    let second_bytes = canonical_json(&second_evidence).expect("second canonical evidence");
    assert_eq!(first_bytes.as_bytes(), second_bytes.as_bytes());
    assert_portfolio_contract(&first_comparison);
    print_summary(
        &first_comparison,
        &first_evidence,
        candidates.len(),
        opponent_policies.len(),
        first_elapsed.as_millis(),
        second_elapsed.as_millis(),
    );
}
