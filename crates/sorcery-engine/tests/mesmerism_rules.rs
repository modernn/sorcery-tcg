//! Direct proofs for nearby-control Magic range (RULE-CATALOG-0146,
//! RULE-CATALOG-0683–0684).
//!
//! After the caster Avatar steps from C4 to C3, `gainControlOfTargetNearbyMinion`
//! offers the adjacent C2 minion and not the two-step C1 minion. Stealing the
//! adjacent minion transfers its Deathrite; the two-step minion stays with its
//! controller. Distinct from 0659–0660, which prove same-cell nearby and a
//! far-only board three steps from an unmoved caster.

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

fn deathrite(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
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

fn mesmerism() -> Value {
    json!({
        "cardType": "magic",
        "gainControlOfTargetNearbyMinion": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn mesmerism_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "mesmerism-range" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-mesmerism-range-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lash": lash(),
            "north-mesmerism": mesmerism(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": deathrite(json!({})),
            "south-site": site(),
            "south-target": deathrite(json!({})),
            "south-warded": json!({
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "summonToAnySite": true,
                "ward": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-mesmerism",
                    "north-lash",
                    "north-mesmerism",
                    "north-lash",
                    "north-mesmerism",
                    "north-lash",
                    "north-lash",
                    "north-lash",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-far",
                    "south-target",
                    "south-warded",
                    "south-far",
                    "south-target",
                    "south-warded",
                    "south-far",
                    "south-target",
                ],
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
    let mut session = Session::new(encoded).expect("valid Mesmerism session");
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

fn hand_has(snapshot: &Value, seat: &str, card_id: &str) -> bool {
    snapshot["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("spellbook hand")
        .iter()
        .any(|card| card["cardId"] == card_id)
}

fn seed_with(start: u32) -> String {
    (start..start + 512)
        .map(mesmerism_manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Mesmerism seed candidate");
            let opening = state(&preview);
            ["north-lash", "north-mesmerism"]
                .into_iter()
                .all(|card_id| hand_has(&opening, "north", card_id))
                && ["south-far", "south-target"]
                    .into_iter()
                    .all(|card_id| hand_has(&opening, "south", card_id))
        })
        .expect("bounded seed with Mesmerism, Lash, and both South minions in hand")
}

/// Walks into North Avatar at C3 with South minions at C1 (two steps) and C2 (adjacent).
fn mesmerism_opening(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (near, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == "C2"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let avatar_id = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == avatar_id.as_str()
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    (
        far["cardInstanceId"]
            .as_str()
            .expect("far minion identity")
            .to_owned(),
        near["cardInstanceId"]
            .as_str()
            .expect("nearby minion identity")
            .to_owned(),
    )
}

fn mesmerism_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Mesmerism actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-mesmerism"
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

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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

fn assert_range_targets(session: &Session, far_id: &str, near_id: &str) {
    let targets = mesmerism_targets(session);
    assert_eq!(targets, [near_id]);
    assert!(!targets.iter().any(|id| id == far_id));
}

fn lash_kills(session: &mut Session, instance_id: &str) -> Receipt {
    let (lash, killed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == instance_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], instance_id);
    killed
}

#[test]
fn rule_catalog_0683_mesmerism_steals_the_adjacent_minion_and_its_deathrite_after_the_caster_moves()
{
    let encoded = seed_with(683);
    let mut session = opening_main(&encoded);
    let (far_id, near_id) = mesmerism_opening(&mut session);
    assert_range_targets(&session, &far_id, &near_id);

    let (_, stolen) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == near_id
    });
    assert_eq!(
        event_types(&stolen),
        ["magic-cast", "minion-control-changed", "magic-resolved"]
    );
    let changed = stolen
        .events
        .iter()
        .find(|event| event.event_type == "minion-control-changed")
        .expect("control change");
    assert_eq!(changed.payload["fromSeat"], "south");
    assert_eq!(changed.payload["seat"], "north");
    let stolen_state = state(&session);
    let transferred = realm_unit(&stolen_state, &near_id).expect("stolen minion");
    assert_eq!(transferred["controller"], "north");
    assert_eq!(transferred["owner"], "south");
    let far = realm_unit(&stolen_state, &far_id).expect("untouched far minion");
    assert_eq!(far["controller"], "south");

    let north_atlas = atlas_len(&stolen_state, "north");
    let south_atlas = atlas_len(&stolen_state, "south");
    let killed = lash_kills(&mut session, &near_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &near_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &near_id));
    assert!(!cemetery_has(&finished, "north", &near_id));
    assert_eq!(
        realm_unit(&finished, &far_id).expect("surviving far minion")["controller"],
        "south"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0684_mesmerism_does_not_offer_a_two_step_minion_while_an_adjacent_one_remains() {
    let encoded = seed_with(684);
    let mut session = opening_main(&encoded);
    let (far_id, near_id) = mesmerism_opening(&mut session);
    assert_range_targets(&session, &far_id, &near_id);

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let killed = lash_kills(&mut session, &far_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &far_id).is_none());
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert!(cemetery_has(&finished, "south", &far_id));
    let nearby = realm_unit(&finished, &near_id).expect("unstolen nearby minion");
    assert_eq!(nearby["controller"], "south");
    assert_eq!(nearby["owner"], "south");
    assert_exact_replay(&session);
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
    let Ok(StepResult::Accepted(receipt)) = session.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    }) else {
        return None;
    };
    Some((descriptor, receipt))
}

fn try_warded_opening(encoded: &str) -> Option<(Session, String)> {
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let (warded, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-warded"
            && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == avatar_id.as_str()
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    })?;
    Some((session, warded["cardInstanceId"].as_str()?.to_owned()))
}

#[test]
fn rule_catalog_0910_mesmerism_ward_absorbs_control_without_transferring_controller() {
    let (mut session, warded_id) = (910..910 + 512)
        .map(mesmerism_manifest)
        .find_map(|candidate| try_warded_opening(&candidate))
        .expect("bounded seed with completable Warded Mesmerism setup");
    assert_eq!(mesmerism_targets(&session), [warded_id.as_str()]);
    let snapshot = state(&session);
    let before = realm_unit(&snapshot, &warded_id).expect("warded minion");
    assert_eq!(before["controller"], "south");
    assert_eq!(before["warded"], true);

    let (_, absorbed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == warded_id
    });
    assert_eq!(
        event_types(&absorbed),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    assert!(
        !absorbed
            .events
            .iter()
            .any(|event| event.event_type == "minion-control-changed")
    );
    let finished = state(&session);
    let after = realm_unit(&finished, &warded_id).expect("surviving minion");
    assert_eq!(after["controller"], "south");
    assert_eq!(after["owner"], "south");
    assert_eq!(after["warded"], false);
    assert_exact_replay(&session);
}
