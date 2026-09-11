//! Direct proofs for official power-threshold site entry (RULE-CATALOG-0323–0324).
//!
//! A site can prevent units whose current power is at least a printed threshold
//! from entering. Summoning uses the power the minion would have on that cell.
//! Movement uses current derived power, so a later power grant can close an
//! otherwise legal step.

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
