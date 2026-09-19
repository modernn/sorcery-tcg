//! Direct proofs for nearby-control Magic range (RULE-CATALOG-0146,
//! RULE-CATALOG-0683–0684, RULE-CATALOG-1042, RULE-CATALOG-2383–2388).
//!
//! After the caster Avatar steps from C4 to C3, `gainControlOfTargetNearbyMinion`
//! offers the adjacent C2 minion and not the two-step C1 minion. Stealing the
//! adjacent minion transfers its Deathrite; the two-step minion stays with its
//! controller. Distinct from 0659–0660, which prove same-cell nearby and a
//! far-only board three steps from an unmoved caster. While Deathrites wait for
//! ordering, Mesmerism Magic stays withheld until the chain drains.
//! Supplemental 2383–2388 keep persistence, paid-noop-repeat, enemy-arrival,
//! multi-minion, far-minion, and new-summon proofs on that same moved-caster
//! range board.

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

fn deathrite_mesmerism_manifest(seed: u32) -> String {
    let fixture = "mesmerism-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-mesmerism": mesmerism(),
            "north-rain": rain_spell(),
            "north-site": site(),
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
                    "north-mesmerism",
                    "north-rain",
                    "north-rain",
                    "north-mesmerism",
                    "north-rain",
                    "north-mesmerism",
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

fn north_has_mesmerism_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-mesmerism", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteMesmerismSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_nearby_visitor(
    encoded: &str,
) -> Option<PendingDeathriteMesmerismSetup> {
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
    if !north_has_mesmerism_and_rain(&state(&session)) {
        return None;
    }
    if !mesmerism_targets(&session).contains(&visitor_id) {
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
    Some(PendingDeathriteMesmerismSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_mesmerism_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_mesmerism_manifest)
        .find(|candidate| try_pending_deathrite_with_nearby_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Mesmerism Magic in hand")
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

#[test]
fn rule_catalog_1042_mesmerism_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_mesmerism_seed_with(1042);
    let mut setup = try_pending_deathrite_with_nearby_visitor(&encoded)
        .expect("complete Mesmerism Deathrite withheld setup");
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
    assert!(realm_unit(&paused, &visitor_id).is_some());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(mesmerism_targets(session).is_empty());

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
    assert!(realm_unit(&resumed, &visitor_id).is_some());
    assert_eq!(mesmerism_targets(session), [visitor_id.as_str()]);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-control-changed", "magic-resolved"]
    );
    assert_eq!(
        realm_unit(&state(session), &visitor_id).expect("stolen visitor")["controller"],
        "north"
    );
    assert_exact_replay(session);
}

fn range_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "mesmerism-range-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-mesmerism-range-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-mesmerism": mesmerism(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-mesmerism"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_hand_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
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

fn supplemental_seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(683..683 + 2048)
        .map(range_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-mesmerism")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Mesmerism and required South minions")
}

fn mesmerism_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-mesmerism")
                .count()
        })
        .unwrap_or_default()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .ok()
        .is_some_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn decline_attack_if_needed(session: &mut Session) {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

fn end_turn_if_offered(session: &mut Session) {
    decline_attack_if_needed(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn try_summon_south_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn cast_mesmerism_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn try_move_north_avatar_to_c3(session: &mut Session) -> Option<()> {
    let avatar_id = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == avatar_id.as_str()
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    })?;
    let _ = try_accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    Some(())
}

/// Caster at C3, adjacent minions at C2, two-step minion at C1.
fn try_range_opening(session: &mut Session, near_count: usize) -> Option<(String, Vec<String>)> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let far_id = try_summon_south_at(session, "C1")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let near_ids: Vec<String> = (0..near_count)
        .map(|_| try_summon_south_at(session, "C2"))
        .collect::<Option<_>>()?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_move_north_avatar_to_c3(session)?;
    Some((far_id, near_ids))
}

fn range_opening(session: &mut Session, near_count: usize) -> (String, Vec<String>) {
    try_range_opening(session, near_count).expect("moved-caster nearby-range opening")
}

fn try_second_steal_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let (_, near_ids) = try_range_opening(&mut session, 1)?;
    let first = cast_mesmerism_on(&mut session, &near_ids[0]);
    if !event_types(&first).contains(&"minion-control-changed") {
        return None;
    }
    if mesmerism_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && (descriptor["cell"] == "D2" || descriptor["cell"] == "B2")
    })?;
    let minion_id = try_summon_south_at(&mut session, "C4")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    mesmerism_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_steal_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(683..683 + 8192)
        .find_map(|seed| {
            let encoded = range_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 3
            {
                return None;
            }
            try_second_steal_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second nearby-range enemy-arrival setup")
}

fn try_second_steal_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let (_, near_ids) = try_range_opening(&mut session, 1)?;
    let first = cast_mesmerism_on(&mut session, &near_ids[0]);
    if !event_types(&first).contains(&"minion-control-changed") {
        return None;
    }
    if unit(&state(&session), &near_ids[0])["controller"] != "north" {
        return None;
    }
    if mesmerism_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = try_summon_south_at(&mut session, "C2")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    mesmerism_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_steal_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(683..683 + 8192)
        .find_map(|seed| {
            let encoded = range_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_steal_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second nearby-range new-summon setup")
}

#[test]
fn rule_catalog_2383_stolen_adjacent_minion_stays_with_the_new_controller_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2383, 2);
    let mut session = opening_main(&encoded);
    let (far_id, near_ids) = range_opening(&mut session, 1);
    let near_id = &near_ids[0];
    assert_range_targets(&session, &far_id, near_id);
    let receipt = cast_mesmerism_on(&mut session, near_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), near_id)["controller"], "north");
    assert_eq!(unit(&state(&session), near_id)["owner"], "south");
    assert_eq!(unit(&state(&session), near_id)["location"], "C2");
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), near_id)["controller"], "north");
    assert_eq!(unit(&state(&session), near_id)["owner"], "south");
    assert_eq!(unit(&state(&session), near_id)["location"], "C2");
    assert_eq!(unit(&state(&session), &far_id)["controller"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2384_second_mesmerism_on_the_stolen_adjacent_minion_is_a_paid_noop() {
    let encoded = (2384..2384 + 8192)
        .chain(683..683 + 8192)
        .find_map(|seed| {
            let candidate = range_supplemental_manifest(seed);
            let mut session = opening_main(&candidate);
            let (_, near_ids) = try_range_opening(&mut session, 1)?;
            let first = cast_mesmerism_on(&mut session, &near_ids[0]);
            if !event_types(&first).contains(&"minion-control-changed") {
                return None;
            }
            if unit(&state(&session), &near_ids[0])["controller"] != "north" {
                return None;
            }
            (mesmerism_spells_in_hand(&state(&session)) >= 1
                && mesmerism_targets(&session).contains(&near_ids[0]))
            .then_some(candidate)
        })
        .expect("bounded seed with two Mesmerism casts after stealing the adjacent minion");
    let mut session = opening_main(&encoded);
    let (far_id, near_ids) = range_opening(&mut session, 1);
    let near_id = &near_ids[0];
    let first = cast_mesmerism_on(&mut session, near_id);
    assert!(event_types(&first).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), near_id)["controller"], "north");
    assert!(mesmerism_spells_in_hand(&state(&session)) >= 1);
    assert!(mesmerism_targets(&session).contains(near_id));
    let second = cast_mesmerism_on(&mut session, near_id);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), near_id)["controller"], "north");
    assert_eq!(unit(&state(&session), near_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &far_id)["controller"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2385_second_mesmerism_steals_a_newly_arrived_adjacent_minion_after_enemy_site_placement()
 {
    let encoded = seed_for_second_steal_enemy_arrival(2385);
    let (mut session, minion_id) = try_second_steal_enemy_arrival_prefix(&encoded)
        .expect("second nearby-range enemy-arrival prefix");
    let receipt = cast_mesmerism_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2386_mesmerism_offers_every_adjacent_minion_after_the_caster_moves() {
    let encoded = supplemental_seed_with_start(2386, 3);
    let mut session = opening_main(&encoded);
    let (far_id, near_ids) = range_opening(&mut session, 2);
    let offered = mesmerism_targets(&session);
    for minion_id in &near_ids {
        assert!(offered.contains(minion_id));
    }
    assert!(!offered.contains(&far_id));
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2387_mesmerism_leaves_a_two_step_minion_untouched() {
    let encoded = supplemental_seed_with_start(2387, 2);
    let mut session = opening_main(&encoded);
    let (far_id, near_ids) = range_opening(&mut session, 1);
    let near_id = &near_ids[0];
    assert_range_targets(&session, &far_id, near_id);
    let receipt = cast_mesmerism_on(&mut session, near_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), near_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &far_id)["controller"], "south");
    assert_eq!(unit(&state(&session), &far_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &far_id)["location"], "C1");
    assert!(!mesmerism_targets(&session).contains(&far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2388_second_mesmerism_steals_a_newly_summoned_adjacent_minion() {
    let encoded = seed_for_second_steal_new_summon(2388);
    let (mut session, minion_id) = try_second_steal_new_summon_prefix(&encoded)
        .expect("second nearby-range new-summon prefix");
    let receipt = cast_mesmerism_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C2");
    assert_exact_replay(&session);
}
