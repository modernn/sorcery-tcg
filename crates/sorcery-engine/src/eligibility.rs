//! TEST-04 eligibility: no result is ranked unless every gate passes.

use serde::Serialize;

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
    /// Stable blocking reasons. Currently always includes partial rules and unverified authority.
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

/// Evaluates TEST-04 gates. Partial rules and unverified authority always block ranking.
#[must_use]
pub fn evaluate_eligibility(gates: EligibilityGates) -> EligibilityReport {
    let mut reasons = vec![
        EligibilityReason::PartialRules,
        EligibilityReason::UnverifiedAuthority,
    ];
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
    EligibilityReport {
        classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
        gates,
        ranked: gates.all_passed() && reasons.is_empty(),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::{EligibilityGates, EligibilityReason, evaluate_eligibility};

    #[test]
    fn passing_gates_still_do_not_rank_partial_unverified_results() {
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
        assert!(!report.gates.all_passed() || !report.reasons.is_empty());
        assert_eq!(
            report.reasons,
            [
                EligibilityReason::PartialRules,
                EligibilityReason::UnverifiedAuthority
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
    }
}
