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
