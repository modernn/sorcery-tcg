//! Authoritative request, receipt, and replay boundary.

use std::error::Error;
use std::fmt;

use serde_json::{Value, json};

use crate::canonical::{CanonicalError, IdentityHash, identity_hash};
use crate::contract::{
    ActionRequest, Attempt, LegalAction, Receipt, ReceiptInput, Rejection, RejectionCode, Seat,
    accepted_attempt, create_events, create_receipt, create_rejection, rejected_attempt,
};
use crate::game::{Game, GameError, MulliganAction};

/// An accepted receipt or stable rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepResult {
    /// The request changed authoritative state and produced a receipt.
    Accepted(Receipt),
    /// The request left authoritative state and transcript unchanged.
    Rejected(Rejection),
}

/// An authoritative game session and its journals.
#[derive(Clone, Debug)]
pub struct Session {
    attempts: Vec<Attempt>,
    game: Game,
    manifest_json: String,
    transcript: Vec<Receipt>,
}

/// Session construction, hashing, or replay failed.
#[derive(Debug)]
pub enum SessionError {
    /// The game manifest or transition failed validation.
    Game(GameError),
    /// Canonical hashing failed.
    Canonical(CanonicalError),
    /// Boundary serialization failed.
    Json(serde_json::Error),
    /// A replayed action was rejected.
    ReplayRejected(RejectionCode),
    /// A journal sequence exhausted its supported range.
    SequenceExhausted,
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Game(error) => error.fmt(formatter),
            Self::Canonical(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::ReplayRejected(code) => write!(formatter, "replay rejected action: {code:?}"),
            Self::SequenceExhausted => formatter.write_str("session journal sequence exhausted"),
        }
    }
}

impl Error for SessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Game(error) => Some(error),
            Self::Canonical(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::ReplayRejected(_) | Self::SequenceExhausted => None,
        }
    }
}

impl From<GameError> for SessionError {
    fn from(error: GameError) -> Self {
        Self::Game(error)
    }
}

impl From<CanonicalError> for SessionError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<serde_json::Error> for SessionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl Session {
    /// Creates a session from canonical manifest JSON.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the manifest or deterministic setup is invalid.
    pub fn new(manifest_json: &str) -> Result<Self, SessionError> {
        Ok(Self {
            attempts: Vec::new(),
            game: Game::from_manifest_json(manifest_json)?,
            manifest_json: manifest_json.to_owned(),
            transcript: Vec::new(),
        })
    }

    /// Returns legal actions materialized at the external boundary.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when action identity or descriptor serialization fails.
    pub fn legal_actions(&self) -> Result<Vec<LegalAction>, SessionError> {
        self.game
            .legal_mulligans()?
            .iter()
            .map(MulliganAction::to_legal_action)
            .collect::<Result<Vec<_>, _>>()
            .map_err(SessionError::Json)
    }

    /// Checks and applies one engine-issued action request.
    ///
    /// Rejections append only to the attempts journal. They do not mutate game
    /// state, PRNG state, or the accepted transcript.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when authoritative hashing or receipt creation fails.
    pub fn step(&mut self, request: ActionRequest) -> Result<StepResult, SessionError> {
        let state_version = self.game.position().state_version();
        let state_hash = self.game.state_hash()?;
        if request.state_version != state_version {
            return self.reject(
                request,
                state_version,
                state_hash,
                RejectionCode::StaleVersion,
            );
        }
        if request.seat != self.game.position().decision_seat() {
            return self.reject(request, state_version, state_hash, RejectionCode::WrongSeat);
        }
        let action = self
            .game
            .legal_mulligans()?
            .into_iter()
            .find(|action| action.action_id().as_str() == request.action_id);
        let Some(action) = action else {
            return self.reject(
                request,
                state_version,
                state_hash,
                RejectionCode::UnknownAction,
            );
        };

        let receipt_sequence = next_sequence(self.transcript.len())?;
        let event_count = self.transcript.iter().try_fold(0_u64, |count, receipt| {
            let length =
                u64::try_from(receipt.events.len()).map_err(|_| SessionError::SequenceExhausted)?;
            count
                .checked_add(length)
                .ok_or(SessionError::SequenceExhausted)
        })?;
        let first_event_sequence = event_count
            .checked_add(1)
            .ok_or(SessionError::SequenceExhausted)?;
        self.game.apply_mulligan(&action)?;
        let post_state_hash = self.game.state_hash()?;
        let mut outcomes = vec![(
            "mulligan-completed".to_owned(),
            json!({
                "atlasCount": action.descriptor().atlas_order().len(),
                "seat": action.seat(),
                "spellbookCount": action.descriptor().spellbook_order().len(),
            }),
        )];
        if action.seat() == Seat::South {
            outcomes.push((
                "turn-started".to_owned(),
                json!({
                    "drawSkipped": true,
                    "seat": self.game.rules().first_seat(),
                    "turnNumber": 1,
                }),
            ));
        }
        let events = create_events(
            action.action_id(),
            receipt_sequence,
            first_event_sequence,
            &outcomes,
        )?;
        let receipt = create_receipt(ReceiptInput {
            action_id: action.action_id().clone(),
            events,
            next_state_version: self.game.position().state_version(),
            post_state_hash,
            pre_state_hash: state_hash.clone(),
            random_draws: Vec::new(),
            receipt_sequence,
            seat: request.seat,
            state_version,
        })?;
        self.attempts.push(accepted_attempt(
            next_sequence(self.attempts.len())?,
            request,
            state_version,
            state_hash,
            receipt.receipt_id.clone(),
        ));
        self.transcript.push(receipt.clone());
        Ok(StepResult::Accepted(receipt))
    }

    /// Replays action IDs from the same canonical manifest.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when setup fails or a supplied action is rejected.
    pub fn replay(manifest_json: &str, action_ids: &[IdentityHash]) -> Result<Self, SessionError> {
        let mut session = Self::new(manifest_json)?;
        for action_id in action_ids {
            let request = ActionRequest {
                action_id: action_id.to_string(),
                seat: session.game.position().decision_seat(),
                state_version: session.game.position().state_version(),
            };
            if let StepResult::Rejected(rejection) = session.step(request)? {
                return Err(SessionError::ReplayRejected(rejection.code));
            }
        }
        Ok(session)
    }

    /// Replays this transcript and compares initial draws, state, and receipts.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] if the replay cannot be completed.
    pub fn verify_replay(&self) -> Result<bool, SessionError> {
        let action_ids: Vec<_> = self
            .transcript
            .iter()
            .map(|receipt| receipt.action_id.clone())
            .collect();
        let replayed = Self::replay(&self.manifest_json, &action_ids)?;
        Ok(
            self.game.initial_random_draws() == replayed.game.initial_random_draws()
                && self.game.authoritative_state() == replayed.game.authoritative_state()
                && self.transcript == replayed.transcript,
        )
    }

    /// Returns the immutable accepted-action transcript.
    #[must_use]
    pub fn transcript(&self) -> &[Receipt] {
        &self.transcript
    }

    /// Returns the request-attempt audit journal.
    #[must_use]
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }

    /// Returns the current authoritative state version.
    #[must_use]
    pub const fn state_version(&self) -> u64 {
        self.game.position().state_version()
    }

    /// Returns the current deciding seat.
    #[must_use]
    pub const fn decision_seat(&self) -> Seat {
        self.game.position().decision_seat()
    }

    /// Hashes the current authoritative state.
    ///
    /// # Errors
    ///
    /// Returns [`CanonicalError`] when state materialization cannot be canonicalized.
    pub fn state_hash(&self) -> Result<IdentityHash, CanonicalError> {
        self.game.state_hash()
    }

    /// Hashes authoritative setup draw records.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when draw serialization or hashing fails.
    pub fn initial_random_draws_hash(&self) -> Result<IdentityHash, SessionError> {
        self.game
            .initial_random_draws_hash()
            .map_err(SessionError::Game)
    }

    /// Hashes the accepted-action transcript.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when transcript serialization or hashing fails.
    pub fn transcript_hash(&self) -> Result<IdentityHash, SessionError> {
        Ok(identity_hash(&serde_json::to_value(&self.transcript)?)?)
    }

    /// Hashes all authoritative state and journals reconstructed by a checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when session data cannot be serialized or hashed.
    pub fn session_hash(&self) -> Result<IdentityHash, SessionError> {
        Ok(identity_hash(&json!({
            "attempts": self.attempts,
            "initialRandomDraws": self.game.initial_random_draws(),
            "manifestId": self.game.rules().manifest_id(),
            "state": self.game.authoritative_state(),
            "transcript": self.transcript,
        }))?)
    }

    /// Returns the canonical manifest bytes retained for replay and checkpoints.
    #[must_use]
    pub fn manifest_json(&self) -> &str {
        &self.manifest_json
    }

    /// Materializes replay-verification data at the authoritative boundary.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when setup draws cannot be serialized.
    pub fn replay_value(&self) -> Result<Value, SessionError> {
        Ok(json!({
            "initialRandomDraws": serde_json::to_value(self.game.initial_random_draws())?,
            "state": self.game.authoritative_state(),
            "transcript": self.transcript,
        }))
    }

    fn reject(
        &mut self,
        request: ActionRequest,
        state_version: u64,
        state_hash: IdentityHash,
        code: RejectionCode,
    ) -> Result<StepResult, SessionError> {
        let rejection = create_rejection(code, state_version, state_hash.clone());
        self.attempts.push(rejected_attempt(
            next_sequence(self.attempts.len())?,
            request,
            state_version,
            state_hash,
            code,
        ));
        Ok(StepResult::Rejected(rejection))
    }
}

fn next_sequence(length: usize) -> Result<u64, SessionError> {
    u64::try_from(length)
        .ok()
        .and_then(|length| length.checked_add(1))
        .ok_or(SessionError::SequenceExhausted)
}
