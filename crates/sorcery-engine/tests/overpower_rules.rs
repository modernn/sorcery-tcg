//! Direct proofs for Overpower this-turn power Magic (RULE-CATALOG-0033, 0700).
//!
//! `grantPowerToAllyThisTurn` offers controlled allies, raises current derived
//! power by +2 per source until the current End Phase, and feeds source-aware
//! prevention. Distinct from 0529–0530, which grant to a minion then draw and
//! do not offer Avatars.

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

fn fighter() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
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
