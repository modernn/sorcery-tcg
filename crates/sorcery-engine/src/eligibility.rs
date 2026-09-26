//! TEST-04 eligibility: no result is ranked unless every gate passes.

use serde::Serialize;
use serde_json::Value;

use crate::batch::BatchClassification;

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

/// The corpus audit found missing shared mechanics, including in admitted bindings.
/// Successful scenarios and replay do not establish complete card rules or authority.
pub const CURRENT_ELIGIBILITY_POLICY: EligibilityPolicy = EligibilityPolicy {
    rules_complete: false,
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

/// Returns the conservative policy for untrusted manifest authority claims.
///
/// A caller-supplied mode, content hash, or hash file cannot prove that the
/// manifest's rules and card facts match independently verified authority.
/// Keep results unranked until that verification is bound to the actual inputs.
#[must_use]
pub const fn eligibility_policy_for_manifest(_manifest: &Value) -> EligibilityPolicy {
    CURRENT_ELIGIBILITY_POLICY
}

/// Returns the conservative policy for manifest batches, including empty ones.
#[must_use]
pub fn eligibility_policy_for_manifest_jsons<'a>(
    _manifest_jsons: impl IntoIterator<Item = &'a str>,
) -> EligibilityPolicy {
    CURRENT_ELIGIBILITY_POLICY
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
        EligibilityGates, EligibilityPolicy, EligibilityReason, eligibility_policy_for_manifest,
        eligibility_policy_for_manifest_jsons, evaluate_eligibility,
        evaluate_eligibility_with_policy,
    };

    #[test]
    fn passing_gates_cannot_certify_incomplete_rules_or_unverified_authority() {
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
    fn complete_rules_alone_do_not_verify_authority() {
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
                authority_verified: false,
            },
        );
        assert!(!report.ranked);
        assert_eq!(report.reasons, [EligibilityReason::UnverifiedAuthority]);
        assert_eq!(
            report.classification,
            BatchClassification::UnrankedUnverifiedAuthority
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
    fn private_local_fixture_hash_cannot_verify_authority() {
        let policy = eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
                "mode": "private-local",
            },
        }));
        assert!(!policy.authority_verified);
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
    fn synthetic_manifest_rejects_fixture_hash() {
        let policy = eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
                "mode": "synthetic",
            },
        }));
        assert!(!policy.authority_verified);
    }

    #[test]
    fn private_local_manifest_rejects_arbitrary_hash() {
        let policy = eligibility_policy_for_manifest(&json!({
            "authority": {
                "contentHash": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                "mode": "private-local",
            },
        }));
        assert!(!policy.authority_verified);
    }

    #[test]
    fn empty_or_invalid_manifest_batches_cannot_verify_authority() {
        assert!(!eligibility_policy_for_manifest_jsons([]).authority_verified);
        assert!(!eligibility_policy_for_manifest_jsons(["not JSON"]).authority_verified);
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
            BatchClassification::UnrankedPartialRulesUnverifiedAuthority
        );
    }
}
