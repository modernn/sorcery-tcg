//! Direct proofs for fight-ally-with-adjacent-enemy Magic (RULE-CATALOG-0603–0604).
//!
//! Duel makes a chosen ally fight a targeted adjacent enemy through the shared
//! fight pipeline. Ward on the target breaks without entering combat.

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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
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

fn duel() -> Value {
    json!({
        "cardType": "magic",
        "fightAllyWithAdjacentEnemy": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn duel_manifest(seed: u32, ward: bool) -> String {
    let south_enemy = if ward {
        minion(json!({ "attack": 2, "defense": 3, "ward": true }))
    } else {
        minion(json!({ "attack": 2, "defense": 3 }))
    };
    let fixture = if ward { "duel-ward" } else { "duel-fight" };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": minion(json!({ "attack": 3, "defense": 4 })),
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "spellcaster": true })),
            "north-duel": duel(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": south_enemy,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-duel", "north-ally", "north-caster", "north-duel", "north-ally", "north-caster"],
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

fn try_setup_duel(encoded: &str) -> Option<(Session, String, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let (ally_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = ally_summon["cardInstanceId"].as_str()?.to_owned();
    let (caster_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    })?;
    let caster_id = caster_summon["cardInstanceId"].as_str()?.to_owned();
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
    Some((session, ally_id, caster_id, enemy_id))
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

fn setup_duel(encoded: &str) -> (Session, String, String, String) {
    try_setup_duel(encoded).expect("complete Duel setup")
}

fn seed_with(ward: bool, start: u32) -> String {
    (start..start + 512)
        .map(|seed| duel_manifest(seed, ward))
        .find(|candidate| try_setup_duel(candidate).is_some())
        .expect("bounded seed with complete Duel setup")
}

#[test]
fn rule_catalog_0603_duel_magic_fights_an_adjacent_enemy_through_the_shared_pipeline() {
    let encoded = seed_with(false, 603);
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert!(event_types(&receipt).contains(&"fight-started"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    assert!(realm_unit(&state(&session), &enemy_id).is_none());
    assert!(realm_unit(&state(&session), &ally_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0604_duel_magic_breaks_ward_without_entering_combat() {
    let encoded = seed_with(true, 604);
    let (mut session, ally_id, caster_id, enemy_id) = setup_duel(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-duel"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let after = state(&session);
    assert_eq!(
        realm_unit(&after, &enemy_id).expect("ward survivor")["warded"],
        false
    );
    assert_eq!(
        realm_unit(&after, &ally_id).expect("unharmed ally")["damage"],
        0
    );
    assert_exact_replay(&session);
}
