//! Damage provenance belongs to the resolving spell, never its caster.
use serde_json::{Value, json};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::selfplay_manifest_with;

fn manifest(filter: &str, elements: &[&str], effect: Value) -> String {
    selfplay_manifest_with(1907, |m| {
        let zero = json!({"earth":0,"fire":0,"water":0,"air":0});
        let mut thresholds = zero.clone();
        for element in elements {
            thresholds[*element] = json!(1);
        }
        m["cards"] = json!({
            "avatar":{"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "site":{"cardType":"site","elements":["earth","fire","water","air"]},
            "protected":{"cardType":"minion","attack":1,"defense":8,"manaCost":0,"thresholds":zero,"preventsDamageFrom":filter},
            "caster":{"cardType":"minion","attack":1,"defense":8,"manaCost":0,"thresholds":zero,"elements":["fire"],"spellcaster":true},
            "spell":{"cardType":"magic","manaCost":0,"thresholds":thresholds},
        });
        m["cards"]["spell"]
            .as_object_mut()
            .unwrap()
            .extend(match effect {
                Value::Object(fields) => fields,
                _ => panic!("effect object"),
            });
        m["decks"] = json!({
            "north":{"avatar":"avatar","atlas":vec!["site";6],"spellbook":["protected","caster","spell"]},
            "south":{"avatar":"avatar","atlas":vec!["site";6],"spellbook":vec!["protected";3]},
        });
    })
}

fn act(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Receipt {
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| predicate(&a.descriptor))
        .expect("issued action");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap()
    else {
        panic!("issued action rejected")
    };
    receipt
}

fn ready(filter: &str, elements: &[&str], effect: Value) -> (Session, Value, Value) {
    let mut session = Session::new(&manifest(filter, elements, effect)).unwrap();
    for _ in 0..2 {
        act(&mut session, |d| {
            d["kind"] == "mulligan"
                && d["atlasOrder"] == json!([])
                && d["spellbookOrder"] == json!([])
        });
    }
    act(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C4"
    });
    for card in ["protected", "caster"] {
        act(&mut session, |d| {
            d["kind"] == "summon-minion" && d["cardId"] == card && d["cell"] == "C4"
        });
    }
    let state = session.replay_value().unwrap();
    let units = state["state"]["realm"]["units"].as_array().unwrap();
    let id = |name: &str| units.iter().find(|u| u["cardId"] == name).unwrap()["instanceId"].clone();
    (session, id("protected"), id("caster"))
}

fn assert_damage(session: &Session, target: &Value, expected: u64) {
    let amounts: Vec<_> = session
        .transcript()
        .iter()
        .flat_map(|r| &r.events)
        .filter(|e| e.event_type == "damage-dealt" && e.payload["instanceId"] == *target)
        .map(|e| e.payload["amount"].as_u64().unwrap())
        .collect();
    assert_eq!(amounts, vec![expected]);
    assert!(session.verify_replay().unwrap());
}

#[test]
fn spell_element_filters_ignore_caster_identity_and_match_mixed_spells() {
    for element in ["earth", "fire", "water", "air"] {
        for spell_element in ["earth", "fire", "water", "air"] {
            let (mut session, target, caster) = ready(
                &format!("{element}-magic"),
                &[spell_element],
                json!({"damageTargetUnit":2}),
            );
            act(&mut session, |d| {
                d["kind"] == "cast-magic"
                    && d["casterInstanceId"] == caster
                    && d["target"]["instanceId"] == target
            });
            assert_damage(
                &session,
                &target,
                if element == spell_element { 0 } else { 2 },
            );
        }
    }
    for filter in ["fire-magic", "air-magic", "magic", "ranged-strikes"] {
        let (mut session, target, caster) =
            ready(filter, &["fire", "air"], json!({"damageTargetUnit":2}));
        act(&mut session, |d| {
            d["kind"] == "cast-magic"
                && d["casterInstanceId"] == caster
                && d["target"]["instanceId"] == target
        });
        assert_damage(
            &session,
            &target,
            if filter == "ranged-strikes" { 2 } else { 0 },
        );
    }
    for filter in ["magic", "fire-magic"] {
        let (mut session, target, caster) = ready(filter, &[], json!({"damageTargetUnit":2}));
        act(&mut session, |d| {
            d["kind"] == "cast-magic"
                && d["casterInstanceId"] == caster
                && d["target"]["instanceId"] == target
        });
        assert_damage(&session, &target, if filter == "magic" { 0 } else { 2 });
    }
}

#[test]
fn composed_magic_keeps_origin_across_pending_choice_and_checkpoint() {
    let effect = json!({"effectProgram":{"effects":[
        {"op":"choose-unit","kind":"minion","relation":"anywhere","alliedOnly":true},
        {"op":"damage","amount":2,"recipients":"chosen"}
    ]}});
    let (mut session, target, caster) = ready("fire-magic", &["fire"], effect);
    act(&mut session, |d| {
        d["kind"] == "cast-magic" && d["casterInstanceId"] == caster
    });
    assert_eq!(
        session.replay_value().unwrap()["state"]["phase"],
        "ability-choice"
    );
    assert!(
        session
            .replay_value()
            .unwrap()
            .to_string()
            .contains("damageOrigin")
    );
    let mut resumed = resume_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
    let predicate =
        |d: &Value| d["kind"] == "choose-ability" && d["target"]["instanceId"] == target;
    assert_eq!(act(&mut session, predicate), act(&mut resumed, predicate));
    assert_eq!(
        session.replay_value().unwrap(),
        resumed.replay_value().unwrap()
    );
    assert_damage(&session, &target, 0);
}

#[test]
fn chain_and_area_magic_retain_spell_origin() {
    for filter in ["fire-magic", "air-magic"] {
        let (mut session, target, caster) =
            ready(filter, &["fire"], json!({"damageChainNearbyUnits":true}));
        act(&mut session, |d| {
            d["kind"] == "begin-chain-magic"
                && d["casterInstanceId"] == caster
                && d["target"]["instanceId"] == target
        });
        let mut resumed =
            resume_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
        assert_eq!(
            act(&mut session, |d| d["kind"] == "resolve-chain-magic"),
            act(&mut resumed, |d| d["kind"] == "resolve-chain-magic")
        );
        assert_damage(
            &session,
            &target,
            if filter == "fire-magic" { 0 } else { 2 },
        );
        let (mut area, target, caster) =
            ready(filter, &["fire"], json!({"damageEachAbovegroundMinion":1}));
        act(&mut area, |d| {
            d["kind"] == "cast-magic" && d["casterInstanceId"] == caster
        });
        assert_damage(&area, &target, u64::from(filter != "fire-magic"));
    }
}
