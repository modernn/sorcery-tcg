//! Direct proofs for fight-ally-with-adjacent-enemy Magic (RULE-CATALOG-0603–0604, 0711–0712, 0946, 0965, 1007).
//!
//! Duel makes a chosen ally fight a targeted adjacent enemy through the shared
//! fight pipeline. Ward on the target breaks without entering combat. Avatar allies
//! route strike damage through avatar-life-lost instead of minion damage. Underground
//! first-strike Duels pause in deathrite-order, survive checkpoint round-trip, and
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
    assert_eq!(pending["phase"], "deathrite-order");
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
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == target_ids[0]
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
    assert_eq!(south_atlas, 1, "thin South atlas leaves one site before the Duel kill");

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
        fight_damage < site_drawn
            && site_drawn < minion_died
            && minion_died < magic_resolved,
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

