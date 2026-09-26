use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Seat};
use sorcery_engine::game::GameError;
use sorcery_engine::game_record::game_record_from_session;
use sorcery_engine::session::{Session, SessionError, StepResult};
use sorcery_engine::session_json::{RpcRequest, SessionJsonService};

const UNSUPPORTED: &str = "ordering Ward and other damage prevention";

fn request_where(session: &Session, predicate: impl Fn(&Value) -> bool) -> ActionRequest {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected issued action");
    ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    }
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) {
    let request = request_where(session, predicate);
    assert!(matches!(session.step(request), Ok(StepResult::Accepted(_))));
}

fn warded_reduction_session() -> Session {
    let avatar = json!({
        "attack": 1, "defense": 1, "cardType": "avatar", "drawSpell": false, "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let threshold = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "session-unsupported" })).unwrap(),
            "mode": "synthetic",
            "revisionId": "synthetic-session-unsupported-v1",
        },
        "cards": {
            "north-avatar": avatar, "south-avatar": avatar,
            "north-site": site, "south-site": site,
            "north-minion": {
                "attack": 4, "defense": 4, "cardType": "minion", "manaCost": 0,
                "takesLessDamage": 1, "thresholds": threshold,
            },
            "north-ward": {
                "cardType": "magic", "manaCost": 0, "grantWardToTargetMinion": true,
                "thresholds": threshold,
            },
            "north-rain": {
                "cardType": "magic", "manaCost": 1, "damageEachAbovegroundMinion": 1,
                "thresholds": threshold,
            },
            "south-minion": {
                "attack": 1, "defense": 1, "cardType": "minion", "manaCost": 0,
                "thresholds": threshold,
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 5], "avatar": "north-avatar",
                "spellbook": ["north-minion", "north-ward", "north-rain"],
            },
            "south": {
                "atlas": vec!["south-site"; 5], "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 5],
            },
        },
        "engineVersion": "sorcery-core-v1", "firstSeat": "north",
        "schemaVersion": 1, "seed": 31,
    });
    manifest["manifestId"] = json!(identity_hash(&manifest).unwrap());
    let mut session = Session::new(&canonical_json(&manifest).unwrap()).unwrap();
    for _ in 0..2 {
        accept_where(&mut session, |action| {
            action["kind"] == "mulligan"
                && action["atlasOrder"] == json!([])
                && action["spellbookOrder"] == json!([])
        });
    }
    accept_where(&mut session, |action| {
        action["kind"] == "play-site" && action["cell"] == "C4"
    });
    accept_where(&mut session, |action| action["kind"] == "summon-minion");
    accept_where(&mut session, |action| {
        action["kind"] == "cast-magic" && action["cardId"] == "north-ward"
    });
    session
}

fn unsupported<T>(result: Result<T, SessionError>) {
    assert!(matches!(
        result,
        Err(SessionError::Game(GameError::UnsupportedMechanic(reason))) if reason == UNSUPPORTED
    ));
}

#[test]
fn unsupported_transition_preserves_state_but_aborts_session_and_certification() {
    let mut session = warded_reduction_session();
    let checkpoint = create_game_checkpoint(&session).unwrap();
    let before = session.replay_value().unwrap();
    let view = session.public_view(Seat::North).unwrap();
    let state_hash = session.state_hash().unwrap();
    let transcript = session.transcript().to_vec();
    let attempts = session.attempts().to_vec();
    let rain = request_where(&session, |action| action["cardId"] == "north-rain");
    let end_turn = request_where(&session, |action| action["kind"] == "end-turn");

    unsupported(session.step(rain));
    assert_eq!(session.unsupported_mechanic(), Some(UNSUPPORTED));
    assert_eq!(session.public_view(Seat::North).unwrap(), view);
    assert_eq!(session.state_hash().unwrap(), state_hash);
    assert_eq!(session.transcript(), transcript);
    assert_eq!(session.attempts(), attempts);
    unsupported(session.step(end_turn.clone()));
    unsupported(session.clone().step(end_turn.clone()));
    unsupported(session.legal_actions());
    unsupported(session.verify_replay());
    unsupported(session.replay_value());
    unsupported(session.session_hash());
    unsupported(session.transcript_hash());
    assert!(
        create_game_checkpoint(&session)
            .unwrap_err()
            .to_string()
            .contains(UNSUPPORTED)
    );
    assert!(
        game_record_from_session(&session)
            .unwrap_err()
            .to_string()
            .contains(UNSUPPORTED)
    );
    assert_eq!(session.attempts(), attempts);
    assert_eq!(session.transcript(), transcript);

    let mut branch = resume_game_checkpoint(&checkpoint).unwrap();
    assert_eq!(branch.unsupported_mechanic(), None);
    assert_eq!(branch.replay_value().unwrap(), before);
    assert!(matches!(branch.step(end_turn), Ok(StepResult::Accepted(_))));
}

fn rpc(service: &mut SessionJsonService, method: &str, params: &Value) -> Value {
    let request: RpcRequest = serde_json::from_value(json!({
        "schemaVersion": 1, "id": 1, "method": method, "params": params,
    }))
    .unwrap();
    serde_json::to_value(service.handle(&request)).unwrap()
}

#[test]
fn session_json_cannot_continue_or_export_after_unsupported_transition() {
    let session = warded_reduction_session();
    let checkpoint = create_game_checkpoint(&session).unwrap();
    let rain = request_where(&session, |action| action["cardId"] == "north-rain");
    let end_turn = request_where(&session, |action| action["kind"] == "end-turn");
    let mut service = SessionJsonService::new();
    let resume = json!({ "checkpoint": checkpoint });
    assert!(rpc(&mut service, "resume", &resume)["error"].is_null());
    let view = rpc(&mut service, "publicView", &json!({ "seat": "north" }));
    let failed = rpc(&mut service, "step", &json!(rain));
    assert!(
        failed["error"]["message"]
            .as_str()
            .unwrap()
            .contains(UNSUPPORTED)
    );
    let continued = rpc(&mut service, "step", &json!(end_turn));
    assert_eq!(continued["error"], failed["error"]);
    for method in [
        "checkpoint",
        "exportSession",
        "exportGameRecord",
        "verifyReplay",
        "replaySteps",
    ] {
        assert_eq!(
            rpc(&mut service, method, &json!({}))["error"],
            failed["error"]
        );
    }
    assert_eq!(
        rpc(&mut service, "publicView", &json!({ "seat": "north" })),
        view
    );
    assert!(rpc(&mut service, "resume", &resume)["error"].is_null());
    assert!(rpc(&mut service, "step", &json!(end_turn))["error"].is_null());
}
