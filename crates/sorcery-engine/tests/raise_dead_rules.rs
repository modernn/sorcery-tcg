//! Direct proofs for summon-random-minion-from-any-cemetery Magic
//! (RULE-CATALOG-0583–0584).
//!
//! Ordinary Raise Dead draws one random minion from either cemetery and
//! opens free placement. An empty cemetery pool still resolves the spell as a
//! paid no-op.

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

fn victim() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raise_dead() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "summonRandomMinionFromAnyCemetery": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn raise_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "raise-dead" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-raise-dead-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-raise": raise_dead(),
            "north-site": site(),
            "north-zap": zap(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-victim": victim(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-zap",
                    "north-raise",
                    "north-zap",
                    "north-raise",
                    "north-zap",
                    "north-raise",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-victim"; 6],
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
    let mut session = Session::new(encoded).expect("valid raise-dead session");
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

fn seed_with(required: &[&str]) -> String {
    (583..583 + 512)
        .map(raise_manifest)
        .find(|candidate| {
            required
                .iter()
                .all(|id| opening_spell_ids(candidate).iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn end_and_draw_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn setup_south_victim_in_cemetery(session: &mut Session) -> String {
    end_and_draw_spellbook(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-victim"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let victim_id = summoned["cardInstanceId"]
        .as_str()
        .expect("victim identity")
        .to_owned();
    end_and_draw_spellbook(session);
    end_and_draw_spellbook(session);
    let (_, kill) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == victim_id
    });
    assert!(event_types(&kill).contains(&"minion-died"));
    assert!(
        state(session)["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["instanceId"] == victim_id)
    );
    victim_id
}

#[test]
fn rule_catalog_0583_raise_dead_summons_a_random_cemetery_minion_to_a_legal_site() {
    let encoded = seed_with(&["north-zap", "north-raise"]);
    let mut session = opening_main(&encoded);
    let victim_id = setup_south_victim_in_cemetery(&mut session);

    let (cast, cast_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert_eq!(
        event_types(&cast_receipt),
        ["magic-cast", "dead-minion-selected"]
    );
    assert_eq!(cast_receipt.events[1].payload["instanceId"], victim_id);
    assert_eq!(cast_receipt.events[1].payload["cardId"], "south-victim");

    let (_, summon_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-victim"
            && descriptor["cell"] == "C4"
            && descriptor["manaCost"] == 0
    });
    assert!(
        event_types(&summon_receipt)
            .iter()
            .any(|event| *event == "minion-summoned")
    );
    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == victim_id && unit["location"] == "C4")
    );
    assert_eq!(cast["cardInstanceId"], cast_receipt.events[0].payload["sourceInstanceId"]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0584_raise_dead_with_empty_cemetery_is_a_paid_noop() {
    let encoded = seed_with(&["north-raise"]);
    let mut session = opening_main(&encoded);

    assert!(
        state(&session)["players"]["north"]["cemetery"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}
