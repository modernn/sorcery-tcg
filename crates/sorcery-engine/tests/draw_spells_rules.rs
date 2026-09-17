//! Direct proofs for targetless draw-spell Magic (RULE-CATALOG-0645–0646,
//! RULE-CATALOG-1038).
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
    if state(&session)["phase"] != "deathrite-order" {
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
