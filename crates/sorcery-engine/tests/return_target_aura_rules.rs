//! Direct proofs for return-target-aura-to-owner-hand Magic
//! (RULE-CATALOG-0669–0670, RULE-CATALOG-1084, RULE-CATALOG-2313–2318).
//!
//! Targeted realm Aura bounce returns a Flood in play to its owner's hidden
//! Spellbook hand. It is not cemetery Aura return: a minion in play is not a
//! target, and an empty realm offers no cast. While Deathrites wait for
//! ordering, return-aura Magic stays withheld until the chain drains.
//! Supplemental 2313–2318 bind persistence, no-target repeat, enemy-arrival,
//! multi-aura offer, unselected remainder, and a newly placed Aura.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn return_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetAuraToOwnerHand": true,
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

fn return_aura_manifest(seed: u32, south_plays_flood: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-return": return_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    if south_plays_flood {
        cards["south-flood"] = flood();
    } else {
        cards["south-minion"] = minion();
    }
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "return-target-aura" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-target-aura-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-return"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": if south_plays_flood {
                    vec!["south-flood"; 6]
                } else {
                    vec!["south-minion"; 6]
                },
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
    let mut session = Session::new(encoded).expect("valid return-aura session");
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

fn south_affinity(session: &Session) -> (u64, u64) {
    let view = session.public_view(Seat::South).expect("South public view");
    (
        view["players"]["south"]["affinity"]["earth"]
            .as_u64()
            .expect("earth affinity"),
        view["players"]["south"]["affinity"]["water"]
            .as_u64()
            .expect("water affinity"),
    )
}

fn return_aura_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("return-aura actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return"
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

fn stage_south_flood(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "south-flood"
            && cells_include(descriptor, "C1")
    });
    let aura_id = state(session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("Flood identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    aura_id
}

fn stage_south_minion(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    minion_id
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

fn deathrite_return_aura_manifest(seed: u32) -> String {
    let fixture = "return-target-aura-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-return": return_spell(),
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
                    "north-return",
                    "north-rain",
                    "north-rain",
                    "north-return",
                    "north-rain",
                    "north-return",
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

fn north_has_return_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-return", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn realm_has_aura(value: &Value, instance_id: &str) -> bool {
    value["realm"]
        .get("auras")
        .and_then(Value::as_array)
        .is_some_and(|auras| auras.iter().any(|aura| aura["instanceId"] == instance_id))
}

struct PendingDeathriteReturnAuraSetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_return_aura_target(
    encoded: &str,
) -> Option<PendingDeathriteReturnAuraSetup> {
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
    if !north_has_return_and_rain(&state(&session)) {
        return None;
    }
    if return_aura_targets(&session).is_empty() {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteReturnAuraSetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_return_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_return_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_return_aura_target(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with return-aura Magic in hand")
}

#[test]
fn rule_catalog_0669_return_target_aura_returns_flood_to_its_owners_hand() {
    let mut session = opening_main(&return_aura_manifest(669, true));
    let aura_id = stage_south_flood(&mut session);
    let before = state(&session);
    let south_hand_before = before["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South Spellbook hand")
        .len();
    assert_eq!(south_affinity(&session), (1, 1));
    assert_eq!(
        return_aura_targets(&session).as_slice(),
        std::slice::from_ref(&aura_id)
    );
    assert_eq!(before["realm"]["auras"][0]["cardId"], "south-flood");
    assert!(
        !session
            .legal_actions()
            .expect("return-aura actions")
            .iter()
            .any(|action| action.descriptor.get("cemeteryMinionInstanceId").is_some())
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    let returned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "aura-returned-to-hand")
        .expect("Aura return");
    assert_eq!(returned.payload["cardId"], "south-flood");
    assert_eq!(returned.payload["instanceId"], aura_id);
    assert_eq!(returned.payload["owner"], "south");
    assert_eq!(returned.payload["seat"], "south");
    assert_eq!(
        returned.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-destroyed"
                || event.event_type == "aura-dispelled"
                || event.event_type == "aura-banished")
    );

    let after = state(&session);
    assert!(after["realm"].get("auras").is_none());
    assert!(
        after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert_eq!(
        after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .len(),
        south_hand_before + 1
    );
    assert!(
        !after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(
        north_view["players"]["south"]["hand"]["spellbook"],
        json!(south_hand_before + 1)
    );
    assert_eq!(south_affinity(&session), (1, 0));
    assert_eq!(return_aura_targets(&session), Vec::<String>::new());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0670_return_target_aura_offers_no_target_without_a_realm_aura() {
    let mut session = opening_main(&return_aura_manifest(670, false));
    let minion_id = stage_south_minion(&mut session);
    let before = state(&session);
    assert!(
        before["realm"]
            .get("auras")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    assert_eq!(return_aura_targets(&session), Vec::<String>::new());
    assert!(
        !session
            .legal_actions()
            .expect("post-staging actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return")
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
            .any(|card| card["cardId"] == "north-return")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1084_return_target_aura_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_return_aura_seed_with(1084);
    let mut setup = try_pending_deathrite_with_return_aura_target(&encoded)
        .expect("complete return-aura Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
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
    assert!(return_aura_targets(session).is_empty());

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(realm_has_aura(&resumed, &aura_id));
    assert_eq!(
        return_aura_targets(session).as_slice(),
        std::slice::from_ref(&aura_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    assert!(!realm_has_aura(&state(session), &aura_id));
    assert_exact_replay(session);
}

fn return_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "return-target-aura-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-target-aura-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-return": return_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-flood": flood(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-return"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-flood"; 8],
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
        .chain(669..669 + 2048)
        .map(return_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-return")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-flood")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with return-aura Magic and required South Floods")
}

fn return_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-return")
                .count()
        })
        .unwrap_or_default()
}

fn realm_aura_ids(snapshot: &Value) -> Vec<String> {
    snapshot["realm"]
        .get("auras")
        .and_then(Value::as_array)
        .map(|auras| {
            auras
                .iter()
                .filter_map(|aura| aura["instanceId"].as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn hand_has_instance(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["instanceId"] == instance_id))
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

fn setup_south_floods(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut ids = Vec::new();
    for _ in 0..count {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-aura"
                && descriptor["cardId"] == "south-flood"
                && cells_include(descriptor, "C1")
        });
        let id = realm_aura_ids(&state(session))
            .into_iter()
            .find(|id| !ids.contains(id))
            .expect("new Flood identity");
        ids.push(id);
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    ids
}

fn cast_return_on(session: &mut Session, aura_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    receipt
}

fn try_second_return_enemy_aura_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_south_floods(&mut session, 1).into_iter().next()?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetAuraInstanceId"] == first_id
    })?;
    pass_turn_to_north_spellbook(&mut session);
    if return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "south-flood"
            && cells_include(descriptor, "C2")
    })?;
    let aura_id = realm_aura_ids(&state(&session)).into_iter().next()?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    return_aura_targets(&session)
        .contains(&aura_id)
        .then_some((session, aura_id))
}

fn seed_for_second_return_enemy_aura(start: u32) -> String {
    (start..start + 8192)
        .chain(669..669 + 8192)
        .find_map(|seed| {
            let encoded = return_supplemental_manifest(seed);
            if !opening_hand_spell_ids(&encoded, "north")
                .iter()
                .any(|card| card == "north-return")
                || opening_hand_spell_ids(&encoded, "south")
                    .iter()
                    .filter(|card| *card == "south-flood")
                    .count()
                    < 1
            {
                return None;
            }
            try_second_return_enemy_aura_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second return-aura enemy-arrival setup")
}

fn try_second_return_new_aura_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_south_floods(&mut session, 1).into_iter().next()?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetAuraInstanceId"] == first_id
    })?;
    if realm_has_aura(&state(&session), &first_id) {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "south-flood"
            && cells_include(descriptor, "C1")
    })?;
    let aura_id = realm_aura_ids(&state(&session)).into_iter().next()?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    return_aura_targets(&session)
        .contains(&aura_id)
        .then_some((session, aura_id))
}

fn seed_for_second_return_new_aura(start: u32) -> String {
    (start..start + 8192)
        .chain(669..669 + 8192)
        .find_map(|seed| {
            let encoded = return_supplemental_manifest(seed);
            if !opening_hand_spell_ids(&encoded, "north")
                .iter()
                .any(|card| card == "north-return")
                || opening_hand_spell_ids(&encoded, "south")
                    .iter()
                    .filter(|card| *card == "south-flood")
                    .count()
                    < 1
            {
                return None;
            }
            try_second_return_new_aura_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second return-aura new-placement setup")
}

#[test]
fn rule_catalog_2313_returned_aura_stays_in_hand_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2313, 1);
    let mut session = opening_main(&encoded);
    let aura_id = setup_south_floods(&mut session, 1)
        .into_iter()
        .next()
        .expect("Flood identity");
    let receipt = cast_return_on(&mut session, &aura_id);
    assert!(event_types(&receipt).contains(&"aura-returned-to-hand"));
    assert!(hand_has_instance(&state(&session), "south", &aura_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(hand_has_instance(&state(&session), "south", &aura_id));
    assert!(!realm_has_aura(&state(&session), &aura_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2314_second_return_offers_no_targets_after_returning_the_only_aura() {
    let encoded = (2314..2314 + 8192)
        .chain(669..669 + 8192)
        .find_map(|seed| {
            let candidate = return_supplemental_manifest(seed);
            if !opening_hand_spell_ids(&candidate, "north")
                .iter()
                .any(|card| card == "north-return")
                || opening_hand_spell_ids(&candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-flood")
                    .count()
                    < 1
            {
                return None;
            }
            let mut session = opening_main(&candidate);
            let aura_id = setup_south_floods(&mut session, 1).into_iter().next()?;
            let first = cast_return_on(&mut session, &aura_id);
            if !event_types(&first).contains(&"aura-returned-to-hand") {
                return None;
            }
            if realm_has_aura(&state(&session), &aura_id) {
                return None;
            }
            if return_spells_in_hand(&state(&session)) < 1 {
                pass_turn_to_north_spellbook(&mut session);
            }
            (return_spells_in_hand(&state(&session)) >= 1
                && return_aura_targets(&session).is_empty())
            .then_some(candidate)
        })
        .expect("bounded seed with two return-aura casts after returning the only Aura");
    let mut session = opening_main(&encoded);
    let aura_id = setup_south_floods(&mut session, 1)
        .into_iter()
        .next()
        .expect("Flood identity");
    let first = cast_return_on(&mut session, &aura_id);
    assert!(event_types(&first).contains(&"aura-returned-to-hand"));
    assert!(!realm_has_aura(&state(&session), &aura_id));
    if return_spells_in_hand(&state(&session)) < 1 {
        pass_turn_to_north_spellbook(&mut session);
    }
    assert!(return_spells_in_hand(&state(&session)) >= 1);
    assert_eq!(return_aura_targets(&session), Vec::<String>::new());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2315_second_return_returns_a_newly_arrived_aura_after_enemy_site_placement() {
    let encoded = seed_for_second_return_enemy_aura(2315);
    let (mut session, aura_id) = try_second_return_enemy_aura_prefix(&encoded)
        .expect("second return-aura enemy-arrival prefix");
    let receipt = cast_return_on(&mut session, &aura_id);
    assert!(event_types(&receipt).contains(&"aura-returned-to-hand"));
    assert!(!realm_has_aura(&state(&session), &aura_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2316_return_target_aura_offers_every_realm_aura() {
    let encoded = supplemental_seed_with_start(2316, 2);
    let mut session = opening_main(&encoded);
    let aura_ids = setup_south_floods(&mut session, 2);
    let offered = return_aura_targets(&session);
    for aura_id in &aura_ids {
        assert!(offered.contains(aura_id));
    }
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2317_return_target_aura_leaves_a_far_aura_untouched() {
    let encoded = supplemental_seed_with_start(2317, 2);
    let mut session = opening_main(&encoded);
    let aura_ids = setup_south_floods(&mut session, 2);
    let far_id = aura_ids[0].clone();
    let near_id = aura_ids[1].clone();
    let receipt = cast_return_on(&mut session, &near_id);
    assert!(event_types(&receipt).contains(&"aura-returned-to-hand"));
    assert!(!realm_has_aura(&state(&session), &near_id));
    assert!(realm_has_aura(&state(&session), &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2318_second_return_returns_a_newly_placed_aura() {
    let encoded = seed_for_second_return_new_aura(2318);
    let (mut session, aura_id) = try_second_return_new_aura_prefix(&encoded)
        .expect("second return-aura new-placement prefix");
    let receipt = cast_return_on(&mut session, &aura_id);
    assert!(event_types(&receipt).contains(&"aura-returned-to-hand"));
    assert!(!realm_has_aura(&state(&session), &aura_id));
    assert!(hand_has_instance(&state(&session), "south", &aura_id));
    assert_exact_replay(&session);
}
