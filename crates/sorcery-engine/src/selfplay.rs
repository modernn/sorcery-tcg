//! Deterministic neighborhood training and replay-gated policy promotion.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::canonical::IdentityHash;
use crate::contract::Seat;
use crate::deck::DeckValidation;
use crate::game::{Game, GameOutcome};
use crate::policy::{PolicyError, PolicySnapshot};
use crate::simulator::{SimulatorError, replay_selected, run_game};

/// One side of a seat-swapped policy evaluation pair.
#[derive(Clone, Copy, Debug)]
pub struct SelfPlayCase<'a> {
    /// Seed declared by the manifest.
    pub seed: u32,
    /// Stable opponent or matchup grouping used for regression gates.
    pub subgroup: &'a str,
    /// Canonical manifest bytes for this seat orientation.
    pub manifest_json: &'a str,
    /// Seat controlled by the policy being evaluated.
    pub candidate_seat: Seat,
    /// Fixed opposing policy.
    pub opponent: &'a PolicySnapshot,
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
    /// Champion retained or immutable child promoted.
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

/// Nominates one deterministic neighbor and promotes it only after replay-gated heldout gains.
///
/// # Errors
///
/// Returns [`SelfPlayError`] for invalid deck bindings, malformed seat pairs, overlapping seed
/// splits, bounded nonterminal games, unsupported engine behavior, or replay divergence.
pub fn train_and_promote(
    champion: &PolicySnapshot,
    assigned_deck: &DeckValidation,
    training: &[SelfPlayCase<'_>],
    heldout: &[SelfPlayCase<'_>],
    max_actions: usize,
) -> Result<PromotionResult, SelfPlayError> {
    if !assigned_deck.ranked_eligible || champion.deck_id() != &assigned_deck.deck_id {
        return Err(SelfPlayError::Invalid(
            "self-play requires the champion's ranked-eligible assigned deck",
        ));
    }
    validate_suite(training)?;
    validate_suite(heldout)?;
    let training_seeds = seeds(training);
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
        let score = match score_policy(&child, &assigned_deck.deck_id, training, max_actions, false)
        {
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
    let champion_heldout =
        score_policy(champion, &assigned_deck.deck_id, heldout, max_actions, true)?;
    let nominee_heldout =
        score_policy(&nominee, &assigned_deck.deck_id, heldout, max_actions, true)?;
    let promoted = improves_without_regression(&nominee_heldout, &champion_heldout);
    let nominee_policy_id = nominee.policy_id().clone();
    Ok(PromotionResult {
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
    cases: &[SelfPlayCase<'_>],
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
    for case in cases {
        let game = Game::from_manifest_json(case.manifest_json).map_err(SimulatorError::from)?;
        candidate.validate_binding(
            game.rules().authority_hash(),
            assigned_deck_id,
            game.rules().engine_version(),
        )?;
        case.opponent.validate_binding(
            game.rules().authority_hash(),
            case.opponent.deck_id(),
            game.rules().engine_version(),
        )?;
        let (north, south) = match case.candidate_seat {
            Seat::North => (candidate, case.opponent),
            Seat::South => (case.opponent, candidate),
        };
        let rollout = run_game(game, north, south, max_actions)?;
        let outcome = rollout.outcome().ok_or(SelfPlayError::NonTerminal)?;
        if verify_replays {
            let replay = replay_selected(case.manifest_json, &rollout)?;
            if replay.outcome() != Some(outcome) {
                return Err(SelfPlayError::Invalid(
                    "heldout replay did not reproduce the rollout outcome",
                ));
            }
        }
        let half_points = match outcome {
            GameOutcome::Draw => 1,
            GameOutcome::Win { winner, .. } if winner == case.candidate_seat => 2,
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
            .entry(case.subgroup.to_owned())
            .or_default();
        *subgroup = subgroup
            .checked_add(half_points)
            .ok_or(SelfPlayError::Invalid(
                "self-play subgroup score overflowed",
            ))?;
    }
    Ok(score)
}

fn seeds(cases: &[SelfPlayCase<'_>]) -> BTreeSet<u32> {
    cases.iter().map(|case| case.seed).collect()
}

fn validate_suite(cases: &[SelfPlayCase<'_>]) -> Result<(), SelfPlayError> {
    if cases.is_empty() {
        return Err(SelfPlayError::Invalid(
            "self-play suites must contain at least one seat-swapped pair",
        ));
    }
    let mut pairs = BTreeMap::<(u32, &str), u8>::new();
    for case in cases {
        if case.subgroup.trim().is_empty() {
            return Err(SelfPlayError::Invalid(
                "self-play subgroup must be nonempty",
            ));
        }
        let manifest: serde_json::Value = serde_json::from_str(case.manifest_json)?;
        if manifest.get("seed").and_then(serde_json::Value::as_u64) != Some(u64::from(case.seed)) {
            return Err(SelfPlayError::Invalid(
                "self-play case seed does not match its manifest",
            ));
        }
        let seat_bit = match case.candidate_seat {
            Seat::North => 1,
            Seat::South => 2,
        };
        let pair = pairs.entry((case.seed, case.subgroup)).or_default();
        if *pair & seat_bit != 0 {
            return Err(SelfPlayError::Invalid(
                "self-play suite repeats a seat in one matchup",
            ));
        }
        *pair |= seat_bit;
    }
    if pairs.values().any(|pair| *pair != 3) {
        return Err(SelfPlayError::Invalid(
            "self-play suite must evaluate every matchup from both seats",
        ));
    }
    Ok(())
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
