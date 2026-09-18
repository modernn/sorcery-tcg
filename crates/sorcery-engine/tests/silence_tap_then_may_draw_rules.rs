//! Direct proofs for silence-and-tap-nearby-minion then may-draw-spell
//! Magic (RULE-CATALOG-0553–0554, RULE-CATALOG-1024, RULE-CATALOG-1743–1748).
//!
//! Ordinary Magic can Silence and tap one nearby minion, then may draw one
//! hidden spell. Silence is this-turn ability loss, not Disable. A far
//! minion is not offered. Declining the draw still Silences and taps. While
//! Deathrites wait for ordering, Insult Magic stays withheld until the chain
//! drains.

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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn caster() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "spellcaster": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn grounded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn insult() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "silenceAndTapNearbyMinionThenMayDrawSpell": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "cardType": "magic",
        "healController": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
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

fn visitor() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
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

fn insult_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "silence-tap-then-may-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-silence-tap-then-may-draw-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-caster": caster(),
            "north-dummy": dummy(),
            "north-insult": insult(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-far": grounded(),
            "south-raider": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-caster",
                    "north-dummy",
                    "north-dummy",
                    "north-insult",
                    "north-insult",
                    "north-insult",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-far"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-raider", 2))
                    .collect::<Vec<_>>(),
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
    let mut session = Session::new(encoded).expect("valid silence-tap session");
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

fn opening_spell_ids(encoded: &str) -> Vec<String> {
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn insult_targets(session: &Session) -> Vec<(String, bool)> {
    session
        .legal_actions()
        .expect("insult actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-insult"
        })
        .filter_map(|action| {
            Some((
                action.descriptor["target"]["instanceId"]
                    .as_str()?
                    .to_owned(),
                action.descriptor["drawZone"] == "spellbook",
            ))
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (553..553 + 256)
        .map(insult_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Insult opening cards")
}

fn deathrite_insult_manifest(seed: u32) -> String {
    let fixture = "silence-tap-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-insult": insult(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-insult",
                    "north-rain",
                    "north-rain",
                    "north-insult",
                    "north-rain",
                    "north-insult",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-visitor", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_insult_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-insult", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn insult_target_ids(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = insult_targets(session)
        .into_iter()
        .map(|(instance_id, _)| instance_id)
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

struct PendingDeathriteSilenceTapSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_unsilenced_visitor(
    encoded: &str,
) -> Option<PendingDeathriteSilenceTapSetup> {
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
    let visitor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    })?;
    let visitor_id = visitor.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_insult_and_rain(&state(&session)) {
        return None;
    }
    let snapshot = state(&session);
    let visitor_unit = unit(&snapshot, &visitor_id);
    if !visitor_unit["silenced"].is_null() {
        return None;
    }
    if visitor_unit["tapped"] != false {
        return None;
    }
    if !insult_target_ids(&session).contains(&visitor_id) {
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
    Some(PendingDeathriteSilenceTapSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_silence_tap_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_insult_manifest)
        .find(|candidate| try_pending_deathrite_with_unsilenced_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Insult Magic in hand")
}

fn south_plays_c1_and_summons_far(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("far identity")
        .to_owned()
}

fn south_raids_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("raider identity")
        .to_owned()
}

fn south_plays_c1_and_raids_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("raider identity")
        .to_owned()
}

fn insult_manifest_with_spellbook(seed: u32, spellbook: &[&str]) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-caster": caster(),
        "north-dummy": dummy(),
        "north-insult": insult(),
        "north-site": earth_site(),
        "south-avatar": avatar(),
        "south-far": grounded(),
        "south-raider": raider(),
        "south-site": earth_site(),
    });
    if spellbook.contains(&"north-second") {
        cards["north-second"] = grounded();
    }
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "silence-tap-proof" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-silence-tap-proof-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-far"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-raider", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn seed_with_spellbook(required: &[&str], start: u32) -> String {
    let spellbook = vec![
        "north-caster",
        "north-second",
        "north-dummy",
        "north-insult",
        "north-insult",
        "north-insult",
    ];
    (start..start + 256)
        .map(|seed| insult_manifest_with_spellbook(seed, &spellbook))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Insult opening cards")
}

fn summon_north_caster(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("caster instance identity")
        .to_owned()
}

fn opening_with_caster(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let caster_id = summon_north_caster(&mut session);
    (session, caster_id)
}

fn host_setup(start: u32) -> (Session, String) {
    let encoded = seed_with_spellbook(&["north-caster", "north-insult"], start);
    opening_with_caster(&encoded)
}

fn host_setup_with_two_insults(start: u32) -> (Session, String) {
    let encoded = seed_with_spellbook(&["north-caster", "north-insult", "north-insult"], start);
    opening_with_caster(&encoded)
}

fn host_setup_insult_then_second_caster(start: u32) -> (Session, String, String) {
    let encoded = seed_with_spellbook(
        &[
            "north-caster",
            "north-insult",
            "north-insult",
            "north-second",
        ],
        start,
    );
    let (mut session, first_id) = opening_with_caster(&encoded);
    cast_insult_no_draw(&mut session, &first_id);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (second, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-second"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let second_id = second["cardInstanceId"]
        .as_str()
        .expect("second ally identity")
        .to_owned();
    (session, first_id, second_id)
}

fn cast_insult_draw(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-insult"
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["drawZone"] == "spellbook"
    });
    receipt
}

fn cast_insult_no_draw(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-insult"
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["drawZone"].is_null()
    });
    receipt
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
fn rule_catalog_0553_insult_silences_and_taps_a_nearby_minion_then_draws() {
    let encoded = seed_with(&["north-caster", "north-dummy", "north-insult"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let caster_id = summoned["cardInstanceId"]
        .as_str()
        .expect("caster instance identity")
        .to_owned();
    let far_id = south_plays_c1_and_summons_far(&mut session);
    let before = state(&session);
    assert_eq!(unit(&before, &caster_id)["tapped"], false);
    assert!(unit(&before, &caster_id)["silenced"].is_null());
    assert!(unit(&before, &caster_id)["disableEffects"].is_null());
    assert!(unit(&before, &caster_id)["disabledUntilDamaged"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("pre-insult actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["cardId"] == "north-dummy"
                    && action.descriptor["casterInstanceId"] == caster_id
            })
    );
    let offered = insult_targets(&session);
    assert!(offered.iter().any(|(id, draw)| *id == caster_id && *draw));
    assert!(offered.iter().any(|(id, draw)| *id == caster_id && !*draw));
    assert!(!offered.iter().any(|(id, _)| *id == far_id));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-insult"
            && descriptor["target"]["instanceId"] == caster_id
            && descriptor["drawZone"] == "spellbook"
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], caster_id);
    assert_eq!(granted.events[1].payload["seat"], "north");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    let silenced = unit(&after, &caster_id);
    assert_eq!(silenced["tapped"], true);
    assert_eq!(silenced["silenced"], true);
    assert!(silenced["disableEffects"].is_null());
    assert!(silenced["disabledUntilDamaged"].is_null());
    assert_eq!(
        silenced["temporarySilenceSources"],
        json!([cast["cardInstanceId"]])
    );
    assert!(
        !session
            .legal_actions()
            .expect("post-insult actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["casterInstanceId"] == caster_id
            })
    );
    assert!(
        session
            .legal_actions()
            .expect("avatar still casts")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["cardId"] == "north-dummy"
                    && action.descriptor["casterInstanceId"]
                        == after["players"]["north"]["avatar"]["card"]["instanceId"]
            })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0554_insult_still_silences_and_taps_when_the_draw_is_declined() {
    let encoded = seed_with(&["north-caster", "north-insult"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let caster_id = summoned["cardInstanceId"]
        .as_str()
        .expect("caster instance identity")
        .to_owned();
    south_plays_c1_and_summons_far(&mut session);
    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-insult"
            && descriptor["target"]["instanceId"] == caster_id
            && descriptor["drawZone"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let silenced = state(&session);
    assert_eq!(unit(&silenced, &caster_id)["tapped"], true);
    assert_eq!(unit(&silenced, &caster_id)["silenced"], true);
    assert!(unit(&silenced, &caster_id)["disableEffects"].is_null());

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "silence-expired"
            && event.payload["instanceId"] == caster_id
            && event.payload["sourceInstanceId"] == cast["cardInstanceId"]
    }));
    let after = state(&session);
    assert_eq!(unit(&after, &caster_id)["tapped"], true);
    assert!(unit(&after, &caster_id)["silenced"].is_null());
    assert!(unit(&after, &caster_id)["temporarySilenceSources"].is_null());
    assert!(unit(&after, &caster_id)["disableEffects"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1024_silence_tap_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_silence_tap_seed_with(1024);
    let mut setup = try_pending_deathrite_with_unsilenced_visitor(&encoded)
        .expect("complete Insult Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
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
    assert_eq!(unit(&paused, &visitor_id)["tapped"], false);
    assert!(unit(&paused, &visitor_id)["silenced"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(insult_target_ids(session).is_empty());

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
    assert_eq!(unit(&resumed, &visitor_id)["tapped"], false);
    assert!(unit(&resumed, &visitor_id)["silenced"].is_null());
    assert!(insult_target_ids(session).contains(&visitor_id));

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-insult"
            && descriptor["target"]["instanceId"] == visitor_id
            && descriptor["drawZone"].is_null()
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], visitor_id);
    assert_eq!(receipt.events[1].payload["seat"], "south");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(session);
    let silenced = unit(&after, &visitor_id);
    assert_eq!(silenced["tapped"], true);
    assert_eq!(silenced["silenced"], true);
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1743_insult_tap_persists_during_the_opponent_turn() {
    let (mut session, caster_id) = host_setup(1743);
    south_plays_c1_and_summons_far(&mut session);
    cast_insult_no_draw(&mut session, &caster_id);
    assert_eq!(unit(&state(&session), &caster_id)["tapped"], true);
    assert_eq!(unit(&state(&session), &caster_id)["silenced"], true);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let after_end = state(&session);
    assert_eq!(unit(&after_end, &caster_id)["tapped"], true);
    assert!(unit(&after_end, &caster_id)["silenced"].is_null());
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert_eq!(unit(&state(&session), &caster_id)["tapped"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1744_second_insult_omits_an_already_silenced_and_tapped_minion() {
    let (mut session, caster_id) = host_setup_with_two_insults(1744);
    south_plays_c1_and_summons_far(&mut session);
    let first = cast_insult_no_draw(&mut session, &caster_id);
    assert_eq!(
        event_types(&first),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    assert!(!insult_target_ids(&session).contains(&caster_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1745_second_insult_without_draw_still_silences_and_taps() {
    let (mut session, caster_id) = host_setup_with_two_insults(1745);
    south_plays_c1_and_summons_far(&mut session);
    cast_insult_draw(&mut session, &caster_id);
    assert_eq!(unit(&state(&session), &caster_id)["tapped"], true);
    let enemy_id = south_raids_c4(&mut session);
    let granted = cast_insult_no_draw(&mut session, &enemy_id);
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    assert_eq!(unit(&state(&session), &enemy_id)["tapped"], true);
    assert_eq!(unit(&state(&session), &enemy_id)["silenced"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1746_insult_silences_and_taps_an_enemy_minion_at_the_caster_site() {
    let encoded = seed_with_spellbook(&["north-caster", "north-insult"], 1746);
    let mut session = opening_main(&encoded);
    summon_north_caster(&mut session);
    let enemy_id = south_plays_c1_and_raids_c4(&mut session);
    assert!(insult_target_ids(&session).contains(&enemy_id));
    let granted = cast_insult_no_draw(&mut session, &enemy_id);
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    assert_eq!(unit(&state(&session), &enemy_id)["tapped"], true);
    assert_eq!(unit(&state(&session), &enemy_id)["silenced"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1747_insult_pulls_targets_only_from_minions_near_the_caster_site() {
    let encoded = seed_with_spellbook(&["north-caster", "north-insult"], 1747);
    let mut session = opening_main(&encoded);
    let caster_id = summon_north_caster(&mut session);
    let far_id = south_plays_c1_and_summons_far(&mut session);
    let offered = insult_target_ids(&session);
    assert!(offered.contains(&caster_id));
    assert!(!offered.contains(&far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1748_second_insult_silences_a_newly_arrived_ally_at_the_same_cell() {
    let (mut session, first_id, second_id) = host_setup_insult_then_second_caster(1748);
    assert!(unit(&state(&session), &first_id)["silenced"].is_null());
    assert_eq!(unit(&state(&session), &second_id)["tapped"], false);
    assert!(unit(&state(&session), &second_id)["silenced"].is_null());
    let granted = cast_insult_no_draw(&mut session, &second_id);
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], second_id);
    assert_eq!(unit(&state(&session), &second_id)["tapped"], true);
    assert_eq!(unit(&state(&session), &second_id)["silenced"], true);
    assert_exact_replay(&session);
}
