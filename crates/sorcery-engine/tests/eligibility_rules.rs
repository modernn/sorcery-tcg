//! Direct proofs for TEST-04 ranked-ready eligibility classification.

use serde_json::{Value, json};
use sorcery_engine::batch::{BatchClassification, MAX_GAME_ACTIONS};
use sorcery_engine::canonical::{CanonicalError, IdentityHash, canonical_json, identity_hash};
use sorcery_engine::eligibility::{
    EligibilityReason, TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH, eligibility_policy_for_manifest,
    eligibility_policy_for_manifest_jsons, evaluate_eligibility_with_policy,
};
use sorcery_engine::game_record::record_policy_game;
use sorcery_engine::game_record::record_synthetic_demo;
use sorcery_engine::policy::{BASELINE_POLICY_DECK_ID, baseline_policy_snapshot};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

fn verified_private_local_manifest(seed: u32) -> Result<String, CanonicalError> {
    let mut manifest: Value =
        serde_json::from_str(&synthetic_demo_manifest_json(seed)?).expect("manifest JSON");
    let object = manifest.as_object_mut().expect("manifest object");
    object.remove("manifestId");
    object["authority"] = json!({
        "contentHash": TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH,
        "mode": "private-local",
        "revisionId": "eligibility-scenario-fixture-v1",
    });
    manifest["manifestId"] = json!(identity_hash(&manifest)?);
    canonical_json(&manifest)
}

fn non_allowlisted_private_local_manifest(seed: u32) -> Result<String, CanonicalError> {
    let mut manifest: Value =
        serde_json::from_str(&synthetic_demo_manifest_json(seed)?).expect("manifest JSON");
    let object = manifest.as_object_mut().expect("manifest object");
    object.remove("manifestId");
    object["authority"] = json!({
        "contentHash": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "mode": "private-local",
        "revisionId": "eligibility-scenario-fixture-v1",
    });
    manifest["manifestId"] = json!(identity_hash(&manifest)?);
    canonical_json(&manifest)
}

#[test]
fn rule_catalog_0506_finished_synthetic_game_classifies_unranked_unverified_authority() {
    let record = record_synthetic_demo(31).expect("seed-31 finished synthetic record");

    assert!(record.replay_verified);
    assert!(record.eligibility.gates.all_passed());
    assert!(!record.eligibility.ranked);
    assert_eq!(
        record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        record.eligibility.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        record.eligibility.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );
}

#[test]
fn rule_catalog_0881_private_local_allowlisted_manifest_classifies_ranked() {
    let manifest_json = verified_private_local_manifest(31).expect("verified manifest");
    let game = sorcery_engine::game::Game::from_manifest_json(&manifest_json).expect("game");
    let policy =
        baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
            .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let record = record_policy_game(
        &manifest_json,
        &deck_id,
        &policy,
        &deck_id,
        &policy,
        MAX_GAME_ACTIONS,
    )
    .expect("seed-31 finished verified record");

    assert!(record.replay_verified);
    assert!(record.eligibility.gates.all_passed());
    assert!(record.eligibility.ranked);
    assert_eq!(record.classification, BatchClassification::Ranked);
    assert_eq!(
        record.eligibility.classification,
        BatchClassification::Ranked
    );
    assert!(record.eligibility.reasons.is_empty());
}

#[test]
fn rule_catalog_0882_mixed_manifest_batch_keeps_unverified_authority_policy() {
    let verified = verified_private_local_manifest(31).expect("verified manifest");
    let synthetic = synthetic_demo_manifest_json(31).expect("synthetic manifest");

    let verified_only = eligibility_policy_for_manifest_jsons([verified.as_str()]);
    assert!(verified_only.authority_verified);

    let mixed = eligibility_policy_for_manifest_jsons([verified.as_str(), synthetic.as_str()]);
    assert!(!mixed.authority_verified);

    let synthetic_only = eligibility_policy_for_manifest_jsons([synthetic.as_str()]);
    assert!(!synthetic_only.authority_verified);
}

#[test]
fn rule_catalog_0883_synthetic_mode_rejects_allowlisted_authority_hash() {
    let policy = eligibility_policy_for_manifest(&json!({
        "authority": {
            "contentHash": TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH,
            "mode": "synthetic",
        },
    }));
    assert!(!policy.authority_verified);
}

#[test]
fn rule_catalog_0884_private_local_non_allowlisted_hash_stays_unverified() {
    let policy = eligibility_policy_for_manifest(&json!({
        "authority": {
            "contentHash": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            "mode": "private-local",
        },
    }));
    assert!(!policy.authority_verified);
}

#[test]
fn rule_catalog_0942_mixed_verified_and_synthetic_batch_downgrades_verified_game() {
    let verified_manifest = verified_private_local_manifest(31).expect("verified manifest");
    let synthetic = synthetic_demo_manifest_json(31).expect("synthetic manifest");
    let game = sorcery_engine::game::Game::from_manifest_json(&verified_manifest).expect("game");
    let policy =
        baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
            .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let record = record_policy_game(
        &verified_manifest,
        &deck_id,
        &policy,
        &deck_id,
        &policy,
        MAX_GAME_ACTIONS,
    )
    .expect("seed-31 finished verified record");

    assert!(record.replay_verified);
    assert!(record.eligibility.gates.all_passed());
    assert!(record.eligibility.ranked);
    assert_eq!(record.classification, BatchClassification::Ranked);

    let batch_policy =
        eligibility_policy_for_manifest_jsons([verified_manifest.as_str(), synthetic.as_str()]);
    assert!(!batch_policy.authority_verified);

    let batch_eligibility =
        evaluate_eligibility_with_policy(record.eligibility.gates, batch_policy);
    assert!(!batch_eligibility.ranked);
    assert_eq!(
        batch_eligibility.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        batch_eligibility.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );
}

#[test]
fn rule_catalog_0950_verified_only_batch_keeps_verified_game_ranked() {
    let verified_manifest = verified_private_local_manifest(31).expect("verified manifest");
    let other_verified = verified_private_local_manifest(32).expect("other verified manifest");
    let game = sorcery_engine::game::Game::from_manifest_json(&verified_manifest).expect("game");
    let policy =
        baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
            .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let record = record_policy_game(
        &verified_manifest,
        &deck_id,
        &policy,
        &deck_id,
        &policy,
        MAX_GAME_ACTIONS,
    )
    .expect("seed-31 finished verified record");

    assert!(record.replay_verified);
    assert!(record.eligibility.gates.all_passed());
    assert!(record.eligibility.ranked);
    assert_eq!(record.classification, BatchClassification::Ranked);

    let batch_policy = eligibility_policy_for_manifest_jsons([
        verified_manifest.as_str(),
        other_verified.as_str(),
    ]);
    assert!(batch_policy.authority_verified);

    let batch_eligibility =
        evaluate_eligibility_with_policy(record.eligibility.gates, batch_policy);
    assert!(batch_eligibility.ranked);
    assert_eq!(
        batch_eligibility.classification,
        BatchClassification::Ranked
    );
    assert!(batch_eligibility.reasons.is_empty());
}

#[test]
fn rule_catalog_0961_synthetic_only_batch_keeps_synthetic_game_unranked() {
    let synthetic_manifest = synthetic_demo_manifest_json(31).expect("synthetic manifest");
    let other_synthetic = synthetic_demo_manifest_json(32).expect("other synthetic manifest");
    let record = record_synthetic_demo(31).expect("seed-31 finished synthetic record");

    assert!(record.replay_verified);
    assert!(record.eligibility.gates.all_passed());
    assert!(!record.eligibility.ranked);
    assert_eq!(
        record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let batch_policy = eligibility_policy_for_manifest_jsons([
        synthetic_manifest.as_str(),
        other_synthetic.as_str(),
    ]);
    assert!(!batch_policy.authority_verified);

    let batch_eligibility =
        evaluate_eligibility_with_policy(record.eligibility.gates, batch_policy);
    assert!(!batch_eligibility.ranked);
    assert_eq!(
        batch_eligibility.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        batch_eligibility.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );
}

#[test]
fn rule_catalog_0982_mixed_allowlisted_and_non_allowlisted_private_local_batch_stays_unranked_for_both()
 {
    let allowlisted_manifest = verified_private_local_manifest(31).expect("allowlisted manifest");
    let non_allowlisted_manifest =
        non_allowlisted_private_local_manifest(32).expect("non-allowlisted manifest");

    let allowlisted_game =
        sorcery_engine::game::Game::from_manifest_json(&allowlisted_manifest).expect("game");
    let allowlisted_policy = baseline_policy_snapshot(
        allowlisted_game.rules().authority_hash(),
        allowlisted_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let allowlisted_record = record_policy_game(
        &allowlisted_manifest,
        &deck_id,
        &allowlisted_policy,
        &deck_id,
        &allowlisted_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("allowlisted finished record");

    assert!(allowlisted_record.replay_verified);
    assert!(allowlisted_record.eligibility.gates.all_passed());
    assert!(allowlisted_record.eligibility.ranked);
    assert_eq!(
        allowlisted_record.classification,
        BatchClassification::Ranked
    );

    let non_allowlisted_game =
        sorcery_engine::game::Game::from_manifest_json(&non_allowlisted_manifest).expect("game");
    let non_allowlisted_policy = baseline_policy_snapshot(
        non_allowlisted_game.rules().authority_hash(),
        non_allowlisted_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let non_allowlisted_record = record_policy_game(
        &non_allowlisted_manifest,
        &deck_id,
        &non_allowlisted_policy,
        &deck_id,
        &non_allowlisted_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("non-allowlisted finished record");

    assert!(non_allowlisted_record.replay_verified);
    assert!(non_allowlisted_record.eligibility.gates.all_passed());
    assert!(!non_allowlisted_record.eligibility.ranked);
    assert_eq!(
        non_allowlisted_record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let batch_policy = eligibility_policy_for_manifest_jsons([
        allowlisted_manifest.as_str(),
        non_allowlisted_manifest.as_str(),
    ]);
    assert!(!batch_policy.authority_verified);

    let allowlisted_batch =
        evaluate_eligibility_with_policy(allowlisted_record.eligibility.gates, batch_policy);
    assert!(!allowlisted_batch.ranked);
    assert_eq!(
        allowlisted_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        allowlisted_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );

    let non_allowlisted_batch =
        evaluate_eligibility_with_policy(non_allowlisted_record.eligibility.gates, batch_policy);
    assert!(!non_allowlisted_batch.ranked);
    assert_eq!(
        non_allowlisted_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        non_allowlisted_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );
}

#[test]
fn rule_catalog_1001_mixed_allowlisted_private_local_and_synthetic_batch_stays_unranked_for_both() {
    let allowlisted_manifest = verified_private_local_manifest(31).expect("allowlisted manifest");
    let synthetic_manifest = synthetic_demo_manifest_json(32).expect("synthetic manifest");

    let allowlisted_game =
        sorcery_engine::game::Game::from_manifest_json(&allowlisted_manifest).expect("game");
    let allowlisted_policy = baseline_policy_snapshot(
        allowlisted_game.rules().authority_hash(),
        allowlisted_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let allowlisted_record = record_policy_game(
        &allowlisted_manifest,
        &deck_id,
        &allowlisted_policy,
        &deck_id,
        &allowlisted_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("allowlisted finished record");

    assert!(allowlisted_record.replay_verified);
    assert!(allowlisted_record.eligibility.gates.all_passed());
    assert!(allowlisted_record.eligibility.ranked);
    assert_eq!(
        allowlisted_record.classification,
        BatchClassification::Ranked
    );

    let synthetic_record = record_synthetic_demo(32).expect("seed-32 finished synthetic record");

    assert!(synthetic_record.replay_verified);
    assert!(synthetic_record.eligibility.gates.all_passed());
    assert!(!synthetic_record.eligibility.ranked);
    assert_eq!(
        synthetic_record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let batch_policy = eligibility_policy_for_manifest_jsons([
        allowlisted_manifest.as_str(),
        synthetic_manifest.as_str(),
    ]);
    assert!(!batch_policy.authority_verified);

    let allowlisted_batch =
        evaluate_eligibility_with_policy(allowlisted_record.eligibility.gates, batch_policy);
    assert!(!allowlisted_batch.ranked);
    assert_eq!(
        allowlisted_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        allowlisted_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );

    let synthetic_batch =
        evaluate_eligibility_with_policy(synthetic_record.eligibility.gates, batch_policy);
    assert!(!synthetic_batch.ranked);
    assert_eq!(
        synthetic_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        synthetic_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );
}

#[test]
fn rule_catalog_1012_three_way_verified_private_synthetic_batch_downgrades_all_games() {
    let allowlisted_manifest = verified_private_local_manifest(31).expect("allowlisted manifest");
    let non_allowlisted_manifest =
        non_allowlisted_private_local_manifest(32).expect("non-allowlisted manifest");
    let synthetic_manifest = synthetic_demo_manifest_json(33).expect("synthetic manifest");

    let allowlisted_game =
        sorcery_engine::game::Game::from_manifest_json(&allowlisted_manifest).expect("game");
    let allowlisted_policy = baseline_policy_snapshot(
        allowlisted_game.rules().authority_hash(),
        allowlisted_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let allowlisted_record = record_policy_game(
        &allowlisted_manifest,
        &deck_id,
        &allowlisted_policy,
        &deck_id,
        &allowlisted_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("allowlisted finished record");

    assert!(allowlisted_record.replay_verified);
    assert!(allowlisted_record.eligibility.gates.all_passed());
    assert!(allowlisted_record.eligibility.ranked);
    assert_eq!(
        allowlisted_record.classification,
        BatchClassification::Ranked
    );

    let non_allowlisted_game =
        sorcery_engine::game::Game::from_manifest_json(&non_allowlisted_manifest).expect("game");
    let non_allowlisted_policy = baseline_policy_snapshot(
        non_allowlisted_game.rules().authority_hash(),
        non_allowlisted_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let non_allowlisted_record = record_policy_game(
        &non_allowlisted_manifest,
        &deck_id,
        &non_allowlisted_policy,
        &deck_id,
        &non_allowlisted_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("non-allowlisted finished record");

    assert!(non_allowlisted_record.replay_verified);
    assert!(non_allowlisted_record.eligibility.gates.all_passed());
    assert!(!non_allowlisted_record.eligibility.ranked);
    assert_eq!(
        non_allowlisted_record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let synthetic_record = record_synthetic_demo(33).expect("seed-33 finished synthetic record");

    assert!(synthetic_record.replay_verified);
    assert!(synthetic_record.eligibility.gates.all_passed());
    assert!(!synthetic_record.eligibility.ranked);
    assert_eq!(
        synthetic_record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let batch_policy = eligibility_policy_for_manifest_jsons([
        allowlisted_manifest.as_str(),
        non_allowlisted_manifest.as_str(),
        synthetic_manifest.as_str(),
    ]);
    assert!(!batch_policy.authority_verified);

    let allowlisted_batch =
        evaluate_eligibility_with_policy(allowlisted_record.eligibility.gates, batch_policy);
    assert!(!allowlisted_batch.ranked);
    assert_eq!(
        allowlisted_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        allowlisted_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );

    let non_allowlisted_batch =
        evaluate_eligibility_with_policy(non_allowlisted_record.eligibility.gates, batch_policy);
    assert!(!non_allowlisted_batch.ranked);
    assert_eq!(
        non_allowlisted_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        non_allowlisted_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );

    let synthetic_batch =
        evaluate_eligibility_with_policy(synthetic_record.eligibility.gates, batch_policy);
    assert!(!synthetic_batch.ranked);
    assert_eq!(
        synthetic_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        synthetic_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );
}

#[test]
fn rule_catalog_1122_synthetic_batch_keeps_all_games_unranked_unverified_authority() {
    let seeds = [31_u32, 32, 33];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 3);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            record.eligibility.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            record.eligibility.reasons,
            [EligibilityReason::UnverifiedAuthority]
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);

    for record in &records {
        let batch_eligibility =
            evaluate_eligibility_with_policy(record.eligibility.gates, batch_policy);
        assert!(!batch_eligibility.ranked);
        assert_eq!(
            batch_eligibility.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            batch_eligibility.reasons,
            [EligibilityReason::UnverifiedAuthority]
        );
    }
}

#[test]
fn rule_catalog_1172_multi_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [31_u32, 32];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert!(manifests.len() > 1);
    assert_eq!(records.len(), manifests.len());
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            record.eligibility.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            record.eligibility.reasons,
            [EligibilityReason::UnverifiedAuthority]
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);

    for record in &records {
        let batch_eligibility =
            evaluate_eligibility_with_policy(record.eligibility.gates, batch_policy);
        assert!(!batch_eligibility.ranked);
        assert_eq!(
            batch_eligibility.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            batch_eligibility.reasons,
            [EligibilityReason::UnverifiedAuthority]
        );
    }
}

#[test]
fn rule_catalog_1182_three_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [40_u32, 41, 42];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 3);
    assert_eq!(records.len(), 3);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            record.eligibility.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            record.eligibility.reasons,
            [EligibilityReason::UnverifiedAuthority]
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);

    for record in &records {
        let batch_eligibility =
            evaluate_eligibility_with_policy(record.eligibility.gates, batch_policy);
        assert!(!batch_eligibility.ranked);
        assert_eq!(
            batch_eligibility.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
        assert_eq!(
            batch_eligibility.reasons,
            [EligibilityReason::UnverifiedAuthority]
        );
    }
}
