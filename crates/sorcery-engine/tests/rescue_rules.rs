//! Direct proofs for return-minion-from-own-cemetery Magic
//! (RULE-CATALOG-0593–0594).
//!
//! Ordinary Rescue Magic offers only minions in the caster's own cemetery and
//! returns the chosen instance to the hidden Spellbook hand. An empty own
//! cemetery still resolves the spell as a paid no-op.

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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn kill() -> Value {
    json!({
        "cardType": "magic",
        "killTargetMinion": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn rescue() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnMinionFromOwnCemetery": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn empty_rescue_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rescue-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rescue-empty-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rescue": rescue(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-rescue"; 6],
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

fn rescue_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rescue-minion" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rescue-minion-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-kill": kill(),
            "north-minion": minion(),
            "north-rescue": rescue(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-minion",
                    "north-kill",
                    "north-rescue",
                    "north-minion",
                    "north-kill",
                    "north-rescue",
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
    let mut session = Session::new(encoded).expect("valid rescue session");
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

fn seed_with(required: &[&str], start: u32) -> String {
    (start..start + 256)
        .map(rescue_manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            let snapshot = state(&preview);
            let hand = snapshot["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("opening hand")
                .iter()
                .map(|card| card["cardId"].as_str().expect("card id"))
                .collect::<Vec<_>>();
            required.iter().all(|id| hand.contains(id))
        })
        .expect("bounded seed with required opening cards")
}

fn cemetery_minions<'a>(snapshot: &'a Value, seat: &str) -> Vec<&'a Value> {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|card| card["cardType"] == "minion")
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

fn setup_own_cemetery_minion(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned minion identity")
        .to_owned();
    let (_, killed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == minion_id
    });
    assert!(event_types(&killed).contains(&"minion-killed"));
    assert!(
        state(session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == minion_id)
    );
    minion_id
}

#[test]
fn rule_catalog_0593_rescue_returns_an_own_cemetery_minion_to_hidden_hand() {
    let encoded = seed_with(&["north-minion", "north-kill", "north-rescue"], 593);
    let mut session = opening_main(&encoded);
    let minion_id = setup_own_cemetery_minion(&mut session);
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();

    let (cast, returned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-rescue"
            && descriptor["cemeteryMinionInstanceId"] == minion_id
    });
    assert_eq!(
        event_types(&returned),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(returned.events[1].payload["instanceId"], minion_id);
    assert_eq!(
        returned.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == minion_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .is_none_or(|cemetery| !cemetery.iter().any(|card| card["instanceId"] == minion_id))
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0594_rescue_with_empty_cemetery_is_a_paid_noop() {
    let encoded = (594..594 + 256)
        .map(empty_rescue_manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            let snapshot = state(&preview);
            snapshot["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rescue"))
                && cemetery_minions(&snapshot, "north").is_empty()
        })
        .expect("bounded seed with Rescue and no cemetery minions");
    let mut session = opening_main(&encoded);
    assert!(cemetery_minions(&state(&session), "north").is_empty());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rescue"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(cemetery_minions(&state(&session), "north").is_empty());
    assert_exact_replay(&session);
}
