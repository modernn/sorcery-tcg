//! Direct proofs for leap-attack-ally Magic (RULE-CATALOG-0599–0600,
//! RULE-CATALOG-1046, RULE-CATALOG-1963–1968).
//!
//! Leap Attack optionally steps a controlled ally before striking every enemy
//! at the destination. Immobile allies may only stay and strike where they stand.
//! While Deathrites wait for ordering, leap Magic stays withheld until the
//! chain drains.

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

fn ally(extra: Value) -> Value {
    let mut value = json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn leap() -> Value {
    json!({
        "cardType": "magic",
        "leapAttackAlly": true,
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

fn step_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "leap-step" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-leap-step-v1",
        },
        "cards": {
            "north-ally": ally(json!({})),
            "north-avatar": avatar(),
            "north-leap": leap(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": ally(json!({ "attack": 2, "defense": 3 })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-leap", "north-ally", "north-leap", "north-ally", "north-leap", "north-ally"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn immobile_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "leap-immobile" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-leap-immobile-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-immobile": ally(json!({ "immobile": true })),
            "north-leap": leap(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-leap", "north-immobile", "north-leap", "north-immobile", "north-leap", "north-immobile"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["north-immobile"; 6],
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

fn try_setup_step_leap(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = summoned["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (enemy_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C3"
    })?;
    let enemy_id = enemy_summon["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some((session, ally_id, enemy_id))
}

fn setup_step_leap(encoded: &str) -> (Session, String, String) {
    try_setup_step_leap(encoded).expect("complete Leap Attack setup")
}

fn deathrite_leap_manifest(seed: u32) -> String {
    let fixture = "leap-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": ally(json!({})),
            "north-avatar": avatar(),
            "north-leap": leap(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": ally(json!({ "attack": 2, "defense": 3 })),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-leap",
                    "north-rain",
                    "north-leap",
                    "north-rain",
                    "north-leap",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-enemy", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_leap_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-leap", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn leap_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("leap actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-leap"
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

fn leap_ready_for_ally(session: &Session, ally_id: &str) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-leap"
                && action.descriptor["ally"]["instanceId"] == ally_id
        })
    })
}

fn leap_minion_targets(session: &Session) -> Vec<String> {
    let snapshot = state(session);
    leap_targets(session)
        .into_iter()
        .filter(|target_id| {
            snapshot["realm"]["units"]
                .as_array()
                .is_some_and(|units| units.iter().any(|unit| unit["instanceId"] == *target_id))
        })
        .collect()
}

struct PendingDeathriteLeapSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    enemy_id: String,
    session: Session,
}

fn try_pending_deathrite_with_ready_leap(encoded: &str) -> Option<PendingDeathriteLeapSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let enemy = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C3"
    })?;
    let enemy_id = enemy.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_leap_and_rain(&state(&session)) {
        return None;
    }
    if leap_targets(&session).is_empty() {
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
    Some(PendingDeathriteLeapSetup {
        ally_id,
        deathrite_ids,
        enemy_id,
        session,
    })
}

fn deathrite_leap_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_leap_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_leap(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Leap Magic in hand")
}

#[test]
fn rule_catalog_0599_leap_attack_steps_an_ally_and_strikes_enemies_at_the_destination() {
    let encoded = (599..599 + 512)
        .map(step_manifest)
        .find(|candidate| try_setup_step_leap(candidate).is_some())
        .expect("bounded seed with complete Leap Attack setup");
    let (mut session, ally_id, enemy_id) = setup_step_leap(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-leap"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["allyDestination"]["cell"] == "C3"
    });
    assert!(event_types(&receipt).contains(&"unit-stepped"));
    assert!(event_types(&receipt).contains(&"strike-damage-allocated"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    let after = state(&session);
    assert_eq!(
        realm_unit(&after, &ally_id).expect("surviving ally")["location"],
        "C3"
    );
    assert!(realm_unit(&after, &enemy_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0600_leap_attack_lets_an_immobile_ally_only_stay_and_strike() {
    let encoded = (600..600 + 256)
        .map(immobile_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                state(&preview)["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| {
                        hand.iter().any(|card| card["cardId"] == "north-leap")
                            && hand.iter().any(|card| card["cardId"] == "north-immobile")
                    })
            })
        })
        .expect("bounded seed with Leap and immobile ally in opening hand");
    let mut session = Session::new(&encoded).expect("valid immobile Leap session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-immobile"
            && descriptor["cell"] == "C4"
    });
    let immobile_id = summoned["cardInstanceId"]
        .as_str()
        .expect("immobile identity")
        .to_owned();
    let leap_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .find(|card| card["cardId"] == "north-leap")
        .expect("leap in hand")["instanceId"]
        .as_str()
        .expect("leap identity")
        .to_owned();
    let leap_actions: Vec<_> = session
        .legal_actions()
        .expect("leap actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == leap_id
                && action.descriptor["ally"]["instanceId"] == immobile_id
        })
        .collect();
    assert_eq!(leap_actions.len(), 1);
    assert_eq!(leap_actions[0].descriptor["allyDestination"]["cell"], "C4");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == leap_id
            && descriptor["ally"]["instanceId"] == immobile_id
    });
    assert!(!event_types(&receipt).contains(&"unit-stepped"));
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1046_leap_attack_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_leap_seed_with(1046);
    let mut setup = try_pending_deathrite_with_ready_leap(&encoded)
        .expect("complete leap Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
    let enemy_id = setup.enemy_id.clone();
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
    assert!(realm_unit(&paused, &ally_id).is_some());
    assert!(realm_unit(&paused, &enemy_id).is_some());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(leap_targets(session).is_empty());

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
    assert!(realm_unit(&resumed, &ally_id).is_some());
    assert!(realm_unit(&resumed, &enemy_id).is_some());
    assert!(leap_targets(session).contains(&ally_id));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-leap"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["allyDestination"]["cell"] == "C3"
    });
    assert!(event_types(&receipt).contains(&"unit-stepped"));
    assert!(event_types(&receipt).contains(&"strike-damage-allocated"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    assert!(realm_unit(&state(session), &enemy_id).is_none());
    assert_exact_replay(session);
}

fn leap_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "leap-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-leap-supplemental-v1",
        },
        "cards": {
            "north-ally": ally(json!({})),
            "north-avatar": avatar(),
            "north-leap": leap(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": ally(json!({ "attack": 2, "defense": 3 })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-leap", "north-ally", "north-leap", "north-ally", "north-leap",
                    "north-ally", "north-leap", "north-ally", "north-leap", "north-ally",
                    "north-leap", "north-ally",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 12],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(599..599 + 2048)
        .map(leap_supplemental_manifest)
        .find(|candidate| {
            required
                .iter()
                .all(|card| opening_spell_ids(candidate).iter().any(|id| id == card))
        })
        .expect("bounded seed with required Leap opening cards")
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn leap_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-leap")
                .count()
        })
        .unwrap_or_default()
}

fn cast_leap_to_c3(session: &mut Session, ally_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-leap"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["allyDestination"]["cell"] == "C3"
    });
    receipt
}

fn try_cast_leap_to_c3(session: &mut Session, ally_id: &str) -> Option<Receipt> {
    let (_, receipt) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-leap"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["allyDestination"]["cell"] == "C3"
    })?;
    Some(receipt)
}

fn advance_full_round(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn try_summon_north_ally_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn try_two_ally_setup_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let first = try_summon_north_ally_at(&mut session, "C4")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let second = try_summon_north_ally_at(&mut session, "C1")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (enemy_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C3"
    })?;
    let enemy_id = enemy_summon["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let offered = leap_targets(&session);
    (offered.contains(&first) && offered.contains(&second)).then_some((
        session,
        vec![first, second],
        enemy_id,
    ))
}

fn seed_for_two_ally_leap_targets(start: u32) -> String {
    (start..start + 16384)
        .chain(599..599 + 16384)
        .find_map(|seed| try_two_ally_setup_prefix(&leap_supplemental_manifest(seed)).map(|_| seed))
        .map(leap_supplemental_manifest)
        .expect("bounded seed reaching two-ally Leap target setup")
}

fn try_far_enemy_prefix(encoded: &str) -> Option<(Session, String, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally_id = try_summon_north_ally_at(&mut session, "C4")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let far_id = try_summon_south_enemy_at(&mut session, "C1")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let nearby_id = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (leap_spells_in_hand(&state(&session)) >= 1 && leap_ready_for_ally(&session, &ally_id))
        .then_some((session, ally_id, nearby_id, far_id))
}

fn try_summon_south_enemy_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn seed_for_far_enemy(start: u32) -> String {
    (start..start + 16384)
        .chain(599..599 + 16384)
        .find_map(|seed| try_far_enemy_prefix(&leap_supplemental_manifest(seed)).map(|_| seed))
        .map(leap_supplemental_manifest)
        .expect("bounded seed reaching Leap far-enemy setup")
}

fn try_second_leap_new_enemy_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let (mut session, ally_id, first_enemy) = try_setup_step_leap(encoded)?;
    let receipt = try_cast_leap_to_c3(&mut session, &ally_id)?;
    if !event_types(&receipt).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &first_enemy).is_some() {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if leap_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let second_enemy = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (leap_spells_in_hand(&state(&session)) >= 1 && leap_ready_for_ally(&session, &ally_id))
        .then_some((session, ally_id, second_enemy))
}

fn seed_for_second_leap_new_enemy(start: u32) -> String {
    (start..start + 16384)
        .chain(599..599 + 16384)
        .find_map(|seed| {
            try_second_leap_new_enemy_prefix(&leap_supplemental_manifest(seed)).map(|_| seed)
        })
        .map(leap_supplemental_manifest)
        .expect("bounded seed reaching second Leap new-enemy setup")
}

fn try_second_leap_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let (mut session, ally_id, first_enemy) = try_setup_step_leap(encoded)?;
    let receipt = try_cast_leap_to_c3(&mut session, &ally_id)?;
    if !event_types(&receipt).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &first_enemy).is_some() {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if leap_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let second_enemy = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (leap_spells_in_hand(&state(&session)) >= 1 && leap_ready_for_ally(&session, &ally_id))
        .then_some((session, ally_id, second_enemy))
}

fn seed_for_second_leap_enemy_arrival(start: u32) -> String {
    (start..start + 16384)
        .chain(599..599 + 16384)
        .find_map(|seed| {
            try_second_leap_enemy_arrival_prefix(&leap_supplemental_manifest(seed)).map(|_| seed)
        })
        .map(leap_supplemental_manifest)
        .expect("bounded seed reaching second Leap enemy-arrival setup")
}

#[test]
fn rule_catalog_1963_leaped_ally_stays_at_its_destination_after_turns_pass() {
    let encoded = seed_with_start(1963, &["north-leap"]);
    let (mut session, ally_id, _enemy_id) = setup_step_leap(&encoded);
    cast_leap_to_c3(&mut session, &ally_id);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C3");
    advance_full_round(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1964_second_leap_without_an_enemy_is_still_a_paid_noop() {
    let encoded = (1964..1964 + 8192)
        .chain(599..599 + 8192)
        .map(leap_supplemental_manifest)
        .find(|candidate| {
            let (mut session, ally_id, enemy_id) = setup_step_leap(candidate);
            if leap_spells_in_hand(&state(&session)) < 2 {
                return false;
            }
            let first = cast_leap_to_c3(&mut session, &ally_id);
            if !event_types(&first).contains(&"minion-died") {
                return false;
            }
            realm_unit(&state(&session), &enemy_id).is_none()
                && leap_spells_in_hand(&state(&session)) >= 1
        })
        .expect("bounded seed with two Leap casts after setup");
    let (mut session, ally_id, enemy_id) = setup_step_leap(&encoded);
    let first = cast_leap_to_c3(&mut session, &ally_id);
    assert!(event_types(&first).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert!(leap_spells_in_hand(&state(&session)) >= 1);
    let second = cast_leap_to_c3(&mut session, &ally_id);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"strike-damage-allocated"));
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1965_second_leap_strikes_a_newly_arrived_enemy_after_enemy_site_placement() {
    let encoded = seed_for_second_leap_enemy_arrival(1965);
    let (mut session, ally_id, enemy_id) =
        try_second_leap_enemy_arrival_prefix(&encoded).expect("second Leap enemy-arrival prefix");
    let receipt = cast_leap_to_c3(&mut session, &ally_id);
    assert!(event_types(&receipt).contains(&"strike-damage-allocated"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1966_leap_offers_every_controlled_ally() {
    let encoded = seed_for_two_ally_leap_targets(1966);
    let (session, ally_ids, _enemy_id) =
        try_two_ally_setup_prefix(&encoded).expect("two-ally Leap prefix");
    let offered = leap_minion_targets(&session);
    assert_eq!(offered.len(), 2);
    for ally_id in &ally_ids {
        assert!(offered.contains(ally_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1967_leap_leaves_a_far_enemy_unstruck() {
    let encoded = seed_for_far_enemy(1967);
    let (mut session, ally_id, nearby_id, far_id) =
        try_far_enemy_prefix(&encoded).expect("Leap far-enemy prefix");
    let receipt = cast_leap_to_c3(&mut session, &ally_id);
    assert!(event_types(&receipt).contains(&"strike-damage-allocated"));
    assert!(realm_unit(&state(&session), &nearby_id).is_none());
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C1");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1968_second_leap_strikes_a_newly_summoned_enemy() {
    let encoded = seed_for_second_leap_new_enemy(1968);
    let (mut session, ally_id, enemy_id) =
        try_second_leap_new_enemy_prefix(&encoded).expect("second Leap new-enemy prefix");
    let receipt = cast_leap_to_c3(&mut session, &ally_id);
    assert!(event_types(&receipt).contains(&"strike-damage-allocated"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert_exact_replay(&session);
}
