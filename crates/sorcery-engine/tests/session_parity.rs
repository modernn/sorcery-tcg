use serde::Deserialize;
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, RejectionCode, Seat};
use sorcery_engine::session::{Session, StepResult};

#[derive(Deserialize)]
struct Fixture {
    games: Vec<FixtureGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureGame {
    action_ids: Vec<String>,
    final_state_hash: String,
    initial: FixtureInitial,
    manifest_json: Option<String>,
    replay: serde_json::Value,
    seed: u32,
    steps: Vec<FixtureStep>,
    transcript_hash: String,
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
    legal_action_ids: Vec<String>,
    post_state_hash: String,
    pre_state_hash: String,
    random_draws_hash: String,
    receipt_id: String,
    selected_action_id: String,
    state_version: u64,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid checked-in TypeScript parity fixture")
}

fn seed_31_fixture() -> FixtureGame {
    fixture()
        .games
        .into_iter()
        .find(|game| game.seed == 31)
        .expect("seed-31 fixture")
}

fn manifest_for_seed(template: &str, seed: u32) -> String {
    let mut manifest: serde_json::Value =
        serde_json::from_str(template).expect("canonical fixture manifest");
    let body = manifest.as_object_mut().expect("manifest object");
    body.remove("manifestId").expect("manifest identity");
    body.insert("seed".to_owned(), serde_json::json!(seed));
    let manifest_id = identity_hash(&manifest).expect("manifest identity for selected seed");
    manifest["manifestId"] = serde_json::json!(manifest_id);
    canonical_json(&manifest).expect("canonical manifest with selected seed")
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
fn sessions_should_match_all_complete_typescript_fixture_games() {
    let fixture = fixture();
    let template = fixture
        .games
        .iter()
        .find(|game| game.seed == 31)
        .and_then(|game| game.manifest_json.as_deref())
        .expect("seed-31 canonical manifest JSON")
        .to_owned();
    for game in fixture.games {
        verify_fixture_game(&game, &manifest_for_seed(&template, game.seed));
    }
}

fn verify_fixture_game(fixture: &FixtureGame, manifest: &str) {
    let fixture_manifest = fixture.manifest_json.as_deref().unwrap_or(manifest);
    assert_eq!(fixture_manifest, manifest);
    let mut session = Session::new(manifest).expect("valid session");
    let initial_draws_hash = session
        .initial_random_draws_hash()
        .expect("initial setup draws hash");

    let mut accepted_action_ids = Vec::new();
    for step_index in 0..fixture.steps.len() {
        accepted_action_ids.push(accept_fixture_step(
            &mut session,
            &fixture.steps[step_index],
            step_index,
        ));
    }

    assert_eq!(
        accepted_action_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        fixture.action_ids
    );
    assert_eq!(
        session.state_hash().expect("final state hash").as_str(),
        fixture.final_state_hash
    );
    assert_eq!(
        session.transcript_hash().expect("transcript hash").as_str(),
        fixture.transcript_hash
    );
    assert_eq!(
        serde_json::json!({
            "finalStateHash": session.state_hash().expect("replay final state hash"),
            "transcriptHash": session.transcript_hash().expect("replay transcript hash"),
            "verified": session.verify_replay().expect("fixture replay verification"),
        }),
        fixture.replay
    );

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

fn accept_fixture_step(
    session: &mut Session,
    expected: &FixtureStep,
    step_index: usize,
) -> IdentityHash {
    let legal_actions = session.legal_actions().expect("legal actions");
    assert_eq!(
        legal_actions
            .iter()
            .map(|action| action.action_id.as_str())
            .collect::<Vec<_>>(),
        expected.legal_action_ids,
        "legal action order at step {step_index}"
    );
    let action = legal_actions
        .into_iter()
        .find(|action| action.action_id.as_str() == expected.selected_action_id)
        .unwrap_or_else(|| panic!("fixture-selected legal action at step {step_index}"));
    let action_id = action.action_id.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap_or_else(|error| panic!("accepted authoritative step {step_index}: {error}"));
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
    action_id
}
