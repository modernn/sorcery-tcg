//! Direct proofs for targetless draw-spell Magic (RULE-CATALOG-0645–0646,
//! RULE-CATALOG-1038, RULE-CATALOG-2193–2198).
//!
//! Draw-spell Magic pays, draws the printed number of hidden Spellbook cards
//! in deck order, and enters its owner's cemetery. Drawing from an empty
//! library loses immediately after any partial draws.

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

fn draw_spell() -> Value {
    json!({
        "cardType": "magic",
        "drawSpells": 2,
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

fn draw_spells_manifest(seed: u32, north_spellbook: &[&str], include_filler: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-draw": draw_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-minion": minion(),
        "south-site": site(),
    });
    if include_filler {
        cards["north-filler"] = minion();
    }
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "draw-spells-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-draw-spells-magic-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
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
    let mut session = Session::new(encoded).expect("valid draw-spells session");
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
fn rule_catalog_0645_draw_spells_magic_draws_hidden_spellbook_cards() {
    let encoded = draw_spells_manifest(645, &["north-draw"; 6], false);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let expected: Vec<_> = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .iter()
        .take(2)
        .map(|card| card["instanceId"].clone())
        .collect();
    assert_eq!(expected.len(), 2);
    let south_before = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_before["players"]["north"]["hand"]["spellbook"], 3);
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
        ["magic-cast", "spell-drawn", "spell-drawn", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], spell_id);
    let after = state(&session);
    let hand = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand");
    assert_eq!(hand.len(), 4);
    for instance_id in &expected {
        assert!(
            hand.iter().any(|card| &card["instanceId"] == instance_id),
            "the drawn identities must enter the hidden Spellbook hand"
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
        south_after["players"]["north"]["hand"]["spellbook"], 4,
        "the opponent sees only the new hand count"
    );
    assert_eq!(south_after["players"]["north"]["spellbookCount"], 1);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("draw-spells checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized draw-spells");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed draw-spells");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed draw-spells session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0646_draw_spells_magic_exhausts_then_loses_on_empty_library() {
    for remaining in 0..=1 {
        let mut north_spells = vec!["north-draw", "north-draw", "north-draw"];
        if remaining > 0 {
            north_spells.extend(std::iter::repeat_n("north-filler", remaining));
        }
        let encoded = draw_spells_manifest(
            646 + u32::try_from(remaining).expect("small remaining count"),
            &north_spells,
            remaining > 0,
        );
        let mut session = opening_main(&encoded);
        let before = state(&session);
        let expected = before["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .clone();
        assert_eq!(expected.len(), remaining);
        let (_, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
        });
        let kinds = event_types(&receipt);
        assert_eq!(kinds.first(), Some(&"magic-cast"));
        assert_eq!(
            kinds.iter().filter(|kind| **kind == "spell-drawn").count(),
            expected.len()
        );
        assert_eq!(kinds.last(), Some(&"game-ended"));
        assert!(kinds.contains(&"magic-resolved"));
        let after = state(&session);
        assert_eq!(after["players"]["north"]["spellbook"], json!([]));
        assert_eq!(after["terminal"]["reason"], "deck_empty");
        assert_eq!(after["terminal"]["loser"], "north");
        for card in expected {
            assert!(
                after["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .expect("north hand")
                    .contains(&card)
            );
        }
        assert_exact_replay(&session);
    }
}

fn draw_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("draw-spell actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-draw"
        })
        .count()
}

fn deathrite_draw_manifest(seed: u32) -> String {
    let fixture = "draw-spells-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": draw_spell(),
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

struct PendingDeathriteDrawSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_draw_magic(encoded: &str) -> Option<PendingDeathriteDrawSetup> {
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
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteDrawSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_draw_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_draw_manifest)
        .find(|candidate| try_pending_deathrite_with_draw_magic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with draw-spell Magic in hand")
}

#[test]
fn rule_catalog_1038_draw_spells_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_draw_seed_with(1038);
    let mut setup = try_pending_deathrite_with_draw_magic(&encoded)
        .expect("complete draw-spell Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
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
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(draw_casts(session) >= 1);

    let before = state(session);
    let expected: Vec<_> = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
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
        ["magic-cast", "spell-drawn", "spell-drawn", "magic-resolved"]
    );
    let after = state(session);
    let hand = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand");
    for instance_id in &expected {
        assert!(
            hand.iter().any(|card| &card["instanceId"] == instance_id),
            "the drawn identities must enter the hidden Spellbook hand"
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

fn supplemental_draw_spell() -> Value {
    let mut value = draw_spell();
    value["manaCost"] = json!(0);
    value
}

fn draw_supplemental_manifest(seed: u32, north_spellbook: usize) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "draw-spells-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-draw-spells-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw": supplemental_draw_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-draw"; north_spellbook],
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

fn opening_hand_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
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

fn supplemental_seed_with_start(
    start: u32,
    north_spellbook: usize,
    required_draws: usize,
) -> String {
    (start..start + 2048)
        .chain(645..645 + 2048)
        .map(|seed| draw_supplemental_manifest(seed, north_spellbook))
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .filter(|card| *card == "north-draw")
                .count()
                >= required_draws
        })
        .expect("bounded seed with draw-spell Magic in the opening hand")
}

fn draw_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-draw")
                .count()
        })
        .unwrap_or_default()
}

fn library_ids(session: &Session, seat: &str) -> Vec<String> {
    state(session)["players"][seat]["spellbook"]
        .as_array()
        .expect("library")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("library identity")
                .to_owned()
        })
        .collect()
}

fn hand_ids(session: &Session, seat: &str) -> Vec<String> {
    state(session)["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("hand")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("hand identity")
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
        .filter(|event| **event == "spell-drawn")
        .count()
}

fn try_second_draw_enemy_arrival_prefix(encoded: &str) -> Option<(Session, Vec<String>)> {
    let mut session = opening_main(encoded);
    if draw_spells_in_hand(&state(&session)) < 2 {
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
    if draw_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let remaining = library_ids(&session, "north");
    (remaining.len() >= 2).then_some((session, remaining))
}

fn seed_for_second_draw_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(645..645 + 8192)
        .find_map(|seed| {
            let encoded = draw_supplemental_manifest(seed, 8);
            try_second_draw_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second draw-spells enemy-arrival setup")
}

#[test]
fn rule_catalog_2193_drawn_library_cards_stay_in_hand_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2193, 8, 1);
    let mut session = opening_main(&encoded);
    let expected: Vec<_> = library_ids(&session, "north").into_iter().take(2).collect();
    assert_eq!(expected.len(), 2);
    let (_, receipt) = cast_draw(&mut session);
    assert_eq!(drawn_count(&receipt), 2);
    for instance_id in &expected {
        assert!(hand_ids(&session, "north").contains(instance_id));
        assert!(!library_ids(&session, "north").contains(instance_id));
    }
    pass_turn_to_north_spellbook(&mut session);
    for instance_id in &expected {
        assert!(hand_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2194_second_draw_on_an_empty_library_decks_out() {
    let encoded = supplemental_seed_with_start(2194, 5, 2);
    let mut session = opening_main(&encoded);
    assert_eq!(library_ids(&session, "north").len(), 2);
    assert!(draw_spells_in_hand(&state(&session)) >= 2);
    let (_, first) = cast_draw(&mut session);
    assert_eq!(drawn_count(&first), 2);
    assert_eq!(library_ids(&session, "north").len(), 0);
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert!(draw_spells_in_hand(&state(&session)) >= 1);
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
fn rule_catalog_2195_second_draw_still_draws_after_enemy_site_placement() {
    let encoded = seed_for_second_draw_enemy_arrival(2195);
    let (mut session, remaining) = try_second_draw_enemy_arrival_prefix(&encoded)
        .expect("second draw-spells enemy-arrival prefix");
    let expected: Vec<_> = remaining.into_iter().take(2).collect();
    let (_, receipt) = cast_draw(&mut session);
    assert_eq!(drawn_count(&receipt), 2);
    for instance_id in &expected {
        assert!(hand_ids(&session, "north").contains(instance_id));
        assert!(!library_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2196_draw_spells_is_targetless_and_draws_printed_cards() {
    let encoded = supplemental_seed_with_start(2196, 8, 1);
    let mut session = opening_main(&encoded);
    let draw_actions: Vec<_> = session
        .legal_actions()
        .expect("draw-spell actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-draw"
        })
        .collect();
    assert!(!draw_actions.is_empty());
    assert_eq!(draw_actions.len(), draw_spells_in_hand(&state(&session)));
    assert!(
        draw_actions
            .iter()
            .all(|action| action.descriptor.get("target").is_none())
    );
    let expected: Vec<_> = library_ids(&session, "north").into_iter().take(2).collect();
    let (descriptor, receipt) = cast_draw(&mut session);
    assert!(descriptor.get("target").is_none());
    assert_eq!(drawn_count(&receipt), 2);
    for instance_id in &expected {
        assert!(hand_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2197_draw_spells_leaves_the_opponent_library_untouched() {
    let encoded = supplemental_seed_with_start(2197, 8, 1);
    let mut session = opening_main(&encoded);
    let north_before = library_ids(&session, "north");
    let south_before = library_ids(&session, "south");
    assert!(north_before.len() >= 2);
    let (_, receipt) = cast_draw(&mut session);
    assert_eq!(drawn_count(&receipt), 2);
    assert_eq!(library_ids(&session, "south"), south_before);
    for instance_id in north_before.iter().take(2) {
        assert!(!library_ids(&session, "north").contains(instance_id));
        assert!(hand_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2198_second_draw_draws_the_newly_exposed_library_top() {
    let encoded = supplemental_seed_with_start(2198, 8, 2);
    let mut session = opening_main(&encoded);
    let before = library_ids(&session, "north");
    assert!(before.len() >= 4);
    assert!(draw_spells_in_hand(&state(&session)) >= 2);
    let (_, first) = cast_draw(&mut session);
    assert_eq!(drawn_count(&first), 2);
    let exposed: Vec<_> = before.iter().skip(2).take(2).cloned().collect();
    let remaining = library_ids(&session, "north");
    assert_eq!(&remaining[..2], &exposed[..]);
    let (_, second) = cast_draw(&mut session);
    assert_eq!(drawn_count(&second), 2);
    for instance_id in &exposed {
        assert!(hand_ids(&session, "north").contains(instance_id));
        assert!(!library_ids(&session, "north").contains(instance_id));
    }
    assert_exact_replay(&session);
}
