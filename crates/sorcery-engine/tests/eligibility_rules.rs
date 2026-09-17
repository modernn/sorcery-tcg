//! Direct proofs for TEST-04 ranked-ready eligibility classification.

use serde_json::{Value, json};
use sorcery_engine::batch::{BatchClassification, MAX_GAME_ACTIONS};
use sorcery_engine::canonical::{CanonicalError, IdentityHash, canonical_json, identity_hash};
use sorcery_engine::eligibility::{
    EligibilityReason, TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH,
    eligibility_policy_for_manifest_jsons,
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
