//! Direct proofs for Genesis strike-each-enemy-here (RULE-CATALOG-0057,
//! RULE-CATALOG-0675–0676).
//!
//! On entry, a minion strikes every enemy sharing its location, including the
//! Avatar standing on that site. Allies and the striker are skipped. Ward
//! absorbs a strike. Enemies on a different cell are not reached.

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

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn titan() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 3,
        "genesisStrikeEachEnemyHere": true,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn plain() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn warded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn strike_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "genesis-strike-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-genesis-strike-here-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-site": site(),
            "north-titan": titan(),
            "south-avatar": avatar(),
            "south-plain": plain(),
            "south-site": site(),
            "south-warded": warded(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-titan",
                    "north-ally",
                    "north-titan",
                    "north-ally",
                    "north-titan",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-plain",
                    "south-warded",
                    "south-plain",
                    "south-warded",
                    "south-plain",
                    "south-warded",
                ],
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
    let mut session = Session::new(encoded).expect("valid genesis-strike-here session");
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn avatar_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar identity")
        .to_owned()
}

fn summon_at(session: &mut Session, card_id: &str, cell: &str) -> (String, Receipt) {
    let (summoned, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    (
        summoned["cardInstanceId"]
            .as_str()
            .expect("summoned identity")
            .to_owned(),
        receipt,
    )
}

fn seed_with(required_north: &[&str], required_south: &[&str]) -> String {
    (675..675 + 256)
        .map(strike_manifest)
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("candidate session"));
            let has = |seat: &str, wanted: &[&str]| {
                let hand = opening["players"][seat]["hand"]["spellbook"]
                    .as_array()
                    .expect("opening hand");
                wanted
                    .iter()
                    .all(|id| hand.iter().any(|card| card["cardId"] == *id))
            };
            has("north", required_north) && has("south", required_south)
        })
        .expect("bounded seed with required opening cards")
}

fn enemies_at_c1(session: &mut Session) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut enemy_ids = Vec::new();
    for card_id in ["south-plain", "south-warded"] {
        enemy_ids.push(summon_at(session, card_id, "C1").0);
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_ids
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
fn rule_catalog_0675_genesis_strike_hits_every_enemy_sharing_the_newcomers_cell() {
    let encoded = seed_with(
        &["north-ally", "north-titan"],
        &["south-plain", "south-warded"],
    );
    let mut session = opening_main(&encoded);
    let enemy_ids = enemies_at_c1(&mut session);
    let ally_id = summon_at(&mut session, "north-ally", "C1").0;
    let (titan_id, receipt) = summon_at(&mut session, "north-titan", "C1");

    let mut struck: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 3);
            assert_eq!(event.payload["strikerInstanceId"], titan_id.as_str());
            event.payload["targetInstanceId"]
                .as_str()
                .expect("struck identity")
                .to_owned()
        })
        .collect();
    let resolved = state(&session);
    let enemy_avatar_id = avatar_id(&resolved, "south");
    let mut expected = enemy_ids.clone();
    expected.push(enemy_avatar_id);
    struck.sort_unstable();
    expected.sort_unstable();
    assert_eq!(struck, expected);
    assert!(!struck.contains(&ally_id));
    assert!(!struck.contains(&titan_id));
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken")
    );

    let plain_id = enemy_ids
        .iter()
        .find(|instance_id| unit(&resolved, instance_id)["cardId"] == "south-plain")
        .expect("plain enemy")
        .clone();
    let warded_id = enemy_ids
        .iter()
        .find(|instance_id| **instance_id != plain_id)
        .expect("warded enemy")
        .clone();
    assert_eq!(unit(&resolved, &plain_id)["damage"], 3);
    let warded = unit(&resolved, &warded_id);
    assert_eq!(warded["damage"], 0);
    assert_eq!(warded["warded"], false);
    assert_eq!(unit(&resolved, &ally_id)["damage"], 0);
    assert_eq!(unit(&resolved, &titan_id)["damage"], 0);
    assert_eq!(resolved["players"]["south"]["avatar"]["life"], 17);
    assert_eq!(resolved["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0676_genesis_strike_skips_allies_and_far_enemies() {
    let encoded = seed_with(
        &["north-ally", "north-titan"],
        &["south-plain", "south-warded"],
    );
    let mut session = opening_main(&encoded);
    let enemy_ids = enemies_at_c1(&mut session);
    let ally_id = summon_at(&mut session, "north-ally", "C4").0;
    let (titan_id, receipt) = summon_at(&mut session, "north-titan", "C4");

    assert_eq!(event_types(&receipt), ["minion-summoned"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "strike-damage-allocated")
    );

    let resolved = state(&session);
    let warded_id = enemy_ids
        .iter()
        .find(|instance_id| unit(&resolved, instance_id)["cardId"] == "south-warded")
        .expect("warded enemy");
    for enemy_id in &enemy_ids {
        assert_eq!(unit(&resolved, enemy_id)["damage"], 0);
        assert_eq!(unit(&resolved, enemy_id)["location"], "C1");
    }
    assert_eq!(unit(&resolved, warded_id)["warded"], true);
    assert_eq!(unit(&resolved, &ally_id)["damage"], 0);
    assert_eq!(unit(&resolved, &titan_id)["damage"], 0);
    assert_eq!(resolved["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(resolved["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}
