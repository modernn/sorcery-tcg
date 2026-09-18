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
