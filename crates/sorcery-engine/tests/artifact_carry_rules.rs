//! Direct proofs for Pick Up and Drop locality, Disable filters, and oversized drop placement
//! (RULE-CATALOG-0140, RULE-CATALOG-0727, RULE-CATALOG-0920).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

#[test]
fn rule_catalog_0727_pick_up_and_drop_should_ignore_non_local_artifacts_and_disabled_units() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0727_pick_up_and_drop_should_ignore_non_local_artifacts_and_disabled_units(
    );
}

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
        "genesisGainMana": 6,
    })
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

fn power_artifact() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "artifact-carry-0920" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-artifact-carry-0920-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth_site(),
            "north-giant": giant(),
            "north-sword": power_artifact(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 9],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-giant", "north-sword", "north-giant", "north-sword",
                    "north-giant", "north-sword", "north-giant", "north-sword",
                ],
            },
            "south": {
                "atlas": vec!["south-earth"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["north-giant"; 16],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
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

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == cell
    });
}

fn end_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn end_and_draw_atlas(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_unit(current: &Value, instance_id: &str) -> Option<Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .cloned()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn establish_oversized_footprint(session: &mut Session) {
    keep(session);
    keep(session);
    play_site(session, "C4");
    end_and_draw(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-earth"
            && descriptor["cell"] == "C1"
    });
    end_and_draw(session);
    play_site(session, "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    end_and_draw_atlas(session);
    play_site(session, "B3");
}

#[test]
fn rule_catalog_0920_oversized_drop_leaves_artifact_on_remembered_pick_up_cell() {
    let manifest = (1..=4096)
        .map(manifest)
        .find(|candidate| {
            let session = Session::new(candidate).expect("carry candidate");
            let replay = session.replay_value().expect("replay");
            let hand = replay["state"]["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("opening spellbook hand");
            hand.iter().any(|card| card["cardId"] == "north-giant")
                && hand.iter().any(|card| card["cardId"] == "north-sword")
        })
        .expect("bounded seed opening with a 2×2 bearer and a sword in hand");
    let mut session = Session::new(&manifest).expect("valid oversized carry scenario");
    establish_oversized_footprint(&mut session);

    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"].is_null()
    });
    let giant_id = summoned["cardInstanceId"]
        .as_str()
        .expect("2×2 identity")
        .to_owned();
    let (cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-sword"
            && descriptor["cell"] == "C4"
            && descriptor["bearer"].is_null()
    });
    let sword_id = cast["cardInstanceId"]
        .as_str()
        .expect("loose sword identity")
        .to_owned();

    let (picked, picked_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["unit"]["instanceId"] == giant_id.as_str()
            && descriptor["cell"] == "C4"
            && descriptor["artifactInstanceIds"] == json!([sword_id.as_str()])
    });
    assert_eq!(
        picked_receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-picked-up"]
    );
    let carried = state(&session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == sword_id.as_str())
        .expect("carried sword")
        .clone();
    assert_eq!(carried["bearerCell"], "C4");
    assert_eq!(
        realm_unit(&state(&session), &giant_id).expect("giant remains")["location"],
        "B3"
    );

    let (_, dropped_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "drop-artifacts"
            && descriptor["unit"]["instanceId"] == giant_id.as_str()
            && descriptor["artifactInstanceIds"] == json!([sword_id.as_str()])
    });
    assert_eq!(
        dropped_receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-dropped"]
    );
    let loose = state(&session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == sword_id.as_str())
        .expect("dropped sword")
        .clone();
    assert!(loose["bearer"].is_null());
    assert_eq!(loose["location"], "C4");
    assert_eq!(loose["region"], "surface");
    assert_eq!(
        realm_unit(&state(&session), &giant_id).expect("giant remains")["location"],
        "B3"
    );
    assert_eq!(picked["cell"], "C4");
    assert_exact_replay(&session);
}
