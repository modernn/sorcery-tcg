//! TEST-04 eligibility: no result is ranked unless every gate passes.

use std::sync::OnceLock;

use serde::Serialize;
use serde_json::Value;

use crate::batch::BatchClassification;

/// Compile-time allowlist entry for eligibility scenario proofs only.
pub const TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";

const VERIFIED_PRIVATE_LOCAL_AUTHORITY_HASHES: &[&str] =
    &[TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH];

/// The seven TEST-04 gates that must all pass before a result may be ranked.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EligibilityGates {
    /// Coverage evidence was recorded and no unsupported mechanic was exercised.
    pub coverage: bool,
    /// Competitor policies were bound to the pinned decks and engine.
    pub design: bool,
    /// The game reached a finished public terminal.
    pub execution: bool,
    /// Every committed action was engine-issued.
    pub legality: bool,
    /// Manifest identity, seed, engine version, and authority hash are pinned.
    pub pinned_input: bool,
    /// Authoritative replay reproduced the transcript and hashes.
    pub replay: bool,
    /// A classified report was produced.
    pub reporting: bool,
}

/// Why a result stays unranked or a gate failed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EligibilityReason {
    /// Coverage evidence is missing or an unsupported mechanic was exercised.
    CoverageFailed,
    /// Policies were not bound to the pinned decks and engine.
    DesignFailed,
    /// The game did not finish.
    ExecutionFailed,
    /// A non-engine action was applied.
    LegalityFailed,
    /// Supported rules are still a partial slice.
    PartialRules,
    /// Manifest identity, seed, engine version, or authority hash is missing.
    PinnedInputFailed,
    /// Replay diverged or was not verified.
    ReplayFailed,
    /// The result was not reported.
    ReportingFailed,
    /// Raw manifests are not independently verified official authority.
    UnverifiedAuthority,
}

/// Repository policy for which blocking reasons apply before ranking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EligibilityPolicy {
    /// Whether the supported rules catalog is treated as complete.
    pub rules_complete: bool,
    /// Whether manifests are bound to independently verified official authority.
    pub authority_verified: bool,
}

/// Current master policy: catalog rules are complete; authority remains unverified.
pub const CURRENT_ELIGIBILITY_POLICY: EligibilityPolicy = EligibilityPolicy {
    rules_complete: true,
    authority_verified: false,
};

/// Integer-canonical TEST-04 eligibility verdict.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EligibilityReport {
    /// Ranked/public result classification.
    pub classification: BatchClassification,
    /// Individual TEST-04 gates.
    pub gates: EligibilityGates,
    /// True only when every gate passes and no blocking reason remains.
    pub ranked: bool,
    /// Stable blocking reasons.
    pub reasons: Vec<EligibilityReason>,
}

impl EligibilityGates {
    /// Returns whether every TEST-04 gate passed.
    #[must_use]
    pub const fn all_passed(self) -> bool {
        self.coverage
            && self.design
            && self.execution
            && self.legality
            && self.pinned_input
            && self.replay
            && self.reporting
    }
}

/// Returns TEST-04 policy for one manifest's authority binding.
#[must_use]
pub fn eligibility_policy_for_manifest(manifest: &Value) -> EligibilityPolicy {
    let mut policy = CURRENT_ELIGIBILITY_POLICY;
    policy.authority_verified = manifest_uses_verified_private_local_authority(manifest);
    policy
}

/// Returns TEST-04 policy when every manifest JSON verifies private-local authority.
#[must_use]
pub fn eligibility_policy_for_manifest_jsons<'a>(
    manifest_jsons: impl IntoIterator<Item = &'a str>,
) -> EligibilityPolicy {
    let mut policy = CURRENT_ELIGIBILITY_POLICY;
    policy.authority_verified = manifest_jsons.into_iter().all(|manifest_json| {
        serde_json::from_str::<Value>(manifest_json)
            .is_ok_and(|manifest| eligibility_policy_for_manifest(&manifest).authority_verified)
    });
    policy
}

/// Evaluates TEST-04 gates under the current repository policy.
#[must_use]
pub fn evaluate_eligibility(gates: EligibilityGates) -> EligibilityReport {
    evaluate_eligibility_with_policy(gates, CURRENT_ELIGIBILITY_POLICY)
}

/// Evaluates TEST-04 gates under an explicit policy.
#[must_use]
pub fn evaluate_eligibility_with_policy(
    gates: EligibilityGates,
    policy: EligibilityPolicy,
) -> EligibilityReport {
    let mut reasons = Vec::new();
    if !policy.rules_complete {
        reasons.push(EligibilityReason::PartialRules);
    }
    if !policy.authority_verified {
        reasons.push(EligibilityReason::UnverifiedAuthority);
    }
    if !gates.coverage {
        reasons.push(EligibilityReason::CoverageFailed);
    }
    if !gates.design {
        reasons.push(EligibilityReason::DesignFailed);
    }
    if !gates.execution {
        reasons.push(EligibilityReason::ExecutionFailed);
    }
    if !gates.legality {
        reasons.push(EligibilityReason::LegalityFailed);
    }
    if !gates.pinned_input {
        reasons.push(EligibilityReason::PinnedInputFailed);
    }
    if !gates.replay {
        reasons.push(EligibilityReason::ReplayFailed);
    }
    if !gates.reporting {
        reasons.push(EligibilityReason::ReportingFailed);
    }
    reasons.sort();
    let ranked = gates.all_passed() && reasons.is_empty();
    EligibilityReport {
        classification: classification_for_policy(policy, ranked),
        gates,
        ranked,
        reasons,
    }
}

fn manifest_uses_verified_private_local_authority(manifest: &Value) -> bool {
    let Some(authority) = manifest.get("authority").and_then(Value::as_object) else {
        return false;
    };
    if authority.get("mode").and_then(Value::as_str) != Some("private-local") {
        return false;
    }
    authority
        .get("contentHash")
        .and_then(Value::as_str)
        .is_some_and(verified_private_local_authority_hash_is_allowlisted)
}

fn verified_private_local_authority_hash_is_allowlisted(content_hash: &str) -> bool {
    VERIFIED_PRIVATE_LOCAL_AUTHORITY_HASHES.contains(&content_hash)
        || runtime_verified_private_local_authority_hashes()
            .iter()
            .any(|hash| hash == content_hash)
}

fn runtime_verified_private_local_authority_hashes() -> &'static [String] {
    static CACHE: OnceLock<Vec<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        std::env::var("SORCERY_VERIFIED_AUTHORITY_HASHES_FILE")
            .ok()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|content| parse_verified_authority_hash_lines(&content))
            .unwrap_or_default()
    })
}

fn parse_verified_authority_hash_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && line.starts_with("sha256:"))
        .map(str::to_owned)
        .collect()
}

const fn classification_for_policy(policy: EligibilityPolicy, ranked: bool) -> BatchClassification {
    if ranked {
        return BatchClassification::Ranked;
    }
    if policy.rules_complete {
        BatchClassification::UnrankedUnverifiedAuthority
    } else {
        BatchClassification::UnrankedPartialRulesUnverifiedAuthority
    }
}

#[cfg(test)]
mod tests {
    use crate::batch::BatchClassification;

    use serde_json::json;

    use super::{
        EligibilityGates, EligibilityPolicy, EligibilityReason,
        TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH, eligibility_policy_for_manifest,
        evaluate_eligibility, evaluate_eligibility_with_policy,
    };

    #[test]
    fn passing_gates_still_do_not_rank_unverified_authority() {
        let report = evaluate_eligibility(EligibilityGates {
            coverage: true,
            design: true,
            execution: true,
            legality: true,
            pinned_input: true,
            replay: true,
            reporting: true,
        });
        assert!(!report.ranked);
        assert!(report.gates.all_passed());
        assert_eq!(report.reasons, [EligibilityReason::UnverifiedAuthority]);
        assert_eq!(
            report.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }

    #[test]
    fn partial_rules_policy_keeps_legacy_classification() {
        let report = evaluate_eligibility_with_policy(
            EligibilityGates {
                coverage: true,
                design: true,
                execution: true,
                legality: true,
                pinned_input: true,
                replay: true,
                reporting: true,
            },
            EligibilityPolicy {
                rules_complete: false,
                authority_verified: false,
            },
        );
        assert!(!report.ranked);
        assert_eq!(
            report.reasons,
            [
                EligibilityReason::PartialRules,
                EligibilityReason::UnverifiedAuthority
            ]
        );
        assert_eq!(
            report.classification,
            BatchClassification::UnrankedPartialRulesUnverifiedAuthority
        );
    }

    #[test]
    fn verified_authority_and_passing_gates_rank() {
        let report = evaluate_eligibility_with_policy(
            EligibilityGates {
                coverage: true,
                design: true,
                execution: true,
                legality: true,
                pinned_input: true,
                replay: true,
                reporting: true,
            },
            EligibilityPolicy {
                rules_complete: true,
                authority_verified: true,
            },
        );
        assert!(report.ranked);
        assert!(report.reasons.is_empty());
        assert_eq!(report.classification, BatchClassification::Ranked);
    }

    #[test]
    fn private_local_allowlisted_hash_enables_verified_authority_policy() {
        let policy = eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH,
                "mode": "private-local",
            },
        }));
        assert!(policy.authority_verified);
    }

    #[test]
    fn synthetic_manifest_keeps_unverified_authority_policy() {
        let policy = eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                "mode": "synthetic",
            },
        }));
        assert!(!policy.authority_verified);
    }

    #[test]
    fn synthetic_manifest_rejects_allowlisted_hash_without_private_local_mode() {
        let policy = eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": TEST_ELIGIBILITY_SCENARIO_AUTHORITY_HASH,
                "mode": "synthetic",
            },
        }));
        assert!(!policy.authority_verified);
    }

    #[test]
    fn private_local_manifest_rejects_non_allowlisted_hash() {
        let policy = eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                "mode": "private-local",
            },
        }));
        assert!(!policy.authority_verified);
    }

    #[test]
    fn parse_verified_authority_hash_lines_skips_blank_and_non_sha256_rows() {
        assert_eq!(
            super::parse_verified_authority_hash_lines(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\nnot-a-hash\n sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb \n"
            ),
            vec![
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_owned(),
            ]
        );
    }

    #[test]
    fn failed_replay_is_an_explicit_blocking_reason() {
        let report = evaluate_eligibility(EligibilityGates {
            coverage: true,
            design: true,
            execution: true,
            legality: true,
            pinned_input: true,
            replay: false,
            reporting: true,
        });
        assert!(!report.ranked);
        assert!(!report.gates.replay);
        assert!(report.reasons.contains(&EligibilityReason::ReplayFailed));
        assert_eq!(
            report.classification,
            BatchClassification::UnrankedUnverifiedAuthority
        );
    }
}
