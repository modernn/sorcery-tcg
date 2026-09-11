//! Immutable per-game artifacts for a finished authoritative session.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::path::{Component, Path};

use serde::Serialize;
use serde_json::Value;

use crate::batch::{BatchClassification, FinishedTerminal, MAX_GAME_ACTIONS};
use crate::canonical::{CanonicalError, IdentityHash, canonical_json, identity_hash};
use crate::contract::{ActionRequest, Event, Receipt};
use crate::eligibility::{EligibilityGates, EligibilityReport, evaluate_eligibility};
use crate::game::{ENGINE_VERSION, Game};
use crate::policy::{BASELINE_POLICY_DECK_ID, PolicySnapshot, baseline_policy_snapshot};
use crate::session::{Session, SessionError, StepResult};
use crate::simulator::{SimulatorError, replay_selected, run_game};
use crate::synthetic::synthetic_demo_manifest_json;

/// Coverage evidence collected from one finished transcript.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCoverage {
    /// Action kinds committed, in first-seen order.
    pub committed_action_kinds: Vec<String>,
    /// Event types emitted, in first-seen order.
    pub committed_event_types: Vec<String>,
    /// Action kinds offered by the engine, in first-seen order.
    pub offered_action_kinds: Vec<String>,
}

/// Immutable SIM-03 artifacts for one finished game.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRecord {
    /// Number of accepted actions.
    pub accepted_action_count: usize,
    /// Ranked/public result classification.
    pub classification: BatchClassification,
    /// Action and event kinds exercised by this game.
    pub coverage: GameCoverage,
    /// TEST-04 eligibility. Finished synthetic games stay unranked.
    pub eligibility: EligibilityReport,
    /// Canonical JSONL of every transcript event, one object per line.
    pub event_jsonl: String,
    /// Identity of the flattened event list.
    pub events_hash: IdentityHash,
    /// Number of fights started.
    pub fight_count: usize,
    /// Final authoritative state identity.
    pub final_state_hash: IdentityHash,
    /// Immutable bound manifest.
    pub manifest: Value,
    /// Canonical manifest identity.
    pub manifest_id: IdentityHash,
    /// Whether authoritative replay reproduced the transcript.
    pub replay_verified: bool,
    /// Record schema.
    pub schema_version: u8,
    /// Exact finished public terminal.
    pub terminal: FinishedTerminal,
    /// Accepted-action receipts.
    pub transcript: Vec<Receipt>,
    /// Identity of the accepted-action transcript.
    pub transcript_hash: IdentityHash,
    /// Final turn number.
    pub turn_count: u64,
}

/// Fixed filenames written by [`write_game_artifacts`].
pub const GAME_ARTIFACT_FILES: [&str; 5] = [
    "manifest.json",
    "transcript.json",
    "events.jsonl",
    "coverage.json",
    "outcome.json",
];

/// Why an artifact replay did not match the recorded game.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReplayMismatch {
    /// Saved `authority.contentHash` is not the hash the engine bound.
    Authority,
    /// Saved `engineVersion` is not this engine.
    EngineVersion,
    /// Flattened events did not reproduce the recorded events hash.
    EventsHash,
    /// Replayed state did not reproduce the recorded final state hash.
    FinalStateHash,
    /// Saved or replayed manifest identity disagreed with the recorded outcome.
    ManifestIdentity,
    /// A recorded action was not legal in the replayed position.
    ReplayRejected,
    /// Saved `schemaVersion` is not this engine's schema.
    SchemaVersion,
    /// Replayed receipts did not reproduce the recorded transcript hash.
    TranscriptHash,
}

/// Integer-canonical result of replaying one SIM-03 artifact directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReplayReport {
    /// Ranked/public result classification.
    pub classification: BatchClassification,
    /// TEST-04 eligibility. Replays stay unranked.
    pub eligibility: EligibilityReport,
    /// Every compared hash and identity matched.
    pub matched: bool,
    /// First classified mismatch, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<ReplayMismatch>,
    /// Authoritative replay reproduced the recorded transcript and hashes.
    pub replay_verified: bool,
    /// Replay report schema.
    pub schema_version: u8,
}

/// Classified outcome and terminal identities for one finished game.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOutcomeArtifact {
    /// Number of accepted actions.
    pub accepted_action_count: usize,
    /// Ranked/public result classification.
    pub classification: BatchClassification,
    /// TEST-04 eligibility. Finished synthetic games stay unranked.
    pub eligibility: EligibilityReport,
    /// Identity of the flattened event list.
    pub events_hash: IdentityHash,
    /// Number of fights started.
    pub fight_count: usize,
    /// Final authoritative state identity.
    pub final_state_hash: IdentityHash,
    /// Canonical manifest identity.
    pub manifest_id: IdentityHash,
    /// Whether authoritative replay reproduced the transcript.
    pub replay_verified: bool,
    /// Record schema.
    pub schema_version: u8,
    /// Exact finished public terminal.
    pub terminal: FinishedTerminal,
    /// Identity of the accepted-action transcript.
    pub transcript_hash: IdentityHash,
    /// Final turn number.
    pub turn_count: u64,
}

/// Building or writing a per-game record failed.
#[derive(Debug)]
pub enum GameRecordError {
    /// The compact rollout or authoritative replay failed.
    Simulator(SimulatorError),
    /// The session finished with a disagreeing outcome and reason.
    Invalid(&'static str),
    /// Writing artifact files failed.
    Io(std::io::Error),
    /// The session or rollout is still active.
    NonTerminal,
}

impl fmt::Display for GameRecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simulator(error) => error.fmt(formatter),
            Self::Invalid(message) => formatter.write_str(message),
            Self::Io(error) => error.fmt(formatter),
            Self::NonTerminal => formatter.write_str("game record requires a finished session"),
        }
    }
}

impl Error for GameRecordError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Simulator(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Invalid(_) | Self::NonTerminal => None,
        }
    }
}

impl From<SimulatorError> for GameRecordError {
    fn from(error: SimulatorError) -> Self {
        Self::Simulator(error)
    }
}

impl From<SessionError> for GameRecordError {
    fn from(error: SessionError) -> Self {
        Self::Simulator(SimulatorError::from(error))
    }
}

impl From<CanonicalError> for GameRecordError {
    fn from(error: CanonicalError) -> Self {
        Self::Simulator(SimulatorError::from(SessionError::from(error)))
    }
}

impl From<serde_json::Error> for GameRecordError {
    fn from(error: serde_json::Error) -> Self {
        Self::Simulator(SimulatorError::from(SessionError::from(error)))
    }
}

impl From<std::io::Error> for GameRecordError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl GameRecord {
    /// Returns the classified outcome artifact derived from this record.
    #[must_use]
    pub fn outcome_artifact(&self) -> GameOutcomeArtifact {
        GameOutcomeArtifact {
            accepted_action_count: self.accepted_action_count,
            classification: self.classification,
            eligibility: self.eligibility.clone(),
            events_hash: self.events_hash.clone(),
            fight_count: self.fight_count,
            final_state_hash: self.final_state_hash.clone(),
            manifest_id: self.manifest_id.clone(),
            replay_verified: self.replay_verified,
            schema_version: self.schema_version,
            terminal: self.terminal,
            transcript_hash: self.transcript_hash.clone(),
            turn_count: self.turn_count,
        }
    }
}

/// Writes the six SIM-03 artifact files into `dir`.
///
/// # Errors
///
/// Returns [`GameRecordError`] when `dir` escapes (`..`), cannot be created, or
/// a file cannot be written.
pub fn write_game_artifacts(dir: &Path, record: &GameRecord) -> Result<(), GameRecordError> {
    validate_artifacts_dir(dir)?;
    std::fs::create_dir_all(dir)?;
    write_canonical_file(&dir.join("manifest.json"), &record.manifest)?;
    write_canonical_file(
        &dir.join("transcript.json"),
        &serde_json::to_value(&record.transcript)?,
    )?;
    std::fs::write(dir.join("events.jsonl"), &record.event_jsonl)?;
    write_canonical_file(
        &dir.join("coverage.json"),
        &serde_json::to_value(&record.coverage)?,
    )?;
    write_canonical_file(
        &dir.join("outcome.json"),
        &serde_json::to_value(record.outcome_artifact())?,
    )?;
    Ok(())
}

/// Replays `manifest.json` + `transcript.json` and checks recorded hashes.
///
/// Classifies engine, schema, authority, identity, transcript, event, and
/// state mismatches instead of treating them as a silent success.
///
/// # Errors
///
/// Returns [`GameRecordError`] when the directory escapes, a required file is
/// missing, or the artifact JSON cannot be read.
pub fn replay_game_artifacts(dir: &Path) -> Result<ArtifactReplayReport, GameRecordError> {
    validate_artifacts_dir(dir)?;
    let manifest = read_artifact_json(dir, "manifest.json")?;
    let transcript = read_transcript(dir)?;
    let outcome = read_artifact_json(dir, "outcome.json")?;
    let events_hash = hash_events_file(dir)?;
    if let Some(mismatch) = manifest_version_mismatch(&manifest) {
        return Ok(mismatch_report(mismatch));
    }
    if manifest_authority_hash(&manifest).is_none() {
        return Ok(mismatch_report(ReplayMismatch::Authority));
    }
    let action_ids = transcript
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect::<Vec<_>>();
    let manifest_json = canonical_json(&manifest)?;
    let replayed = match Session::replay(&manifest_json, &action_ids) {
        Ok(session) => session,
        Err(SessionError::ReplayRejected(_)) => {
            return Ok(mismatch_report(ReplayMismatch::ReplayRejected));
        }
        Err(error) => return Err(error.into()),
    };
    let record = game_record_from_session(&replayed)?;
    Ok(compare_replay(&record, &manifest, &outcome, &events_hash))
}

fn mismatch_report(mismatch: ReplayMismatch) -> ArtifactReplayReport {
    ArtifactReplayReport {
        classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
        eligibility: evaluate_eligibility(EligibilityGates {
            coverage: false,
            design: false,
            execution: false,
            legality: false,
            pinned_input: false,
            replay: false,
            reporting: true,
        }),
        matched: false,
        mismatch: Some(mismatch),
        replay_verified: false,
        schema_version: 1,
    }
}

fn manifest_version_mismatch(manifest: &Value) -> Option<ReplayMismatch> {
    match manifest.get("engineVersion") {
        Some(Value::String(version)) if version == ENGINE_VERSION => {}
        _ => return Some(ReplayMismatch::EngineVersion),
    }
    match manifest.get("schemaVersion") {
        Some(Value::Number(version)) if version.as_u64() == Some(1) => None,
        _ => Some(ReplayMismatch::SchemaVersion),
    }
}

fn manifest_authority_hash(manifest: &Value) -> Option<IdentityHash> {
    IdentityHash::parse(
        manifest
            .get("authority")
            .and_then(Value::as_object)
            .and_then(|authority| authority.get("contentHash"))
            .and_then(Value::as_str)?,
    )
    .ok()
}

fn compare_replay(
    record: &GameRecord,
    manifest: &Value,
    outcome: &Value,
    events_file_hash: &IdentityHash,
) -> ArtifactReplayReport {
    let recorded_authority = manifest_authority_hash(manifest);
    let replayed_authority = manifest_authority_hash(&record.manifest);
    let mismatch = if recorded_authority != replayed_authority {
        Some(ReplayMismatch::Authority)
    } else if outcome.get("manifestId").and_then(Value::as_str) != Some(record.manifest_id.as_str())
    {
        Some(ReplayMismatch::ManifestIdentity)
    } else if outcome.get("transcriptHash").and_then(Value::as_str)
        != Some(record.transcript_hash.as_str())
    {
        Some(ReplayMismatch::TranscriptHash)
    } else if outcome.get("eventsHash").and_then(Value::as_str) != Some(record.events_hash.as_str())
        || events_file_hash != &record.events_hash
    {
        Some(ReplayMismatch::EventsHash)
    } else if outcome.get("finalStateHash").and_then(Value::as_str)
        != Some(record.final_state_hash.as_str())
    {
        Some(ReplayMismatch::FinalStateHash)
    } else {
        None
    };
    let matched = mismatch.is_none();
    ArtifactReplayReport {
        classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
        eligibility: if matched {
            record.eligibility.clone()
        } else {
            evaluate_eligibility(EligibilityGates {
                coverage: record.eligibility.gates.coverage,
                design: record.eligibility.gates.design,
                execution: record.eligibility.gates.execution,
                legality: record.eligibility.gates.legality,
                pinned_input: record.eligibility.gates.pinned_input,
                replay: false,
                reporting: true,
            })
        },
        matched,
        mismatch,
        replay_verified: matched,
        schema_version: 1,
    }
}

fn read_artifact_json(dir: &Path, name: &str) -> Result<Value, GameRecordError> {
    serde_json::from_str(&std::fs::read_to_string(dir.join(name))?).map_err(GameRecordError::from)
}

fn read_transcript(dir: &Path) -> Result<Vec<Receipt>, GameRecordError> {
    serde_json::from_str(&std::fs::read_to_string(dir.join("transcript.json"))?)
        .map_err(GameRecordError::from)
}

fn hash_events_file(dir: &Path) -> Result<IdentityHash, GameRecordError> {
    let text = std::fs::read_to_string(dir.join("events.jsonl"))?;
    let mut events = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        events.push(serde_json::from_str::<Value>(line)?);
    }
    identity_hash(&serde_json::to_value(events)?).map_err(GameRecordError::from)
}

/// Rejects empty artifact paths and parent-directory escapes.
///
/// # Errors
///
/// Returns [`GameRecordError::Invalid`] when the path is empty or contains `..`.
pub fn validate_artifacts_dir(dir: &Path) -> Result<(), GameRecordError> {
    if dir.as_os_str().is_empty() {
        return Err(GameRecordError::Invalid("artifacts directory is empty"));
    }
    if dir
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(GameRecordError::Invalid(
            "artifacts directory must not contain ..",
        ));
    }
    Ok(())
}

fn write_canonical_file(path: &Path, value: &Value) -> Result<(), GameRecordError> {
    let output = canonical_json(value)?;
    std::fs::write(path, format!("{output}\n"))?;
    Ok(())
}

/// Builds the SIM-03 record from one finished authoritative session.
///
/// # Errors
///
/// Returns [`GameRecordError`] when the session is unfinished, replay diverges,
/// or canonicalization fails.
pub fn game_record_from_session(session: &Session) -> Result<GameRecord, GameRecordError> {
    let (Some(outcome), Some(reason)) = (session.outcome(), session.terminal_reason()) else {
        return Err(GameRecordError::NonTerminal);
    };
    let Some(terminal) = FinishedTerminal::from_game(outcome, reason) else {
        return Err(GameRecordError::Invalid(
            "game terminal outcome and reason disagree",
        ));
    };
    let coverage = coverage_from_session(session)?;
    let events = flatten_events(session.transcript());
    let manifest: Value = serde_json::from_str(session.manifest_json())?;
    let eligibility = finished_game_eligibility(&manifest, &coverage, true);
    Ok(GameRecord {
        accepted_action_count: session.transcript().len(),
        classification: BatchClassification::UnrankedPartialRulesUnverifiedAuthority,
        coverage,
        eligibility,
        event_jsonl: event_jsonl(&events)?,
        events_hash: identity_hash(&serde_json::to_value(&events)?)?,
        fight_count: events
            .iter()
            .filter(|event| event.event_type == "fight-started")
            .count(),
        final_state_hash: session.state_hash()?,
        manifest,
        manifest_id: session.manifest_id().clone(),
        replay_verified: true,
        schema_version: 1,
        terminal,
        transcript: session.transcript().to_vec(),
        transcript_hash: session.transcript_hash()?,
        turn_count: session.turn_number(),
    })
}

fn finished_game_eligibility(
    manifest: &Value,
    coverage: &GameCoverage,
    replay_verified: bool,
) -> EligibilityReport {
    evaluate_eligibility(EligibilityGates {
        coverage: !coverage.offered_action_kinds.is_empty()
            || !coverage.committed_action_kinds.is_empty(),
        design: manifest
            .get("decks")
            .and_then(Value::as_object)
            .is_some_and(|decks| decks.contains_key("north") && decks.contains_key("south")),
        execution: true,
        legality: true,
        pinned_input: manifest.get("seed").is_some()
            && manifest.get("manifestId").is_some()
            && manifest.get("engineVersion").and_then(Value::as_str) == Some(ENGINE_VERSION)
            && manifest_authority_hash(manifest).is_some(),
        replay: replay_verified,
        reporting: true,
    })
}

/// Runs one policy-controlled game and writes its SIM-03 record.
///
/// # Errors
///
/// Returns [`GameRecordError`] when policy binding, rollout, replay, or
/// record construction fails.
pub fn record_policy_game(
    manifest_json: &str,
    north_deck_id: &IdentityHash,
    north_policy: &PolicySnapshot,
    south_deck_id: &IdentityHash,
    south_policy: &PolicySnapshot,
    max_actions: usize,
) -> Result<GameRecord, GameRecordError> {
    let game = Game::from_manifest_json(manifest_json).map_err(SimulatorError::from)?;
    for (policy, deck_id) in [(north_policy, north_deck_id), (south_policy, south_deck_id)] {
        policy
            .validate_binding(
                game.rules().authority_hash(),
                deck_id,
                game.rules().engine_version(),
            )
            .map_err(SimulatorError::from)?;
    }
    let rollout = run_game(game, north_policy, south_policy, max_actions)?;
    if rollout.outcome().is_none() {
        return Err(GameRecordError::NonTerminal);
    }
    game_record_from_session(&replay_selected(manifest_json, &rollout)?)
}

/// Records the public synthetic demo for one seed.
///
/// # Errors
///
/// Returns [`GameRecordError`] under the same conditions as [`record_policy_game`].
pub fn record_synthetic_demo(seed: u32) -> Result<GameRecord, GameRecordError> {
    let manifest_json = synthetic_demo_manifest_json(seed)?;
    let game = Game::from_manifest_json(&manifest_json).map_err(SimulatorError::from)?;
    let policy =
        baseline_policy_snapshot(game.rules().authority_hash(), game.rules().engine_version())
            .map_err(SimulatorError::from)?;
    let deck_id = IdentityHash::parse(BASELINE_POLICY_DECK_ID)
        .map_err(|_| GameRecordError::Invalid("baseline policy deckId is invalid"))?;
    record_policy_game(
        &manifest_json,
        &deck_id,
        &policy,
        &deck_id,
        &policy,
        MAX_GAME_ACTIONS,
    )
}

/// Formats flattened events as canonical JSONL with a trailing newline.
///
/// # Errors
///
/// Returns [`CanonicalError`] when an event cannot be canonicalized.
pub fn event_jsonl(events: &[Event]) -> Result<String, CanonicalError> {
    let mut lines = Vec::with_capacity(events.len());
    for event in events {
        let value = serde_json::to_value(event).map_err(CanonicalError::StringSerialization)?;
        lines.push(canonical_json(&value)?);
    }
    if lines.is_empty() {
        return Ok(String::new());
    }
    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn flatten_events(transcript: &[Receipt]) -> Vec<Event> {
    transcript
        .iter()
        .flat_map(|receipt| receipt.events.iter().cloned())
        .collect()
}

fn coverage_from_session(session: &Session) -> Result<GameCoverage, GameRecordError> {
    let mut offered_action_kinds = Vec::new();
    let mut seen_offered = BTreeSet::new();
    let mut committed_action_kinds = Vec::new();
    let mut seen_committed = BTreeSet::new();
    let mut committed_event_types = Vec::new();
    let mut seen_events = BTreeSet::new();
    let mut replay = Session::new(session.manifest_json())?;
    for receipt in session.transcript() {
        let mut found = false;
        for action in replay.legal_actions()? {
            let kind = descriptor_kind(&action.descriptor)?;
            if seen_offered.insert(kind.clone()) {
                offered_action_kinds.push(kind.clone());
            }
            if action.action_id == receipt.action_id {
                found = true;
                if seen_committed.insert(kind.clone()) {
                    committed_action_kinds.push(kind);
                }
            }
        }
        if !found {
            return Err(SimulatorError::ReplayDiverged.into());
        }
        match replay.step(ActionRequest {
            action_id: receipt.action_id.to_string(),
            seat: replay.decision_seat(),
            state_version: replay.state_version(),
        })? {
            StepResult::Accepted(stepped) if stepped.receipt_id == receipt.receipt_id => {}
            StepResult::Accepted(_) | StepResult::Rejected(_) => {
                return Err(SimulatorError::ReplayDiverged.into());
            }
        }
        for event in &receipt.events {
            if seen_events.insert(event.event_type.clone()) {
                committed_event_types.push(event.event_type.clone());
            }
        }
    }
    if replay.state_hash()? != session.state_hash()? {
        return Err(SimulatorError::ReplayDiverged.into());
    }
    Ok(GameCoverage {
        committed_action_kinds,
        committed_event_types,
        offered_action_kinds,
    })
}

fn descriptor_kind(descriptor: &Value) -> Result<String, GameRecordError> {
    match descriptor.get("kind") {
        Some(Value::String(kind)) => Ok(kind.clone()),
        _ => Err(SimulatorError::ReplayDiverged.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        GAME_ARTIFACT_FILES, ReplayMismatch, event_jsonl, game_record_from_session,
        record_synthetic_demo, replay_game_artifacts, validate_artifacts_dir, write_game_artifacts,
    };
    use crate::canonical::canonical_json;
    use crate::synthetic::synthetic_demo_session;

    #[test]
    fn opening_session_is_not_a_game_record() {
        let session = synthetic_demo_session(31);
        assert!(game_record_from_session(&session).is_err());
    }

    #[test]
    fn seed_31_record_writes_the_six_sim03_artifacts() {
        let record = record_synthetic_demo(31).expect("seed-31 record");
        let events: Vec<serde_json::Value> = record
            .event_jsonl
            .lines()
            .map(|line| serde_json::from_str(line).expect("event line"))
            .collect();
        let rebuilt = event_jsonl(
            &record
                .transcript
                .iter()
                .flat_map(|receipt| receipt.events.iter().cloned())
                .collect::<Vec<_>>(),
        )
        .expect("rebuilt JSONL");

        assert_eq!(record.schema_version, 1);
        assert!(record.replay_verified);
        assert!(!record.eligibility.ranked);
        assert!(record.eligibility.gates.all_passed());
        assert_eq!(
            record.eligibility.reasons,
            [
                crate::eligibility::EligibilityReason::PartialRules,
                crate::eligibility::EligibilityReason::UnverifiedAuthority
            ]
        );
        assert_eq!(record.accepted_action_count, 230);
        assert_eq!(record.fight_count, 6);
        assert_eq!(record.turn_count, 27);
        assert_eq!(
            record.final_state_hash.as_str(),
            "sha256:be86c59b046db97838faec73c34ccc8dd8b9d56587c04a3cd335be6c588ccc65"
        );
        assert_eq!(
            record.transcript_hash.as_str(),
            "sha256:fbdad70e092de2166ee9d853bae9300d45e9921c33a88865cb94147f4cd2ad47"
        );
        assert_eq!(
            record.manifest["manifestId"],
            serde_json::Value::String(record.manifest_id.as_str().to_owned())
        );
        assert_eq!(record.transcript.len(), 230);
        assert_eq!(
            events.len(),
            record
                .transcript
                .iter()
                .map(|receipt| receipt.events.len())
                .sum::<usize>()
        );
        assert_eq!(record.event_jsonl, rebuilt);
        assert!(
            record
                .coverage
                .committed_action_kinds
                .contains(&"summon-minion".to_owned())
        );
        assert!(
            record
                .coverage
                .committed_event_types
                .contains(&"fight-started".to_owned())
        );
        assert!(
            record
                .coverage
                .offered_action_kinds
                .contains(&"end-turn".to_owned())
        );
    }

    #[test]
    fn artifact_directory_rejects_parent_escape_and_writes_fixed_files() {
        assert!(validate_artifacts_dir("..".as_ref()).is_err());
        assert!(validate_artifacts_dir("games/../secret".as_ref()).is_err());
        assert!(validate_artifacts_dir("".as_ref()).is_err());

        let record = record_synthetic_demo(31).expect("seed-31 record");
        let dir =
            std::env::temp_dir().join(format!("sorcery-game-artifacts-{}-31", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write_game_artifacts(&dir, &record).expect("write artifacts");
        let names = fs::read_dir(&dir)
            .expect("artifact dir")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        for file in GAME_ARTIFACT_FILES {
            assert!(names.contains(&file.to_owned()), "{file}");
        }
        assert_eq!(
            fs::read_to_string(dir.join("events.jsonl")).expect("events"),
            record.event_jsonl
        );
        assert_eq!(
            fs::read_to_string(dir.join("manifest.json")).expect("manifest"),
            format!(
                "{}\n",
                canonical_json(&record.manifest).expect("canonical manifest")
            )
        );
        let outcome: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("outcome.json")).expect("outcome"))
                .expect("outcome JSON");
        assert_eq!(outcome["finalStateHash"], record.final_state_hash.as_str());
        assert_eq!(outcome["transcriptHash"], record.transcript_hash.as_str());
        assert_eq!(outcome["eventsHash"], record.events_hash.as_str());
        assert_eq!(
            outcome["classification"],
            "unranked_partial_rules_unverified_authority"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn seed_31_artifacts_replay_and_match() {
        let record = record_synthetic_demo(31).expect("seed-31 record");
        let dir =
            std::env::temp_dir().join(format!("sorcery-artifact-replay-{}-31", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write_game_artifacts(&dir, &record).expect("write artifacts");
        let report = replay_game_artifacts(&dir).expect("replay");
        assert!(report.matched);
        assert!(report.replay_verified);
        assert_eq!(report.mismatch, None);
        assert!(!report.eligibility.ranked);
        assert!(report.eligibility.gates.all_passed());
        assert_eq!(
            report.classification,
            crate::batch::BatchClassification::UnrankedPartialRulesUnverifiedAuthority
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn artifact_replay_classifies_engine_and_hash_mismatches() {
        let record = record_synthetic_demo(31).expect("seed-31 record");
        let dir = std::env::temp_dir().join(format!(
            "sorcery-artifact-mismatch-{}-31",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        write_game_artifacts(&dir, &record).expect("write artifacts");

        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("manifest.json")).expect("manifest"))
                .expect("manifest JSON");
        manifest["engineVersion"] = serde_json::json!("sorcery-core-v0");
        fs::write(
            dir.join("manifest.json"),
            format!("{}\n", canonical_json(&manifest).expect("canonical")),
        )
        .expect("write manifest");
        let engine = replay_game_artifacts(&dir).expect("engine mismatch");
        assert!(!engine.matched);
        assert_eq!(engine.mismatch, Some(ReplayMismatch::EngineVersion));

        write_game_artifacts(&dir, &record).expect("restore");
        let mut outcome: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("outcome.json")).expect("outcome"))
                .expect("outcome JSON");
        outcome["finalStateHash"] = serde_json::json!(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        fs::write(
            dir.join("outcome.json"),
            format!("{}\n", canonical_json(&outcome).expect("canonical")),
        )
        .expect("write outcome");
        let state = replay_game_artifacts(&dir).expect("state mismatch");
        assert!(!state.matched);
        assert_eq!(state.mismatch, Some(ReplayMismatch::FinalStateHash));
        let _ = fs::remove_dir_all(&dir);
    }
}
