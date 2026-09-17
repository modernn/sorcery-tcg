//! Direct proofs for measured disable-until-damaged Magic (RULE-CATALOG-0527–0528,
//! RULE-CATALOG-1063).
//!
//! Ordinary Magic can disable a minion at a location up to two measured
//! cardinal steps away until that minion takes damage. Unlike Freeze, enemy
//! Ward absorbs the disable, the flag does not expire at the caster's next
//! Start Phase, and later positive damage wakes the minion. Avatars are never
//! offered. Enemy Stealth is excluded. While Deathrites wait for ordering,
//! Sleep Magic stays withheld until the chain drains.

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

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn far() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn sleep() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetMinionWithinTwoStepsUntilDamaged": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
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

fn warded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    })
}

fn plain() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
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

fn wake_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "disable-until-damaged-wake" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-disable-until-damaged-wake-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-site": site(),
            "north-sleep": sleep(),
            "north-zap": zap(),
            "south-avatar": avatar(),
            "south-far": far(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-sleep",
                    "north-sleep",
                    "north-zap",
                    "north-zap",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-far"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn ward_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "disable-until-damaged-ward" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-disable-until-damaged-ward-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-sleep": sleep(),
            "south-avatar": avatar(),
            "south-plain": plain(),
            "south-site": site(),
            "south-warded": warded(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-sleep"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-plain",
                    "south-plain",
                    "south-plain",
                    "south-warded",
                    "south-warded",
                    "south-warded",
                ],
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
    try_accept_where(session, predicate).expect("expected engine-issued action")
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid disable-until-damaged session");
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

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn avatar_instance_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar instance identity")
        .to_owned()
}

fn sleep_target_ids(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Sleep actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-sleep"
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

fn has_activate_mana(session: &Session, instance_id: &str) -> bool {
    session
        .legal_actions()
        .expect("mana actions")
        .into_iter()
        .any(|action| {
            action.descriptor["kind"] == "activate-mana"
                && action.descriptor["unitInstanceId"] == instance_id
        })
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

fn pass_south_first_site(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn north_second_main(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn full_turn_cycle_to_north_main(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

#[test]
fn rule_catalog_0527_measured_disable_offers_two_steps_and_wakes_on_damage() {
    let encoded = (527..527 + 256)
        .map(wake_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate, "north");
            ["north-ally", "north-sleep", "north-zap"]
                .into_iter()
                .all(|card_id| hand.iter().any(|id| id == card_id))
        })
        .expect("bounded seed with ally, Sleep, and damage Magic in the opening hand");
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    pass_south_first_site(&mut session);
    let (far_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let far_id = far_summon["cardInstanceId"]
        .as_str()
        .expect("far instance identity")
        .to_owned();
    north_second_main(&mut session);

    let before = state(&session);
    let north_avatar = avatar_instance_id(&before, "north");
    let south_avatar = avatar_instance_id(&before, "south");
    let targets = sleep_target_ids(&session);
    assert!(
        targets.contains(&ally_id),
        "a minion at the caster's location is within two measured steps"
    );
    assert!(
        !targets.contains(&far_id),
        "a minion three measured steps away is outside Sleep range"
    );
    assert!(!targets.contains(&north_avatar) && !targets.contains(&south_avatar));
    assert!(
        has_activate_mana(&session, &ally_id),
        "the ready ally can tap for mana before Sleep"
    );

    let (_, slept) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-sleep"
            && descriptor["target"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&slept),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert_eq!(slept.events[1].payload["instanceId"], ally_id);
    assert!(slept.events[1].payload["expiresAtSeat"].is_null());
    let after_sleep = state(&session);
    assert_eq!(unit(&after_sleep, &ally_id)["disabledUntilDamaged"], true);
    assert!(!has_activate_mana(&session, &ally_id));

    let (_, woken) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == ally_id
    });
    assert!(
        event_types(&woken).contains(&"minion-awakened"),
        "positive damage wakes a Sleep-disabled minion"
    );
    let after_wake = state(&session);
    assert!(unit(&after_wake, &ally_id)["disabledUntilDamaged"].is_null());
    assert!(has_activate_mana(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0528_enemy_ward_absorbs_and_disable_survives_a_turn_cycle() {
    let encoded = (528..528 + 256)
        .map(ward_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate, "south");
            hand.iter().any(|id| id == "south-plain") && hand.iter().any(|id| id == "south-warded")
        })
        .expect("bounded seed with warded and plain South minions in the opening hand");
    let mut session = opening_main(&encoded);
    pass_south_first_site(&mut session);
    let (warded_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-warded"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let warded_id = warded_summon["cardInstanceId"]
        .as_str()
        .expect("warded instance identity")
        .to_owned();
    let (plain_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-plain"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let plain_id = plain_summon["cardInstanceId"]
        .as_str()
        .expect("plain instance identity")
        .to_owned();
    north_second_main(&mut session);

    let targets = sleep_target_ids(&session);
    assert!(targets.contains(&warded_id) && targets.contains(&plain_id));

    let (_, absorbed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-sleep"
            && descriptor["target"]["instanceId"] == warded_id
    });
    assert_eq!(
        event_types(&absorbed),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let after_ward = state(&session);
    assert_eq!(unit(&after_ward, &warded_id)["warded"], false);
    assert!(unit(&after_ward, &warded_id)["disabledUntilDamaged"].is_null());

    let (_, slept) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-sleep"
            && descriptor["target"]["instanceId"] == plain_id
    });
    assert_eq!(
        event_types(&slept),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    let after_sleep = state(&session);
    assert_eq!(unit(&after_sleep, &plain_id)["disabledUntilDamaged"], true);

    full_turn_cycle_to_north_main(&mut session);
    let after_cycle = state(&session);
    assert_eq!(unit(&after_cycle, &plain_id)["disabledUntilDamaged"], true);
    assert!(
        session
            .transcript()
            .iter()
            .all(|receipt| !event_types(receipt).contains(&"minion-disable-expired")),
        "Sleep disable is not a Freeze-style Start Phase expiry"
    );
    assert_exact_replay(&session);
}

fn deathrite_sleep_manifest(seed: u32) -> String {
    let fixture = "disable-until-damaged-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "north-sleep": sleep(),
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
                    "north-sleep",
                    "north-rain",
                    "north-rain",
                    "north-sleep",
                    "north-rain",
                    "north-sleep",
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

fn north_has_sleep_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-sleep", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteSleepSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteSleepSetup> {
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
    if !north_has_sleep_and_rain(&state(&session)) {
        return None;
    }
    if sleep_target_ids(&session).is_empty() {
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
    Some(PendingDeathriteSleepSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_sleep_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_sleep_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Sleep Magic in hand")
}

#[test]
fn rule_catalog_1063_disable_until_damaged_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_sleep_seed_with(1063);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete Sleep Deathrite withheld setup");
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
    assert_eq!(
        unit(&paused, &visitor_id)["disabledUntilDamaged"],
        Value::Null
    );
    assert!(session
        .legal_actions()
        .expect("paused legal actions")
        .iter()
        .all(|action| action.descriptor["kind"] != "cast-magic"));
    assert!(sleep_target_ids(session).is_empty());

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
    assert_eq!(
        unit(&resumed, &visitor_id)["disabledUntilDamaged"],
        Value::Null
    );
    assert_eq!(sleep_target_ids(session), [visitor_id.as_str()]);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-sleep"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], visitor_id);
    assert!(receipt.events[1].payload["expiresAtSeat"].is_null());
    assert_eq!(
        unit(&state(session), &visitor_id)["disabledUntilDamaged"],
        true
    );
    assert_exact_replay(session);
}
