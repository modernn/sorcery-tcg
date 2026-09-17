//! Direct proofs for draw-then-may-play filtered site Magic (RULE-CATALOG-0525–0526,
//! RULE-CATALOG-1102-1103).
//!
//! Ordinary Magic can draw a site and then offer an extra land or water site
//! play. The extra play is independent of the Avatar's once-per-turn site play:
//! it is legal while the Avatar is already tapped and does not tap the Avatar.
//! Printed water is a site whose elements include Water; every other site is land.
//! While Deathrites wait for ordering, the cast stays withheld until the chain
//! drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::game::{Game, IssuedAction};
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

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
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

fn draw_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-draw"
        })
        .count()
}

fn deathrite_filtered_site_play_manifest(seed: u32) -> String {
    let fixture = "filtered-site-play-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_then_play(false),
            "north-earth": site(&["earth"]),
            "north-rain": rain_spell(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-draw",
                    "north-rain",
                    "north-rain",
                    "north-draw",
                    "north-rain",
                    "north-draw",
                ],
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

fn north_has_draw_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-draw", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteFilteredSitePlaySetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_filtered_site_play(
    encoded: &str,
) -> Option<PendingDeathriteFilteredSitePlaySetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_draw_and_rain(&state(&session)) {
        return None;
    }
    if draw_casts(&session) == 0 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteFilteredSitePlaySetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_filtered_site_play_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_filtered_site_play_manifest)
        .find(|candidate| try_pending_deathrite_with_filtered_site_play(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with draw-then-may-play Magic in hand",
        )
}

#[test]
fn rule_catalog_1102_draw_then_may_play_site_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_filtered_site_play_seed_with(1102);
    let mut setup = try_pending_deathrite_with_filtered_site_play(&encoded)
        .expect("complete draw-then-may-play Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert_eq!(draw_casts(session), 0);

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(draw_casts(session) >= 1);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "site-drawn"]);
    let after = state(session);
    assert_eq!(after["phase"], "filtered-site-play");
    assert_eq!(after["pendingFilteredSitePlay"]["water"], false);
    assert_exact_replay(session);
}

fn deathrite_filtered_water_site_play_manifest(seed: u32) -> String {
    let fixture = "filtered-site-play-water-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_then_play(true),
            "north-earth": site(&["earth"]),
            "north-rain": rain_spell(),
            "north-water": site(&["water"]),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
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
                "spellbook": [
                    "north-draw",
                    "north-rain",
                    "north-rain",
                    "north-draw",
                    "north-rain",
                    "north-draw",
                ],
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

fn try_pending_deathrite_with_water_filtered_site_play(
    encoded: &str,
) -> Option<PendingDeathriteFilteredSitePlaySetup> {
    try_pending_deathrite_with_filtered_site_play(encoded)
}

fn deathrite_filtered_water_site_play_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_filtered_water_site_play_manifest)
        .find(|candidate| try_pending_deathrite_with_water_filtered_site_play(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with water draw-then-may-play Magic in hand",
        )
}

#[test]
fn rule_catalog_1103_draw_then_may_play_water_site_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_filtered_water_site_play_seed_with(1103);
    let mut setup = try_pending_deathrite_with_water_filtered_site_play(&encoded)
        .expect("complete water draw-then-may-play Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert_eq!(draw_casts(session), 0);

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(draw_casts(session) >= 1);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "site-drawn"]);
    let after = state(session);
    assert_eq!(after["phase"], "filtered-site-play");
    assert_eq!(after["pendingFilteredSitePlay"]["water"], true);
    assert_exact_replay(session);
}

fn power_bonus_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "otherNearbyAlliesPowerBonus": 1,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn filtered_site_play_interrupt_manifest(seed: u32) -> String {
    let fixture = "filtered-site-play-deathrite-interrupt";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_then_play(false),
            "north-earth": site(&["earth"]),
            "north-rain": rain_spell(),
            "north-water": site(&["water"]),
            "south-aura": power_bonus_minion(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
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
                "spellbook": [
                    "north-draw",
                    "north-rain",
                    "north-rain",
                    "north-draw",
                    "north-rain",
                    "north-draw",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-minion",
                    "south-aura",
                    "south-minion",
                    "south-minion",
                    "south-aura",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn issued_descriptor(action: &IssuedAction) -> Value {
    serde_json::to_value(action.descriptor()).expect("typed descriptor JSON")
}

fn replay_game(session: &Session) -> Game {
    let mut game = Game::from_manifest_json(session.manifest_json()).expect("valid replay game");
    for receipt in session.transcript() {
        let action = game
            .legal_actions()
            .expect("replay legal actions")
            .into_iter()
            .find(|action| {
                action
                    .to_legal_action()
                    .is_ok_and(|action| action.action_id == receipt.action_id)
            })
            .expect("recorded engine-issued action");
        game.apply_action(&action).expect("replay action");
    }
    game
}

fn apply_where(game: &mut Game, predicate: impl Fn(&Value) -> bool) {
    let action = game
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&issued_descriptor(action)))
        .expect("expected engine-issued action");
    game.apply_action(&action).expect("authoritative Game step");
}

struct PendingFilteredSitePlayInterruptSetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_ready_filtered_site_play_interrupt(
    encoded: &str,
) -> Option<PendingFilteredSitePlayInterruptSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let aura = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-aura"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_draw_and_rain(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    let snapshot = state(&session);
    if snapshot["phase"] != "main" {
        return None;
    }
    let aura_id = aura.0["cardInstanceId"].as_str()?.to_owned();
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    if deathrite_ids.iter().any(|instance_id| {
        realm_unit(&snapshot, instance_id).is_none_or(|unit| unit["damage"] != 1)
    }) || realm_unit(&snapshot, &aura_id).is_none_or(|unit| unit["damage"] != 1)
    {
        return None;
    }
    Some(PendingFilteredSitePlayInterruptSetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

fn filtered_site_play_interrupt_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(filtered_site_play_interrupt_manifest)
        .find(|candidate| try_ready_filtered_site_play_interrupt(candidate).is_some())
        .expect("bounded seed that reaches wounded Deathrites after Rain in main")
}

#[test]
fn rule_catalog_1175_filtered_site_play_withheld_during_pending_deathrite_order() {
    let encoded = filtered_site_play_interrupt_seed_with(1175);
    let setup = try_ready_filtered_site_play_interrupt(&encoded)
        .expect("complete filtered-site-play Deathrite interrupt setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    assert_exact_replay(&setup.session);

    let mut control = setup.session.clone();
    try_accept_where(&mut control, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    })
    .expect("draw-then-may-play Magic");
    assert_eq!(state(&control)["phase"], "filtered-site-play");
    assert!(mixed_hand(&control));

    let mut branched = replay_game(&setup.session);
    assert!(
        branched.test_remove_realm_unit(&aura_id),
        "checkpoint branch must drop the power-bonus ally so wounded Deathrites settle"
    );
    apply_where(&mut branched, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    });

    let paused = branched.authoritative_state();
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(
        paused["pendingDeathrites"]["returnPhase"],
        "filtered-site-play"
    );
    assert_eq!(paused["pendingFilteredSitePlay"]["water"], false);
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        branched
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| issued_descriptor(action)["kind"] != "play-site"),
        "deathrite-order must issue no play-site while filtered-site-play stays pending"
    );
    assert!(
        branched
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| issued_descriptor(action)["kind"] != "decline-filtered-site-play"),
        "deathrite-order must issue no decline-filtered-site-play while filtered-site-play stays pending"
    );

    let order_sources: Vec<_> = branched
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| issued_descriptor(action)["kind"] == "order-deathrites")
        .map(|action| {
            issued_descriptor(&action)["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);
    apply_where(&mut branched, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = branched.authoritative_state();
    assert_eq!(resumed["phase"], "filtered-site-play");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        branched
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| issued_descriptor(action)["kind"] == "decline-filtered-site-play"),
        "filtered-site-play must return once deathrite-order clears"
    );
    assert!(!play_site_card_ids_from_game(&branched).is_empty());
}

fn play_site_card_ids_from_game(game: &Game) -> Vec<String> {
    let mut ids: Vec<_> = game
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter_map(|action| {
            let descriptor = issued_descriptor(&action);
            (descriptor["kind"] == "play-site").then(|| {
                descriptor["cardId"]
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
