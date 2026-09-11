//! Direct proofs that Fate Lose strips Genesis paid tokens (RULE-CATALOG-0333–0334).
//!
//! Official Fate is a Lose effect: a covered non-Ordinary site has no printed
//! abilities, including optional Genesis paid tokens. Ordinary sites in the
//! same 2x2 keep Genesis. Fate can cover voids, so these proofs play the
//! Genesis site onto a covered empty cell and never stand a minion on it first.

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

fn fate() -> Value {
    json!({
        "affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn scout() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    })
}

fn gate(ordinary: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisPayOneManaToSummonToken": "scout",
    });
    if ordinary {
        value["ordinary"] = json!(true);
    }
    value
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32, ordinary_gate: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "fate-genesis-token",
                "ordinary": ordinary_gate,
            }))
            .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": if ordinary_gate {
                "synthetic-fate-genesis-token-ordinary-v1"
            } else {
                "synthetic-fate-genesis-token-v1"
            },
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fate": fate(),
            "north-gate": gate(ordinary_gate),
            "north-open": site(),
            "scout": scout(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-open",
                    "north-gate",
                    "north-open",
                    "north-gate",
                    "north-open",
                    "north-gate",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-fate",
                    "south-dummy",
                    "north-fate",
                    "south-dummy",
                    "north-fate",
                    "south-dummy",
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
    }))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
        });
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

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn opening(ordinary_gate: bool) -> Session {
    (1..=4096)
        .map(|seed| manifest(seed, ordinary_gate))
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Fate Genesis token candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-open".to_owned())
                && atlas.contains(&"north-gate".to_owned())
                && spells.contains(&"north-fate".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with Fate, an open site, and a Genesis site")
}

fn fate_covers_c3_not_c4(descriptor: &Value) -> bool {
    descriptor["kind"] == "cast-aura"
        && descriptor["cardId"] == "north-fate"
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().any(|value| value == "C3"))
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().all(|value| value != "C4"))
}

fn pays_gate_at_c3(descriptor: &Value) -> bool {
    descriptor["kind"] == "play-site"
        && descriptor["cardId"] == "north-gate"
        && descriptor["cell"] == "C3"
        && descriptor["genesisTokenChoice"] == "pay-one-mana"
}

fn declines_gate_at_c3(descriptor: &Value) -> bool {
    descriptor["kind"] == "play-site"
        && descriptor["cardId"] == "north-gate"
        && descriptor["cell"] == "C3"
        && descriptor["genesisTokenChoice"] == "decline"
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn cover_c3_then_offer_gate(ordinary_gate: bool) -> Session {
    let mut session = opening(ordinary_gate);
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-open"
            && descriptor["cell"] == "C4"
    });
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
    accept_where(&mut session, fate_covers_c3_not_c4);
    session
}

#[test]
fn rule_catalog_0333_fate_strips_genesis_paid_token_from_a_non_ordinary_site() {
    let mut session = cover_c3_then_offer_gate(false);
    assert!(
        offers(&session, declines_gate_at_c3),
        "Decline remains legal after Fate Lose"
    );
    assert!(
        !offers(&session, pays_gate_at_c3),
        "Fate Lose strips the printed Genesis paid-token ability"
    );
    accept_where(&mut session, declines_gate_at_c3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0334_ordinary_site_keeps_genesis_paid_token_under_fate() {
    let mut session = cover_c3_then_offer_gate(true);
    assert!(
        offers(&session, pays_gate_at_c3),
        "Fate does not strip printed abilities from an Ordinary Genesis site"
    );
    let (_, paid) = accept_where(&mut session, pays_gate_at_c3);
    assert_eq!(
        paid.events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["site-played", "minion-summoned"]
    );
    assert_exact_replay(&session);
}
