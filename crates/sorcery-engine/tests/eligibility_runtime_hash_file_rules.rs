//! Direct proof that `SORCERY_VERIFIED_AUTHORITY_HASHES_FILE` extends ranked
//! eligibility (RULE-CATALOG-0891).
//!
//! Isolated in its own integration-test binary so the runtime allowlist cache
//! initializes from this test's hash file before any other eligibility probe.

use serde_json::json;
use sorcery_engine::eligibility::{
    eligibility_policy_for_manifest, prime_runtime_authority_hash_file_for_tests,
};

const RUNTIME_HASH: &str =
    "sha256:3333333333333333333333333333333333333333333333333333333333333333";

#[test]
fn rule_catalog_0891_runtime_hash_file_extends_private_local_allowlist() {
    let path = std::env::temp_dir().join("sorcery-eligibility-runtime-hash-0891.txt");
    std::fs::write(&path, format!("{RUNTIME_HASH}\n\nnot-a-hash\n"))
        .expect("write runtime hash file");
    prime_runtime_authority_hash_file_for_tests(path.to_string_lossy().into_owned());
    let policy = eligibility_policy_for_manifest(&json!({
        "authority": {
            "contentHash": RUNTIME_HASH,
            "mode": "private-local",
        },
    }));
    assert!(policy.authority_verified);
}
