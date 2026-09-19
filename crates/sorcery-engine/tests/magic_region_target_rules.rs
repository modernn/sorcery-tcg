//! Direct proofs for Magic target filters by caster region and enemy Stealth
//! (RULE-CATALOG-0023, RULE-CATALOG-0697, RULE-CATALOG-2453–2458).
//!
//! Targeted Magic offers only units in the caster region. Enemy Stealth is
//! excluded; own Stealth stays targetable. Distinct from 0617–0618 (Grant-
//! Stealth), from 0673–0674 (Fatality's wounded-minion region filter), and
//! from 0698 (Freeze Disable killing an underground burrower). Supplemental
//! 2453–2458 bind persistence, empty-repeat, enemy-arrival, multi-minion,
//! underground, and a newly summoned surface minion. Seed search starts at
//! the catalog id and falls back to 697.

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
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bury() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn region_target_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "magic-region-target" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-magic-region-target-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-magic": zap(),
            "north-minion": minion(json!({ "burrowing": true, "stealth": true })),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-plain": minion(json!({})),
            "south-site": site(),
            "south-stealth": minion(json!({ "stealth": true })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-bury",
                    "north-bury",
                    "north-magic",
                    "north-magic",
                    "north-minion",
                    "north-minion",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-plain",
                    "south-plain",
                    "south-plain",
                    "south-stealth",
                    "south-stealth",
                    "south-stealth",
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid region-target session");
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

fn magic_target_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("targeted Magic actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-magic"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn opening_has_all(encoded: &str, card_ids: &[&str]) -> bool {
    Session::new(encoded).ok().is_some_and(|preview| {
        state(&preview)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| {
                card_ids
                    .iter()
                    .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
            })
    })
}

fn seed_with(start: u32, need: &[&str]) -> String {
    (start..start + 512)
        .map(region_target_manifest)
        .find(|candidate| opening_has_all(candidate, need))
        .expect("bounded seed with required Magic targeting cards")
}

fn summon_north_minion(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("friendly minion identity")
        .to_owned()
}

fn south_plays_c1_and_summons(
    session: &mut Session,
    stealth: bool,
    plain: bool,
) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let stealth_id = if stealth {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-stealth"
                && descriptor["region"].is_null()
        });
        summoned["cardInstanceId"]
            .as_str()
            .expect("enemy Stealth identity")
            .to_owned()
    } else {
        String::new()
    };
    let plain_id = if plain {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-plain"
                && descriptor["region"].is_null()
        });
        summoned["cardInstanceId"]
            .as_str()
            .expect("plain enemy identity")
            .to_owned()
    } else {
        String::new()
    };
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (stealth_id, plain_id)
}

#[test]
fn rule_catalog_0697_magic_targets_stay_in_the_caster_region_not_underground() {
    let encoded = seed_with(697, &["north-bury", "north-magic", "north-minion"]);
    let mut session = opening_main(&encoded);
    let friendly_id = summon_north_minion(&mut session);
    let (_, plain_id) = south_plays_c1_and_summons(&mut session, false, true);

    let surface = magic_target_ids(&session);
    assert!(
        surface.contains(&friendly_id),
        "a surface ally stays targetable"
    );
    assert!(
        surface.contains(&plain_id),
        "an exposed surface enemy stays targetable"
    );

    let bury = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();
    let burrowed = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == bury
            && descriptor["target"]["instanceId"] == friendly_id
    })
    .1;
    assert_eq!(
        event_types(&burrowed),
        ["magic-cast", "minion-burrowed", "magic-resolved"]
    );
    assert_eq!(
        realm_unit(&state(&session), &friendly_id).expect("burrowed ally")["region"],
        "underground"
    );

    let confined = magic_target_ids(&session);
    assert!(
        !confined.contains(&friendly_id),
        "a surface caster may not reach its own burrowed ally"
    );
    assert!(
        confined.contains(&plain_id),
        "the surface enemy stays reachable from the surface"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0023_magic_targets_exclude_enemy_stealth() {
    let encoded = seed_with(23, &["north-magic", "north-minion"]);
    let mut session = opening_main(&encoded);
    let friendly_id = summon_north_minion(&mut session);
    let (stealth_id, plain_id) = south_plays_c1_and_summons(&mut session, true, true);

    let surface = magic_target_ids(&session);
    assert!(
        surface.contains(&friendly_id),
        "own Stealth stays targetable"
    );
    assert!(
        surface.contains(&plain_id),
        "an exposed enemy stays targetable"
    );
    assert!(
        !surface.contains(&stealth_id),
        "enemy active Stealth must never be offered"
    );
    assert_exact_replay(&session);
}

fn region_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "magic-region-target-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-magic-region-target-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-magic": zap(),
            "north-minion": minion(json!({ "burrowing": true })),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-plain": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": std::iter::repeat_n("north-magic", 8)
                    .chain(std::iter::repeat_n("north-bury", 4))
                    .chain(std::iter::repeat_n("north-minion", 4))
                    .collect::<Vec<_>>(),
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-plain"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    realm_unit(snapshot, instance_id).expect("expected realm unit")
}

fn magic_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-magic")
                .count()
        })
        .unwrap_or_default()
}

fn opening_south_plains(encoded: &str) -> usize {
    Session::new(encoded)
        .ok()
        .and_then(|preview| {
            state(&preview)["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .map(|hand| {
                    hand.iter()
                        .filter(|card| card["cardId"] == "south-plain")
                        .count()
                })
        })
        .unwrap_or_default()
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

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .ok()
        .is_some_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
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

fn try_draw_any(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })
    .map(|_| ())
}

fn try_summon_north_minion(session: &mut Session) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["region"].is_null()
    })?;
    summoned["cardInstanceId"].as_str().map(str::to_owned)
}

fn try_south_plays_c1_and_summons(session: &mut Session, count: usize) -> Option<Vec<String>> {
    end_turn_if_offered(session);
    try_draw_any(session)?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let mut enemy_ids = Vec::new();
    for _ in 0..count {
        let (summoned, _) = try_accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-plain"
                && descriptor["region"].is_null()
        })?;
        enemy_ids.push(summoned["cardInstanceId"].as_str()?.to_owned());
    }
    end_turn_if_offered(session);
    try_draw_any(session)?;
    Some(enemy_ids)
}

fn try_bury(session: &mut Session, instance_id: &str) -> Option<Receipt> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["instanceId"] == instance_id
    })
    .map(|(_, receipt)| receipt)
}

fn try_zap(session: &mut Session, instance_id: &str) -> Option<Receipt> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-magic"
            && descriptor["target"]["instanceId"] == instance_id
    })
    .map(|(_, receipt)| receipt)
}

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    end_turn_if_offered(session);
    try_draw_any(session)?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn try_opening_bury(encoded: &str, south_count: usize) -> Option<(Session, String, Vec<String>)> {
    if !opening_has_all(encoded, &["north-bury", "north-magic", "north-minion"])
        || opening_south_plains(encoded) < south_count
    {
        return None;
    }
    let mut session = opening_main(encoded);
    let friendly_id = try_summon_north_minion(&mut session)?;
    let surface_ids = try_south_plays_c1_and_summons(&mut session, south_count)?;
    let buried = try_bury(&mut session, &friendly_id)?;
    if !event_types(&buried).contains(&"minion-burrowed") {
        return None;
    }
    if unit(&state(&session), &friendly_id)["region"] != "underground" {
        return None;
    }
    let offered = magic_target_ids(&session);
    if offered.contains(&friendly_id) || surface_ids.iter().any(|id| !offered.contains(id)) {
        return None;
    }
    Some((session, friendly_id, surface_ids))
}

fn seed_for_opening_bury(start: u32, south_count: usize) -> String {
    (start..start + 8192)
        .chain(697..697 + 8192)
        .map(region_supplemental_manifest)
        .find(|candidate| try_opening_bury(candidate, south_count).is_some())
        .expect("bounded seed with caster-region bury setup")
}

fn try_empty_repeat_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let (mut session, friendly_id, surface_ids) = try_opening_bury(encoded, 1)?;
    if magic_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let surface_id = surface_ids[0].clone();
    let zapped = try_zap(&mut session, &surface_id)?;
    if !event_types(&zapped).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &surface_id).is_some() {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    let offered = magic_target_ids(&session);
    (magic_spells_in_hand(&state(&session)) >= 1 && !offered.contains(&friendly_id)).then_some((
        session,
        friendly_id,
        surface_id,
    ))
}

fn seed_for_empty_repeat(start: u32) -> String {
    (start..start + 8192)
        .chain(697..697 + 8192)
        .map(region_supplemental_manifest)
        .find(|candidate| try_empty_repeat_prefix(candidate).is_some())
        .expect("bounded seed with caster-region empty-repeat setup")
}

fn try_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let (mut session, friendly_id, _) = try_opening_bury(encoded, 1)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    if magic_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_draw_any(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-plain"
            && descriptor["cell"] == "C2"
            && descriptor["region"].is_null()
    })?;
    let visitor_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let offered = magic_target_ids(&session);
    (offered.contains(&visitor_id) && !offered.contains(&friendly_id)).then_some((
        session,
        visitor_id,
        friendly_id,
    ))
}

fn seed_for_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(697..697 + 8192)
        .map(region_supplemental_manifest)
        .find(|candidate| {
            opening_south_plains(candidate) >= 2 && try_enemy_arrival_prefix(candidate).is_some()
        })
        .expect("bounded seed with caster-region enemy-arrival setup")
}

fn try_new_summon_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let (mut session, friendly_id, _) = try_opening_bury(encoded, 1)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    if magic_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_draw_any(&mut session)?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-plain"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let new_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let offered = magic_target_ids(&session);
    (offered.contains(&new_id) && !offered.contains(&friendly_id)).then_some((
        session,
        new_id,
        friendly_id,
    ))
}

fn seed_for_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(697..697 + 8192)
        .map(region_supplemental_manifest)
        .find(|candidate| {
            opening_south_plains(candidate) >= 2 && try_new_summon_prefix(candidate).is_some()
        })
        .expect("bounded seed with caster-region new-summon setup")
}

#[test]
fn rule_catalog_2453_burrowed_ally_stays_unoffered_after_turns_pass() {
    let encoded = seed_for_opening_bury(2453, 1);
    let (mut session, friendly_id, surface_ids) =
        try_opening_bury(&encoded, 1).expect("caster-region persistence prefix");
    let surface_id = surface_ids[0].clone();
    assert_eq!(
        unit(&state(&session), &friendly_id)["region"],
        "underground"
    );
    try_pass_turn_to_north_spellbook(&mut session).expect("turn cycle after bury");
    let offered = magic_target_ids(&session);
    assert!(!offered.contains(&friendly_id));
    assert!(offered.contains(&surface_id));
    assert_eq!(
        unit(&state(&session), &friendly_id)["region"],
        "underground"
    );
    assert_eq!(unit(&state(&session), &friendly_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2454_second_zap_still_omits_the_burrowed_ally() {
    let encoded = seed_for_empty_repeat(2454);
    let (session, friendly_id, surface_id) =
        try_empty_repeat_prefix(&encoded).expect("caster-region empty-repeat prefix");
    assert!(realm_unit(&state(&session), &surface_id).is_none());
    assert_eq!(
        unit(&state(&session), &friendly_id)["region"],
        "underground"
    );
    assert!(magic_spells_in_hand(&state(&session)) >= 1);
    assert!(!magic_target_ids(&session).contains(&friendly_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2455_zap_offers_a_newly_arrived_surface_minion_after_enemy_site_placement() {
    let encoded = seed_for_enemy_arrival(2455);
    let (mut session, visitor_id, friendly_id) =
        try_enemy_arrival_prefix(&encoded).expect("caster-region enemy-arrival prefix");
    let receipt = try_zap(&mut session, &visitor_id).expect("zap newly arrived surface minion");
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &visitor_id).is_none());
    assert_eq!(
        unit(&state(&session), &friendly_id)["region"],
        "underground"
    );
    assert_eq!(unit(&state(&session), &friendly_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2456_zap_offers_every_surface_minion_not_underground() {
    let encoded = seed_for_opening_bury(2456, 2);
    let (session, friendly_id, surface_ids) =
        try_opening_bury(&encoded, 2).expect("caster-region multi-minion prefix");
    let offered = magic_target_ids(&session);
    assert!(offered.contains(&surface_ids[0]));
    assert!(offered.contains(&surface_ids[1]));
    assert!(!offered.contains(&friendly_id));
    assert_eq!(
        unit(&state(&session), &friendly_id)["region"],
        "underground"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2457_zap_leaves_a_burrowed_ally_untouched() {
    let encoded = seed_for_opening_bury(2457, 1);
    let (mut session, friendly_id, surface_ids) =
        try_opening_bury(&encoded, 1).expect("caster-region underground prefix");
    let surface_id = surface_ids[0].clone();
    let receipt = try_zap(&mut session, &surface_id).expect("zap surface enemy");
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &surface_id).is_none());
    assert_eq!(
        unit(&state(&session), &friendly_id)["region"],
        "underground"
    );
    assert_eq!(unit(&state(&session), &friendly_id)["location"], "C4");
    assert!(realm_unit(&state(&session), &friendly_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2458_zap_offers_a_newly_summoned_surface_minion() {
    let encoded = seed_for_new_summon(2458);
    let (mut session, new_id, friendly_id) =
        try_new_summon_prefix(&encoded).expect("caster-region new-summon prefix");
    let receipt = try_zap(&mut session, &new_id).expect("zap newly summoned surface minion");
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &new_id).is_none());
    assert_eq!(
        unit(&state(&session), &friendly_id)["region"],
        "underground"
    );
    assert_exact_replay(&session);
}
