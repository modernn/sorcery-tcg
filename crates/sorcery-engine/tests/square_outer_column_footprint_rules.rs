//! Direct proofs for a 2×2 minion that combines `occupiesSquareArea: 2` with
//! `mustBeCastToOuterColumn` (RULE-CATALOG-0375–0376).

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::ActionRequest;
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

fn earth() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn giant() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 10,
        "manaCost": 0,
        "mustBeCastToOuterColumn": true,
        "occupiesSquareArea": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn charm() -> Value {
    json!({
        "bearerControllerChoosesExtraRandomOutcome": true,
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 9,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "square-outer-column-footprint" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-square-outer-column-footprint-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charm": charm(),
            "north-earth": earth(),
            "north-giant": giant(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-giant",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-raider"; 16],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Value {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            let current = session.replay_value().expect("replay");
            panic!(
                "expected engine-issued action in phase {} among {:?}",
                current["state"]["phase"],
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
        });
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(_) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine-issued action must be accepted");
    };
    descriptor
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn play_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn end_and_draw_zone(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn end_and_draw(session: &mut Session) {
    end_and_draw_zone(session, "spellbook");
}

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> Option<&'a Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn giant_summons(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-giant"
        })
        .map(|action| action.descriptor)
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
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

fn opening() -> Session {
    (1..=4096)
        .map(manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("outer-column footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 3
                && spells.contains(&"north-charm".to_owned())
                && spells.contains(&"north-giant".to_owned()))
            .then_some(session)
        })
        .expect(
            "bounded seed opening with earth sites, filler artifact, and the outer-column giant",
        )
}

fn establish_a3_square(session: &mut Session) {
    keep(session);
    keep(session);
    play_site(session, "north-earth", "C4");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-charm"
            && descriptor["bearer"]["kind"] == "avatar"
    });
    end_and_draw(session);
    play_site(session, "south-site", "C1");
    end_and_draw(session);
    play_site(session, "north-earth", "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "north-earth", "B3");
}

fn south_summon(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == cell
    });
}

fn finish_a3_square(session: &mut Session) {
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "A4");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "A3");
}

fn setup_to_a3_summon() -> Session {
    let mut session = opening();
    establish_a3_square(&mut session);
    end_and_draw(&mut session);
    south_summon(&mut session, "B3");
    finish_a3_square(&mut session);
    session
}

#[test]
fn rule_catalog_0375_outer_column_giant_summons_to_an_outer_column_anchor() {
    let mut session = setup_to_a3_summon();

    let summons = giant_summons(&session);
    assert!(
        !summons.is_empty(),
        "north should be able to summon the giant"
    );
    assert!(summons.iter().all(|descriptor| {
        descriptor["cell"]
            .as_str()
            .is_some_and(|cell| cell.starts_with('A') || cell.starts_with('E'))
    }));
    assert!(summons.iter().any(|descriptor| {
        descriptor["cell"] == "A3"
            && descriptor["cells"] == json!(["A3", "A4", "B3", "B4"])
            && descriptor["region"].is_null()
    }));
    assert!(!summons.iter().any(|descriptor| descriptor["cell"] == "B3"));

    let summoned = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "A3"
            && descriptor["cells"] == json!(["A3", "A4", "B3", "B4"])
    });
    let giant_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    let current = state(&session);
    let occupant = realm_unit(&current, &giant_id).expect("giant remains in play");
    assert_eq!(occupant["location"], "A3");
    assert_eq!(occupant["occupiedCells"], json!(["A3", "A4", "B3", "B4"]));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0376_outer_column_giant_movement_is_not_cast_restricted() {
    let mut session = opening();
    establish_a3_square(&mut session);
    end_and_draw(&mut session);
    south_summon(&mut session, "C1");
    finish_a3_square(&mut session);

    let summoned = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "A3"
    });
    let giant_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();

    end_and_draw(&mut session);
    end_and_draw_zone(&mut session, "atlas");
    play_site(&mut session, "north-earth", "C3");
    end_and_draw(&mut session);
    end_and_draw(&mut session);

    let inward = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == giant_id.as_str()
            && descriptor["to"]["cell"] == "B3"
    };
    assert!(
        session
            .legal_actions()
            .expect("legal actions")
            .iter()
            .any(|action| inward(&action.descriptor)),
        "the cast restriction must not block stepping the footprint inward"
    );

    accept_where(&mut session, inward);
    if session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| action.descriptor["kind"] == "decline-attack")
    {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "decline-attack"
        });
    }

    let current = state(&session);
    let moved = realm_unit(&current, &giant_id).expect("giant after movement");
    assert_eq!(moved["location"], "B3");
    assert_eq!(moved["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_exact_replay(&session);
}
