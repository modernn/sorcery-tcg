//! Direct proofs for ward-nearby-minion-or-site Magic
//! (RULE-CATALOG-0551–0552, RULE-CATALOG-1077, RULE-CATALOG-1733–1738).
//!
//! Ordinary Magic can Ward one nearby minion or one nearby site. Far sites
//! are not offered. Site Ward is a one-shot mark consumed by the next
//! targeted destroy. While Deathrites wait for ordering, ward-nearby Magic
//! stays withheld until the chain drains.

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

fn bless() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "wardNearbyMinionOrSite": true,
    })
}

fn destroy_site() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
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

fn bless_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-nearby-minion-or-site" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-nearby-minion-or-site-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-bless": bless(),
            "north-site": earth_site(),
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
                    "north-bless",
                    "north-bless",
                    "north-bless",
                    "north-bless",
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

fn destroy_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-nearby-site-destroy" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-nearby-site-destroy-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bless": bless(),
            "north-destroy": destroy_site(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-dummy": grounded(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-bless",
                    "north-bless",
                    "north-destroy",
                    "north-destroy",
                    "north-destroy",
                    "north-destroy",
                ],
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
    let mut session = Session::new(encoded).expect("valid ward-nearby session");
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

fn bless_minion_ids(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("bless actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-bless"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn bless_site_cells(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("bless actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-bless"
        })
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (551..551 + 256)
        .map(bless_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Bless opening cards")
}

fn seed_destroy() -> String {
    seed_destroy_at(552)
}

fn seed_destroy_at(start: u32) -> String {
    (start..start + 256)
        .map(destroy_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().filter(|card| *card == "north-bless").count() >= 1
                && hand.iter().filter(|card| *card == "north-destroy").count() >= 2
        })
        .expect("bounded seed with Bless and two destroy-site cards")
}

fn try_site_ward_persistence(encoded: &str) -> Option<Session> {
    let mut session = opening_main(encoded);
    south_plays_c1(&mut session);
    if !bless_site_cells(&session).contains(&"C4".to_owned()) {
        return None;
    }
    cast_bless_site(&mut session, "C4");
    Some(session)
}

fn site_ward_persistence_setup(start: u32) -> Session {
    (start..start + 256)
        .filter_map(|seed| {
            let hand = opening_spell_ids(&destroy_manifest(seed));
            (hand.iter().filter(|card| *card == "north-bless").count() >= 1
                && hand.iter().filter(|card| *card == "north-destroy").count() >= 2)
                .then(|| destroy_manifest(seed))
        })
        .find_map(|encoded| try_site_ward_persistence(&encoded))
        .expect("bounded seed with Bless site ward persistence")
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

fn advance_full_round(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bless_manifest_with_spellbook(seed: u32, spellbook: &[&str]) -> String {
    let mut cards = json!({
        "north-ally": grounded(),
        "north-avatar": avatar(),
        "north-bless": bless(),
        "north-site": earth_site(),
        "south-avatar": avatar(),
        "south-raider": raider(),
        "south-site": earth_site(),
    });
    if spellbook.contains(&"north-second") {
        cards["north-second"] = grounded();
    }
    if spellbook.contains(&"north-destroy") {
        cards["north-destroy"] = destroy_site();
    }
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-nearby-proof" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-nearby-proof-v1",
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
                "spellbook": vec!["south-raider"; 6],
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
        "north-ally",
        "north-second",
        "north-bless",
        "north-bless",
        "north-bless",
        "north-bless",
    ];
    (start..start + 256)
        .map(|seed| bless_manifest_with_spellbook(seed, &spellbook))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Bless opening cards")
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

fn host_setup(start: u32) -> (Session, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-bless"], start);
    opening_with_ally(&encoded)
}

fn host_setup_with_two_blesses(start: u32) -> (Session, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-bless", "north-bless"], start);
    opening_with_ally(&encoded)
}

fn host_setup_ward_then_second_ally(start: u32) -> (Session, String, String) {
    let encoded = seed_with_spellbook(
        &["north-ally", "north-bless", "north-bless", "north-second"],
        start,
    );
    let (mut session, first_id) = opening_with_ally(&encoded);
    cast_bless_minion(&mut session, &first_id);
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

fn cast_bless_minion(session: &mut Session, minion_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == minion_id
    });
    receipt
}

fn cast_bless_site(session: &mut Session, cell: &str) -> Receipt {
    let site_id = state(session)["realm"]["sites"][cell]["instanceId"]
        .as_str()
        .expect("site identity")
        .to_owned();
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetSiteInstanceId"] == site_id
    });
    receipt
}

fn south_hand_has_zap(snapshot: &Value) -> bool {
    snapshot["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-zap"))
}

fn bless_zap_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-nearby-zap" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-nearby-zap-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-bless": bless(),
            "north-site": earth_site(),
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
                    "north-bless",
                    "north-bless",
                    "north-bless",
                    "north-bless",
                    "north-bless",
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

fn try_bless_then_south_zap(encoded: &str) -> Option<(Session, String)> {
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
    if bless_minion_ids(&session).is_empty() {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["target"]["instanceId"] == ally_id
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    south_hand_has_zap(&state(&session)).then_some((session, ally_id))
}

fn bless_zap_setup(start: u32) -> (Session, String) {
    (start..start + 256)
        .map(bless_zap_manifest)
        .filter(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-ally")
                && opening_spell_ids(candidate)
                    .iter()
                    .any(|card| card == "north-bless")
        })
        .find_map(|candidate| try_bless_then_south_zap(&candidate))
        .expect("bounded seed reaching Bless ward then south Zap")
}

fn south_zaps_ally(session: &mut Session, ally_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-zap"
            && descriptor["target"]["instanceId"] == ally_id
    });
    receipt
}

fn deathrite_bless_manifest(seed: u32) -> String {
    let fixture = "ward-nearby-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bless": bless(),
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
                    "north-bless",
                    "north-rain",
                    "north-rain",
                    "north-bless",
                    "north-rain",
                    "north-bless",
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

fn north_has_bless_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-bless", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteBlessSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteBlessSetup> {
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
    if !north_has_bless_and_rain(&state(&session)) {
        return None;
    }
    if bless_minion_ids(&session).is_empty() {
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
    Some(PendingDeathriteBlessSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_bless_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_bless_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with ward-nearby Magic in hand")
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
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
fn rule_catalog_0551_bless_wards_a_nearby_minion_and_offers_a_nearby_site() {
    let encoded = seed_with(&["north-ally", "north-bless"]);
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
    let offered_minions = bless_minion_ids(&session);
    assert!(offered_minions.contains(&ally_id));
    assert!(offered_minions.contains(&enemy_id));
    let offered_sites = bless_site_cells(&session);
    assert!(offered_sites.contains(&"C4".to_owned()));
    assert!(!offered_sites.contains(&"C1".to_owned()));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == ally_id
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
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["warded"], true);
    assert_eq!(unit(&after, &enemy_id)["warded"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0552_bless_wards_a_nearby_site_until_targeted_destroy() {
    let encoded = seed_destroy();
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    let offered_sites = bless_site_cells(&session);
    assert!(offered_sites.contains(&"C4".to_owned()));
    assert!(!offered_sites.contains(&"C1".to_owned()));
    assert!(bless_minion_ids(&session).is_empty());
    let site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("C4 site identity")
        .to_owned();

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "site-warded", "magic-resolved"]
    );
    assert_eq!(granted.events[1].payload["cell"], "C4");
    assert_eq!(granted.events[1].payload["instanceId"], site_id);
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let warded = state(&session);
    assert_eq!(warded["realm"]["sites"]["C4"]["warded"], true);
    assert_eq!(warded["realm"]["sites"]["C4"]["instanceId"], site_id);

    let (_, consumed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&consumed),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    assert_eq!(consumed.events[1].payload["cell"], "C4");
    assert_eq!(consumed.events[1].payload["instanceId"], site_id);
    let after_ward = state(&session);
    assert_eq!(after_ward["realm"]["sites"]["C4"]["instanceId"], site_id);
    assert!(after_ward["realm"]["sites"]["C4"]["warded"].is_null());

    let (_, destroyed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&destroyed),
        [
            "magic-cast",
            "site-destroyed",
            "rubble-created",
            "magic-resolved"
        ]
    );
    let rubble = state(&session);
    assert_eq!(rubble["realm"]["sites"]["C4"]["rubble"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1077_ward_nearby_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_bless_seed_with(1077);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete ward-nearby Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
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
    assert!(realm_unit(&paused, &visitor_id).is_some());
    assert_eq!(unit(&paused, &visitor_id)["warded"], false);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(bless_minion_ids(session).is_empty());

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
    assert!(realm_unit(&resumed, &visitor_id).is_some());
    assert_eq!(unit(&resumed, &visitor_id)["warded"], false);
    assert_eq!(bless_minion_ids(session), [visitor_id.as_str()]);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-warded", "magic-resolved"]
    );
    assert_eq!(unit(&state(session), &visitor_id)["warded"], true);
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1733_bless_minion_ward_persists_after_turns_pass() {
    let (mut session, ally_id) = host_setup(1733);
    cast_bless_minion(&mut session, &ally_id);
    assert_eq!(unit(&state(&session), &ally_id)["warded"], true);
    pass_full_round(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1734_second_bless_on_warded_minion_emits_no_duplicate_ward() {
    let (mut session, ally_id) = host_setup_with_two_blesses(1734);
    let first = cast_bless_minion(&mut session, &ally_id);
    assert_eq!(
        event_types(&first),
        ["magic-cast", "minion-warded", "magic-resolved"]
    );
    let second = cast_bless_minion(&mut session, &ally_id);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert_eq!(unit(&state(&session), &ally_id)["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1735_bless_site_ward_persists_after_turns_pass() {
    let mut session = site_ward_persistence_setup(1735);
    assert_eq!(state(&session)["realm"]["sites"]["C4"]["warded"], true);
    advance_full_round(&mut session);
    assert_eq!(state(&session)["realm"]["sites"]["C4"]["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1736_bless_at_one_nearby_site_leaves_other_nearby_site_unwarded() {
    let encoded = seed_with_spellbook(&["north-bless"], 1736);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    lay_site_at(&mut session, "C3");
    assert!(bless_site_cells(&session).contains(&"C4".to_owned()));
    assert!(bless_site_cells(&session).contains(&"C3".to_owned()));
    cast_bless_site(&mut session, "C4");
    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C4"]["warded"], true);
    assert!(after["realm"]["sites"]["C3"]["warded"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1737_bless_minion_ward_absorbs_enemy_zap() {
    let (mut session, ally_id) = bless_zap_setup(1737);
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
fn rule_catalog_1738_second_bless_wards_newly_arrived_ally_at_same_cell() {
    let (mut session, first_id, second_id) = host_setup_ward_then_second_ally(1738);
    assert_eq!(unit(&state(&session), &first_id)["warded"], true);
    assert_eq!(unit(&state(&session), &second_id)["warded"], false);
    let granted = cast_bless_minion(&mut session, &second_id);
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "minion-warded", "magic-resolved"]
    );
    assert_eq!(granted.events[1].payload["instanceId"], second_id);
    assert_eq!(unit(&state(&session), &first_id)["warded"], true);
    assert_eq!(unit(&state(&session), &second_id)["warded"], true);
    assert_exact_replay(&session);
}
