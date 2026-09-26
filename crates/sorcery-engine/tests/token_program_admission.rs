//! Admission proofs for authored token references and dependency closure.

use serde_json::{Value, json};
use sorcery_engine::game::Game;
use sorcery_engine::synthetic::selfplay_manifest_with;

fn summon(token: &str) -> Value {
    json!({
        "op": "summon-token",
        "token": token,
        "count": 1,
        "destination": "source",
    })
}

fn magic(effects: &[Value]) -> Value {
    json!({
        "cardType": "magic",
        "effectProgram": { "effects": effects },
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn token_minion(genesis: Option<Value>) -> Value {
    let mut card = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "token": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if let Some(program) = genesis {
        card["genesisProgram"] = program;
    }
    card
}

fn genesis(effects: &[Value]) -> Value {
    json!({ "effects": effects })
}

fn rejected(manifest: &str, expected_error: &str) {
    let error = Game::from_manifest_json(manifest).expect_err("malformed token fixture admitted");
    let rendered = error.to_string();
    assert!(
        rendered.contains(expected_error),
        "expected error containing {expected_error:?}, got {rendered}"
    );
}

#[test]
fn authored_magic_accepts_multiple_token_kinds_and_nested_genesis_dependencies() {
    let manifest = selfplay_manifest_with(7101, |manifest| {
        manifest["cards"]["north-spell-1"] = magic(&[summon("token-a"), summon("token-b")]);
        manifest["cards"]["token-a"] = token_minion(Some(genesis(&[summon("token-b")])));
        manifest["cards"]["token-b"] = token_minion(None);
    });

    Game::from_manifest_json(&manifest).expect("token references and nested Genesis are admitted");
}

#[test]
fn token_program_rejects_missing_and_non_token_references() {
    let missing = selfplay_manifest_with(7102, |manifest| {
        manifest["cards"]["north-spell-1"] = magic(&[summon("missing-token")]);
    });
    rejected(&missing, "token effect must reference a token minion");

    let non_token = selfplay_manifest_with(7103, |manifest| {
        manifest["cards"]["north-spell-1"] = magic(&[summon("ordinary-minion")]);
        manifest["cards"]["ordinary-minion"] = token_minion(None);
        manifest["cards"]["ordinary-minion"]
            .as_object_mut()
            .unwrap()
            .remove("token");
    });
    rejected(&non_token, "token effect must reference a token minion");
}

#[test]
fn token_program_rejects_recursive_and_over_depth_dependencies() {
    let cycle = selfplay_manifest_with(7104, |manifest| {
        manifest["cards"]["north-spell-1"] = magic(&[summon("cycle-a")]);
        manifest["cards"]["cycle-a"] = token_minion(Some(genesis(&[summon("cycle-b")])));
        manifest["cards"]["cycle-b"] = token_minion(Some(genesis(&[summon("cycle-a")])));
    });
    rejected(&cycle, "recursive token dependencies are unsupported");

    let too_deep = selfplay_manifest_with(7105, |manifest| {
        // The short route completes the tail first; memoization must not hide the long path.
        manifest["cards"]["north-spell-1"] = magic(&[summon("chain-0"), summon("chain-32")]);
        for index in 0..65 {
            let next = (index < 64).then(|| summon(&format!("chain-{}", index + 1)));
            manifest["cards"][format!("chain-{index}")] =
                token_minion(next.map(|effect| genesis(&[effect])));
        }
    });
    rejected(
        &too_deep,
        "token dependency chains longer than 64 are unsupported",
    );
}

#[test]
fn token_program_rejects_invalid_operands_and_bindings() {
    let zero_count = selfplay_manifest_with(7106, |manifest| {
        let mut effect = summon("token-a");
        effect["count"] = json!(0);
        manifest["cards"]["north-spell-1"] = magic(&[effect]);
        manifest["cards"]["token-a"] = token_minion(None);
    });
    rejected(&zero_count, "count must be 1-32");

    let excessive_count = selfplay_manifest_with(7107, |manifest| {
        let mut effect = summon("token-a");
        effect["count"] = json!(33);
        manifest["cards"]["north-spell-1"] = magic(&[effect]);
        manifest["cards"]["token-a"] = token_minion(None);
    });
    rejected(&excessive_count, "count must be 1-32");

    let unbound_destination = selfplay_manifest_with(7108, |manifest| {
        let mut effect = summon("token-a");
        effect["destination"] = json!("chosen");
        manifest["cards"]["north-spell-1"] = magic(&[effect]);
        manifest["cards"]["token-a"] = token_minion(None);
    });
    rejected(
        &unbound_destination,
        "destination chosen requires a preceding choose-unit",
    );
}
