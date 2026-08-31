use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Seat};
use sorcery_engine::game::Game;
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};
use sorcery_engine::session::Session;
use sorcery_engine::simulator::SimulatorError;
use sorcery_engine::simulator::{replay_selected, run_game, search_root_actions};

const HASH_A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const MAX_ACTIONS: usize = 400;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid parity fixture")
}

fn manifest(fixture: &Value) -> &str {
    fixture["games"]
        .as_array()
        .and_then(|games| games.iter().find(|game| game["seed"] == 31))
        .and_then(|game| game["manifestJson"].as_str())
        .expect("seed-31 manifest")
}

fn baseline_policy() -> PolicySnapshot {
    let mut body = json!({
        "authorityHash": HASH_A,
        "deckId": HASH_B,
        "engineVersion": "sorcery-core-v1",
        "generation": 0,
        "observationVersion": "seat-observation-v1",
        "schemaVersion": 1,
        "selector": {
            "atlasReserve": 3,
            "featurePriority": [
                "keep-mulligan",
                "play-site",
                "summon-minion",
                "preferred-draw",
                "powered-movement",
                "beneficial-tactic",
                "move-toward-enemy",
                "end-turn",
                "canonical-fallback"
            ]
        },
        "tieBreak": "canonical-action-order-v1"
    });
    body["policyId"] = json!(identity_hash(&body).expect("policy identity"));
    parse_policy_snapshot(&canonical_json(&body).expect("canonical policy"))
        .expect("valid baseline policy")
}

#[test]
fn game_rollout_should_repeat_exactly() {
    let fixture = fixture();
    let manifest = manifest(&fixture);
    let policy = baseline_policy();
    let first = run_game(
        Game::from_manifest_json(manifest).expect("first game"),
        &policy,
        &policy,
        MAX_ACTIONS,
    )
    .expect("first rollout");
    let second = run_game(
        Game::from_manifest_json(manifest).expect("second game"),
        &policy,
        &policy,
        MAX_ACTIONS,
    )
    .expect("second rollout");

    assert_eq!(first, second);
    assert!(first.is_terminal());
}

#[test]
fn root_search_should_cover_canonical_actions_in_order() {
    let fixture = fixture();
    let game = Game::from_manifest_json(manifest(&fixture)).expect("valid game");
    let expected = 0..game.legal_actions().expect("root actions").len().min(4);
    let policy = baseline_policy();
    let rollouts = search_root_actions(&game, &policy, &policy, MAX_ACTIONS, expected.len())
        .expect("root rollouts");

    assert_eq!(
        rollouts
            .iter()
            .map(|rollout| rollout.action_indices()[0])
            .collect::<Vec<_>>(),
        expected.collect::<Vec<_>>()
    );
}

#[test]
fn selected_rollout_should_replay_authoritatively() {
    let fixture = fixture();
    let manifest = manifest(&fixture);
    let game = Game::from_manifest_json(manifest).expect("valid game");
    let policy = baseline_policy();
    let rollout = search_root_actions(&game, &policy, &policy, MAX_ACTIONS, 1)
        .expect("root rollout")
        .pop()
        .expect("selected rollout");
    let session = replay_selected(manifest, &rollout).expect("verified replay");

    assert!(session.verify_replay().expect("replay verification"));
    assert!(rollout.is_terminal());
}

#[test]
fn speculative_and_recorded_transitions_should_produce_identical_state() {
    let fixture = fixture();
    let manifest = manifest(&fixture);
    let mut game = Game::from_manifest_json(manifest).expect("valid speculative game");
    let action = game.legal_actions().expect("legal action")[0].clone();
    let mut session = Session::new(manifest).expect("valid authoritative session");

    game.apply_action(&action).expect("speculative transition");
    session
        .step(ActionRequest {
            action_id: action
                .to_legal_action()
                .expect("materialized action")
                .action_id
                .to_string(),
            seat: Seat::North,
            state_version: 0,
        })
        .expect("recorded transition");

    assert_eq!(
        game.state_hash().expect("speculative state hash"),
        session.state_hash().expect("recorded state hash")
    );
}

#[test]
fn selected_rollout_should_reject_a_different_manifest() {
    let fixture = fixture();
    let source_manifest = manifest(&fixture);
    let policy = baseline_policy();
    let rollout = run_game(
        Game::from_manifest_json(source_manifest).expect("source game"),
        &policy,
        &policy,
        1,
    )
    .expect("source rollout");
    let mut other: Value = serde_json::from_str(source_manifest).expect("manifest value");
    let body = other.as_object_mut().expect("manifest object");
    body.remove("manifestId").expect("manifest identity");
    body.insert("seed".to_owned(), json!(32));
    other["manifestId"] = json!(identity_hash(&other).expect("other manifest identity"));
    let other = canonical_json(&other).expect("canonical other manifest");

    assert!(matches!(
        replay_selected(&other, &rollout),
        Err(SimulatorError::ReplayDiverged)
    ));
}
