use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, LegalAction, Seat};
use sorcery_engine::session::{SESSION_MAX_ATTEMPTS, Session, SessionError, StepResult};
use sorcery_engine::session_json::{RpcRequest, SessionJsonService};
use sorcery_engine::synthetic::{selfplay_manifest_with, synthetic_demo_manifest_json};

fn fresh() -> Session {
    Session::new(&synthetic_demo_manifest_json(31).unwrap()).unwrap()
}

fn request(action: &LegalAction) -> ActionRequest {
    ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    }
}

fn stale() -> ActionRequest {
    ActionRequest {
        action_id: "synthetic-unknown".to_owned(),
        seat: Seat::North,
        state_version: 9_007_199_254_740_991,
    }
}

fn fingerprint(session: &Session) -> Value {
    json!({
        "hash": session.session_hash().unwrap(),
        "replay": session.replay_value().unwrap(),
        "attempts": session.attempts(),
        "actions": session.legal_actions().unwrap(),
    })
}

fn round_trip(session: &Session) -> Session {
    let text = serialize_game_checkpoint(&create_game_checkpoint(session).unwrap()).unwrap();
    let parsed = parse_game_checkpoint(&text).unwrap();
    assert_eq!(serialize_game_checkpoint(&parsed).unwrap(), text);
    let resumed = resume_game_checkpoint(&parsed).unwrap();
    assert_eq!(fingerprint(&resumed), fingerprint(session));
    assert!(resumed.verify_replay().unwrap());
    resumed
}

#[test]
fn malformed_requests_leave_complete_session_unchanged_and_recoverable() {
    for (action_id, state_version) in [
        (String::new(), 0),
        ("x".repeat(257), 0),
        ("🦄".repeat(129), 0),
        ("x".to_owned(), 9_007_199_254_740_992),
        ("x".to_owned(), u64::MAX),
    ] {
        let mut session = fresh();
        let before = fingerprint(&session);
        assert!(matches!(
            session.step(ActionRequest {
                action_id,
                seat: Seat::North,
                state_version,
            }),
            Err(SessionError::InvalidRequest(_))
        ));
        assert_eq!(fingerprint(&session), before);
        round_trip(&session);
        let action = session.legal_actions().unwrap().remove(0);
        assert!(matches!(
            session.step(request(&action)),
            Ok(StepResult::Accepted(_))
        ));
        round_trip(&session);
    }
    let mut session = fresh();
    for action_id in ["x".repeat(256), "🦄".repeat(128)] {
        assert!(matches!(
            session.step(ActionRequest {
                action_id,
                ..stale()
            }),
            Ok(StepResult::Rejected(_))
        ));
    }
    assert_eq!(session.attempts().len(), 2);
    assert_eq!(session.attempts()[1].attempt_sequence, 2);
    round_trip(&session);
}

fn assert_full(session: &Session) {
    assert_eq!(session.attempts().len(), SESSION_MAX_ATTEMPTS);
    let before = fingerprint(session);
    let mut resumed = round_trip(session);
    let legal = request(&resumed.legal_actions().unwrap().remove(0));
    for next in [stale(), legal.clone(), stale(), legal] {
        assert!(matches!(
            resumed.step(next),
            Err(SessionError::AttemptLimitReached)
        ));
        assert_eq!(fingerprint(&resumed), before);
    }
    round_trip(&resumed);
}

#[test]
fn rejected_and_mixed_histories_stop_exactly_at_v1_capacity() {
    for accept_last in [false, true] {
        let mut session = fresh();
        for _ in 0..SESSION_MAX_ATTEMPTS - 1 {
            assert!(matches!(session.step(stale()), Ok(StepResult::Rejected(_))));
        }
        let next = if accept_last {
            request(&session.legal_actions().unwrap().remove(0))
        } else {
            stale()
        };
        let result = session.step(next).unwrap();
        assert_eq!(matches!(result, StepResult::Accepted(_)), accept_last);
        assert_eq!(session.transcript().len(), usize::from(accept_last));
        assert_full(&session);
    }
}

#[test]
fn accepted_history_stops_exactly_at_v1_capacity() {
    let manifest = selfplay_manifest_with(31, |manifest| {
        for seat in ["north", "south"] {
            for zone in ["atlas", "spellbook"] {
                let cards = manifest["decks"][seat][zone].as_array().unwrap().clone();
                manifest["decks"][seat][zone] =
                    json!(cards.iter().cycle().take(200).collect::<Vec<_>>());
            }
        }
    });
    let mut session = Session::new(&manifest).unwrap();
    for _ in 0..SESSION_MAX_ATTEMPTS {
        let actions = session.legal_actions().unwrap();
        let view = session.public_view(Seat::North).unwrap();
        let action = actions
            .iter()
            .find(|action| {
                action.descriptor["kind"] == "mulligan"
                    && action.descriptor["atlasOrder"] == json!([])
                    && action.descriptor["spellbookOrder"] == json!([])
            })
            .or_else(|| {
                actions
                    .iter()
                    .find(|action| action.descriptor["kind"] == "end-turn")
            })
            .or_else(|| {
                actions.iter().find(|action| {
                    let seat = if action.seat == Seat::North {
                        "north"
                    } else {
                        "south"
                    };
                    let zone = if view["players"][seat]["atlasCount"] == 0 {
                        "spellbook"
                    } else {
                        "atlas"
                    };
                    action.descriptor["kind"] == "draw" && action.descriptor["zone"] == zone
                })
            })
            .or_else(|| actions.first())
            .expect("a live game action");
        assert!(matches!(
            session.step(request(action)),
            Ok(StepResult::Accepted(_))
        ));
    }
    assert_eq!(session.transcript().len(), SESSION_MAX_ATTEMPTS);
    assert_full(&session);
}

fn rpc(service: &mut SessionJsonService, method: &str, params: &Value) -> Value {
    let request: RpcRequest = serde_json::from_value(json!({
        "id": 1, "method": method, "params": params, "schemaVersion": 1,
    }))
    .unwrap();
    serde_json::to_value(service.handle(&request)).unwrap()
}

#[test]
fn unknown_fields_are_rejected_in_nested_checkpoints_attempts_and_raw_rpc() {
    let mut session = fresh();
    session.step(stale()).unwrap();
    let checkpoint = create_game_checkpoint(&session).unwrap();
    let mut injected = serde_json::to_value(&checkpoint).unwrap();
    injected["requests"][0]["injected"] = json!(true);
    assert!(
        parse_game_checkpoint(&injected.to_string())
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );
    let mut attempt = serde_json::to_value(&session.attempts()[0]).unwrap();
    attempt["request"]["injected"] = json!(true);
    assert!(serde_json::from_value::<sorcery_engine::contract::Attempt>(attempt).is_err());

    let mut service = SessionJsonService::new();
    assert!(
        rpc(
            &mut service,
            "new",
            &json!({"manifestJson": session.manifest_json()})
        )["error"]
            .is_null()
    );
    let before = rpc(&mut service, "exportSession", &json!({}))["result"].clone();
    let action = fresh().legal_actions().unwrap().remove(0);
    for params in [
        json!({"actionId": "x", "seat": "north", "stateVersion": 0, "injected": true}),
        serde_json::to_value(&action).unwrap(),
        json!({"actionId": "", "seat": "north", "stateVersion": 0}),
    ] {
        assert!(rpc(&mut service, "step", &params)["error"]["message"].is_string());
        assert_eq!(
            rpc(&mut service, "exportSession", &json!({}))["result"],
            before
        );
        assert!(rpc(&mut service, "checkpoint", &json!({}))["error"].is_null());
    }
    assert_eq!(
        rpc(
            &mut service,
            "step",
            &serde_json::to_value(request(&action)).unwrap()
        )["result"]["accepted"],
        true
    );
}

#[test]
fn valid_v1_checkpoint_remains_byte_identical_to_pre_fix_fixture() {
    let mut session = fresh();
    session
        .step(ActionRequest {
            state_version: 9,
            ..stale()
        })
        .unwrap();
    let checkpoint = create_game_checkpoint(&session).unwrap();
    assert_eq!(
        checkpoint.checkpoint_id.as_str(),
        "sha256:7bdcd6d479c017e19b7f3ec7ec36f11ed5fd70ebf5c9151f15525c3a75c5b5b6"
    );
    assert_eq!(
        checkpoint.expected_session_hash.as_str(),
        "sha256:4c738088d1ff935385ca84a249846393a3389eb6eec13019183c9a673999ffb3"
    );
    let text = serialize_game_checkpoint(&checkpoint).unwrap();
    assert_eq!(
        identity_hash(&serde_json::from_str::<Value>(&text).unwrap())
            .unwrap()
            .as_str(),
        "sha256:10fbc1a4c438652369cb7add0565a11b80a4a7555dfafa199fc868d1b6406411"
    );
    round_trip(&session);
}
