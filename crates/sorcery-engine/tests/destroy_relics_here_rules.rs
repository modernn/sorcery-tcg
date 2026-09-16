//! Direct proofs for destroy-artifacts-and-auras-at-location-within-two-steps
//! Magic (RULE-CATALOG-0569–0570).
//!
//! Ordinary Magic chooses a location within two measured steps of the caster
//! and destroys every Artifact whose cell and region match, plus every Aura
//! occupying that cell. Tokens banish. An empty offered location is a paid
//! no-op. This is one composed fact, not destroy-target-artifact plus
//! destroy-target-aura.

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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
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

fn unmake() -> Value {
    json!({
        "cardType": "magic",
        "destroyArtifactsAndAurasAtLocationWithinTwoSteps": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn destroy_relics_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-relics-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-relics-here-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-relic": relic(),
            "north-unmake": unmake(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-relic": relic(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-relic",
                    "north-relic",
                    "north-relic",
                    "north-unmake",
                    "north-unmake",
                    "north-unmake",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-relic",
                    "south-relic",
                    "south-relic",
                    "south-dummy",
                    "south-dummy",
                    "south-dummy",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    try_accept_where(session, predicate).expect("expected engine-issued action")
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid destroy-relics-here session");
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

fn seed_with(required: &[&str]) -> String {
    (569..569 + 256)
        .map(destroy_relics_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn south_casts_relic_at_c1(session: &mut Session) -> String {
    south_plays_c1(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C1"
    });
    let relic_id = artifact_at(session, "south-relic", "C1");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    relic_id
}

fn south_ends_without_relic(session: &mut Session) {
    south_plays_c1(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn unmake_locations(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("unmake actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-unmake"
        })
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
}

fn artifact_at(session: &Session, card_id: &str, cell: &str) -> String {
    state(session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["cardId"] == card_id && artifact["location"] == cell)
        .expect("expected artifact at cell")["instanceId"]
        .as_str()
        .expect("artifact instance identity")
        .to_owned()
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

#[test]
fn rule_catalog_0569_destroy_relics_here_destroys_a_nearby_artifact() {
    let encoded = seed_with(&["north-unmake", "north-relic"]);
    let mut session = opening_main(&encoded);
    let far_id = south_casts_relic_at_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let near_id = artifact_at(&session, "north-relic", "C3");
    let before = state(&session);
    assert_eq!(
        realm_artifact(&before, &near_id).expect("near relic")["location"],
        "C3"
    );
    assert_eq!(
        realm_artifact(&before, &far_id).expect("far relic")["location"],
        "C1"
    );
    assert_eq!(unmake_locations(&session), ["C3", "C4"]);

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unmake"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    let types = event_types(&receipt);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(types.contains(&"artifact-destroyed"));
    assert!(!types.contains(&"artifact-banished"));
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "artifact-destroyed")
        .expect("artifact destruction");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], near_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert!(realm_artifact(&after, &near_id).is_none());
    assert_eq!(
        realm_artifact(&after, &far_id).expect("far relic remains")["location"],
        "C1"
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == near_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0570_destroy_relics_here_empty_location_is_a_paid_noop() {
    let encoded = seed_with(&["north-unmake"]);
    let mut session = opening_main(&encoded);
    south_ends_without_relic(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(unmake_locations(&session), ["C3", "C4"]);
    assert!(
        state(&session)["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unmake"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    let types = event_types(&receipt);
    assert_eq!(types, ["magic-cast", "magic-resolved"]);
    assert!(
        state(&session)["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}
