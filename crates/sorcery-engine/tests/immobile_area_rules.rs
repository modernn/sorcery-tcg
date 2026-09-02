use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    })
}

fn site(extra: Value) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
    });
    let Value::Object(extra) = extra else {
        panic!("extra site facts must be an object");
    };
    value.as_object_mut().expect("site facts").extend(extra);
    value
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "immobile-area-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-immobile-area-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-minion": minion(),
            "north-site": site(json!({})),
            "north-trap": site(json!({ "genesisImmobilizeNearbyUntilNextTurn": true })),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-site",
                    "north-trap",
                    "north-site",
                    "north-site",
                    "north-site",
                    "north-site",
                ],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
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
        panic!("engine-issued action must be accepted");
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

fn draw_atlas(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn play_site(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("site identity")
        .to_owned()
}

fn summon(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("minion identity")
        .to_owned()
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn immobile_areas(session: &Session) -> Value {
    state(session)["realm"]["immobileAreas"].clone()
}

fn move_options(session: &Session, instance_id: &str) -> Vec<(String, String)> {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .map(|action| {
            (
                action.descriptor["from"]["cell"]
                    .as_str()
                    .expect("movement origin")
                    .to_owned(),
                action.descriptor["to"]["cell"]
                    .as_str()
                    .expect("movement destination")
                    .to_owned(),
            )
        })
        .collect()
}

fn can_relocate(session: &Session, instance_id: &str) -> bool {
    move_options(session, instance_id)
        .into_iter()
        .any(|(from, to)| from != to)
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0063_site_genesis_should_immobilize_nearby_units_until_its_controllers_next_turn() {
    let mut session = Session::new(&manifest(630)).expect("valid immobilize scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "north-site", "C4");
    let north_minion = summon(&mut session, "north-minion", "C4");
    end_turn(&mut session);

    draw_atlas(&mut session);
    play_site(&mut session, "south-site", "C1");
    let south_minion = summon(&mut session, "south-minion", "C1");
    end_turn(&mut session);

    draw_atlas(&mut session);
    play_site(&mut session, "north-site", "C3");
    end_turn(&mut session);

    draw_atlas(&mut session);
    play_site(&mut session, "south-site", "C2");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == south_minion
            && descriptor["from"]["cell"] == "C1"
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    end_turn(&mut session);

    draw_atlas(&mut session);
    assert_eq!(immobile_areas(&session), Value::Null);
    assert!(can_relocate(&session, &north_minion));
    let trap = play_site(&mut session, "north-trap", "B3");
    assert_eq!(
        immobile_areas(&session),
        json!([{
            "cells": ["B3", "C2", "C3", "C4"],
            "expiresAtSeat": "north",
            "sourceInstanceId": trap,
        }])
    );
    assert_eq!(
        move_options(&session, &north_minion),
        [("C4".to_owned(), "C4".to_owned())],
        "an immobilized minion keeps only its stand-and-fight option"
    );
    end_turn(&mut session);

    draw_atlas(&mut session);
    assert_eq!(
        immobile_areas(&session)[0]["sourceInstanceId"],
        json!(trap),
        "the area must survive into the immobilized opponent's turn"
    );
    assert!(!can_relocate(&session, &south_minion));
    end_turn(&mut session);

    draw_atlas(&mut session);
    assert_eq!(immobile_areas(&session), Value::Null);
    assert!(can_relocate(&session, &north_minion));
    assert_exact_replay(&session);
}
