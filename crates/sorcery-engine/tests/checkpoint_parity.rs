use serde::Deserialize;
use serde_json::Value;
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Seat};
use sorcery_engine::session::{Session, StepResult};

#[derive(Deserialize)]
struct Fixture {
    games: Vec<FixtureGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureGame {
    manifest_json: Option<String>,
    seed: u32,
}

fn manifest() -> String {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid checked-in TypeScript parity fixture");
    fixture
        .games
        .into_iter()
        .find(|game| game.seed == 31)
        .and_then(|game| game.manifest_json)
        .expect("seed-31 canonical manifest JSON")
}

#[test]
fn checkpoint_should_restore_accepted_and_rejected_attempts() {
    let mut session = Session::new(&manifest()).expect("valid session");
    let north = session.legal_actions().expect("north actions")[0].clone();
    assert!(matches!(
        session
            .step(ActionRequest {
                action_id: north.action_id.to_string(),
                seat: north.seat,
                state_version: north.state_version,
            })
            .expect("accepted north action"),
        StepResult::Accepted(_)
    ));
    assert!(matches!(
        session
            .step(ActionRequest {
                action_id: "unknown-action".to_owned(),
                seat: Seat::South,
                state_version: 1,
            })
            .expect("stable rejection"),
        StepResult::Rejected(_)
    ));
    let south = session.legal_actions().expect("south actions")[0].clone();
    session
        .step(ActionRequest {
            action_id: south.action_id.to_string(),
            seat: south.seat,
            state_version: south.state_version,
        })
        .expect("accepted south action");

    let checkpoint = create_game_checkpoint(&session).expect("captured checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("canonical checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");

    assert_eq!(restored.attempts(), session.attempts());
    assert_eq!(restored.transcript(), session.transcript());
    assert_eq!(
        restored.session_hash().expect("restored hash"),
        session.session_hash().expect("original hash")
    );
    assert!(restored.verify_replay().expect("verified replay"));
}

#[test]
fn checkpoint_should_reject_duplicates_and_tampered_history() {
    let session = Session::new(&manifest()).expect("valid session");
    let checkpoint = create_game_checkpoint(&session).expect("captured checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("canonical checkpoint");
    let duplicate = serialized.replacen(
        "\"kind\":\"sorcery-game-checkpoint\"",
        "\"kind\":\"sorcery-game-checkpoint\",\"kind\":\"changed\"",
        1,
    );
    assert!(parse_game_checkpoint(&duplicate).is_err());

    let mut value: Value = serde_json::from_str(&serialized).expect("checkpoint JSON");
    value["requests"] = serde_json::json!([{
        "actionId": "changed-action",
        "seat": "north",
        "stateVersion": 0,
    }]);
    let mut body = value.as_object().expect("checkpoint object").clone();
    body.remove("checkpointId");
    value["checkpointId"] = serde_json::to_value(
        identity_hash(&Value::Object(body)).expect("tampered checkpoint identity"),
    )
    .expect("hash JSON");
    let changed = parse_game_checkpoint(&serde_json::to_string(&value).expect("changed JSON"))
        .expect("self-consistent changed checkpoint");

    assert!(resume_game_checkpoint(&changed).is_err());
}
