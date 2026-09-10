//! Direct proofs for Flood and Drought terrain Auras (RULE-CATALOG-0266–0267).
//!
//! Official Flood is a persistent 2×2 Aura: affected sites are flooded, so they
//! are Water sites and still provide their other elemental affinities. Official
//! Drought is the later-timestamp inverse: affected sites are not Water sites
//! and provide no Water threshold. Neither uses the 3-turn immobilize machine.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn flood() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drought() -> Value {
    json!({
        "affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn manifest(north_spell: &str, south_spell: &str) -> String {
    let north_card = match north_spell {
        "north-flood" => flood(),
        "north-drought" => drought(),
        _ => panic!("unsupported north spell {north_spell}"),
    };
    let south_card = match south_spell {
        "south-drought" => drought(),
        "south-flood" => flood(),
        "south-minion" => minion(),
        _ => panic!("unsupported south spell {south_spell}"),
    };
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    cards[north_spell] = north_card;
    cards[south_spell] = south_card;
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "terrain-aura" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-terrain-aura-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec![north_spell; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec![south_spell; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step");
    let StepResult::Accepted(receipt) = result else {
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn north_affinity(session: &Session) -> (u64, u64) {
    let view = session
        .public_view(Seat::North)
        .expect("North public view");
    (
        view["players"]["north"]["affinity"]["earth"]
            .as_u64()
            .expect("earth affinity"),
        view["players"]["north"]["affinity"]["water"]
            .as_u64()
            .expect("water affinity"),
    )
}

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

fn cast_covering(session: &mut Session, card_id: &str, cell: &str) -> Value {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == card_id
            && cells_include(descriptor, cell)
    })
    .0
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

fn after_mulligans(north_spell: &str, south_spell: &str) -> Session {
    let mut session = Session::new(&manifest(north_spell, south_spell)).expect("terrain Aura");
    keep(&mut session);
    keep(&mut session);
    session
}

#[test]
fn rule_catalog_0266_flood_adds_water_and_keeps_other_affinities() {
    let mut session = after_mulligans("north-flood", "south-minion");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(north_affinity(&session), (1, 0));
    let flood_casts = session
        .legal_actions()
        .expect("Flood casts")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-aura" && action.descriptor["cardId"] == "north-flood"
        })
        .collect::<Vec<_>>();
    assert!(
        flood_casts
            .iter()
            .all(|action| action.descriptor["cells"].as_array().is_some_and(|cells| cells.len() == 4)),
        "Flood uses the default 2×2 footprint"
    );
    assert!(
        flood_casts
            .iter()
            .any(|action| cells_include(&action.descriptor, "C4")),
        "Flood can cover the played earth site"
    );
    let cast = cast_covering(&mut session, "north-flood", "C4");
    assert!(cells_include(&cast, "C4"));
    assert_eq!(north_affinity(&session), (1, 1));
    let after = state(&session);
    assert!(after["realm"].get("immobileAreas").is_none());
    assert_eq!(after["realm"]["auras"].as_array().expect("auras").len(), 1);
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(after["realm"]["auras"][0]["turnCounters"], 0);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    assert_eq!(north_affinity(&session), (1, 1));
    let persisted = state(&session);
    assert!(persisted["realm"].get("immobileAreas").is_none());
    assert_eq!(persisted["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(persisted["realm"]["auras"][0]["turnCounters"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0267_later_drought_wins_over_flood() {
    let mut session = after_mulligans("north-flood", "south-drought");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    cast_covering(&mut session, "north-flood", "C4");
    assert_eq!(north_affinity(&session), (1, 1));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    cast_covering(&mut session, "south-drought", "C4");
    assert_eq!(north_affinity(&session), (1, 0));
    let after = state(&session);
    assert!(after["realm"].get("immobileAreas").is_none());
    assert_eq!(after["realm"]["auras"].as_array().expect("auras").len(), 2);
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(after["realm"]["auras"][1]["cardId"], "south-drought");
    assert_exact_replay(&session);
}

#[test]
fn later_flood_wins_when_it_enters_after_drought() {
    let mut session = after_mulligans("north-drought", "south-flood");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    cast_covering(&mut session, "north-drought", "C4");
    assert_eq!(north_affinity(&session), (1, 0));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    cast_covering(&mut session, "south-flood", "C4");
    assert_eq!(north_affinity(&session), (1, 1));
    let after = state(&session);
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-drought");
    assert_eq!(after["realm"]["auras"][1]["cardId"], "south-flood");
    assert_exact_replay(&session);
}
