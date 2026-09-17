//! Direct proofs for burrow-target-minion-or-artifact Magic occupancy of
//! Artifacts (RULE-CATALOG-0679–0680).
//!
//! These slices complement `bury_magic_rules.rs` 0655–0656, which prove an
//! ordinary minion dies after a forceful burrow or stays put on Water, and
//! `cave_in_rules.rs` 0587–0588, which prove minion burrows at a land site.
//! Targeted Bury also burrows a chosen loose Artifact, and detaches an
//! Avatar-carried Artifact while the Avatar stays on the surface. Area
//! Cave-In Artifact occupancy is 0677–0678, not these targeted casts.

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

fn artifact() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bury() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn bury_artifact_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "burrow-legacy-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-burrow-legacy-artifact-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-site": earth_site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bury"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-artifact"; 6],
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
    let mut session = Session::new(encoded).expect("valid Bury Artifact session");
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

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(bury_artifact_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-bury")
        })
        .expect("bounded seed with Bury in the opening hand")
}

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
        .expect("expected realm artifact")
}

fn stage_south_artifact(session: &mut Session, carried: bool) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && if carried {
                descriptor["bearer"]["kind"] == "avatar"
            } else {
                descriptor["bearer"].is_null() && descriptor["cell"] == "C1"
            }
    });
    let artifact_id = state(session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    artifact_id
}

fn cast_bury_on_artifact(session: &mut Session, artifact_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["targetArtifactInstanceId"] == artifact_id
    })
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
fn rule_catalog_0679_bury_burrows_a_loose_artifact() {
    let encoded = seed_with(679);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session, false);
    let before = realm_artifact(&state(&session), &artifact_id).clone();
    assert_eq!(before["location"], "C1");
    assert_eq!(before["region"], "surface");
    assert!(before.get("bearer").is_none());

    let (cast, receipt) = cast_bury_on_artifact(&mut session, &artifact_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-burrowed", "magic-resolved"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": artifact_id,
            "owner": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after_state = state(&session);
    let after = realm_artifact(&after_state, &artifact_id);
    assert_eq!(after["location"], before["location"]);
    assert_eq!(after["region"], "underground");
    assert!(after.get("bearer").is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0680_bury_detaches_an_avatar_carried_artifact_and_leaves_the_avatar() {
    let encoded = seed_with(680);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session, true);
    let before = realm_artifact(&state(&session), &artifact_id).clone();
    assert_eq!(before["bearer"]["kind"], "avatar");
    assert_eq!(before["bearer"]["seat"], "south");
    assert_eq!(
        state(&session)["players"]["south"]["avatar"]["region"],
        "surface"
    );

    let (cast, receipt) = cast_bury_on_artifact(&mut session, &artifact_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-burrowed", "magic-resolved"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": artifact_id,
            "owner": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after = state(&session);
    let moved = realm_artifact(&after, &artifact_id);
    assert_eq!(moved["location"], "C1");
    assert_eq!(moved["owner"], "south");
    assert_eq!(moved["region"], "underground");
    assert!(moved.get("bearer").is_none());
    assert_eq!(after["players"]["south"]["avatar"]["region"], "surface");
    assert_exact_replay(&session);
}
