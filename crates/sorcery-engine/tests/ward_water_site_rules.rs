//! Direct proofs for ward-each-allied-minion-at-target-water-site Magic
//! (RULE-CATALOG-0545–0546, RULE-CATALOG-1076, RULE-CATALOG-1703–1708).
//!
//! Ordinary Magic can target a Water site and Ward every allied minion
//! occupying that site. Enemy minions there are not warded. Earth sites are
//! not offered. An empty Water site is a paid no-op. While Deathrites wait
//! for ordering, Baptize Magic stays withheld until the chain drains.

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

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["water"],
    })
}

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
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

fn baptize() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "wardEachAlliedMinionAtTargetWaterSite": true,
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

fn baptize_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-water-site" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-water-site-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-baptize": baptize(),
            "north-site": water_site(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-baptize",
                    "north-baptize",
                    "north-baptize",
                    "north-baptize",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-raider"; 6],
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
    let mut session = Session::new(encoded).expect("valid ward-water-site session");
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

fn baptize_cells(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("baptize actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-baptize"
        })
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (545..545 + 256)
        .map(baptize_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Baptize opening cards")
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
        .expect("enemy identity")
        .to_owned()
}

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn pass_full_round(session: &mut Session) {
    south_plays_c1(session);
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn baptize_manifest_with_spellbook(seed: u32, spellbook: &[&str]) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-water-site-proof" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-water-site-proof-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-baptize": baptize(),
            "north-second": grounded(),
            "north-site": water_site(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-raider"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn summon_north_ally(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned()
}

fn opening_with_ally(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let ally_id = summon_north_ally(&mut session);
    (session, ally_id)
}

fn lay_site_at(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn seed_with_spellbook(required: &[&str], start: u32) -> String {
    let spellbook = vec![
        "north-ally",
        "north-second",
        "north-baptize",
        "north-baptize",
        "north-baptize",
        "north-baptize",
    ];
    (start..start + 256)
        .map(|seed| baptize_manifest_with_spellbook(seed, &spellbook))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Baptize opening cards")
}

fn host_setup(start: u32) -> (Session, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-baptize"], start);
    opening_with_ally(&encoded)
}

fn host_setup_with_two_baptizes(start: u32) -> (Session, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-baptize", "north-baptize"], start);
    opening_with_ally(&encoded)
}

fn host_setup_with_two_allies_at_c4(start: u32) -> (Session, String, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-second", "north-baptize"], start);
    let (mut session, first_id) = opening_with_ally(&encoded);
    south_plays_c1(&mut session);
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

fn host_setup_with_allies_at_c4_and_c3(start: u32) -> (Session, String, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-second", "north-baptize"], start);
    let (mut session, home_id) = opening_with_ally(&encoded);
    south_plays_c1(&mut session);
    lay_site_at(&mut session, "C3");
    let (away, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-second"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    let away_id = away["cardInstanceId"]
        .as_str()
        .expect("away ally identity")
        .to_owned();
    (session, home_id, away_id)
}

fn cast_baptize(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-baptize"
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
}

fn warded_ally_ids(receipt: &Receipt) -> Vec<String> {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-warded")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("warded ally identity")
                .to_owned()
        })
        .collect()
}

fn south_hand_has_zap(snapshot: &Value) -> bool {
    snapshot["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-zap"))
}

fn baptize_zap_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-water-site-zap" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-water-site-zap-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-baptize": baptize(),
            "north-site": water_site(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
            "south-zap": zap(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-baptize",
                    "north-baptize",
                    "north-baptize",
                    "north-baptize",
                    "north-baptize",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-zap"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn try_opening_with_ally(encoded: &str) -> Option<(Session, String)> {
    let mut session = Session::new(encoded).ok()?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let ally_id = summoned["cardInstanceId"].as_str()?.to_owned();
    Some((session, ally_id))
}

fn try_baptize_then_south_zap(encoded: &str) -> Option<(Session, String)> {
    let (mut session, ally_id) = try_opening_with_ally(encoded)?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if baptize_cells(&session).is_empty() {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-baptize"
            && descriptor["targetLocation"]["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    south_hand_has_zap(&state(&session)).then_some((session, ally_id))
}

fn baptize_zap_setup(start: u32) -> (Session, String) {
    (start..start + 256)
        .map(baptize_zap_manifest)
        .filter(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-ally")
                && opening_spell_ids(candidate)
                    .iter()
                    .any(|card| card == "north-baptize")
        })
        .find_map(|candidate| try_baptize_then_south_zap(&candidate))
        .expect("bounded seed reaching Baptize ward then south Zap")
}

fn host_setup_ward_then_second_ally(start: u32) -> (Session, String, String) {
    let encoded = seed_with_spellbook(
        &[
            "north-ally",
            "north-baptize",
            "north-baptize",
            "north-second",
        ],
        start,
    );
    let (mut session, first_id) = opening_with_ally(&encoded);
    cast_baptize(&mut session, "C4");
    south_plays_c1(&mut session);
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

fn south_zaps_ally(session: &mut Session, ally_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-zap"
            && descriptor["target"]["instanceId"] == ally_id
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

fn deathrite_baptize_manifest(seed: u32) -> String {
    let fixture = "baptize-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-baptize": baptize(),
            "north-rain": rain_spell(),
            "north-site": water_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-baptize",
                    "north-rain",
                    "north-rain",
                    "north-baptize",
                    "north-baptize",
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

fn north_has_baptize_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-baptize", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteBaptizeSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_ready_ally(encoded: &str) -> Option<PendingDeathriteBaptizeSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_baptize_and_rain(&state(&session)) {
        return None;
    }
    if baptize_cells(&session).is_empty() {
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
    Some(PendingDeathriteBaptizeSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_baptize_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_baptize_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_ally(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Baptize Magic in hand")
}

#[test]
fn rule_catalog_0545_baptize_wards_allied_minions_at_a_water_site() {
    let encoded = seed_with(&["north-ally", "north-baptize"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    let enemy_id = south_plays_c1_and_raids_c4(&mut session);
    let before = state(&session);
    assert_eq!(unit(&before, &ally_id)["warded"], false);
    assert_eq!(unit(&before, &enemy_id)["warded"], false);
    let offered = baptize_cells(&session);
    assert!(offered.contains(&"C4".to_owned()));
    assert!(!offered.contains(&"C1".to_owned()));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-baptize"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "minion-warded", "magic-resolved"]
    );
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    assert_eq!(granted.events[1].payload["seat"], "north");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert!(!granted.events.iter().any(
        |event| event.event_type == "minion-warded" && event.payload["instanceId"] == enemy_id
    ));
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["warded"], true);
    assert_eq!(unit(&after, &enemy_id)["warded"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0546_baptize_ignores_earth_sites_and_empty_water() {
    let encoded = seed_with(&["north-baptize"]);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    let offered = baptize_cells(&session);
    assert!(!offered.is_empty());
    assert!(offered.iter().all(|cell| cell == "C4"));
    assert!(!offered.iter().any(|cell| cell == "C1"));

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-baptize"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(event_types(&granted), ["magic-cast", "magic-resolved"]);
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "minion-warded")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1076_ward_water_site_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_baptize_seed_with(1076);
    let mut setup = try_pending_deathrite_with_ready_ally(&encoded)
        .expect("complete Baptize Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
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
    assert_eq!(unit(&paused, &ally_id)["warded"], false);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(baptize_cells(session).is_empty());

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
    assert_eq!(unit(&resumed, &ally_id)["warded"], false);
    assert!(baptize_cells(session).contains(&"C4".to_owned()));

    let (cast, granted) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-baptize"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "minion-warded", "magic-resolved"]
    );
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    assert_eq!(granted.events[1].payload["seat"], "north");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(unit(&state(session), &ally_id)["warded"], true);
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1703_baptize_ward_persists_after_turns_pass() {
    let (mut session, ally_id) = host_setup(1703);
    cast_baptize(&mut session, "C4");
    assert_eq!(unit(&state(&session), &ally_id)["warded"], true);
    pass_full_round(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1704_second_baptize_on_warded_ally_emits_no_duplicate_ward() {
    let (mut session, ally_id) = host_setup_with_two_baptizes(1704);
    let first = cast_baptize(&mut session, "C4");
    assert_eq!(warded_ally_ids(&first), vec![ally_id.clone()]);
    let second = cast_baptize(&mut session, "C4");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert_eq!(unit(&state(&session), &ally_id)["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1705_baptize_wards_every_ally_co_located_at_the_water_site() {
    let (mut session, first_id, second_id) = host_setup_with_two_allies_at_c4(1705);
    let granted = cast_baptize(&mut session, "C4");
    let mut warded = warded_ally_ids(&granted);
    warded.sort_unstable();
    let mut expected = vec![first_id.clone(), second_id.clone()];
    expected.sort_unstable();
    assert_eq!(warded, expected);
    assert_eq!(unit(&state(&session), &first_id)["warded"], true);
    assert_eq!(unit(&state(&session), &second_id)["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1706_baptize_at_one_water_site_leaves_allies_at_other_sites_unwarded() {
    let (mut session, home_id, away_id) = host_setup_with_allies_at_c4_and_c3(1706);
    let granted = cast_baptize(&mut session, "C3");
    assert_eq!(warded_ally_ids(&granted), vec![away_id.clone()]);
    assert_eq!(unit(&state(&session), &away_id)["warded"], true);
    assert_eq!(unit(&state(&session), &home_id)["warded"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1707_baptize_ward_absorbs_enemy_zap() {
    let (mut session, ally_id) = baptize_zap_setup(1707);
    assert_eq!(unit(&state(&session), &ally_id)["warded"], true);
    let blocked = south_zaps_ally(&mut session, &ally_id);
    assert!(event_types(&blocked).contains(&"ward-broken"));
    assert!(event_types(&blocked).contains(&"magic-resolved"));
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["warded"], false);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1708_second_baptize_wards_newly_arrived_ally_at_same_site() {
    let (mut session, first_id, second_id) = host_setup_ward_then_second_ally(1708);
    assert_eq!(unit(&state(&session), &first_id)["warded"], true);
    assert_eq!(unit(&state(&session), &second_id)["warded"], false);
    let granted = cast_baptize(&mut session, "C4");
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "minion-warded", "magic-resolved"]
    );
    assert_eq!(granted.events[1].payload["instanceId"], second_id);
    assert_eq!(unit(&state(&session), &first_id)["warded"], true);
    assert_eq!(unit(&state(&session), &second_id)["warded"], true);
    assert_exact_replay(&session);
}
