//! Direct proofs for draw-then-may-play filtered site Magic (RULE-CATALOG-0525–0526).
//!
//! Ordinary Magic can draw a site and then offer an extra land or water site
//! play. The extra play is independent of the Avatar's once-per-turn site play:
//! it is legal while the Avatar is already tapped and does not tap the Avatar.
//! Printed water is a site whose elements include Water; every other site is land.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn site(elements: &[&str]) -> Value {
    json!({
        "cardType": "site",
        "elements": elements,
    })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn draw_then_play(water: bool) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value[if water {
        "drawSiteThenMayPlayWaterSite"
    } else {
        "drawSiteThenMayPlayLandSite"
    }] = json!(true);
    value
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32, water: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "filtered-site-play" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-filtered-site-play-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_then_play(water),
            "north-earth": site(&["earth"]),
            "north-water": site(&["water"]),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": vec!["north-draw"; 6],
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
    }))
}

fn try_accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    try_accept_where(session, predicate).expect("expected engine-issued action")
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid filtered-site-play session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn hand_card_ids(session: &Session) -> Vec<String> {
    state(session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand")
        .iter()
        .map(|card| {
            card["cardId"]
                .as_str()
                .expect("hand card identity")
                .to_owned()
        })
        .collect()
}

fn mixed_hand(session: &Session) -> bool {
    let ids = hand_card_ids(session);
    ids.iter().any(|id| id == "north-earth") && ids.iter().any(|id| id == "north-water")
}

fn pending_after_cast(water: bool, seed: u32) -> Option<Session> {
    let mut session = opening_main(&manifest(seed, water));
    if !state(&session)["players"]["north"]["avatar"]["tapped"]
        .as_bool()
        .expect("opening Avatar tap")
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    })?;
    let after = state(&session);
    if after["phase"] != "filtered-site-play" || !mixed_hand(&session) {
        return None;
    }
    Some(session)
}

fn seed_with_mixed_hand(water: bool, start: u32) -> (u32, Session) {
    (start..start + 256)
        .find_map(|seed| pending_after_cast(water, seed).map(|session| (seed, session)))
        .unwrap_or_else(|| panic!("no mixed land/water hand in seeds {start}.."))
}

fn play_site_card_ids(session: &Session) -> Vec<String> {
    let mut ids: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter_map(|action| {
            (action.descriptor["kind"] == "play-site").then(|| {
                action.descriptor["cardId"]
                    .as_str()
                    .expect("played site identity")
                    .to_owned()
            })
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
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

#[test]
fn rule_catalog_0525_land_draw_offers_an_extra_untapped_land_site_play() {
    let (_seed, mut session) = seed_with_mixed_hand(false, 525);
    assert_eq!(
        event_types(session.transcript().last().expect("cast receipt")),
        ["magic-cast", "site-drawn"]
    );
    let after_cast = state(&session);
    assert_eq!(after_cast["phase"], "filtered-site-play");
    assert_eq!(after_cast["pendingFilteredSitePlay"]["water"], false);
    assert_eq!(after_cast["players"]["north"]["avatar"]["tapped"], true);
    let offered = play_site_card_ids(&session);
    assert_eq!(offered, ["north-earth"]);
    assert!(
        session
            .legal_actions()
            .expect("legal actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "decline-filtered-site-play")
    );
    let before_play = after_cast.clone();
    let (played, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "north-earth"
    });
    assert_eq!(event_types(&receipt), ["site-played", "magic-resolved"]);
    assert_ne!(played["cell"], "C4");
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["pendingFilteredSitePlay"], Value::Null);
    assert_eq!(after["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(
        after["players"]["north"]["mana"],
        before_play["players"]["north"]["mana"]
            .as_u64()
            .expect("mana before extra play")
            + 1
    );
    assert_eq!(
        after["realm"]["sites"][played["cell"].as_str().expect("played cell")]["cardId"],
        "north-earth"
    );
    assert!(hand_card_ids(&session).iter().any(|id| id == "north-water"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0526_water_draw_excludes_land_and_decline_keeps_the_site() {
    let (_seed, mut session) = seed_with_mixed_hand(true, 526);
    assert_eq!(
        event_types(session.transcript().last().expect("cast receipt")),
        ["magic-cast", "site-drawn"]
    );
    let after_cast = state(&session);
    assert_eq!(after_cast["phase"], "filtered-site-play");
    assert_eq!(after_cast["pendingFilteredSitePlay"]["water"], true);
    assert_eq!(after_cast["players"]["north"]["avatar"]["tapped"], true);
    let offered = play_site_card_ids(&session);
    assert_eq!(offered, ["north-water"]);
    let before_hand = hand_card_ids(&session);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-filtered-site-play"
    });
    assert_eq!(event_types(&receipt), ["magic-resolved"]);
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["pendingFilteredSitePlay"], Value::Null);
    assert_eq!(after["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(
        after["players"]["north"]["mana"],
        after_cast["players"]["north"]["mana"]
    );
    assert_eq!(hand_card_ids(&session), before_hand);
    assert!(hand_card_ids(&session).iter().any(|id| id == "north-water"));
    assert_exact_replay(&session);
}
