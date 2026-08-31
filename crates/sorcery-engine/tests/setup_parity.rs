use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::game::Game;

#[derive(Deserialize)]
struct Fixture {
    games: Vec<FixtureGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureGame {
    initial: FixtureInitial,
    manifest_id: String,
    manifest_json: Option<String>,
    seed: u32,
    steps: Vec<FixtureStep>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureInitial {
    legal_action_ids: Vec<String>,
    random_draws_hash: String,
    state_hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureStep {
    legal_action_ids: Vec<String>,
    post_state_hash: String,
    selected_action_id: String,
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
        .expect("seed-31 game fixture")
}

#[test]
fn setup_and_mulligans_should_match_typescript_seed_31() {
    let fixture = seed_31_fixture();
    let mut game = Game::from_manifest_json(
        fixture
            .manifest_json
            .as_deref()
            .expect("seed-31 canonical manifest JSON"),
    )
    .expect("valid canonical synthetic manifest");
    let branch = game.clone();

    assert!(Arc::ptr_eq(game.rules(), branch.rules()));
    assert_eq!(game.rules().manifest_id().as_str(), fixture.manifest_id);
    assert_eq!(
        game.state_hash().expect("initial state hash").as_str(),
        fixture.initial.state_hash
    );
    assert_eq!(
        game.initial_random_draws_hash()
            .expect("initial random draw hash")
            .as_str(),
        fixture.initial.random_draws_hash
    );

    let north_actions = game.legal_actions().expect("north mulligans");
    assert_eq!(north_actions.len(), 76);
    assert_eq!(
        north_actions
            .iter()
            .map(|action| action.action_id().as_str())
            .collect::<Vec<_>>(),
        fixture.initial.legal_action_ids
    );
    let north_action = north_actions
        .iter()
        .find(|action| action.action_id().as_str() == fixture.steps[0].selected_action_id)
        .expect("fixture north action");
    game.apply_action(north_action)
        .expect("apply north mulligan");
    assert_eq!(
        game.state_hash().expect("north post-state hash").as_str(),
        fixture.steps[0].post_state_hash
    );

    let south_actions = game.legal_actions().expect("south mulligans");
    assert_eq!(
        south_actions
            .iter()
            .map(|action| action.action_id().as_str())
            .collect::<Vec<_>>(),
        fixture.steps[1].legal_action_ids
    );
    let south_action = south_actions
        .iter()
        .find(|action| action.action_id().as_str() == fixture.steps[1].selected_action_id)
        .expect("fixture south action");
    game.apply_action(south_action)
        .expect("apply south mulligan");
    assert_eq!(game.position().state_version(), 2);
    assert_eq!(
        game.state_hash().expect("south post-state hash").as_str(),
        fixture.steps[1].post_state_hash
    );
    let main_actions = game.legal_actions().expect("first main actions");
    assert_eq!(
        main_actions
            .iter()
            .map(|action| action.action_id().as_str())
            .collect::<Vec<_>>(),
        fixture.steps[2].legal_action_ids
    );
    let site_action = main_actions
        .iter()
        .find(|action| action.action_id().as_str() == fixture.steps[2].selected_action_id)
        .expect("fixture site action");
    game.apply_action(site_action).expect("apply first site");
    assert_eq!(
        game.state_hash().expect("site post-state hash").as_str(),
        fixture.steps[2].post_state_hash
    );
}

#[test]
fn manifest_json_should_reject_duplicate_top_level_keys() {
    let fixture = seed_31_fixture();
    let manifest = fixture
        .manifest_json
        .expect("seed-31 canonical manifest JSON");
    let duplicate = manifest.replacen('{', r#"{"seed":31,"#, 1);

    assert!(Game::from_manifest_json(&duplicate).is_err());
}

#[test]
fn rules_context_should_resolve_typed_token_references() {
    let fixture = seed_31_fixture();
    let mut manifest: Value = serde_json::from_str(
        fixture
            .manifest_json
            .as_deref()
            .expect("seed-31 canonical manifest JSON"),
    )
    .expect("manifest value");
    let spell_id = manifest["decks"]["north"]["spellbook"][0]
        .as_str()
        .expect("north spell id")
        .to_owned();
    manifest["cards"][&spell_id] = json!({
        "cardType": "magic",
        "manaCost": 1,
        "summonTokenToEachControlledSiteBorderingEnemySite": "test-token",
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    manifest["cards"]["test-token"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    let mut body = manifest.as_object().expect("manifest object").clone();
    body.remove("manifestId");
    manifest["manifestId"] =
        serde_json::to_value(identity_hash(&Value::Object(body)).expect("manifest identity"))
            .expect("identity JSON");
    let canonical = canonical_json(&manifest).expect("canonical token manifest");

    Game::from_manifest_json(&canonical).expect("resolved token definition");

    manifest["cards"]["test-token"]["token"] = Value::Bool(false);
    let mut invalid_body = manifest.as_object().expect("manifest object").clone();
    invalid_body.remove("manifestId");
    manifest["manifestId"] = serde_json::to_value(
        identity_hash(&Value::Object(invalid_body)).expect("invalid manifest identity"),
    )
    .expect("identity JSON");
    let invalid = canonical_json(&manifest).expect("canonical invalid manifest");
    assert!(Game::from_manifest_json(&invalid).is_err());
}
