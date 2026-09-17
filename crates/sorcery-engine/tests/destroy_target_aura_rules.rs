//! Direct proofs for destroy-target-aura Magic (RULE-CATALOG-0667–0668,
//! RULE-CATALOG-1083).
//!
//! Destroy-aura Magic offers every realm Aura and moves a real Aura into its
//! owner's cemetery via `aura-destroyed`, not a return to hand or a duration
//! dispel. It offers no target when the realm has no Aura. While Deathrites
//! wait for ordering, destroy-aura Magic stays withheld until the chain drains.

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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn flood() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn destroy_spell() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetAura": true,
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

fn destroy_aura_manifest(seed: u32, south_plays_aura: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-destroy": destroy_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    let south_spell = if south_plays_aura {
        cards["south-flood"] = flood();
        "south-flood"
    } else {
        cards["south-minion"] = minion();
        "south-minion"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-aura-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-aura-magic-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-destroy"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec![south_spell; 6],
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid destroy-aura session");
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

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

fn destroy_aura_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("destroy-aura actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-destroy"
        })
        .filter_map(|action| {
            action.descriptor["targetAuraInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn stage_south_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn finish_south_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn stage_south_flood(session: &mut Session) -> String {
    stage_south_turn(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "south-flood"
            && cells_include(descriptor, "C1")
    });
    let aura_id = state(session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("Flood identity")
        .to_owned();
    finish_south_turn(session);
    aura_id
}

fn stage_south_minion(session: &mut Session) -> String {
    stage_south_turn(session);
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned();
    finish_south_turn(session);
    minion_id
}

fn deathrite_destroy_aura_manifest(seed: u32) -> String {
    let fixture = "destroy-aura-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_spell(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-flood": flood(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-destroy",
                    "north-rain",
                    "north-rain",
                    "north-destroy",
                    "north-rain",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-flood"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-minion", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_destroy_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-destroy", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteDestroyAuraSetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_destroy_aura_target(
    encoded: &str,
) -> Option<PendingDeathriteDestroyAuraSetup> {
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "south-flood"
            && cells_include(descriptor, "C1")
    })?;
    let aura_id = state(&session)["realm"]["auras"][0]["instanceId"]
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
    if !north_has_destroy_and_rain(&state(&session)) {
        return None;
    }
    if destroy_aura_targets(&session).is_empty() {
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
    Some(PendingDeathriteDestroyAuraSetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_destroy_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_destroy_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_destroy_aura_target(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with destroy-aura Magic in hand")
}

fn realm_has_aura(value: &Value, instance_id: &str) -> bool {
    value["realm"]
        .get("auras")
        .and_then(Value::as_array)
        .is_some_and(|auras| auras.iter().any(|aura| aura["instanceId"] == instance_id))
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
fn rule_catalog_0667_destroy_target_aura_sends_flood_to_its_owners_cemetery() {
    let encoded = destroy_aura_manifest(667, true);
    let mut session = opening_main(&encoded);
    assert_eq!(destroy_aura_targets(&session), Vec::<String>::new());
    let aura_id = stage_south_flood(&mut session);
    assert_eq!(
        destroy_aura_targets(&session).as_slice(),
        std::slice::from_ref(&aura_id)
    );
    assert_eq!(
        state(&session)["realm"]["auras"][0]["cardId"],
        "south-flood"
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-destroyed", "magic-resolved"]
    );
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "aura-destroyed")
        .expect("Aura destruction");
    assert_eq!(destroyed.payload["cardId"], "south-flood");
    assert_eq!(destroyed.payload["instanceId"], aura_id);
    assert_eq!(destroyed.payload["owner"], "south");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-returned-to-hand"
                || event.event_type == "aura-dispelled"
                || event.event_type == "aura-banished")
    );

    let after = state(&session);
    assert!(after["realm"].get("auras").is_none());
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert!(
        !after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert_eq!(destroy_aura_targets(&session), Vec::<String>::new());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0668_destroy_target_aura_offers_no_target_without_an_aura() {
    let encoded = destroy_aura_manifest(668, false);
    let mut session = opening_main(&encoded);
    let minion_id = stage_south_minion(&mut session);
    let before = state(&session);
    assert!(
        before["realm"]
            .get("auras")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    assert_eq!(destroy_aura_targets(&session), Vec::<String>::new());
    assert!(
        !session
            .legal_actions()
            .expect("post-staging actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-destroy")
    );

    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == minion_id)
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .iter()
            .any(|card| card["cardId"] == "north-destroy")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1083_destroy_target_aura_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_destroy_aura_seed_with(1083);
    let mut setup = try_pending_deathrite_with_destroy_aura_target(&encoded)
        .expect("complete destroy-aura Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
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
    assert!(realm_has_aura(&paused, &aura_id));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(destroy_aura_targets(session).is_empty());

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
    assert!(realm_has_aura(&resumed, &aura_id));
    assert_eq!(
        destroy_aura_targets(session).as_slice(),
        std::slice::from_ref(&aura_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-destroyed", "magic-resolved"]
    );
    assert!(!realm_has_aura(&state(session), &aura_id));
    assert_exact_replay(session);
}
