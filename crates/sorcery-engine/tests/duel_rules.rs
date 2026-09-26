//! Direct proofs for fight-ally-with-adjacent-enemy Magic (RULE-CATALOG-0603–0604, 0711–0712, 0946,
//! 0965, 1007, 1045, 1983–1988).
//!
//! Duel makes a chosen ally fight a targeted adjacent enemy through the shared
//! fight pipeline. Ward on the target breaks without entering combat. Avatar allies
//! route strike damage through avatar-life-lost instead of minion damage. Underground
//! first-strike Duels pause in trigger-order, survive checkpoint round-trip, and
//! finish through ordered Deathrites to terminal.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn duel() -> Value {
    json!({
        "cardType": "magic",
        "fightAllyWithAdjacentEnemy": true,
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

fn burrow_all() -> Value {
    json!({
        "cardType": "magic",
        "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn double_mask() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": 0,
        "nearbyStrikesAgainstUnitsDealDoubleDamage": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn duel_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "duel-deathrite-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-duel-deathrite-draw-v1",
        },
        "cards": {
            "north-ally": minion(json!({ "attack": 3, "defense": 4 })),
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "spellcaster": true })),
            "north-duel": duel(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": minion(json!({
                "attack": 2,
                "deathriteDrawSite": true,
                "defense": 3,
            })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-duel", "north-ally", "north-caster", "north-duel", "north-ally", "north-caster"],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn duel_manifest(seed: u32, ward: bool) -> String {
    let south_enemy = if ward {
        minion(json!({ "attack": 2, "defense": 3, "ward": true }))
    } else {
        minion(json!({ "attack": 2, "defense": 3 }))
    };
    let fixture = if ward { "duel-ward" } else { "duel-fight" };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": minion(json!({ "attack": 3, "defense": 4 })),
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "spellcaster": true })),
            "north-duel": duel(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": south_enemy,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-duel", "north-ally", "north-caster", "north-duel", "north-ally", "north-caster"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn duel_nearby_mask_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "duel-nearby-mask-double-strike" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-duel-nearby-mask-double-strike-v1",
        },
        "cards": {
            "north-ally": minion(json!({ "attack": 1, "defense": 4 })),
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "spellcaster": true })),
            "north-duel": duel(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": minion(json!({ "attack": 1, "defense": 2 })),
            "south-mask": double_mask(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-duel", "north-ally", "north-caster", "north-duel", "north-ally", "north-caster"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-mask",
                    "south-enemy",
                    "south-enemy",
                    "south-mask",
                    "south-enemy",
                    "south-enemy",
                ],
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

fn damage_dealt_amount(receipt: &Receipt, target_id: &str) -> i64 {
    receipt
        .events
        .iter()
        .find(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == target_id
        })
        .expect("damage dealt to target")
        .payload["amount"]
        .as_i64()
        .expect("damage amount")
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

fn try_setup_duel(encoded: &str) -> Option<(Session, String, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let (ally_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = ally_summon["cardInstanceId"].as_str()?.to_owned();
    let (caster_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    })?;
    let caster_id = caster_summon["cardInstanceId"].as_str()?.to_owned();
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (enemy_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C3"
    })?;
    let enemy_id = enemy_summon["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some((session, ally_id, caster_id, enemy_id))
}

fn try_setup_duel_with_mask(
    encoded: &str,
    mask_on_enemy_bearer: bool,
) -> Option<(Session, String, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let (ally_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = ally_summon["cardInstanceId"].as_str()?.to_owned();
    let (caster_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    })?;
    let caster_id = caster_summon["cardInstanceId"].as_str()?.to_owned();
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (enemy_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C3"
    })?;
    let enemy_id = enemy_summon["cardInstanceId"].as_str()?.to_owned();
    if mask_on_enemy_bearer {
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "south-mask"
                && descriptor["bearer"]["instanceId"] == enemy_id
        })?;
    } else {
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "south-mask"
                && descriptor["cell"] == "C1"
                && descriptor["bearer"].is_null()
        })?;
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some((session, ally_id, caster_id, enemy_id))
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

fn setup_duel(encoded: &str) -> (Session, String, String, String) {
    try_setup_duel(encoded).expect("complete Duel setup")
}

fn underground_duel_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "duel-underground-first-strike" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-duel-underground-first-strike-v1",
        },
        "cards": {
            "north-ally": minion(json!({
                "attack": 2,
                "burrowing": true,
                "defense": 10,
                "strikesFirstWhileAttacking": true,
            })),
            "north-avatar": avatar(),
            "north-burrow": burrow_all(),
            "north-duel": duel(),
            "north-filler": minion(json!({})),
            "north-pinger": minion(json!({
                "defense": 10,
                "genesisDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-buff": minion(json!({
                "burrowing": true,
                "deathriteDrawSite": true,
                "otherNearbyAlliesPowerBonus": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-pinger",
                    "north-burrow",
                    "north-duel",
                    "north-filler",
                    "north-filler",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-buff"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn summon_minion(session: &mut Session, card_id: &str, cell: &str, region: Option<&str>) -> String {
    let (descriptor, _) = accept_where(session, |candidate| {
        candidate["kind"] == "summon-minion"
            && candidate["cardId"] == card_id
            && candidate["cell"] == cell
            && region.map_or_else(
                || candidate["region"].is_null(),
                |expected| candidate["region"] == expected,
            )
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned minion identity")
        .to_owned()
}

fn try_setup_underground_duel_checkpoint(encoded: &str) -> Option<(Session, String, Vec<String>)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    let opening = state(&session);
    let hand = opening["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North opening hand");
    if !hand.iter().any(|card| card["cardId"] == "north-ally") {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally_id = summon_minion(&mut session, "north-ally", "C4", None);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let mut target_ids = Vec::new();
    for _ in 0..2 {
        target_ids.push(summon_minion(&mut session, "south-buff", "C4", None));
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let drawn = state(&session);
    let north_hand = drawn["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand after one draw")
        .iter()
        .filter_map(|card| card["cardId"].as_str())
        .collect::<Vec<_>>();
    if !["north-pinger", "north-burrow", "north-duel"]
        .into_iter()
        .all(|card_id| north_hand.contains(&card_id))
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    summon_minion(&mut session, "north-pinger", "C4", None);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-burrow"
            && descriptor["targetLocation"]["cell"] == "C4"
    })?;
    let pre_duel = state(&session);
    if !target_ids.iter().all(|instance_id| {
        realm_unit(&pre_duel, instance_id).is_some_and(|unit| unit["damage"] == 1)
    }) {
        return None;
    }
    Some((session, ally_id, target_ids))
}

fn seed_underground_duel_checkpoint(start: u32) -> String {
    (start..start + 512)
        .map(underground_duel_manifest)
        .find(|candidate| try_setup_underground_duel_checkpoint(candidate).is_some())
        .expect("bounded seed with complete underground Duel checkpoint setup")
}

fn seed_with(ward: bool, start: u32) -> String {
    (start..start + 512)
        .map(|seed| duel_manifest(seed, ward))
        .find(|candidate| try_setup_duel(candidate).is_some())
        .expect("bounded seed with complete Duel setup")
}

fn seed_duel_with_mask(start: u32, mask_on_enemy_bearer: bool) -> String {
    (start..start + 512)
        .map(duel_nearby_mask_manifest)
        .find(|candidate| try_setup_duel_with_mask(candidate, mask_on_enemy_bearer).is_some())
        .expect("bounded seed with complete Mask Duel setup")
}

fn seed_duel_deathrite(start: u32) -> String {
    (start..start + 512)
        .map(duel_deathrite_manifest)
        .find(|candidate| try_setup_duel(candidate).is_some())
        .expect("bounded seed with complete Deathrite Duel setup")
}

fn deathrite_duel_manifest(seed: u32) -> String {
    let fixture = "duel-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": minion(json!({ "attack": 3, "defense": 4 })),
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "defense": 3, "spellcaster": true })),
            "north-duel": duel(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-duel",
                    "north-rain",
                    "north-ally",
                    "north-caster",
                    "north-duel",
                    "north-rain",
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

fn north_has_duel_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-duel", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn duel_targets(session: &Session) -> Vec<(String, String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("duel actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-duel"
        })
        .filter_map(|action| {
            Some((
                action.descriptor["ally"]["instanceId"].as_str()?.to_owned(),
                action.descriptor["casterInstanceId"].as_str()?.to_owned(),
                action.descriptor["target"]["instanceId"]
                    .as_str()?
                    .to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

struct PendingDeathriteDuelSetup {
    ally_id: String,
    caster_id: String,
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_duel(encoded: &str) -> Option<PendingDeathriteDuelSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let (ally_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = ally_summon["cardInstanceId"].as_str()?.to_owned();
    let (caster_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    })?;
    let caster_id = caster_summon["cardInstanceId"].as_str()?.to_owned();
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let visitor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C3"
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
    if !north_has_duel_and_rain(&state(&session)) {
        return None;
    }
    if duel_targets(&session).is_empty() {
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
    Some(PendingDeathriteDuelSetup {
        ally_id,
        caster_id,
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_duel_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_duel_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_duel(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with duel Magic in hand")
}

fn setup_duel_with_mask(
    encoded: &str,
    mask_on_enemy_bearer: bool,
) -> (Session, String, String, String) {
    try_setup_duel_with_mask(encoded, mask_on_enemy_bearer).expect("complete Mask Duel setup")
}

#[test]
fn rule_catalog_0603_duel_magic_fights_an_adjacent_enemy_through_the_shared_pipeline() {
    let encoded = seed_with(false, 603);
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert!(event_types(&receipt).contains(&"fight-started"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert!(realm_unit(&state(&session), &ally_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0604_duel_magic_breaks_ward_without_entering_combat() {
    let encoded = seed_with(true, 604);
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let after = state(&session);
    assert_eq!(
        realm_unit(&after, &enemy_id).expect("ward survivor")["warded"],
        false
    );
    assert_eq!(
        realm_unit(&after, &ally_id).expect("unharmed ally")["damage"],
        0
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0711_duel_magic_fights_through_avatar_ally_adjacent_to_enemy_minion() {
    let encoded = seed_with(false, 711);
    let (mut session, _ally_id, caster_id, enemy_id) = setup_duel(&encoded);
    let north_avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();

    let (descriptor, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == north_avatar_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert_eq!(descriptor["ally"]["kind"], "avatar");
    assert_eq!(descriptor["target"]["kind"], "minion");
    assert_eq!(
        event_types(&fight),
        [
            "magic-cast",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
            "damage-dealt",
            "magic-resolved",
        ]
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    assert!(realm_unit(&state(&session), &enemy_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0712_duel_checkpoints_underground_first_strike_and_finishes_before_terminal() {
    let encoded = seed_underground_duel_checkpoint(712);
    let (mut session, ally_id, mut target_ids) =
        try_setup_underground_duel_checkpoint(&encoded).expect("complete underground Duel setup");
    target_ids.sort_unstable();
    let target_id = target_ids[0].clone();
    let duel_action = session
        .legal_actions()
        .expect("underground Duel actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-duel"
                && action.descriptor["ally"]["instanceId"] == ally_id
                && action.descriptor["target"]["instanceId"] == target_id
        })
        .expect("engine-issued underground Duel");
    let StepResult::Accepted(cast) = session
        .step(ActionRequest {
            action_id: duel_action.action_id.to_string(),
            seat: duel_action.seat,
            state_version: duel_action.state_version,
        })
        .expect("underground Duel cast")
    else {
        panic!("engine-issued underground Duel must be accepted");
    };
    assert_eq!(
        event_types(&cast),
        [
            "magic-cast",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
        ]
    );
    let pending = state(&session);
    assert_eq!(pending["phase"], "trigger-order");
    assert_eq!(pending["decisionSeat"], "south");
    assert_eq!(
        pending["pendingDeathrites"]["continuation"]["kind"],
        "first-strike"
    );
    assert_eq!(
        pending["pendingDeathrites"]["continuation"]["pending"]["region"],
        "underground"
    );
    assert_eq!(
        pending["pendingDeathrites"]["deferredOutcomes"],
        json!([{
            "payload": {
                "cardId": "north-duel",
                "instanceId": cast.events[0].payload["instanceId"],
                "owner": "north",
            },
            "type": "magic-resolved",
        }])
    );

    let checkpoint = create_game_checkpoint(&session).expect("pending Duel checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized pending Duel");
    session = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed pending Duel checkpoint"),
    )
    .expect("resumed pending Duel checkpoint");
    assert_eq!(state(&session), pending);
    let (_, completed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == target_ids[0]
    });
    let completed_types = event_types(&completed);
    assert_eq!(
        &completed_types[completed_types.len() - 2..],
        ["magic-resolved", "game-ended"]
    );
    assert_eq!(state(&session)["phase"], "terminal");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0946_duel_magic_fight_strike_deals_double_damage_when_struck_unit_is_nearby_mask() {
    let encoded = seed_duel_with_mask(946, true);
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel_with_mask(&encoded, true);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert!(event_types(&receipt).contains(&"fight-started"));
    assert_eq!(damage_dealt_amount(&receipt, &enemy_id), 2);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert_eq!(
        realm_unit(&state(&session), &ally_id).expect("ally survives the doubled return strike")["damage"],
        2
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0965_duel_magic_fight_strike_is_not_doubled_when_struck_unit_is_not_nearby_mask() {
    let encoded = seed_duel_with_mask(965, false);
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel_with_mask(&encoded, false);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert!(event_types(&receipt).contains(&"fight-started"));
    assert_eq!(damage_dealt_amount(&receipt, &enemy_id), 1);
    assert!(!event_types(&receipt).contains(&"minion-died"));
    let after = state(&session);
    let enemy = realm_unit(&after, &enemy_id)
        .expect("2-defense minion survives an undoubled 1-power Duel strike");
    assert_eq!(enemy["damage"], 1);
    assert_eq!(
        realm_unit(&after, &ally_id).expect("ally survives the return strike")["damage"],
        1
    );
    assert_exact_replay(&session);
}
#[test]
fn rule_catalog_1007_duel_kill_triggers_deathrite_draw_before_magic_resolved() {
    let encoded = seed_duel_deathrite(1007);
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel(&encoded);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    assert_eq!(
        south_atlas, 1,
        "thin South atlas leaves one site before the Duel kill"
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    let types = event_types(&receipt);
    assert!(types.contains(&"fight-started"));
    assert!(types.contains(&"damage-dealt"));
    assert!(types.contains(&"minion-died"));
    assert!(types.contains(&"site-drawn"));
    assert!(types.contains(&"magic-resolved"));

    let fight_damage = types
        .iter()
        .rposition(|event_type| *event_type == "damage-dealt")
        .expect("fight damage-dealt");
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let minion_died = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion-died index");
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        fight_damage < site_drawn && site_drawn < minion_died && minion_died < magic_resolved,
        "expected fight damage, deathrite site-drawn, minion-died, then magic-resolved; got {types:?}"
    );

    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], enemy_id);

    let finished = state(&session);
    assert!(realm_unit(&finished, &enemy_id).is_none());
    assert!(realm_unit(&finished, &ally_id).is_some());
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1045_duel_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_duel_seed_with(1045);
    let mut setup = try_pending_deathrite_with_ready_duel(&encoded)
        .expect("complete duel Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
    let caster_id = setup.caster_id.clone();
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
    assert!(realm_unit(&paused, &ally_id).is_some());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(duel_targets(session).is_empty());

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
    let expected = (ally_id.clone(), caster_id.clone(), visitor_id.clone());
    assert!(
        duel_targets(session).contains(&expected),
        "expected duel target {expected:?} among {:?}",
        duel_targets(session)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert!(event_types(&receipt).contains(&"fight-started"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    assert!(realm_unit(&state(session), &visitor_id).is_none());
    assert!(realm_unit(&state(session), &ally_id).is_some());
    assert_exact_replay(session);
}

fn duel_supplemental_manifest(seed: u32, nonlethal: bool) -> String {
    let (ally_stats, enemy_stats) = if nonlethal {
        (
            json!({ "attack": 1, "defense": 5 }),
            json!({ "attack": 1, "defense": 5 }),
        )
    } else {
        (
            json!({ "attack": 3, "defense": 4 }),
            json!({ "attack": 2, "defense": 3 }),
        )
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "duel-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-duel-supplemental-v1",
        },
        "cards": {
            "north-ally": minion(ally_stats),
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "spellcaster": true })),
            "north-duel": duel(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": minion(enemy_stats),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-caster",
                    "north-ally",
                    "north-caster",
                    "north-duel",
                    "north-duel",
                    "north-duel",
                    "north-duel",
                    "north-ally",
                    "north-caster",
                    "north-duel",
                    "north-duel",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 12],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn duel_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-duel")
                .count()
        })
        .unwrap_or_default()
}

fn cast_duel(session: &mut Session, ally_id: &str, caster_id: &str, enemy_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    receipt
}

fn try_cast_duel(
    session: &mut Session,
    ally_id: &str,
    caster_id: &str,
    enemy_id: &str,
) -> Option<Receipt> {
    let (_, receipt) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    })?;
    Some(receipt)
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

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn duel_ready_for_target(
    session: &Session,
    ally_id: &str,
    caster_id: &str,
    enemy_id: &str,
) -> bool {
    duel_targets(session).contains(&(
        ally_id.to_owned(),
        caster_id.to_owned(),
        enemy_id.to_owned(),
    ))
}

fn duel_minion_ally_targets(session: &Session) -> Vec<String> {
    let snapshot = state(session);
    let mut allies: Vec<_> = duel_targets(session)
        .into_iter()
        .filter_map(|(ally_id, _caster_id, _enemy_id)| {
            realm_unit(&snapshot, &ally_id)
                .and_then(|unit| (unit["cardId"] == "north-ally").then_some(ally_id))
        })
        .collect();
    allies.sort_unstable();
    allies.dedup();
    allies
}

fn try_summon_south_enemy_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn try_summon_north_ally_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn try_two_ally_duel_prefix(encoded: &str) -> Option<(Session, Vec<String>, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let first = try_summon_north_ally_at(&mut session, "C4")?;
    let (caster_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    })?;
    let caster_id = caster_summon["cardInstanceId"].as_str()?.to_owned();
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let second = try_summon_north_ally_at(&mut session, "C2")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let enemy_id = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let offered = duel_minion_ally_targets(&session);
    (offered.contains(&first) && offered.contains(&second)).then_some((
        session,
        vec![first, second],
        caster_id,
        enemy_id,
    ))
}

fn seed_for_two_ally_duel_targets(start: u32) -> String {
    (start..start + 16384)
        .chain(603..603 + 16384)
        .find_map(|seed| {
            try_two_ally_duel_prefix(&duel_supplemental_manifest(seed, false)).map(|_| seed)
        })
        .map(|seed| duel_supplemental_manifest(seed, false))
        .expect("bounded seed reaching two-ally Duel target setup")
}

fn try_far_enemy_duel_prefix(encoded: &str) -> Option<(Session, String, String, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally_id = try_summon_north_ally_at(&mut session, "C4")?;
    let (caster_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    })?;
    let caster_id = caster_summon["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let far_id = try_summon_south_enemy_at(&mut session, "C1")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let nearby_id = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    duel_ready_for_target(&session, &ally_id, &caster_id, &nearby_id)
        .then_some((session, ally_id, caster_id, nearby_id, far_id))
}

fn seed_for_far_enemy_duel(start: u32) -> String {
    (start..start + 16384)
        .chain(603..603 + 16384)
        .find_map(|seed| {
            try_far_enemy_duel_prefix(&duel_supplemental_manifest(seed, false)).map(|_| seed)
        })
        .map(|seed| duel_supplemental_manifest(seed, false))
        .expect("bounded seed reaching Duel far-enemy setup")
}

fn try_second_duel_new_enemy_prefix(encoded: &str) -> Option<(Session, String, String, String)> {
    let (mut session, ally_id, caster_id, first_enemy) = try_setup_duel(encoded)?;
    let receipt = try_cast_duel(&mut session, &ally_id, &caster_id, &first_enemy)?;
    if !event_types(&receipt).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &first_enemy).is_some() {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if duel_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let second_enemy = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    duel_ready_for_target(&session, &ally_id, &caster_id, &second_enemy).then_some((
        session,
        ally_id,
        caster_id,
        second_enemy,
    ))
}

fn seed_for_second_duel_new_enemy(start: u32) -> String {
    (start..start + 16384)
        .chain(603..603 + 16384)
        .find_map(|seed| {
            try_second_duel_new_enemy_prefix(&duel_supplemental_manifest(seed, false)).map(|_| seed)
        })
        .map(|seed| duel_supplemental_manifest(seed, false))
        .expect("bounded seed reaching second Duel new-enemy setup")
}

fn try_second_duel_enemy_arrival_prefix(
    encoded: &str,
) -> Option<(Session, String, String, String)> {
    let (mut session, ally_id, caster_id, first_enemy) = try_setup_duel(encoded)?;
    let receipt = try_cast_duel(&mut session, &ally_id, &caster_id, &first_enemy)?;
    if !event_types(&receipt).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &first_enemy).is_some() {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if duel_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let second_enemy = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    duel_ready_for_target(&session, &ally_id, &caster_id, &second_enemy).then_some((
        session,
        ally_id,
        caster_id,
        second_enemy,
    ))
}

fn seed_for_second_duel_enemy_arrival(start: u32) -> String {
    (start..start + 16384)
        .chain(603..603 + 16384)
        .find_map(|seed| {
            try_second_duel_enemy_arrival_prefix(&duel_supplemental_manifest(seed, false))
                .map(|_| seed)
        })
        .map(|seed| duel_supplemental_manifest(seed, false))
        .expect("bounded seed reaching second Duel enemy-arrival setup")
}

#[test]
fn rule_catalog_1983_dueled_units_stay_on_the_board_after_turns_pass() {
    let encoded = (1983..1983 + 8192)
        .chain(603..603 + 8192)
        .find_map(|seed| {
            let candidate = duel_supplemental_manifest(seed, true);
            try_setup_duel(&candidate).is_some().then_some(candidate)
        })
        .expect("bounded nonlethal seed with complete Duel setup");
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel(&encoded);
    cast_duel(&mut session, &ally_id, &caster_id, &enemy_id);
    let ally_location = unit(&state(&session), &ally_id)["location"].clone();
    let enemy_location = unit(&state(&session), &enemy_id)["location"].clone();
    advance_full_round(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["location"], ally_location);
    assert_eq!(
        unit(&state(&session), &enemy_id)["location"],
        enemy_location
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1984_second_duel_without_adjacent_enemy_is_not_castable() {
    let encoded = (1984..1984 + 8192)
        .chain(603..603 + 8192)
        .map(|seed| duel_supplemental_manifest(seed, false))
        .find(|candidate| {
            let Some((mut session, ally_id, caster_id, enemy_id)) = try_setup_duel(candidate)
            else {
                return false;
            };
            if duel_spells_in_hand(&state(&session)) < 2 {
                return false;
            }
            let first = cast_duel(&mut session, &ally_id, &caster_id, &enemy_id);
            if !event_types(&first).contains(&"minion-died") {
                return false;
            }
            realm_unit(&state(&session), &enemy_id).is_none()
                && duel_spells_in_hand(&state(&session)) >= 1
                && duel_targets(&session).is_empty()
        })
        .expect("bounded seed with withheld second Duel after setup");
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel(&encoded);
    let first = cast_duel(&mut session, &ally_id, &caster_id, &enemy_id);
    assert!(event_types(&first).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert!(duel_spells_in_hand(&state(&session)) >= 1);
    assert!(duel_targets(&session).is_empty());
    assert!(
        !session
            .legal_actions()
            .expect("legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["cardId"] == "north-duel"
            })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1985_second_duel_fights_a_newly_arrived_enemy_after_enemy_site_placement() {
    let encoded = seed_for_second_duel_enemy_arrival(1985);
    let (mut session, ally_id, caster_id, enemy_id) =
        try_second_duel_enemy_arrival_prefix(&encoded).expect("second Duel enemy-arrival prefix");
    let receipt = cast_duel(&mut session, &ally_id, &caster_id, &enemy_id);
    assert!(event_types(&receipt).contains(&"fight-started"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1986_duel_offers_every_controlled_north_ally_adjacent_to_enemy_at_c3() {
    let encoded = seed_for_two_ally_duel_targets(1986);
    let (session, ally_ids, _caster_id, _enemy_id) =
        try_two_ally_duel_prefix(&encoded).expect("two-ally Duel prefix");
    let offered = duel_minion_ally_targets(&session);
    assert_eq!(offered.len(), 2);
    for ally_id in &ally_ids {
        assert!(offered.contains(ally_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1987_duel_leaves_a_far_enemy_untouched() {
    let encoded = seed_for_far_enemy_duel(1987);
    let (mut session, ally_id, caster_id, nearby_id, far_id) =
        try_far_enemy_duel_prefix(&encoded).expect("Duel far-enemy prefix");
    let far_location = unit(&state(&session), &far_id)["location"].clone();
    let far_damage = unit(&state(&session), &far_id)["damage"].clone();
    let receipt = cast_duel(&mut session, &ally_id, &caster_id, &nearby_id);
    assert!(event_types(&receipt).contains(&"fight-started"));
    assert!(realm_unit(&state(&session), &nearby_id).is_none());
    assert_eq!(unit(&state(&session), &far_id)["location"], far_location);
    assert_eq!(unit(&state(&session), &far_id)["damage"], far_damage);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1988_second_duel_fights_a_newly_summoned_enemy() {
    let encoded = seed_for_second_duel_new_enemy(1988);
    let (mut session, ally_id, caster_id, enemy_id) =
        try_second_duel_new_enemy_prefix(&encoded).expect("second Duel new-enemy prefix");
    let receipt = cast_duel(&mut session, &ally_id, &caster_id, &enemy_id);
    assert!(event_types(&receipt).contains(&"fight-started"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert_exact_replay(&session);
}
