//! Direct proofs that a tap-pair Artifact measures range from a 2×2 bearer's
//! occupied cells (RULE-CATALOG-0367–0368).
//!
//! Siege Ballista shoots a unit within two measured steps of the cells its
//! bearer stands on. Occupancy, not the remembered carried cell, decides those
//! origins. A B3-anchored square carrying a Ballista at B3 therefore reaches
//! D4, which is three steps from B3 and one step from C4. E3 is three steps
//! from every occupied cell along existing surface and stays out of range.

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

fn far_target() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "summonToAnySite": true,
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
            "contentHash": identity_hash(&json!({ "fixture": "artifact-range-footprint" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-artifact-range-footprint-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth(),
            "north-giant": giant(),
            "north-helper": helper(),
            "siege-ballista": ballista(),
            "south-avatar": avatar(),
            "south-far": far_target(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 12],
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
                "spellbook": vec!["south-far"; 16],
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

fn artifact_damage_targets(session: &Session, artifact_instance_id: &str) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "activate-artifact-damage"
                && action.descriptor["artifactInstanceId"] == artifact_instance_id
        })
        .map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .expect("target identity")
                .to_owned()
        })
        .collect();
    targets.sort();
    targets.dedup();
    targets
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
            let session = Session::new(&candidate).expect("artifact range footprint candidate");
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

fn play_east_site(session: &mut Session, cell: &str) {
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", cell);
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

fn summon_helper_on_b3(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-helper"
            && descriptor["cell"] == "B3"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("helper identity")
        .to_owned()
}

fn conjure_ballista_at_b3(session: &mut Session, bearer_id: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "siege-ballista"
            && descriptor["bearer"]["instanceId"] == bearer_id
            && descriptor["bearerCell"] == "B3"
    });
    let artifacts = realm_artifacts(&state(session));
    artifacts
        .iter()
        .find(|artifact| artifact["bearer"]["instanceId"] == bearer_id)
        .and_then(|artifact| artifact["instanceId"].as_str())
        .expect("the Ballista carried at B3")
        .to_owned()
}

fn south_summon_at(session: &mut Session, cell: &str) -> String {
    end_and_draw(session);
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("far target identity")
        .to_owned();
    end_and_draw(session);
    target_id
}

fn ready_square_ballista(session: &mut Session) -> (String, String) {
    let giant_id = summon_b3_square(session);
    let helper_id = summon_helper_on_b3(session);
    let ballista_id = conjure_ballista_at_b3(session, &giant_id);
    let current = state(session);
    let carried = realm_artifacts(&current)
        .into_iter()
        .find(|artifact| artifact["instanceId"] == ballista_id)
        .expect("carried Ballista");
    assert_eq!(carried["bearerCell"], "B3");
    assert_eq!(
        realm_unit(&current, &helper_id).expect("helper remains")["location"],
        "B3"
    );
    (ballista_id, helper_id)
}

#[test]
fn rule_catalog_0367_square_bearer_reaches_target_two_steps_from_occupied_cell() {
    let mut session = opening();
    establish_square(&mut session);
    play_east_site(&mut session, "D4");
    let target_id = south_summon_at(&mut session, "D4");
    let (ballista_id, helper_id) = ready_square_ballista(&mut session);

    let current = state(&session);
    assert_eq!(
        realm_unit(&current, &target_id).expect("far target remains")["location"],
        "D4"
    );
    let targets = artifact_damage_targets(&session, &ballista_id);
    assert!(
        targets.contains(&target_id),
        "D4 is one step from occupied C4, so a Ballista remembered at B3 must reach it, got {targets:?}"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-artifact-damage"
            && descriptor["artifactInstanceId"] == ballista_id.as_str()
            && descriptor["helper"]["instanceId"] == helper_id.as_str()
            && descriptor["target"]["instanceId"] == target_id.as_str()
    });
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0368_square_bearer_cannot_reach_target_three_steps_from_every_cell() {
    let mut session = opening();
    establish_square(&mut session);
    play_east_site(&mut session, "D4");
    play_east_site(&mut session, "E4");
    play_east_site(&mut session, "E3");
    let target_id = south_summon_at(&mut session, "E3");
    let (ballista_id, _) = ready_square_ballista(&mut session);

    let current = state(&session);
    assert_eq!(
        realm_unit(&current, &target_id).expect("far target remains")["location"],
        "E3"
    );
    let targets = artifact_damage_targets(&session, &ballista_id);
    assert!(
        !targets.contains(&target_id),
        "E3 is three measured steps from every occupied cell of the B3 square, got {targets:?}"
    );
    assert_exact_replay(&session);
}
