//! Direct proof that invalid runtime hash-file rows do not widen ranked
//! eligibility (RULE-CATALOG-0892).
//!
//! Isolated in its own integration-test binary so the runtime allowlist cache
//! initializes from this test's hash file before any other eligibility probe.

use serde_json::json;
use sorcery_engine::eligibility::{
    TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH, eligibility_policy_for_manifest,
    prime_runtime_authority_hash_file_for_tests,
};

const UNKNOWN_HASH: &str =
    "sha256:4444444444444444444444444444444444444444444444444444444444444444";

#[test]
fn rule_catalog_0892_runtime_hash_file_skips_invalid_lines_without_widening_allowlist() {
    let path = std::env::temp_dir().join("sorcery-eligibility-runtime-hash-0892.txt");
    std::fs::write(
        &path,
        "\nnot-a-hash\n sha256:not-64-hex \n# comment-like row\n",
    )
    .expect("write invalid runtime hash file");
    prime_runtime_authority_hash_file_for_tests(path.to_string_lossy().into_owned());
    assert!(
        !eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": UNKNOWN_HASH,
                "mode": "private-local",
            },
        }))
        .authority_verified
    );
    assert!(
        eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH,
                "mode": "private-local",
            },
        }))
        .authority_verified
    );
}
