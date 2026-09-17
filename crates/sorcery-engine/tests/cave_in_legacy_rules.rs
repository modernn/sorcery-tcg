//! Direct proofs for Cave-In Artifact occupancy
//! (RULE-CATALOG-0677–0678, RULE-CATALOG-0740).
//!
//! These slices complement `cave_in_rules.rs` 0587–0588, which prove minion
//! burrows at a land site and that water-only sites are unoffered. Cave-In
//! also burrows a loose Artifact at the chosen land site, detaches an
//! Avatar-carried Artifact there while the Avatar stays on the surface, and
//! keeps a minion-carried Artifact attached when the bearer burrows.

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

fn burrower() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn cave_in() -> Value {
    json!({
        "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
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

fn cave_in_artifact_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-legacy-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-legacy-artifact-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-site": earth_site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 6],
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

fn cave_in_minion_carrier_manifest(seed: u32) -> String {
    let mut south_spellbook = vec!["south-minion"; 8];
    south_spellbook.extend(vec!["south-artifact"; 4]);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-legacy-minion-carrier" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-legacy-minion-carrier-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-site": earth_site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-minion": burrower(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
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
    let mut session = Session::new(encoded).expect("valid Cave-In Artifact session");
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

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
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
        .map(cave_in_artifact_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-cave-in")
        })
        .expect("bounded seed with Cave-In in the opening hand")
}

fn minion_carrier_seed_with(start: u32) -> String {
    (start..start + 1024)
        .map(cave_in_minion_carrier_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-cave-in")
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .any(|card| card == "south-minion")
        })
        .expect("bounded seed with Cave-In and burrowing minion in opening hands")
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

fn cast_cave_in_at_c1(session: &mut Session) -> (Value, Receipt) {
    let land_site_id = state(session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("Land Site identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == land_site_id
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
fn rule_catalog_0677_cave_in_burrows_a_loose_artifact_at_the_land_site() {
    let encoded = seed_with(677);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session, false);
    let before = realm_artifact(&state(&session), &artifact_id).clone();
    assert_eq!(before["location"], "C1");
    assert_eq!(before["region"], "surface");
    assert!(before.get("bearer").is_none());

    let (cast, receipt) = cast_cave_in_at_c1(&mut session);
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
fn rule_catalog_0678_cave_in_detaches_an_avatar_carried_artifact_and_leaves_the_avatar() {
    let encoded = seed_with(678);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session, true);
    let before = realm_artifact(&state(&session), &artifact_id).clone();
    assert_eq!(before["bearer"]["kind"], "avatar");
    assert_eq!(before["bearer"]["seat"], "south");
    assert_eq!(
        state(&session)["players"]["south"]["avatar"]["region"],
        "surface"
    );

    let (cast, receipt) = cast_cave_in_at_c1(&mut session);
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn stage_south_minion_carried_artifact(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("burrowing minion identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"]["kind"] == "minion"
            && descriptor["bearer"]["instanceId"] == minion_id
    });
    let artifact_id = state(session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (minion_id, artifact_id)
}

#[test]
fn rule_catalog_0740_cave_in_keeps_minion_carried_artifact_attached_while_burrowing() {
    let encoded = minion_carrier_seed_with(740);
    let mut session = opening_main(&encoded);
    let (minion_id, artifact_id) = stage_south_minion_carried_artifact(&mut session);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &minion_id)["region"], "surface");
    assert_eq!(
        realm_artifact(&before, &artifact_id)["bearer"]["instanceId"],
        minion_id
    );

    let (cast, receipt) = cast_cave_in_at_c1(&mut session);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-burrowed",
            "artifact-burrowed",
            "magic-resolved"
        ]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": minion_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after = state(&session);
    assert_eq!(realm_unit(&after, &minion_id)["region"], "underground");
    let carried = realm_artifact(&after, &artifact_id);
    assert_eq!(carried["bearer"]["kind"], "minion");
    assert_eq!(carried["bearer"]["instanceId"], minion_id);
    assert!(carried.get("location").is_none());
    assert_exact_replay(&session);
}
