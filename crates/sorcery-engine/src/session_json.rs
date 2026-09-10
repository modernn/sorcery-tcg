//! Line-delimited JSON-RPC boundary for authoritative [`Session`] control.

use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::canonical::{CanonicalError, canonical_json, parse_json_without_duplicate_keys};
use crate::checkpoint::{create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint};
use crate::contract::{ActionRequest, Seat};
use crate::novelty::NoveltyStep;
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
            "selectPolicyAction" => self.select_policy_action(request.id),
            "probeNovelty" => self.probe_novelty(request.id, &request.params),
            "runNoveltyRollout" => self.run_novelty_rollout(request.id, &request.params),
            "runCounterfactual" => self.run_counterfactual(request.id, &request.params),
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

    fn select_policy_action(&self, id: u64) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        match session.select_baseline_policy_action() {
            Ok(action) => ok_response(id, json!({ "action": action })),
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn probe_novelty(&self, id: u64, params: &Value) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        let committed_action_kinds = match string_list(params, "committedActionKinds") {
            Ok(value) => value,
            Err(message) => return error_response(id, &message),
        };
        let committed_event_types = match string_list(params, "committedEventTypes") {
            Ok(value) => value,
            Err(message) => return error_response(id, &message),
        };
        match session.probe_novelty(&committed_action_kinds, &committed_event_types) {
            Ok(step) => ok_response(id, novelty_step_value(&step)),
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn run_novelty_rollout(&self, id: u64, params: &Value) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        let Some(max_actions) = params.get("maxActions").and_then(Value::as_u64) else {
            return error_response(id, "runNoveltyRollout requires maxActions");
        };
        let Ok(max_actions) = usize::try_from(max_actions) else {
            return error_response(id, "runNoveltyRollout maxActions is out of range");
        };
        match session.run_novelty_rollout(max_actions) {
            Ok(output) => match serde_json::to_value(output.emitted_checkpoints()) {
                Ok(emitted_checkpoints) => ok_response(
                    id,
                    json!({
                        "emittedCheckpoints": emitted_checkpoints,
                        "result": output.result(),
                    }),
                ),
                Err(error) => error_response(id, &error.to_string()),
            },
            Err(error) => error_response(id, &error.to_string()),
        }
    }

    fn run_counterfactual(&self, id: u64, params: &Value) -> RpcResponse {
        let Some(session) = &self.session else {
            return error_response(id, "session-json process has no active session");
        };
        let Some(max_continuation) = params
            .get("maxContinuationDecisions")
            .and_then(Value::as_u64)
        else {
            return error_response(id, "runCounterfactual requires maxContinuationDecisions");
        };
        let Ok(max_continuation) = usize::try_from(max_continuation) else {
            return error_response(
                id,
                "runCounterfactual maxContinuationDecisions is out of range",
            );
        };
        match session.run_counterfactual(max_continuation) {
            Ok(report) => ok_response(id, json!({ "result": report.result() })),
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
        "poweredUnitInstanceIds": observation.powered_unit_instance_ids(),
        "seat": match observation.seat() {
            Seat::North => "north",
            Seat::South => "south",
        },
        "spellbookRemaining": observation.spellbook_remaining(),
    })
}

fn string_list(params: &Value, key: &str) -> Result<Vec<String>, String> {
    let Some(value) = params.get(key) else {
        return Err(format!("probeNovelty requires {key}"));
    };
    let Some(items) = value.as_array() else {
        return Err(format!("probeNovelty {key} must be an array of strings"));
    };
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("probeNovelty {key} must be an array of strings"))
        })
        .collect()
}

fn novelty_step_value(step: &NoveltyStep) -> Value {
    json!({
        "probes": step
            .probes()
            .iter()
            .map(|probe| {
                json!({
                    "actionId": probe.action_id(),
                    "actionKind": probe.action_kind(),
                    "eventTypes": probe.event_types(),
                    "newActionKind": probe.new_action_kind(),
                    "newEventCount": probe.new_event_count(),
                    "postStateHash": probe.post_state_hash(),
                    "selectedByFallback": probe.selected_by_fallback(),
                })
            })
            .collect::<Vec<_>>(),
        "selectedIndex": step.selected_index(),
        "tooWide": step.too_wide(),
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
    fn service_should_select_the_baseline_policy_action() {
        let manifest = synthetic_demo_manifest_json(31).expect("manifest");
        let mut service = SessionJsonService::new();
        assert!(
            service
                .handle(&rpc(1, "new", json!({ "manifestJson": manifest })))
                .error
                .is_none()
        );
        let selected = service.handle(&rpc(2, "selectPolicyAction", json!({})));
        let result = selected.result.expect("selected action");
        let action = result["action"].as_object().expect("action object");
        assert_eq!(action["descriptor"]["kind"], "mulligan");
        assert_eq!(action["descriptor"]["atlasOrder"], json!([]));
        assert_eq!(action["descriptor"]["spellbookOrder"], json!([]));
        assert_eq!(action["seat"], "north");
    }

    #[test]
    fn service_should_probe_one_step_novelty() {
        let manifest = synthetic_demo_manifest_json(31).expect("manifest");
        let mut service = SessionJsonService::new();
        assert!(
            service
                .handle(&rpc(1, "new", json!({ "manifestJson": manifest })))
                .error
                .is_none()
        );
        let selected = service.handle(&rpc(2, "selectPolicyAction", json!({})));
        let action = selected.result.expect("selected action")["action"].clone();
        let novelty = service.handle(&rpc(
            3,
            "probeNovelty",
            json!({
                "committedActionKinds": [],
                "committedEventTypes": [],
            }),
        ));
        let result = novelty.result.expect("novelty result");
        let probes = result["probes"].as_array().expect("probes");
        assert!(!probes.is_empty());
        assert_eq!(result["tooWide"], false);
        let selected_index =
            usize::try_from(result["selectedIndex"].as_u64().expect("selectedIndex"))
                .expect("selectedIndex fits usize");
        assert!(selected_index < probes.len());
        assert_eq!(
            probes
                .iter()
                .filter(|probe| probe["selectedByFallback"] == true)
                .count(),
            1
        );
        assert!(
            probes
                .iter()
                .any(|probe| probe["actionId"] == action["actionId"]
                    && probe["selectedByFallback"] == true)
        );
    }

    #[test]
    fn service_should_run_a_zero_horizon_novelty_rollout() {
        let manifest = synthetic_demo_manifest_json(31).expect("manifest");
        let mut service = SessionJsonService::new();
        assert!(
            service
                .handle(&rpc(1, "new", json!({ "manifestJson": manifest })))
                .error
                .is_none()
        );
        let rollout = service.handle(&rpc(2, "runNoveltyRollout", json!({ "maxActions": 0 })));
        let result = rollout.result.expect("rollout result");
        assert_eq!(result["result"]["status"], "horizon");
        assert_eq!(result["result"]["acceptedActionCount"], 0);
        assert_eq!(
            result["emittedCheckpoints"]
                .as_array()
                .expect("checkpoints")
                .len(),
            1
        );
    }

    #[test]
    fn service_should_run_opening_counterfactual() {
        let manifest = synthetic_demo_manifest_json(31).expect("manifest");
        let mut service = SessionJsonService::new();
        assert!(
            service
                .handle(&rpc(1, "new", json!({ "manifestJson": manifest })))
                .error
                .is_none()
        );
        let report = service.handle(&rpc(
            2,
            "runCounterfactual",
            json!({ "maxContinuationDecisions": 0 }),
        ));
        let result = report.result.expect("counterfactual result")["result"].clone();
        assert!(result["status"] == "complete" || result["status"] == "too-wide");
        assert_eq!(result["policyVersion"], "deterministic-demo-v1");
        assert_eq!(result["rootActionLimit"], 128);
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
