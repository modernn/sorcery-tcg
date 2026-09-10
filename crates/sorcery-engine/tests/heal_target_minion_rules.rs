//! Direct proofs for heal-target-minion Magic (RULE-CATALOG-0298–0299).
//!
//! Official Magic can remove damage from a living minion without targeting
//! Avatars or breaking Ward. Healing a healthy minion is a paid no-op. End
//! Phase clears leftover damage, so the wound and the heal must share a turn.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn magic(effect: (&str, Value)) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("Magic facts")
        .insert(effect.0.to_owned(), effect.1);
    value
}

fn visitor(ward: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if ward {
        value["ward"] = json!(true);
    }
    value
}

fn manifest(seed: u32, ward: bool) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "heal-target-minion" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-heal-target-minion-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-damage": magic(("damageTargetUnit", json!(1))),
            "north-heal": magic(("healTargetMinion", json!(1))),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-visitor": visitor(ward),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-damage", "north-damage", "north-damage", "north-heal", "north-heal", "north-heal"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-visitor"; 6],
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

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn realm_unit<'a>(value: &'a Value, instance_id: &str) -> Option<&'a Value> {
    value["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn heal_minion_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("heal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-heal"
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

fn after_visitor_on_c4(seed: u32, ward: bool) -> (Session, String) {
    let mut session =
        Session::new(&manifest(seed, ward)).expect("valid heal-target-minion session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    let visitor_id = summoned["cardInstanceId"]
        .as_str()
        .expect("visitor identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (session, visitor_id)
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

fn assert_checkpoint(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("heal checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized heal");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed heal");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed heal session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0298_heal_target_minion_clears_same_turn_damage_without_targeting_avatars() {
    let (mut session, visitor_id) = after_visitor_on_c4(298, false);
    let before = state(&session);
    let visitor = realm_unit(&before, &visitor_id).expect("wounded-to-be visitor");
    assert_eq!(visitor["damage"], 0);
    assert_eq!(visitor["warded"], false);
    let north_avatar = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert_eq!(
        heal_minion_targets(&session),
        [("minion".to_owned(), visitor_id.clone())]
    );
    assert!(
        !heal_minion_targets(&session)
            .iter()
            .any(|(_, instance_id)| *instance_id == north_avatar || *instance_id == south_avatar)
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-damage"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    let wounded = state(&session);
    let visitor = realm_unit(&wounded, &visitor_id).expect("wounded visitor");
    assert_eq!(visitor["damage"], 1);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-healed", "magic-resolved"]
    );
    let healed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-healed")
        .expect("heal event");
    assert_eq!(healed.payload["amount"], 1);
    assert_eq!(healed.payload["attemptedAmount"], 1);
    assert_eq!(healed.payload["damage"], 0);
    assert_eq!(healed.payload["instanceId"], visitor_id);
    assert_eq!(healed.payload["seat"], "south");
    assert_eq!(
        healed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken"
                || event.event_type == "minion-died"
                || event.event_type == "avatar-healed")
    );

    let after = state(&session);
    let visitor = realm_unit(&after, &visitor_id).expect("healed visitor");
    assert_eq!(visitor["damage"], 0);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["decisionSeat"], "north");
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    assert_checkpoint(&session);
}

#[test]
fn rule_catalog_0299_heal_target_minion_is_a_paid_noop_on_an_undamaged_minion() {
    let (mut session, visitor_id) = after_visitor_on_c4(299, true);
    let before = state(&session);
    let visitor = realm_unit(&before, &visitor_id).expect("healthy visitor");
    assert_eq!(visitor["damage"], 0);
    assert_eq!(visitor["warded"], true);
    assert_eq!(
        heal_minion_targets(&session),
        [("minion".to_owned(), visitor_id.clone())]
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-healed"
                || event.event_type == "ward-broken"
                || event.event_type == "minion-died")
    );

    let after = state(&session);
    let visitor = realm_unit(&after, &visitor_id).expect("still-healthy visitor");
    assert_eq!(visitor["damage"], 0);
    assert_eq!(visitor["warded"], true);
    assert_exact_replay(&session);
    assert_checkpoint(&session);
}
