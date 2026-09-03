//! Direct proof for conjured Aura (RULE-CATALOG-0064): canonical two-by-two areas, grounded site
//! minions, Avatar freedom, and third-controller-turn dispel.

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn base_manifest(seed: u32) -> Value {
    json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "aura-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-aura-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": vec![
                    "spell-0", "spell-1", "spell-2", "spell-3",
                    "spell-4", "spell-5", "spell-6", "spell-7",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["spell-8"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    })
}

fn manifest(seed: u32) -> String {
    let mut base = base_manifest(seed);
    for spell in 0..=8 {
        base["cards"][format!("spell-{spell}")] = minion(json!({}));
    }
    let preview = Session::new(
        &canonical_json(&{
            let mut value = base.clone();
            value["manifestId"] = json!(identity_hash(&value).expect("preview identity"));
            value
        })
        .expect("preview manifest"),
    )
    .expect("preview session");
    let preview_state = preview.replay_value().expect("preview state");
    let hand = preview_state["state"]["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north opening spellbook");
    let [aura_id, flyer_id, burrower_id]: [&str; 3] = hand
        .iter()
        .map(|card| card["cardId"].as_str().expect("opening card id"))
        .collect::<Vec<_>>()
        .try_into()
        .expect("three opening spellbook cards");
    let voidwalk_id = preview_state["state"]["players"]["south"]["hand"]["spellbook"][0]["cardId"]
        .as_str()
        .expect("south opening spell");
    base["cards"][aura_id] = json!({
        "cardType": "aura",
        "immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    base["cards"][flyer_id] = minion(json!({ "airborne": true }));
    base["cards"][burrower_id] = minion(json!({ "burrowing": true }));
    base["cards"][voidwalk_id] = minion(json!({ "voidwalk": true }));
    base["manifestId"] = json!(identity_hash(&base).expect("manifest identity"));
    canonical_json(&base).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let offered = session.legal_actions().expect("legal actions");
    let action = offered
        .iter()
        .find(|action| predicate(&action.descriptor))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action; offered {:?}",
                offered
                    .iter()
                    .map(|action| action.descriptor.to_string())
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

fn draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn end_turn(session: &mut Session) -> Receipt {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn").1
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn descriptors_of_kind(session: &Session, kind: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .map(|action| action.descriptor)
        .filter(|descriptor| descriptor["kind"] == kind)
        .collect()
}

fn move_destinations(session: &Session, instance_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .map(|action| {
            format!(
                "{}/{}",
                action.descriptor["to"]["cell"].as_str().expect("cell"),
                action.descriptor["to"]["region"]
                    .as_str()
                    .unwrap_or("surface")
            )
        })
        .collect()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
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

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one replayed scenario proves canonical areas, grounding, dispel, and replay"
)]
fn rule_catalog_0064_an_aura_should_hold_site_minions_across_a_canonical_area_for_three_turns() {
    let manifest_json = manifest(247);
    let preview = Session::new(&manifest_json).expect("preview session");
    let preview_state = preview.replay_value().expect("preview state");
    let hand = preview_state["state"]["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north opening spellbook");
    let aura_card_id = hand[0]["cardId"].as_str().expect("aura card id");
    let flyer_card_id = hand[1]["cardId"].as_str().expect("flyer card id");
    let burrower_card_id = hand[2]["cardId"].as_str().expect("burrower card id");
    let voidwalk_card_id =
        preview_state["state"]["players"]["south"]["hand"]["spellbook"][0]["cardId"]
            .as_str()
            .expect("voidwalk card id");

    let mut session = Session::new(&manifest_json).expect("valid aura scenario");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C4"
    });
    let (flyer_cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == flyer_card_id
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let (burrower_cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == burrower_card_id
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underground"
    });
    let flyer = flyer_cast["cardInstanceId"]
        .as_str()
        .expect("flyer identity");
    let burrower = burrower_cast["cardInstanceId"]
        .as_str()
        .expect("burrower identity");

    let mut aura_areas: Vec<_> = descriptors_of_kind(&session, "cast-aura")
        .iter()
        .map(|descriptor| descriptor["cells"].clone())
        .collect();
    aura_areas.sort_by(|left, right| {
        canonical_json(left)
            .expect("cells")
            .cmp(&canonical_json(right).expect("cells"))
    });
    aura_areas.dedup();
    assert_eq!(
        aura_areas,
        vec![
            json!(["A1", "A2", "B1", "B2"]),
            json!(["A2", "A3", "B2", "B3"]),
            json!(["A3", "A4", "B3", "B4"]),
            json!(["B1", "B2", "C1", "C2"]),
            json!(["B2", "B3", "C2", "C3"]),
            json!(["B3", "B4", "C3", "C4"]),
            json!(["C1", "C2", "D1", "D2"]),
            json!(["C2", "C3", "D2", "D3"]),
            json!(["C3", "C4", "D3", "D4"]),
            json!(["D1", "D2", "E1", "E2"]),
            json!(["D2", "D3", "E2", "E3"]),
            json!(["D3", "D4", "E3", "E4"]),
        ],
        "an Aura may occupy any canonical two-by-two area of the realm"
    );

    let (cast, cast_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == aura_card_id
            && descriptor["cells"] == json!(["B3", "B4", "C3", "C4"])
    });
    let aura = cast["cardInstanceId"].as_str().expect("aura identity");
    assert!(event_types(&cast_receipt).contains(&"aura-conjured"));
    let realm = state(&session)["realm"].clone();
    assert_eq!(
        realm["auras"],
        json!([{
            "cardId": aura_card_id,
            "cells": ["B3", "B4", "C3", "C4"],
            "controller": "north",
            "instanceId": aura,
            "owner": "north",
            "turnCounters": 0,
        }])
    );
    assert_eq!(
        realm["immobileAreas"],
        json!([{
            "cells": ["B3", "B4", "C3", "C4"],
            "minionsAtSitesOnly": true,
            "sourceInstanceId": aura,
            "suppressesAirborne": true,
        }])
    );

    end_turn(&mut session);
    assert_eq!(
        state(&session)["realm"]["auras"][0]["turnCounters"],
        json!(1)
    );

    draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    let (voidwalk_cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == voidwalk_card_id
            && descriptor["cell"] == "B3"
            && descriptor["region"] == "void"
    });
    let voidwalk = voidwalk_cast["cardInstanceId"]
        .as_str()
        .expect("voidwalk identity");
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == voidwalk)
            .expect("voidwalk on board")["location"],
        "B3",
        "void minions inside an Aura area are not grounded"
    );

    end_turn(&mut session);
    draw(&mut session, "spellbook");
    let flyer_moves = move_destinations(&session, flyer);
    assert_eq!(
        flyer_moves,
        ["C4/surface"],
        "a held minion keeps only its stand-and-fight option"
    );
    assert_eq!(
        move_destinations(&session, burrower),
        ["C4/underground"],
        "the area holds the minions standing on its sites in every region it covers"
    );
    end_turn(&mut session);
    assert_eq!(
        state(&session)["realm"]["auras"][0]["turnCounters"],
        json!(2)
    );

    draw(&mut session, "spellbook");
    end_turn(&mut session);
    draw(&mut session, "spellbook");
    let dispel = end_turn(&mut session);
    assert_eq!(
        event_types(&dispel)
            .into_iter()
            .filter(|event| event.starts_with("aura-"))
            .collect::<Vec<_>>(),
        ["aura-turn-counted", "aura-dispelled"]
    );

    let settled = state(&session);
    assert_eq!(settled["realm"]["auras"], Value::Null);
    assert_eq!(settled["realm"]["immobileAreas"], Value::Null);
    assert!(
        settled["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == aura),
        "a dispelled Aura is laid to rest in its owner's cemetery"
    );
    assert_exact_replay(&session);
}
