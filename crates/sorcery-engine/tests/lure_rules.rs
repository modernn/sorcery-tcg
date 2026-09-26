//! Direct proofs for lure-enemy-minion-one-step-closer Magic (RULE-CATALOG-0601–0602,
//! RULE-CATALOG-1023, RULE-CATALOG-1973–1978).
//!
//! Lure tempts a nearby enemy minion to take its own closer step toward the
//! caster's ally. Without a qualifying enemy the cast still resolves as a paid
//! no-op. While Deathrites wait for ordering, lure Magic stays withheld until
//! the chain drains.

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

fn enemy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lure() -> Value {
    json!({
        "cardType": "magic",
        "lureEnemyMinionOneStepCloser": true,
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

fn lure_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lure-enemy" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lure-enemy-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lure": lure(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lure"; 6],
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

fn end_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("realm unit")
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

fn lure_enemy_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("lure actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-lure"
        })
        .filter_map(|action| {
            let ally = action.descriptor.get("ally")?;
            let enemy = action.descriptor.get("temptedEnemy")?;
            Some((
                ally["instanceId"].as_str()?.to_owned(),
                enemy["instanceId"].as_str()?.to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn deathrite_lure_manifest(seed: u32) -> String {
    let fixture = "lure-enemy-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lure": lure(),
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
                    "north-lure",
                    "north-rain",
                    "north-rain",
                    "north-lure",
                    "north-rain",
                    "north-lure",
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

fn north_has_lure_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-lure", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteLureSetup {
    avatar_id: String,
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_nearby_enemy(encoded: &str) -> Option<PendingDeathriteLureSetup> {
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
    let visitor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C3"
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
    if !north_has_lure_and_rain(&state(&session)) {
        return None;
    }
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    if lure_enemy_targets(&session).is_empty() {
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
    Some(PendingDeathriteLureSetup {
        avatar_id,
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_lure_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_lure_manifest)
        .find(|candidate| try_pending_deathrite_with_nearby_enemy(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with lure Magic in hand")
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

fn opening_with_tempted_enemy(encoded: &str) -> (Session, String, String) {
    let mut session = Session::new(encoded).expect("valid lure session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    end_and_draw(&mut session);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C3"
    });
    let enemy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned();
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar identity")
        .to_owned();
    (session, avatar_id, enemy_id)
}

#[test]
fn rule_catalog_0601_lure_magic_tempts_a_nearby_enemy_one_step_closer() {
    let encoded = (601..601 + 512)
        .map(lure_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                state(&preview)["players"]["south"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-enemy"))
            })
        })
        .expect("bounded seed with a South minion in the opening hand");
    let (mut session, avatar_id, enemy_id) = opening_with_tempted_enemy(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lure"
            && descriptor["temptedEnemy"]["instanceId"] == enemy_id
            && descriptor["ally"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "unit-lured", "magic-resolved"]
    );
    assert_eq!(realm_unit(&state(&session), &enemy_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0602_lure_magic_no_ops_without_a_nearby_enemy_minion() {
    let encoded = lure_manifest(602);
    let mut session = Session::new(&encoded).expect("valid lure session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let lure_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .find(|card| card["cardId"] == "north-lure")
        .expect("lure in hand")["instanceId"]
        .as_str()
        .expect("lure identity")
        .to_owned();
    let actions: Vec<_> = session
        .legal_actions()
        .expect("lure actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == lure_id
        })
        .collect();
    assert_eq!(actions.len(), 1);
    assert!(actions[0].descriptor.get("temptedEnemy").is_none());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardInstanceId"] == lure_id
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1023_lure_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_lure_seed_with(1023);
    let mut setup = try_pending_deathrite_with_nearby_enemy(&encoded)
        .expect("complete lure Deathrite withheld setup");
    let avatar_id = setup.avatar_id.clone();
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
    assert_eq!(realm_unit(&paused, &visitor_id)["location"], "C3");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(lure_enemy_targets(session).is_empty());

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
    assert_eq!(realm_unit(&resumed, &visitor_id)["location"], "C3");
    assert_eq!(
        lure_enemy_targets(session),
        [(avatar_id.clone(), visitor_id.clone())]
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lure"
            && descriptor["temptedEnemy"]["instanceId"] == visitor_id
            && descriptor["ally"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "unit-lured", "magic-resolved"]
    );
    assert_eq!(realm_unit(&state(session), &visitor_id)["location"], "C4");
    assert_exact_replay(session);
}

fn lure_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lure-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lure-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lure": lure(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-lure", "north-rain", "north-lure", "north-rain", "north-lure",
                    "north-rain", "north-lure", "north-rain", "north-lure", "north-rain",
                    "north-lure", "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 12],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_north_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"]["north"]["hand"]["spellbook"]
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

fn opening_south_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"]["south"]["hand"]["spellbook"]
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

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(601..601 + 2048)
        .map(lure_supplemental_manifest)
        .find(|candidate| {
            required.iter().all(|card| {
                opening_north_spell_ids(candidate)
                    .iter()
                    .any(|id| id == card)
            })
        })
        .expect("bounded seed with required Lure opening cards")
}

fn lure_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-lure")
                .count()
        })
        .unwrap_or_default()
}

fn cast_lure(session: &mut Session, ally_id: &str, enemy_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lure"
            && descriptor["temptedEnemy"]["instanceId"] == enemy_id
            && descriptor["ally"]["instanceId"] == ally_id
    });
    receipt
}

fn try_cast_lure(session: &mut Session, ally_id: &str, enemy_id: &str) -> Option<Receipt> {
    let (_, receipt) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lure"
            && descriptor["temptedEnemy"]["instanceId"] == enemy_id
            && descriptor["ally"]["instanceId"] == ally_id
    })?;
    Some(receipt)
}

fn cast_rain(session: &mut Session) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    receipt
}

fn try_cast_rain(session: &mut Session) -> Option<Receipt> {
    let (_, receipt) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    Some(receipt)
}

fn advance_full_round(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn lure_ready_for_enemy(session: &Session, ally_id: &str, enemy_id: &str) -> bool {
    lure_enemy_targets(session).contains(&(ally_id.to_owned(), enemy_id.to_owned()))
}

fn try_opening_with_tempted_enemy(
    encoded: &str,
    play_avatar_site: bool,
) -> Option<(Session, String, String)> {
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
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C3"
    })?;
    let enemy_id = summoned["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if play_avatar_site {
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
        })?;
    }
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    lure_ready_for_enemy(&session, &avatar_id, &enemy_id).then_some((session, avatar_id, enemy_id))
}

fn try_summon_south_enemy_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn try_two_nearby_enemies_prefix(encoded: &str) -> Option<(Session, String, Vec<String>)> {
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
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let first = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let second = try_summon_south_enemy_at(&mut session, "B4")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    let offered = lure_enemy_targets(&session);
    (offered.contains(&(avatar_id.clone(), first.clone()))
        && offered.contains(&(avatar_id.clone(), second.clone())))
    .then_some((session, avatar_id, vec![first, second]))
}

fn seed_for_two_nearby_enemies(start: u32) -> String {
    (start..start + 16384)
        .chain(601..601 + 16384)
        .find_map(|seed| {
            let encoded = lure_supplemental_manifest(seed);
            if opening_south_spell_ids(&encoded)
                .iter()
                .filter(|card| *card == "south-enemy")
                .count()
                < 2
            {
                return None;
            }
            try_two_nearby_enemies_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching two-nearby-enemy Lure setup")
}

fn try_far_enemy_prefix(encoded: &str) -> Option<(Session, String, String, String)> {
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
    let nearby_id = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let far_id = try_summon_south_enemy_at(&mut session, "C4")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    lure_ready_for_enemy(&session, &avatar_id, &nearby_id)
        .then_some((session, avatar_id, nearby_id, far_id))
}

fn seed_for_far_enemy(start: u32) -> String {
    (start..start + 16384)
        .chain(601..601 + 16384)
        .find_map(|seed| {
            let encoded = lure_supplemental_manifest(seed);
            if opening_south_spell_ids(&encoded)
                .iter()
                .filter(|card| *card == "south-enemy")
                .count()
                < 2
            {
                return None;
            }
            try_far_enemy_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching Lure far-enemy setup")
}

fn try_second_lure_new_enemy_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let (mut session, avatar_id, first_enemy) = try_opening_with_tempted_enemy(encoded, false)?;
    let receipt = try_cast_lure(&mut session, &avatar_id, &first_enemy)?;
    if !event_types(&receipt).contains(&"unit-lured") {
        return None;
    }
    try_cast_rain(&mut session)?;
    if session
        .replay_value()
        .ok()
        .and_then(|value| {
            value["state"]["realm"]["units"]
                .as_array()
                .and_then(|units| {
                    units
                        .iter()
                        .find(|unit| unit["instanceId"] == first_enemy)
                        .map(|_| ())
                })
        })
        .is_some()
    {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if lure_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let second_enemy = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    lure_ready_for_enemy(&session, &avatar_id, &second_enemy).then_some((
        session,
        avatar_id,
        second_enemy,
    ))
}

fn seed_for_second_lure_new_enemy(start: u32) -> String {
    (start..start + 16384)
        .chain(601..601 + 16384)
        .find_map(|seed| {
            let encoded = lure_supplemental_manifest(seed);
            if opening_south_spell_ids(&encoded)
                .iter()
                .filter(|card| *card == "south-enemy")
                .count()
                < 2
            {
                return None;
            }
            try_second_lure_new_enemy_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Lure new-enemy setup")
}

fn try_second_lure_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let (mut session, avatar_id, first_enemy) = try_opening_with_tempted_enemy(encoded, false)?;
    let receipt = try_cast_lure(&mut session, &avatar_id, &first_enemy)?;
    if !event_types(&receipt).contains(&"unit-lured") {
        return None;
    }
    try_cast_rain(&mut session)?;
    if session
        .replay_value()
        .ok()
        .and_then(|value| {
            value["state"]["realm"]["units"]
                .as_array()
                .and_then(|units| {
                    units
                        .iter()
                        .find(|unit| unit["instanceId"] == first_enemy)
                        .map(|_| ())
                })
        })
        .is_some()
    {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if lure_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let second_enemy = try_summon_south_enemy_at(&mut session, "C3")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    lure_ready_for_enemy(&session, &avatar_id, &second_enemy).then_some((
        session,
        avatar_id,
        second_enemy,
    ))
}

fn seed_for_second_lure_enemy_arrival(start: u32) -> String {
    (start..start + 16384)
        .chain(601..601 + 16384)
        .find_map(|seed| {
            let encoded = lure_supplemental_manifest(seed);
            if opening_south_spell_ids(&encoded)
                .iter()
                .filter(|card| *card == "south-enemy")
                .count()
                < 2
            {
                return None;
            }
            try_second_lure_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Lure enemy-arrival setup")
}

#[test]
fn rule_catalog_1973_lured_enemy_stays_at_its_destination_after_turns_pass() {
    let encoded = seed_with_start(1973, &["north-lure"]);
    let (mut session, avatar_id, enemy_id) =
        try_opening_with_tempted_enemy(&encoded, true).expect("Lure opening prefix");
    cast_lure(&mut session, &avatar_id, &enemy_id);
    let location = realm_unit(&state(&session), &enemy_id)["location"]
        .as_str()
        .expect("lured location")
        .to_owned();
    advance_full_round(&mut session);
    assert_eq!(
        realm_unit(&state(&session), &enemy_id)["location"],
        location
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1974_second_lure_without_a_nearby_enemy_is_still_a_paid_noop() {
    let encoded = (1974..1974 + 8192)
        .chain(601..601 + 8192)
        .find_map(|seed| {
            let candidate = lure_supplemental_manifest(seed);
            let (mut session, avatar_id, enemy_id) =
                try_opening_with_tempted_enemy(&candidate, true)?;
            if lure_spells_in_hand(&state(&session)) < 2 {
                return None;
            }
            let first = cast_lure(&mut session, &avatar_id, &enemy_id);
            if !event_types(&first).contains(&"unit-lured") {
                return None;
            }
            cast_rain(&mut session);
            if state(&session)["realm"]["units"]
                .as_array()
                .is_some_and(|units| units.iter().any(|unit| unit["instanceId"] == enemy_id))
            {
                return None;
            }
            (lure_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two Lure casts after setup");
    let (mut session, avatar_id, enemy_id) =
        try_opening_with_tempted_enemy(&encoded, true).expect("Lure opening prefix");
    let first = cast_lure(&mut session, &avatar_id, &enemy_id);
    assert!(event_types(&first).contains(&"unit-lured"));
    cast_rain(&mut session);
    assert!(lure_spells_in_hand(&state(&session)) >= 1);
    let second = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-lure"
    })
    .1;
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"unit-lured"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1975_second_lure_tempts_a_newly_arrived_enemy_after_enemy_site_placement() {
    let encoded = seed_for_second_lure_enemy_arrival(1975);
    let (mut session, avatar_id, enemy_id) =
        try_second_lure_enemy_arrival_prefix(&encoded).expect("second Lure enemy-arrival prefix");
    let receipt = cast_lure(&mut session, &avatar_id, &enemy_id);
    assert!(event_types(&receipt).contains(&"unit-lured"));
    assert_ne!(realm_unit(&state(&session), &enemy_id)["location"], "C3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1976_lure_offers_every_nearby_enemy_minion() {
    let encoded = seed_for_two_nearby_enemies(1976);
    let (session, avatar_id, enemy_ids) =
        try_two_nearby_enemies_prefix(&encoded).expect("two-nearby-enemy Lure prefix");
    let offered = lure_enemy_targets(&session);
    assert_eq!(offered.len(), 2);
    for enemy_id in &enemy_ids {
        assert!(offered.contains(&(avatar_id.clone(), enemy_id.clone())));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1977_lure_leaves_a_far_enemy_untouched() {
    let encoded = seed_for_far_enemy(1977);
    let (mut session, avatar_id, nearby_id, far_id) =
        try_far_enemy_prefix(&encoded).expect("Lure far-enemy prefix");
    let far_location = realm_unit(&state(&session), &far_id)["location"].clone();
    let receipt = cast_lure(&mut session, &avatar_id, &nearby_id);
    assert!(event_types(&receipt).contains(&"unit-lured"));
    assert_ne!(realm_unit(&state(&session), &nearby_id)["location"], "C3");
    assert_eq!(
        realm_unit(&state(&session), &far_id)["location"],
        far_location
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1978_second_lure_tempts_a_newly_summoned_enemy() {
    let encoded = seed_for_second_lure_new_enemy(1978);
    let (mut session, avatar_id, enemy_id) =
        try_second_lure_new_enemy_prefix(&encoded).expect("second Lure new-enemy prefix");
    let receipt = cast_lure(&mut session, &avatar_id, &enemy_id);
    assert!(event_types(&receipt).contains(&"unit-lured"));
    assert_ne!(realm_unit(&state(&session), &enemy_id)["location"], "C3");
    assert_exact_replay(&session);
}
