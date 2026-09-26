//! Direct proofs for kill-target-minion Magic (RULE-CATALOG-0619–0620, 1014,
//! 1085, RULE-CATALOG-2063–2068).
//!
//! Kill-target-minion Magic destroys a healthy same-region minion. Enemy Ward
//! absorbs the kill without destroying the minion. Deathrite minions draw a
//! site for their controller before magic-resolved. While Deathrites wait for
//! ordering, kill-target Magic stays withheld until the chain drains.

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
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if ward {
        value["ward"] = json!(true);
    }
    value
}

fn deathrite_minion() -> Value {
    let mut value = minion(false);
    value["deathriteDrawSite"] = json!(true);
    value
}

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_order_minion() -> Value {
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

fn kill_spell() -> Value {
    json!({
        "cardType": "magic",
        "killTargetMinion": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn kill_target_manifest(seed: u32, ward: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "kill-target-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-kill-target-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-kill": kill_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(ward),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-kill"; 6],
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

fn kill_target_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "kill-target-deathrite-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-kill-target-deathrite-draw-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-kill": kill_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-kill"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid kill-target session");
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

fn kill_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("kill-minion actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-kill"
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

fn deathrite_kill_manifest(seed: u32) -> String {
    let fixture = "kill-target-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-kill": kill_spell(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_order_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-kill",
                    "north-rain",
                    "north-rain",
                    "north-kill",
                    "north-rain",
                    "north-kill",
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

fn north_has_kill_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-kill", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteKillSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteKillSetup> {
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
    if !north_has_kill_and_rain(&state(&session)) {
        return None;
    }
    if kill_targets(&session).is_empty() {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteKillSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_kill_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_kill_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with kill-target Magic in hand")
}

#[test]
fn rule_catalog_0619_kill_target_minion_destroys_a_healthy_minion_and_excludes_avatars() {
    let encoded = kill_target_manifest(619, false);
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
    let targets = kill_targets(&session);
    assert_eq!(targets, [enemy_id.as_str()]);
    assert!(!targets.contains(&north_avatar));
    assert!(!targets.contains(&south_avatar));
    assert_eq!(
        realm_unit(&before, &enemy_id).expect("healthy enemy")["damage"],
        0
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-kill"
    });
    assert_eq!(descriptor["target"]["instanceId"], enemy_id);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    let killed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-killed")
        .expect("unconditional kill event");
    assert_eq!(killed.payload["cardId"], "south-minion");
    assert_eq!(killed.payload["owner"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &enemy_id).is_none());
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == enemy_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0620_kill_target_minion_ward_absorbs_the_kill() {
    let encoded = kill_target_manifest(620, true);
    let mut session = opening_main(&encoded);
    let enemy_id = stage_south_minion(&mut session);
    assert_eq!(kill_targets(&session), [enemy_id.as_str()]);
    assert_eq!(
        realm_unit(&state(&session), &enemy_id).expect("warded enemy")["warded"],
        true
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == enemy_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let broken = receipt
        .events
        .iter()
        .find(|event| event.event_type == "ward-broken")
        .expect("Ward absorption");
    assert_eq!(broken.payload["instanceId"], enemy_id);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed" || event.event_type == "minion-died")
    );

    let after = state(&session);
    let survivor = realm_unit(&after, &enemy_id).expect("Ward survivor");
    assert_eq!(survivor["warded"], false);
    assert_eq!(survivor["damage"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1014_kill_target_minion_deathrite_draws_for_controller_before_magic_resolved() {
    let encoded = kill_target_deathrite_manifest(1014);
    let mut session = opening_main(&encoded);
    let enemy_id = stage_south_minion(&mut session);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    assert_eq!(
        south_atlas, 1,
        "thin South atlas leaves one site before the kill-target Deathrite draw"
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == enemy_id
    });
    let types = event_types(&receipt);
    assert!(types.contains(&"magic-cast"));
    assert!(types.contains(&"minion-killed"));
    assert!(types.contains(&"site-drawn"));
    assert!(types.contains(&"minion-died"));
    assert!(types.contains(&"magic-resolved"));

    let minion_killed = types
        .iter()
        .position(|event_type| *event_type == "minion-killed")
        .expect("minion-killed index");
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let minion_died = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion-died index");
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        minion_killed < site_drawn && site_drawn < minion_died && minion_died < magic_resolved,
        "expected minion-killed, deathrite site-drawn, minion-died, then magic-resolved; got {types:?}"
    );

    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], enemy_id);

    let finished = state(&session);
    assert!(realm_unit(&finished, &enemy_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1085_kill_target_minion_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_kill_seed_with(1085);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete kill-target Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
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
    assert!(kill_targets(session).is_empty());

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(realm_unit(&resumed, &visitor_id).is_some());
    assert_eq!(kill_targets(session), [visitor_id.as_str()]);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert!(realm_unit(&state(session), &visitor_id).is_none());
    assert_exact_replay(session);
}

fn supplemental_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn kill_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "kill-target-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-kill-target-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-kill": kill_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": supplemental_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-kill"; 8],
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
        .chain(619..619 + 2048)
        .map(kill_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-kill")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with kill-target Magic and required South minions")
}

fn kill_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-kill")
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

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("kill-target supplemental identity")
        .to_owned()
}

fn setup_c2_with_south_minions(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    (0..count).map(|_| summon_south_at(session, "C2")).collect()
}

fn cast_kill_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    let c2_ids = setup_c2_with_south_minions(&mut session, 2);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let far_id = summon_south_at(&mut session, "C4");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (kill_spells_in_hand(&state(&session)) >= 1).then_some((session, c2_ids[0].clone(), far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(619..619 + 2048)
        .find_map(|seed| {
            let encoded = kill_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching kill-target far-minion setup")
}

fn try_second_kill_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_kill_on(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if kill_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let minion_id = summon_south_at(&mut session, "C3");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    kill_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_kill_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(619..619 + 8192)
        .find_map(|seed| {
            let encoded = kill_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_kill_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second kill-target enemy-arrival setup")
}

fn try_second_kill_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_kill_on(&mut session, &first_id);
    if realm_unit(&state(&session), &first_id).is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if kill_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    kill_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_kill_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(619..619 + 8192)
        .find_map(|seed| {
            let encoded = kill_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_kill_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second kill-target new-summon setup")
}

#[test]
fn rule_catalog_2063_healthy_minion_stays_at_the_location_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2063, 1);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 0);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C2");
    assert!(realm_unit(&state(&session), &minion_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2064_second_kill_offers_no_targets_after_killing_the_only_minion() {
    let encoded = (2064..2064 + 8192)
        .chain(619..619 + 8192)
        .find_map(|seed| {
            let candidate = kill_supplemental_manifest(seed);
            let mut session = opening_main(&candidate);
            let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
            north_draws_spellbook(&mut session);
            let first = cast_kill_on(&mut session, &minion_id);
            if !event_types(&first).contains(&"minion-died") {
                return None;
            }
            if realm_unit(&state(&session), &minion_id).is_some() {
                return None;
            }
            (kill_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two kill-target casts after clearing minions");
    let mut session = opening_main(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_kill_on(&mut session, &minion_id);
    assert!(event_types(&first).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(kill_spells_in_hand(&state(&session)) >= 1);
    assert!(kill_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-kill"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2065_second_kill_destroys_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_kill_enemy_arrival(2065);
    let (mut session, minion_id) = try_second_kill_enemy_arrival_prefix(&encoded)
        .expect("second kill-target enemy-arrival prefix");
    let receipt = cast_kill_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2066_kill_target_offers_every_same_region_minion_in_the_caster_region() {
    let encoded = supplemental_seed_with_start(2066, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c2_with_south_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = kill_targets(&session);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2067_kill_target_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(2067);
    let (mut session, killed_id, far_id) =
        try_far_minion_prefix(&encoded).expect("kill-target far-minion prefix");
    let receipt = cast_kill_on(&mut session, &killed_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &killed_id).is_none());
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2068_second_kill_destroys_a_newly_summoned_minion() {
    let encoded = seed_for_second_kill_new_summon(2068);
    let (mut session, minion_id) =
        try_second_kill_new_summon_prefix(&encoded).expect("second kill-target new-summon prefix");
    let receipt = cast_kill_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == minion_id))
    );
    assert_exact_replay(&session);
}
