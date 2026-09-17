//! Direct proof that `SORCERY_VERIFIED_AUTHORITY_HASHES_FILE` alone extends ranked
//! eligibility when set before the runtime allowlist cache initializes
//! (RULE-CATALOG-0895).
//!
//! Isolated in its own integration-test binary so the runtime allowlist cache
//! initializes from the env var before any other eligibility probe.

use serde_json::json;
use sorcery_engine::eligibility::{
    TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH, eligibility_policy_for_manifest,
};

const ENV_ONLY_HASH: &str =
    "sha256:5555555555555555555555555555555555555555555555555555555555555555";

const _: () = assert!(ENV_ONLY_HASH != TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH);

#[test]
#[allow(unsafe_code)]
fn rule_catalog_0895_runtime_hash_env_var_extends_private_local_allowlist() {
    let path = std::env::temp_dir().join("sorcery-eligibility-runtime-hash-0895.txt");
    std::fs::write(&path, format!("{ENV_ONLY_HASH}\n\nnot-a-hash\n"))
        .expect("write runtime hash file");
    unsafe {
        std::env::set_var(
            "SORCERY_VERIFIED_AUTHORITY_HASHES_FILE",
            path.to_string_lossy().as_ref(),
        );
    }
    let policy = eligibility_policy_for_manifest(&json!({
        "authority": {
            "contentHash": ENV_ONLY_HASH,
            "mode": "private-local",
        },
    }));
    assert!(policy.authority_verified);
}
