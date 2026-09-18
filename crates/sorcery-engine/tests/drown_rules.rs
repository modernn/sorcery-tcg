//! Direct proofs for submerge-target-minion Magic (RULE-CATALOG-0591–0592,
//! RULE-CATALOG-1034, RULE-CATALOG-1089, RULE-CATALOG-1933–1938).
//!
//! Ordinary Drown Magic forcefully submerges a same-region minion at a Water
//! site. A Submerge minion survives underwater. An earth-only site resolves as a
//! paid no-op because no underwater layer exists there. While Deathrites wait
//! for ordering, Drown Magic stays withheld until the chain drains.

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

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth", "water"],
    })
}

fn swimmer() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drown() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "submergeTargetMinion": true,
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

fn drown_manifest(seed: u32, water: bool) -> String {
    let site = if water { water_site() } else { earth_site() };
    let fixture = if water { "drown-water" } else { "drown-earth" };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drown": drown(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": swimmer(),
            "south-site": site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-drown"; 6],
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
    let mut session = Session::new(encoded).expect("valid drown session");
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

fn seed_with(water: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| drown_manifest(seed, water))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                state(&preview)["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-drown"))
            })
        })
        .expect("bounded seed with Drown in the opening hand")
}

fn south_plays_c1_and_summons(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Drown target identity")
        .to_owned()
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn drown_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("drown actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-drown"
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

fn deathrite_drown_manifest(seed: u32) -> String {
    let fixture = "drown-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drown": drown(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": water_site(),
            "south-swimmer": swimmer(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drown",
                    "north-rain",
                    "north-rain",
                    "north-drown",
                    "north-rain",
                    "north-drown",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-swimmer", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_drown_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-drown", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteDrownSetup {
    deathrite_ids: [String; 2],
    session: Session,
    swimmer_id: String,
}

fn try_pending_deathrite_with_ready_swimmer(encoded: &str) -> Option<PendingDeathriteDrownSetup> {
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
    let swimmer = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-swimmer"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let swimmer_id = swimmer.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_drown_and_rain(&state(&session)) {
        return None;
    }
    if !drown_targets(&session).contains(&swimmer_id) {
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
    Some(PendingDeathriteDrownSetup {
        deathrite_ids,
        session,
        swimmer_id,
    })
}

fn deathrite_drown_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_drown_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_swimmer(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Drown Magic in hand")
}

fn setup_drown_target(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let target_id = south_plays_c1_and_summons(&mut session);
    (session, target_id)
}

#[test]
fn rule_catalog_0591_drown_submerges_a_submerge_minion_at_a_water_site() {
    let encoded = seed_with(true, 591);
    let (mut session, target_id) = setup_drown_target(&encoded);
    assert_eq!(unit(&state(&session), &target_id)["region"], "surface");

    let (cast, submerged) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-drown"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&submerged),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(
        submerged.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": target_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    assert_eq!(unit(&state(&session), &target_id)["region"], "underwater");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0592_drown_on_earth_only_site_is_a_paid_noop() {
    let encoded = seed_with(false, 592);
    let (mut session, target_id) = setup_drown_target(&encoded);
    let before = unit(&state(&session), &target_id).clone();

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-drown"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert_eq!(unit(&state(&session), &target_id), &before);
    assert_exact_replay(&session);
}

fn prove_drown_magic_withheld_during_pending_deathrite_order(start: u32) {
    let encoded = deathrite_drown_seed_with(start);
    let mut setup = try_pending_deathrite_with_ready_swimmer(&encoded)
        .expect("complete drown Deathrite withheld setup");
    let swimmer_id = setup.swimmer_id.clone();
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
    assert_eq!(unit(&paused, &swimmer_id)["region"], "surface");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(drown_targets(session).is_empty());

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
    assert_eq!(unit(&resumed, &swimmer_id)["region"], "surface");
    assert_eq!(drown_targets(session), [swimmer_id.as_str()]);

    let (cast, submerged) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-drown"
            && descriptor["target"]["instanceId"] == swimmer_id
    });
    assert_eq!(
        event_types(&submerged),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(
        submerged.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": swimmer_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    assert_eq!(unit(&state(session), &swimmer_id)["region"], "underwater");
    assert!(realm_unit(&state(session), &deathrite_ids[0]).is_none());
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1034_drown_magic_withheld_during_pending_deathrite_order() {
    prove_drown_magic_withheld_during_pending_deathrite_order(1034);
}

#[test]
fn rule_catalog_1089_drown_magic_withheld_during_pending_deathrite_order() {
    prove_drown_magic_withheld_during_pending_deathrite_order(1089);
}

fn drown_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drown-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drown-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drown": drown(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": swimmer(),
            "south-site": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-drown"; 8],
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

fn seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(591..591 + 2048)
        .map(drown_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-drown")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Drown and required South minions")
}

fn drown_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-drown")
                .count()
        })
        .unwrap_or_default()
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

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Drown target identity")
        .to_owned()
}

fn setup_c1_with_swimmers(session: &mut Session, count: usize) -> Vec<String> {
    south_plays_c1(session);
    (0..count).map(|_| summon_south_at(session, "C1")).collect()
}

fn cast_drown_target(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-drown"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = opening_main(encoded);
    let c1_ids = setup_c1_with_swimmers(&mut session, 2);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let far_id = summon_south_at(&mut session, "C4");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (!drown_targets(&session).is_empty()).then_some((session, c1_ids, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(591..591 + 2048)
        .find_map(|seed| {
            let encoded = drown_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching Drown far-minion setup")
}

fn try_second_drown_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_swimmers(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_drown_target(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if drown_spells_in_hand(&state(&session)) < 1 {
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
    let minion_id = summon_south_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    drown_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_drown_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(591..591 + 8192)
        .find_map(|seed| {
            let encoded = drown_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_drown_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Drown enemy-arrival setup")
}

fn try_second_drown_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_swimmers(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_drown_target(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if drown_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C1");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    drown_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_drown_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(591..591 + 8192)
        .find_map(|seed| {
            let encoded = drown_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_drown_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Drown new-summon setup")
}

#[test]
fn rule_catalog_1933_submerged_minion_stays_underwater_after_turns_pass() {
    let encoded = seed_with_start(1933, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_swimmers(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_drown_target(&mut session, &target_id);
    assert_eq!(unit(&state(&session), &target_id)["region"], "underwater");
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &target_id)["region"], "underwater");
    assert!(realm_unit(&state(&session), &target_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1934_second_drown_without_a_surface_target_stays_unoffered() {
    let encoded = seed_with_start(1934, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_swimmers(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_drown_target(&mut session, &target_id);
    assert_eq!(unit(&state(&session), &target_id)["region"], "underwater");
    assert!(drown_spells_in_hand(&state(&session)) >= 1);
    assert!(drown_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-drown"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1935_second_drown_submerges_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_drown_enemy_arrival(1935);
    let (mut session, minion_id) =
        try_second_drown_enemy_arrival_prefix(&encoded).expect("second Drown enemy-arrival prefix");
    let receipt = cast_drown_target(&mut session, &minion_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(unit(&state(&session), &minion_id)["region"], "underwater");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1936_drown_offers_every_surface_minion_at_the_target_water_site() {
    let encoded = seed_with_start(1936, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c1_with_swimmers(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = drown_targets(&session);
    assert_eq!(offered.len(), 2);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1937_drown_leaves_a_far_minion_on_the_surface() {
    let encoded = seed_for_far_minion(1937);
    let (mut session, c1_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("Drown far-minion prefix");
    let drowned_id = &c1_ids[0];
    cast_drown_target(&mut session, drowned_id);
    assert_eq!(unit(&state(&session), drowned_id)["region"], "underwater");
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("far minion")["location"],
        "C4"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1938_second_drown_submerges_a_newly_summoned_minion() {
    let encoded = seed_for_second_drown_new_summon(1938);
    let (mut session, minion_id) =
        try_second_drown_new_summon_prefix(&encoded).expect("second Drown new-summon prefix");
    let receipt = cast_drown_target(&mut session, &minion_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(unit(&state(&session), &minion_id)["region"], "underwater");
    assert_exact_replay(&session);
}
