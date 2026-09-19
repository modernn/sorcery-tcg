fn combat_gift() -> Value {
    json!({
        "cardType": "magic",
        "grantAirborneToAllyThisTurn": true,
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

fn fragile_ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn roaming_ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn south_enemy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn library_manifest(
    seed: u32,
    north_ally: &Value,
    north_spellbook: &[&str],
    south_spellbook: &[&str],
    south_card: (&str, Value),
    fixture: &str,
    revision: &str,
) -> String {
    let (south_id, south_value) = south_card;
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": revision,
        },
        "cards": {
            "north-ally": north_ally.clone(),
            "north-avatar": avatar(),
            "north-gift": combat_gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            south_id: south_value,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn admission_supplemental_manifest(seed: u32) -> String {
    library_manifest(
        seed,
        &ally(),
        &["north-ally", "north-ally", "north-gift", "north-gift"],
        &["south-enemy"; 6],
        ("south-enemy", south_enemy()),
        "grant-admission-supplemental",
        "synthetic-grant-admission-supplemental-v1",
    )
}

fn roaming_admission_manifest(seed: u32) -> String {
    library_manifest(
        seed,
        &roaming_ally(),
        &["north-ally", "north-ally", "north-gift", "north-gift"],
        &["south-enemy"; 6],
        ("south-enemy", south_enemy()),
        "grant-admission-roaming-supplemental",
        "synthetic-grant-admission-roaming-supplemental-v1",
    )
}

fn empty_repeat_manifest(seed: u32) -> String {
    library_manifest(
        seed,
        &fragile_ally(),
        &["north-ally", "north-gift", "north-gift", "north-gift"],
        &["south-rain"; 6],
        ("south-rain", rain_spell()),
        "grant-admission-empty-repeat",
        "synthetic-grant-admission-empty-repeat-v1",
    )
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

fn can_summon_north_ally_at_c4(session: &Session) -> bool {
    offers(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })
}

fn seed_reaching_north_summon(start: u32, manifest: fn(u32) -> String) -> String {
    (start..start + 256)
        .chain(734..734 + 256)
        .find(|seed| {
            let encoded = manifest(*seed);
            let session = opening_main(&encoded);
            can_summon_north_ally_at_c4(&session)
        })
        .map(manifest)
        .expect("bounded seed reaching North ally summon at C4 after opening")
}

fn grant_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-gift")
                .count()
        })
        .unwrap_or_default()
}

fn temporary_airborne_len(snapshot: &Value, instance_id: &str) -> usize {
    unit(snapshot, instance_id)
        .get("temporaryAirborneSources")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    });
}

fn summon_north_ally_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned()
}

fn setup_north_c4_and_south_c1(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let enemy_id = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })
    .0["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let ally_id = summon_north_ally_at(session, "C4");
    (ally_id, enemy_id)
}

fn setup_two_north_allies_at_c4(session: &mut Session) -> (String, String) {
    let first = summon_north_ally_at(session, "C4");
    let second = summon_north_ally_at(session, "C4");
    (first, second)
}

fn opening_ally_count(encoded: &str) -> usize {
    opening_spell_ids(encoded)
        .iter()
        .filter(|card| *card == "north-ally")
        .count()
}

fn try_south_plays_c1_then_casts_rain(session: &mut Session) -> Option<Receipt> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    Some(
        try_accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "south-rain"
        })?
        .1,
    )
}

fn south_plays_c1_then_casts_rain(session: &mut Session) -> Receipt {
    try_south_plays_c1_then_casts_rain(session).expect("South Rain after site placement at C1")
}

fn cast_combat_grant_on(session: &mut Session, ally_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    receipt
}

fn grant_minion_ally_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("grant actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-gift"
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

fn try_second_grant_ally_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_north_c4_and_south_c1(&mut session).0;
    if !grant_minion_ally_targets(&session).contains(&first_id) {
        return None;
    }
    let first = cast_combat_grant_on(&mut session, &first_id);
    if !event_types(&first).contains(&"airborne-granted") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if grant_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let ally_id = summon_north_ally_at(&mut session, "C3");
    pass_turn_to_north_spellbook(&mut session);
    grant_minion_ally_targets(&session)
        .contains(&ally_id)
        .then_some((session, ally_id))
}

fn seed_for_second_grant_ally_arrival(start: u32) -> String {
    (start..start + 2048)
        .chain(734..734 + 2048)
        .find_map(|seed| {
            let encoded = roaming_admission_manifest(seed);
            if opening_spell_ids(&encoded)
                .iter()
                .filter(|card| *card == "north-ally")
                .count()
                < 2
            {
                return None;
            }
            try_second_grant_ally_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second grant-Airborne-to-ally arrival setup")
}

fn try_second_grant_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_north_c4_and_south_c1(&mut session).0;
    if !grant_minion_ally_targets(&session).contains(&first_id) {
        return None;
    }
    let first = cast_combat_grant_on(&mut session, &first_id);
    if !event_types(&first).contains(&"airborne-granted") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if grant_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let ally_id = summon_north_ally_at(&mut session, "C4");
    pass_turn_to_north_spellbook(&mut session);
    grant_minion_ally_targets(&session)
        .contains(&ally_id)
        .then_some((session, ally_id))
}

fn seed_for_second_grant_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(734..734 + 8192)
        .find_map(|seed| {
            let encoded = admission_supplemental_manifest(seed);
            if opening_spell_ids(&encoded)
                .iter()
                .filter(|card| *card == "north-ally")
                .count()
                < 2
            {
                return None;
            }
            try_second_grant_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second grant-Airborne-to-ally new-summon setup")
}

fn persistence_manifest(seed: u32) -> String {
    library_manifest(
        seed,
        &ally(),
        &["north-ally", "north-gift", "north-gift"],
        &["south-enemy"; 6],
        ("south-enemy", south_enemy()),
        "grant-admission-persistence",
        "synthetic-grant-admission-persistence-v1",
    )
}

#[test]
fn rule_catalog_2573_granted_ally_stays_at_c4_after_turns_pass() {
    let encoded = seed_reaching_north_summon(2573, persistence_manifest);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert!(event_types(&receipt).contains(&"airborne-granted"));
    assert_eq!(temporary_airborne_len(&state(&session), &ally_id), 1);
    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "airborne-expired" && event.payload["instanceId"] == ally_id
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C4");
    assert_eq!(temporary_airborne_len(&state(&session), &ally_id), 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2574_second_grant_stays_unoffered_after_south_rain_kills_the_only_ally() {
    let encoded = (2574..2574 + 2048)
        .chain(734..734 + 2048)
        .map(empty_repeat_manifest)
        .find(|candidate| {
            let mut session = opening_main(candidate);
            if !can_summon_north_ally_at_c4(&session) {
                return false;
            }
            let ally_id = summon_north_ally_at(&mut session, "C4");
            try_south_plays_c1_then_casts_rain(&mut session).is_some_and(|rain| {
                event_types(&rain).contains(&"minion-died")
                    && state(&session)["realm"]["units"]
                        .as_array()
                        .is_some_and(|units| units.iter().all(|unit| unit["instanceId"] != ally_id))
            })
        })
        .expect("bounded seed with South Rain killing the only North ally");
    let mut session = opening_main(&encoded);
    let ally_id = summon_north_ally_at(&mut session, "C4");
    let rain = south_plays_c1_then_casts_rain(&mut session);
    assert!(event_types(&rain).contains(&"minion-died"));
    assert!(state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .all(|unit| unit["instanceId"] != ally_id));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(grant_spells_in_hand(&state(&session)) >= 1);
    assert!(grant_minion_ally_targets(&session).is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2575_second_grant_marks_a_newly_arrived_ally_after_north_site_placement() {
    let encoded = seed_for_second_grant_ally_arrival(2575);
    let (mut session, ally_id) = try_second_grant_ally_arrival_prefix(&encoded)
        .expect("second grant-Airborne-to-ally arrival prefix");
    let receipt = cast_combat_grant_on(&mut session, &ally_id);
    assert!(event_types(&receipt).contains(&"airborne-granted"));
    assert_eq!(temporary_airborne_len(&state(&session), &ally_id), 1);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2576_grant_airborne_to_ally_offers_every_allied_minion_at_c4() {
    let encoded = (2576..2576 + 2048)
        .chain(734..734 + 2048)
        .map(admission_supplemental_manifest)
        .find(|candidate| {
            opening_ally_count(candidate) >= 2
                && grant_spells_in_hand(&state(&opening_main(candidate))) >= 1
        })
        .expect("bounded seed with two grant targets at C4");
    let mut session = opening_main(&encoded);
    let (first_id, second_id) = setup_two_north_allies_at_c4(&mut session);
    let offered = grant_minion_ally_targets(&session);
    assert!(offered.contains(&first_id));
    assert!(offered.contains(&second_id));
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2577_grant_airborne_to_ally_leaves_a_far_enemy_untouched() {
    let encoded = (2577..2577 + 256)
        .chain(734..734 + 256)
        .map(admission_supplemental_manifest)
        .find(|candidate| {
            let mut session = opening_main(candidate);
            if !can_summon_north_ally_at_c4(&session) {
                return false;
            }
            let (ally_id, enemy_id) = setup_north_c4_and_south_c1(&mut session);
            let offered = grant_minion_ally_targets(&session);
            offered.contains(&ally_id) && !offered.contains(&enemy_id)
        })
        .expect("bounded seed with ally-only grant targets");
    let mut session = opening_main(&encoded);
    let (ally_id, enemy_id) = setup_north_c4_and_south_c1(&mut session);
    let offered = grant_minion_ally_targets(&session);
    assert!(offered.contains(&ally_id));
    assert!(!offered.contains(&enemy_id));
    let receipt = cast_combat_grant_on(&mut session, &ally_id);
    assert!(event_types(&receipt).contains(&"airborne-granted"));
    assert_eq!(temporary_airborne_len(&state(&session), &ally_id), 1);
    assert_eq!(temporary_airborne_len(&state(&session), &enemy_id), 0);
    assert_eq!(unit(&state(&session), &enemy_id)["location"], "C1");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2578_second_grant_marks_a_newly_summoned_ally() {
    let encoded = seed_for_second_grant_new_summon(2578);
    let (mut session, ally_id) = try_second_grant_new_summon_prefix(&encoded)
        .expect("second grant-Airborne-to-ally new-summon prefix");
    let receipt = cast_combat_grant_on(&mut session, &ally_id);
    assert!(event_types(&receipt).contains(&"airborne-granted"));
    assert_eq!(temporary_airborne_len(&state(&session), &ally_id), 1);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C4");
    assert_exact_replay(&session);
}
