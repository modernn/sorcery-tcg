//! Direct proofs for token Magic summon identity and banishment
//! (RULE-CATALOG-0014, RULE-CATALOG-0687–0688).
//!
//! 0579–0580 prove bordering-site placement and the empty no-op. 0372
//! proves a 2x2 Genesis token banishes. These proofs keep the 0014 harness
//! slice: identities hash from cell, ordinal, source, and state version
//! with no random draws, and a Magic-summoned 1x1 token that dies in combat
//! is banished instead of entering a cemetery.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

const TOKEN_ID: &str = "foot-soldier-token";

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
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn token() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    })
}

fn token_magic() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "summonTokenToEachControlledSiteBorderingEnemySite": TOKEN_ID,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn token_summon_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-summon-harness" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-summon-harness-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-magic": token_magic(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
            TOKEN_ID: token(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-magic"; 6],
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
    let mut session = Session::new(encoded).expect("valid token-summon harness session");
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

fn north_magic_count(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map_or(0, |hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-magic")
                .count()
        })
}

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(token_summon_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_magic_count(&state(&preview)) >= 2)
        })
        .expect("bounded seed with two token Magic cards in the opening hand")
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

fn play_to_token_summon(session: &mut Session) -> (Value, Receipt, String, u64) {
    let (_, empty) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor.get("cemeteryMinionInstanceId").is_none()
    });
    assert_eq!(event_types(&empty), ["magic-cast", "magic-resolved"]);
    assert!(
        state(session)["realm"]["units"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );

    for cell in ["C1", "C3", "C2", "B3", "B2"] {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == cell
        });
    }
    let (summoned_attacker, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "B2"
    });
    let attacker_id = summoned_attacker["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let pre_cast_version = state(session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, summoned) = accept_where(session, |descriptor| descriptor["kind"] == "cast-magic");
    (cast, summoned, attacker_id, pre_cast_version)
}

fn expected_token_ids(source_id: &str, pre_cast_version: u64) -> Vec<String> {
    ["B3", "C3"]
        .into_iter()
        .enumerate()
        .map(|(ordinal, cell)| {
            identity_hash(&json!({
                "cardId": TOKEN_ID,
                "cell": cell,
                "ordinal": ordinal,
                "owner": "north",
                "source": "token",
                "sourceInstanceId": source_id,
                "stateVersion": pre_cast_version,
            }))
            .expect("expected token identity")
            .to_string()
        })
        .collect()
}

#[test]
fn rule_catalog_0687_token_magic_summons_in_cell_order_with_deterministic_identities() {
    let encoded = seed_with(687);
    let mut session = opening_main(&encoded);
    let (cast, summoned, _, pre_cast_version) = play_to_token_summon(&mut session);
    assert_eq!(
        event_types(&summoned),
        [
            "magic-cast",
            "minion-summoned",
            "minion-summoned",
            "magic-resolved",
        ]
    );
    assert_eq!(
        summoned.events[1..3]
            .iter()
            .map(|event| event.payload["cell"].as_str().expect("token cell"))
            .collect::<Vec<_>>(),
        ["B3", "C3"]
    );
    assert!(summoned.random_draws.is_empty());
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    assert!(
        summoned.events[1..3]
            .iter()
            .all(|event| event.payload["sourceInstanceId"] == source_id)
    );
    assert_eq!(
        summoned.events[1..3]
            .iter()
            .map(|event| event.payload["instanceId"]
                .as_str()
                .expect("token identity"))
            .collect::<Vec<_>>(),
        expected_token_ids(source_id, pre_cast_version)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0688_dead_token_banishes_instead_of_entering_a_cemetery() {
    let encoded = seed_with(688);
    let mut session = opening_main(&encoded);
    let (_, summoned, attacker_id, _) = play_to_token_summon(&mut session);
    let killed_id = summoned.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity")
        .to_owned();

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "B3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == killed_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let token_exits: Vec<_> = fight
        .events
        .iter()
        .filter(|event| event.payload["instanceId"] == killed_id)
        .map(|event| event.event_type.as_str())
        .filter(|event_type| matches!(*event_type, "minion-died" | "minion-banished"))
        .collect();
    assert_eq!(token_exits, ["minion-died", "minion-banished"]);
    assert!(
        state(&session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != killed_id)
    );
    assert_exact_replay(&session);
}
