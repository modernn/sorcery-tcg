//! Blink Magic self-play admission matrix (RULE-CATALOG-0733) and moved-caster
//! nearby-ally supplemental proofs (RULE-CATALOG-2563–2568).
//!
//! After the caster Avatar steps from C4 to C3, `teleportNearbyAllyThenDrawCard`
//! offers adjacent C2 allies and not the two-step C1 ally. Distinct from 1123–1124,
//! which prove destination and aura settlement on an unmoved board.

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

fn blink() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "teleportNearbyAllyThenDrawCard": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn north_ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
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

fn range_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "blink-admission-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-blink-admission-supplemental-v1",
        },
        "cards": {
            "north-ally": north_ally(),
            "north-avatar": avatar(),
            "north-blink": blink(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 4,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-ally",
                    "north-ally",
                    "north-ally",
                    "north-blink",
                    "north-blink",
                    "north-blink",
                ],
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
    let Ok(StepResult::Accepted(receipt)) = session.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    }) else {
        return None;
    };
    Some((descriptor, receipt))
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid Blink admission session");
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
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

fn blink_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-blink")
                .count()
        })
        .unwrap_or_default()
}

fn blink_minion_ally_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Blink actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-blink"
                && action.descriptor["ally"]["kind"] == "minion"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn unit_location(snapshot: &Value, instance_id: &str) -> String {
    unit(snapshot, instance_id)["location"]
        .as_str()
        .expect("unit location")
        .to_owned()
}

fn is_nearby_to_c3(cell: &str) -> bool {
    matches!(cell, "C2" | "C4" | "D3" | "B3" | "D2" | "B2" | "D4" | "B4")
}

fn blink_nearby_minion_ally_targets(session: &Session) -> Vec<String> {
    let snapshot = state(session);
    blink_minion_ally_targets(session)
        .into_iter()
        .filter(|ally_id| is_nearby_to_c3(&unit_location(&snapshot, ally_id)))
        .collect()
}

fn cast_blink_on(session: &mut Session, ally_id: &str, cell: &str, zone: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-blink"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["drawZone"] == zone
    });
    receipt
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

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D4"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn try_summon_north_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn try_play_site_at(session: &mut Session, cell: &str) -> Option<()> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    })
    .map(|_| ())
}

fn try_move_north_avatar_to_c3(session: &mut Session) -> Option<()> {
    let avatar_id = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == avatar_id.as_str()
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    })?;
    let _ = try_accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    Some(())
}

fn pass_south_then_north_draw(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })
    .map(|_| ())
}

/// Caster at C3 with adjacent allies at C2 only.
fn try_adjacent_only_opening(session: &mut Session, near_count: usize) -> Option<Vec<String>> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_play_site_at(session, "C1")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_play_site_at(session, "C3")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_play_site_at(session, "C2")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let near_ids: Vec<String> = (0..near_count)
        .map(|_| try_summon_north_at(session, "C2"))
        .collect::<Option<_>>()?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_move_north_avatar_to_c3(session)?;
    Some(near_ids)
}

/// Caster at C3, adjacent allies at C2, two-step ally at C1.
fn try_range_opening(session: &mut Session, near_count: usize) -> Option<(String, Vec<String>)> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_play_site_at(session, "C1")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let far_id = try_summon_north_at(session, "C1")?;
    try_play_site_at(session, "C3")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_play_site_at(session, "C2")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let near_ids: Vec<String> = (0..near_count)
        .map(|_| try_summon_north_at(session, "C2"))
        .collect::<Option<_>>()?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    pass_south_then_north_draw(session)?;
    try_move_north_avatar_to_c3(session)?;
    Some((far_id, near_ids))
}

fn range_opening(session: &mut Session, near_count: usize) -> (String, Vec<String>) {
    try_range_opening(session, near_count).expect("moved-caster nearby-ally opening")
}

fn supplemental_seed_with_start(start: u32, near_count: usize) -> String {
    (start..start + 2048)
        .chain(733..733 + 2048)
        .find_map(|seed| {
            let candidate = range_supplemental_manifest(seed);
            let mut session = opening_main(&candidate);
            try_range_opening(&mut session, near_count).map(|_| candidate)
        })
        .expect("bounded seed with Blink and required North allies")
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

fn assert_near_allies_offered(session: &Session, near_ids: &[String]) {
    let offered = blink_nearby_minion_ally_targets(session);
    for near_id in near_ids {
        assert!(offered.contains(near_id));
    }
}

fn try_second_blink_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let near_ids = try_adjacent_only_opening(&mut session, 1)?;
    let first = cast_blink_on(&mut session, &near_ids[0], "C1", "spellbook");
    if !event_types(&first).contains(&"unit-teleported") {
        return None;
    }
    if blink_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && (descriptor["cell"] == "D2" || descriptor["cell"] == "B2")
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let ally_id = try_summon_north_at(&mut session, "C4")?;
    blink_nearby_minion_ally_targets(&session)
        .contains(&ally_id)
        .then_some((session, ally_id))
}

fn seed_for_second_blink_enemy_arrival(start: u32) -> String {
    (start..start + 2048)
        .chain(733..733 + 2048)
        .find_map(|seed| {
            let encoded = range_supplemental_manifest(seed);
            try_second_blink_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second nearby-ally enemy-arrival setup")
}

fn try_second_blink_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let near_ids = try_adjacent_only_opening(&mut session, 1)?;
    let first = cast_blink_on(&mut session, &near_ids[0], "C1", "spellbook");
    if !event_types(&first).contains(&"unit-teleported") {
        return None;
    }
    if blink_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let ally_id = try_summon_north_at(&mut session, "C2")?;
    blink_nearby_minion_ally_targets(&session)
        .contains(&ally_id)
        .then_some((session, ally_id))
}

fn seed_for_second_blink_new_summon(start: u32) -> String {
    (start..start + 2048)
        .chain(733..733 + 2048)
        .find_map(|seed| {
            let encoded = range_supplemental_manifest(seed);
            try_second_blink_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second nearby-ally new-summon setup")
}

#[test]
fn rule_catalog_0733_blink_magic_admits_minion_slices_and_common_modifiers() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0733_blink_magic_admits_minion_slices_and_common_modifiers(
    );
}

#[test]
fn rule_catalog_2563_blinked_ally_stays_at_the_destination_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2563, 1);
    let mut session = opening_main(&encoded);
    let (_far_id, near_ids) = range_opening(&mut session, 1);
    let near_id = &near_ids[0];
    assert_near_allies_offered(&session, near_ids.as_slice());
    let receipt = cast_blink_on(&mut session, near_id, "C1", "spellbook");
    assert!(event_types(&receipt).contains(&"unit-teleported"));
    assert_eq!(unit(&state(&session), near_id)["location"], "C1");
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), near_id)["location"], "C1");
    assert_eq!(unit(&state(&session), near_id)["controller"], "north");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2564_second_blink_offers_no_nearby_allies_after_the_only_copy_teleports_out_of_range()
 {
    let encoded = (2564..2564 + 2048)
        .chain(733..733 + 2048)
        .find_map(|seed| {
            let candidate = range_supplemental_manifest(seed);
            if opening_hand_spell_ids(&candidate, "north")
                .iter()
                .filter(|card| *card == "north-ally")
                .count()
                < 1
            {
                return None;
            }
            let mut session = opening_main(&candidate);
            let near_ids = try_adjacent_only_opening(&mut session, 1)?;
            let first = cast_blink_on(&mut session, &near_ids[0], "C1", "spellbook");
            if !event_types(&first).contains(&"unit-teleported") {
                return None;
            }
            if blink_spells_in_hand(&state(&session)) < 1 {
                return None;
            }
            blink_nearby_minion_ally_targets(&session)
                .is_empty()
                .then_some(candidate)
        })
        .expect("bounded seed with Blink leaving no nearby minion allies after teleporting away");
    let mut session = opening_main(&encoded);
    let near_ids =
        try_adjacent_only_opening(&mut session, 1).expect("single adjacent ally opening");
    let near_id = &near_ids[0];
    let first = cast_blink_on(&mut session, near_id, "C1", "spellbook");
    assert!(event_types(&first).contains(&"unit-teleported"));
    assert_eq!(unit(&state(&session), near_id)["location"], "C1");
    assert!(blink_spells_in_hand(&state(&session)) >= 1);
    assert!(blink_nearby_minion_ally_targets(&session).is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2565_second_blink_teleports_a_newly_arrived_adjacent_ally_after_enemy_site_placement()
 {
    let encoded = seed_for_second_blink_enemy_arrival(2565);
    let (mut session, ally_id) = try_second_blink_enemy_arrival_prefix(&encoded)
        .expect("second nearby-ally enemy-arrival prefix");
    let receipt = cast_blink_on(&mut session, &ally_id, "C3", "spellbook");
    assert!(event_types(&receipt).contains(&"unit-teleported"));
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C3");
    assert_eq!(unit(&state(&session), &ally_id)["controller"], "north");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2566_blink_offers_every_adjacent_ally_after_the_caster_moves() {
    let encoded = supplemental_seed_with_start(2566, 2);
    let mut session = opening_main(&encoded);
    let (far_id, near_ids) = range_opening(&mut session, 2);
    let offered = blink_nearby_minion_ally_targets(&session);
    for ally_id in &near_ids {
        assert!(offered.contains(ally_id));
    }
    assert!(!offered.contains(&far_id));
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2567_blink_leaves_a_two_step_ally_untouched() {
    let encoded = supplemental_seed_with_start(2567, 1);
    let mut session = opening_main(&encoded);
    let (far_id, near_ids) = range_opening(&mut session, 1);
    let near_id = &near_ids[0];
    assert_near_allies_offered(&session, near_ids.as_slice());
    let receipt = cast_blink_on(&mut session, near_id, "C3", "spellbook");
    assert!(event_types(&receipt).contains(&"unit-teleported"));
    assert_eq!(unit(&state(&session), near_id)["location"], "C3");
    assert_eq!(unit(&state(&session), &far_id)["location"], "C1");
    assert_eq!(unit(&state(&session), &far_id)["controller"], "north");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2568_second_blink_teleports_a_newly_summoned_adjacent_ally() {
    let encoded = seed_for_second_blink_new_summon(2568);
    let (mut session, ally_id) =
        try_second_blink_new_summon_prefix(&encoded).expect("second nearby-ally new-summon prefix");
    let receipt = cast_blink_on(&mut session, &ally_id, "C3", "spellbook");
    assert!(event_types(&receipt).contains(&"unit-teleported"));
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C3");
    assert_eq!(unit(&state(&session), &ally_id)["controller"], "north");
    assert_exact_replay(&session);
}
