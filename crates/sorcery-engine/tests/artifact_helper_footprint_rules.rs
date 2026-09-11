//! Direct proofs that a tap-pair Artifact helper stands with a 2×2 bearer
//! (RULE-CATALOG-0365–0366).
//!
//! Siege Ballista pays by tapping its bearer and one other ready ally standing
//! with it. Occupancy, not the remembered carried cell, decides "with". A
//! B3-anchored square that carries a Ballista at C4 therefore accepts a helper
//! on B3. A helper on B2, which the square does not occupy, cannot pay.

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

fn earth() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn giant() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 1,
        "manaCost": 0,
        "occupiesSquareArea": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn helper() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn ballista() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": 0,
        "tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps": 3,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "artifact-helper-footprint" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-artifact-helper-footprint-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth(),
            "north-giant": giant(),
            "north-helper": helper(),
            "siege-ballista": ballista(),
            "south-avatar": avatar(),
            "south-plain": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 9],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-giant",
                    "north-helper",
                    "siege-ballista",
                    "north-giant",
                    "north-helper",
                    "siege-ballista",
                    "north-giant",
                    "north-helper",
                    "siege-ballista",
                    "north-giant",
                    "north-helper",
                    "siege-ballista",
                    "north-giant",
                    "north-helper",
                    "siege-ballista",
                    "north-giant",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-plain"; 16],
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
            let current = session.replay_value().expect("replay");
            panic!(
                "expected engine-issued action in phase {} among {:?}",
                current["state"]["phase"],
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

fn play_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn end_and_draw_zone(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn end_and_draw(session: &mut Session) {
    end_and_draw_zone(session, "spellbook");
}

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> Option<&'a Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn realm_artifacts(current: &Value) -> Vec<Value> {
    current["realm"]["artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn artifact_damage_helpers(session: &Session, artifact_instance_id: &str) -> Vec<String> {
    let mut helpers: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "activate-artifact-damage"
                && action.descriptor["artifactInstanceId"] == artifact_instance_id
        })
        .map(|action| {
            action.descriptor["helper"]["instanceId"]
                .as_str()
                .expect("helper identity")
                .to_owned()
        })
        .collect();
    helpers.sort();
    helpers.dedup();
    helpers
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

fn opening() -> Session {
    (1..=4096)
        .map(manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("artifact helper footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 3
                && spells.contains(&"north-giant".to_owned())
                && spells.contains(&"north-helper".to_owned())
                && spells.contains(&"siege-ballista".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with earth, a 2x2 bearer, a helper, and a Ballista")
}

fn establish_square(session: &mut Session) {
    keep(session);
    keep(session);
    play_site(session, "north-earth", "C4");
    end_and_draw(session);
    play_site(session, "south-site", "C1");
    end_and_draw(session);
    play_site(session, "north-earth", "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "north-earth", "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "B3");
}

fn summon_b3_square(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"].is_null()
    });
    let giant_id = summoned["cardInstanceId"]
        .as_str()
        .expect("2x2 identity")
        .to_owned();
    let current = state(session);
    let occupant = realm_unit(&current, &giant_id).expect("2x2 remains in play");
    assert_eq!(occupant["location"], "B3");
    assert_eq!(occupant["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    giant_id
}

fn summon_helper(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-helper"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("helper identity")
        .to_owned()
}

fn conjure_ballista_at_c4(session: &mut Session, bearer_id: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "siege-ballista"
            && descriptor["bearer"]["instanceId"] == bearer_id
            && descriptor["bearerCell"] == "C4"
    });
    let artifacts = realm_artifacts(&state(session));
    artifacts
        .iter()
        .find(|artifact| artifact["bearer"]["instanceId"] == bearer_id)
        .and_then(|artifact| artifact["instanceId"].as_str())
        .expect("the Ballista carried at C4")
        .to_owned()
}

#[test]
fn rule_catalog_0365_square_bearer_accepts_helper_on_occupied_non_carried_cell() {
    let mut session = opening();
    establish_square(&mut session);
    let giant_id = summon_b3_square(&mut session);
    let helper_id = summon_helper(&mut session, "B3");
    let ballista_id = conjure_ballista_at_c4(&mut session, &giant_id);

    let current = state(&session);
    let carried = realm_artifacts(&current)
        .into_iter()
        .find(|artifact| artifact["instanceId"] == ballista_id)
        .expect("carried Ballista");
    assert_eq!(carried["bearerCell"], "C4");
    assert_eq!(
        realm_unit(&current, &helper_id).expect("helper remains")["location"],
        "B3"
    );

    let helpers = artifact_damage_helpers(&session, &ballista_id);
    assert!(
        helpers.contains(&helper_id),
        "a helper occupying B3 stands with a B3-anchored bearer even when the Ballista is remembered at C4, got {helpers:?}"
    );

    let north_avatar = current["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-artifact-damage"
            && descriptor["artifactInstanceId"] == ballista_id.as_str()
            && descriptor["helper"]["instanceId"] == helper_id.as_str()
            && descriptor["target"]["instanceId"] == north_avatar.as_str()
    });
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0366_helper_outside_square_cannot_pay_second_tap() {
    let mut session = opening();
    establish_square(&mut session);
    let giant_id = summon_b3_square(&mut session);
    end_and_draw(&mut session);
    end_and_draw_zone(&mut session, "atlas");
    play_site(&mut session, "north-earth", "B2");
    let helper_id = summon_helper(&mut session, "B2");
    let ballista_id = conjure_ballista_at_c4(&mut session, &giant_id);

    let current = state(&session);
    assert_eq!(
        realm_unit(&current, &helper_id).expect("helper remains")["location"],
        "B2"
    );
    let helpers = artifact_damage_helpers(&session, &ballista_id);
    assert!(
        !helpers.contains(&helper_id),
        "a helper on B2 does not stand with a square occupying B3-B4-C3-C4, got {helpers:?}"
    );
    assert_exact_replay(&session);
}
