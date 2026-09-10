//! Direct proofs for Start Phase "doesn't untap" (RULE-CATALOG-0308–0309).
//!
//! Official minions can skip the controller's Start Phase untap. That is a
//! replacement on the shared turn-transition untap, not a start-turn trigger.
//! Disable suppresses the replacement, so a frozen copy untaps normally.

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

fn sleeper() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 2,
        "doesNotUntapDuringControllersStartPhase": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn freeze() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn stay_tapped_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "does-not-untap" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-does-not-untap-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-sleeper": sleeper(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-sleeper"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
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

fn disabled_untap_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "does-not-untap-disabled" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-does-not-untap-disabled-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-freeze": freeze(),
            "north-site": site(),
            "north-sleeper": sleeper(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-sleeper",
                    "north-sleeper",
                    "north-sleeper",
                    "north-freeze",
                    "north-freeze",
                    "north-freeze"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
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

fn disabled_opening_manifest() -> String {
    (1..=4096)
        .map(disabled_untap_manifest)
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("does-not-untap candidate"));
            let hand = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("north opening spellbook");
            hand.iter().any(|card| card["cardId"] == "north-sleeper")
                && hand.iter().any(|card| card["cardId"] == "north-freeze")
        })
        .expect("bounded seed opening with a sleeper and Freeze")
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

fn unit<'a>(state: &'a Value, card_id: &str) -> &'a Value {
    state["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("expected unit")
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

fn summon_and_tap_sleeper(session: &mut Session) -> Value {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-sleeper"
            && descriptor["cell"] == "C4"
    });
    let summoned = state(session);
    let instance_id = unit(&summoned, "north-sleeper")["instanceId"].clone();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    instance_id
}

fn south_plays_and_ends(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

#[test]
fn rule_catalog_0308_does_not_untap_during_the_controllers_start_phase() {
    let mut session = Session::new(&stay_tapped_manifest()).expect("valid does-not-untap session");
    let instance_id = summon_and_tap_sleeper(&mut session);
    let before = state(&session);
    assert_eq!(unit(&before, "north-sleeper")["tapped"], true);
    south_plays_and_ends(&mut session);
    let after_start = state(&session);
    assert_eq!(after_start["phase"], "draw");
    assert_eq!(after_start["activeSeat"], "north");
    assert_eq!(unit(&after_start, "north-sleeper")["tapped"], true);
    assert_eq!(
        unit(&after_start, "north-sleeper")["instanceId"],
        instance_id
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert_eq!(unit(&after, "north-sleeper")["tapped"], true);
    let can_move = session
        .legal_actions()
        .expect("North T2 actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
        });
    assert!(!can_move, "a still-tapped sleeper cannot Move and Attack");
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0309_disable_suppresses_the_does_not_untap_replacement() {
    let mut session =
        Session::new(&disabled_opening_manifest()).expect("valid Disabled does-not-untap session");
    let instance_id = summon_and_tap_sleeper(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == instance_id
    });
    let frozen = state(&session);
    assert_eq!(unit(&frozen, "north-sleeper")["tapped"], true);
    assert_eq!(
        unit(&frozen, "north-sleeper")["disableEffects"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    south_plays_and_ends(&mut session);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(unit(&after, "north-sleeper")["tapped"], false);
    assert_eq!(
        unit(&after, "north-sleeper")["disableEffects"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(unit(&after, "north-sleeper")["instanceId"], instance_id);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}
