//! Line-delimited JSON-RPC boundary for authoritative [`Session`] control.

use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::canonical::{CanonicalError, canonical_json, parse_json_without_duplicate_keys};
use crate::checkpoint::{create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint};
use crate::contract::{ActionRequest, Seat};
use crate::session::{Session, StepResult};

const SCHEMA_VERSION: u8 = 1;
const MAX_LINE_BYTES: usize = 16 * 1024 * 1024;

/// One inbound RPC request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RpcRequest {
    id: u64,
    method: String,
    params: Value,
    schema_version: u8,
}

/// One outbound RPC response.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcResponse {
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    schema_version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

/// One outbound RPC failure.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RpcError {
    message: String,
}

/// Stateful handler for one persistent session-json process.
#[derive(Default)]
pub struct SessionJsonService {
    session: Option<Session>,
}

impl SessionJsonService {
    /// Creates an empty service.
    #[must_use]
    pub const fn new() -> Self {
        Self { session: None }
    }

    /// Handles one RPC request against the current session state.
    pub fn handle(&mut self, request: &RpcRequest) -> RpcResponse {
        if request.schema_version != SCHEMA_VERSION {
            return error_response(request.id, "session-json schemaVersion must be 1");
        }
        match request.method.as_str() {
            "new" => self.new_session(request.id, &request.params),
            "legalActions" => self.legal_actions(request.id, &request.params),
            "step" => self.step(request.id, &request.params),
            "observe" => self.observe(request.id, &request.params),
            "publicView" => self.public_view(request.id, &request.params),
            "verifyReplay" => self.verify_replay(request.id),
            "exportSession" => self.export_session(request.id),
            "checkpoint" => self.checkpoint(request.id),
            "resume" => self.resume(request.id, &request.params),
            _ => error_response(request.id, "session-json method is unsupported"),
        }
    }

    /// Reads newline-delimited requests from `input` and writes responses to `output`.
    ///
    /// # Errors
    ///
    /// Returns I/O or JSON errors. Operational failures are encoded as RPC errors.
    pub fn serve(
        &mut self,
        input: impl BufRead,
        mut output: impl Write,
    ) -> Result<(), SessionJsonError> {
        for line in input.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            if line.len() > MAX_LINE_BYTES {
                return Err(SessionJsonError::LineTooLarge);
            }
            let request: RpcRequest =
                match parse_json_without_duplicate_keys(&line).and_then(serde_json::from_value) {
                    Ok(request) => request,
                    Err(error) => {
                        write_response(
                            &mut output,
                            error_response(0, &format!("invalid session-json request: {error}")),
                        )?;
                        continue;
                    }
                };
            let response = self.handle(&request);
            write_response(&mut output, response)?;
        }
        Ok(())
    }

    fn new_session(&mut self, id: u64, params: &Value) -> RpcResponse {
        let Some(manifest_json) = params.get("manifestJson").and_then(Value::as_str) else {
            return error_response(id, "new requires manifestJson");
        };
        if manifest_json.len() > MAX_LINE_BYTES {
            return error_response(id, "manifestJson exceeds 16 MiB");
        }
        match Session::new(manifest_json) {
            Ok(session) => {
                let state_hash = match session.state_hash() {
                    Ok(state_hash) => state_hash,
                    Err(error) => return error_response(id, &error.to_string()),
                };
                let state_version = session.state_version();
                let manifest_id = session.manifest_id().clone();
                self.session = Some(session);
                ok_response(
                    id,
                    json!({
                        "manifestId": manifest_id,
                        "stateHash": state_hash,
                        "stateVersion": state_version,
                    }),
                )
            }
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn legal_actions(&self, id: u64, params: &Value) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        let Ok(seat) = parse_seat_param(params, id) else {
            return error_response(id, "legalActions requires seat north or south");
        };
        if seat != session.decision_seat() {
            return ok_response(id, json!({ "actions": [] }));
        }
        match session.legal_actions() {
            Ok(actions) => ok_response(id, json!({ "actions": actions })),
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn step(&mut self, id: u64, params: &Value) -> RpcResponse {
        let Some(session) = self.session.as_mut() else {
            return error_response(id, "session-json process has no active session");
        };
        let Ok(request) = serde_json::from_value::<ActionRequest>(params.clone()) else {
            return error_response(id, "step params must be a valid ActionRequest");
        };
        match session.step(request) {
            Ok(result) => ok_response(id, step_result_value(result)),
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn observe(&self, id: u64, params: &Value) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        let Ok(seat) = parse_seat_param(params, id) else {
            return error_response(id, "observe requires seat north or south");
        };
        ok_response(
            id,
            json!({ "observation": observation_value(&session.observe(seat)) }),
        )
    }

    fn public_view(&self, id: u64, params: &Value) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        let Ok(seat) = parse_seat_param(params, id) else {
            return error_response(id, "publicView requires seat north or south");
        };
        match session.public_view(seat) {
            Ok(view) => {
                let state_hash = match session.state_hash() {
                    Ok(state_hash) => state_hash,
                    Err(error) => return error_response(id, &error.to_string()),
                };
                ok_response(
                    id,
                    json!({
                        "stateHash": state_hash,
                        "view": view,
                    }),
                )
            }
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn verify_replay(&self, id: u64) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        match session.verify_replay() {
            Ok(verified) => ok_response(id, json!({ "verified": verified })),
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn export_session(&self, id: u64) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        match session.replay_value() {
            Ok(replay) => match serde_json::from_str::<Value>(session.manifest_json()) {
                Ok(manifest) => ok_response(
                    id,
                    json!({
                        "attempts": session.attempts(),
                        "initialRandomDraws": replay["initialRandomDraws"].clone(),
                        "manifest": manifest,
                        "state": replay["state"].clone(),
                        "transcript": session.transcript(),
                    }),
                ),
                Err(error) => error_response(id, &error.to_string()),
            },
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn checkpoint(&self, id: u64) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        match create_game_checkpoint(session) {
            Ok(checkpoint) => match serde_json::to_value(checkpoint) {
                Ok(value) => ok_response(id, json!({ "checkpoint": value })),
                Err(error) => error_response(id, &error.to_string()),
            },
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn resume(&mut self, id: u64, params: &Value) -> RpcResponse {
        let Some(checkpoint_value) = params.get("checkpoint") else {
            return error_response(id, "resume requires checkpoint");
        };
        let checkpoint_text = match canonical_json(checkpoint_value) {
            Ok(text) => text,
            Err(error) => return error_response(id, &error.to_string()),
        };
        let checkpoint = match parse_game_checkpoint(&checkpoint_text) {
            Ok(checkpoint) => checkpoint,
            Err(error) => return error_response(id, &error.to_string()),
        };
        match resume_game_checkpoint(&checkpoint) {
            Ok(session) => {
                let state_hash = match session.state_hash() {
                    Ok(state_hash) => state_hash,
                    Err(error) => return error_response(id, &error.to_string()),
                };
                let state_version = session.state_version();
                let expected_session_hash = checkpoint.expected_session_hash.clone();
                self.session = Some(session);
                ok_response(
                    id,
                    json!({
                        "expectedSessionHash": expected_session_hash,
                        "stateHash": state_hash,
                        "stateVersion": state_version,
                    }),
                )
            }
            Err(error) => error_response(id, &error.to_string()),
        }
    }
}

/// Session-json transport or parsing failed.
#[derive(Debug)]
pub enum SessionJsonError {
    /// One request or response line exceeded the size bound.
    LineTooLarge,
    /// Canonical JSON encoding failed.
    Canonical(CanonicalError),
    /// Underlying I/O failed.
    Io(io::Error),
    /// JSON decoding failed.
    Json(serde_json::Error),
}

impl From<CanonicalError> for SessionJsonError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<io::Error> for SessionJsonError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for SessionJsonError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl std::fmt::Display for SessionJsonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LineTooLarge => formatter.write_str("session-json line exceeds 16 MiB"),
            Self::Canonical(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SessionJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LineTooLarge => None,
            Self::Canonical(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}

fn observation_value(observation: &crate::game::SeatObservation) -> Value {
    let enemy_avatar = observation.enemy_avatar();
    json!({
        "atlasRemaining": observation.atlas_remaining(),
        "enemyAvatar": {
            "cell": enemy_avatar.cell.to_string(),
            "region": enemy_avatar.region,
        },
        "seat": match observation.seat() {
            Seat::North => "north",
            Seat::South => "south",
        },
        "spellbookRemaining": observation.spellbook_remaining(),
    })
}

fn parse_seat_param(params: &Value, id: u64) -> Result<Seat, ()> {
    let _ = id;
    match params.get("seat").and_then(Value::as_str) {
        Some("north") => Ok(Seat::North),
        Some("south") => Ok(Seat::South),
        _ => Err(()),
    }
}

fn step_result_value(result: StepResult) -> Value {
    match result {
        StepResult::Accepted(receipt) => json!({
            "accepted": true,
            "receipt": receipt,
        }),
        StepResult::Rejected(rejection) => json!({
            "accepted": false,
            "rejection": rejection,
        }),
    }
}

fn ok_response(id: u64, result: Value) -> RpcResponse {
    RpcResponse {
        id,
        result: Some(result),
        schema_version: SCHEMA_VERSION,
        error: None,
    }
}

fn error_response(id: u64, message: &str) -> RpcResponse {
    RpcResponse {
        id,
        result: None,
        schema_version: SCHEMA_VERSION,
        error: Some(RpcError {
            message: message.to_owned(),
        }),
    }
}

fn write_response(output: &mut impl Write, response: RpcResponse) -> Result<(), SessionJsonError> {
    let line = canonical_json(&serde_json::to_value(response)?)?;
    writeln!(output, "{line}")?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use super::*;
    use crate::synthetic::synthetic_demo_manifest_json;

    fn rpc(id: u64, method: &str, params: Value) -> RpcRequest {
        RpcRequest {
            id,
            method: method.to_owned(),
            params,
            schema_version: SCHEMA_VERSION,
        }
    }

    #[test]
    fn service_should_create_step_and_export_a_session() {
        let manifest = synthetic_demo_manifest_json(31).expect("manifest");
        let mut service = SessionJsonService::new();
        let created = service.handle(&rpc(1, "new", json!({ "manifestJson": manifest })));
        assert!(created.error.is_none());
        let actions = service.handle(&rpc(2, "legalActions", json!({ "seat": "north" })));
        let actions_result = actions.result.expect("actions result");
        let actions = actions_result["actions"].as_array().expect("actions array");
        assert!(!actions.is_empty());
        let first = &actions[0];
        let stepped = service.handle(&rpc(
            3,
            "step",
            json!({
                "actionId": first["actionId"],
                "seat": first["seat"],
                "stateVersion": first["stateVersion"],
            }),
        ));
        assert_eq!(stepped.result.expect("step result")["accepted"], true);
        let exported = service.handle(&rpc(4, "exportSession", json!({})));
        let exported = exported.result.expect("export result");
        assert!(exported.get("manifest").is_some());
        assert!(exported.get("state").is_some());
        assert!(exported.get("transcript").is_some());
        let view = service.handle(&rpc(5, "publicView", json!({ "seat": "north" })));
        let view = view.result.expect("public view");
        assert_eq!(view["view"]["viewer"], "north");
        assert!(view["view"]["players"]["north"]["hand"]["atlas"].is_array());
        assert!(view["view"]["players"]["south"]["hand"]["atlas"].is_number());
        assert!(view["stateHash"].as_str().is_some());
    }

    #[test]
    fn serve_should_round_trip_one_line_delimited_exchange() {
        let manifest = synthetic_demo_manifest_json(31).expect("manifest");
        let request = canonical_json(&json!({
            "schemaVersion": 1,
            "id": 1,
            "method": "new",
            "params": { "manifestJson": manifest },
        }))
        .expect("canonical request");
        let input = Cursor::new(format!("{request}\n"));
        let mut output = Vec::new();
        SessionJsonService::new()
            .serve(BufReader::new(input), &mut output)
            .expect("serve");
        let response_line = std::str::from_utf8(&output)
            .expect("utf8")
            .lines()
            .next()
            .expect("one response");
        let response: RpcResponse = serde_json::from_str(response_line).expect("response json");
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}
