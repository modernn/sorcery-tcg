//! Authored Genesis shares area damage, ordered death settlement, ordinary choice,
//! untapping and draw with Magic. Every fixture definition is synthetic.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn minion() -> Value {
    json!({"cardType":"minion", "attack":1, "defense":3, "manaCost":0,
        "thresholds":{"air":0,"earth":0,"fire":0,"water":0}, "summonToAnySite":true})
}

fn manifest(seed: u32) -> String {
    let mut programmed = minion();
    programmed["genesisProgram"] = json!({"effects":[
        {"op":"damage","amount":1,"recipients":{"query":{
            "area":"source","relation":"nearby","excludeSource":true}}},
        {"op":"choose-unit","relation":"adjacent","alliedOnly":true},
        {"op":"untap","recipients":"chosen"},
        {"op":"draw","zone":"spellbook","count":1}
    ]});
    let mut dying = minion();
    dying["defense"] = json!(1);
    dying["deathriteDrawSite"] = json!(true);
    let avatar = json!({"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false});
    let site = json!({"cardType":"site","elements":["earth"]});
    let mut value = json!({
        "schemaVersion":1, "engineVersion":"sorcery-core-v1", "firstSeat":"north", "seed":seed,
        "authority":{"mode":"synthetic","revisionId":"synthetic-genesis-program-v1",
            "contentHash":identity_hash(&json!({"fixture":"genesis-program"})).unwrap()},
        "cards":{"avatar":avatar,"site":site,"program":programmed,"dying":dying,"tough":minion()},
        "decks":{
            "north":{"avatar":"avatar","atlas":vec!["site";12],
                "spellbook":["program","program","program","program","dying","dying","dying","dying"]},
            "south":{"avatar":"avatar","atlas":vec!["site";12],"spellbook":vec!["tough";12]}
        }
    });
    value["manifestId"] = json!(identity_hash(&value).unwrap());
    canonical_json(&value).unwrap()
}

fn state(session: &Session) -> Value {
    session.replay_value().unwrap()["state"].clone()
}

fn act(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| predicate(&a.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor;
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap()
    else {
        panic!("issued action rejected");
    };
    (descriptor, receipt)
}

fn checkpoint(session: &mut Session) {
    let before = session.replay_value().unwrap();
    *session = resume_game_checkpoint(&create_game_checkpoint(session).unwrap()).unwrap();
    assert_eq!(before, session.replay_value().unwrap());
    assert!(session.verify_replay().unwrap());
}

fn unit(snapshot: &Value, id: &Value) -> Value {
    snapshot["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["instanceId"] == *id)
        .unwrap()
        .clone()
}

fn types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|e| e.event_type.as_str())
        .collect()
}

#[test]
#[allow(clippy::too_many_lines)]
fn authored_genesis_expands_nearby_cohorts_and_resumes_after_ordered_deaths() {
    let mut session = (1..=64)
        .find_map(|seed| {
            let s = Session::new(&manifest(seed)).unwrap();
            let hand = state(&s)["players"]["north"]["hand"]["spellbook"].clone();
            let cards = hand.as_array().unwrap();
            (cards.iter().filter(|c| c["cardId"] == "dying").count() == 2
                && cards.iter().any(|c| c["cardId"] == "program"))
            .then_some(s)
        })
        .expect("opening program and two Deathrites");
    for _ in 0..2 {
        act(&mut session, |d| {
            d["kind"] == "mulligan"
                && d["atlasOrder"] == json!([])
                && d["spellbookOrder"] == json!([])
        });
    }
    let mut dying_ids = Vec::new();
    let mut enemy_ids = Vec::new();
    for round in 0..3 {
        if round > 0 {
            act(&mut session, |d| {
                d["kind"] == "draw" && d["zone"] == "atlas"
            });
        }
        act(&mut session, |d| {
            d["kind"] == "play-site" && d["cell"] == ["C4", "C3", "B3"][round]
        });
        if round == 0 {
            for _ in 0..2 {
                dying_ids.push(
                    act(&mut session, |d| {
                        d["kind"] == "summon-minion" && d["cardId"] == "dying" && d["cell"] == "C4"
                    })
                    .0["cardInstanceId"]
                        .clone(),
                );
            }
        }
        act(&mut session, |d| d["kind"] == "end-turn");
        act(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "atlas"
        });
        let cell = ["C1", "C2", "B2"][round];
        act(&mut session, |d| {
            d["kind"] == "play-site" && d["cell"] == cell
        });
        enemy_ids.push(
            act(&mut session, |d| {
                d["kind"] == "summon-minion" && d["cell"] == cell
            })
            .0["cardInstanceId"]
                .clone(),
        );
        act(&mut session, |d| d["kind"] == "end-turn");
    }
    act(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    act(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "B4"
    });
    let before = state(&session);
    let spell_count = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .unwrap()
        .len();
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"].clone();
    assert_eq!(before["players"]["north"]["avatar"]["tapped"], true);
    let (summon, receipt) = act(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "program" && d["cell"] == "C3"
    });
    assert!(!types(&receipt).contains(&"spell-drawn"));
    let after_damage = state(&session);
    let source_id = &summon["cardInstanceId"];
    assert_eq!(unit(&after_damage, source_id)["damage"], 0);
    assert_eq!(
        unit(&after_damage, &enemy_ids[0])["damage"],
        0,
        "distant enemy"
    );
    assert_eq!(
        unit(&after_damage, &enemy_ids[1])["damage"],
        1,
        "orthogonal enemy"
    );
    assert_eq!(
        unit(&after_damage, &enemy_ids[2])["damage"],
        1,
        "diagonal enemy"
    );
    assert_eq!(after_damage["players"]["north"]["avatar"]["life"], 19);
    assert_eq!(after_damage["players"]["south"]["avatar"]["life"], 20);
    checkpoint(&mut session);
    let (_, death) = act(&mut session, |d| {
        d["kind"] == "order-triggers" && d["sourceInstanceId"] == dying_ids[0]
    });
    assert!(!types(&death).contains(&"spell-drawn"));
    assert_eq!(
        types(&death)
            .iter()
            .filter(|kind| **kind == "site-drawn")
            .count(),
        2
    );
    checkpoint(&mut session);
    let choices = session.legal_actions().unwrap();
    assert!(
        choices
            .iter()
            .any(|a| a.descriptor["kind"] == "choose-ability"
                && a.descriptor["target"]["instanceId"] == avatar_id)
    );
    assert!(
        choices
            .iter()
            .filter(|a| a.descriptor["kind"] == "choose-ability")
            .all(|a| a.descriptor["target"]["seat"] == "north")
    );
    let (_, choice) = act(&mut session, |d| {
        d["kind"] == "choose-ability" && d["target"]["instanceId"] == avatar_id
    });
    assert_eq!(
        types(&choice),
        ["ability-choice-committed", "avatar-untapped", "spell-drawn"]
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["tapped"], false);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .unwrap()
            .len(),
        spell_count
    );
    checkpoint(&mut session);
}
