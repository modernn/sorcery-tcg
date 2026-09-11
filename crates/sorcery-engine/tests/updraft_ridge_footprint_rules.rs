//! Direct proofs that Updraft Ridge frees a 2×2 that occupies the Ridge
//! (RULE-CATALOG-0361–0362).
//!
//! An Airborne minion atop a Ridge takes its first departure at cost zero.
//! Occupancy, not the path anchor, decides "atop". A B3-anchored square that
//! occupies a Ridge at C4 can therefore take two surface steps. The same
//! square with no Ridge under it cannot.

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

fn ridge() -> Value {
    json!({
        "airborneMinionsAtopMoveFreelyAway": true,
        "cardType": "site",
        "elements": ["air"],
    })
}

fn giant() -> Value {
    json!({
        "airborne": true,
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "occupiesSquareArea": 2,
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

fn ridge_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "updraft-ridge-footprint" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-updraft-ridge-footprint-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth(),
            "north-giant": giant(),
            "north-ridge": ridge(),
            "south-avatar": avatar(),
            "south-plain": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-ridge",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": vec!["north-giant"; 16],
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

fn land_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "updraft-ridge-footprint-land" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-updraft-ridge-footprint-land-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth(),
            "north-giant": giant(),
            "south-avatar": avatar(),
            "south-plain": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 9],
                "avatar": "north-avatar",
                "spellbook": vec!["north-giant"; 16],
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

fn path_locations(descriptor: &Value) -> String {
    descriptor["path"]
        .as_array()
        .expect("movement path")
        .iter()
        .map(|location| {
            format!(
                "{}/{}",
                location["cell"].as_str().expect("path cell"),
                location["region"].as_str().expect("path region")
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn unit_paths(session: &Session, instance_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .map(|action| path_locations(&action.descriptor))
        .collect()
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> Option<&'a Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
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

fn ridge_opening() -> Session {
    (1..=4096)
        .map(ridge_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Updraft Ridge footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-ridge".to_owned())
                && atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && spells.contains(&"north-giant".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with a Ridge, earth, and a 2x2 Airborne")
}

fn land_opening() -> Session {
    (1..=4096)
        .map(land_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("land footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 3
                && spells.contains(&"north-giant".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with earth and a 2x2 Airborne")
}

fn establish_square(session: &mut Session, c4_ridge: bool) {
    keep(session);
    keep(session);
    if c4_ridge {
        play_site(session, "north-ridge", "C4");
    } else {
        play_site(session, "north-earth", "C4");
    }
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

fn expand_two_step_landing(session: &mut Session) {
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "B2");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "C2");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "B1");
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

#[test]
fn rule_catalog_0361_updraft_ridge_frees_square_occupying_non_anchor_ridge() {
    let mut session = ridge_opening();
    establish_square(&mut session, true);
    let giant_id = summon_b3_square(&mut session);
    expand_two_step_landing(&mut session);

    let paths = unit_paths(&session, &giant_id);
    assert!(
        paths.contains(&"B3/surface,B2/surface,B1/surface".to_owned()),
        "occupying the C4 Ridge must free a two-step departure to B1, got {paths:?}"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == giant_id.as_str()
            && path_locations(descriptor) == "B3/surface,B2/surface,B1/surface"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let current = state(&session);
    let occupant = realm_unit(&current, &giant_id).expect("2x2 departed");
    assert_eq!(occupant["location"], "B1");
    assert_eq!(occupant["occupiedCells"], json!(["B1", "B2", "C1", "C2"]));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0362_square_without_occupied_ridge_cannot_take_two_steps() {
    let mut session = land_opening();
    establish_square(&mut session, false);
    let giant_id = summon_b3_square(&mut session);
    expand_two_step_landing(&mut session);

    let paths = unit_paths(&session, &giant_id);
    assert!(
        !paths.contains(&"B3/surface,B2/surface,B1/surface".to_owned()),
        "a 2x2 that occupies no Ridge must not take two paid steps, got {paths:?}"
    );
    assert!(
        paths.contains(&"B3/surface,B2/surface".to_owned()),
        "ordinary one-step departure from B3 must remain legal, got {paths:?}"
    );
    assert_exact_replay(&session);
}
