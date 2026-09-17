//! Direct proofs for return-target-minion-to-owner-hand Magic (RULE-CATALOG-0621–0622,
//! RULE-CATALOG-1025).
//!
//! Bounce Magic returns a healthy same-region minion to its owner's hand.
//! Enemy Ward absorbs the return without moving the minion. While Deathrites
//! wait for ordering, bounce Magic stays withheld until the chain drains.

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

fn minion(ward: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if ward {
        value["ward"] = json!(true);
    }
    value
}

fn bounce_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetMinionToOwnerHand": true,
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

fn bounce_manifest(seed: u32, ward: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "bounce-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-bounce-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bounce": bounce_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(ward),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bounce"; 6],
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
    let mut session = Session::new(encoded).expect("valid bounce session");
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

fn bounce_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("bounce actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-bounce"
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

fn deathrite_bounce_manifest(seed: u32) -> String {
    let fixture = "bounce-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bounce": bounce_spell(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-bounce",
                    "north-rain",
                    "north-rain",
                    "north-bounce",
                    "north-rain",
                    "north-bounce",
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

fn north_has_bounce_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-bounce", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteBounceSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteBounceSetup> {
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
    if !north_has_bounce_and_rain(&state(&session)) {
        return None;
    }
    if bounce_targets(&session).is_empty() {
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
    Some(PendingDeathriteBounceSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_bounce_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_bounce_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with bounce Magic in hand")
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
    let enemy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_id
}

#[test]
fn rule_catalog_0621_bounce_magic_returns_a_healthy_minion_to_its_owners_hand() {
    let encoded = bounce_manifest(621, false);
    let mut session = opening_main(&encoded);
    let enemy_id = stage_south_minion(&mut session);
    let before = state(&session);
    let north_avatar = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    let south_hand_before = before["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South Spellbook hand")
        .len();
    assert_eq!(bounce_targets(&session), [enemy_id.as_str()]);
    assert!(!bounce_targets(&session).contains(&north_avatar));
    assert!(!bounce_targets(&session).contains(&south_avatar));

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-bounce"
    });
    assert_eq!(descriptor["target"]["instanceId"], enemy_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    let returned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-returned-to-hand")
        .expect("bounce return event");
    assert_eq!(returned.payload["cardId"], "south-minion");
    assert_eq!(returned.payload["instanceId"], enemy_id);
    assert_eq!(returned.payload["owner"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &enemy_id).is_none());
    assert!(
        finished["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == enemy_id)
    );
    assert_eq!(
        finished["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .len(),
        south_hand_before + 1
    );
    let north_view = session
        .public_view(Seat::North)
        .expect("North public view after the bounce");
    assert_eq!(
        north_view["players"]["south"]["hand"]["spellbook"],
        south_hand_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0622_bounce_magic_ward_absorbs_the_return() {
    let encoded = bounce_manifest(622, true);
    let mut session = opening_main(&encoded);
    let enemy_id = stage_south_minion(&mut session);
    assert_eq!(bounce_targets(&session), [enemy_id.as_str()]);
    assert_eq!(
        realm_unit(&state(&session), &enemy_id).expect("warded enemy")["warded"],
        true
    );
    let hand_before = state(&session)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South Spellbook hand")
        .len();

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bounce"
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-returned-to-hand")
    );

    let after = state(&session);
    assert!(realm_unit(&after, &enemy_id).is_some());
    assert_eq!(
        after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1025_bounce_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_bounce_seed_with(1025);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete bounce Deathrite withheld setup");
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
    assert!(bounce_targets(session).is_empty());

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
    assert_eq!(bounce_targets(session), [visitor_id.as_str()]);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bounce"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    assert!(realm_unit(&state(session), &visitor_id).is_none());
    assert_exact_replay(session);
}
