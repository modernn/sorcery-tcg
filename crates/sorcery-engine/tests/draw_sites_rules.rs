//! Direct proofs for targetless draw-site Magic (RULE-CATALOG-0647–0648,
//! RULE-CATALOG-1044, RULE-CATALOG-2203–2208).
//!
//! Draw-site Magic pays, draws the printed number of hidden Atlas cards in
//! deck order, and enters its owner's cemetery. Drawing from an empty Atlas
//! loses immediately after any partial draws. While Deathrites wait for
//! ordering, draw-site Magic stays withheld until the chain drains.
//! Supplemental proofs cover persistence, empty-library deck-out repeat,
//! enemy-arrival, targetless offering, the opponent's Atlas, and newly
//! remaining library cards.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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
    json!({
        "cardType": "site",
        "elements": ["earth"],
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

fn draw_site() -> Value {
    json!({
        "cardType": "magic",
        "drawSites": 2,
        "manaCost": 1,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_sites_manifest(seed: u32, atlas_count: usize) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "draw-sites-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-draw-sites-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_site(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; atlas_count],
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid draw-sites session");
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
fn rule_catalog_0647_draw_sites_magic_draws_hidden_atlas_cards() {
    let encoded = draw_sites_manifest(647, 6);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let expected: Vec<_> = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .iter()
        .take(2)
        .map(|card| card["instanceId"].clone())
        .collect();
    assert_eq!(expected.len(), 2);
    let south_before = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_before["players"]["north"]["hand"]["atlas"], 2);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    });
    let spell_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("draw Magic identity")
        .to_owned();
    assert!(descriptor.get("target").is_none());
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-drawn", "site-drawn", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], spell_id);
    let after = state(&session);
    let hand = after["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand");
    assert_eq!(hand.len(), 4);
    for instance_id in &expected {
        assert!(
            hand.iter().any(|card| &card["instanceId"] == instance_id),
            "the drawn identities must enter the hidden Atlas hand"
        );
    }
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    let south_after = session
        .public_view(Seat::South)
        .expect("South public view after the draws");
    assert_eq!(
        south_after["players"]["north"]["hand"]["atlas"], 4,
        "the opponent sees only the new Atlas hand count"
    );
    assert_eq!(south_after["players"]["north"]["atlasCount"], 1);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("draw-sites checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized draw-sites");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed draw-sites");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed draw-sites session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0648_draw_sites_magic_exhausts_then_loses_on_empty_library() {
    for remaining in 0..=1 {
        let encoded = draw_sites_manifest(
            648 + u32::try_from(remaining).expect("small remaining count"),
            3 + remaining,
        );
        let mut session = opening_main(&encoded);
        let before = state(&session);
        let expected = before["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .clone();
        assert_eq!(expected.len(), remaining);
        let (_, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
        });
        let kinds = event_types(&receipt);
        assert_eq!(kinds.first(), Some(&"magic-cast"));
        assert_eq!(
            kinds.iter().filter(|kind| **kind == "site-drawn").count(),
            expected.len()
        );
        assert_eq!(kinds.last(), Some(&"game-ended"));
        assert!(kinds.contains(&"magic-resolved"));
        let after = state(&session);
        assert_eq!(after["players"]["north"]["atlas"], json!([]));
        assert_eq!(after["terminal"]["reason"], "deck_empty");
        assert_eq!(after["terminal"]["loser"], "north");
        for card in expected {
            assert!(
                after["players"]["north"]["hand"]["atlas"]
                    .as_array()
                    .expect("north Atlas hand")
                    .contains(&card)
            );
        }
        assert_exact_replay(&session);
    }
}

fn draw_sites_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("draw-sites actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-draw"
        })
        .count()
}

fn deathrite_draw_sites_manifest(seed: u32) -> String {
    let fixture = "draw-sites-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_site(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
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

struct PendingDeathriteDrawSitesSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_draw_sites_magic(
    encoded: &str,
) -> Option<PendingDeathriteDrawSitesSetup> {
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
    if draw_sites_casts(&session) == 0 {
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
    Some(PendingDeathriteDrawSitesSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_draw_sites_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_draw_sites_manifest)
        .find(|candidate| try_pending_deathrite_with_draw_sites_magic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with draw-sites Magic in hand")
}

#[test]
fn rule_catalog_1044_draw_sites_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_draw_sites_seed_with(1044);
    let mut setup = try_pending_deathrite_with_draw_sites_magic(&encoded)
        .expect("complete draw-sites Deathrite withheld setup");
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
    assert_eq!(draw_sites_casts(session), 0);

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
    assert!(draw_sites_casts(session) >= 1);

    let before = state(session);
    let expected: Vec<_> = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .iter()
        .take(2)
        .map(|card| card["instanceId"].clone())
        .collect();
    let (descriptor, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    });
    let spell_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("draw Magic identity")
        .to_owned();
    assert!(descriptor.get("target").is_none());
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-drawn", "site-drawn", "magic-resolved"]
    );
    let after = state(session);
    let hand = after["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand");
    for instance_id in &expected {
        assert!(
            hand.iter().any(|card| &card["instanceId"] == instance_id),
            "the drawn identities must enter the hidden Atlas hand"
        );
    }
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    assert_exact_replay(session);
}

fn supplemental_draw_site() -> Value {
    let mut value = draw_site();
    value["manaCost"] = json!(0);
    value
}

fn draw_sites_supplemental_manifest(seed: u32, north_atlas: usize) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "draw-sites-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-draw-sites-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": supplemental_draw_site(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; north_atlas],
                "avatar": "north-avatar",
                "spellbook": vec!["north-draw"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_hand_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("opening Spellbook hand")
        .iter()
        .map(|card| {
            card["cardId"]
                .as_str()
                .expect("hand card identity")
                .to_owned()
        })
        .collect()
}

fn supplemental_seed_with_start(start: u32, north_atlas: usize, required_draws: usize) -> String {
    (start..start + 2048)
        .chain(647..647 + 2048)
        .map(|seed| draw_sites_supplemental_manifest(seed, north_atlas))
        .find(|candidate| {
            opening_hand_spell_ids(candidate)
                .iter()
                .filter(|card| *card == "north-draw")
                .count()
                >= required_draws
        })
        .expect("bounded seed with draw-sites Magic in the opening hand")
}

fn draw_sites_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-draw")
                .count()
        })
        .unwrap_or_default()
}

fn atlas_ids(session: &Session, seat: &str) -> Vec<String> {
    state(session)["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("atlas identity")
                .to_owned()
        })
        .collect()
}

fn atlas_hand_ids(session: &Session, seat: &str) -> Vec<String> {
    state(session)["players"][seat]["hand"]["atlas"]
        .as_array()
        .expect("atlas hand")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("atlas hand identity")
                .to_owned()
        })
        .collect()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .is_ok_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn decline_attack_if_needed(session: &mut Session) {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

fn end_turn_if_offered(session: &mut Session) {
    decline_attack_if_needed(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn cast_draw(session: &mut Session) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    })
}

fn drawn_count(receipt: &Receipt) -> usize {
    event_types(receipt)
        .iter()
        .filter(|event| **event == "site-drawn")
        .count()
}

fn try_second_draw_enemy_arrival_prefix(encoded: &str) -> Option<(Session, Vec<String>)> {
    let mut session = opening_main(encoded);
    if draw_sites_in_hand(&state(&session)) < 2 {
        return None;
    }
    let (_, first) = cast_draw(&mut session);
    if drawn_count(&first) != 2 {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if state(&session)["realm"]["sites"]["C1"].is_null() {
        return None;
    }
    if draw_sites_in_hand(&state(&session)) < 1 {
        return None;
    }
    let remaining = atlas_ids(&session, "north");
    (remaining.len() >= 2).then_some((session, remaining))
}

fn seed_for_second_draw_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(647..647 + 8192)
        .find_map(|seed| {
            let encoded = draw_sites_supplemental_manifest(seed, 24);
            try_second_draw_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second draw-sites enemy-arrival setup")
}

#[test]
fn rule_catalog_2203_drawn_atlas_cards_stay_in_hand_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2203, 24, 1);
    let mut session = opening_main(&encoded);
    let expected: Vec<_> = atlas_ids(&session, "north").into_iter().take(2).collect();
    assert_eq!(expected.len(), 2);
    let (_, receipt) = cast_draw(&mut session);
    assert_eq!(drawn_count(&receipt), 2);
    for instance_id in &expected {
        assert!(atlas_hand_ids(&session, "north").contains(instance_id));
        assert!(!atlas_ids(&session, "north").contains(instance_id));
    }
    pass_turn_to_north_spellbook(&mut session);
    for instance_id in &expected {
        assert!(atlas_hand_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2204_second_draw_on_an_empty_atlas_decks_out() {
    let encoded = supplemental_seed_with_start(2204, 5, 2);
    let mut session = opening_main(&encoded);
    assert_eq!(atlas_ids(&session, "north").len(), 2);
    assert!(draw_sites_in_hand(&state(&session)) >= 2);
    let (_, first) = cast_draw(&mut session);
    assert_eq!(drawn_count(&first), 2);
    assert_eq!(atlas_ids(&session, "north").len(), 0);
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert!(draw_sites_in_hand(&state(&session)) >= 1);
    let (_, second) = cast_draw(&mut session);
    let kinds = event_types(&second);
    assert_eq!(kinds.first(), Some(&"magic-cast"));
    assert_eq!(drawn_count(&second), 0);
    assert_eq!(kinds.last(), Some(&"game-ended"));
    assert!(kinds.contains(&"magic-resolved"));
    let after = state(&session);
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_eq!(after["terminal"]["loser"], "north");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2205_second_draw_still_draws_after_enemy_site_placement() {
    let encoded = seed_for_second_draw_enemy_arrival(2205);
    let (mut session, remaining) = try_second_draw_enemy_arrival_prefix(&encoded)
        .expect("second draw-sites enemy-arrival prefix");
    let expected: Vec<_> = remaining.into_iter().take(2).collect();
    let (_, receipt) = cast_draw(&mut session);
    assert_eq!(drawn_count(&receipt), 2);
    for instance_id in &expected {
        assert!(atlas_hand_ids(&session, "north").contains(instance_id));
        assert!(!atlas_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2206_draw_sites_is_targetless_and_draws_printed_cards() {
    let encoded = supplemental_seed_with_start(2206, 24, 1);
    let mut session = opening_main(&encoded);
    let draw_actions: Vec<_> = session
        .legal_actions()
        .expect("draw-sites actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-draw"
        })
        .collect();
    assert!(!draw_actions.is_empty());
    assert_eq!(draw_actions.len(), draw_sites_in_hand(&state(&session)));
    assert!(
        draw_actions
            .iter()
            .all(|action| action.descriptor.get("target").is_none())
    );
    let expected: Vec<_> = atlas_ids(&session, "north").into_iter().take(2).collect();
    let (descriptor, receipt) = cast_draw(&mut session);
    assert!(descriptor.get("target").is_none());
    assert_eq!(drawn_count(&receipt), 2);
    for instance_id in &expected {
        assert!(atlas_hand_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2207_draw_sites_leaves_the_opponent_atlas_untouched() {
    let encoded = supplemental_seed_with_start(2207, 24, 1);
    let mut session = opening_main(&encoded);
    let north_before = atlas_ids(&session, "north");
    let south_before = atlas_ids(&session, "south");
    assert!(north_before.len() >= 2);
    let (_, receipt) = cast_draw(&mut session);
    assert_eq!(drawn_count(&receipt), 2);
    assert_eq!(atlas_ids(&session, "south"), south_before);
    for instance_id in north_before.iter().take(2) {
        assert!(!atlas_ids(&session, "north").contains(instance_id));
        assert!(atlas_hand_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2208_second_draw_draws_the_newly_exposed_atlas_top() {
    let encoded = supplemental_seed_with_start(2208, 24, 2);
    let mut session = opening_main(&encoded);
    let before = atlas_ids(&session, "north");
    assert!(before.len() >= 4);
    assert!(draw_sites_in_hand(&state(&session)) >= 2);
    let (_, first) = cast_draw(&mut session);
    assert_eq!(drawn_count(&first), 2);
    let exposed: Vec<_> = before.iter().skip(2).take(2).cloned().collect();
    let remaining = atlas_ids(&session, "north");
    assert_eq!(&remaining[..2], &exposed[..]);
    let (_, second) = cast_draw(&mut session);
    assert_eq!(drawn_count(&second), 2);
    for instance_id in &exposed {
        assert!(atlas_hand_ids(&session, "north").contains(instance_id));
        assert!(!atlas_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}
