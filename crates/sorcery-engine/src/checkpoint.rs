//! Authority-private deterministic session checkpoints.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::canonical::{
    CanonicalError, IdentityHash, canonical_json, identity_hash, parse_json_without_duplicate_keys,
};
use crate::contract::ActionRequest;
use crate::session::{Session, SessionError};

/// Maximum accepted checkpoint byte length.
pub const GAME_CHECKPOINT_MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_CHECKPOINT_REQUESTS: usize = 1_000;
const MAX_ACTION_ID_UTF16_UNITS: usize = 256;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// A private checkpoint that reconstructs hidden state by replaying every request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameCheckpoint {
    /// Hash of the body excluding this field.
    pub checkpoint_id: IdentityHash,
    /// Hash of the reconstructed state and journals.
    pub expected_session_hash: IdentityHash,
    /// Stable checkpoint contract discriminator.
    pub kind: String,
    /// Canonical authoritative manifest.
    pub manifest: Value,
    /// Every accepted and rejected request in original order.
    pub requests: Vec<ActionRequest>,
    /// Checkpoint schema version.
    pub schema_version: u8,
}

/// Checkpoint parsing, validation, hashing, or replay failed.
#[derive(Debug)]
pub enum CheckpointError {
    /// Canonical serialization or hashing failed.
    Canonical(CanonicalError),
    /// JSON decoding failed.
    Json(serde_json::Error),
    /// Session construction or replay failed.
    Session(SessionError),
    /// A checkpoint contract invariant was violated.
    Invalid(String),
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::Session(error) => error.fmt(formatter),
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl Error for CheckpointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Session(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<CanonicalError> for CheckpointError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<serde_json::Error> for CheckpointError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<SessionError> for CheckpointError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

fn body_value(checkpoint: &GameCheckpoint) -> Value {
    json!({
        "expectedSessionHash": checkpoint.expected_session_hash,
        "kind": checkpoint.kind,
        "manifest": checkpoint.manifest,
        "requests": checkpoint.requests,
        "schemaVersion": checkpoint.schema_version,
    })
}

fn validate(checkpoint: &GameCheckpoint) -> Result<(), CheckpointError> {
    if checkpoint.kind != "sorcery-game-checkpoint" || checkpoint.schema_version != 1 {
        return Err(CheckpointError::Invalid(
            "checkpoint kind or schema version is unsupported".to_owned(),
        ));
    }
    if checkpoint.requests.len() > MAX_CHECKPOINT_REQUESTS {
        return Err(CheckpointError::Invalid(format!(
            "checkpoint requests must contain at most {MAX_CHECKPOINT_REQUESTS} entries"
        )));
    }
    for (index, request) in checkpoint.requests.iter().enumerate() {
        let action_length = request.action_id.encode_utf16().count();
        if action_length == 0
            || action_length > MAX_ACTION_ID_UTF16_UNITS
            || request.state_version > MAX_SAFE_INTEGER
        {
            return Err(CheckpointError::Invalid(format!(
                "checkpoint request {index} is invalid"
            )));
        }
    }
    let manifest_json = canonical_json(&checkpoint.manifest)?;
    Session::new(&manifest_json)?;
    if identity_hash(&body_value(checkpoint))? != checkpoint.checkpoint_id {
        return Err(CheckpointError::Invalid(
            "checkpoint identity is invalid".to_owned(),
        ));
    }
    Ok(())
}

/// Captures a replayable checkpoint, including rejected request attempts.
///
/// # Errors
///
/// Returns [`CheckpointError`] when the session cannot be serialized or hashed.
pub fn create_game_checkpoint(session: &Session) -> Result<GameCheckpoint, CheckpointError> {
    let mut checkpoint = GameCheckpoint {
        checkpoint_id: identity_hash(&Value::Null)?,
        expected_session_hash: session.session_hash()?,
        kind: "sorcery-game-checkpoint".to_owned(),
        manifest: serde_json::from_str(session.manifest_json())?,
        requests: session
            .attempts()
            .iter()
            .map(|attempt| attempt.request.clone())
            .collect(),
        schema_version: 1,
    };
    checkpoint.checkpoint_id = identity_hash(&body_value(&checkpoint))?;
    validate(&checkpoint)?;
    Ok(checkpoint)
}

/// Serializes a validated checkpoint as canonical JSON.
///
/// # Errors
///
/// Returns [`CheckpointError`] when the checkpoint is invalid or cannot be serialized.
pub fn serialize_game_checkpoint(checkpoint: &GameCheckpoint) -> Result<String, CheckpointError> {
    validate(checkpoint)?;
    Ok(canonical_json(&serde_json::to_value(checkpoint)?)?)
}

/// Parses and validates a checkpoint without admitting duplicate JSON keys.
///
/// # Errors
///
/// Returns [`CheckpointError`] when bytes exceed the bound or violate the contract.
pub fn parse_game_checkpoint(text: &str) -> Result<GameCheckpoint, CheckpointError> {
    if text.len() > GAME_CHECKPOINT_MAX_BYTES {
        return Err(CheckpointError::Invalid(format!(
            "checkpoint exceeds {GAME_CHECKPOINT_MAX_BYTES} bytes"
        )));
    }
    let value = parse_json_without_duplicate_keys(text)?;
    let checkpoint: GameCheckpoint = serde_json::from_value(value)?;
    validate(&checkpoint)?;
    Ok(checkpoint)
}

/// Reconstructs a full session and verifies its expected hash.
///
/// # Errors
///
/// Returns [`CheckpointError`] when validation, replay, or session identity fails.
pub fn resume_game_checkpoint(checkpoint: &GameCheckpoint) -> Result<Session, CheckpointError> {
    validate(checkpoint)?;
    let manifest_json = canonical_json(&checkpoint.manifest)?;
    let mut session = Session::new(&manifest_json)?;
    for request in &checkpoint.requests {
        session.step(request.clone())?;
    }
    if session.session_hash()? != checkpoint.expected_session_hash {
        return Err(CheckpointError::Invalid(
            "checkpoint session hash does not match reconstructed history".to_owned(),
        ));
    }
    Ok(session)
}
