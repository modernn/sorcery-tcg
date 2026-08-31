use serde_json::{Value, json};
use sorcery_engine::action::ActionDescriptor;
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::Seat;
use sorcery_engine::game::Game;
use sorcery_engine::policy::{PolicySnapshot, parse_policy_snapshot};

const HASH_A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/engine/fixtures/typescript-parity-v1.json"
    ))
    .expect("valid parity fixture")
}

fn seed_31_manifest(fixture: &Value) -> &str {
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
    let policy_id = identity_hash(&body).expect("policy body hash");
    body["policyId"] = json!(policy_id);
    parse_policy_snapshot(&canonical_json(&body).expect("canonical policy"))
        .expect("valid baseline policy")
}

#[test]
fn baseline_policy_should_reproduce_the_complete_seed_31_action_sequence() {
    let fixture = fixture();
    let game_fixture = fixture["games"]
        .as_array()
        .and_then(|games| games.iter().find(|game| game["seed"] == 31))
        .expect("seed-31 game");
    let expected = game_fixture["actionIds"].as_array().expect("action IDs");
    let mut game = Game::from_manifest_json(seed_31_manifest(&fixture)).expect("valid game");
    let policy = baseline_policy();

    for (step, expected_id) in expected.iter().enumerate() {
        let observation = game.observe(game.position().decision_seat());
        let actions = game.legal_actions().expect("legal actions");
        let selected = policy
            .select_action(observation, &actions)
            .expect("selected action");
        assert_eq!(
            selected.action_id().as_str(),
            expected_id.as_str().expect("action identity"),
            "policy action at step {step}"
        );
        game.apply_action(selected).expect("accepted policy action");
    }
    assert!(game.is_terminal());
    assert!(game.legal_actions().expect("terminal actions").is_empty());
    assert_eq!(
        game.state_hash().expect("final state hash").as_str(),
        game_fixture["finalStateHash"]
            .as_str()
            .expect("fixture final state hash")
    );
}

#[test]
fn policy_observation_should_not_change_with_opponent_hidden_order() {
    let fixture = fixture();
    let manifest = seed_31_manifest(&fixture);
    let original = Game::from_manifest_json(manifest).expect("original game");
    let mut changed: Value = serde_json::from_str(manifest).expect("manifest value");
    let body = changed.as_object_mut().expect("manifest object");
    body.remove("manifestId").expect("manifest identity");
    let spellbook = body["decks"]["south"]["spellbook"]
        .as_array_mut()
        .expect("south Spellbook");
    spellbook.swap(0, 1);
    let manifest_id = identity_hash(&changed).expect("changed manifest identity");
    changed["manifestId"] = json!(manifest_id);
    let changed = Game::from_manifest_json(&canonical_json(&changed).expect("canonical manifest"))
        .expect("changed game");

    assert_eq!(original.observe(Seat::North), changed.observe(Seat::North));
    assert_eq!(
        original
            .legal_actions()
            .expect("original actions")
            .iter()
            .map(|action| action.action_id().clone())
            .collect::<Vec<IdentityHash>>(),
        changed
            .legal_actions()
            .expect("changed actions")
            .iter()
            .map(|action| action.action_id().clone())
            .collect::<Vec<IdentityHash>>()
    );
}

#[test]
fn policy_binding_and_empty_action_sets_should_fail_closed() {
    let policy = baseline_policy();
    let authority = IdentityHash::parse(HASH_A).expect("authority hash");
    let deck = IdentityHash::parse(HASH_B).expect("deck hash");
    let wrong = IdentityHash::parse(
        "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    )
    .expect("wrong hash");
    assert!(
        policy
            .validate_binding(&authority, &deck, "sorcery-core-v1")
            .is_ok()
    );
    assert!(
        policy
            .validate_binding(&wrong, &deck, "sorcery-core-v1")
            .is_err()
    );

    let fixture = fixture();
    let game = Game::from_manifest_json(seed_31_manifest(&fixture)).expect("valid game");
    let error = policy
        .select_action(game.observe(Seat::North), &[])
        .expect_err("empty legal actions must fail");
    assert!(error.to_string().contains("at least one legal action"));
    assert!(matches!(
        game.legal_actions().expect("initial actions")[0].descriptor(),
        ActionDescriptor::Mulligan { .. }
    ));
}
