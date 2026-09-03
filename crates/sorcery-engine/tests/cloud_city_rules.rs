//! Direct proof for Cloud City flight (RULE-CATALOG-0101): once per turn at three Air affinity,
//! into a nearby empty cell, carrying normal occupants.

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

fn cloud_city_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["air"],
        "flyToNearbyVoidOncePerTurnAtAirThreshold": 3,
    })
}

fn north_minion() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 1,
        "thresholds": { "air": 1, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn filler_site() -> Value {
    json!({ "cardType": "site", "elements": ["air"] })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cloud-city-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cloud-city-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": cloud_city_site(),
            "north-spell": north_minion(),
            "south-avatar": avatar(),
            "south-filler": north_minion(),
            "south-site": filler_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": vec!["north-spell"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 8],
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
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

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn draw_spell(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn pass_turn(session: &mut Session) {
    end_turn(session);
    draw_spell(session);
}

#[test]
fn rule_catalog_0101_cloud_city_should_fly_once_per_turn_at_three_air_and_carry_occupants() {
    let mut session = Session::new(&manifest(136)).expect("valid Cloud City scenario");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "C4");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let current = state(&session);
    let source_site_id = current["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("source site identity")
        .to_owned();
    let minion_id = current["realm"]["units"][0]["instanceId"]
        .as_str()
        .expect("summoned minion identity")
        .to_owned();
    assert!(!offers(&session, |descriptor| descriptor["kind"] == "fly-site"));

    pass_turn(&mut session);
    play_site(&mut session, "C1");
    pass_turn(&mut session);
    play_site(&mut session, "C3");
    pass_turn(&mut session);
    play_site(&mut session, "B1");
    pass_turn(&mut session);
    play_site(&mut session, "B3");

    let mana_before = state(&session)["players"]["north"]["mana"].clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "fly-site"
            && descriptor["sourceSiteInstanceId"] == source_site_id.as_str()
            && descriptor["targetCell"] == "D4"
    });
    let current = state(&session);
    assert!(current["realm"]["sites"]["C4"].is_null());
    assert_eq!(
        current["realm"]["sites"]["D4"]["instanceId"],
        source_site_id
    );
    assert_eq!(current["players"]["north"]["avatar"]["location"], "D4");
    assert_eq!(
        current["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == minion_id)
            .expect("carried minion")["location"],
        "D4"
    );
    assert_eq!(current["players"]["north"]["mana"], mana_before);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-flown")
    );
    assert!(!offers(&session, |descriptor| descriptor["kind"]
        == "fly-site"
        && descriptor["sourceSiteInstanceId"]
            == source_site_id.as_str()));
    assert_exact_replay(&session);
}
