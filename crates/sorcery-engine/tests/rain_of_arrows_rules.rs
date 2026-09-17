//! Direct proofs for damage-each-aboveground-minion Magic (RULE-CATALOG-0605–0606).
//!
//! Rain of Arrows simultaneously damages every aboveground minion. Ward absorbs
//! the damage without killing the minion.

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
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn rain() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn rain_manifest(seed: u32, ward: bool) -> String {
    let south_target = if ward {
        minion(json!({ "summonToAnySite": true, "ward": true }))
    } else {
        minion(json!({ "summonToAnySite": true }))
    };
    let fixture = if ward { "rain-ward" } else { "rain-lethal" };
    rain_manifest_with_target(seed, fixture, &south_target)
}

fn rain_deathrite_manifest(seed: u32) -> String {
    rain_manifest_with_target(
        seed,
        "rain-deathrite",
        &minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
    )
}

fn rain_manifest_with_target(seed: u32, fixture: &str, south_target: &Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rain": rain(),
            "north-site": site(),
            "north-victim": minion(json!({})),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-target": south_target,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-rain", "north-victim", "north-rain", "north-victim", "north-rain", "north-victim"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-target"; 6],
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

fn try_setup_rain(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let (victim_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-victim"
            && descriptor["cell"] == "C4"
    })?;
    let victim_id = victim_summon["cardInstanceId"].as_str()?.to_owned();
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
    let (target_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == "C1"
    })?;
    let target_id = target_summon["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some((session, victim_id, target_id))
}

fn setup_rain(encoded: &str) -> (Session, String, String) {
    try_setup_rain(encoded).expect("complete Rain of Arrows setup")
}

fn seed_with(ward: bool, start: u32) -> String {
    (start..start + 512)
        .map(|seed| rain_manifest(seed, ward))
        .find(|candidate| try_setup_rain(candidate).is_some())
        .expect("bounded seed with complete Rain of Arrows setup")
}

#[test]
fn rule_catalog_0605_rain_of_arrows_damages_every_aboveground_minion() {
    let encoded = seed_with(false, 605);
    let (mut session, victim_id, target_id) = setup_rain(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    assert!(event_types(&receipt).contains(&"damage-dealt"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    let after = state(&session);
    assert!(realm_unit(&after, &victim_id).is_none());
    assert!(realm_unit(&after, &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0606_rain_of_arrows_lets_ward_absorb_the_damage() {
    let encoded = seed_with(true, 606);
    let (mut session, victim_id, target_id) = setup_rain(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    assert!(event_types(&receipt).contains(&"ward-broken"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    let after = state(&session);
    assert!(realm_unit(&after, &victim_id).is_none());
    assert_eq!(
        realm_unit(&after, &target_id).expect("ward survivor")["warded"],
        false
    );
    assert_eq!(
        realm_unit(&after, &target_id).expect("ward survivor")["damage"],
        0
    );
    assert_exact_replay(&session);
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn deathrite_seed(start: u32) -> String {
    (start..start + 512)
        .map(rain_deathrite_manifest)
        .find(|candidate| try_setup_rain(candidate).is_some())
        .expect("bounded seed with complete Rain of Arrows Deathrite setup")
}

#[test]
fn rule_catalog_1008_area_damage_deathrite_draws_for_minion_controller_on_kill() {
    let encoded = deathrite_seed(1008);
    let (mut session, _victim_id, target_id) = setup_rain(&encoded);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let receipt = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })
    .1;
    let types = event_types(&receipt);
    assert!(types.starts_with(&["magic-cast", "magic-damage-allocated"][..]));
    assert!(types.contains(&"damage-dealt"));
    let drawn_index = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("Deathrite site draw");
    let first_death_index = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion death");
    assert!(
        drawn_index < first_death_index,
        "Deathrite draw must resolve before corpses settle: {types:?}"
    );
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw event");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], target_id);
    assert!(types.contains(&"magic-resolved"));
    let finished = state(&session);
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}
