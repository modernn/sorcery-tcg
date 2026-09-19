//! Direct proofs for Craterize discard-cost site destruction and damage grid
//! (RULE-CATALOG-0160, 0705–0706, 1075, 2253–2258).
//!
//! 0657–0658 cover the Session replay slice. 0705 is the unprotected
//! discard/destroy/grid happy path; 0706 is discard-cost refusal plus
//! protected-site damage without destroying the site. While Deathrites wait
//! for ordering, Craterize Magic stays withheld until the chain drains.
//! 2253–2258 are the unprotected 0657 supplemental persistence, no-real-site
//! repeat, enemy-arrival, multi-site, far-site, and new-placement proofs.

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

fn craterize_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageUnitsAboveAndBelowTargetSiteByManhattanDistance": [1, 1, 1, 1, 1],
        "destroyTargetSite": true,
        "discardSiteAsAdditionalCost": true,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
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

fn craterize_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Craterize actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-craterize"
                && action.descriptor["discardSiteInstanceId"].is_string()
        })
        .filter_map(|action| {
            Some((
                action.descriptor["targetLocation"]["cell"]
                    .as_str()?
                    .to_owned(),
                action.descriptor["targetSiteInstanceId"]
                    .as_str()?
                    .to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn deathrite_craterize_manifest(seed: u32) -> String {
    let fixture = "craterize-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-craterize": craterize_spell(),
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
                    "north-craterize",
                    "north-rain",
                    "north-rain",
                    "north-craterize",
                    "north-rain",
                    "north-craterize",
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

fn north_has_craterize_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-craterize", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteCraterizeSetup {
    deathrite_ids: [String; 2],
    session: Session,
    south_site_id: String,
}

fn try_pending_deathrite_with_craterize_target(
    encoded: &str,
) -> Option<PendingDeathriteCraterizeSetup> {
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
    let south_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()?
        .to_owned();
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
    if !north_has_craterize_and_rain(&state(&session)) {
        return None;
    }
    if craterize_targets(&session).is_empty() {
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
    Some(PendingDeathriteCraterizeSetup {
        deathrite_ids,
        session,
        south_site_id,
    })
}

fn deathrite_craterize_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_craterize_manifest)
        .find(|candidate| try_pending_deathrite_with_craterize_target(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Craterize Magic in hand")
}

#[test]
fn rule_catalog_0705_craterize_discards_destroy_target_and_applies_damage_grid() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0705_craterize_discards_destroy_target_and_applies_damage_grid();
}

#[test]
fn rule_catalog_0706_craterize_enforces_discard_cost_and_still_damages_protected_sites() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0706_craterize_enforces_discard_cost_and_still_damages_protected_sites();
}

#[test]
fn rule_catalog_1075_craterize_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_craterize_seed_with(1075);
    let mut setup = try_pending_deathrite_with_craterize_target(&encoded)
        .expect("complete Craterize Deathrite withheld setup");
    let south_site_id = setup.south_site_id.clone();
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
    assert_eq!(paused["realm"]["sites"]["C1"]["instanceId"], south_site_id);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(craterize_targets(session).is_empty());

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
    assert_eq!(resumed["realm"]["sites"]["C1"]["instanceId"], south_site_id);
    assert!(
        craterize_targets(session)
            .iter()
            .any(|(cell, id)| cell == "C1" && *id == south_site_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-craterize"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
            && descriptor["discardSiteInstanceId"].is_string()
    });
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-destroyed",
            "magic-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
            "rubble-created",
            "magic-resolved"
        ]
    );
    assert_eq!(state(session)["realm"]["sites"]["C1"]["rubble"], true);
    assert_exact_replay(session);
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid Craterize session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
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

fn craterize_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "craterize-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-craterize-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-craterize": craterize_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-craterize"; 8],
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

fn supplemental_seed_with_start(start: u32) -> String {
    (start..start + 2048)
        .chain(657..657 + 2048)
        .map(craterize_supplemental_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-craterize")
        })
        .expect("bounded seed with Craterize Magic in the opening hand")
}

fn craterize_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-craterize")
                .count()
        })
        .unwrap_or_default()
}

fn site_instance_at(snapshot: &Value, cell: &str) -> String {
    snapshot["realm"]["sites"][cell]["instanceId"]
        .as_str()
        .expect("site at cell")
        .to_owned()
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
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn play_south_site_at(session: &mut Session, cell: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == cell
    });
    site_instance_at(&state(session), cell)
}

fn setup_south_sites_at(session: &mut Session, cells: &[&str]) -> Vec<(String, String)> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let mut placed = vec![(cells[0].to_string(), play_south_site_at(session, cells[0]))];
    for cell in cells.iter().skip(1) {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw"
                && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
        });
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw"
                && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
        });
        placed.push(((*cell).to_string(), play_south_site_at(session, cell)));
    }
    placed
}

fn cast_craterize_on(session: &mut Session, cell: &str, site_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-craterize"
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetSiteInstanceId"] == site_id
            && descriptor["discardSiteInstanceId"].is_string()
    });
    receipt
}

fn try_cast_craterize_on(session: &mut Session, cell: &str, site_id: &str) -> Option<Receipt> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-craterize"
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetSiteInstanceId"] == site_id
            && descriptor["discardSiteInstanceId"].is_string()
    })
    .map(|(_, receipt)| receipt)
}

fn craterize_real_site_targets(session: &Session) -> Vec<(String, String)> {
    let snapshot = state(session);
    craterize_targets(session)
        .into_iter()
        .filter(|(cell, id)| {
            snapshot["realm"]["sites"][cell]["instanceId"] == *id
                && snapshot["realm"]["sites"][cell]["rubble"] != json!(true)
        })
        .collect()
}

fn try_second_craterize_enemy_site_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    let (c1, south_c1) = setup_south_sites_at(&mut session, &["C1", "C2"])[0].clone();
    north_draws_spellbook(&mut session);
    let first = try_cast_craterize_on(&mut session, &c1, &south_c1)?;
    if !event_types(&first).contains(&"site-destroyed") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if craterize_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    let south_c3 = play_south_site_at(&mut session, "C3");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    craterize_targets(&session)
        .iter()
        .any(|(cell, id)| cell == "C3" && *id == south_c3)
        .then_some((session, "C3".to_owned(), south_c3))
}

fn seed_for_second_craterize_enemy_site(start: u32) -> String {
    (start..start + 8192)
        .chain(657..657 + 8192)
        .find_map(|seed| {
            let encoded = craterize_supplemental_manifest(seed);
            try_second_craterize_enemy_site_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Craterize enemy-arrival setup")
}

fn try_second_craterize_new_site_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    let (c1, south_c1) = setup_south_sites_at(&mut session, &["C1"])[0].clone();
    north_draws_spellbook(&mut session);
    let first = try_cast_craterize_on(&mut session, &c1, &south_c1)?;
    if !event_types(&first).contains(&"site-destroyed") {
        return None;
    }
    if state(&session)["realm"]["sites"]["C1"]["rubble"] != json!(true) {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if craterize_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let south_c2 = play_south_site_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    craterize_targets(&session)
        .iter()
        .any(|(cell, id)| cell == "C2" && *id == south_c2)
        .then_some((session, "C2".to_owned(), south_c2))
}

fn seed_for_second_craterize_new_site(start: u32) -> String {
    (start..start + 8192)
        .chain(657..657 + 8192)
        .find_map(|seed| {
            let encoded = craterize_supplemental_manifest(seed);
            try_second_craterize_new_site_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Craterize new-placement setup")
}

#[test]
fn rule_catalog_2253_site_stays_at_the_location_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2253);
    let mut session = opening_main(&encoded);
    let site_id = setup_south_sites_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["instanceId"],
        site_id
    );
    assert_ne!(
        state(&session)["realm"]["sites"]["C1"]["rubble"],
        json!(true)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2254_second_craterize_offers_no_real_site_targets_after_the_only_real_site_becomes_rubble()
 {
    let encoded = (2254..2254 + 8192)
        .chain(657..657 + 8192)
        .find_map(|seed| {
            let candidate = craterize_supplemental_manifest(seed);
            if !opening_spell_ids(&candidate)
                .iter()
                .any(|card| card == "north-craterize")
            {
                return None;
            }
            let mut session = opening_main(&candidate);
            let north_site = site_instance_at(&state(&session), "C4");
            let first = try_cast_craterize_on(&mut session, "C4", &north_site)?;
            if !event_types(&first).contains(&"site-destroyed") {
                return None;
            }
            if state(&session)["realm"]["sites"]["C4"]["rubble"] != json!(true) {
                return None;
            }
            if craterize_spells_in_hand(&state(&session)) < 1 {
                return None;
            }
            craterize_real_site_targets(&session)
                .is_empty()
                .then_some(candidate)
        })
        .expect("bounded seed with two Craterize casts after clearing real sites");
    let mut session = opening_main(&encoded);
    let north_site = site_instance_at(&state(&session), "C4");
    let first = cast_craterize_on(&mut session, "C4", &north_site);
    assert!(event_types(&first).contains(&"site-destroyed"));
    assert!(event_types(&first).contains(&"card-discarded"));
    assert_eq!(state(&session)["realm"]["sites"]["C4"]["rubble"], true);
    assert!(craterize_spells_in_hand(&state(&session)) >= 1);
    assert!(craterize_real_site_targets(&session).is_empty());
    assert_eq!(
        craterize_targets(&session),
        [("C4".to_owned(), site_instance_at(&state(&session), "C4"))]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2255_second_craterize_replaces_a_newly_arrived_site_after_enemy_site_placement() {
    let encoded = seed_for_second_craterize_enemy_site(2255);
    let (mut session, cell, site_id) =
        try_second_craterize_enemy_site_prefix(&encoded).expect("second Craterize prefix");
    let receipt = cast_craterize_on(&mut session, &cell, &site_id);
    assert!(event_types(&receipt).contains(&"site-destroyed"));
    assert!(event_types(&receipt).contains(&"card-discarded"));
    assert_eq!(state(&session)["realm"]["sites"][&cell]["rubble"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2256_craterize_offers_every_real_site_in_the_realm() {
    let encoded = supplemental_seed_with_start(2256);
    let mut session = opening_main(&encoded);
    let south_sites = setup_south_sites_at(&mut session, &["C1", "C2"]);
    north_draws_spellbook(&mut session);
    let north_site = site_instance_at(&state(&session), "C4");
    let offered = craterize_targets(&session);
    for (cell, site_id) in &south_sites {
        assert!(offered.contains(&(cell.clone(), site_id.clone())));
    }
    assert!(offered.contains(&("C4".to_owned(), north_site)));
    assert_eq!(offered.len(), 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2257_craterize_leaves_a_far_site_untouched() {
    let encoded = supplemental_seed_with_start(2257);
    let mut session = opening_main(&encoded);
    let far_id = setup_south_sites_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    let near_id = site_instance_at(&state(&session), "C3");
    let receipt = cast_craterize_on(&mut session, "C3", &near_id);
    assert!(event_types(&receipt).contains(&"site-destroyed"));
    assert_eq!(state(&session)["realm"]["sites"]["C3"]["rubble"], true);
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["instanceId"],
        far_id
    );
    assert_ne!(
        state(&session)["realm"]["sites"]["C1"]["rubble"],
        json!(true)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2258_second_craterize_replaces_a_newly_placed_site() {
    let encoded = seed_for_second_craterize_new_site(2258);
    let (mut session, cell, site_id) = try_second_craterize_new_site_prefix(&encoded)
        .expect("second Craterize new-placement prefix");
    let receipt = cast_craterize_on(&mut session, &cell, &site_id);
    assert!(event_types(&receipt).contains(&"site-destroyed"));
    assert!(event_types(&receipt).contains(&"card-discarded"));
    assert_eq!(state(&session)["realm"]["sites"][&cell]["rubble"], true);
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == site_id))
    );
    assert_exact_replay(&session);
}
