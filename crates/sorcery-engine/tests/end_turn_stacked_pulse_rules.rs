//! Direct proofs for stacked end-of-controller-turn pulses on one minion
//! (RULE-CATALOG-0389–0390, RULE-CATALOG-0395–0396, RULE-CATALOG-0905,
//! RULE-CATALOG-0915, RULE-CATALOG-0925, RULE-CATALOG-0935), and deferred
//! end-turn here-area pulses withheld during deathrite-order
//! (RULE-CATALOG-1208–1220).

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    })
}

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn stacked_life() -> Value {
    json!({
        "atEndOfControllerTurnControllerGainsLife": 3,
        "atEndOfControllerTurnControllerLosesLife": 2,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn stacked_gain_here_damage() -> Value {
    json!({
        "atEndOfControllerTurnControllerGainsLife": 2,
        "atEndOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn stacked_partial_gain_here_damage() -> Value {
    json!({
        "atEndOfControllerTurnControllerGainsLife": 3,
        "atEndOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn stacked_loss_here_damage() -> Value {
    json!({
        "atEndOfControllerTurnControllerLosesLife": 2,
        "atEndOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn stacked_triple() -> Value {
    json!({
        "atEndOfControllerTurnControllerGainsLife": 3,
        "atEndOfControllerTurnControllerLosesLife": 2,
        "atEndOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn stacked_triple_disabled_on_entry() -> Value {
    let mut minion = stacked_triple();
    minion["genesisDisableSelfUntilDamaged"] = json!(true);
    minion
}

fn drain() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "targetPlayerLosesLife": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn visitor(defense: u8) -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn life_stack_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-stacked-life" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-stacked-life-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-site": site(),
            "north-source": stacked_life(),
            "south-avatar": avatar(20),
            "south-drain": drain(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-drain"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn triple_pulse_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-triple-pulse" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-triple-pulse-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-pulser": stacked_triple(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-drain": drain(),
            "south-site": site(),
            "south-visitor": visitor(2),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-pulser"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn triple_pulse_disabled_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-triple-pulse-disabled" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-triple-pulse-disabled-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-pulser": stacked_triple_disabled_on_entry(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-drain": drain(),
            "south-site": site(),
            "south-visitor": visitor(2),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-pulser"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn loss_here_damage_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-loss-here-damage" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-loss-here-damage-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-pulser": stacked_loss_here_damage(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-drain": drain(),
            "south-site": site(),
            "south-visitor": visitor(2),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-pulser"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn partial_gain_here_damage_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-partial-gain-here-damage" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-partial-gain-here-damage-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-pulser": stacked_partial_gain_here_damage(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-drain": drain(),
            "south-site": site(),
            "south-visitor": visitor(2),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-pulser"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn gain_here_damage_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-gain-here-damage" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-gain-here-damage-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-pulser": stacked_gain_here_damage(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-drain": drain(),
            "south-site": site(),
            "south-visitor": visitor(2),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-pulser"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                    "south-drain",
                    "south-visitor",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
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

fn avatar_id(session: &Session, seat: &str) -> Value {
    state(session)["players"][seat]["avatar"]["card"]["instanceId"].clone()
}

fn allocated_here_damage_targets(receipt: &Receipt, source_id: &Value) -> Vec<Value> {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == "end-turn-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 1);
            assert_eq!(event.payload["sourceInstanceId"], *source_id);
            event.payload["targetInstanceId"].clone()
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

fn after_north_source_ready_to_end_turn() -> Session {
    let mut session = Session::new(&life_stack_manifest()).expect("valid stacked life session");
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
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    session
}

fn after_north_source_ready_at_printed_cap() -> Session {
    let mut session = Session::new(&life_stack_manifest()).expect("valid stacked life session");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
    });
    session
}

fn after_north_ready_to_end_turn_with_triple_pulser() -> Session {
    let mut session = Session::new(&triple_pulse_manifest()).expect("valid triple-pulse session");
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
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    });
    session
}

fn after_north_ready_to_end_turn_with_disabled_triple_pulser() -> Session {
    let mut session =
        Session::new(&triple_pulse_disabled_manifest()).expect("valid disabled triple session");
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
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    });
    session
}

fn after_north_ready_to_end_turn_with_loss_here_pulser() -> Session {
    let mut session =
        Session::new(&loss_here_damage_manifest()).expect("valid loss-here-damage session");
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
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    });
    session
}

fn after_north_ready_to_end_turn_with_partial_gain_here_pulser() -> Session {
    let mut session = Session::new(&partial_gain_here_damage_manifest())
        .expect("valid partial-gain-here-damage session");
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
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    });
    session
}

fn after_north_ready_to_end_turn_with_visitor() -> Session {
    let mut session =
        Session::new(&gain_here_damage_manifest()).expect("valid gain-here-damage session");
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
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    });
    session
}

fn event_index(receipt: &Receipt, event_type: &str) -> Option<usize> {
    receipt
        .events
        .iter()
        .position(|event| event.event_type == event_type)
}

#[test]
fn rule_catalog_0915_end_turn_gain_noop_at_cap_then_loss_applies() {
    let mut session = after_north_source_ready_at_printed_cap();
    let source_id = unit_id(&session, "north-source");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(before["phase"], "main");
    assert_eq!(before["activeSeat"], "north");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-healed"),
        "life gain at printed cap must not emit avatar-healed"
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-life-lost"
            && event.payload["amount"] == 2
            && event.payload["life"] == 18
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 18);
    assert!(after["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0389_end_turn_gain_then_loss_applies_net_avatar_life() {
    let mut session = after_north_source_ready_to_end_turn();
    let source_id = unit_id(&session, "north-source");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    assert_eq!(before["phase"], "main");
    assert_eq!(before["activeSeat"], "north");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let heal_index = event_index(&receipt, "avatar-healed").expect("life gain event");
    let loss_index = event_index(&receipt, "avatar-life-lost").expect("life loss event");
    assert!(
        heal_index < loss_index,
        "end-turn life gain must resolve before life loss on the same minion"
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-healed"
            && event.payload["amount"] == 2
            && event.payload["attemptedAmount"] == 3
            && event.payload["life"] == 20
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-life-lost"
            && event.payload["amount"] == 2
            && event.payload["life"] == 18
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 18);
    assert!(after["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0925_end_turn_partial_gain_then_here_damage_on_same_minion() {
    let mut session = after_north_ready_to_end_turn_with_partial_gain_here_pulser();
    let source_id = unit_id(&session, "north-pulser");
    let visitor_id = unit_id(&session, "south-visitor");
    let north_avatar_id = avatar_id(&session, "north");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    assert_eq!(before["phase"], "main");
    assert_eq!(before["activeSeat"], "north");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let heal_index = event_index(&receipt, "avatar-healed").expect("life gain event");
    let damage_index =
        event_index(&receipt, "end-turn-damage-allocated").expect("here damage event");
    assert!(
        heal_index < damage_index,
        "end-turn life gain must resolve before here-area damage on the same minion"
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-healed"
            && event.payload["amount"] == 2
            && event.payload["attemptedAmount"] == 3
            && event.payload["life"] == 20
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let mut here_targets = allocated_here_damage_targets(&receipt, &source_id);
    here_targets.sort_by_key(Value::to_string);
    let mut expected_here_targets = vec![north_avatar_id, visitor_id.clone()];
    expected_here_targets.sort_by_key(Value::to_string);
    assert_eq!(here_targets, expected_here_targets);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 19);
    assert_eq!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == visitor_id)
            .expect("visitor")["damage"],
        0,
        "end-turn here damage clears during end-phase cleanup unlike start-turn pulses"
    );
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0390_end_turn_gain_then_here_damage_on_same_minion() {
    let mut session = after_north_ready_to_end_turn_with_visitor();
    let source_id = unit_id(&session, "north-pulser");
    let visitor_id = unit_id(&session, "south-visitor");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let heal_index = event_index(&receipt, "avatar-healed").expect("life gain event");
    let damage_index =
        event_index(&receipt, "end-turn-damage-allocated").expect("here damage event");
    assert!(
        heal_index < damage_index,
        "end-turn life gain must resolve before here-area damage on the same minion"
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-healed"
            && event.payload["amount"] == 2
            && event.payload["attemptedAmount"] == 2
            && event.payload["life"] == 20
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "end-turn-damage-allocated"
            && event.payload["amount"] == 1
            && event.payload["sourceInstanceId"] == source_id
            && event.payload["targetInstanceId"] == visitor_id
    }));
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 19);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["cardId"] == "north-pulser" && unit["instanceId"] == source_id)
    );
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["cardId"] == "south-visitor" && unit["instanceId"] == visitor_id)
    );
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0395_end_turn_gain_loss_then_here_damage_on_same_minion() {
    let mut session = after_north_ready_to_end_turn_with_triple_pulser();
    let source_id = unit_id(&session, "north-pulser");
    let visitor_id = unit_id(&session, "south-visitor");
    let north_avatar_id = avatar_id(&session, "north");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let heal_index = event_index(&receipt, "avatar-healed").expect("life gain event");
    let loss_index = event_index(&receipt, "avatar-life-lost").expect("life loss event");
    let damage_index =
        event_index(&receipt, "end-turn-damage-allocated").expect("here damage event");
    assert!(
        heal_index < loss_index && loss_index < damage_index,
        "end-turn pulses must resolve gain, then loss, then here-area damage on the same minion"
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-healed"
            && event.payload["amount"] == 2
            && event.payload["attemptedAmount"] == 3
            && event.payload["life"] == 20
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-life-lost"
            && event.payload["amount"] == 2
            && event.payload["life"] == 18
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let mut here_targets = allocated_here_damage_targets(&receipt, &source_id);
    here_targets.sort_by_key(Value::to_string);
    let mut expected_here_targets = vec![north_avatar_id, visitor_id.clone()];
    expected_here_targets.sort_by_key(Value::to_string);
    assert_eq!(here_targets, expected_here_targets);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 17);
    assert_eq!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == visitor_id)
            .expect("visitor")["damage"],
        0
    );
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0905_end_turn_loss_then_here_damage_on_same_minion() {
    let mut session = after_north_ready_to_end_turn_with_loss_here_pulser();
    let source_id = unit_id(&session, "north-pulser");
    let visitor_id = unit_id(&session, "south-visitor");
    let north_avatar_id = avatar_id(&session, "north");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    assert_eq!(before["phase"], "main");
    assert_eq!(before["activeSeat"], "north");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let loss_index = event_index(&receipt, "avatar-life-lost").expect("life loss event");
    let damage_index =
        event_index(&receipt, "end-turn-damage-allocated").expect("here damage event");
    assert!(
        loss_index < damage_index,
        "end-turn life loss must resolve before here-area damage on the same minion"
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-healed"),
        "loss-here stack must not emit life gain"
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-life-lost"
            && event.payload["amount"] == 2
            && event.payload["life"] == 16
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let mut here_targets = allocated_here_damage_targets(&receipt, &source_id);
    here_targets.sort_by_key(Value::to_string);
    let mut expected_here_targets = vec![north_avatar_id, visitor_id.clone()];
    expected_here_targets.sort_by_key(Value::to_string);
    assert_eq!(here_targets, expected_here_targets);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 15);
    assert_eq!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == visitor_id)
            .expect("visitor")["damage"],
        0,
        "end-turn here damage clears during end-phase cleanup unlike start-turn pulses"
    );
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0935_end_turn_partial_gain_loss_then_here_damage_on_same_minion() {
    let mut session = after_north_ready_to_end_turn_with_triple_pulser();
    let source_id = unit_id(&session, "north-pulser");
    let visitor_id = unit_id(&session, "south-visitor");
    let north_avatar_id = avatar_id(&session, "north");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    assert_eq!(before["phase"], "main");
    assert_eq!(before["activeSeat"], "north");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let heal_index = event_index(&receipt, "avatar-healed").expect("partial life gain event");
    let loss_index = event_index(&receipt, "avatar-life-lost").expect("life loss event");
    let damage_index =
        event_index(&receipt, "end-turn-damage-allocated").expect("here damage event");
    assert!(
        heal_index < loss_index && loss_index < damage_index,
        "below printed life, end-turn triple pulses must resolve partial gain, then loss, then here-area damage"
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-healed"
            && event.payload["amount"] == 2
            && event.payload["attemptedAmount"] == 3
            && event.payload["life"] == 20
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-life-lost"
            && event.payload["amount"] == 2
            && event.payload["life"] == 18
            && event.payload["seat"] == "north"
            && event.payload["sourceInstanceId"] == source_id
    }));
    let mut here_targets = allocated_here_damage_targets(&receipt, &source_id);
    here_targets.sort_by_key(Value::to_string);
    let mut expected_here_targets = vec![north_avatar_id, visitor_id.clone()];
    expected_here_targets.sort_by_key(Value::to_string);
    assert_eq!(here_targets, expected_here_targets);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 17);
    assert_eq!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == visitor_id)
            .expect("visitor")["damage"],
        0,
        "end-turn here damage clears during end-phase cleanup unlike start-turn pulses"
    );
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0396_disabled_triple_pulse_minion_skips_all_end_turn_effects() {
    let mut session = after_north_ready_to_end_turn_with_disabled_triple_pulser();
    let source_id = unit_id(&session, "north-pulser");
    let visitor_id = unit_id(&session, "south-visitor");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    assert_eq!(
        before["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_id)
            .expect("disabled pulser")["disabledUntilDamaged"],
        true
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.payload.get("sourceInstanceId") == Some(&source_id)),
        "disabled triple-pulse minion must not emit end-turn pulse events"
    );
    assert!(
        receipt.events.iter().all(|event| {
            event.event_type != "avatar-healed"
                && event.event_type != "avatar-life-lost"
                && event.event_type != "end-turn-damage-allocated"
        }),
        "disabled triple-pulse minion must skip life and here-area end-turn pulses"
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 18);
    assert_eq!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == visitor_id)
            .expect("visitor")["damage"],
        0
    );
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_exact_replay(&session);
}

fn end_turn_here_pulser() -> Value {
    json!({
        "atEndOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn dual_end_turn_pulse_withheld_manifest(
    seed: u32,
    second_card: &str,
    second_minion: Value,
) -> String {
    let mut cards = json!({
        "north-avatar": avatar(20),
        "north-pulser-a": end_turn_here_pulser(),
        "north-site": site(),
        "south-avatar": avatar(20),
        "south-deathrite": deathrite_minion(),
        "south-site": site(),
    });
    cards[second_card] = second_minion;
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-dual-pulse-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-dual-pulse-deathrite-withheld-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser-a",
                    second_card,
                    "north-pulser-a",
                    second_card,
                    "north-pulser-a",
                    second_card,
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

struct PendingEndTurnPulseSetup {
    deathrite_ids: [String; 2],
    deferred_pulser_id: String,
    first_pulser_id: String,
    session: Session,
}

fn try_pending_deathrite_during_dual_end_turn_pulse(
    encoded: &str,
    second_card: &str,
) -> Option<PendingEndTurnPulseSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    let first_deathrite = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let second_deathrite = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser-a"
            && descriptor["cell"] == "C4"
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == second_card
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let first_pulser_id = first.0["cardInstanceId"].as_str()?.to_owned();
    let second_pulser_id = second.0["cardInstanceId"].as_str()?.to_owned();
    let snapshot = state(&session);
    let remaining = snapshot["pendingDeathrites"]["continuation"]["remainingHereDamageInstanceIds"]
        .as_array()?;
    let resolved_source = session
        .transcript()
        .iter()
        .flat_map(|receipt| receipt.events.iter())
        .find(|event| event.event_type == "end-turn-damage-allocated")?
        .payload["sourceInstanceId"]
        .as_str()?;
    if resolved_source != first_pulser_id && resolved_source != second_pulser_id {
        return None;
    }
    let deferred_pulser_id = if resolved_source == first_pulser_id {
        second_pulser_id.clone()
    } else {
        first_pulser_id.clone()
    };
    let first_pulser_id = resolved_source.to_owned();
    if !remaining
        .iter()
        .any(|id| id.as_str() == Some(deferred_pulser_id.as_str()))
    {
        return None;
    }
    let mut resolved_sources = std::collections::BTreeSet::new();
    for event in session
        .transcript()
        .iter()
        .flat_map(|receipt| receipt.events.iter())
        .filter(|event| event.event_type == "end-turn-damage-allocated")
    {
        if let Some(source) = event.payload["sourceInstanceId"].as_str() {
            resolved_sources.insert(source.to_owned());
        }
    }
    if resolved_sources.len() != 1 || !resolved_sources.contains(&first_pulser_id) {
        return None;
    }
    let mut deathrite_ids = [
        first_deathrite.0["cardInstanceId"].as_str()?.to_owned(),
        second_deathrite.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingEndTurnPulseSetup {
        deathrite_ids,
        deferred_pulser_id,
        first_pulser_id,
        session,
    })
}

fn assert_end_turn_pulse_withheld(setup: PendingEndTurnPulseSetup) {
    let deathrite_ids = setup.deathrite_ids.clone();
    let deferred_pulser_id = setup.deferred_pulser_id.clone();
    let first_pulser_id = setup.first_pulser_id.clone();
    let mut session = setup.session;
    let paused = state(&session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        session
            .transcript()
            .iter()
            .flat_map(|receipt| receipt.events.iter())
            .any(|event| {
                event.event_type == "end-turn-damage-allocated"
                    && event.payload["sourceInstanceId"] == first_pulser_id
            }),
        "first end-turn here-damage pulse must resolve before deathrite-order"
    );
    assert!(
        session
            .transcript()
            .iter()
            .flat_map(|receipt| receipt.events.iter())
            .all(|event| {
                event.event_type != "end-turn-damage-allocated"
                    || event.payload["sourceInstanceId"] != deferred_pulser_id
            }),
        "deferred end-turn here-damage pulse must not resolve during deathrite-order"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    assert!(
        session
            .transcript()
            .iter()
            .flat_map(|receipt| receipt.events.iter())
            .any(|event| {
                event.event_type == "end-turn-damage-allocated"
                    && event.payload["sourceInstanceId"] == deferred_pulser_id
            }),
        "deferred end-turn here-damage pulse must resume after deathrite-order clears"
    );
    assert_eq!(state(&session)["phase"], "draw");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1208_end_turn_loss_then_here_damage_pulse_withheld_during_pending_deathrite_order()
{
    let encoded = (1208..1208 + 2048)
        .map(|seed| {
            dual_end_turn_pulse_withheld_manifest(seed, "north-stack", stacked_loss_here_damage())
        })
        .find(|candidate| {
            try_pending_deathrite_during_dual_end_turn_pulse(candidate, "north-stack").is_some()
        })
        .expect("bounded seed that reaches pending Deathrites during loss-here end-turn withhold");
    let setup = try_pending_deathrite_during_dual_end_turn_pulse(&encoded, "north-stack")
        .expect("complete loss-here end-turn Deathrite withheld setup");
    assert_end_turn_pulse_withheld(setup);
}

#[test]
fn rule_catalog_1209_end_turn_gain_then_here_damage_pulse_withheld_during_pending_deathrite_order()
{
    let encoded = (1209..1209 + 2048)
        .map(|seed| {
            dual_end_turn_pulse_withheld_manifest(seed, "north-stack", stacked_gain_here_damage())
        })
        .find(|candidate| {
            try_pending_deathrite_during_dual_end_turn_pulse(candidate, "north-stack").is_some()
        })
        .expect("bounded seed that reaches pending Deathrites during gain-here end-turn withhold");
    let setup = try_pending_deathrite_during_dual_end_turn_pulse(&encoded, "north-stack")
        .expect("complete gain-here end-turn Deathrite withheld setup");
    assert_end_turn_pulse_withheld(setup);
}

#[test]
fn rule_catalog_1210_end_turn_triple_pulse_here_damage_withheld_during_pending_deathrite_order() {
    let encoded = (1210..1210 + 2048)
        .map(|seed| dual_end_turn_pulse_withheld_manifest(seed, "north-stack", stacked_triple()))
        .find(|candidate| {
            try_pending_deathrite_during_dual_end_turn_pulse(candidate, "north-stack").is_some()
        })
        .expect("bounded seed that reaches pending Deathrites during triple end-turn withhold");
    let setup = try_pending_deathrite_during_dual_end_turn_pulse(&encoded, "north-stack")
        .expect("complete triple end-turn Deathrite withheld setup");
    assert_end_turn_pulse_withheld(setup);
}

#[test]
fn rule_catalog_1218_end_turn_partial_gain_then_here_damage_pulse_withheld_during_pending_deathrite_order()
 {
    let encoded = (1218..1218 + 2048)
        .map(|seed| {
            dual_end_turn_pulse_withheld_manifest(
                seed,
                "north-stack",
                stacked_partial_gain_here_damage(),
            )
        })
        .find(|candidate| {
            try_pending_deathrite_during_dual_end_turn_pulse(candidate, "north-stack").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites during partial-gain-here end-turn withhold",
        );
    let setup = try_pending_deathrite_during_dual_end_turn_pulse(&encoded, "north-stack")
        .expect("complete partial-gain-here end-turn Deathrite withheld setup");
    assert_end_turn_pulse_withheld(setup);
}

fn stacked_loss_gain_here_damage() -> Value {
    json!({
        "atEndOfControllerTurnControllerLosesLife": 2,
        "atEndOfControllerTurnControllerGainsLife": 3,
        "atEndOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

#[test]
fn rule_catalog_1219_end_turn_loss_gain_then_here_damage_pulse_withheld_during_pending_deathrite_order()
 {
    let encoded = (1219..1219 + 2048)
        .map(|seed| {
            dual_end_turn_pulse_withheld_manifest(
                seed,
                "north-stack",
                stacked_loss_gain_here_damage(),
            )
        })
        .find(|candidate| {
            try_pending_deathrite_during_dual_end_turn_pulse(candidate, "north-stack").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites during loss-gain-here end-turn withhold",
        );
    let setup = try_pending_deathrite_during_dual_end_turn_pulse(&encoded, "north-stack")
        .expect("complete loss-gain-here end-turn Deathrite withheld setup");
    assert_end_turn_pulse_withheld(setup);
}

#[test]
fn rule_catalog_1220_end_turn_dual_here_damage_pulse_withheld_during_pending_deathrite_order() {
    let encoded = (1220..1220 + 2048)
        .map(|seed| {
            dual_end_turn_pulse_withheld_manifest(seed, "north-pulser-b", end_turn_here_pulser())
        })
        .find(|candidate| {
            try_pending_deathrite_during_dual_end_turn_pulse(candidate, "north-pulser-b").is_some()
        })
        .expect("bounded seed that reaches pending Deathrites during dual-here end-turn withhold");
    let setup = try_pending_deathrite_during_dual_end_turn_pulse(&encoded, "north-pulser-b")
        .expect("complete dual-here end-turn Deathrite withheld setup");
    assert_end_turn_pulse_withheld(setup);
}
