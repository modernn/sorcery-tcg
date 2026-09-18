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

#[test]
fn rule_catalog_1191_four_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [43_u32, 44, 45, 46];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 4);
    assert_eq!(records.len(), 4);
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
fn rule_catalog_1192_five_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [47_u32, 48, 49, 50, 51];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 5);
    assert_eq!(records.len(), 5);
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
fn rule_catalog_1198_six_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [52_u32, 53, 54, 55, 56, 57];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 6);
    assert_eq!(records.len(), 6);
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
fn rule_catalog_1199_seven_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [58_u32, 59, 60, 61, 62, 63, 64];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 7);
    assert_eq!(records.len(), 7);
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
fn rule_catalog_1200_eight_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [65_u32, 66, 67, 68, 69, 70, 71, 72];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 8);
    assert_eq!(records.len(), 8);
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
fn rule_catalog_1201_nine_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [73_u32, 74, 75, 76, 77, 78, 79, 80, 81];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 9);
    assert_eq!(records.len(), 9);
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
fn rule_catalog_1202_ten_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [82_u32, 83, 84, 85, 86, 87, 88, 89, 90, 91];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 10);
    assert_eq!(records.len(), 10);
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
fn rule_catalog_1211_eleven_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [92_u32, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 11);
    assert_eq!(records.len(), 11);
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
fn rule_catalog_1212_twelve_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        103_u32, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 12);
    assert_eq!(records.len(), 12);
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
fn rule_catalog_1221_thirteen_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        115_u32, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 13);
    assert_eq!(records.len(), 13);
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
fn rule_catalog_1222_fourteen_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        128_u32, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 14);
    assert_eq!(records.len(), 14);
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
fn rule_catalog_1231_fifteen_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        142_u32, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 15);
    assert_eq!(records.len(), 15);
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
fn rule_catalog_1242_seventeen_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        173_u32, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 17);
    assert_eq!(records.len(), 17);
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
fn rule_catalog_1251_eighteen_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        190_u32, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206,
        207,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 18);
    assert_eq!(records.len(), 18);
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
fn rule_catalog_1252_nineteen_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        208_u32, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224,
        225, 226,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 19);
    assert_eq!(records.len(), 19);
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
fn rule_catalog_1261_twenty_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        227_u32, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243,
        244, 245, 246,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 20);
    assert_eq!(records.len(), 20);
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
fn rule_catalog_1281_twenty_two_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        268_u32, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281, 282, 283, 284,
        285, 286, 287, 288, 289,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 22);
    assert_eq!(records.len(), 22);
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
fn rule_catalog_1335_twenty_eight_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        415_u32, 416, 417, 418, 419, 420, 421, 422, 423, 424, 425, 426, 427, 428, 429, 430, 431,
        432, 433, 434, 435, 436, 437, 438, 439, 440, 441, 442,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 28);
    assert_eq!(records.len(), 28);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1342_twenty_nine_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        443_u32, 444, 445, 446, 447, 448, 449, 450, 451, 452, 453, 454, 455, 456, 457, 458, 459,
        460, 461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 29);
    assert_eq!(records.len(), 29);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1351_thirty_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        472_u32, 473, 474, 475, 476, 477, 478, 479, 480, 481, 482, 483, 484, 485, 486, 487, 488,
        489, 490, 491, 492, 493, 494, 495, 496, 497, 498, 499, 500, 501,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 30);
    assert_eq!(records.len(), 30);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1352_thirty_one_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        502_u32, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513, 514, 515, 516, 517, 518,
        519, 520, 521, 522, 523, 524, 525, 526, 527, 528, 529, 530, 531, 532,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 31);
    assert_eq!(records.len(), 31);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1359_thirty_two_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        533_u32, 534, 535, 536, 537, 538, 539, 540, 541, 542, 543, 544, 545, 546, 547, 548, 549,
        550, 551, 552, 553, 554, 555, 556, 557, 558, 559, 560, 561, 562, 563, 564,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 32);
    assert_eq!(records.len(), 32);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1360_thirty_three_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        565_u32, 566, 567, 568, 569, 570, 571, 572, 573, 574, 575, 576, 577, 578, 579, 580, 581,
        582, 583, 584, 585, 586, 587, 588, 589, 590, 591, 592, 593, 594, 595, 596, 597,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 33);
    assert_eq!(records.len(), 33);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1361_thirty_four_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        598_u32, 599, 600, 601, 602, 603, 604, 605, 606, 607, 608, 609, 610, 611, 612, 613, 614,
        615, 616, 617, 618, 619, 620, 621, 622, 623, 624, 625, 626, 627, 628, 629, 630, 631,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 34);
    assert_eq!(records.len(), 34);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1362_thirty_five_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        632_u32, 633, 634, 635, 636, 637, 638, 639, 640, 641, 642, 643, 644, 645, 646, 647, 648,
        649, 650, 651, 652, 653, 654, 655, 656, 657, 658, 659, 660, 661, 662, 663, 664, 665, 666,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 35);
    assert_eq!(records.len(), 35);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1369_thirty_six_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        667_u32, 668, 669, 670, 671, 672, 673, 674, 675, 676, 677, 678, 679, 680, 681, 682, 683,
        684, 685, 686, 687, 688, 689, 690, 691, 692, 693, 694, 695, 696, 697, 698, 699, 700, 701,
        702,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 36);
    assert_eq!(records.len(), 36);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1370_thirty_seven_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        703_u32, 704, 705, 706, 707, 708, 709, 710, 711, 712, 713, 714, 715, 716, 717, 718, 719,
        720, 721, 722, 723, 724, 725, 726, 727, 728, 729, 730, 731, 732, 733, 734, 735, 736, 737,
        738, 739,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 37);
    assert_eq!(records.len(), 37);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1371_thirty_eight_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        740_u32, 741, 742, 743, 744, 745, 746, 747, 748, 749, 750, 751, 752, 753, 754, 755, 756,
        757, 758, 759, 760, 761, 762, 763, 764, 765, 766, 767, 768, 769, 770, 771, 772, 773, 774,
        775, 776, 777,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 38);
    assert_eq!(records.len(), 38);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1372_thirty_nine_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        778_u32, 779, 780, 781, 782, 783, 784, 785, 786, 787, 788, 789, 790, 791, 792, 793, 794,
        795, 796, 797, 798, 799, 800, 801, 802, 803, 804, 805, 806, 807, 808, 809, 810, 811, 812,
        813, 814, 815, 816,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 39);
    assert_eq!(records.len(), 39);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1383_forty_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        817_u32, 818, 819, 820, 821, 822, 823, 824, 825, 826, 827, 828, 829, 830, 831, 832, 833,
        834, 835, 836, 837, 838, 839, 840, 841, 842, 843, 844, 845, 846, 847, 848, 849, 850, 851,
        852, 853, 854, 855, 856,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 40);
    assert_eq!(records.len(), 40);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1384_forty_one_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        857_u32, 858, 859, 860, 861, 862, 863, 864, 865, 866, 867, 868, 869, 870, 871, 872, 873,
        874, 875, 876, 877, 878, 879, 880, 881, 882, 883, 884, 885, 886, 887, 888, 889, 890, 891,
        892, 893, 894, 895, 896, 897,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 41);
    assert_eq!(records.len(), 41);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1385_forty_two_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        898_u32, 899, 900, 901, 902, 903, 904, 905, 906, 907, 908, 909, 910, 911, 912, 913, 914,
        915, 916, 917, 918, 919, 920, 921, 922, 923, 924, 925, 926, 927, 928, 929, 930, 931, 932,
        933, 934, 935, 936, 937, 938, 939,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 42);
    assert_eq!(records.len(), 42);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1386_forty_three_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        940_u32, 941, 942, 943, 944, 945, 946, 947, 948, 949, 950, 951, 952, 953, 954, 955, 956,
        957, 958, 959, 960, 961, 962, 963, 964, 965, 966, 967, 968, 969, 970, 971, 972, 973, 974,
        975, 976, 977, 978, 979, 980, 981, 982,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 43);
    assert_eq!(records.len(), 43);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1393_forty_four_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        983_u32, 984, 985, 986, 987, 988, 989, 990, 991, 992, 993, 994, 995, 996, 997, 998, 999,
        1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008, 1009, 1010, 1011, 1012, 1013, 1014,
        1015, 1016, 1017, 1018, 1019, 1020, 1021, 1022, 1023, 1024, 1025, 1026,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 44);
    assert_eq!(records.len(), 44);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1394_forty_five_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1027_u32, 1028, 1029, 1030, 1031, 1032, 1033, 1034, 1035, 1036, 1037, 1038, 1039, 1040,
        1041, 1042, 1043, 1044, 1045, 1046, 1047, 1048, 1049, 1050, 1051, 1052, 1053, 1054, 1055,
        1056, 1057, 1058, 1059, 1060, 1061, 1062, 1063, 1064, 1065, 1066, 1067, 1068, 1069, 1070,
        1071,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 45);
    assert_eq!(records.len(), 45);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1395_forty_six_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1072_u32, 1073, 1074, 1075, 1076, 1077, 1078, 1079, 1080, 1081, 1082, 1083, 1084, 1085,
        1086, 1087, 1088, 1089, 1090, 1091, 1092, 1093, 1094, 1095, 1096, 1097, 1098, 1099, 1100,
        1101, 1102, 1103, 1104, 1105, 1106, 1107, 1108, 1109, 1110, 1111, 1112, 1113, 1114, 1115,
        1116, 1117,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 46);
    assert_eq!(records.len(), 46);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1396_forty_seven_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1118_u32, 1119, 1120, 1121, 1122, 1123, 1124, 1125, 1126, 1127, 1128, 1129, 1130, 1131,
        1132, 1133, 1134, 1135, 1136, 1137, 1138, 1139, 1140, 1141, 1142, 1143, 1144, 1145, 1146,
        1147, 1148, 1149, 1150, 1151, 1152, 1153, 1154, 1155, 1156, 1157, 1158, 1159, 1160, 1161,
        1162, 1163, 1164,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 47);
    assert_eq!(records.len(), 47);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1407_forty_eight_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1165_u32, 1166, 1167, 1168, 1169, 1170, 1171, 1172, 1173, 1174, 1175, 1176, 1177, 1178,
        1179, 1180, 1181, 1182, 1183, 1184, 1185, 1186, 1187, 1188, 1189, 1190, 1191, 1192, 1193,
        1194, 1195, 1196, 1197, 1198, 1199, 1200, 1201, 1202, 1203, 1204, 1205, 1206, 1207, 1208,
        1209, 1210, 1211, 1212,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 48);
    assert_eq!(records.len(), 48);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1408_forty_nine_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1213_u32, 1214, 1215, 1216, 1217, 1218, 1219, 1220, 1221, 1222, 1223, 1224, 1225, 1226,
        1227, 1228, 1229, 1230, 1231, 1232, 1233, 1234, 1235, 1236, 1237, 1238, 1239, 1240, 1241,
        1242, 1243, 1244, 1245, 1246, 1247, 1248, 1249, 1250, 1251, 1252, 1253, 1254, 1255, 1256,
        1257, 1258, 1259, 1260, 1261,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 49);
    assert_eq!(records.len(), 49);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1409_fifty_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1262_u32, 1263, 1264, 1265, 1266, 1267, 1268, 1269, 1270, 1271, 1272, 1273, 1274, 1275,
        1276, 1277, 1278, 1279, 1280, 1281, 1282, 1283, 1284, 1285, 1286, 1287, 1288, 1289, 1290,
        1291, 1292, 1293, 1294, 1295, 1296, 1297, 1298, 1299, 1300, 1301, 1302, 1303, 1304, 1305,
        1306, 1307, 1308, 1309, 1310, 1311,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 50);
    assert_eq!(records.len(), 50);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1410_fifty_one_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1312_u32, 1313, 1314, 1315, 1316, 1317, 1318, 1319, 1320, 1321, 1322, 1323, 1324, 1325,
        1326, 1327, 1328, 1329, 1330, 1331, 1332, 1333, 1334, 1335, 1336, 1337, 1338, 1339, 1340,
        1341, 1342, 1343, 1344, 1345, 1346, 1347, 1348, 1349, 1350, 1351, 1352, 1353, 1354, 1355,
        1356, 1357, 1358, 1359, 1360, 1361, 1362,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 51);
    assert_eq!(records.len(), 51);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1419_fifty_two_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1363_u32, 1364, 1365, 1366, 1367, 1368, 1369, 1370, 1371, 1372, 1373, 1374, 1375, 1376,
        1377, 1378, 1379, 1380, 1381, 1382, 1383, 1384, 1385, 1386, 1387, 1388, 1389, 1390, 1391,
        1392, 1393, 1394, 1395, 1396, 1397, 1398, 1399, 1400, 1401, 1402, 1403, 1404, 1405, 1406,
        1407, 1408, 1409, 1410, 1411, 1412, 1413, 1414,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 52);
    assert_eq!(records.len(), 52);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1420_fifty_three_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1415_u32, 1416, 1417, 1418, 1419, 1420, 1421, 1422, 1423, 1424, 1425, 1426, 1427, 1428,
        1429, 1430, 1431, 1432, 1433, 1434, 1435, 1436, 1437, 1438, 1439, 1440, 1441, 1442, 1443,
        1444, 1445, 1446, 1447, 1448, 1449, 1450, 1451, 1452, 1453, 1454, 1455, 1456, 1457, 1458,
        1459, 1460, 1461, 1462, 1463, 1464, 1465, 1466, 1467,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 53);
    assert_eq!(records.len(), 53);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1421_fifty_four_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1468_u32, 1469, 1470, 1471, 1472, 1473, 1474, 1475, 1476, 1477, 1478, 1479, 1480, 1481,
        1482, 1483, 1484, 1485, 1486, 1487, 1488, 1489, 1490, 1491, 1492, 1493, 1494, 1495, 1496,
        1497, 1498, 1499, 1500, 1501, 1502, 1503, 1504, 1505, 1506, 1507, 1508, 1509, 1510, 1511,
        1512, 1513, 1514, 1515, 1516, 1517, 1518, 1519, 1520, 1521,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 54);
    assert_eq!(records.len(), 54);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1422_fifty_five_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1522_u32, 1523, 1524, 1525, 1526, 1527, 1528, 1529, 1530, 1531, 1532, 1533, 1534, 1535,
        1536, 1537, 1538, 1539, 1540, 1541, 1542, 1543, 1544, 1545, 1546, 1547, 1548, 1549, 1550,
        1551, 1552, 1553, 1554, 1555, 1556, 1557, 1558, 1559, 1560, 1561, 1562, 1563, 1564, 1565,
        1566, 1567, 1568, 1569, 1570, 1571, 1572, 1573, 1574, 1575, 1576,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 55);
    assert_eq!(records.len(), 55);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1429_fifty_six_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1577_u32, 1578, 1579, 1580, 1581, 1582, 1583, 1584, 1585, 1586, 1587, 1588, 1589, 1590,
        1591, 1592, 1593, 1594, 1595, 1596, 1597, 1598, 1599, 1600, 1601, 1602, 1603, 1604, 1605,
        1606, 1607, 1608, 1609, 1610, 1611, 1612, 1613, 1614, 1615, 1616, 1617, 1618, 1619, 1620,
        1621, 1622, 1623, 1624, 1625, 1626, 1627, 1628, 1629, 1630, 1631, 1632,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 56);
    assert_eq!(records.len(), 56);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1430_fifty_seven_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1633_u32, 1634, 1635, 1636, 1637, 1638, 1639, 1640, 1641, 1642, 1643, 1644, 1645, 1646,
        1647, 1648, 1649, 1650, 1651, 1652, 1653, 1654, 1655, 1656, 1657, 1658, 1659, 1660, 1661,
        1662, 1663, 1664, 1665, 1666, 1667, 1668, 1669, 1670, 1671, 1672, 1673, 1674, 1675, 1676,
        1677, 1678, 1679, 1680, 1681, 1682, 1683, 1684, 1685, 1686, 1687, 1688, 1689,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 57);
    assert_eq!(records.len(), 57);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1431_fifty_eight_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1690_u32, 1691, 1692, 1693, 1694, 1695, 1696, 1697, 1698, 1699, 1700, 1701, 1702, 1703,
        1704, 1705, 1706, 1707, 1708, 1709, 1710, 1711, 1712, 1713, 1714, 1715, 1716, 1717, 1718,
        1719, 1720, 1721, 1722, 1723, 1724, 1725, 1726, 1727, 1728, 1729, 1730, 1731, 1732, 1733,
        1734, 1735, 1736, 1737, 1738, 1739, 1740, 1741, 1742, 1743, 1744, 1745, 1746, 1747,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 58);
    assert_eq!(records.len(), 58);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1432_fifty_nine_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        1748_u32, 1749, 1750, 1751, 1752, 1753, 1754, 1755, 1756, 1757, 1758, 1759, 1760, 1761,
        1762, 1763, 1764, 1765, 1766, 1767, 1768, 1769, 1770, 1771, 1772, 1773, 1774, 1775, 1776,
        1777, 1778, 1779, 1780, 1781, 1782, 1783, 1784, 1785, 1786, 1787, 1788, 1789, 1790, 1791,
        1792, 1793, 1794, 1795, 1796, 1797, 1798, 1799, 1800, 1801, 1802, 1803, 1804, 1805, 1806,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 59);
    assert_eq!(records.len(), 59);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1327_twenty_seven_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        388_u32, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401, 402, 403, 404,
        405, 406, 407, 408, 409, 410, 411, 412, 413, 414,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 27);
    assert_eq!(records.len(), 27);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1316_twenty_six_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        362_u32, 363, 364, 365, 366, 367, 368, 369, 370, 371, 372, 373, 374, 375, 376, 377, 378,
        379, 380, 381, 382, 383, 384, 385, 386, 387,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 26);
    assert_eq!(records.len(), 26);
    for record in &records {
        assert!(record.replay_verified);
        assert!(record.eligibility.gates.all_passed());
        assert!(!record.eligibility.ranked);
        assert_eq!(
            record.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    let batch_policy = eligibility_policy_for_manifest_jsons(manifests.iter().map(String::as_str));
    assert!(!batch_policy.authority_verified);
}

#[test]
fn rule_catalog_1311_twenty_five_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        337_u32, 338, 339, 340, 341, 342, 343, 344, 345, 346, 347, 348, 349, 350, 351, 352, 353,
        354, 355, 356, 357, 358, 359, 360, 361,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 25);
    assert_eq!(records.len(), 25);
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
fn rule_catalog_1301_twenty_four_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        313_u32, 314, 315, 316, 317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329,
        330, 331, 332, 333, 334, 335, 336,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 24);
    assert_eq!(records.len(), 24);
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
fn rule_catalog_1291_twenty_three_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        290_u32, 291, 292, 293, 294, 295, 296, 297, 298, 299, 300, 301, 302, 303, 304, 305, 306,
        307, 308, 309, 310, 311, 312,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 23);
    assert_eq!(records.len(), 23);
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
fn rule_catalog_1271_twenty_one_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        247_u32, 248, 249, 250, 251, 252, 253, 254, 255, 256, 257, 258, 259, 260, 261, 262, 263,
        264, 265, 266, 267,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 21);
    assert_eq!(records.len(), 21);
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
fn rule_catalog_1232_sixteen_game_synthetic_batch_stays_unranked_unverified_authority() {
    let seeds = [
        157_u32, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172,
    ];
    let manifests: Vec<String> = seeds
        .iter()
        .map(|seed| synthetic_demo_manifest_json(*seed).expect("synthetic manifest"))
        .collect();
    let records: Vec<_> = seeds
        .iter()
        .map(|seed| record_synthetic_demo(*seed).expect("finished synthetic record"))
        .collect();

    assert_eq!(manifests.len(), 16);
    assert_eq!(records.len(), 16);
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
