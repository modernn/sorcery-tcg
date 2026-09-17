//! Direct proofs for grant-Stealth-to-target-minion Magic (RULE-CATALOG-0617–0618).
//!
//! Grant-Stealth Magic hides an enemy minion from later targeting. An
//! already-stealthed minion is not offered as a legal target.

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

fn charger(stealth: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if stealth {
        value["stealth"] = json!(true);
    }
    value
}

fn stealth_spell() -> Value {
    json!({
        "cardType": "magic",
        "grantStealthToTargetMinion": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn grant_stealth_manifest(seed: u32, printed_stealth: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-stealth-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-stealth-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-stealth": stealth_spell(),
            "south-avatar": avatar(),
            "south-charger": charger(printed_stealth),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-stealth"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-charger"; 6],
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
    let mut session = Session::new(encoded).expect("valid grant-Stealth session");
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

fn grant_stealth_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("grant-Stealth actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-stealth"
        })
        .filter_map(|action| {
            let target = action.descriptor.get("target")?;
            Some((
                target["kind"].as_str()?.to_owned(),
                target["instanceId"].as_str()?.to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn stage_south_charger(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-charger"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let charger_id = summoned["cardInstanceId"]
        .as_str()
        .expect("South charger identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    charger_id
}

#[test]
fn rule_catalog_0617_grant_stealth_magic_hides_an_enemy_minion_from_later_targeting() {
    let encoded = grant_stealth_manifest(617, false);
    let mut session = opening_main(&encoded);
    let charger_id = stage_south_charger(&mut session);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &charger_id)["stealthed"], false);
    assert_eq!(
        grant_stealth_targets(&session),
        [("minion".to_owned(), charger_id.clone())]
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-stealth"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == charger_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-stealthed", "magic-resolved"]
    );
    let granted = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-stealthed")
        .expect("Stealth grant");
    assert_eq!(granted.payload["instanceId"], charger_id);
    assert_eq!(granted.payload["seat"], "south");
    assert_eq!(
        granted.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );

    let after = state(&session);
    assert_eq!(realm_unit(&after, &charger_id)["stealthed"], true);
    assert_eq!(grant_stealth_targets(&session), []);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0618_grant_stealth_magic_cannot_target_an_enemy_already_stealthed() {
    let encoded = grant_stealth_manifest(618, true);
    let mut session = opening_main(&encoded);
    let charger_id = stage_south_charger(&mut session);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &charger_id)["stealthed"], true);
    assert_eq!(grant_stealth_targets(&session), []);
    assert!(
        !session
            .legal_actions()
            .expect("post-stealth actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-stealth")
    );

    let after = state(&session);
    assert_eq!(realm_unit(&after, &charger_id)["stealthed"], true);
    assert_exact_replay(&session);
}
