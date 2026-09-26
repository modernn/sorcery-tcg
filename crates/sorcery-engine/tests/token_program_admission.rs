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

fn artifact_token() -> Value {
    json!({
        "cardType": "artifact", "manaCost": null, "token": true, "grantsBearerPower": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn conjure(token: &str) -> Value {
    let mut effect = summon(token);
    effect["op"] = json!("conjure-token");
    effect
}

#[test]
fn artifact_token_dependencies_preserve_kind_cost_and_deck_exclusion() {
    let valid = selfplay_manifest_with(7110, |manifest| {
        manifest["cards"]["north-spell-1"] = magic(&[summon("minion-token")]);
        manifest["cards"]["minion-token"] =
            token_minion(Some(genesis(&[conjure("artifact-token")])));
        manifest["cards"]["artifact-token"] = artifact_token();
    });
    Game::from_manifest_json(&valid).expect("minion Genesis can conjure an artifact token");

    for (effect, card, error) in [
        (
            summon("token"),
            artifact_token(),
            "token effect must reference a token minion",
        ),
        (
            conjure("token"),
            token_minion(None),
            "conjure-token must reference a token artifact",
        ),
        (
            conjure("missing"),
            artifact_token(),
            "conjure-token must reference a token artifact",
        ),
    ] {
        let invalid = selfplay_manifest_with(7111, |manifest| {
            manifest["cards"]["north-spell-1"] = magic(&[effect]);
            manifest["cards"]["token"] = card;
        });
        rejected(&invalid, error);
    }
    let in_deck = selfplay_manifest_with(7112, |manifest| {
        manifest["cards"]["north-spell-1"] = artifact_token();
    });
    rejected(&in_deck, "unsupported token spell");
    let non_token = selfplay_manifest_with(7113, |manifest| {
        let mut artifact = artifact_token();
        artifact.as_object_mut().unwrap().remove("token");
        artifact["manaCost"] = json!(0);
        manifest["cards"]["north-spell-1"] = magic(&[conjure("token")]);
        manifest["cards"]["token"] = artifact;
    });
    rejected(&non_token, "conjure-token must reference a token artifact");
}

#[test]
fn artifact_absent_cost_is_not_zero_or_missing_cost() {
    use sorcery_engine::facts::{CardFacts, parse_card_definition};
    let null = artifact_token();
    let CardFacts::Artifact(facts) = parse_card_definition("absent", &null).unwrap() else {
        panic!("artifact");
    };
    assert_eq!(facts.mana_cost, None);
    assert!(facts.token);
    let mut zero = null.clone();
    zero["manaCost"] = json!(0);
    let CardFacts::Artifact(facts) = parse_card_definition("zero", &zero).unwrap() else {
        panic!("artifact");
    };
    assert_eq!(facts.mana_cost, Some(0));
    let mut missing = null.clone();
    missing.as_object_mut().unwrap().remove("manaCost");
    assert!(parse_card_definition("missing", &missing).is_err());
    let mut non_token = null;
    non_token.as_object_mut().unwrap().remove("token");
    assert!(parse_card_definition("not-token", &non_token).is_err());
}

#[test]
fn carried_conjure_requires_unit_destination_and_carriable_artifact() {
    let mut carried = conjure("token");
    carried["placement"] = json!("carried");
    let valid = selfplay_manifest_with(7114, |m| {
        m["cards"]["north-spell-1"] = magic(&[carried.clone()]);
        m["cards"]["token"] = artifact_token();
    });
    Game::from_manifest_json(&valid).expect("carried source admitted");
    for destination in ["location", "chosen-location"] {
        let invalid = selfplay_manifest_with(7114, |m| {
            let mut effect = carried.clone();
            effect["destination"] = json!(destination);
            m["cards"]["north-spell-1"] = magic(&[effect]);
            m["cards"]["token"] = artifact_token();
        });
        rejected(&invalid, "carried placement requires a unit destination");
    }
    let immovable = selfplay_manifest_with(7114, |m| {
        m["cards"]["north-spell-1"] = magic(&[conjure("token"), carried.clone()]);
        m["cards"]["token"] = artifact_token();
        m["cards"]["token"]["cannotBeCarried"] = json!(true);
    });
    rejected(
        &immovable,
        "carried token must reference a carriable artifact",
    );
    let minion = selfplay_manifest_with(7114, |m| {
        let mut effect = summon("token");
        effect["placement"] = json!("carried");
        m["cards"]["north-spell-1"] = magic(&[effect]);
        m["cards"]["token"] = token_minion(None);
    });
    rejected(&minion, "unknown field");
}
