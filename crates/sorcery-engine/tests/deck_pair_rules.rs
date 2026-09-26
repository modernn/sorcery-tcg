//! Direct proofs for synthetic deck-pair harness (RULE-CATALOG-0377–0378).

use std::collections::BTreeMap;

use serde_json::{Value, json};
use sorcery_engine::batch::{BatchJob, run_game_batch};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Seat};
use sorcery_engine::deck::{
    CandidateDeck, CardCatalogEntry, CardCount, CardType, DeckValidation, FormatContext,
    OfficialCardMapping, Rarity, validate_deck,
};
use sorcery_engine::game::Game;
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::simulator::{replay_selected, run_game};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const OPENING_FEATURES: [&str; 9] = [
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

fn manifest_with_id(mut manifest: Value) -> String {
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical manifest")
}

fn deck_pair_manifest(seed: u32) -> String {
    let mut manifest: Value =
        serde_json::from_str(&synthetic_demo_manifest_json(seed).expect("synthetic manifest"))
            .expect("manifest JSON");
    for seat in ["north", "south"] {
        let spellbook = manifest["decks"][seat]["spellbook"]
            .as_array_mut()
            .expect("spellbook");
        spellbook.extend(spellbook[..10].to_vec());
    }
    manifest_with_id(manifest)
}

fn authority_hash(manifest: &str) -> String {
    serde_json::from_str::<Value>(manifest).expect("manifest JSON")["authority"]["contentHash"]
        .as_str()
        .expect("authority content hash")
        .to_owned()
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

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
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

fn committed_action_kinds(session: &Session) -> Vec<String> {
    let mut replay = Session::new(session.manifest_json()).expect("replay session");
    let mut kinds = Vec::new();
    for receipt in session.transcript() {
        let action = replay
            .legal_actions()
            .expect("replay legal actions")
            .into_iter()
            .find(|action| action.action_id == receipt.action_id)
            .expect("committed action must remain legal at replay point");
        kinds.push(
            action.descriptor["kind"]
                .as_str()
                .expect("action kind")
                .to_owned(),
        );
        let StepResult::Accepted(replay_step) = replay
            .step(ActionRequest {
                action_id: receipt.action_id.to_string(),
                seat: replay.acting_controller(),
                state_version: replay.state_version(),
            })
            .expect("replay step")
        else {
            panic!("committed action must be accepted during kind extraction");
        };
        assert_eq!(replay_step.receipt_id, receipt.receipt_id);
    }
    kinds
}

fn event_types_for_seat(session: &Session, seat: Seat) -> Vec<&str> {
    session
        .transcript()
        .iter()
        .filter(|receipt| receipt.seat == seat)
        .flat_map(|receipt| receipt.events.iter().map(|event| event.event_type.as_str()))
        .collect()
}

#[test]
fn rule_catalog_0377_two_distinct_synthetic_decks_complete_short_deterministic_opening() {
    let manifest = deck_pair_manifest(377);
    let north_deck = validated_manifest_deck(&manifest, "north");
    let south_deck = validated_manifest_deck(&manifest, "south");
    assert_ne!(
        north_deck.deck_id(),
        south_deck.deck_id(),
        "north and south must be distinct deck identities"
    );
    let authority = authority_hash(&manifest);
    let north_policy = policy(&authority, north_deck.deck_id().as_str(), OPENING_FEATURES);
    let south_policy = policy(&authority, south_deck.deck_id().as_str(), OPENING_FEATURES);
    let game = Game::from_manifest_json(&manifest).expect("valid deck-pair game");
    let rollout = run_game(game, &north_policy, &south_policy, 80).expect("opening rollout");
    let session = replay_selected(&manifest, &rollout).expect("authoritative opening replay");

    let action_kinds = committed_action_kinds(&session);
    assert!(
        action_kinds.iter().any(|kind| kind == "mulligan"),
        "opening must include mulligan decisions"
    );
    assert!(
        action_kinds.iter().any(|kind| kind == "play-site"),
        "opening must establish at least one domain"
    );
    assert!(
        action_kinds.iter().any(|kind| kind == "summon-minion"),
        "opening must include at least one summon"
    );
    assert!(
        event_types_for_seat(&session, Seat::North).contains(&"minion-summoned"),
        "north must summon at least one minion"
    );
    assert!(
        event_types_for_seat(&session, Seat::South).contains(&"minion-summoned"),
        "south must summon at least one minion"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0378_deck_pair_batch_reproduces_transcript_hash_for_manifest_and_seed() {
    let manifest = deck_pair_manifest(378);
    let north_deck = validated_manifest_deck(&manifest, "north");
    let south_deck = validated_manifest_deck(&manifest, "south");
    let authority = authority_hash(&manifest);
    let north_policy = policy(&authority, north_deck.deck_id().as_str(), OPENING_FEATURES);
    let south_policy = policy(&authority, south_deck.deck_id().as_str(), OPENING_FEATURES);
    let job = BatchJob {
        manifest_json: &manifest,
        north_deck_id: north_deck.deck_id(),
        north_policy: &north_policy,
        south_deck_id: south_deck.deck_id(),
        south_policy: &south_policy,
    };

    let first = run_game_batch(&[job], 1).expect("first deck-pair batch");
    let second = run_game_batch(&[job], 2).expect("second deck-pair batch");

    assert_eq!(first.len(), 1);
    assert_eq!(first, second);
    assert!(first[0].report.replay_verified);
    assert_eq!(
        first[0].report.transcript_hash,
        second[0].report.transcript_hash
    );
    assert!(first[0].report.accepted_action_count > 0);
    assert_eq!(
        first[0].report.classification,
        sorcery_engine::batch::BatchClassification::UnrankedPartialRulesUnverifiedAuthority
    );
}

fn all_event_types(session: &Session) -> Vec<&str> {
    session
        .transcript()
        .iter()
        .flat_map(|receipt| receipt.events.iter().map(|event| event.event_type.as_str()))
        .collect()
}

#[test]
fn rule_catalog_0503_deck_pair_reaches_combat_with_verified_replay() {
    let manifest = deck_pair_manifest(503);
    let north_deck = validated_manifest_deck(&manifest, "north");
    let south_deck = validated_manifest_deck(&manifest, "south");
    assert_ne!(
        north_deck.deck_id(),
        south_deck.deck_id(),
        "north and south must be distinct deck identities"
    );
    let authority = authority_hash(&manifest);
    let north_policy = policy(&authority, north_deck.deck_id().as_str(), OPENING_FEATURES);
    let south_policy = policy(&authority, south_deck.deck_id().as_str(), OPENING_FEATURES);
    let game = Game::from_manifest_json(&manifest).expect("valid deck-pair game");
    let rollout = run_game(game, &north_policy, &south_policy, 120).expect("combat rollout");
    let session = replay_selected(&manifest, &rollout).expect("combat replay");
    let action_kinds = committed_action_kinds(&session);
    let event_types = all_event_types(&session);

    assert!(
        action_kinds.iter().any(|kind| kind == "declare-attack"),
        "extended opening must commit at least one attack"
    );
    assert!(
        action_kinds.iter().any(|kind| kind == "close-defend"),
        "extended opening must resolve at least one defend choice"
    );
    assert!(
        event_types.contains(&"fight-started"),
        "extended opening must reach at least one fight"
    );
    assert!(
        rollout.action_indices().len() >= 100,
        "combat rollout should exceed the short-opening bound"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0504_deck_pair_completes_terminal_game_with_verified_replay() {
    let manifest = deck_pair_manifest(504);
    let north_deck = validated_manifest_deck(&manifest, "north");
    let south_deck = validated_manifest_deck(&manifest, "south");
    assert_ne!(
        north_deck.deck_id(),
        south_deck.deck_id(),
        "north and south must be distinct deck identities"
    );
    let authority = authority_hash(&manifest);
    let north_policy = policy(&authority, north_deck.deck_id().as_str(), OPENING_FEATURES);
    let south_policy = policy(&authority, south_deck.deck_id().as_str(), OPENING_FEATURES);
    let game = Game::from_manifest_json(&manifest).expect("valid deck-pair game");
    let rollout = run_game(game, &north_policy, &south_policy, 300).expect("terminal rollout");
    let session = replay_selected(&manifest, &rollout).expect("terminal replay");

    assert!(
        rollout.is_terminal(),
        "rollout must reach a terminal outcome"
    );
    assert_eq!(
        rollout.outcome(),
        Some(sorcery_engine::game::GameOutcome::Win {
            loser: Seat::North,
            winner: Seat::South,
        })
    );
    assert!(
        all_event_types(&session).contains(&"fight-started"),
        "terminal deck-pair game must include at least one fight"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0505_deck_pair_terminal_batch_reproduces_transcript_hash() {
    let manifest = deck_pair_manifest(504);
    let north_deck = validated_manifest_deck(&manifest, "north");
    let south_deck = validated_manifest_deck(&manifest, "south");
    let authority = authority_hash(&manifest);
    let north_policy = policy(&authority, north_deck.deck_id().as_str(), OPENING_FEATURES);
    let south_policy = policy(&authority, south_deck.deck_id().as_str(), OPENING_FEATURES);
    let job = BatchJob {
        manifest_json: &manifest,
        north_deck_id: north_deck.deck_id(),
        north_policy: &north_policy,
        south_deck_id: south_deck.deck_id(),
        south_policy: &south_policy,
    };

    let first = run_game_batch(&[job], 1).expect("first terminal deck-pair batch");
    let second = run_game_batch(&[job], 2).expect("second terminal deck-pair batch");

    assert_eq!(first.len(), 1);
    assert_eq!(first, second);
    assert!(first[0].report.replay_verified);
    assert_eq!(first[0].report.terminal.winner(), Some(Seat::South));
    assert_eq!(
        first[0].report.transcript_hash,
        second[0].report.transcript_hash
    );
    assert!(first[0].report.accepted_action_count >= 200);
    assert_eq!(
        first[0].report.classification,
        sorcery_engine::batch::BatchClassification::UnrankedPartialRulesUnverifiedAuthority
    );
}
