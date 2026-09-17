//! Direct proofs for Freeze disable Magic (RULE-CATALOG-0661–0662).
//!
//! Freeze (`disableTargetNearbyMinionUntilNextTurn`) disables a nearby minion
//! until the caster's next Start Phase. Enemy Ward absorbs the cast: Ward
//! breaks and the minion is not Disabled.

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

fn minion(ward: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if ward {
        value["ward"] = json!(true);
    }
    value
}

fn freeze() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn freeze_manifest(seed: u32, ward: bool) -> String {
    let fixture = if ward {
        "disable-magic-ward"
    } else {
        "disable-magic"
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
            "north-freeze": freeze(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(ward),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-freeze"; 6],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid Freeze session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("realm unit")
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

fn stage_south_minion_at_c4(session: &mut Session) -> String {
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
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("South minion identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    minion_id
}

#[test]
fn rule_catalog_0661_freeze_disables_a_nearby_minion_until_caster_next_start_phase() {
    let encoded = freeze_manifest(661, false);
    let mut session = opening_main(&encoded);
    let minion_id = stage_south_minion_at_c4(&mut session);
    let before = state(&session);
    assert!(realm_unit(&before, &minion_id)["disableEffects"].is_null());
    assert_eq!(realm_unit(&before, &minion_id)["warded"], false);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == minion_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    let source_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("Freeze source identity");
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "expiresAtSeat": "north",
            "instanceId": minion_id,
            "seat": "south",
            "sourceInstanceId": source_id,
            "stealthRemoved": false,
            "wardRemoved": false,
        })
    );
    assert_eq!(
        realm_unit(&state(&session), &minion_id)["disableEffects"],
        json!([{
            "expiresAtSeat": "north",
            "sourceInstanceId": source_id,
        }])
    );

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert_eq!(
        realm_unit(&state(&session), &minion_id)["disableEffects"][0]["sourceInstanceId"],
        source_id
    );

    let (_, expired) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_types(&expired),
        ["turn-ended", "minion-disable-expired", "turn-started"]
    );
    assert_eq!(expired.events[1].payload["instanceId"], minion_id);
    assert!(realm_unit(&state(&session), &minion_id)["disableEffects"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0662_freeze_is_absorbed_by_enemy_ward() {
    let encoded = freeze_manifest(662, true);
    let mut session = opening_main(&encoded);
    let minion_id = stage_south_minion_at_c4(&mut session);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &minion_id)["warded"], true);
    assert!(realm_unit(&before, &minion_id)["disableEffects"].is_null());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == minion_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-disabled")
    );

    let after = state(&session);
    assert_eq!(realm_unit(&after, &minion_id)["warded"], false);
    assert!(realm_unit(&after, &minion_id)["disableEffects"].is_null());
    assert_exact_replay(&session);
}
