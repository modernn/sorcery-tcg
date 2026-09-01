//! Deterministic neighborhood training and replay-gated policy promotion.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::batch::BatchClassification;
use crate::canonical::IdentityHash;
use crate::contract::Seat;
use crate::deck::{CanonicalDeck, DeckValidation};
use crate::game::{Game, GameOutcome};
use crate::policy::{PolicyError, PolicySnapshot};
use crate::simulator::{SimulatorError, replay_selected, run_game};

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

/// Integer policy score; a win is two half-points and a draw is one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelfPlayScore {
    games: u64,
    half_points: u64,
    subgroup_half_points: BTreeMap<String, u64>,
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
    /// Final replay-verified score.
    pub score: SelfPlayScore,
}

/// In-memory coordinator that prevents evaluation-seed reuse across generations.
#[derive(Debug)]
pub struct SelfPlayCampaign {
    assigned_deck: DeckValidation,
    final_audit_complete: bool,
    initial_policy: PolicySnapshot,
    promoted_policies: Vec<PolicySnapshot>,
    used_development_seeds: BTreeSet<u32>,
}

/// Self-play configuration, rollout, or replay verification failed.
#[derive(Debug)]
pub enum SelfPlayError {
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
            Self::Policy(error) => Some(error),
            Self::Simulator(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::NonTerminal | Self::Invalid(_) => None,
        }
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
    ) -> Result<Self, SelfPlayError> {
        validate_assigned_policy(&initial_policy, &assigned_deck)?;
        Ok(Self {
            assigned_deck,
            final_audit_complete: false,
            initial_policy,
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
        self.final_audit_complete
    }

    /// Runs one fresh training and promotion generation.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] for seed reuse, a sealed campaign, or any promotion failure.
    pub fn run_generation(
        &mut self,
        training: &[SelfPlayPair<'_>],
        promotion: &[SelfPlayPair<'_>],
        max_actions: usize,
    ) -> Result<PromotionResult, SelfPlayError> {
        if self.final_audit_complete {
            return Err(SelfPlayError::Invalid(
                "self-play campaign is sealed after final audit",
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
        let result = train_and_promote(
            self.champion(),
            &self.assigned_deck,
            training,
            promotion,
            max_actions,
        )?;
        self.used_development_seeds.extend(generation_seeds);
        if result.promoted {
            self.promoted_policies.push(result.policy.clone());
        }
        Ok(result)
    }

    /// Replays a fresh audit suite against the champion, then permanently seals the campaign.
    ///
    /// # Errors
    ///
    /// Returns [`SelfPlayError`] for prior seed use, a sealed campaign, or any replay failure.
    pub fn final_audit(
        &mut self,
        audit: &[SelfPlayPair<'_>],
        max_actions: usize,
    ) -> Result<SelfPlayAudit, SelfPlayError> {
        if self.final_audit_complete {
            return Err(SelfPlayError::Invalid(
                "self-play campaign is sealed after final audit",
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
        let prepared = prepare_suite(audit, &self.assigned_deck, self.champion())?;
        let score = score_policy(
            self.champion(),
            self.assigned_deck.deck_id(),
            &prepared,
            max_actions,
            true,
        )?;
        let audit = SelfPlayAudit {
            classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
            policy_id: self.champion().policy_id().clone(),
            score,
        };
        self.final_audit_complete = true;
        Ok(audit)
    }
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
    let promoted = improves_without_regression(&nominee_heldout, &champion_heldout);
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

fn improves_without_regression(candidate: &SelfPlayScore, champion: &SelfPlayScore) -> bool {
    candidate.half_points > champion.half_points
        && champion
            .subgroup_half_points
            .iter()
            .all(|(subgroup, score)| candidate.subgroup_half_points.get(subgroup) >= Some(score))
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
    };
    for pair in pairs {
        for (candidate_seat, manifest_json, game) in [
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
        ] {
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
            score.games = score
                .games
                .checked_add(1)
                .ok_or(SelfPlayError::Invalid("self-play game count overflowed"))?;
            score.half_points =
                score
                    .half_points
                    .checked_add(half_points)
                    .ok_or(SelfPlayError::Invalid(
                        "self-play half-point total overflowed",
                    ))?;
            let subgroup = score
                .subgroup_half_points
                .entry(pair.subgroup.to_owned())
                .or_default();
            *subgroup = subgroup
                .checked_add(half_points)
                .ok_or(SelfPlayError::Invalid(
                    "self-play subgroup score overflowed",
                ))?;
        }
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
    if pairs.is_empty() {
        return Err(SelfPlayError::Invalid(
            "self-play suites must contain at least one seat-swapped pair",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut prepared = Vec::with_capacity(pairs.len());
    for pair in pairs {
        if pair.subgroup.trim().is_empty() {
            return Err(SelfPlayError::Invalid(
                "self-play subgroup must be nonempty",
            ));
        }
        if !seen.insert((pair.seed, pair.subgroup)) {
            return Err(SelfPlayError::Invalid(
                "self-play suite repeats a seed and subgroup pair",
            ));
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

    use super::{SelfPlayScore, improves_without_regression};

    fn score(total: u64, control: u64, aggro: u64) -> SelfPlayScore {
        SelfPlayScore {
            games: 4,
            half_points: total,
            subgroup_half_points: BTreeMap::from([
                ("aggro".to_owned(), aggro),
                ("control".to_owned(), control),
            ]),
        }
    }

    #[test]
    fn promotion_requires_strict_total_gain_without_subgroup_regression() {
        let champion = score(4, 2, 2);

        assert!(!improves_without_regression(&score(4, 2, 2), &champion));
        assert!(!improves_without_regression(&score(5, 1, 4), &champion));
        assert!(improves_without_regression(&score(5, 2, 3), &champion));
    }
}
