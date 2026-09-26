//! Direct proofs for Leap Attack resuming after ordered movement Deathrites
//! (RULE-CATALOG-0022, 0695, 1009, 2443–2448).
//!
//! 0599–0600 already cover stepping an ally to strike, and the immobile stay
//! edge. This slice keeps the 0022 leftover: stepping away from a nearby
//! power source settles two wounded Deathrite allies into deathrite-order,
//! then the pending leap strike resumes after either order.
//!
//! Supplemental 2443–2448 bind persistence, paid-noop-repeat, enemy-arrival,
//! multi-ally, far-enemy, and new-summon on that Deathrite-resume board.
//! Distinct from ordinary Leap Attack (0599–0600, 1963–1968), which never
//! defers the strike, and from Chain Magic hops (0696).

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, LegalAction, Receipt};
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

fn leap() -> Value {
    json!({
        "cardType": "magic",
        "leapAttackAlly": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn leap_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "leap-attack-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-leap-attack-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fragile-a": minion(json!({ "deathriteDrawSite": true })),
            "north-fragile-b": minion(json!({ "deathriteDrawSite": true })),
            "north-leap": leap(),
            "north-rain": rain(),
            "north-site": site(),
            "north-source": minion(json!({
                "attack": 3,
                "defense": 3,
                "otherNearbyAlliesPowerBonus": 1,
            })),
            "south-avatar": avatar(),
            "south-enemy": minion(json!({
                "attack": 1,
                "defense": 3,
                "summonToAnySite": true,
            })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-fragile-a",
                    "north-fragile-b",
                    "north-source",
                    "north-leap",
                    "north-rain",
                    "north-leap",
                    "north-rain",
                    "north-source",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct LeapDeathriteSetup {
    enemy_id: String,
    fragile_ids: [String; 2],
    leap_id: String,
    session: Session,
    source_id: String,
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

fn try_setup_leap_deathrite(encoded: &str) -> Option<LeapDeathriteSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let fragile_a = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-fragile-a"
            && descriptor["cell"] == "C4"
    })?;
    let fragile_b = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-fragile-b"
            && descriptor["cell"] == "C4"
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
    let source = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let enemy = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    let snapshot = state(&session);
    let leap_id = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == "north-leap")?["instanceId"]
        .as_str()?
        .to_owned();
    let source_id = source.0["cardInstanceId"].as_str()?.to_owned();
    let enemy_id = enemy.0["cardInstanceId"].as_str()?.to_owned();
    let fragile_ids = [
        fragile_a.0["cardInstanceId"].as_str()?.to_owned(),
        fragile_b.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    if !fragile_ids.iter().all(|instance_id| {
        realm_unit(&snapshot, instance_id).is_some_and(|unit| unit["damage"] == 1)
    }) {
        return None;
    }
    Some(LeapDeathriteSetup {
        enemy_id,
        fragile_ids,
        leap_id,
        session,
        source_id,
    })
}

fn seed_leap_deathrite(start: u32) -> String {
    (start..start + 2048)
        .map(leap_deathrite_manifest)
        .find(|candidate| try_setup_leap_deathrite(candidate).is_some())
        .expect("bounded seed with complete Leap Attack Deathrite setup")
}

fn cast_leap(session: &mut Session, leap_id: &str, source_id: &str) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == leap_id
            && descriptor["ally"]["instanceId"] == source_id
            && descriptor["allyDestination"]["cell"] == "C2"
    })
    .1
}

fn pending_leap(setup: &LeapDeathriteSetup) -> Value {
    json!({
        "ally": { "instanceId": setup.source_id, "kind": "minion", "seat": "north" },
        "cardId": "north-leap",
        "instanceId": setup.leap_id,
        "kind": "leap-attack",
        "owner": "north",
        "strikeLocation": { "cell": "C2", "region": "surface" },
    })
}

fn order_deathrite_actions(session: &Session) -> Vec<LegalAction> {
    session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .collect()
}

fn leap_kill_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "leap-attack-kill-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-leap-attack-kill-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-leap": leap(),
            "north-site": site(),
            "north-source": minion(json!({
                "attack": 3,
                "defense": 3,
                "otherNearbyAlliesPowerBonus": 1,
            })),
            "south-avatar": avatar(),
            "south-warded": minion(json!({
                "defense": 3,
                "summonToAnySite": true,
                "ward": true,
            })),
            "south-enemy-a": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-enemy-b": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-source",
                    "north-leap",
                    "north-source",
                    "north-leap",
                    "north-source",
                    "north-leap",
                    "north-source",
                    "north-leap",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-enemy-a",
                    "south-enemy-b",
                    "south-enemy-a",
                    "south-enemy-b",
                    "south-enemy-a",
                    "south-enemy-b",
                    "south-warded",
                    "south-warded",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct LeapKillDeathriteSetup {
    enemy_ids: [String; 2],
    leap_id: String,
    session: Session,
    source_id: String,
    survivor_id: String,
}

fn try_setup_leap_kill_deathrite(encoded: &str) -> Option<LeapKillDeathriteSetup> {
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
    let source = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let enemy_a = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy-a"
            && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let enemy_b = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy-b"
            && descriptor["cell"] == "C2"
    })?;
    let survivor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-warded"
            && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let snapshot = state(&session);
    let leap_id = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == "north-leap")?["instanceId"]
        .as_str()?
        .to_owned();
    Some(LeapKillDeathriteSetup {
        enemy_ids: [
            enemy_a.0["cardInstanceId"].as_str()?.to_owned(),
            enemy_b.0["cardInstanceId"].as_str()?.to_owned(),
        ],
        leap_id,
        session,
        source_id: source.0["cardInstanceId"].as_str()?.to_owned(),
        survivor_id: survivor.0["cardInstanceId"].as_str()?.to_owned(),
    })
}

fn seed_leap_kill_deathrite(start: u32) -> String {
    (start..start + 2048)
        .map(leap_kill_deathrite_manifest)
        .find(|candidate| try_setup_leap_kill_deathrite(candidate).is_some())
        .expect("bounded seed with complete Leap Attack kill Deathrite setup")
}

#[test]
fn rule_catalog_0695_leap_attack_resumes_its_strike_after_ordered_movement_deathrites() {
    let encoded = seed_leap_deathrite(695);
    let mut setup =
        try_setup_leap_deathrite(&encoded).expect("complete Leap Attack Deathrite setup");
    let interrupted = cast_leap(&mut setup.session, &setup.leap_id, &setup.source_id);
    assert_eq!(event_types(&interrupted), ["magic-cast", "unit-stepped"]);
    let pending = state(&setup.session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(
        pending["pendingDeathrites"]["continuation"],
        pending_leap(&setup)
    );
    assert_eq!(
        realm_unit(&pending, &setup.source_id).expect("moved Leap source")["location"],
        "C2"
    );
    assert!(realm_unit(&pending, &setup.enemy_id).is_some());
    assert!(
        setup
            .fragile_ids
            .iter()
            .all(|instance_id| realm_unit(&pending, instance_id).is_none())
    );

    let order = order_deathrite_actions(&setup.session)
        .into_iter()
        .next()
        .expect("engine-issued Deathrite order");
    let (_, ordered) = accept_where(&mut setup.session, |descriptor| {
        descriptor == &order.descriptor
    });
    let types = event_types(&ordered);
    assert_eq!(
        &types[..5],
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
            "minion-died",
        ]
    );
    assert_eq!(types.last(), Some(&"magic-resolved"));
    let strike_index = types
        .iter()
        .position(|event_type| *event_type == "strike-damage-allocated")
        .expect("resumed Leap strike");
    assert!(strike_index > 4);
    let completed = state(&setup.session);
    assert_eq!(completed["phase"], "main");
    assert!(completed["pendingDeathrites"].is_null());
    assert_eq!(
        realm_unit(&completed, &setup.source_id).expect("surviving Leap source")["location"],
        "C2"
    );
    assert!(realm_unit(&completed, &setup.enemy_id).is_none());
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_0799_leap_attack_deathrite_order_branches_match_after_checkpoint() {
    let encoded = seed_leap_deathrite(1695);
    let mut setup =
        try_setup_leap_deathrite(&encoded).expect("complete Leap Attack Deathrite setup");
    let _ = cast_leap(&mut setup.session, &setup.leap_id, &setup.source_id);
    let pending = state(&setup.session);
    let serialized = serialize_game_checkpoint(
        &create_game_checkpoint(&setup.session).expect("ordered Leap checkpoint"),
    )
    .expect("serialized ordered Leap checkpoint");
    let restored = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed ordered Leap checkpoint"),
    )
    .expect("resumed ordered Leap checkpoint");
    assert_eq!(state(&restored), pending);
    let orders = order_deathrite_actions(&restored);
    assert_eq!(orders.len(), 2);

    let mut branch_hashes = Vec::new();
    for order in orders {
        let mut branch = restored.clone();
        accept_where(&mut branch, |descriptor| descriptor == &order.descriptor);
        let completed = state(&branch);
        assert_eq!(completed["phase"], "main");
        assert!(realm_unit(&completed, &setup.enemy_id).is_none());
        assert_exact_replay(&branch);
        branch_hashes.push(branch.state_hash().expect("completed state hash"));
    }
    assert_eq!(branch_hashes[0], branch_hashes[1]);
}

fn assert_initial_leap_strike(result: &Receipt, enemy_ids: &[String], survivor_id: &str) {
    let mut struck_ids: Vec<_> = result
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| {
            event.payload["targetInstanceId"]
                .as_str()
                .expect("struck target")
        })
        .collect();
    struck_ids.sort_unstable();
    let mut expected_ids = vec![enemy_ids[0].as_str(), enemy_ids[1].as_str(), survivor_id];
    expected_ids.sort_unstable();
    assert_eq!(struck_ids, expected_ids, "exactly one strike per enemy");
    assert_eq!(
        event_types(result)
            .iter()
            .filter(|kind| **kind == "ward-broken")
            .count(),
        1
    );
}

#[test]
fn rule_catalog_1009_leap_attack_kill_resolves_deathrites_without_repeating_the_strike() {
    let encoded = seed_leap_kill_deathrite(1009);
    let mut setup =
        try_setup_leap_kill_deathrite(&encoded).expect("complete Leap Attack kill Deathrite setup");
    let interrupted = cast_leap(&mut setup.session, &setup.leap_id, &setup.source_id);
    let types = event_types(&interrupted);
    assert_eq!(&types[..2], ["magic-cast", "unit-stepped"]);
    assert_initial_leap_strike(&interrupted, &setup.enemy_ids, &setup.survivor_id);
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(paused["pendingDeathrites"]["continuation"].is_null());
    assert_eq!(
        paused["pendingDeathrites"]["deferredOutcomes"],
        json!([{
            "payload": {
                "cardId": "north-leap",
                "instanceId": setup.leap_id,
                "owner": "north",
            },
            "type": "magic-resolved",
        }])
    );
    let survivor = realm_unit(&paused, &setup.survivor_id).expect("warded enemy survives");
    assert_eq!(survivor["damage"], 0);
    assert_eq!(survivor["warded"], false);
    let checkpoint = create_game_checkpoint(&setup.session).expect("post-strike checkpoint");
    setup.session = resume_game_checkpoint(&checkpoint).expect("resume post-strike deaths");
    assert!(
        setup
            .enemy_ids
            .iter()
            .all(|instance_id| realm_unit(&paused, instance_id).is_none())
    );

    let order = order_deathrite_actions(&setup.session)
        .into_iter()
        .next()
        .expect("engine-issued Deathrite order");
    assert!(
        setup
            .enemy_ids
            .iter()
            .any(|instance_id| { order.descriptor["sourceInstanceId"] == *instance_id })
    );
    let (_, ordered) = accept_where(&mut setup.session, |descriptor| {
        descriptor == &order.descriptor
    });
    let types = event_types(&ordered);
    assert_eq!(
        &types[..4],
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
        ]
    );
    assert_eq!(types.get(4), Some(&"minion-died"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(
        !types.contains(&"strike-damage-allocated"),
        "the completed strike must not repeat"
    );
    let draw_index = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("Deathrite site draw");
    let resolved_index = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("resumed Leap resolution");
    assert!(draw_index < resolved_index);
    let drawn = ordered
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw payload");
    assert_eq!(drawn.payload["seat"], "south");
    assert!(
        setup
            .enemy_ids
            .iter()
            .any(|instance_id| drawn.payload["sourceInstanceId"] == *instance_id)
    );
    let finished = state(&setup.session);
    assert_eq!(finished["phase"], "main");
    assert!(finished["pendingDeathrites"].is_null());
    assert!(
        setup
            .enemy_ids
            .iter()
            .all(|instance_id| realm_unit(&finished, instance_id).is_none())
    );
    let survivor =
        realm_unit(&finished, &setup.survivor_id).expect("unwarded enemy still survives");
    assert_eq!(survivor["damage"], 0);
    assert_eq!(survivor["warded"], false);
    assert_exact_replay(&setup.session);
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    realm_unit(snapshot, instance_id).expect("expected realm unit")
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

fn leap_ally_targets(session: &Session) -> Vec<String> {
    let mut ids: Vec<String> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-leap"
                && action.descriptor["ally"]["kind"] == "minion"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn decline_attacks(session: &mut Session) {
    while try_accept_where(session, |descriptor| descriptor["kind"] == "decline-attack").is_some() {
    }
}

fn end_turn(session: &mut Session) {
    decline_attacks(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn try_draw_any(session: &mut Session) -> Option<(Value, Receipt)> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })
    .or_else(|| {
        try_accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        })
    })
}

fn pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    end_turn(session);
    try_draw_any(session)?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attacks(session);
    end_turn(session);
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn try_complete_leap_deathrite_strike(setup: &mut LeapDeathriteSetup) -> Option<Receipt> {
    let interrupted = try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == setup.leap_id
            && descriptor["ally"]["instanceId"] == setup.source_id
            && descriptor["allyDestination"]["cell"] == "C2"
    })?
    .1;
    if event_types(&interrupted) != ["magic-cast", "unit-stepped"] {
        return None;
    }
    let order = order_deathrite_actions(&setup.session).into_iter().next()?;
    Some(
        accept_where(&mut setup.session, |descriptor| {
            descriptor == &order.descriptor
        })
        .1,
    )
}

fn complete_leap_deathrite_strike(setup: &mut LeapDeathriteSetup) -> Receipt {
    try_complete_leap_deathrite_strike(setup).expect("Leap Deathrite resume strike")
}

fn cast_leap_to(session: &mut Session, ally_id: &str, cell: &str) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-leap"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["allyDestination"]["cell"] == cell
    })
    .1
}

fn leap_deathrite_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "leap-attack-deathrite-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-leap-attack-deathrite-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fragile-a": minion(json!({ "deathriteDrawSite": true })),
            "north-fragile-b": minion(json!({ "deathriteDrawSite": true })),
            "north-leap": leap(),
            "north-rain": rain(),
            "north-site": site(),
            "north-source": minion(json!({
                "attack": 3,
                "defense": 3,
                "otherNearbyAlliesPowerBonus": 1,
            })),
            "south-avatar": avatar(),
            "south-enemy": minion(json!({
                "attack": 1,
                "defense": 3,
                "summonToAnySite": true,
            })),
            "south-raider": minion(json!({
                "attack": 1,
                "defense": 5,
                "summonToAnySite": true,
            })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-fragile-a",
                    "north-fragile-b",
                    "north-source",
                    "north-leap",
                    "north-rain",
                    "north-leap",
                    "north-rain",
                    "north-source",
                    "north-leap",
                    "north-leap",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-raider",
                    "south-enemy",
                    "south-raider",
                    "south-enemy",
                    "south-raider",
                    "south-enemy",
                    "south-raider",
                    "south-enemy",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn try_setup_leap_deathrite_with_far(encoded: &str) -> Option<(LeapDeathriteSetup, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let fragile_a = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-fragile-a"
            && descriptor["cell"] == "C4"
    })?;
    let fragile_b = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-fragile-b"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let far = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let source = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let enemy = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    let snapshot = state(&session);
    let leap_id = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == "north-leap")?["instanceId"]
        .as_str()?
        .to_owned();
    let source_id = source.0["cardInstanceId"].as_str()?.to_owned();
    let enemy_id = enemy.0["cardInstanceId"].as_str()?.to_owned();
    let far_id = far.0["cardInstanceId"].as_str()?.to_owned();
    let fragile_ids = [
        fragile_a.0["cardInstanceId"].as_str()?.to_owned(),
        fragile_b.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    if !fragile_ids.iter().all(|instance_id| {
        realm_unit(&snapshot, instance_id).is_some_and(|unit| unit["damage"] == 1)
    }) {
        return None;
    }
    if realm_unit(&snapshot, &far_id)?.get("damage") != Some(&json!(1)) {
        return None;
    }
    Some((
        LeapDeathriteSetup {
            enemy_id,
            fragile_ids,
            leap_id,
            session,
            source_id,
        },
        far_id,
    ))
}

fn seed_leap_deathrite_with_far(start: u32) -> String {
    (start..start + 4096)
        .chain(695..695 + 4096)
        .map(leap_deathrite_supplemental_manifest)
        .find(|candidate| try_setup_leap_deathrite_with_far(candidate).is_some())
        .expect("bounded seed with Leap Deathrite far-enemy setup")
}

fn try_complete_setup(encoded: &str) -> Option<LeapDeathriteSetup> {
    let mut setup = try_setup_leap_deathrite(encoded)?;
    let ordered = try_complete_leap_deathrite_strike(&mut setup)?;
    event_types(&ordered)
        .contains(&"magic-resolved")
        .then_some(())?;
    realm_unit(&state(&setup.session), &setup.enemy_id)
        .is_none()
        .then_some(setup)
}

fn try_second_leap_after_deathrite(encoded: &str) -> Option<LeapDeathriteSetup> {
    let mut setup = try_complete_setup(encoded)?;
    pass_turn_to_north_spellbook(&mut setup.session)?;
    (leap_spells_in_hand(&state(&setup.session)) >= 1
        && leap_to_offered(&setup.session, &setup.source_id, "C2"))
    .then_some(setup)
}

fn seed_second_leap_after_deathrite(start: u32) -> String {
    (start..start + 4096)
        .chain(695..695 + 4096)
        .map(leap_deathrite_supplemental_manifest)
        .find(|candidate| try_second_leap_after_deathrite(candidate).is_some())
        .expect("bounded seed with two Leap casts after Deathrite resume")
}

fn leap_to_offered(session: &Session, ally_id: &str, cell: &str) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-leap"
                && action.descriptor["ally"]["instanceId"] == ally_id
                && action.descriptor["allyDestination"]["cell"] == cell
        })
    })
}

fn try_second_leap_enemy_arrival_prefix(encoded: &str) -> Option<(LeapDeathriteSetup, String)> {
    let mut setup = try_complete_setup(encoded)?;
    decline_attacks(&mut setup.session);
    try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    })?;
    try_draw_any(&mut setup.session)?;
    let site = try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "play-site"
            && (descriptor["cell"] == "B2" || descriptor["cell"] == "D2")
    })?;
    let dest = site.0["cell"].as_str()?.to_owned();
    let visitor = try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == dest
    })?;
    let visitor_id = visitor.0["cardInstanceId"].as_str()?.to_owned();
    decline_attacks(&mut setup.session);
    try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    })?;
    try_draw_any(&mut setup.session)?;
    (leap_spells_in_hand(&state(&setup.session)) >= 1
        && leap_to_offered(&setup.session, &setup.source_id, &dest))
    .then_some((setup, visitor_id))
}

fn seed_for_second_leap_enemy_arrival(start: u32) -> String {
    (start..start + 4096)
        .chain(695..695 + 4096)
        .map(leap_deathrite_supplemental_manifest)
        .find(|candidate| try_second_leap_enemy_arrival_prefix(candidate).is_some())
        .expect("bounded seed reaching second Leap Deathrite enemy-arrival setup")
}

fn try_second_leap_new_summon_prefix(encoded: &str) -> Option<(LeapDeathriteSetup, String)> {
    let mut setup = try_complete_setup(encoded)?;
    decline_attacks(&mut setup.session);
    try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    })?;
    try_draw_any(&mut setup.session)?;
    let visitor = try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C2"
    })?;
    let visitor_id = visitor.0["cardInstanceId"].as_str()?.to_owned();
    decline_attacks(&mut setup.session);
    try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    })?;
    try_draw_any(&mut setup.session)?;
    (leap_spells_in_hand(&state(&setup.session)) >= 1
        && leap_to_offered(&setup.session, &setup.source_id, "C2"))
    .then_some((setup, visitor_id))
}

fn seed_for_second_leap_new_summon(start: u32) -> String {
    (start..start + 4096)
        .chain(695..695 + 4096)
        .map(leap_deathrite_supplemental_manifest)
        .find(|candidate| try_second_leap_new_summon_prefix(candidate).is_some())
        .expect("bounded seed reaching second Leap Deathrite new-summon setup")
}

#[test]
fn rule_catalog_2443_leaped_ally_stays_at_its_destination_after_deathrite_resume_and_turns_pass() {
    let encoded = seed_leap_deathrite(2443);
    let mut setup =
        try_setup_leap_deathrite(&encoded).expect("complete Leap Attack Deathrite setup");
    let ordered = complete_leap_deathrite_strike(&mut setup);
    assert_eq!(event_types(&ordered).last(), Some(&"magic-resolved"));
    assert_eq!(
        unit(&state(&setup.session), &setup.source_id)["location"],
        "C2"
    );
    assert!(realm_unit(&state(&setup.session), &setup.enemy_id).is_none());
    pass_turn_to_north_spellbook(&mut setup.session).expect("round after Leap Deathrite resume");
    assert_eq!(
        unit(&state(&setup.session), &setup.source_id)["location"],
        "C2"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_2444_second_leap_after_deathrite_resume_is_a_paid_noop() {
    let encoded = seed_second_leap_after_deathrite(2444);
    let mut setup =
        try_second_leap_after_deathrite(&encoded).expect("two Leap casts after Deathrite resume");
    assert_eq!(
        unit(&state(&setup.session), &setup.source_id)["location"],
        "C2"
    );
    assert!(realm_unit(&state(&setup.session), &setup.enemy_id).is_none());
    assert!(leap_spells_in_hand(&state(&setup.session)) >= 1);
    let second = cast_leap_to(&mut setup.session, &setup.source_id, "C2");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"strike-damage-allocated"));
    assert_eq!(
        unit(&state(&setup.session), &setup.source_id)["location"],
        "C2"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_2445_second_leap_after_deathrite_resume_strikes_a_newly_arrived_enemy_after_enemy_site_placement()
 {
    let encoded = seed_for_second_leap_enemy_arrival(2445);
    let (mut setup, visitor_id) = try_second_leap_enemy_arrival_prefix(&encoded)
        .expect("second Leap Deathrite enemy-arrival prefix");
    let dest = unit(&state(&setup.session), &visitor_id)["location"]
        .as_str()
        .expect("visitor cell")
        .to_owned();
    let receipt = cast_leap_to(&mut setup.session, &setup.source_id, &dest);
    assert!(event_types(&receipt).contains(&"strike-damage-allocated"));
    assert!(realm_unit(&state(&setup.session), &visitor_id).is_none());
    assert_eq!(
        unit(&state(&setup.session), &setup.source_id)["location"],
        dest
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_2446_leap_offers_every_controlled_ally_on_the_deathrite_resume_board() {
    let encoded = seed_leap_deathrite(2446);
    let setup = try_setup_leap_deathrite(&encoded).expect("complete Leap Attack Deathrite setup");
    let offered = leap_ally_targets(&setup.session);
    assert!(offered.contains(&setup.source_id));
    for fragile_id in &setup.fragile_ids {
        assert!(offered.contains(fragile_id));
    }
    assert_eq!(offered.len(), 3);
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_2447_leap_deathrite_resume_leaves_a_far_enemy_unstruck() {
    let encoded = seed_leap_deathrite_with_far(2447);
    let (mut setup, far_id) =
        try_setup_leap_deathrite_with_far(&encoded).expect("Leap Deathrite far-enemy setup");
    assert_eq!(unit(&state(&setup.session), &far_id)["location"], "C1");
    let ordered = complete_leap_deathrite_strike(&mut setup);
    assert!(event_types(&ordered).contains(&"strike-damage-allocated"));
    assert!(realm_unit(&state(&setup.session), &setup.enemy_id).is_none());
    assert_eq!(unit(&state(&setup.session), &far_id)["location"], "C1");
    assert_eq!(unit(&state(&setup.session), &far_id)["damage"], 1);
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_2448_second_leap_after_deathrite_resume_strikes_a_newly_summoned_enemy() {
    let encoded = seed_for_second_leap_new_summon(2448);
    let (mut setup, visitor_id) = try_second_leap_new_summon_prefix(&encoded)
        .expect("second Leap Deathrite new-summon prefix");
    let receipt = cast_leap_to(&mut setup.session, &setup.source_id, "C2");
    assert!(event_types(&receipt).contains(&"strike-damage-allocated"));
    assert!(realm_unit(&state(&setup.session), &visitor_id).is_none());
    assert_eq!(
        unit(&state(&setup.session), &setup.source_id)["location"],
        "C2"
    );
    assert_exact_replay(&setup.session);
}
