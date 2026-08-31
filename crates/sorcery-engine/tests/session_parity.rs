use serde::Deserialize;
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, RejectionCode, Seat};
use sorcery_engine::session::{Session, StepResult};

#[derive(Deserialize)]
struct Fixture {
    games: Vec<FixtureGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureGame {
    initial: FixtureInitial,
    manifest_json: Option<String>,
    seed: u32,
    steps: Vec<FixtureStep>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureInitial {
    random_draws_hash: String,
    state_hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureStep {
    event_ids: Vec<String>,
    event_types: Vec<String>,
    post_state_hash: String,
    pre_state_hash: String,
    random_draws_hash: String,
    receipt_id: String,
    selected_action_id: String,
    state_version: u64,
}

fn seed_31_fixture() -> FixtureGame {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid checked-in TypeScript parity fixture");
    fixture
        .games
        .into_iter()
        .find(|game| game.seed == 31)
        .expect("seed-31 fixture")
}

#[test]
fn rejected_requests_should_not_mutate_authoritative_state() {
    let fixture = seed_31_fixture();
    let manifest = fixture
        .manifest_json
        .as_deref()
        .expect("seed-31 canonical manifest JSON");
    let mut session = Session::new(manifest).expect("valid session");
    let initial_state_hash = session.state_hash().expect("initial state hash");
    let initial_draws_hash = session
        .initial_random_draws_hash()
        .expect("initial setup draws hash");
    assert_eq!(initial_state_hash.as_str(), fixture.initial.state_hash);
    assert_eq!(
        initial_draws_hash.as_str(),
        fixture.initial.random_draws_hash
    );

    let initial_actions = session.legal_actions().expect("north legal actions");
    let stale = session
        .step(ActionRequest {
            action_id: initial_actions[0].action_id.to_string(),
            seat: Seat::North,
            state_version: 9,
        })
        .expect("stable stale rejection");
    assert!(matches!(
        stale,
        StepResult::Rejected(rejection) if rejection.code == RejectionCode::StaleVersion
    ));
    let wrong_seat = session
        .step(ActionRequest {
            action_id: initial_actions[0].action_id.to_string(),
            seat: Seat::South,
            state_version: 0,
        })
        .expect("stable wrong-seat rejection");
    assert!(matches!(
        wrong_seat,
        StepResult::Rejected(rejection) if rejection.code == RejectionCode::WrongSeat
    ));
    let forged = session
        .step(ActionRequest {
            action_id: "forged-but-bounded-action-id".to_owned(),
            seat: Seat::North,
            state_version: 0,
        })
        .expect("stable forged-action rejection");
    assert!(matches!(
        forged,
        StepResult::Rejected(rejection) if rejection.code == RejectionCode::UnknownAction
    ));
    assert_eq!(session.state_version(), 0);
    assert_eq!(
        session.state_hash().expect("unchanged state"),
        initial_state_hash
    );
    assert_eq!(
        session
            .initial_random_draws_hash()
            .expect("unchanged PRNG setup"),
        initial_draws_hash
    );
    assert!(session.transcript().is_empty());
    assert_eq!(session.attempts().len(), 3);
    assert_eq!(
        session.attempts()[2].request.action_id,
        "forged-but-bounded-action-id"
    );
}

#[test]
fn session_receipts_and_replay_should_match_typescript_through_repeated_setup_turns() {
    let fixture = seed_31_fixture();
    let manifest = fixture
        .manifest_json
        .as_deref()
        .expect("seed-31 canonical manifest JSON");
    let mut session = Session::new(manifest).expect("valid session");
    let initial_draws_hash = session
        .initial_random_draws_hash()
        .expect("initial setup draws hash");

    let mut accepted_action_ids = Vec::new();
    for step_index in 0..13 {
        let expected = &fixture.steps[step_index];
        let action = session
            .legal_actions()
            .expect("legal mulligan actions")
            .into_iter()
            .find(|action| action.action_id.as_str() == expected.selected_action_id)
            .unwrap_or_else(|| panic!("fixture-selected legal action at step {step_index}"));
        accepted_action_ids.push(action.action_id.clone());
        let result = session
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .expect("accepted authoritative step");
        let StepResult::Accepted(receipt) = result else {
            panic!("fixture action must be accepted");
        };
        assert_eq!(receipt.state_version, expected.state_version);
        assert_eq!(receipt.pre_state_hash.as_str(), expected.pre_state_hash);
        assert_eq!(receipt.post_state_hash.as_str(), expected.post_state_hash);
        assert_eq!(receipt.receipt_id.as_str(), expected.receipt_id);
        assert_eq!(
            receipt
                .events
                .iter()
                .map(|event| event.event_id.as_str())
                .collect::<Vec<_>>(),
            expected.event_ids
        );
        assert_eq!(
            receipt
                .events
                .iter()
                .map(|event| event.event_type.as_str())
                .collect::<Vec<_>>(),
            expected.event_types
        );
        assert_eq!(
            identity_hash(&serde_json::to_value(&receipt.random_draws).expect("random draws JSON"))
                .expect("random draws hash")
                .as_str(),
            expected.random_draws_hash
        );
    }

    let replayed = Session::replay(manifest, &accepted_action_ids).expect("accepted replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.state_hash().expect("replayed state hash"),
        session.state_hash().expect("session state hash")
    );
    assert_eq!(
        replayed
            .initial_random_draws_hash()
            .expect("replayed setup draws"),
        initial_draws_hash
    );
    assert_eq!(
        replayed.replay_value().expect("replay verification value"),
        session.replay_value().expect("session verification value")
    );
    assert!(
        session
            .verify_replay()
            .expect("verified deterministic replay")
    );
}
