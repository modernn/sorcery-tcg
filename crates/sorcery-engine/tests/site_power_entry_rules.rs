//! Direct proofs for official power-threshold site entry (RULE-CATALOG-0323–0326).
//!
//! A site can prevent units whose current power is at least a printed threshold
//! from entering. Summoning uses the power the minion would have on that cell
//! or 2x2 footprint. A 2x2 cannot occupy a threshold site just because another
//! cell of the square would be a legal 1x1 destination. Movement uses current
//! derived power, so a later power grant can close an otherwise legal step.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn blocked_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "preventsUnitsWithPowerAtLeastFromEntering": 3,
    })
}

fn minion(attack: u8, extra: Value) -> Value {
    let mut value = json!({
        "attack": attack,
        "cardType": "minion",
        "charge": true,
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn overpower() -> Value {
    json!({
        "cardType": "magic",
        "grantPowerToAllyThisTurn": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn giant(attack: u8) -> Value {
    minion(attack, json!({ "occupiesSquareArea": 2 }))
}

const NORTH_SQUARE: [&str; 4] = ["B3", "B4", "C3", "C4"];

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn printed_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-power-entry-printed" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-site-power-entry-printed-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-blocked": blocked_site(),
            "north-heavy": minion(3, json!({})),
            "north-light": minion(2, json!({})),
            "north-open": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": ["north-blocked", "north-open", "north-blocked", "north-open", "north-blocked", "north-open"],
                "avatar": "north-avatar",
                "spellbook": ["north-heavy", "north-light", "north-heavy", "north-light", "north-heavy", "north-light"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn square_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-power-entry-square" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-site-power-entry-square-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-blocked": blocked_site(),
            "north-heavy": giant(3),
            "north-light": giant(2),
            "north-open": site(),
            "south-avatar": avatar(),
            "south-blocked": blocked_site(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-blocked",
                    "north-open",
                    "north-blocked",
                    "north-open",
                    "north-blocked",
                    "north-open",
                    "north-open",
                    "north-open",
                    "north-open",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-heavy",
                    "north-light",
                    "north-heavy",
                    "north-light",
                    "north-heavy",
                    "north-light",
                    "north-heavy",
                    "north-light",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-blocked",
                    "south-site",
                    "south-blocked",
                    "south-site",
                    "south-blocked",
                    "south-site",
                    "south-site",
                    "south-site",
                ],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn square_step_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-power-entry-square-step" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-site-power-entry-square-step-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-heavy": giant(3),
            "north-open": site(),
            "south-avatar": avatar(),
            "south-blocked": blocked_site(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-open"; 9],
                "avatar": "north-avatar",
                "spellbook": vec!["north-heavy"; 8],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-blocked",
                    "south-site",
                    "south-blocked",
                    "south-site",
                    "south-blocked",
                    "south-site",
                    "south-site",
                    "south-site",
                ],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn derived_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-power-entry-derived" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-site-power-entry-derived-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-blocked": blocked_site(),
            "north-light": minion(1, json!({})),
            "north-open": site(),
            "north-overpower": overpower(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": ["north-open", "north-blocked", "north-open", "north-blocked", "north-open", "north-blocked"],
                "avatar": "north-avatar",
                "spellbook": ["north-light", "north-overpower", "north-light", "north-overpower", "north-light", "north-overpower"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
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

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn printed_opening() -> Session {
    (1..=4096)
        .map(printed_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("printed power-entry candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-blocked".to_owned())
                && atlas.contains(&"north-open".to_owned())
                && spells.contains(&"north-heavy".to_owned())
                && spells.contains(&"north-light".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with both printed power minions")
}

fn card_count(ids: &[String], card_id: &str) -> usize {
    ids.iter().filter(|id| id.as_str() == card_id).count()
}

fn square_summon_opening() -> Session {
    (1..=4096)
        .map(square_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("square summon candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (card_count(&atlas, "north-blocked") >= 1
                && card_count(&atlas, "north-open") >= 2
                && spells.contains(&"north-heavy".to_owned())
                && spells.contains(&"north-light".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with a blocked site and both 2x2 minions")
}

fn square_step_opening() -> Session {
    (1..=4096)
        .map(square_step_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("square step candidate");
            let south_atlas =
                session.replay_value().expect("authoritative replay")["state"]["players"]["south"]
                    ["hand"]["atlas"]
                    .as_array()
                    .expect("south atlas hand")
                    .iter()
                    .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>();
            south_atlas
                .iter()
                .any(|card_id| card_id == "south-blocked")
                .then_some(session)
        })
        .expect("bounded seed opening with an all-open 2x2 square and a south threshold site")
}

fn derived_opening() -> Session {
    (1..=4096)
        .map(derived_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("derived power-entry candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-open".to_owned())
                && atlas.contains(&"north-blocked".to_owned())
                && spells.contains(&"north-light".to_owned())
                && spells.contains(&"north-overpower".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Overpower and a light minion")
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

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn play_named_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn end_and_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn establish_north_square_with_blocked_c4(session: &mut Session) {
    keep(session);
    keep(session);
    play_named_site(session, "north-blocked", "C4");
    end_and_draw(session, "spellbook");
    play_site(session, "C1");
    end_and_draw(session, "spellbook");
    play_named_site(session, "north-open", "B4");
    end_and_draw(session, "spellbook");
    end_and_draw(session, "spellbook");
    play_named_site(session, "north-open", "C3");
    end_and_draw(session, "spellbook");
    end_and_draw(session, "atlas");
    play_site(session, "B3");
}

fn summons_square(card_id: &str) -> impl Fn(&Value) -> bool + '_ {
    move |descriptor: &Value| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == "B3"
            && descriptor["cells"] == json!(NORTH_SQUARE)
    }
}

fn steps_to<'a>(unit_id: &'a str, cell: &'a str) -> impl Fn(&Value) -> bool + 'a {
    move |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == unit_id
            && descriptor["to"]["cell"] == cell
            && descriptor["to"]["region"] == "surface"
    }
}

#[test]
fn rule_catalog_0323_printed_power_cannot_enter_a_threshold_site() {
    let mut session = printed_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-blocked"
            && descriptor["cell"] == "C4"
    });
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-heavy"
            && descriptor["cell"] == "C4"
    }));
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-light"
            && descriptor["cell"] == "C4"
    });
    let light_id = summoned["cardInstanceId"]
        .as_str()
        .expect("light minion identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-open"
            && descriptor["cell"] == "C3"
    });
    let (heavy, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-heavy"
            && descriptor["cell"] == "C3"
    });
    let heavy_id = heavy["cardInstanceId"]
        .as_str()
        .expect("heavy minion identity")
        .to_owned();
    assert!(offers(&session, steps_to(&light_id, "C3")));
    assert!(!offers(&session, steps_to(&heavy_id, "C4")));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0324_derived_power_closes_an_otherwise_legal_entry() {
    let mut session = derived_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-open"
            && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-light"
            && descriptor["cell"] == "C4"
    });
    let light_id = summoned["cardInstanceId"]
        .as_str()
        .expect("light minion identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-blocked"
            && descriptor["cell"] == "C3"
    });
    assert!(offers(&session, steps_to(&light_id, "C3")));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-overpower"
            && descriptor["ally"]["instanceId"] == light_id
    });
    assert!(!offers(&session, steps_to(&light_id, "C3")));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0325_a_two_by_two_cannot_occupy_a_threshold_site() {
    let mut session = square_summon_opening();
    establish_north_square_with_blocked_c4(&mut session);
    assert!(!offers(&session, summons_square("north-heavy")));
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-heavy"
    }));
    let (summoned, _) = accept_where(&mut session, summons_square("north-light"));
    assert_eq!(summoned["cells"], json!(NORTH_SQUARE));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0326_a_two_by_two_cannot_step_onto_a_new_threshold_cell() {
    let mut session = square_step_opening();
    keep(&mut session);
    keep(&mut session);
    play_named_site(&mut session, "north-open", "C4");
    end_and_draw(&mut session, "spellbook");
    play_site(&mut session, "C1");
    end_and_draw(&mut session, "spellbook");
    play_named_site(&mut session, "north-open", "B4");
    end_and_draw(&mut session, "spellbook");
    end_and_draw(&mut session, "spellbook");
    play_named_site(&mut session, "north-open", "C3");
    end_and_draw(&mut session, "spellbook");
    end_and_draw(&mut session, "atlas");
    play_named_site(&mut session, "north-open", "B3");
    let (heavy, _) = accept_where(&mut session, summons_square("north-heavy"));
    let heavy_id = heavy["cardInstanceId"]
        .as_str()
        .expect("heavy 2x2 identity")
        .to_owned();
    end_and_draw(&mut session, "atlas");
    play_named_site(&mut session, "south-blocked", "C2");
    end_and_draw(&mut session, "atlas");
    play_site(&mut session, "B2");
    assert!(
        !offers(&session, steps_to(&heavy_id, "B2")),
        "south newly enters threshold-3 C2"
    );
    assert_exact_replay(&session);
}
