//! Direct proofs for kill-target-wounded-minion Magic region filtering
//! (RULE-CATALOG-0147, RULE-CATALOG-0673–0674, RULE-CATALOG-2333–2338).
//!
//! Fatality offers only wounded minions in the caster region. 0609–0610 already
//! prove the wounded-versus-healthy surface slice in `fatality_rules.rs`; these
//! proofs wound a minion and then burrow it so the region filter is the thing
//! under test. Supplemental 2333–2338 bind persistence, empty-repeat,
//! enemy-arrival, multi-wounded, underground, and a newly summoned surface
//! minion. Seed search starts at the catalog id and falls back to 673.

use serde_json::{json, Value};
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

fn burrower() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn fatality() -> Value {
    json!({
        "cardType": "magic",
        "killTargetWoundedMinion": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lash() -> Value {
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

fn fatality_region_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "fatality-region-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-fatality-region-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-fatality": fatality(),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": burrower(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-lash",
                    "north-lash",
                    "north-bury",
                    "north-fatality",
                    "north-lash",
                    "north-fatality",
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
    let mut session = Session::new(encoded).expect("valid fatality-region session");
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    realm_unit(snapshot, instance_id).expect("expected realm unit")
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

fn fatality_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("fatality actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fatality"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn opening_card_ids(encoded: &str) -> (Vec<String>, Vec<String>) {
    let preview = Session::new(encoded).expect("Fatality region seed candidate");
    let snapshot = state(&preview);
    let ids = |cards: &Value| {
        cards
            .as_array()
            .expect("spell cards")
            .iter()
            .map(|card| card["cardId"].as_str().expect("card identity").to_owned())
            .collect()
    };
    (
        ids(&snapshot["players"]["north"]["hand"]["spellbook"]),
        ids(&snapshot["players"]["north"]["spellbook"]),
    )
}

fn opening_south_minions(encoded: &str) -> usize {
    let preview = Session::new(encoded).expect("Fatality region seed candidate");
    state(&preview)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "south-minion")
                .count()
        })
        .unwrap_or_default()
}

fn seed_has_region_spells(encoded: &str, need_second_lash: bool) -> bool {
    let (hand, library) = opening_card_ids(encoded);
    ["north-fatality", "north-lash", "north-bury"]
        .into_iter()
        .all(|card_id| hand.iter().any(|id| id == card_id))
        && (!need_second_lash || library.first().map(String::as_str) == Some("north-lash"))
}

fn seed_with_start(start: u32, need_second_lash: bool) -> String {
    (start..start + 2048)
        .chain(673..673 + 2048)
        .map(fatality_region_manifest)
        .find(|candidate| seed_has_region_spells(candidate, need_second_lash))
        .expect("bounded seed with Fatality region spells in hand")
}

fn seed_with_region_spells(need_second_lash: bool) -> String {
    seed_with_start(673, need_second_lash)
}

fn fatality_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-fatality")
                .count()
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

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    end_turn_if_offered(session);
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
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

fn try_summon_south_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    summoned["cardInstanceId"].as_str().map(str::to_owned)
}

fn try_lash_minion(session: &mut Session, instance_id: &str) -> Option<()> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == instance_id
    })
    .map(|_| ())
}

fn cast_fatality_on(session: &mut Session, instance_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fatality"
            && descriptor["target"]["instanceId"] == instance_id
    });
    receipt
}

fn try_second_fatality_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String, String)> {
    if !seed_has_region_spells(encoded, true) || opening_south_minions(encoded) < 2 {
        return None;
    }
    let mut session = opening_main(encoded);
    let enemy_ids = stage_enemies(&mut session, 1);
    let buried_id = enemy_ids[0].clone();
    lash_minion(&mut session, &buried_id);
    bury_minion(&mut session, &buried_id);
    if unit(&state(&session), &buried_id)["region"] != "underground" {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if fatality_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let visitor_id = try_summon_south_at(&mut session, "C2")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_lash_minion(&mut session, &visitor_id)?;
    fatality_targets(&session)
        .contains(&visitor_id)
        .then_some((session, visitor_id, buried_id))
}

fn seed_for_second_fatality_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(673..673 + 8192)
        .find_map(|seed| {
            let encoded = fatality_region_manifest(seed);
            try_second_fatality_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching Fatality region enemy-arrival setup")
}

fn try_second_fatality_new_summon_prefix(encoded: &str) -> Option<(Session, String, String)> {
    if !seed_has_region_spells(encoded, true) || opening_south_minions(encoded) < 2 {
        return None;
    }
    let mut session = opening_main(encoded);
    let enemy_ids = stage_enemies(&mut session, 1);
    let buried_id = enemy_ids[0].clone();
    lash_minion(&mut session, &buried_id);
    bury_minion(&mut session, &buried_id);
    if unit(&state(&session), &buried_id)["region"] != "underground" {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if fatality_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    let new_id = try_summon_south_at(&mut session, "C1")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_lash_minion(&mut session, &new_id)?;
    fatality_targets(&session)
        .contains(&new_id)
        .then_some((session, new_id, buried_id))
}

fn seed_for_second_fatality_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(673..673 + 8192)
        .find_map(|seed| {
            let encoded = fatality_region_manifest(seed);
            try_second_fatality_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching Fatality region new-summon setup")
}

fn stage_enemies(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut enemy_ids = Vec::new();
    for _ in 0..count {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cell"] == "C1"
                && descriptor["region"].is_null()
        });
        enemy_ids.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("summoned enemy identity")
                .to_owned(),
        );
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_ids
}

fn lash_minion(session: &mut Session, instance_id: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == instance_id
    });
}

fn bury_minion(session: &mut Session, instance_id: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == instance_id
    });
}

#[test]
fn rule_catalog_0673_fatality_kills_a_wounded_minion_in_the_caster_region_not_underground() {
    let encoded = seed_with_region_spells(true);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_enemies(&mut session, 2);
    let surface_id = enemy_ids[0].clone();
    let buried_id = enemy_ids[1].clone();

    assert!(fatality_targets(&session).is_empty());
    lash_minion(&mut session, &surface_id);
    lash_minion(&mut session, &buried_id);
    let mut wounded = fatality_targets(&session);
    wounded.sort_unstable();
    let mut expected = [surface_id.as_str(), buried_id.as_str()];
    expected.sort_unstable();
    assert_eq!(wounded, expected);

    bury_minion(&mut session, &buried_id);
    let after_bury = state(&session);
    let buried = unit(&after_bury, &buried_id);
    assert_eq!(buried["region"], "underground");
    assert_eq!(buried["damage"], 1);
    assert_eq!(fatality_targets(&session), [surface_id.as_str()]);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fatality"
    });
    assert_eq!(descriptor["target"]["instanceId"], surface_id.as_str());
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );

    let finished = state(&session);
    assert!(realm_unit(&finished, &surface_id).is_none());
    let survivor = unit(&finished, &buried_id);
    assert_eq!(survivor["region"], "underground");
    assert_eq!(survivor["damage"], 1);
    assert!(finished["players"]["south"]["cemetery"]
        .as_array()
        .expect("South cemetery")
        .iter()
        .any(|card| card["instanceId"] == surface_id.as_str()));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0674_fatality_offers_no_target_for_a_wounded_underground_minion() {
    let encoded = seed_with_region_spells(false);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_enemies(&mut session, 1);
    let buried_id = enemy_ids[0].clone();

    assert!(fatality_targets(&session).is_empty());
    lash_minion(&mut session, &buried_id);
    assert_eq!(fatality_targets(&session), [buried_id.as_str()]);

    bury_minion(&mut session, &buried_id);
    let after_bury = state(&session);
    let buried = unit(&after_bury, &buried_id);
    assert_eq!(buried["region"], "underground");
    assert_eq!(buried["damage"], 1);
    assert!(fatality_targets(&session).is_empty());
    let fatality_casts: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fatality"
        })
        .collect();
    assert_eq!(fatality_casts.len(), 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2333_wounded_underground_minion_stays_underground_after_turns_pass() {
    let encoded = seed_with_start(2333, false);
    let mut session = opening_main(&encoded);
    let buried_id = stage_enemies(&mut session, 1)[0].clone();
    lash_minion(&mut session, &buried_id);
    bury_minion(&mut session, &buried_id);
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_eq!(unit(&state(&session), &buried_id)["damage"], 1);
    assert!(fatality_targets(&session).is_empty());
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    });
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_eq!(unit(&state(&session), &buried_id)["damage"], 1);
    assert!(fatality_targets(&session).is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2334_second_fatality_offers_no_targets_after_only_underground_wounded_remains() {
    let encoded = (2334..2334 + 8192)
        .chain(673..673 + 8192)
        .find_map(|seed| {
            let candidate = fatality_region_manifest(seed);
            if !seed_has_region_spells(&candidate, true) || opening_south_minions(&candidate) < 2 {
                return None;
            }
            let mut session = opening_main(&candidate);
            let enemy_ids = stage_enemies(&mut session, 2);
            let surface_id = enemy_ids[0].clone();
            let buried_id = enemy_ids[1].clone();
            lash_minion(&mut session, &surface_id);
            lash_minion(&mut session, &buried_id);
            bury_minion(&mut session, &buried_id);
            let first = cast_fatality_on(&mut session, &surface_id);
            if !event_types(&first).contains(&"minion-died") {
                return None;
            }
            if realm_unit(&state(&session), &surface_id).is_some() {
                return None;
            }
            (fatality_spells_in_hand(&state(&session)) >= 1
                && fatality_targets(&session).is_empty())
            .then_some(candidate)
        })
        .expect("bounded seed with two Fatality casts after only underground wounded remains");
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_enemies(&mut session, 2);
    let surface_id = enemy_ids[0].clone();
    let buried_id = enemy_ids[1].clone();
    lash_minion(&mut session, &surface_id);
    lash_minion(&mut session, &buried_id);
    bury_minion(&mut session, &buried_id);
    let first = cast_fatality_on(&mut session, &surface_id);
    assert!(event_types(&first).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &surface_id).is_none());
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert!(fatality_spells_in_hand(&state(&session)) >= 1);
    assert!(fatality_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fatality"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2335_second_fatality_kills_a_newly_arrived_surface_minion_after_enemy_site_placement(
) {
    let encoded = seed_for_second_fatality_enemy_arrival(2335);
    let (mut session, visitor_id, buried_id) = try_second_fatality_enemy_arrival_prefix(&encoded)
        .expect("Fatality region enemy-arrival prefix");
    let receipt = cast_fatality_on(&mut session, &visitor_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &visitor_id).is_none());
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_eq!(unit(&state(&session), &buried_id)["damage"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2336_fatality_offers_every_surface_wounded_minion_not_underground() {
    let encoded = seed_with_start(2336, true);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_enemies(&mut session, 3);
    let first_id = enemy_ids[0].clone();
    let second_id = enemy_ids[1].clone();
    let buried_id = enemy_ids[2].clone();
    for minion_id in &enemy_ids {
        lash_minion(&mut session, minion_id);
    }
    bury_minion(&mut session, &buried_id);
    let offered = fatality_targets(&session);
    assert!(offered.contains(&first_id));
    assert!(offered.contains(&second_id));
    assert!(!offered.contains(&buried_id));
    assert_eq!(offered.len(), 2);
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2337_fatality_leaves_a_wounded_underground_minion_untouched() {
    let encoded = seed_with_start(2337, true);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_enemies(&mut session, 2);
    let surface_id = enemy_ids[0].clone();
    let buried_id = enemy_ids[1].clone();
    lash_minion(&mut session, &surface_id);
    lash_minion(&mut session, &buried_id);
    bury_minion(&mut session, &buried_id);
    let receipt = cast_fatality_on(&mut session, &surface_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &surface_id).is_none());
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_eq!(unit(&state(&session), &buried_id)["damage"], 1);
    assert_eq!(unit(&state(&session), &buried_id)["location"], "C1");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2338_second_fatality_kills_a_newly_summoned_surface_wounded_minion() {
    let encoded = seed_for_second_fatality_new_summon(2338);
    let (mut session, new_id, buried_id) =
        try_second_fatality_new_summon_prefix(&encoded).expect("Fatality region new-summon prefix");
    let receipt = cast_fatality_on(&mut session, &new_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &new_id).is_none());
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert!(state(&session)["players"]["south"]["cemetery"]
        .as_array()
        .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == new_id)));
    assert_exact_replay(&session);
}
