//! Direct proofs for official Monument carry prohibition (RULE-CATALOG-0314–0315).
//!
//! Monuments are Artifacts that stay on a site. They cannot be conjured onto a
//! unit and cannot be picked up. Ordinary Artifacts on the same square remain
//! carryable.

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

fn monument() -> Value {
    json!({
        "atEndOfControllerTurnUntapNearbyAllies": true,
        "cannotBeCarried": true,
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn relic() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
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

fn monument_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "monument-uncarried" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-monument-uncarried-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-monument": monument(),
            "north-relic": relic(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-monument",
                    "north-monument",
                    "north-monument",
                    "north-relic",
                    "north-relic",
                    "north-relic"
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

fn opening_manifest() -> String {
    (1..=4096)
        .map(monument_manifest)
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("Monument candidate"));
            let hand = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("north opening spellbook");
            hand.iter().any(|card| card["cardId"] == "north-monument")
                && hand.iter().any(|card| card["cardId"] == "north-relic")
        })
        .expect("bounded seed opening with a Monument and a carryable Artifact")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let legal = session.legal_actions().expect("legal actions");
    let action = legal
        .iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action; legal={:?}",
                legal
                    .iter()
                    .map(|action| action.descriptor["kind"].clone())
                    .collect::<Vec<_>>()
            )
        })
        .clone();
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

fn artifact_id(session: &Session, card_id: &str) -> Value {
    state(session)["realm"]["artifacts"]
        .as_array()
        .expect("artifacts")
        .iter()
        .find(|artifact| artifact["cardId"] == card_id)
        .expect("expected artifact")["instanceId"]
        .clone()
}

fn legal_descriptors(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .map(|action| action.descriptor)
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

fn setup_site(session: &mut Session) {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
}

#[test]
fn rule_catalog_0314_a_monument_cannot_be_cast_onto_a_unit_or_picked_up() {
    let mut session = Session::new(&opening_manifest()).expect("valid Monument session");
    setup_site(&mut session);
    let casts: Vec<_> = legal_descriptors(&session)
        .into_iter()
        .filter(|descriptor| {
            descriptor["kind"] == "cast-artifact" && descriptor["cardId"] == "north-monument"
        })
        .collect();
    assert!(
        !casts.is_empty(),
        "the Monument can still be conjured onto a site"
    );
    assert!(
        casts
            .iter()
            .all(|descriptor| descriptor["cell"] == "C4" && descriptor.get("bearer").is_none()),
        "a Monument is not offered onto a unit: {casts:?}"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-monument"
            && descriptor["cell"] == "C4"
    });
    let monument_id = artifact_id(&session, "north-monument");
    assert!(
        legal_descriptors(&session).iter().all(|descriptor| {
            descriptor["kind"] != "pick-up-artifacts"
                || !descriptor["artifactInstanceIds"]
                    .as_array()
                    .expect("pickup ids")
                    .contains(&monument_id)
        }),
        "a Monument cannot be picked up"
    );
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0315_a_carryable_artifact_on_the_same_square_can_still_be_picked_up() {
    let mut session = Session::new(&opening_manifest()).expect("valid Monument session");
    setup_site(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-monument"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["cell"] == "C4"
    });
    let relic_id = artifact_id(&session, "north-relic");
    let monument_id = artifact_id(&session, "north-monument");
    let pickups: Vec<_> = legal_descriptors(&session)
        .into_iter()
        .filter(|descriptor| descriptor["kind"] == "pick-up-artifacts")
        .collect();
    assert!(
        pickups
            .iter()
            .any(|descriptor| { descriptor["artifactInstanceIds"] == json!([relic_id]) }),
        "the carryable Artifact is still offered: {pickups:?}"
    );
    assert!(
        pickups.iter().all(|descriptor| {
            !descriptor["artifactInstanceIds"]
                .as_array()
                .expect("pickup ids")
                .contains(&monument_id)
        }),
        "the Monument stays out of every Pick Up combination: {pickups:?}"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["artifactInstanceIds"] == json!([relic_id])
    });
    let after = state(&session);
    let relics = after["realm"]["artifacts"].as_array().expect("artifacts");
    let carried = relics
        .iter()
        .find(|artifact| artifact["instanceId"] == relic_id)
        .expect("carried relic");
    assert!(carried.get("bearer").is_some(), "the relic is carried");
    let monument = relics
        .iter()
        .find(|artifact| artifact["instanceId"] == monument_id)
        .expect("loose monument");
    assert!(monument.get("bearer").is_none(), "the Monument stays loose");
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}
