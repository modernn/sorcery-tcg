//! Direct proofs for burrow-target-minion-or-artifact Magic settlement
//! (RULE-CATALOG-0655–0656, RULE-CATALOG-1036, RULE-CATALOG-1913–1918,
//! RULE-CATALOG-2243–2248).
//!
//! Bury forcefully burrows a chosen ordinary minion on Earth, then region
//! settlement kills it because it has no Burrowing. A Water site is still
//! offered, then resolves as a paid no-op because no Underground layer exists.
//! While Deathrites wait for ordering, bury Magic stays withheld until the
//! chain drains. These slices complement `bury_rules.rs` 0585–0586, which
//! prove a Burrowing survivor rather than immediate death.

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

fn ordinary_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
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

fn bury_manifest(seed: u32, water: bool) -> String {
    let south_site = if water { water_site() } else { earth_site() };
    let fixture = if water {
        "bury-magic-ordinary-water"
    } else {
        "bury-magic-ordinary-earth"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": ordinary_minion(),
            "south-site": south_site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bury"; 6],
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
    let mut session = Session::new(encoded).expect("valid bury magic session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn seed_with(water: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| bury_manifest(seed, water))
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-bury")
        })
        .expect("bounded seed with Bury in the opening hand")
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
        .expect("Bury target identity")
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

fn setup_ordinary_target(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let target_id = south_plays_c1_and_summons(&mut session);
    (session, target_id)
}

fn bury_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("bury actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-bury"
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

fn deathrite_bury_manifest(seed: u32) -> String {
    let fixture = "bury-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
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
                    "north-bury",
                    "north-rain",
                    "north-rain",
                    "north-bury",
                    "north-rain",
                    "north-bury",
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

fn north_has_bury_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-bury", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteBurySetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteBurySetup> {
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
    if !north_has_bury_and_rain(&state(&session)) {
        return None;
    }
    if bury_targets(&session).is_empty() {
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
    Some(PendingDeathriteBurySetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_bury_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_bury_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with bury Magic in hand")
}

#[test]
fn rule_catalog_0655_bury_burrows_then_kills_an_ordinary_minion() {
    let encoded = seed_with(false, 655);
    let (mut session, target_id) = setup_ordinary_target(&encoded);
    let before = state(&session);
    let surface = realm_unit(&before, &target_id).expect("surface ordinary minion");
    assert_eq!(surface["location"], "C1");
    assert_eq!(surface["region"], "surface");

    let (cast, settled) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&settled),
        [
            "magic-cast",
            "minion-burrowed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(
        settled.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": target_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after = state(&session);
    assert!(realm_unit(&after, &target_id).is_none());
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0656_bury_ordinary_minion_on_water_is_a_paid_noop() {
    let encoded = seed_with(true, 656);
    let (mut session, target_id) = setup_ordinary_target(&encoded);
    let before = realm_unit(&state(&session), &target_id)
        .expect("Water target")
        .clone();
    assert_eq!(before["location"], "C1");
    assert_eq!(before["region"], "surface");

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert_eq!(
        realm_unit(&state(&session), &target_id).expect("unchanged target"),
        &before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1036_bury_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_bury_seed_with(1036);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete bury Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
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
    assert!(realm_unit(&paused, &visitor_id).is_some());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(bury_targets(session).is_empty());

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
    assert!(realm_unit(&resumed, &visitor_id).is_some());
    assert_eq!(bury_targets(session), [visitor_id.as_str()]);

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-burrowed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "cell": "C4",
            "instanceId": visitor_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    assert!(realm_unit(&state(session), &visitor_id).is_none());
    assert!(
        state(session)["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == visitor_id)
    );
    assert_exact_replay(session);
}

fn roaming_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bury_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "bury-magic-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-bury-magic-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": roaming_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bury"; 8],
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
        .chain(655..655 + 2048)
        .map(bury_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-bury")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Bury and required South minions")
}

fn bury_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-bury")
                .count()
        })
        .unwrap_or_default()
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == instance_id))
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
        .expect("Bury target identity")
        .to_owned()
}

fn setup_c1_with_minions(session: &mut Session, count: usize) -> Vec<String> {
    south_plays_c1(session);
    (0..count).map(|_| summon_south_at(session, "C1")).collect()
}

fn cast_bury_target(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = opening_main(encoded);
    let c1_ids = setup_c1_with_minions(&mut session, 2);
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
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let far_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (!bury_targets(&session).is_empty()).then_some((session, c1_ids, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(655..655 + 2048)
        .find_map(|seed| {
            let encoded = bury_supplemental_manifest(seed);
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
        .expect("bounded seed reaching Bury far-minion setup")
}

fn try_second_bury_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_bury_target(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if bury_spells_in_hand(&state(&session)) < 1 {
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
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C2"
            && descriptor["region"].is_null()
    })?;
    let minion_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    bury_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_bury_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(655..655 + 8192)
        .find_map(|seed| {
            let encoded = bury_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_bury_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Bury enemy-arrival setup")
}

fn try_second_bury_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_bury_target(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if bury_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let minion_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    bury_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_bury_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(655..655 + 8192)
        .find_map(|seed| {
            let encoded = bury_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_bury_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Bury new-summon setup")
}

#[test]
fn rule_catalog_1913_buried_minion_stays_in_cemetery_after_turns_pass() {
    let encoded = seed_with_start(1913, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_bury_target(&mut session, &target_id);
    assert!(cemetery_has(&state(&session), "south", &target_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(cemetery_has(&state(&session), "south", &target_id));
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1914_second_bury_without_a_surface_target_stays_unoffered() {
    let encoded = seed_with_start(1914, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_bury_target(&mut session, &target_id);
    assert!(bury_spells_in_hand(&state(&session)) >= 1);
    assert!(bury_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| descriptor["kind"]
        == "cast-magic"
        && descriptor["cardId"] == "north-bury"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1915_second_bury_kills_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_bury_enemy_arrival(1915);
    let (mut session, minion_id) =
        try_second_bury_enemy_arrival_prefix(&encoded).expect("second Bury enemy-arrival prefix");
    let receipt = cast_bury_target(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1916_bury_offers_every_surface_minion_at_the_target_land_site() {
    let encoded = seed_with_start(1916, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c1_with_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = bury_targets(&session);
    assert_eq!(offered.len(), 2);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1917_bury_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(1917);
    let (mut session, c1_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("Bury far-minion prefix");
    let buried_id = &c1_ids[0];
    cast_bury_target(&mut session, buried_id);
    assert!(cemetery_has(&state(&session), "south", buried_id));
    assert!(realm_unit(&state(&session), &far_id).is_some());
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("far minion")["location"],
        "C4"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1918_second_bury_kills_a_newly_summoned_minion() {
    let encoded = seed_for_second_bury_new_summon(1918);
    let (mut session, minion_id) =
        try_second_bury_new_summon_prefix(&encoded).expect("second Bury new-summon prefix");
    let receipt = cast_bury_target(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2243_buried_minion_stays_in_cemetery_after_turns_pass() {
    let encoded = seed_with_start(2243, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_bury_target(&mut session, &target_id);
    assert!(cemetery_has(&state(&session), "south", &target_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(cemetery_has(&state(&session), "south", &target_id));
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2244_second_bury_without_a_surface_target_stays_unoffered() {
    let encoded = seed_with_start(2244, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_bury_target(&mut session, &target_id);
    assert!(bury_spells_in_hand(&state(&session)) >= 1);
    assert!(bury_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| descriptor["kind"]
        == "cast-magic"
        && descriptor["cardId"] == "north-bury"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2245_second_bury_kills_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_bury_enemy_arrival(2245);
    let (mut session, minion_id) =
        try_second_bury_enemy_arrival_prefix(&encoded).expect("second Bury enemy-arrival prefix");
    let receipt = cast_bury_target(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2246_bury_offers_every_surface_minion_at_the_target_land_site() {
    let encoded = seed_with_start(2246, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c1_with_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = bury_targets(&session);
    assert_eq!(offered.len(), 2);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2247_bury_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(2247);
    let (mut session, c1_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("Bury far-minion prefix");
    let buried_id = &c1_ids[0];
    cast_bury_target(&mut session, buried_id);
    assert!(cemetery_has(&state(&session), "south", buried_id));
    assert!(realm_unit(&state(&session), &far_id).is_some());
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("far minion")["location"],
        "C4"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2248_second_bury_kills_a_newly_summoned_minion() {
    let encoded = seed_for_second_bury_new_summon(2248);
    let (mut session, minion_id) =
        try_second_bury_new_summon_prefix(&encoded).expect("second Bury new-summon prefix");
    let receipt = cast_bury_target(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}
