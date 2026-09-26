//! Ordinary location choices preserve region, group entry, and deterministic continuations.
use serde_json::{Value, json};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::selfplay_manifest_with;

fn manifest(relation: &str) -> String {
    selfplay_manifest_with(8311, |m| {
        let thresholds = json!({"earth":0,"fire":0,"water":0,"air":0});
        m["cards"] = json!({
            "avatar": {"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "land": {"cardType":"site","elements":["earth"]},
            "water": {"cardType":"site","elements":["water"]},
            "token": {"cardType":"minion","attack":0,"defense":0,"manaCost":null,"thresholds":thresholds,"token":true,"submerge":true},
            "spell": {"cardType":"magic","manaCost":0,"thresholds":thresholds,"effectProgram":{"effects":[
                {"op":"choose-location","relation":relation},
                {"op":"summon-token","token":"token","count":7,"destination":"chosen-location"},
                {"op":"draw","zone":"spellbook","count":1}
            ]}}
        });
        for (seat, site) in [("north", "land"), ("south", "water")] {
            m["decks"][seat] =
                json!({"avatar":"avatar","atlas":vec![site;12],"spellbook":vec!["spell";8]});
        }
    })
}
fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine-issued action was rejected");
    };
    (descriptor, receipt)
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay state")["state"].clone()
}

fn assert_checkpoint_and_replay(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("checkpoint");
    let encoded = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let restored =
        resume_game_checkpoint(&parse_game_checkpoint(&encoded).expect("parsed checkpoint"))
            .expect("restored checkpoint");
    assert_eq!(
        restored.replay_value().expect("restored replay"),
        session.replay_value().expect("replay")
    );

    let action_ids = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect::<Vec<_>>();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed state"),
        session.replay_value().expect("state")
    );
    assert_eq!(replayed.transcript(), session.transcript());
}

fn ready(seat: &str, relation: &str) -> Session {
    let mut session = Session::new(&manifest(relation)).expect("location program admitted");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C4"
    });
    accept_where(&mut session, |d| d["kind"] == "end-turn");
    accept_where(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept_where(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C1"
    });
    if seat == "north" {
        accept_where(&mut session, |d| d["kind"] == "end-turn");
        accept_where(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "atlas"
        });
    }
    session
}

#[test]
fn ordinary_location_choice_groups_tokens_across_regions_then_draws_with_exact_replay() {
    for seat in ["north", "south"] {
        let mut root = ready(seat, "anywhere");
        let (_, cast) = accept_where(&mut root, |d| d["kind"] == "cast-magic");
        assert!(
            !cast
                .events
                .iter()
                .any(|e| e.event_type == "minion-summoned")
        );
        assert_checkpoint_and_replay(&root);
        let choices = root.legal_actions().expect("issued location choices");
        assert_eq!(
            choices.len(),
            22,
            "two sites have two locations each; eighteen void locations"
        );
        for (cell, region, survivors) in [
            ("C1", "underwater", 7),
            ("C4", "underground", 0),
            ("A1", "void", 0),
        ] {
            let mut branch = root.clone();
            let (_, receipt) = accept_where(&mut branch, |d| {
                d["kind"] == "choose-ability-location"
                    && d["location"] == json!({"cell":cell,"region":region})
            });
            let entered = receipt
                .events
                .iter()
                .filter(|e| e.event_type == "minion-summoned")
                .collect::<Vec<_>>();
            assert_eq!(entered.len(), 7, "one choice creates the entire group");
            for event in &entered {
                assert_eq!(event.payload["cell"], cell);
                assert_eq!(event.payload["region"], region);
                assert_eq!(event.payload["manaPaid"], 0);
            }
            let last_entry = receipt
                .events
                .iter()
                .rposition(|e| e.event_type == "minion-summoned")
                .expect("group entry");
            let draw = receipt
                .events
                .iter()
                .position(|e| e.event_type == "spell-drawn")
                .expect("independent draw");
            assert!(last_entry < draw);
            assert_eq!(
                state(&branch)["realm"]["units"]
                    .as_array()
                    .expect("units")
                    .len(),
                survivors
            );
            assert_eq!(state(&branch)["phase"], "main");
            assert_checkpoint_and_replay(&branch);
        }
        assert_eq!(
            state(&root)["phase"],
            "ability-choice",
            "branches leave pending root intact"
        );
    }
}

#[test]
fn nearby_location_choice_uses_source_geometry_and_region() {
    let mut session = ready("north", "nearby");
    accept_where(&mut session, |d| d["kind"] == "cast-magic");
    let choices = session.legal_actions().expect("local choices");
    assert_eq!(choices.len(), 1);
    assert_eq!(
        choices[0].descriptor["location"],
        json!({"cell":"C4","region":"surface"})
    );
    accept_where(&mut session, |d| d["kind"] == "choose-ability-location");
    assert_checkpoint_and_replay(&session);
}
