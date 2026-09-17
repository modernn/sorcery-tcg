//! Direct proofs for return-target-minion-to-owner-hand Magic (RULE-CATALOG-0621–0622).
//!
//! Bounce Magic returns a healthy same-region minion to its owner's hand.
//! Enemy Ward absorbs the return without moving the minion.

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
