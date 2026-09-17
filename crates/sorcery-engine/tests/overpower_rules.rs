//! Direct proofs for Overpower this-turn power Magic (RULE-CATALOG-0033, 0700, 0722,
//! RULE-CATALOG-1053).
//!
//! `grantPowerToAllyThisTurn` offers controlled allies, raises current derived
//! power by +2 per source until the current End Phase, and feeds source-aware
//! prevention. Distinct from 0529–0530, which grant to a minion then draw and
//! do not offer Avatars.

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

fn fighter() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn disabled_power_fighter() -> Value {
    json!({
        "attack": 2,
        "burrowing": true,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "stealth": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    })
}

fn sleep() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetMinionWithinTwoStepsUntilDamaged": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn filler() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn enemy() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "preventsDamageFromUnitsWithPowerAtLeast": 4,
        "summonToAnySite": true,
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

fn overpower() -> Value {
    json!({
        "cardType": "magic",
        "grantPowerToAllyThisTurn": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn overpower_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "overpower-this-turn" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-overpower-this-turn-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fighter": fighter(),
            "north-filler": filler(),
            "north-overpower": overpower(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-overpower",
                    "north-overpower",
                    "north-fighter",
                    "north-filler",
                    "north-filler",
                    "north-filler",
                ],
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

fn power_observed_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "temporary-power-observed" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-temporary-power-observed-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fighter": disabled_power_fighter(),
            "north-overpower": overpower(),
            "north-site": site(),
            "north-sleep": sleep(),
            "south-avatar": avatar(),
            "south-filler": filler(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-overpower",
                    "north-overpower",
                    "north-fighter",
                    "north-sleep",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 6],
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
    let mut session = Session::new(encoded).expect("valid Overpower session");
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

fn observed(session: &Session) -> Value {
    session.public_view(Seat::North).expect("North public view")
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn opening_has(encoded: &str, card_ids: &[&str]) -> bool {
    Session::new(encoded).ok().is_some_and(|preview| {
        let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let overpower_count = hand
            .iter()
            .filter(|card| card["cardId"] == "north-overpower")
            .count();
        overpower_count >= 2
            && card_ids
                .iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
    })
}

fn opening_spell_ids(encoded: &str) -> Vec<String> {
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn observed_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected observed unit")
}

fn avatar_stats(snapshot: &Value, seat: &str) -> (u64, u64) {
    let avatar = &snapshot["players"][seat]["avatar"];
    (
        avatar["attack"].as_u64().expect("avatar attack"),
        avatar["defense"].as_u64().expect("avatar defense"),
    )
}

fn overpower_ally_ids(session: &Session) -> Vec<String> {
    let actions = session.legal_actions().expect("Overpower actions");
    let Some(spell_id) = actions.iter().find_map(|action| {
        if action.descriptor["kind"] == "cast-magic"
            && action.descriptor["cardId"] == "north-overpower"
        {
            action.descriptor["cardInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        } else {
            None
        }
    }) else {
        return Vec::new();
    };
    actions
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == spell_id
        })
        .filter_map(|action| {
            assert!(action.descriptor["target"].is_null());
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
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

fn seed_with(start: u32) -> String {
    (start..start + 512)
        .map(overpower_manifest)
        .find(|candidate| opening_has(candidate, &["north-fighter"]))
        .expect("bounded seed with fighter and two Overpower Magics")
}

fn seed_for_power_observed(start: u32) -> String {
    (start..start + 512)
        .map(power_observed_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-fighter")
                && hand.iter().any(|id| id == "north-sleep")
                && hand.iter().any(|id| id == "north-overpower")
        })
        .expect("bounded seed with fighter, Sleep, and Overpower in the opening hand")
}

fn deathrite_overpower_manifest(seed: u32) -> String {
    let fixture = "overpower-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-overpower": overpower(),
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
                    "north-overpower",
                    "north-rain",
                    "north-rain",
                    "north-overpower",
                    "north-rain",
                    "north-overpower",
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

fn north_has_overpower_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-overpower", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteOverpowerSetup {
    avatar_id: String,
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteOverpowerSetup> {
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
    if !north_has_overpower_and_rain(&state(&session)) {
        return None;
    }
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    if !overpower_ally_ids(&session).contains(&avatar_id) {
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
    Some(PendingDeathriteOverpowerSetup {
        avatar_id,
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_overpower_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_overpower_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Overpower Magic in hand")
}

fn realm_unit_opt<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn play_to_powered_board(session: &mut Session) -> (String, String, String) {
    let (fighter_summon, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-fighter"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let fighter_id = fighter_summon["cardInstanceId"]
        .as_str()
        .expect("fighter identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (enemy_summon, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let enemy_id = enemy_summon["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let avatar_id = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    (fighter_id, enemy_id, avatar_id)
}

#[test]
fn rule_catalog_0700_overpower_changes_current_power_until_the_current_end_phase() {
    let encoded = seed_with(700);
    let mut session = opening_main(&encoded);
    let (fighter_id, enemy_id, _) = play_to_powered_board(&mut session);

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-overpower"
            && descriptor["ally"]["instanceId"] == fighter_id
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "power-granted", "magic-resolved"]
    );
    let source_id = cast["cardInstanceId"]
        .as_str()
        .expect("Overpower identity")
        .to_owned();
    assert_eq!(
        granted.events[1].payload,
        json!({
            "amount": 2,
            "instanceId": fighter_id,
            "seat": "north",
            "sourceInstanceId": source_id,
        })
    );
    assert_eq!(
        realm_unit(&state(&session), &fighter_id)["temporaryPowerSources"],
        json!([source_id])
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == fighter_id
            && descriptor["path"]
                .as_array()
                .is_some_and(|path| path.len() == 1)
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == enemy_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let prevented = fight
        .events
        .iter()
        .find(|event| event.event_type == "damage-dealt" && event.payload["instanceId"] == enemy_id)
        .expect("source-aware prevention event");
    assert_eq!(
        prevented.payload,
        json!({
            "accumulated": 0,
            "amount": 0,
            "attemptedAmount": 4,
            "direct": true,
            "instanceId": enemy_id,
            "prevented": true,
            "seat": "south",
        })
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let expiry = ended
        .events
        .iter()
        .position(|event| event.event_type == "power-expired")
        .expect("power expiry");
    let turn_ended = ended
        .events
        .iter()
        .position(|event| event.event_type == "turn-ended")
        .expect("turn ended");
    assert!(expiry < turn_ended);
    assert_eq!(
        ended.events[expiry].payload,
        json!({
            "amount": 2,
            "instanceId": fighter_id,
            "seat": "north",
            "sourceInstanceId": source_id,
        })
    );
    assert!(realm_unit(&state(&session), &fighter_id)["temporaryPowerSources"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0700_overpower_offers_allies_and_stacks_until_end_phase() {
    let encoded = seed_with(1700);
    let mut session = opening_main(&encoded);
    let (fighter_id, enemy_id, avatar_id) = play_to_powered_board(&mut session);

    let mut ally_ids = overpower_ally_ids(&session);
    ally_ids.sort();
    let mut expected = vec![avatar_id.clone(), fighter_id.clone()];
    expected.sort();
    assert_eq!(ally_ids, expected);
    assert!(!ally_ids.contains(&enemy_id));

    let checkpoint = session.clone();
    let (_, avatar_grant) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-overpower"
            && descriptor["ally"]["kind"] == "avatar"
    });
    assert_eq!(
        event_types(&avatar_grant),
        ["magic-cast", "power-granted", "magic-resolved"]
    );
    let (_, avatar_ended) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let avatar_expiry = avatar_ended
        .events
        .iter()
        .position(|event| event.event_type == "power-expired")
        .expect("Avatar power expiry");
    let avatar_turn_ended = avatar_ended
        .events
        .iter()
        .position(|event| event.event_type == "turn-ended")
        .expect("Avatar turn ended");
    assert!(avatar_expiry < avatar_turn_ended);
    assert!(state(&session)["players"]["north"]["avatar"]["temporaryPowerSources"].is_null());

    session = checkpoint;
    let overpower_ids: Vec<_> = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook hand")
        .iter()
        .filter(|card| card["cardId"] == "north-overpower")
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("Overpower identity")
                .to_owned()
        })
        .collect();
    assert_eq!(overpower_ids.len(), 2);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == overpower_ids[0]
            && descriptor["ally"]["instanceId"] == fighter_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == overpower_ids[1]
            && descriptor["ally"]["instanceId"] == fighter_id
    });
    assert_eq!(
        realm_unit(&state(&session), &fighter_id)["temporaryPowerSources"],
        json!(overpower_ids)
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let expiry: Vec<_> = ended
        .events
        .iter()
        .filter(|event| event.event_type == "power-expired")
        .map(|event| event.payload.clone())
        .collect();
    assert_eq!(
        expiry,
        overpower_ids
            .iter()
            .map(|source_id| json!({
                "amount": 2,
                "instanceId": fighter_id,
                "seat": "north",
                "sourceInstanceId": source_id,
            }))
            .collect::<Vec<_>>()
    );
    assert!(realm_unit(&state(&session), &fighter_id)["temporaryPowerSources"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0722_temporary_power_raises_observed_avatar_and_disabled_minion_stats() {
    let encoded = seed_for_power_observed(722);
    let mut session = opening_main(&encoded);
    let (fighter_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-fighter"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let fighter_id = fighter_summon["cardInstanceId"]
        .as_str()
        .expect("fighter identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let before = observed(&session);
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    assert_eq!(avatar_stats(&before, "north"), (1, 1));
    let fighter_before = observed_unit(&before, &fighter_id);
    assert_eq!(fighter_before["attack"], 2);
    assert_eq!(fighter_before["defense"], 2);
    assert_eq!(fighter_before["stealthed"], true);
    assert_eq!(fighter_before["warded"], true);

    let (_, slept) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-sleep"
            && descriptor["target"]["instanceId"] == fighter_id
    });
    assert_eq!(
        event_types(&slept),
        [
            "magic-cast",
            "minion-disabled",
            "stealth-lost",
            "magic-resolved"
        ]
    );
    let after_sleep = observed(&session);
    let disabled_fighter = observed_unit(&after_sleep, &fighter_id);
    assert_eq!(disabled_fighter["disabled"], true);
    assert_eq!(disabled_fighter["attack"], 2);
    assert_eq!(disabled_fighter["defense"], 2);

    let mut ally_ids = overpower_ally_ids(&session);
    ally_ids.sort();
    let mut expected_allies = vec![avatar_id.clone(), fighter_id.clone()];
    expected_allies.sort();
    assert_eq!(ally_ids, expected_allies);

    let (_, avatar_grant) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-overpower"
            && descriptor["ally"]["kind"] == "avatar"
    });
    assert_eq!(
        event_types(&avatar_grant),
        ["magic-cast", "power-granted", "magic-resolved"]
    );
    let after_avatar = observed(&session);
    assert_eq!(avatar_stats(&after_avatar, "north"), (3, 3));
    assert_eq!(
        after_avatar["players"]["north"]["avatar"]["temporaryPowerSources"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let (_, fighter_grant) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-overpower"
            && descriptor["ally"]["instanceId"] == fighter_id
    });
    assert_eq!(
        event_types(&fighter_grant),
        ["magic-cast", "power-granted", "magic-resolved"]
    );
    let after_fighter = observed(&session);
    let powered = observed_unit(&after_fighter, &fighter_id);
    assert_eq!(powered["disabled"], true);
    assert_eq!(powered["attack"], 4);
    assert_eq!(powered["defense"], 4);
    assert_eq!(powered["stealthed"], false);
    assert_eq!(powered["warded"], true);
    assert_eq!(
        powered["temporaryPowerSources"].as_array().map(Vec::len),
        Some(1)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1053_overpower_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_overpower_seed_with(1053);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete Overpower Deathrite withheld setup");
    let avatar_id = setup.avatar_id.clone();
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
    assert!(realm_unit_opt(&paused, &visitor_id).is_some());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(overpower_ally_ids(session).is_empty());

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
    assert!(realm_unit_opt(&resumed, &visitor_id).is_some());
    assert!(overpower_ally_ids(session).contains(&avatar_id));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-overpower"
            && descriptor["ally"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "power-granted", "magic-resolved"]
    );
    assert_eq!(avatar_stats(&observed(session), "north"), (3, 3));
    assert_exact_replay(session);
}
