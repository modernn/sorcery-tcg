//! Direct proofs for nearby-control Magic and Deathrite (RULE-CATALOG-0659–0660,
//! RULE-CATALOG-0977, RULE-CATALOG-2263–2268).
//!
//! `gainControlOfTargetNearbyMinion` transfers a nearby minion to the caster.
//! Deathrite follows the new controller: targeted Magic is a non-unit source,
//! so killing the stolen minion draws a site for the thief while the corpse
//! still enters the owner's cemetery. A far minion is not a legal steal, and
//! the same Magic damage still resolves Deathrite for the original controller.
//! Distinct from the private Mesmerism fight path, `0977` proves the draw on a
//! later turn via Magic damage rather than immediate same-turn or combat death.
//! Supplemental `2263`–`2268` prove persistence, empty-repeat, enemy-arrival,
//! multi-nearby, far-minion, and new-summon slices of that same transfer.

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

fn deathrite() -> Value {
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

fn mesmerism() -> Value {
    json!({
        "cardType": "magic",
        "gainControlOfTargetNearbyMinion": true,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn deathrite_magic_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "deathrite-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-deathrite-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lash": lash(),
            "north-mesmerism": mesmerism(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-mesmerism",
                    "north-lash",
                    "north-mesmerism",
                    "north-lash",
                    "north-mesmerism",
                    "north-lash"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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
    let mut session = Session::new(encoded).expect("valid Deathrite Magic session");
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

fn north_has_both_spells(snapshot: &Value) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    ["north-mesmerism", "north-lash"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_magic_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_both_spells(&state(&preview)))
        })
        .expect("bounded seed with Mesmerism and Lash in the opening hand")
}

fn stage_south_minion(session: &mut Session, cell: &str) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned()
}

fn mesmerism_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Mesmerism actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-mesmerism"
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

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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
fn rule_catalog_0659_mesmerism_transfers_a_minion_and_its_deathrite_to_the_new_controller() {
    let encoded = seed_with(659);
    let mut session = opening_main(&encoded);
    let nearby_id = stage_south_minion(&mut session, "C4");
    assert_eq!(mesmerism_targets(&session), [nearby_id.as_str()]);

    let (_, stolen) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&stolen),
        ["magic-cast", "minion-control-changed", "magic-resolved"]
    );
    let changed = stolen
        .events
        .iter()
        .find(|event| event.event_type == "minion-control-changed")
        .expect("control change");
    assert_eq!(changed.payload["fromSeat"], "south");
    assert_eq!(changed.payload["seat"], "north");
    let stolen_state = state(&session);
    let transferred = realm_unit(&stolen_state, &nearby_id).expect("stolen minion");
    assert_eq!(transferred["controller"], "north");
    assert_eq!(transferred["owner"], "south");

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], nearby_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &nearby_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &nearby_id));
    assert!(!cemetery_has(&finished, "north", &nearby_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0660_mesmerism_does_not_steal_a_far_minion_and_deathrite_stays_with_controller() {
    let encoded = seed_with(660);
    let mut session = opening_main(&encoded);
    let far_id = stage_south_minion(&mut session, "C1");
    assert!(
        mesmerism_targets(&session).is_empty(),
        "a far minion must not be a nearby-control target"
    );

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == far_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], far_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &far_id).is_none());
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert!(cemetery_has(&finished, "south", &far_id));
    assert_exact_replay(&session);
}

fn end_then_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

#[test]
fn rule_catalog_0977_mesmerism_deathrite_draws_for_new_controller_on_delayed_kill_not_only_fight() {
    let encoded = seed_with(977);
    let mut session = opening_main(&encoded);
    let nearby_id = stage_south_minion(&mut session, "C4");
    assert_eq!(mesmerism_targets(&session), [nearby_id.as_str()]);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    let stolen_state = state(&session);
    let stolen = realm_unit(&stolen_state, &nearby_id).expect("stolen minion");
    assert_eq!(stolen["controller"], "north");
    assert_eq!(stolen["owner"], "south");

    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "spellbook");
    let delayed = state(&session);
    let still_stolen = realm_unit(&delayed, &nearby_id).expect("minion still controlled");
    assert_eq!(still_stolen["controller"], "north");
    assert_eq!(still_stolen["owner"], "south");

    let north_atlas = atlas_len(&delayed, "north");
    let south_atlas = atlas_len(&delayed, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], nearby_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");
    assert_eq!(drawn.payload["sourceInstanceId"], nearby_id);

    let finished = state(&session);
    assert!(realm_unit(&finished, &nearby_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &nearby_id));
    assert!(!cemetery_has(&finished, "north", &nearby_id));
    assert_exact_replay(&session);
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

fn control_deathrite_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "control-deathrite-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-control-deathrite-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-mesmerism": mesmerism(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-mesmerism"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

fn supplemental_seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(659..659 + 2048)
        .map(control_deathrite_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-mesmerism")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-deathrite")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Mesmerism and required South Deathrite minions")
}

fn mesmerism_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-mesmerism")
                .count()
        })
        .unwrap_or_default()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
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

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite supplemental identity")
        .to_owned()
}

fn setup_c4_with_south_minions(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let ids: Vec<String> = (0..count).map(|_| summon_south_at(session, "C4")).collect();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    ids
}

fn cast_mesmerism_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let nearby_id = summon_south_at(&mut session, "C4");
    let far_id = summon_south_at(&mut session, "C1");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let offered = mesmerism_targets(&session);
    (offered.contains(&nearby_id)
        && !offered.contains(&far_id)
        && mesmerism_spells_in_hand(&state(&session)) >= 1)
        .then_some((session, nearby_id, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(659..659 + 2048)
        .find_map(|seed| {
            let encoded = control_deathrite_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-deathrite")
                .count()
                < 2
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching nearby-control far-minion setup")
}

fn try_second_steal_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c4_with_south_minions(&mut session, 1)[0].clone();
    let first = cast_mesmerism_on(&mut session, &first_id);
    if !event_types(&first).contains(&"minion-control-changed") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if mesmerism_spells_in_hand(&state(&session)) < 1 {
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
    let minion_id = summon_south_at(&mut session, "C4");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    mesmerism_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_steal_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(659..659 + 8192)
        .find_map(|seed| {
            let encoded = control_deathrite_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-deathrite")
                .count()
                < 2
            {
                return None;
            }
            try_second_steal_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second nearby-control enemy-arrival setup")
}

fn try_second_steal_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c4_with_south_minions(&mut session, 1)[0].clone();
    let first = cast_mesmerism_on(&mut session, &first_id);
    if !event_types(&first).contains(&"minion-control-changed") {
        return None;
    }
    if unit(&state(&session), &first_id)["controller"] != "north" {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if mesmerism_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C4");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    mesmerism_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_steal_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(659..659 + 8192)
        .find_map(|seed| {
            let encoded = control_deathrite_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-deathrite")
                .count()
                < 2
            {
                return None;
            }
            try_second_steal_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second nearby-control new-summon setup")
}

#[test]
fn rule_catalog_2263_stolen_minion_stays_with_the_new_controller_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2263, 1);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c4_with_south_minions(&mut session, 1)[0].clone();
    let receipt = cast_mesmerism_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C4");
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2264_second_mesmerism_on_the_stolen_minion_is_a_paid_noop() {
    let encoded = (2264..2264 + 8192)
        .chain(659..659 + 8192)
        .find_map(|seed| {
            let candidate = control_deathrite_supplemental_manifest(seed);
            let mut session = opening_main(&candidate);
            let minion_id = setup_c4_with_south_minions(&mut session, 1)[0].clone();
            let first = cast_mesmerism_on(&mut session, &minion_id);
            if !event_types(&first).contains(&"minion-control-changed") {
                return None;
            }
            if unit(&state(&session), &minion_id)["controller"] != "north" {
                return None;
            }
            (mesmerism_spells_in_hand(&state(&session)) >= 1
                && mesmerism_targets(&session).contains(&minion_id))
            .then_some(candidate)
        })
        .expect("bounded seed with two Mesmerism casts after stealing the only nearby minion");
    let mut session = opening_main(&encoded);
    let minion_id = setup_c4_with_south_minions(&mut session, 1)[0].clone();
    let first = cast_mesmerism_on(&mut session, &minion_id);
    assert!(event_types(&first).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert!(mesmerism_spells_in_hand(&state(&session)) >= 1);
    assert_eq!(mesmerism_targets(&session).as_slice(), [minion_id.as_str()]);
    let second = cast_mesmerism_on(&mut session, &minion_id);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2265_second_mesmerism_steals_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_steal_enemy_arrival(2265);
    let (mut session, minion_id) = try_second_steal_enemy_arrival_prefix(&encoded)
        .expect("second nearby-control enemy-arrival prefix");
    let receipt = cast_mesmerism_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2266_mesmerism_offers_every_nearby_minion() {
    let encoded = supplemental_seed_with_start(2266, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c4_with_south_minions(&mut session, 2);
    let offered = mesmerism_targets(&session);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2267_mesmerism_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(2267);
    let (mut session, stolen_id, far_id) =
        try_far_minion_prefix(&encoded).expect("nearby-control far-minion prefix");
    let receipt = cast_mesmerism_on(&mut session, &stolen_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &stolen_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &far_id)["controller"], "south");
    assert_eq!(unit(&state(&session), &far_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &far_id)["location"], "C1");
    assert!(!mesmerism_targets(&session).contains(&far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2268_second_mesmerism_steals_a_newly_summoned_minion() {
    let encoded = seed_for_second_steal_new_summon(2268);
    let (mut session, minion_id) = try_second_steal_new_summon_prefix(&encoded)
        .expect("second nearby-control new-summon prefix");
    let receipt = cast_mesmerism_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_exact_replay(&session);
}
