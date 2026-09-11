//! Direct proofs that returning a site banishes 2×2 occupants of that cell
//! (RULE-CATALOG-0347–0348).
//!
//! Return-site remaps occupants into the void so they banish instead of dying
//! as if stranded. Settlement already treats any occupied cell as the
//! footprint. The remap must use that same occupancy, not only the anchor
//! cell, so a B3-anchored square on C3 banishes when C3 is returned.

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
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "occupiesSquareArea": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bounce() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetSiteToOwnerHand": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn footprint_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "return-site-footprint" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-site-footprint-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth(),
            "north-giant": giant(),
            "south-avatar": avatar(),
            "south-bounce": bounce(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-giant"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-bounce"; 8],
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

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
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

fn opening_ids(session: &Session, seat: &str, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"][seat]["hand"][zone]
        .as_array()
        .expect("hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
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

fn cemetery_has(current: &Value, instance_id: &str) -> bool {
    ["north", "south"].into_iter().any(|seat| {
        current["players"][seat]["cemetery"]
            .as_array()
            .expect("cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id)
    })
}

fn realm_has_unit(current: &Value, instance_id: &str) -> bool {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .any(|unit| unit["instanceId"] == instance_id)
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

fn footprint_opening() -> Session {
    (1..=4096)
        .map(footprint_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("return-site footprint candidate");
            let atlas = opening_ids(&session, "north", "atlas");
            let spells = opening_ids(&session, "north", "spellbook");
            let south = opening_ids(&session, "south", "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 3
                && spells.contains(&"north-giant".to_owned())
                && south.contains(&"south-bounce".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with earth, a 2x2 burrower, and return-site")
}

fn establish_north_square(session: &mut Session) {
    keep(session);
    keep(session);
    play_site(session, "C4");
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);
    play_site(session, "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "B3");
}

fn summon_b3_then_south_ready(session: &mut Session, region: &str) -> String {
    establish_north_square(session);
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"] == region
    });
    let giant_id = summoned["cardInstanceId"]
        .as_str()
        .expect("2x2 identity")
        .to_owned();
    let occupant = state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == giant_id)
        .expect("2x2 remains in play");
    assert_eq!(occupant["location"], "B3");
    assert_eq!(occupant["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(occupant["region"], region);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    giant_id
}

fn return_c3(session: &mut Session) -> Receipt {
    let site_instance = state(session)["realm"]["sites"]["C3"]["instanceId"]
        .as_str()
        .expect("C3 site identity")
        .to_owned();
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-bounce"
            && descriptor["targetLocation"]["cell"] == "C3"
            && descriptor["targetSiteInstanceId"] == site_instance
    });
    receipt
}

fn assert_banished_not_killed(session: &Session, receipt: &Receipt, giant_id: &str) {
    assert_eq!(
        event_types(receipt),
        [
            "magic-cast",
            "site-returned-to-hand",
            "minion-banished",
            "magic-resolved"
        ]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died"
                || event.event_type == "rubble-created"
                || event.event_type == "site-destroyed")
    );
    let current = state(session);
    assert!(current["realm"]["sites"].get("C3").is_none());
    assert_eq!(current["realm"]["sites"]["B3"]["cardId"], "north-earth");
    assert!(!realm_has_unit(&current, giant_id));
    assert!(!cemetery_has(&current, giant_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_0347_returning_non_anchor_site_banishes_surface_square() {
    let mut session = footprint_opening();
    let giant_id = summon_b3_then_south_ready(&mut session, "surface");
    let receipt = return_c3(&mut session);
    assert_banished_not_killed(&session, &receipt, &giant_id);
}

#[test]
fn rule_catalog_0348_returning_non_anchor_site_banishes_underground_square() {
    let mut session = footprint_opening();
    let giant_id = summon_b3_then_south_ready(&mut session, "underground");
    let receipt = return_c3(&mut session);
    assert_banished_not_killed(&session, &receipt, &giant_id);
}
