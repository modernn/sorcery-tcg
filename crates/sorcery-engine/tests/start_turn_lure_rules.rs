//! Direct proofs for start-turn nearby enemy lures (RULE-CATALOG-0256–0257,
//! 0403–0404, RULE-CATALOG-0926, RULE-CATALOG-0960).
//!
//! Official cards such as Guile Sirens force a nearby same-region enemy minion
//! to take one card-effect step toward the source. The ability is mandatory,
//! does not tap, and cannot be intercepted.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-lure-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-lure-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": minion(json!({
                "atStartOfControllerTurnLureNearbyEnemyMinion": true,
                "defense": 4,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step");
    let StepResult::Accepted(receipt) = result else {
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn unit_id(session: &Session, card_id: &str) -> Value {
    state(session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("expected unit")["instanceId"]
        .clone()
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

fn after_draw_site_lure_source_summoned() -> Session {
    let mut session =
        Session::new(&draw_site_lure_stack_manifest()).expect("valid draw-site-lure session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn draw_site_lure_stack_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-site-lure-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-site-lure-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnLureNearbyEnemyMinion": true,
                "defense": 4,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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
        "seed": 960,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn after_draw_lure_source_summoned() -> Session {
    let mut session = Session::new(&draw_lure_stack_manifest()).expect("valid draw-lure session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn draw_lure_stack_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-lure-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-lure-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSpells": 1,
                "atStartOfControllerTurnLureNearbyEnemyMinion": true,
                "defense": 4,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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
        "seed": 403,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn after_source_summoned() -> Session {
    let mut session = Session::new(&manifest()).expect("valid lure session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn resolve_empty_start_turn(session: &mut Session, source_id: &Value) {
    assert_eq!(state(session)["phase"], "start-turn");
    let legal = session.legal_actions().expect("empty start-turn lure");
    assert!(legal.iter().all(|action| {
        action.descriptor["kind"] == "resolve-start-turn-trigger"
            && action.descriptor["sourceInstanceId"] == *source_id
            && action.descriptor.get("lureTargetInstanceId").is_none()
    }));
    accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == *source_id
            && descriptor.get("lureTargetInstanceId").is_none()
    });
}

#[test]
fn rule_catalog_0256_start_turn_lure_forces_a_nearby_enemy_one_step_closer() {
    let mut session = after_source_summoned();
    let source_id = unit_id(&session, "north-source");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    resolve_empty_start_turn(&mut session, &source_id);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let target_id = unit_id(&session, "south-minion");
    assert_eq!(state(&session)["phase"], "start-turn");
    let legal = session.legal_actions().expect("mandatory start-turn lure");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
                && action.descriptor["lureTargetInstanceId"] == target_id
        }),
        "a nearby enemy minion makes the lure mandatory"
    );
    assert!(legal.iter().any(|action| {
        action.descriptor["lureDestination"] == json!({ "cell": "C4", "region": "surface" })
    }));
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor["lureTargetInstanceId"] == target_id
            && descriptor["lureDestination"] == json!({ "cell": "C4", "region": "surface" })
    });
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "unit-lured"
                && event.payload["targetInstanceId"] == target_id
                && event.payload["to"] == json!({ "cell": "C4", "region": "surface" })
                && event.payload["steps"] == 1)
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("lured minion");
    assert_eq!(target["location"], "C4");
    assert_eq!(target["tapped"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0257_start_turn_lure_no_ops_when_no_enemy_is_nearby() {
    let mut session = after_source_summoned();
    let source_id = unit_id(&session, "north-source");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let target_id = unit_id(&session, "south-minion");
    resolve_empty_start_turn(&mut session, &source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("unmoved south minion");
    assert_eq!(target["location"], "C1");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0403_start_turn_draw_spells_then_lure_resolves_in_order() {
    let mut session = after_draw_lure_source_summoned();
    let source_id = unit_id(&session, "north-source");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    resolve_empty_start_turn(&mut session, &source_id);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let target_id = unit_id(&session, "south-minion");
    assert_eq!(state(&session)["phase"], "start-turn");
    let legal = session
        .legal_actions()
        .expect("mandatory start-turn draw-lure");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
                && action.descriptor["lureTargetInstanceId"] == target_id
        }),
        "a nearby enemy minion keeps the lure branch mandatory"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor["lureTargetInstanceId"] == target_id
            && descriptor["lureDestination"] == json!({ "cell": "C4", "region": "surface" })
    });
    assert_eq!(event_types(&receipt), ["spell-drawn", "unit-lured"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert_eq!(receipt.events[1].payload["targetInstanceId"], target_id);
    assert_eq!(
        receipt.events[1].payload["to"],
        json!({ "cell": "C4", "region": "surface" })
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("lured minion");
    assert_eq!(target["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0404_start_turn_draw_spells_then_lure_no_ops_when_no_enemy_is_nearby() {
    let mut session = after_draw_lure_source_summoned();
    let source_id = unit_id(&session, "north-source");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let target_id = unit_id(&session, "south-minion");
    assert_eq!(state(&session)["phase"], "start-turn");
    let legal = session.legal_actions().expect("empty start-turn draw-lure");
    assert!(legal.iter().all(|action| {
        action.descriptor["kind"] == "resolve-start-turn-trigger"
            && action.descriptor["sourceInstanceId"] == source_id
            && action.descriptor.get("lureTargetInstanceId").is_none()
    }));
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor.get("lureTargetInstanceId").is_none()
    });
    assert_eq!(event_types(&receipt), ["spell-drawn"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("unmoved south minion");
    assert_eq!(target["location"], "C1");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0960_start_turn_draw_sites_then_lure_resolves_in_order() {
    let mut session = after_draw_site_lure_source_summoned();
    let source_id = unit_id(&session, "north-source");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    resolve_empty_start_turn(&mut session, &source_id);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let target_id = unit_id(&session, "south-minion");
    assert_eq!(state(&session)["phase"], "start-turn");
    let legal = session
        .legal_actions()
        .expect("mandatory start-turn draw-site-lure");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
                && action.descriptor["lureTargetInstanceId"] == target_id
        }),
        "a nearby enemy minion keeps the lure branch mandatory"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor["lureTargetInstanceId"] == target_id
            && descriptor["lureDestination"] == json!({ "cell": "C4", "region": "surface" })
    });
    assert_eq!(event_types(&receipt), ["site-drawn", "unit-lured"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert_eq!(receipt.events[1].payload["targetInstanceId"], target_id);
    assert_eq!(
        receipt.events[1].payload["to"],
        json!({ "cell": "C4", "region": "surface" })
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("lured minion");
    assert_eq!(target["location"], "C4");
    assert_exact_replay(&session);
}

fn draw_lure_thin_library_manifest(seed: u32, north_spellbook: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-lure-thin-library" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-lure-thin-library-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-site": site(),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSpells": 1,
                "atStartOfControllerTurnLureNearbyEnemyMinion": true,
                "defense": 4,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
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
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_lure_thin_library_start_turn(seed: u32, north_spellbook: &[&str]) -> Session {
    let mut session = Session::new(&draw_lure_thin_library_manifest(seed, north_spellbook))
        .expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

#[test]
fn rule_catalog_0926_start_turn_draw_then_lure_draws_last_spell_when_lure_no_ops() {
    let mut session = draw_lure_thin_library_start_turn(
        926,
        &[
            "north-draw-card",
            "north-draw-card",
            "north-draw-card",
            "north-source",
        ],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let target_id = unit_id(&session, "south-minion");
    let drawn_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("only spell")["instanceId"]
        .clone();
    let library_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    assert_eq!(library_before, 1);
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let legal = session.legal_actions().expect("empty start-turn draw-lure");
    assert!(legal.iter().all(|action| {
        action.descriptor["kind"] == "resolve-start-turn-trigger"
            && action.descriptor["sourceInstanceId"] == source_id
            && action.descriptor.get("lureTargetInstanceId").is_none()
    }));
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor.get("lureTargetInstanceId").is_none()
    });
    assert_eq!(event_types(&receipt), ["spell-drawn"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_id)
    );
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("unmoved south minion");
    assert_eq!(target["location"], "C1");
    assert_exact_replay(&session);
}

fn draw_site_lure_thin_library_manifest(seed: u32, north_atlas: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-site-lure-thin-library" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-site-lure-thin-library-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnLureNearbyEnemyMinion": true,
                "defense": 4,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": north_atlas,
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 4],
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_site_lure_thin_library_start_turn(seed: u32, north_atlas: &[&str]) -> Session {
    let mut session = Session::new(&draw_site_lure_thin_library_manifest(seed, north_atlas))
        .expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

#[test]
fn rule_catalog_0968_start_turn_draw_sites_then_lure_draws_last_site_when_lure_no_ops() {
    let mut session = draw_site_lure_thin_library_start_turn(968, &["north-site"; 4]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let target_id = unit_id(&session, "south-minion");
    let drawn_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .first()
        .expect("only site")["instanceId"]
        .clone();
    let library_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    assert_eq!(library_before, 1);
    let hand_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand")
        .len();
    let legal = session.legal_actions().expect("empty start-turn draw-site-lure");
    assert!(legal.iter().all(|action| {
        action.descriptor["kind"] == "resolve-start-turn-trigger"
            && action.descriptor["sourceInstanceId"] == source_id
            && action.descriptor.get("lureTargetInstanceId").is_none()
    }));
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
            && descriptor.get("lureTargetInstanceId").is_none()
    });
    assert_eq!(event_types(&receipt), ["site-drawn"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .len(),
        hand_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_id)
    );
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("unmoved south minion");
    assert_eq!(target["location"], "C1");
    assert_exact_replay(&session);
}
