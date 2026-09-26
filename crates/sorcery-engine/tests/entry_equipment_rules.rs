//! Direct proofs for minion entry equipment, including oversized bearers and Genesis order.

use serde_json::{Value, json};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::ActionRequest;
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::selfplay_manifest_with;

fn manifest(oversized: bool) -> String {
    selfplay_manifest_with(9401, |m| {
        let thresholds = json!({"air":0,"earth":0,"fire":0,"water":0});
        m["cards"] = json!({
            "avatar": {"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "site": {"cardType":"site","elements":["earth"]},
            "token": {"cardType":"artifact","manaCost":null,"thresholds":thresholds,"token":true,
                "bearerUnitStrike":{"damageBonus":1}},
            "bearer": {"cardType":"minion","attack":1,"defense":4,"manaCost":0,
                "thresholds":thresholds,"entersCarrying":["token","token"],
                "genesisProgram":{"effects":[{"op":"conjure-token","token":"token","count":1,"destination":"source"}]}},
        });
        if oversized {
            m["cards"]["bearer"]["occupiesSquareArea"] = json!(2);
        }
        for seat in ["north", "south"] {
            m["decks"][seat] = json!({
                "avatar":"avatar", "atlas":vec!["site";12],
            "spellbook":vec!["bearer","bearer","bearer","bearer","bearer","bearer"]
            });
        }
    })
}

fn accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> (Value, sorcery_engine::contract::Receipt) {
    let actions = session.legal_actions().expect("legal actions");
    let action = actions
        .iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "engine-issued action; got {:?}",
                actions.iter().map(|a| &a.descriptor).collect::<Vec<_>>()
            )
        });
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine action rejected")
    };
    (descriptor, receipt)
}

fn keep(session: &mut Session) {
    accept_where(session, |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
}

fn next_turn(session: &mut Session) {
    accept_where(session, |d| d["kind"] == "end-turn");
    accept_where(session, |d| d["kind"] == "draw" && d["zone"] == "atlas");
}

fn ready(oversized: bool) -> Session {
    let mut session = Session::new(&manifest(oversized)).expect("manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C4"
    });
    next_turn(&mut session);
    accept_where(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C1"
    });
    next_turn(&mut session);
    if oversized {
        for cell in ["B4", "B3", "C3"] {
            accept_where(&mut session, |d| {
                d["kind"] == "play-site" && d["cell"] == cell
            });
            if cell != "C3" {
                next_turn(&mut session);
                next_turn(&mut session);
            }
        }
    }
    session
}

fn checkpoint_replay(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("checkpoint");
    let encoded = serialize_game_checkpoint(&checkpoint).expect("serialize");
    let resumed =
        resume_game_checkpoint(&parse_game_checkpoint(&encoded).expect("parse")).expect("resume");
    assert_eq!(
        resumed.replay_value().expect("resumed replay"),
        session.replay_value().expect("replay")
    );
    let actions = session
        .transcript()
        .iter()
        .map(|r| r.action_id.clone())
        .collect::<Vec<_>>();
    let replayed = Session::replay(session.manifest_json(), &actions).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed"),
        session.replay_value().expect("state")
    );
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay")["state"].clone()
}

#[test]
fn paid_entry_equipment_precedes_genesis_and_creates_distinct_tokens_for_both_seats() {
    for seat in ["north", "south"] {
        let mut session = ready(false);
        if seat == "south" {
            next_turn(&mut session);
        }
        let (_, receipt) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion" && d["cardId"] == "bearer"
        });
        let current = state(&session);
        let artifacts = current["realm"]["artifacts"].as_array().expect("artifacts");
        assert_eq!(artifacts.len(), 3);
        let ids = artifacts
            .iter()
            .map(|a| a["instanceId"].as_str().expect("id"))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), 3);
        assert_eq!(
            artifacts.iter().filter(|a| a["bearer"].is_object()).count(),
            2
        );
        assert_eq!(
            artifacts
                .iter()
                .filter(|a| a.get("bearer").is_none())
                .count(),
            1
        );
        let conjured = receipt
            .events
            .iter()
            .filter(|e| e.event_type == "artifact-conjured")
            .map(|e| e.payload["bearer"].is_object())
            .collect::<Vec<_>>();
        assert_eq!(
            conjured,
            [true, true, false],
            "entry equipment precedes Genesis"
        );
        checkpoint_replay(&session);
    }
}

#[test]
fn oversized_entry_equipment_keeps_each_token_at_an_independent_footprint_cell() {
    let mut session = ready(true);
    accept_where(&mut session, |d| {
        d["kind"] == "summon-minion"
            && d["cardId"] == "bearer"
            && d["cells"] == json!(["B3", "B4", "C3", "C4"])
    });
    assert!(state(&session)["realm"].get("artifacts").is_none());
    for (index, cell) in ["B3", "C4", "C3"].into_iter().enumerate() {
        checkpoint_replay(&session);
        let choices = session.legal_actions().unwrap();
        assert_eq!(choices.len(), 4);
        assert!(
            choices
                .iter()
                .all(|a| a.descriptor["kind"] == "choose-ability-location")
        );
        accept_where(&mut session, |d| {
            d["kind"] == "choose-ability-location"
                && d["location"] == json!({"cell":cell,"region":"surface"})
        });
        let current = state(&session);
        let artifacts = current["realm"]["artifacts"].as_array().unwrap();
        assert_eq!(artifacts.len(), index + 1);
        let field = if index < 2 { "bearerCell" } else { "location" };
        let created = artifacts.iter().find(|a| a[field] == cell).unwrap();
        assert_eq!(created["bearer"].is_object(), index < 2);
    }
    checkpoint_replay(&session);
}
