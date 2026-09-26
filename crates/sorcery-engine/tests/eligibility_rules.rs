//! Direct proofs for TEST-04 eligibility and unverified authority rejection.

use serde_json::{Value, json};
use sorcery_engine::batch::{BatchClassification, MAX_GAME_ACTIONS};
use sorcery_engine::canonical::{CanonicalError, IdentityHash, canonical_json, identity_hash};
use sorcery_engine::eligibility::{
    EligibilityReason, eligibility_policy_for_manifest, eligibility_policy_for_manifest_jsons,
    evaluate_eligibility_with_policy,
};
use sorcery_engine::game_record::record_policy_game;
use sorcery_engine::game_record::record_synthetic_demo;
use sorcery_engine::policy::{BASELINE_POLICY_DECK_ID, baseline_policy_snapshot};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

const TEST_AUTHORITY_HASH: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";

fn claimed_private_local_manifest(seed: u32) -> Result<String, CanonicalError> {
    let mut manifest: Value =
        serde_json::from_str(&synthetic_demo_manifest_json(seed)?).expect("manifest JSON");
    let object = manifest.as_object_mut().expect("manifest object");
    object.remove("manifestId");
    object["authority"] = json!({
        "contentHash": TEST_AUTHORITY_HASH,
        "mode": "private-local",
        "revisionId": "eligibility-scenario-fixture-v1",
    });
    manifest["manifestId"] = json!(identity_hash(&manifest)?);
    canonical_json(&manifest)
}

fn other_claimed_private_local_manifest(seed: u32) -> Result<String, CanonicalError> {
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
fn rule_catalog_0881_relabelled_synthetic_manifest_stays_unranked() {
    let manifest_json = claimed_private_local_manifest(31).expect("claimed manifest");
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
    .expect("seed-31 finished unverified record");

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
fn rule_catalog_0882_private_local_and_mixed_batches_stay_unverified() {
    let claimed = claimed_private_local_manifest(31).expect("claimed manifest");
    let synthetic = synthetic_demo_manifest_json(31).expect("synthetic manifest");

    let claimed_only = eligibility_policy_for_manifest_jsons([claimed.as_str()]);
    assert!(!claimed_only.authority_verified);

    let mixed = eligibility_policy_for_manifest_jsons([claimed.as_str(), synthetic.as_str()]);
    assert!(!mixed.authority_verified);

    let synthetic_only = eligibility_policy_for_manifest_jsons([synthetic.as_str()]);
    assert!(!synthetic_only.authority_verified);
}

#[test]
fn rule_catalog_0883_synthetic_mode_rejects_fixture_authority_hash() {
    let policy = eligibility_policy_for_manifest(&json!({
        "authority": {
            "contentHash": TEST_AUTHORITY_HASH,
            "mode": "synthetic",
        },
    }));
    assert!(!policy.authority_verified);
}

#[test]
fn rule_catalog_0884_private_local_arbitrary_hash_stays_unverified() {
    let policy = eligibility_policy_for_manifest(&json!({
        "authority": {
            "contentHash": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            "mode": "private-local",
        },
    }));
    assert!(!policy.authority_verified);
}

#[test]
fn rule_catalog_0942_mixed_claimed_and_synthetic_batch_stays_unranked() {
    let claimed_manifest = claimed_private_local_manifest(31).expect("claimed manifest");
    let synthetic = synthetic_demo_manifest_json(31).expect("synthetic manifest");
    let game = sorcery_engine::game::Game::from_manifest_json(&claimed_manifest).expect("game");
    let policy =
        baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
            .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let record = record_policy_game(
        &claimed_manifest,
        &deck_id,
        &policy,
        &deck_id,
        &policy,
        MAX_GAME_ACTIONS,
    )
    .expect("seed-31 finished unverified record");

    assert!(record.replay_verified);
    assert!(record.eligibility.gates.all_passed());
    assert!(!record.eligibility.ranked);
    assert_eq!(
        record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let batch_policy =
        eligibility_policy_for_manifest_jsons([claimed_manifest.as_str(), synthetic.as_str()]);
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
fn rule_catalog_0950_private_local_claims_only_batch_stays_unranked() {
    let claimed_manifest = claimed_private_local_manifest(31).expect("claimed manifest");
    let other_claimed = claimed_private_local_manifest(32).expect("other claimed manifest");
    let game = sorcery_engine::game::Game::from_manifest_json(&claimed_manifest).expect("game");
    let policy =
        baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
            .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let record = record_policy_game(
        &claimed_manifest,
        &deck_id,
        &policy,
        &deck_id,
        &policy,
        MAX_GAME_ACTIONS,
    )
    .expect("seed-31 finished unverified record");

    assert!(record.replay_verified);
    assert!(record.eligibility.gates.all_passed());
    assert!(!record.eligibility.ranked);
    assert_eq!(
        record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let batch_policy =
        eligibility_policy_for_manifest_jsons([claimed_manifest.as_str(), other_claimed.as_str()]);
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
fn rule_catalog_0982_different_private_local_claims_stay_unranked_for_both() {
    let claimed_manifest = claimed_private_local_manifest(31).expect("claimed manifest");
    let other_claimed_manifest =
        other_claimed_private_local_manifest(32).expect("other claimed manifest");

    let claimed_game =
        sorcery_engine::game::Game::from_manifest_json(&claimed_manifest).expect("game");
    let claimed_policy = baseline_policy_snapshot(
        claimed_game.rules().authority_hash(),
        claimed_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let claimed_record = record_policy_game(
        &claimed_manifest,
        &deck_id,
        &claimed_policy,
        &deck_id,
        &claimed_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("claimed finished record");

    assert!(claimed_record.replay_verified);
    assert!(claimed_record.eligibility.gates.all_passed());
    assert!(!claimed_record.eligibility.ranked);
    assert_eq!(
        claimed_record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let other_claimed_game =
        sorcery_engine::game::Game::from_manifest_json(&other_claimed_manifest).expect("game");
    let other_claimed_policy = baseline_policy_snapshot(
        other_claimed_game.rules().authority_hash(),
        other_claimed_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let other_claimed_record = record_policy_game(
        &other_claimed_manifest,
        &deck_id,
        &other_claimed_policy,
        &deck_id,
        &other_claimed_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("other claimed finished record");

    assert!(other_claimed_record.replay_verified);
    assert!(other_claimed_record.eligibility.gates.all_passed());
    assert!(!other_claimed_record.eligibility.ranked);
    assert_eq!(
        other_claimed_record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let batch_policy = eligibility_policy_for_manifest_jsons([
        claimed_manifest.as_str(),
        other_claimed_manifest.as_str(),
    ]);
    assert!(!batch_policy.authority_verified);

    let claimed_batch =
        evaluate_eligibility_with_policy(claimed_record.eligibility.gates, batch_policy);
    assert!(!claimed_batch.ranked);
    assert_eq!(
        claimed_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        claimed_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );

    let other_claimed_batch =
        evaluate_eligibility_with_policy(other_claimed_record.eligibility.gates, batch_policy);
    assert!(!other_claimed_batch.ranked);
    assert_eq!(
        other_claimed_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        other_claimed_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );
}

#[test]
fn rule_catalog_1001_mixed_private_local_claim_and_synthetic_batch_stays_unranked_for_both() {
    let claimed_manifest = claimed_private_local_manifest(31).expect("claimed manifest");
    let synthetic_manifest = synthetic_demo_manifest_json(32).expect("synthetic manifest");

    let claimed_game =
        sorcery_engine::game::Game::from_manifest_json(&claimed_manifest).expect("game");
    let claimed_policy = baseline_policy_snapshot(
        claimed_game.rules().authority_hash(),
        claimed_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let claimed_record = record_policy_game(
        &claimed_manifest,
        &deck_id,
        &claimed_policy,
        &deck_id,
        &claimed_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("claimed finished record");

    assert!(claimed_record.replay_verified);
    assert!(claimed_record.eligibility.gates.all_passed());
    assert!(!claimed_record.eligibility.ranked);
    assert_eq!(
        claimed_record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
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
        claimed_manifest.as_str(),
        synthetic_manifest.as_str(),
    ]);
    assert!(!batch_policy.authority_verified);

    let claimed_batch =
        evaluate_eligibility_with_policy(claimed_record.eligibility.gates, batch_policy);
    assert!(!claimed_batch.ranked);
    assert_eq!(
        claimed_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        claimed_batch.reasons,
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
fn rule_catalog_1012_three_way_private_claims_and_synthetic_batch_stays_unranked() {
    let claimed_manifest = claimed_private_local_manifest(31).expect("claimed manifest");
    let other_claimed_manifest =
        other_claimed_private_local_manifest(32).expect("other claimed manifest");
    let synthetic_manifest = synthetic_demo_manifest_json(33).expect("synthetic manifest");

    let claimed_game =
        sorcery_engine::game::Game::from_manifest_json(&claimed_manifest).expect("game");
    let claimed_policy = baseline_policy_snapshot(
        claimed_game.rules().authority_hash(),
        claimed_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID).expect("baseline deck id");
    let claimed_record = record_policy_game(
        &claimed_manifest,
        &deck_id,
        &claimed_policy,
        &deck_id,
        &claimed_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("claimed finished record");

    assert!(claimed_record.replay_verified);
    assert!(claimed_record.eligibility.gates.all_passed());
    assert!(!claimed_record.eligibility.ranked);
    assert_eq!(
        claimed_record.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );

    let other_claimed_game =
        sorcery_engine::game::Game::from_manifest_json(&other_claimed_manifest).expect("game");
    let other_claimed_policy = baseline_policy_snapshot(
        other_claimed_game.rules().authority_hash(),
        other_claimed_game.rules().engine_version(),
    )
    .expect("baseline policy");
    let other_claimed_record = record_policy_game(
        &other_claimed_manifest,
        &deck_id,
        &other_claimed_policy,
        &deck_id,
        &other_claimed_policy,
        MAX_GAME_ACTIONS,
    )
    .expect("other claimed finished record");

    assert!(other_claimed_record.replay_verified);
    assert!(other_claimed_record.eligibility.gates.all_passed());
    assert!(!other_claimed_record.eligibility.ranked);
    assert_eq!(
        other_claimed_record.classification,
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
        claimed_manifest.as_str(),
        other_claimed_manifest.as_str(),
        synthetic_manifest.as_str(),
    ]);
    assert!(!batch_policy.authority_verified);

    let claimed_batch =
        evaluate_eligibility_with_policy(claimed_record.eligibility.gates, batch_policy);
    assert!(!claimed_batch.ranked);
    assert_eq!(
        claimed_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        claimed_batch.reasons,
        [EligibilityReason::UnverifiedAuthority]
    );

    let other_claimed_batch =
        evaluate_eligibility_with_policy(other_claimed_record.eligibility.gates, batch_policy);
    assert!(!other_claimed_batch.ranked);
    assert_eq!(
        other_claimed_batch.classification,
        BatchClassification::UnrankedUnverifiedAuthority
    );
    assert_eq!(
        other_claimed_batch.reasons,
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
