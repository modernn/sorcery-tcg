//! A caller-controlled hash file cannot verify manifest authority (RULE-CATALOG-0892).
//!
//! Kept in a single-test binary because this regression sets a process environment variable.

use serde_json::json;
use sorcery_engine::eligibility::eligibility_policy_for_manifest;

#[test]
#[allow(unsafe_code)]
fn rule_catalog_0892_runtime_hash_invalid_cannot_verify_private_local_authority() {
    let path = std::env::temp_dir().join(format!(
        "sorcery-eligibility-runtime-hash-0892-{}.txt",
        std::process::id(),
    ));
    std::fs::write(
        &path,
        "\nnot-a-hash\n sha256:not-64-hex \n# comment-like row\n",
    )
    .expect("write untrusted hash file");
    // SAFETY: this integration-test binary contains only this test and starts no threads.
    unsafe {
        std::env::set_var("SORCERY_VERIFIED_AUTHORITY_HASHES_FILE", &path);
    }
    let policy = eligibility_policy_for_manifest(&json!({
        "authority": {
            "contentHash": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
            "mode": "private-local",
        },
    }));
    std::fs::remove_file(&path).expect("remove untrusted hash file");
    assert!(!policy.authority_verified);
}
