//! Deterministic neighborhood training and replay-gated policy promotion.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::batch::BatchClassification;
use crate::canonical::{
    CanonicalError, IdentityHash, canonical_json, identity_hash, parse_json_without_duplicate_keys,
};
use crate::contract::Seat;
use crate::deck::{CanonicalDeck, DeckValidation};
use crate::game::{Game, GameOutcome};
use crate::policy::{
    PolicyError, PolicySnapshot, parse_policy_snapshot, serialize_policy_snapshot,
};
use crate::simulator::{SimulatorError, replay_selected, run_game};

/// Largest accepted self-play suite. This keeps exact paired statistics in `u128`.
pub const MAX_SELF_PLAY_PAIRS: usize = 128;
/// Smallest heldout suite allowed to promote a policy.
pub const MIN_PROMOTION_PAIRS: usize = 20;
/// Largest precommitted number of promotion attempts in one campaign.
pub const MAX_CAMPAIGN_PROMOTION_ATTEMPTS: u8 = 10;
/// Maximum accepted canonical campaign checkpoint length.
pub const SELF_PLAY_CHECKPOINT_MAX_BYTES: usize = 1024 * 1024;

const CAMPAIGN_SIGNIFICANCE_DENOMINATOR: u128 = 100 * (MAX_CAMPAIGN_PROMOTION_ATTEMPTS as u128 + 1);

/// One complete seat-swapped policy evaluation pair.
#[derive(Clone, Copy, Debug)]
pub struct SelfPlayPair<'a> {
    /// Seed declared by both manifests.
    pub seed: u32,
    /// Stable opponent or matchup grouping used for regression gates.
    pub subgroup: &'a str,
    /// Canonical manifest with the candidate deck in the North seat.
    pub candidate_as_north_manifest_json: &'a str,
    /// Canonical manifest with the candidate deck in the South seat.
    pub candidate_as_south_manifest_json: &'a str,
    /// Fixed opposing policy.
    pub opponent: &'a PolicySnapshot,
    /// Validated deck bound to the opposing policy.
    pub opponent_deck: &'a DeckValidation,
}

#[derive(Debug)]
struct PreparedPair<'a> {
    seed: u32,
    subgroup: &'a str,
    candidate_as_north_manifest_json: &'a str,
    candidate_as_south_manifest_json: &'a str,
    candidate_as_north: Game,
    candidate_as_south: Game,
    opponent: &'a PolicySnapshot,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortfolioCell {
    subgroup: String,
    opponent_policy_id: IdentityHash,
    opponent_deck_id: IdentityHash,
    scenario_id: IdentityHash,
}

type Portfolio = BTreeMap<PortfolioCell, usize>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortfolioEntry {
    cell: PortfolioCell,
    count: usize,
}

/// Integer policy score; a win is two half-points and a draw is one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelfPlayScore {
    games: u64,
    half_points: u64,
    subgroup_half_points: BTreeMap<String, u64>,
    pairs: Vec<SelfPlayPairScore>,
}

/// One paired evaluation observation spanning both physical seats and one unique suite seed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelfPlayPairScore {
    seed: u32,
    subgroup: String,
    opponent_policy_id: IdentityHash,
    north_manifest_id: IdentityHash,
    south_manifest_id: IdentityHash,
    candidate_as_north_half_points: u8,
    candidate_as_south_half_points: u8,
}

impl SelfPlayScore {
    /// Returns the number of completed, counted games.
    #[must_use]
    pub const fn games(&self) -> u64 {
        self.games
    }

    /// Returns total integer half-points.
    #[must_use]
    pub const fn half_points(&self) -> u64 {
        self.half_points
    }

    /// Returns deterministic subgroup totals.
    #[must_use]
    pub const fn subgroup_half_points(&self) -> &BTreeMap<String, u64> {
        &self.subgroup_half_points
    }

    /// Returns ordered independent seat-pair observations.
    #[must_use]
    pub fn pairs(&self) -> &[SelfPlayPairScore] {
        &self.pairs
    }
}

impl SelfPlayPairScore {
    /// Returns the unique suite seed.
    #[must_use]
    pub const fn seed(&self) -> u32 {
        self.seed
    }

    /// Returns the predeclared matchup subgroup.
    #[must_use]
    pub fn subgroup(&self) -> &str {
        &self.subgroup
    }

    /// Returns the bound opposing policy identity.
    #[must_use]
    pub const fn opponent_policy_id(&self) -> &IdentityHash {
        &self.opponent_policy_id
    }

    /// Returns manifest identities in candidate-North, candidate-South order.
    #[must_use]
    pub const fn manifest_ids(&self) -> [&IdentityHash; 2] {
        [&self.north_manifest_id, &self.south_manifest_id]
    }

    /// Returns candidate half-points in North, South order.
    #[must_use]
    pub const fn seat_half_points(&self) -> [u8; 2] {
        [
            self.candidate_as_north_half_points,
            self.candidate_as_south_half_points,
        ]
    }

    /// Returns the two-seat score in integer half-points.
    #[must_use]
    pub const fn half_points(&self) -> u8 {
        self.candidate_as_north_half_points + self.candidate_as_south_half_points
    }

    fn same_case(&self, other: &Self) -> bool {
        self.seed == other.seed
            && self.subgroup == other.subgroup
            && self.opponent_policy_id == other.opponent_policy_id
            && self.north_manifest_id == other.north_manifest_id
            && self.south_manifest_id == other.south_manifest_id
    }
}

/// Result of one train/heldout promotion cycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionResult {
    /// Public result classification for the raw-manifest evaluation boundary.
    pub classification: BatchClassification,
    /// Champion retained or immutable child selected for the next generation.
    pub policy: PolicySnapshot,
    /// Whether `policy` is the selected child.
    pub promoted: bool,
    /// Canonical identity of the training-selected child.
    pub nominee_policy_id: IdentityHash,
    /// Training score used to nominate the child.
    pub nominee_training: SelfPlayScore,
    /// Champion score on the heldout suite.
    pub champion_heldout: SelfPlayScore,
    /// Nominee score on the heldout suite.
    pub nominee_heldout: SelfPlayScore,
}

/// Replay-verified final score for a sealed self-play campaign.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelfPlayAudit {
    /// Public result classification for the raw-manifest evaluation boundary.
    pub classification: BatchClassification,
    /// Policy evaluated by the fresh final audit.
    pub policy_id: IdentityHash,
    /// Root policy score on the same fresh audit suite.
    pub baseline_score: SelfPlayScore,
    /// Final replay-verified score.
    pub score: SelfPlayScore,
}

/// Coordinator that prevents evaluation-seed reuse across one campaign.
///
/// Use the campaign checkpoint functions to preserve completed-call state across process restarts.
#[derive(Debug)]
pub struct SelfPlayCampaign {
    assigned_deck: DeckValidation,
    final_audit_state: FinalAuditState,
    initial_policy: PolicySnapshot,
    max_actions: usize,
    pending_operation: Option<PendingOperation>,
    promotion_portfolio: Option<Portfolio>,
    promotion_attempts: u8,
    promoted_policies: Vec<PolicySnapshot>,
    used_development_seeds: BTreeSet<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum FinalAuditState {
    Open,
    Failed,
    Complete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum PendingOperation {
    Generation { suite_id: IdentityHash },
    FinalAudit { suite_id: IdentityHash },
}

/// Canonical, self-hashed private campaign control state.
///
/// The validated assigned deck is deliberately external and must be supplied again on resume.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfPlayCampaignCheckpoint {
    assigned_deck_id: IdentityHash,
    checkpoint_id: IdentityHash,
    final_audit_state: FinalAuditState,
    initial_policy: PolicySnapshot,
    kind: String,
    max_actions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_operation: Option<PendingOperation>,
    promotion_attempts: u8,
    promotion_portfolio: Option<Vec<PortfolioEntry>>,
    promoted_policies: Vec<PolicySnapshot>,
    schema_version: u8,
    used_development_seeds: Vec<u32>,
}

impl SelfPlayCampaignCheckpoint {
    /// Returns the self-hash over the checkpoint body.
    #[must_use]
    pub const fn checkpoint_id(&self) -> &IdentityHash {
        &self.checkpoint_id
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawSelfPlayCampaignCheckpoint {
    assigned_deck_id: IdentityHash,
    checkpoint_id: IdentityHash,
    final_audit_state: FinalAuditState,
    initial_policy: serde_json::Value,
    kind: String,
    max_actions: usize,
    #[serde(default)]
    pending_operation: Option<PendingOperation>,
    promotion_attempts: u8,
    promotion_portfolio: Option<Vec<PortfolioEntry>>,
    promoted_policies: Vec<serde_json::Value>,
    schema_version: u8,
    used_development_seeds: Vec<u32>,
}

impl FinalAuditState {
    fn begin(&mut self) -> Result<(), SelfPlayError> {
        if *self != Self::Open {
            return Err(SelfPlayError::Invalid(
                "self-play campaign cannot repeat its final audit",
            ));
        }
        *self = Self::Failed;
        Ok(())
    }
}

/// Self-play configuration, rollout, or replay verification failed.
#[derive(Debug)]
pub enum SelfPlayError {
    /// Canonical serialization or hashing failed.
    Canonical(CanonicalError),
    /// A policy snapshot was invalid.
    Policy(PolicyError),
    /// A rollout or authoritative replay failed.
    Simulator(SimulatorError),
    /// Manifest JSON could not be decoded.
    Json(serde_json::Error),
    /// A policy failed to finish one bounded game.
    NonTerminal,
    /// A self-play invariant was violated.
    Invalid(&'static str),
}

impl fmt::Display for SelfPlayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(error) => error.fmt(formatter),
            Self::Policy(error) => error.fmt(formatter),
            Self::Simulator(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::NonTerminal => {
                formatter.write_str("self-play game did not terminate within the action bound")
            }
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl Error for SelfPlayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            Self::Policy(error) => Some(error),
            Self::Simulator(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::NonTerminal | Self::Invalid(_) => None,
        }
    }
}

impl From<CanonicalError> for SelfPlayError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<PolicyError> for SelfPlayError {
    fn from(error: PolicyError) -> Self {
        Self::Policy(error)
    }
}

impl From<SimulatorError> for SelfPlayError {
    fn from(error: SimulatorError) -> Self {
        Self::Simulator(error)
    }
}

impl From<serde_json::Error> for SelfPlayError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl SelfPlayCampaign {
    /// Starts a campaign for one immutable policy and validated deck.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] when the policy is not bound to the assigned deck.
    pub fn new(
        initial_policy: PolicySnapshot,
        assigned_deck: DeckValidation,
        max_actions: usize,
    ) -> Result<Self, SelfPlayError> {
        if max_actions == 0 {
            return Err(SelfPlayError::Invalid(
                "self-play action bound must be greater than zero",
            ));
        }
        if initial_policy.generation() != 0 || initial_policy.parent_policy_id().is_some() {
            return Err(SelfPlayError::Invalid(
                "self-play campaigns must start from a generation-zero root policy",
            ));
        }
        validate_assigned_policy(&initial_policy, &assigned_deck)?;
        Ok(Self {
            assigned_deck,
            final_audit_state: FinalAuditState::Open,
            initial_policy,
            max_actions,
            pending_operation: None,
            promotion_portfolio: None,
            promotion_attempts: 0,
            promoted_policies: Vec::new(),
            used_development_seeds: BTreeSet::new(),
        })
    }

    /// Returns the current immutable champion.
    #[must_use]
    pub fn champion(&self) -> &PolicySnapshot {
        self.promoted_policies
            .last()
            .unwrap_or(&self.initial_policy)
    }

    /// Returns the root and every actually promoted child in order.
    #[must_use]
    pub fn lineage(&self) -> impl DoubleEndedIterator<Item = &PolicySnapshot> {
        std::iter::once(&self.initial_policy).chain(self.promoted_policies.iter())
    }

    /// Returns whether a successful fresh final audit sealed the campaign.
    #[must_use]
    pub const fn is_finalized(&self) -> bool {
        matches!(self.final_audit_state, FinalAuditState::Complete)
    }

    /// Returns the immutable per-game action bound for this campaign.
    #[must_use]
    pub const fn max_actions(&self) -> usize {
        self.max_actions
    }

    /// Returns the number of completed promotion comparisons.
    #[must_use]
    pub const fn promotion_attempts(&self) -> u8 {
        self.promotion_attempts
    }

    /// Reserves one exact training and promotion suite before evaluation.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] for invalid input, seed reuse, or another pending operation.
    pub fn reserve_generation(
        &mut self,
        training: &[SelfPlayPair<'_>],
        promotion: &[SelfPlayPair<'_>],
    ) -> Result<(), SelfPlayError> {
        if self.pending_operation.is_some() {
            return Err(SelfPlayError::Invalid(
                "self-play campaign already has a pending operation",
            ));
        }
        if self.final_audit_state != FinalAuditState::Open {
            return Err(SelfPlayError::Invalid(
                "self-play campaign cannot continue after its final audit attempt",
            ));
        }
        if training.is_empty() || training.len() > MAX_SELF_PLAY_PAIRS {
            return Err(SelfPlayError::Invalid(
                "self-play training suites must contain 1-128 seat pairs",
            ));
        }
        if promotion.len() < MIN_PROMOTION_PAIRS || promotion.len() > MAX_SELF_PLAY_PAIRS {
            return Err(SelfPlayError::Invalid(
                "self-play promotion suites must contain 20-128 unique-seed seat pairs",
            ));
        }
        if self.promotion_attempts == MAX_CAMPAIGN_PROMOTION_ATTEMPTS {
            return Err(SelfPlayError::Invalid(
                "self-play campaign exhausted its promotion-attempt budget",
            ));
        }
        let generation_seeds = raw_seeds(training)
            .union(&raw_seeds(promotion))
            .copied()
            .collect::<BTreeSet<_>>();
        if generation_seeds
            .iter()
            .any(|seed| self.used_development_seeds.contains(seed))
        {
            return Err(SelfPlayError::Invalid(
                "self-play campaign cannot reuse a development seed",
            ));
        }
        let promotion_portfolio = portfolio(promotion)?;
        if self
            .promotion_portfolio
            .as_ref()
            .is_some_and(|expected| expected != &promotion_portfolio)
        {
            return Err(SelfPlayError::Invalid(
                "self-play promotion portfolio changed across generations",
            ));
        }
        let prepared_training = prepare_suite(training, &self.assigned_deck, self.champion())?;
        let prepared_promotion = prepare_suite(promotion, &self.assigned_deck, self.champion())?;
        let training_seeds = seeds(&prepared_training);
        if prepared_promotion
            .iter()
            .any(|case| training_seeds.contains(&case.seed))
        {
            return Err(SelfPlayError::Invalid(
                "training and heldout seeds must be disjoint",
            ));
        }
        self.pending_operation = Some(PendingOperation::Generation {
            suite_id: generation_suite_id(training, promotion)?,
        });
        Ok(())
    }

    /// Completes the exact generation previously reserved by [`Self::reserve_generation`].
    ///
    /// Failures retain the reservation so the same deterministic suite can resume after restart.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] when the suite differs or evaluation fails.
    pub fn complete_generation(
        &mut self,
        training: &[SelfPlayPair<'_>],
        promotion: &[SelfPlayPair<'_>],
    ) -> Result<PromotionResult, SelfPlayError> {
        let suite_id = generation_suite_id(training, promotion)?;
        if !matches!(
            &self.pending_operation,
            Some(PendingOperation::Generation { suite_id: expected }) if expected == &suite_id
        ) {
            return Err(SelfPlayError::Invalid(
                "self-play generation does not match the pending suite",
            ));
        }
        let generation_seeds = raw_seeds(training)
            .union(&raw_seeds(promotion))
            .copied()
            .collect::<BTreeSet<_>>();
        let promotion_portfolio = portfolio(promotion)?;
        let result = train_and_promote_at_significance(
            self.champion(),
            &self.assigned_deck,
            training,
            promotion,
            self.max_actions,
            CAMPAIGN_SIGNIFICANCE_DENOMINATOR,
        )?;
        if self.promotion_portfolio.is_none() {
            self.promotion_portfolio = Some(promotion_portfolio);
        }
        self.used_development_seeds.extend(generation_seeds);
        self.promotion_attempts += 1;
        if result.promoted {
            self.promoted_policies.push(result.policy.clone());
        }
        self.pending_operation = None;
        Ok(result)
    }

    /// Runs one generation, reserving its exact suites before evaluation.
    ///
    /// Use [`Self::reserve_generation`], persist a checkpoint, then call
    /// [`Self::complete_generation`] when crash-discard protection is required.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] for reservation or evaluation failure.
    pub fn run_generation(
        &mut self,
        training: &[SelfPlayPair<'_>],
        promotion: &[SelfPlayPair<'_>],
    ) -> Result<PromotionResult, SelfPlayError> {
        if self.pending_operation.is_none() {
            self.reserve_generation(training, promotion)?;
        }
        self.complete_generation(training, promotion)
    }

    /// Reserves one exact final-audit suite before replay evaluation.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] for prior seed use, invalid coverage, or another operation.
    pub fn reserve_final_audit(&mut self, audit: &[SelfPlayPair<'_>]) -> Result<(), SelfPlayError> {
        if self.pending_operation.is_some() {
            return Err(SelfPlayError::Invalid(
                "self-play campaign already has a pending operation",
            ));
        }
        if self.final_audit_state != FinalAuditState::Open {
            return Err(SelfPlayError::Invalid(
                "self-play campaign cannot repeat its final audit",
            ));
        }
        if audit.is_empty() || audit.len() > MAX_SELF_PLAY_PAIRS {
            return Err(SelfPlayError::Invalid(
                "self-play audit suites must contain 1-128 seat pairs",
            ));
        }
        if raw_seeds(audit)
            .iter()
            .any(|seed| self.used_development_seeds.contains(seed))
        {
            return Err(SelfPlayError::Invalid(
                "final audit seeds must be fresh from campaign development",
            ));
        }
        let expected_portfolio =
            self.promotion_portfolio
                .as_ref()
                .ok_or(SelfPlayError::Invalid(
                    "final audit requires a completed self-play generation",
                ))?;
        let audit_portfolio = portfolio(audit)?;
        if !portfolio_covers(&audit_portfolio, expected_portfolio) {
            return Err(SelfPlayError::Invalid(
                "final audit must cover the locked promotion portfolio",
            ));
        }
        prepare_suite(audit, &self.assigned_deck, self.champion())?;
        self.pending_operation = Some(PendingOperation::FinalAudit {
            suite_id: final_audit_suite_id(audit)?,
        });
        Ok(())
    }

    /// Completes the exact final audit previously reserved by [`Self::reserve_final_audit`].
    ///
    /// Once the exact reservation is consumed, every returned evaluation failure leaves the
    /// campaign permanently failed and clears the pending operation.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] when the suite differs or replay evaluation fails.
    pub fn complete_final_audit(
        &mut self,
        audit: &[SelfPlayPair<'_>],
    ) -> Result<SelfPlayAudit, SelfPlayError> {
        let suite_id = final_audit_suite_id(audit)?;
        if !matches!(
            &self.pending_operation,
            Some(PendingOperation::FinalAudit { suite_id: expected }) if expected == &suite_id
        ) {
            return Err(SelfPlayError::Invalid(
                "self-play final audit does not match the pending suite",
            ));
        }
        self.final_audit_state.begin()?;
        self.pending_operation = None;
        let prepared = prepare_suite(audit, &self.assigned_deck, self.champion())?;
        let baseline_score = score_policy(
            &self.initial_policy,
            self.assigned_deck.deck_id(),
            &prepared,
            self.max_actions,
            true,
        )?;
        let score = score_policy(
            self.champion(),
            self.assigned_deck.deck_id(),
            &prepared,
            self.max_actions,
            true,
        )?;
        if !self.promoted_policies.is_empty()
            && !improves_without_regression_at_significance(
                &score,
                &baseline_score,
                CAMPAIGN_SIGNIFICANCE_DENOMINATOR,
            )
        {
            return Err(SelfPlayError::Invalid(
                "final audit did not reproduce a campaign-wide significant gain",
            ));
        }
        let audit = SelfPlayAudit {
            classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
            policy_id: self.champion().policy_id().clone(),
            baseline_score,
            score,
        };
        self.final_audit_state = FinalAuditState::Complete;
        Ok(audit)
    }

    /// Replays a fresh audit suite against the champion, then permanently seals the campaign.
    ///
    /// Use [`Self::reserve_final_audit`], persist a checkpoint, then call
    /// [`Self::complete_final_audit`] when crash-discard protection is required.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] for reservation or replay failure.
    pub fn final_audit(
        &mut self,
        audit: &[SelfPlayPair<'_>],
    ) -> Result<SelfPlayAudit, SelfPlayError> {
        if self.pending_operation.is_none() {
            self.reserve_final_audit(audit)?;
        }
        self.complete_final_audit(audit)
    }
}

fn checkpoint_body_value(checkpoint: &SelfPlayCampaignCheckpoint) -> serde_json::Value {
    let mut body = serde_json::json!({
        "assignedDeckId": checkpoint.assigned_deck_id,
        "finalAuditState": checkpoint.final_audit_state,
        "initialPolicy": checkpoint.initial_policy,
        "kind": checkpoint.kind,
        "maxActions": checkpoint.max_actions,
        "promotionAttempts": checkpoint.promotion_attempts,
        "promotionPortfolio": checkpoint.promotion_portfolio,
        "promotedPolicies": checkpoint.promoted_policies,
        "schemaVersion": checkpoint.schema_version,
        "usedDevelopmentSeeds": checkpoint.used_development_seeds,
    });
    if let Some(pending) = &checkpoint.pending_operation {
        body["pendingOperation"] = serde_json::json!(pending);
    }
    body
}

fn validate_checkpoint(checkpoint: &SelfPlayCampaignCheckpoint) -> Result<(), SelfPlayError> {
    if checkpoint.kind != "sorcery-self-play-campaign-checkpoint"
        || checkpoint.schema_version != 1
        || checkpoint.max_actions == 0
        || checkpoint.promotion_attempts > MAX_CAMPAIGN_PROMOTION_ATTEMPTS
    {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint kind, version, or bounds are invalid",
        ));
    }
    if identity_hash(&checkpoint_body_value(checkpoint))? != checkpoint.checkpoint_id {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint identity is invalid",
        ));
    }
    if checkpoint.assigned_deck_id != *checkpoint.initial_policy.deck_id()
        || checkpoint.initial_policy.generation() != 0
        || checkpoint.initial_policy.parent_policy_id().is_some()
    {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint root policy is invalid",
        ));
    }
    serialize_policy_snapshot(&checkpoint.initial_policy)?;
    validate_checkpoint_lineage(checkpoint)?;
    validate_checkpoint_progress(checkpoint)?;
    Ok(())
}

fn validate_checkpoint_lineage(
    checkpoint: &SelfPlayCampaignCheckpoint,
) -> Result<(), SelfPlayError> {
    if checkpoint.promoted_policies.len() > usize::from(checkpoint.promotion_attempts) {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint has more promotions than attempts",
        ));
    }
    let mut parent = &checkpoint.initial_policy;
    for policy in &checkpoint.promoted_policies {
        serialize_policy_snapshot(policy)?;
        policy.validate_binding(
            checkpoint.initial_policy.authority_hash(),
            &checkpoint.assigned_deck_id,
            checkpoint.initial_policy.engine_version(),
        )?;
        if policy.parent_policy_id() != Some(parent.policy_id())
            || policy.generation() != parent.generation().saturating_add(1)
            || !parent
                .neighbors()?
                .iter()
                .any(|neighbor| neighbor == policy)
        {
            return Err(SelfPlayError::Invalid(
                "self-play checkpoint policy lineage is invalid",
            ));
        }
        parent = policy;
    }
    Ok(())
}

fn validate_checkpoint_progress(
    checkpoint: &SelfPlayCampaignCheckpoint,
) -> Result<(), SelfPlayError> {
    let portfolio_size = match (
        checkpoint.promotion_attempts,
        checkpoint.promotion_portfolio.as_deref(),
    ) {
        (0, None) if checkpoint.used_development_seeds.is_empty() => 0,
        (attempts, Some(entries)) if attempts > 0 => validate_checkpoint_portfolio(entries)?,
        _ => {
            return Err(SelfPlayError::Invalid(
                "self-play checkpoint progress is inconsistent",
            ));
        }
    };
    let attempts = usize::from(checkpoint.promotion_attempts);
    if checkpoint
        .used_development_seeds
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
        || checkpoint.used_development_seeds.len() < attempts * (portfolio_size + 1)
        || checkpoint.used_development_seeds.len()
            > attempts * (portfolio_size + MAX_SELF_PLAY_PAIRS)
    {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint development seeds are invalid",
        ));
    }
    if checkpoint.promotion_attempts == 0 && checkpoint.final_audit_state != FinalAuditState::Open {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint audit state is inconsistent",
        ));
    }
    match (&checkpoint.pending_operation, checkpoint.final_audit_state) {
        (None, _) => {}
        (Some(PendingOperation::Generation { .. }), FinalAuditState::Open)
            if checkpoint.promotion_attempts < MAX_CAMPAIGN_PROMOTION_ATTEMPTS => {}
        (Some(PendingOperation::FinalAudit { .. }), FinalAuditState::Open)
            if checkpoint.promotion_attempts > 0 => {}
        _ => {
            return Err(SelfPlayError::Invalid(
                "self-play checkpoint pending operation is inconsistent",
            ));
        }
    }
    Ok(())
}

fn validate_checkpoint_portfolio(entries: &[PortfolioEntry]) -> Result<usize, SelfPlayError> {
    if entries.windows(2).any(|pair| pair[0].cell >= pair[1].cell)
        || entries
            .iter()
            .any(|entry| entry.cell.subgroup.trim().is_empty())
    {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint portfolio is not canonical",
        ));
    }
    let total = entries.iter().try_fold(0_usize, |total, entry| {
        if entry.count == 0 || entry.count > MAX_SELF_PLAY_PAIRS {
            return Err(SelfPlayError::Invalid(
                "self-play checkpoint portfolio count is invalid",
            ));
        }
        total.checked_add(entry.count).ok_or(SelfPlayError::Invalid(
            "self-play checkpoint portfolio count overflowed",
        ))
    })?;
    if !(MIN_PROMOTION_PAIRS..=MAX_SELF_PLAY_PAIRS).contains(&total) {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint portfolio size is invalid",
        ));
    }
    Ok(total)
}

fn portfolio_entries(portfolio: &Portfolio) -> Vec<PortfolioEntry> {
    portfolio
        .iter()
        .map(|(cell, &count)| PortfolioEntry {
            cell: cell.clone(),
            count,
        })
        .collect()
}

fn parse_checkpoint_policy(value: &serde_json::Value) -> Result<PolicySnapshot, SelfPlayError> {
    Ok(parse_policy_snapshot(&canonical_json(value)?)?)
}

/// Captures restart-safe campaign control state without authority-private card data.
///
/// # Errors
///
/// Returns [`SelfPlayError`] when the campaign state cannot be validated or hashed.
pub fn create_selfplay_campaign_checkpoint(
    campaign: &SelfPlayCampaign,
) -> Result<SelfPlayCampaignCheckpoint, SelfPlayError> {
    let mut checkpoint = SelfPlayCampaignCheckpoint {
        assigned_deck_id: campaign.assigned_deck.deck_id().clone(),
        checkpoint_id: identity_hash(&serde_json::Value::Null)?,
        final_audit_state: campaign.final_audit_state,
        initial_policy: campaign.initial_policy.clone(),
        kind: "sorcery-self-play-campaign-checkpoint".to_owned(),
        max_actions: campaign.max_actions,
        pending_operation: campaign.pending_operation.clone(),
        promotion_attempts: campaign.promotion_attempts,
        promotion_portfolio: campaign.promotion_portfolio.as_ref().map(portfolio_entries),
        promoted_policies: campaign.promoted_policies.clone(),
        schema_version: 1,
        used_development_seeds: campaign.used_development_seeds.iter().copied().collect(),
    };
    checkpoint.checkpoint_id = identity_hash(&checkpoint_body_value(&checkpoint))?;
    validate_checkpoint(&checkpoint)?;
    Ok(checkpoint)
}

/// Serializes a validated campaign checkpoint as its unique canonical JSON representation.
///
/// # Errors
///
/// Returns [`SelfPlayError`] when the checkpoint is inconsistent or cannot be serialized.
pub fn serialize_selfplay_campaign_checkpoint(
    checkpoint: &SelfPlayCampaignCheckpoint,
) -> Result<String, SelfPlayError> {
    validate_checkpoint(checkpoint)?;
    Ok(canonical_json(&serde_json::to_value(checkpoint)?)?)
}

/// Parses strict canonical campaign state and verifies its self-hash and policy lineage.
///
/// # Errors
///
/// Returns [`SelfPlayError`] for oversized, noncanonical, malformed, or inconsistent input.
pub fn parse_selfplay_campaign_checkpoint(
    text: &str,
) -> Result<SelfPlayCampaignCheckpoint, SelfPlayError> {
    if text.len() > SELF_PLAY_CHECKPOINT_MAX_BYTES {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint exceeds the supported byte bound",
        ));
    }
    let value = parse_json_without_duplicate_keys(text)?;
    if canonical_json(&value)? != text {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint must use canonical JSON",
        ));
    }
    let raw: RawSelfPlayCampaignCheckpoint = serde_json::from_value(value)?;
    let checkpoint = SelfPlayCampaignCheckpoint {
        assigned_deck_id: raw.assigned_deck_id,
        checkpoint_id: raw.checkpoint_id,
        final_audit_state: raw.final_audit_state,
        initial_policy: parse_checkpoint_policy(&raw.initial_policy)?,
        kind: raw.kind,
        max_actions: raw.max_actions,
        pending_operation: raw.pending_operation,
        promotion_attempts: raw.promotion_attempts,
        promotion_portfolio: raw.promotion_portfolio,
        promoted_policies: raw
            .promoted_policies
            .iter()
            .map(parse_checkpoint_policy)
            .collect::<Result<_, _>>()?,
        schema_version: raw.schema_version,
        used_development_seeds: raw.used_development_seeds,
    };
    validate_checkpoint(&checkpoint)?;
    Ok(checkpoint)
}

/// Restores a campaign only after revalidating its external assigned deck.
///
/// # Errors
///
/// Returns [`SelfPlayError`] when the checkpoint or supplied deck binding is invalid.
pub fn resume_selfplay_campaign(
    checkpoint: &SelfPlayCampaignCheckpoint,
    assigned_deck: DeckValidation,
) -> Result<SelfPlayCampaign, SelfPlayError> {
    validate_checkpoint(checkpoint)?;
    if assigned_deck.deck_id() != &checkpoint.assigned_deck_id {
        return Err(SelfPlayError::Invalid(
            "self-play checkpoint assigned deck does not match",
        ));
    }
    validate_assigned_policy(&checkpoint.initial_policy, &assigned_deck)?;
    Ok(SelfPlayCampaign {
        assigned_deck,
        final_audit_state: checkpoint.final_audit_state,
        initial_policy: checkpoint.initial_policy.clone(),
        max_actions: checkpoint.max_actions,
        pending_operation: checkpoint.pending_operation.clone(),
        promotion_portfolio: checkpoint.promotion_portfolio.as_ref().map(|entries| {
            entries
                .iter()
                .map(|entry| (entry.cell.clone(), entry.count))
                .collect()
        }),
        promotion_attempts: checkpoint.promotion_attempts,
        promoted_policies: checkpoint.promoted_policies.clone(),
        used_development_seeds: checkpoint.used_development_seeds.iter().copied().collect(),
    })
}

fn validate_assigned_policy(
    policy: &PolicySnapshot,
    assigned_deck: &DeckValidation,
) -> Result<(), SelfPlayError> {
    if !assigned_deck.ranked_eligible() || policy.deck_id() != assigned_deck.deck_id() {
        return Err(SelfPlayError::Invalid(
            "self-play requires the policy's format-legal, engine-supported assigned deck",
        ));
    }
    Ok(())
}

fn raw_seeds(pairs: &[SelfPlayPair<'_>]) -> BTreeSet<u32> {
    pairs.iter().map(|pair| pair.seed).collect()
}

fn suite_value(pairs: &[SelfPlayPair<'_>]) -> Result<serde_json::Value, SelfPlayError> {
    pairs
        .iter()
        .map(|pair| {
            Ok(serde_json::json!({
                "candidateAsNorthManifest": parse_json_without_duplicate_keys(
                    pair.candidate_as_north_manifest_json,
                )?,
                "candidateAsSouthManifest": parse_json_without_duplicate_keys(
                    pair.candidate_as_south_manifest_json,
                )?,
                "opponentDeckId": pair.opponent_deck.deck_id(),
                "opponentPolicyId": pair.opponent.policy_id(),
                "seed": pair.seed,
                "subgroup": pair.subgroup,
            }))
        })
        .collect::<Result<Vec<_>, SelfPlayError>>()
        .map(serde_json::Value::Array)
}

fn generation_suite_id(
    training: &[SelfPlayPair<'_>],
    promotion: &[SelfPlayPair<'_>],
) -> Result<IdentityHash, SelfPlayError> {
    Ok(identity_hash(&serde_json::json!({
        "kind": "self-play-generation",
        "promotion": suite_value(promotion)?,
        "training": suite_value(training)?,
    }))?)
}

fn final_audit_suite_id(audit: &[SelfPlayPair<'_>]) -> Result<IdentityHash, SelfPlayError> {
    Ok(identity_hash(&serde_json::json!({
        "audit": suite_value(audit)?,
        "kind": "self-play-final-audit",
    }))?)
}

fn portfolio(pairs: &[SelfPlayPair<'_>]) -> Result<Portfolio, SelfPlayError> {
    let mut portfolio = Portfolio::new();
    for pair in pairs {
        let manifest: serde_json::Value =
            serde_json::from_str(pair.candidate_as_north_manifest_json)?;
        let mut scenario = shared_manifest_body(&manifest).ok_or(SelfPlayError::Invalid(
            "self-play manifest lacks a complete shared scenario body",
        ))?;
        scenario
            .as_object_mut()
            .expect("shared manifest body is an object")
            .remove("seed")
            .ok_or(SelfPlayError::Invalid(
                "self-play manifest lacks a scenario seed",
            ))?;
        let scenario_id = identity_hash(&scenario).map_err(|_| {
            SelfPlayError::Invalid("self-play scenario identity could not be canonicalized")
        })?;
        let count = portfolio
            .entry(PortfolioCell {
                subgroup: pair.subgroup.to_owned(),
                opponent_policy_id: pair.opponent.policy_id().clone(),
                opponent_deck_id: pair.opponent_deck.deck_id().clone(),
                scenario_id,
            })
            .or_default();
        *count = count.checked_add(1).ok_or(SelfPlayError::Invalid(
            "self-play portfolio count overflowed",
        ))?;
    }
    Ok(portfolio)
}

fn portfolio_covers(candidate: &Portfolio, expected: &Portfolio) -> bool {
    candidate.len() == expected.len()
        && expected
            .iter()
            .all(|(cell, count)| candidate.get(cell).is_some_and(|seen| seen >= count))
}

/// Nominates one deterministic neighbor and promotes it only after replay-gated heldout gains.
///
/// # Errors
///
/// Returns [`SelfPlayError`] for invalid deck bindings, malformed seat pairs, overlapping seed
/// splits, bounded nonterminal games, unsupported engine behavior, or replay divergence.
pub fn train_and_promote(
    champion: &PolicySnapshot,
    assigned_deck: &DeckValidation,
    training: &[SelfPlayPair<'_>],
    heldout: &[SelfPlayPair<'_>],
    max_actions: usize,
) -> Result<PromotionResult, SelfPlayError> {
    train_and_promote_at_significance(champion, assigned_deck, training, heldout, max_actions, 100)
}

fn train_and_promote_at_significance(
    champion: &PolicySnapshot,
    assigned_deck: &DeckValidation,
    training: &[SelfPlayPair<'_>],
    heldout: &[SelfPlayPair<'_>],
    max_actions: usize,
    significance_denominator: u128,
) -> Result<PromotionResult, SelfPlayError> {
    validate_assigned_policy(champion, assigned_deck)?;
    let training = prepare_suite(training, assigned_deck, champion)?;
    let heldout = prepare_suite(heldout, assigned_deck, champion)?;
    let training_seeds = seeds(&training);
    if heldout
        .iter()
        .any(|case| training_seeds.contains(&case.seed))
    {
        return Err(SelfPlayError::Invalid(
            "training and heldout seeds must be disjoint",
        ));
    }

    let mut nominee: Option<(PolicySnapshot, SelfPlayScore)> = None;
    for child in champion.neighbors()? {
        let score = match score_policy(
            &child,
            assigned_deck.deck_id(),
            &training,
            max_actions,
            false,
        ) {
            Ok(score) => score,
            Err(SelfPlayError::NonTerminal) => continue,
            Err(error) => return Err(error),
        };
        let replace = nominee.as_ref().is_none_or(|(best, best_score)| {
            score.half_points > best_score.half_points
                || score.half_points == best_score.half_points
                    && child.policy_id() < best.policy_id()
        });
        if replace {
            nominee = Some((child, score));
        }
    }
    let (nominee, nominee_training) = nominee.ok_or(SelfPlayError::Invalid(
        "champion produced no policy neighbors",
    ))?;
    let champion_heldout = score_policy(
        champion,
        assigned_deck.deck_id(),
        &heldout,
        max_actions,
        true,
    )?;
    let nominee_heldout = score_policy(
        &nominee,
        assigned_deck.deck_id(),
        &heldout,
        max_actions,
        true,
    )?;
    let promoted = improves_without_regression_at_significance(
        &nominee_heldout,
        &champion_heldout,
        significance_denominator,
    );
    let nominee_policy_id = nominee.policy_id().clone();
    Ok(PromotionResult {
        classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
        policy: if promoted { nominee } else { champion.clone() },
        promoted,
        nominee_policy_id,
        nominee_training,
        champion_heldout,
        nominee_heldout,
    })
}

#[cfg(test)]
fn improves_without_regression(candidate: &SelfPlayScore, champion: &SelfPlayScore) -> bool {
    improves_without_regression_at_significance(candidate, champion, 100)
}

fn improves_without_regression_at_significance(
    candidate: &SelfPlayScore,
    champion: &SelfPlayScore,
    significance_denominator: u128,
) -> bool {
    if candidate.pairs.len() < MIN_PROMOTION_PAIRS
        || candidate.pairs.len() > MAX_SELF_PLAY_PAIRS
        || candidate.pairs.len() != champion.pairs.len()
        || candidate
            .pairs
            .iter()
            .zip(&champion.pairs)
            .any(|(candidate, champion)| !candidate.same_case(champion))
        || candidate.half_points <= champion.half_points
    {
        return false;
    }

    let mut total_gain = 0_i16;
    let mut wins = 0;
    let mut losses = 0;
    let mut subgroup_seat_deltas = BTreeMap::<&str, [i16; 2]>::new();
    for (candidate, champion) in candidate.pairs.iter().zip(&champion.pairs) {
        let north = i16::from(candidate.candidate_as_north_half_points)
            - i16::from(champion.candidate_as_north_half_points);
        let south = i16::from(candidate.candidate_as_south_half_points)
            - i16::from(champion.candidate_as_south_half_points);
        let difference = north + south;
        total_gain += difference;
        wins += usize::from(difference > 0);
        losses += usize::from(difference < 0);
        let subgroup = subgroup_seat_deltas
            .entry(candidate.subgroup.as_str())
            .or_default();
        subgroup[0] += north;
        subgroup[1] += south;
    }

    total_gain > 0
        && subgroup_seat_deltas
            .values()
            .all(|seats| seats[0] >= 0 && seats[1] >= 0)
        && 5 * i32::from(total_gain)
            >= i32::try_from(candidate.pairs.len()).expect("self-play pair bound fits i32")
        && exact_sign_test_at_most(wins, losses, significance_denominator)
}

#[cfg(test)]
fn exact_sign_test_at_most_one_percent(wins: usize, losses: usize) -> bool {
    exact_sign_test_at_most(wins, losses, 100)
}

fn exact_sign_test_at_most(wins: usize, losses: usize, denominator: u128) -> bool {
    let decisive = wins + losses;
    if decisive == 0 || decisive > MAX_SELF_PLAY_PAIRS || denominator == 0 {
        return false;
    }

    let mut row = vec![0_u128; decisive + 1];
    row[0] = 1;
    for n in 1..=decisive {
        for k in (1..=n).rev() {
            row[k] = row[k].saturating_add(row[k - 1]);
        }
    }
    let upper_tail = row[wins..]
        .iter()
        .copied()
        .fold(0_u128, u128::saturating_add);
    let threshold = if decisive == u128::BITS as usize {
        let quotient = u128::MAX / denominator;
        let remainder = u128::MAX % denominator;
        quotient + u128::from(remainder == denominator - 1)
    } else {
        (1_u128 << decisive) / denominator
    };
    upper_tail <= threshold
}

fn score_policy(
    candidate: &PolicySnapshot,
    assigned_deck_id: &IdentityHash,
    pairs: &[PreparedPair<'_>],
    max_actions: usize,
    verify_replays: bool,
) -> Result<SelfPlayScore, SelfPlayError> {
    if candidate.deck_id() != assigned_deck_id {
        return Err(SelfPlayError::Invalid(
            "policy child changed its assigned deck binding",
        ));
    }
    let mut score = SelfPlayScore {
        games: 0,
        half_points: 0,
        subgroup_half_points: BTreeMap::new(),
        pairs: Vec::with_capacity(pairs.len()),
    };
    for pair in pairs {
        let mut seat_half_points = [0_u8; 2];
        for (seat_index, (candidate_seat, manifest_json, game)) in [
            (
                Seat::North,
                pair.candidate_as_north_manifest_json,
                &pair.candidate_as_north,
            ),
            (
                Seat::South,
                pair.candidate_as_south_manifest_json,
                &pair.candidate_as_south,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            candidate.validate_binding(
                game.rules().authority_hash(),
                assigned_deck_id,
                game.rules().engine_version(),
            )?;
            let (north, south) = match candidate_seat {
                Seat::North => (candidate, pair.opponent),
                Seat::South => (pair.opponent, candidate),
            };
            let rollout = run_game(game.clone(), north, south, max_actions)?;
            let outcome = rollout.outcome().ok_or(SelfPlayError::NonTerminal)?;
            if verify_replays {
                let replay = replay_selected(manifest_json, &rollout)?;
                if replay.outcome() != Some(outcome) {
                    return Err(SelfPlayError::Invalid(
                        "heldout replay did not reproduce the rollout outcome",
                    ));
                }
            }
            let half_points = match outcome {
                GameOutcome::Draw => 1,
                GameOutcome::Win { winner, .. } if winner == candidate_seat => 2,
                GameOutcome::Win { .. } => 0,
            };
            seat_half_points[seat_index] = half_points;
            score.games = score
                .games
                .checked_add(1)
                .ok_or(SelfPlayError::Invalid("self-play game count overflowed"))?;
            score.half_points = score
                .half_points
                .checked_add(u64::from(half_points))
                .ok_or(SelfPlayError::Invalid(
                    "self-play half-point total overflowed",
                ))?;
            let subgroup = score
                .subgroup_half_points
                .entry(pair.subgroup.to_owned())
                .or_default();
            *subgroup =
                subgroup
                    .checked_add(u64::from(half_points))
                    .ok_or(SelfPlayError::Invalid(
                        "self-play subgroup score overflowed",
                    ))?;
        }
        score.pairs.push(SelfPlayPairScore {
            seed: pair.seed,
            subgroup: pair.subgroup.to_owned(),
            opponent_policy_id: pair.opponent.policy_id().clone(),
            north_manifest_id: pair.candidate_as_north.rules().manifest_id().clone(),
            south_manifest_id: pair.candidate_as_south.rules().manifest_id().clone(),
            candidate_as_north_half_points: seat_half_points[0],
            candidate_as_south_half_points: seat_half_points[1],
        });
    }
    Ok(score)
}

fn seeds(pairs: &[PreparedPair<'_>]) -> BTreeSet<u32> {
    pairs.iter().map(|pair| pair.seed).collect()
}

fn prepare_suite<'a>(
    pairs: &[SelfPlayPair<'a>],
    assigned_deck: &DeckValidation,
    champion: &PolicySnapshot,
) -> Result<Vec<PreparedPair<'a>>, SelfPlayError> {
    if pairs.is_empty() || pairs.len() > MAX_SELF_PLAY_PAIRS {
        return Err(SelfPlayError::Invalid(
            "self-play suites must contain 1-128 seat-swapped pairs",
        ));
    }
    let mut seen_seeds = BTreeSet::new();
    let mut seen_pair_identities = BTreeSet::new();
    let mut prepared = Vec::with_capacity(pairs.len());
    for pair in pairs {
        if pair.subgroup.trim().is_empty() {
            return Err(SelfPlayError::Invalid(
                "self-play subgroup must be nonempty",
            ));
        }
        if !seen_seeds.insert(pair.seed) {
            return Err(SelfPlayError::Invalid("self-play suite repeats a seed"));
        }
        if !pair.opponent_deck.ranked_eligible()
            || pair.opponent.deck_id() != pair.opponent_deck.deck_id()
        {
            return Err(SelfPlayError::Invalid(
                "self-play requires the opponent's ranked-eligible assigned deck",
            ));
        }
        let north_raw: serde_json::Value =
            serde_json::from_str(pair.candidate_as_north_manifest_json)?;
        let south_raw: serde_json::Value =
            serde_json::from_str(pair.candidate_as_south_manifest_json)?;
        if manifest_seed(&north_raw) != Some(pair.seed)
            || manifest_seed(&south_raw) != Some(pair.seed)
        {
            return Err(SelfPlayError::Invalid(
                "self-play pair seed does not match both manifests",
            ));
        }
        if shared_manifest_body(&north_raw) != shared_manifest_body(&south_raw) {
            return Err(SelfPlayError::Invalid(
                "self-play seat pair changed authority, card facts, engine, seed, or setup",
            ));
        }
        let north_decks = manifest_decks(&north_raw)?;
        let south_decks = manifest_decks(&south_raw)?;
        if north_decks.0 != south_decks.1 || north_decks.1 != south_decks.0 {
            return Err(SelfPlayError::Invalid(
                "self-play manifests are not exact deck seat swaps",
            ));
        }
        if !manifest_deck_matches(north_decks.0, assigned_deck.deck())
            || !manifest_deck_matches(north_decks.1, pair.opponent_deck.deck())
        {
            return Err(SelfPlayError::Invalid(
                "self-play manifest decks do not match their validated assignments",
            ));
        }
        let candidate_as_north = Game::from_manifest_json(pair.candidate_as_north_manifest_json)
            .map_err(SimulatorError::from)?;
        let candidate_as_south = Game::from_manifest_json(pair.candidate_as_south_manifest_json)
            .map_err(SimulatorError::from)?;
        for game in [&candidate_as_north, &candidate_as_south] {
            game.ensure_selfplay_supported()
                .map_err(SimulatorError::from)?;
            champion.validate_binding(
                game.rules().authority_hash(),
                assigned_deck.deck_id(),
                game.rules().engine_version(),
            )?;
            pair.opponent.validate_binding(
                game.rules().authority_hash(),
                pair.opponent_deck.deck_id(),
                game.rules().engine_version(),
            )?;
        }
        if !seen_pair_identities.insert((
            pair.seed,
            candidate_as_north.rules().manifest_id().clone(),
            candidate_as_south.rules().manifest_id().clone(),
            pair.opponent.policy_id().clone(),
        )) {
            return Err(SelfPlayError::Invalid(
                "self-play suite repeats a matchup under another subgroup",
            ));
        }
        prepared.push(PreparedPair {
            seed: pair.seed,
            subgroup: pair.subgroup,
            candidate_as_north_manifest_json: pair.candidate_as_north_manifest_json,
            candidate_as_south_manifest_json: pair.candidate_as_south_manifest_json,
            candidate_as_north,
            candidate_as_south,
            opponent: pair.opponent,
        });
    }
    Ok(prepared)
}

fn manifest_seed(manifest: &serde_json::Value) -> Option<u32> {
    u32::try_from(manifest.get("seed")?.as_u64()?).ok()
}

fn shared_manifest_body(manifest: &serde_json::Value) -> Option<serde_json::Value> {
    let mut body = manifest.as_object()?.clone();
    body.remove("decks")?;
    body.remove("manifestId")?;
    Some(serde_json::Value::Object(body))
}

fn manifest_decks(
    manifest: &serde_json::Value,
) -> Result<(&serde_json::Value, &serde_json::Value), SelfPlayError> {
    let decks = manifest
        .get("decks")
        .and_then(serde_json::Value::as_object)
        .ok_or(SelfPlayError::Invalid("self-play manifest lacks decks"))?;
    Ok((
        decks.get("north").ok_or(SelfPlayError::Invalid(
            "self-play manifest lacks North deck",
        ))?,
        decks.get("south").ok_or(SelfPlayError::Invalid(
            "self-play manifest lacks South deck",
        ))?,
    ))
}

fn manifest_deck_matches(manifest: &serde_json::Value, expected: &CanonicalDeck) -> bool {
    let Some(deck) = manifest.as_object() else {
        return false;
    };
    if deck.get("avatar").and_then(serde_json::Value::as_str) != Some(&expected.avatar) {
        return false;
    }
    [
        ("atlas", expected.atlas.as_slice()),
        ("spellbook", expected.spellbook.as_slice()),
    ]
    .into_iter()
    .all(|(zone, expected)| {
        let Some(ids) = deck.get(zone).and_then(serde_json::Value::as_array) else {
            return false;
        };
        let mut counts = BTreeMap::<&str, u32>::new();
        for id in ids {
            let Some(id) = id.as_str() else {
                return false;
            };
            let count = counts.entry(id).or_default();
            let Some(next) = count.checked_add(1) else {
                return false;
            };
            *count = next;
        }
        counts.len() == expected.len()
            && expected
                .iter()
                .all(|row| counts.get(row.card_id.as_str()).copied() == Some(row.copies))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::canonical::IdentityHash;

    use super::{
        FinalAuditState, SelfPlayPairScore, SelfPlayScore, exact_sign_test_at_most_one_percent,
        improves_without_regression,
    };

    fn identity(seed: u32, offset: u32) -> IdentityHash {
        IdentityHash::parse(&format!("sha256:{:064x}", seed + offset)).expect("test identity")
    }

    fn score(rows: &[(u8, u8, &str)]) -> SelfPlayScore {
        let mut score = SelfPlayScore {
            games: 0,
            half_points: 0,
            subgroup_half_points: BTreeMap::new(),
            pairs: Vec::new(),
        };
        for (index, &(north, south, subgroup)) in rows.iter().enumerate() {
            let seed = u32::try_from(index).expect("test seed");
            score.games += 2;
            score.half_points += u64::from(north + south);
            *score
                .subgroup_half_points
                .entry(subgroup.to_owned())
                .or_default() += u64::from(north + south);
            score.pairs.push(SelfPlayPairScore {
                seed,
                subgroup: subgroup.to_owned(),
                opponent_policy_id: identity(0, 1_000),
                north_manifest_id: identity(seed, 2_000),
                south_manifest_id: identity(seed, 3_000),
                candidate_as_north_half_points: north,
                candidate_as_south_half_points: south,
            });
        }
        score
    }

    #[test]
    fn promotion_requires_paired_evidence_and_no_seat_or_subgroup_regression() {
        let champion = score(&vec![(1, 1, "mirror"); 20]);

        assert!(!improves_without_regression(
            &score(&vec![(2, 1, "mirror"); 19]),
            &score(&vec![(1, 1, "mirror"); 19])
        ));

        let mut noisy_gain = Vec::new();
        noisy_gain.extend((0..11).map(|_| (2, 2, "mirror")));
        noisy_gain.extend((0..5).map(|_| (0, 1, "mirror")));
        noisy_gain.extend((0..4).map(|_| (1, 0, "mirror")));
        assert!(!improves_without_regression(&score(&noisy_gain), &champion));

        let mut reliable_gain = vec![(2, 1, "mirror"); 17];
        reliable_gain.extend(vec![(0, 1, "mirror"); 3]);
        assert!(improves_without_regression(
            &score(&reliable_gain),
            &champion
        ));

        let mut seat_regression = vec![(2, 1, "mirror"); 20];
        seat_regression[0] = (2, 0, "mirror");
        let seat_champion = score(&vec![(0, 2, "mirror"); 20]);
        assert!(!improves_without_regression(
            &score(&seat_regression),
            &seat_champion
        ));

        let mut changed_case = score(&reliable_gain);
        changed_case.pairs[0].seed = 999;
        assert!(!improves_without_regression(&changed_case, &champion));

        let mut subgroup_regression = vec![(2, 1, "aggro"); 17];
        subgroup_regression.extend(vec![(0, 1, "control"); 3]);
        let mut subgroup_champion = vec![(1, 1, "aggro"); 17];
        subgroup_champion.extend(vec![(1, 1, "control"); 3]);
        assert!(!improves_without_regression(
            &score(&subgroup_regression),
            &score(&subgroup_champion)
        ));

        assert!(!exact_sign_test_at_most_one_percent(6, 0));
        assert!(exact_sign_test_at_most_one_percent(7, 0));
        assert!(!exact_sign_test_at_most_one_percent(77, 51));
        assert!(exact_sign_test_at_most_one_percent(78, 50));
    }

    #[test]
    fn promotion_effect_boundary_should_be_exact() {
        let mut below_champion = vec![(1, 1, "mirror"); 15];
        below_champion.extend(vec![(2, 2, "mirror"); 3]);
        below_champion.extend(vec![(1, 1, "mirror"); 2]);
        let mut below = Vec::new();
        below.extend((0..8).map(|_| (2, 1, "mirror")));
        below.extend((0..7).map(|_| (1, 2, "mirror")));
        below.extend(vec![(0, 0, "mirror"); 3]);
        below.extend(vec![(1, 1, "mirror"); 2]);
        assert!(!improves_without_regression(
            &score(&below),
            &score(&below_champion)
        ));

        let mut boundary_champion = vec![(1, 1, "mirror"); 16];
        boundary_champion.extend(vec![(2, 2, "mirror"); 3]);
        boundary_champion.push((1, 1, "mirror"));
        let mut boundary = Vec::new();
        boundary.extend(vec![(2, 1, "mirror"); 8]);
        boundary.extend(vec![(1, 2, "mirror"); 8]);
        boundary.extend(vec![(0, 0, "mirror"); 3]);
        boundary.push((1, 1, "mirror"));
        assert!(improves_without_regression(
            &score(&boundary),
            &score(&boundary_champion)
        ));
    }

    #[test]
    fn failed_final_audit_should_permanently_consume_the_reserved_attempt() {
        let mut state = FinalAuditState::Open;

        state.begin().expect("first audit attempt");

        assert_eq!(state, FinalAuditState::Failed);
        assert!(state.begin().is_err());
    }
}
