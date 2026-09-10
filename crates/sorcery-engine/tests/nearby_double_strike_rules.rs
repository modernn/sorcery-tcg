//! Direct proofs that nearby doubled unit strikes apply to Ranged projectiles
//! and Genesis strikes (RULE-CATALOG-0252–0255).
//!
//! Official Mask of Mayhem FAQ doubles a strike when the struck unit is nearby
//! the source, including distant Ranged strikers. Ordinary combat already uses
//! the shared helper; these proofs cover the remaining strike apply sites.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn double_mask() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": 0,
        "nearbyStrikesAgainstUnitsDealDoubleDamage": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn unit_id(session: &Session, card_id: &str) -> Value {
    state(session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("expected unit")["instanceId"]
        .clone()
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

fn strike_amount(receipt: &Receipt, target_id: &Value) -> i64 {
    receipt
        .events
        .iter()
        .find(|event| {
            event.event_type == "strike-damage-allocated"
                && event.payload["targetInstanceId"] == *target_id
        })
        .expect("strike allocation")
        .payload["amount"]
        .as_i64()
        .expect("strike amount")
}

fn south_opening_manifest(fixture: &str, north_spell: &str, north_card: &Value) -> String {
    (1..=4096)
        .map(|seed| {
            let mut cards = json!({
                "north-avatar": avatar(),
                "north-site": site(),
                "south-avatar": avatar(),
                "south-mask": double_mask(),
                "south-minion": minion(json!({ "summonToAnySite": true })),
                "south-site": site(),
            });
            cards[north_spell] = north_card.clone();
            let mut value = json!({
                "authority": {
                    "contentHash": identity_hash(&json!({ "fixture": fixture }))
                        .expect("synthetic authority identity"),
                    "mode": "synthetic",
                    "revisionId": fixture,
                },
                "cards": cards,
                "decks": {
                    "north": {
                        "atlas": vec!["north-site"; 6],
                        "avatar": "north-avatar",
                        "spellbook": vec![north_spell; 6],
                    },
                    "south": {
                        "atlas": vec!["south-site"; 6],
                        "avatar": "south-avatar",
                        "spellbook": [
                            "south-mask",
                            "south-minion",
                            "south-minion",
                            "south-mask",
                            "south-minion",
                            "south-minion",
                        ],
                    },
                },
                "engineVersion": "sorcery-core-v1",
                "firstSeat": "north",
                "schemaVersion": 1,
                "seed": seed,
            });
            value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
            sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
        })
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("opening candidate"));
            let hand = opening["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("south opening spellbook");
            hand.iter().any(|card| card["cardId"] == "south-mask")
                && hand.iter().any(|card| card["cardId"] == "south-minion")
        })
        .expect("bounded seed opening with a Mask and a south minion")
}

fn cast_south_mask(session: &mut Session, carry_on_minion: bool) {
    if carry_on_minion {
        let bearer_id = unit_id(session, "south-minion");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "south-mask"
                && descriptor["bearer"]["instanceId"] == bearer_id
        });
    } else {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "south-mask"
                && descriptor["cell"] == "C1"
                && descriptor["bearer"].is_null()
        });
    }
}

fn after_ranged_ready(carry_mask: bool) -> Session {
    let mut session = Session::new(&south_opening_manifest(
        "synthetic-nearby-ranged-double-strike-v1",
        "north-shooter",
        &minion(json!({
            "defense": 4,
            "ranged": true,
        })),
    ))
    .expect("valid ranged session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-shooter"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    });
    cast_south_mask(&mut session, carry_mask);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    session
}

fn shoot_south(session: &mut Session, shooter_id: &Value, target_id: &Value) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["direction"] == "south"
            && descriptor["shooterInstanceId"] == *shooter_id
            && descriptor["hit"]["instanceId"] == *target_id
    })
    .1
}

fn after_south_holds_c4(carry_mask: bool, north_spell: &str, north_card: &Value) -> Session {
    let mut session = Session::new(&south_opening_manifest(
        "synthetic-nearby-genesis-double-strike-v1",
        north_spell,
        north_card,
    ))
    .expect("valid genesis session");
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
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    cast_south_mask(&mut session, carry_mask);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    session
}

#[test]
fn rule_catalog_0252_ranged_strike_deals_double_damage_when_the_struck_unit_is_nearby() {
    let mut session = after_ranged_ready(true);
    let shooter_id = unit_id(&session, "north-shooter");
    let target_id = unit_id(&session, "south-minion");
    let receipt = shoot_south(&mut session, &shooter_id, &target_id);
    assert_eq!(strike_amount(&receipt, &target_id), 2);
    assert_eq!(state(&session)["phase"], "main");
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["instanceId"] != target_id),
        "a 1-power nearby Ranged strike must deal 2 and kill a 2-defense minion"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0253_ranged_strike_is_not_doubled_when_the_struck_unit_is_not_nearby() {
    let mut session = after_ranged_ready(false);
    let shooter_id = unit_id(&session, "north-shooter");
    let target_id = unit_id(&session, "south-minion");
    let receipt = shoot_south(&mut session, &shooter_id, &target_id);
    assert_eq!(strike_amount(&receipt, &target_id), 1);
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("south minion survives an undoubled 1-power Ranged strike");
    assert_eq!(target["damage"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0254_genesis_strike_deals_double_damage_when_the_struck_unit_is_nearby() {
    let mut session = after_south_holds_c4(
        true,
        "north-titan",
        &minion(json!({
            "defense": 4,
            "genesisStrikeEachEnemyHere": true,
            "summonToAnySite": true,
        })),
    );
    let target_id = unit_id(&session, "south-minion");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-titan"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(strike_amount(&receipt, &target_id), 2);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["instanceId"] != target_id),
        "a 1-power nearby Genesis strike must deal 2 and kill a 2-defense minion"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0255_genesis_strike_is_not_doubled_when_the_struck_unit_is_not_nearby() {
    let mut session = after_south_holds_c4(
        false,
        "north-titan",
        &minion(json!({
            "defense": 4,
            "genesisStrikeEachEnemyHere": true,
            "summonToAnySite": true,
        })),
    );
    let target_id = unit_id(&session, "south-minion");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-titan"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(strike_amount(&receipt, &target_id), 1);
    let after = state(&session);
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("south minion survives an undoubled 1-power Genesis strike");
    assert_eq!(target["damage"], 1);
    assert_exact_replay(&session);
}
