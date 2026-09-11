//! Direct proofs that Secret Tunnel hops from a 2×2 that occupies the tunnel
//! (RULE-CATALOG-0357–0358).
//!
//! A tunnel reaches every other controlled site. Occupancy, not the anchor
//! cell, decides whether a 2×2 is at that tunnel. A B3-anchored square that
//! occupies C4 hops to a far controlled land. The same square with no tunnel
//! under it cannot.

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

fn tunnel() -> Value {
    json!({
        "cardType": "site",
        "connectsBurrowedAllies": true,
        "elements": ["earth"],
    })
}

fn giant() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "movementBonus": 1,
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

fn tunnel_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "secret-tunnel-footprint" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-secret-tunnel-footprint-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth(),
            "north-giant": giant(),
            "north-tunnel": tunnel(),
            "south-avatar": avatar(),
            "south-plain": dummy(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-tunnel",
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
            "contentHash": identity_hash(&json!({ "fixture": "secret-tunnel-footprint-land" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-secret-tunnel-footprint-land-v1",
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

fn tunnel_opening() -> Session {
    (1..=4096)
        .map(tunnel_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Secret Tunnel footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.contains(&"north-tunnel".to_owned())
                && atlas.iter().filter(|card| *card == "north-earth").count() >= 2
                && spells.contains(&"north-giant".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with a tunnel, earth, and a 2x2 burrower")
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
        .expect("bounded seed opening with earth and a 2x2 burrower")
}

fn establish_square(session: &mut Session, c4_tunnel: bool) {
    keep(session);
    keep(session);
    if c4_tunnel {
        play_site(session, "north-tunnel", "C4");
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

fn expand_hop_landing(session: &mut Session) {
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "C2");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "D3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "D2");
}

fn summon_b3_underground(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"] == "underground"
    });
    let giant_id = summoned["cardInstanceId"]
        .as_str()
        .expect("2x2 identity")
        .to_owned();
    let current = state(session);
    let occupant = realm_unit(&current, &giant_id).expect("2x2 remains in play");
    assert_eq!(occupant["location"], "B3");
    assert_eq!(occupant["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(occupant["region"], "underground");
    giant_id
}

#[test]
fn rule_catalog_0357_secret_tunnel_hops_square_occupying_non_anchor_tunnel() {
    let mut session = tunnel_opening();
    establish_square(&mut session, true);
    let giant_id = summon_b3_underground(&mut session);
    expand_hop_landing(&mut session);

    let paths = unit_paths(&session, &giant_id);
    assert!(
        paths.contains(&"B3/underground,C2/underground".to_owned()),
        "occupying the C4 tunnel must hop to far controlled land C2, got {paths:?}"
    );
    assert!(
        !paths.contains(&"B3/underground,C1/underground".to_owned()),
        "a controller's tunnel must not hop onto an opponent site"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == giant_id.as_str()
            && path_locations(descriptor) == "B3/underground,C2/underground"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let current = state(&session);
    let occupant = realm_unit(&current, &giant_id).expect("2x2 hopped");
    assert_eq!(occupant["location"], "C2");
    assert_eq!(occupant["occupiedCells"], json!(["C2", "C3", "D2", "D3"]));
    assert_eq!(occupant["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0358_square_without_occupied_tunnel_does_not_hop_to_far_land() {
    let mut session = land_opening();
    establish_square(&mut session, false);
    let giant_id = summon_b3_underground(&mut session);
    expand_hop_landing(&mut session);

    let paths = unit_paths(&session, &giant_id);
    assert!(
        !paths.contains(&"B3/underground,C2/underground".to_owned()),
        "a 2x2 that occupies no tunnel must not hop to far land, got {paths:?}"
    );
    assert_exact_replay(&session);
}
