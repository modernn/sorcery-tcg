//! Direct proofs that nearby doubled unit strikes apply to Ranged projectiles,
//! Genesis strikes (RULE-CATALOG-0252–0255, RULE-CATALOG-0953, RULE-CATALOG-0969), Leap Attack
//! strikes (RULE-CATALOG-0945, RULE-CATALOG-0964), and ally-strike-here Magic
//! (RULE-CATALOG-0947, RULE-CATALOG-0966).
//!
//! Official Mask of Mayhem FAQ doubles a strike when the struck unit is nearby
//! the source, including distant Ranged strikers. Ordinary combat already uses
//! the shared helper; these proofs cover the remaining strike apply sites.

use std::sync::OnceLock;

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

fn ranged_opening_manifest() -> &'static str {
    static MANIFEST: OnceLock<String> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        south_opening_manifest(
            "synthetic-nearby-ranged-double-strike-v1",
            "north-shooter",
            &minion(json!({
                "defense": 4,
                "ranged": true,
            })),
        )
    })
}

fn genesis_opening_manifest() -> &'static str {
    static MANIFEST: OnceLock<String> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        south_opening_manifest(
            "synthetic-nearby-genesis-double-strike-v1",
            "north-titan",
            &minion(json!({
                "defense": 4,
                "genesisStrikeEachEnemyHere": true,
                "summonToAnySite": true,
            })),
        )
    })
}

fn leap() -> Value {
    json!({
        "cardType": "magic",
        "leapAttackAlly": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn leap_opening_manifest() -> &'static str {
    static MANIFEST: OnceLock<String> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        (1..=4096)
            .map(|seed| {
                let cards = json!({
                    "north-ally": minion(json!({ "summonToAnySite": true })),
                    "north-avatar": avatar(),
                    "north-leap": leap(),
                    "north-site": site(),
                    "south-avatar": avatar(),
                    "south-mask": double_mask(),
                    "south-minion": minion(json!({ "summonToAnySite": true })),
                    "south-site": site(),
                });
                let mut value = json!({
                    "authority": {
                        "contentHash": identity_hash(&json!({
                            "fixture": "synthetic-nearby-leap-double-strike-v1"
                        }))
                        .expect("synthetic authority identity"),
                        "mode": "synthetic",
                        "revisionId": "synthetic-nearby-leap-double-strike-v1",
                    },
                    "cards": cards,
                    "decks": {
                        "north": {
                            "atlas": vec!["north-site"; 6],
                            "avatar": "north-avatar",
                            "spellbook": [
                                "north-leap",
                                "north-ally",
                                "north-leap",
                                "north-ally",
                                "north-leap",
                                "north-ally",
                            ],
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
                sorcery_engine::canonical::canonical_json(&value)
                    .expect("canonical synthetic manifest")
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
    })
}

fn ally_strike_magic() -> Value {
    json!({
        "allyStrikesEachEnemyAtItsLocation": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn ally_strike_opening_manifest() -> &'static str {
    static MANIFEST: OnceLock<String> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        (1..=4096)
            .map(|seed| {
                let cards = json!({
                    "north-ally": minion(json!({})),
                    "north-avatar": avatar(),
                    "north-site": site(),
                    "north-spin": ally_strike_magic(),
                    "south-avatar": avatar(),
                    "south-mask": double_mask(),
                    "south-minion": minion(json!({ "summonToAnySite": true })),
                    "south-site": site(),
                });
                let mut value = json!({
                    "authority": {
                        "contentHash": identity_hash(&json!({
                            "fixture": "synthetic-nearby-ally-strike-double-strike-v1"
                        }))
                        .expect("synthetic authority identity"),
                        "mode": "synthetic",
                        "revisionId": "synthetic-nearby-ally-strike-double-strike-v1",
                    },
                    "cards": cards,
                    "decks": {
                        "north": {
                            "atlas": vec!["north-site"; 6],
                            "avatar": "north-avatar",
                            "spellbook": [
                                "north-ally",
                                "north-spin",
                                "north-spin",
                                "north-ally",
                                "north-spin",
                                "north-spin",
                            ],
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
                sorcery_engine::canonical::canonical_json(&value)
                    .expect("canonical synthetic manifest")
            })
            .find(|candidate| {
                let opening = state(&Session::new(candidate).expect("opening candidate"));
                let hand = opening["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .expect("north opening spellbook");
                hand.iter().any(|card| card["cardId"] == "north-spin")
                    && hand.iter().any(|card| card["cardId"] == "north-ally")
            })
            .expect("bounded seed opening with ally-strike Magic and a north ally")
    })
}

fn after_ally_strike_ready(carry_mask: bool) -> Session {
    let mut session =
        Session::new(ally_strike_opening_manifest()).expect("valid ally-strike session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    cast_south_mask(&mut session, carry_mask);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    session
}

fn after_leap_ready(carry_mask: bool) -> Session {
    let mut session = Session::new(leap_opening_manifest()).expect("valid leap session");
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    session
}

fn after_ranged_ready(carry_mask: bool) -> Session {
    let mut session = Session::new(ranged_opening_manifest()).expect("valid ranged session");
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

fn after_south_holds_c4(carry_mask: bool) -> Session {
    let mut session = Session::new(genesis_opening_manifest()).expect("valid genesis session");
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

fn after_south_pair_shares_c4_with_mask(carry_mask: bool) -> Session {
    let mut session = Session::new(genesis_opening_manifest()).expect("valid genesis session");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
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
    let mut session = after_south_holds_c4(true);
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
fn rule_catalog_0945_leap_attack_strike_deals_double_damage_when_the_struck_unit_is_nearby() {
    let mut session = after_leap_ready(true);
    let ally_id = unit_id(&session, "north-ally");
    let target_id = unit_id(&session, "south-minion");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-leap"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["allyDestination"]["cell"] == "C4"
    });
    assert_eq!(strike_amount(&receipt, &target_id), 2);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["instanceId"] != target_id),
        "a 1-power nearby Leap Attack strike must deal 2 and kill a 2-defense minion"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0964_leap_attack_strike_is_not_doubled_when_struck_unit_is_not_nearby_mask() {
    let mut session = after_leap_ready(false);
    let ally_id = unit_id(&session, "north-ally");
    let target_id = unit_id(&session, "south-minion");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-leap"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["allyDestination"]["cell"] == "C4"
    });
    assert_eq!(strike_amount(&receipt, &target_id), 1);
    let after = state(&session);
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("south minion survives an undoubled 1-power Leap Attack strike");
    assert_eq!(target["damage"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0255_genesis_strike_is_not_doubled_when_the_struck_unit_is_not_nearby() {
    let mut session = after_south_holds_c4(false);
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

#[test]
fn rule_catalog_0953_genesis_strike_deals_double_damage_when_enemies_share_the_newcomers_cell_with_nearby_mask()
 {
    let mut session = after_south_pair_shares_c4_with_mask(true);
    let first_target = unit_id(&session, "south-minion");
    let second_target = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-minion" && unit["instanceId"] != first_target)
        .expect("second south minion at C4")["instanceId"]
        .clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-titan"
            && descriptor["cell"] == "C4"
    });
    let doubled: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 2);
            event.payload["targetInstanceId"].clone()
        })
        .collect();
    assert_eq!(doubled.len(), 2);
    assert!(doubled.contains(&first_target));
    assert!(doubled.contains(&second_target));
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["instanceId"] != first_target && unit["instanceId"] != second_target),
        "each nearby-doubled Genesis strike must deal 2 and kill a 2-defense minion"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0969_genesis_strike_each_co_located_enemy_is_not_doubled_without_nearby_mask() {
    let mut session = after_south_pair_shares_c4_with_mask(false);
    let first_target = unit_id(&session, "south-minion");
    let second_target = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-minion" && unit["instanceId"] != first_target)
        .expect("second south minion at C4")["instanceId"]
        .clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-titan"
            && descriptor["cell"] == "C4"
    });
    let undoubled: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 1);
            event.payload["targetInstanceId"].clone()
        })
        .collect();
    assert_eq!(undoubled.len(), 2);
    assert!(undoubled.contains(&first_target));
    assert!(undoubled.contains(&second_target));
    let after = state(&session);
    for target_id in [&first_target, &second_target] {
        let target = after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == *target_id)
            .expect("south minion survives an undoubled 1-power Genesis strike");
        assert_eq!(target["damage"], 1);
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0947_ally_strike_magic_deals_double_damage_when_the_struck_enemy_is_nearby() {
    let mut session = after_ally_strike_ready(true);
    let ally_id = unit_id(&session, "north-ally");
    let target_id = unit_id(&session, "south-minion");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-spin"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(strike_amount(&receipt, &target_id), 2);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["instanceId"] != target_id),
        "a 1-power nearby ally-strike Magic hit must deal 2 and kill a 2-defense minion"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0966_ally_strike_magic_is_not_doubled_when_struck_enemy_is_not_nearby_mask() {
    let mut session = after_ally_strike_ready(false);
    let ally_id = unit_id(&session, "north-ally");
    let target_id = unit_id(&session, "south-minion");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-spin"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(strike_amount(&receipt, &target_id), 1);
    let after = state(&session);
    let target = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("south minion survives an undoubled 1-power ally-strike Magic hit");
    assert_eq!(target["damage"], 1);
    assert_exact_replay(&session);
}
