//! Direct proofs for grant-Airborne-this-turn Magic (RULE-CATALOG-0274–0275).
//!
//! Official Magic can grant Airborne for the current turn. The grant uses the
//! same ally choice as Charge, persists only on minions, is lost while the
//! minion is Disabled or grounded, and expires through the shared End Phase
//! temporary-effect cleanup. Grounded attackers cannot strike Airborne minions
//! until they themselves become Airborne.

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn grounded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn airborne_any_site() -> Value {
    json!({
        "airborne": true,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn grant() -> Value {
    json!({
        "cardType": "magic",
        "grantAirborneToAllyThisTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(south_spell: &str) -> String {
    let mut cards = json!({
        "north-ally": grounded(),
        "north-avatar": avatar(),
        "north-grant": grant(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    cards[south_spell] = match south_spell {
        "south-airborne" => airborne_any_site(),
        "south-grounded" => grounded(),
        _ => panic!("unsupported south spell {south_spell}"),
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-airborne" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-airborne-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-grant", "north-grant"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec![south_spell; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
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

fn unit<'a>(after: &'a Value, instance_id: &str) -> &'a Value {
    after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("unit")
}

fn public_airborne(session: &Session, instance_id: &str) -> bool {
    let view = session.public_view(Seat::North).expect("North public view");
    view["realm"]["units"]
        .as_array()
        .expect("public units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("public unit")["airborne"]
        .as_bool()
        .expect("airborne flag")
}

fn can_strike_minion(session: &Session, attacker_id: &str, enemy_id: &str) -> bool {
    let Some(activation) = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == attacker_id
                && action.descriptor["to"]["cell"] == "C4"
        })
    else {
        return false;
    };
    let mut probe = session.clone();
    let StepResult::Accepted(_) = probe
        .step(ActionRequest {
            action_id: activation.action_id.to_string(),
            seat: activation.seat,
            state_version: activation.state_version,
        })
        .expect("zero-step attack")
    else {
        return false;
    };
    probe
        .legal_actions()
        .expect("declare-attack actions")
        .into_iter()
        .any(|action| {
            action.descriptor["kind"] == "declare-attack"
                && action.descriptor["target"]["kind"] == "minion"
                && action.descriptor["target"]["instanceId"] == enemy_id
        })
}

fn opening_main(south_spell: &str) -> Session {
    let mut session = Session::new(&manifest(south_spell)).expect("grant Airborne");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn summon_north_ally(session: &mut Session) -> String {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned()
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
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0274_grant_airborne_makes_a_grounded_minion_airborne_until_end_of_turn() {
    let mut session = opening_main("south-grounded");
    let ally_id = summon_north_ally(&mut session);
    let before = state(&session);
    assert!(
        unit(&before, &ally_id)
            .get("temporaryAirborneSources")
            .is_none()
    );
    assert!(!public_airborne(&session, &ally_id));

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-grant"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "airborne-granted", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], ally_id);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    let granted = state(&session);
    assert_eq!(
        unit(&granted, &ally_id)["temporaryAirborneSources"],
        json!([descriptor["cardInstanceId"]])
    );
    assert!(public_airborne(&session, &ally_id));

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        ended
            .events
            .iter()
            .any(|event| event.event_type == "airborne-expired"
                && event.payload["instanceId"] == ally_id
                && event.payload["sourceInstanceId"] == descriptor["cardInstanceId"])
    );
    let after = state(&session);
    assert!(
        unit(&after, &ally_id)
            .get("temporaryAirborneSources")
            .is_none()
    );
    assert!(!public_airborne(&session, &ally_id));
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("grant-airborne checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized grant-airborne");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed grant-airborne");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed grant-airborne session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0275_granted_airborne_is_required_to_strike_an_airborne_enemy() {
    let mut session = opening_main("south-airborne");
    let ally_id = summon_north_ally(&mut session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    let (enemy_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-airborne"
            && descriptor["cell"] == "C4"
    });
    let enemy_id = enemy_summon["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    assert_eq!(unit(&state(&session), &ally_id)["summoningSickness"], false);
    assert!(!public_airborne(&session, &ally_id));
    assert!(
        !can_strike_minion(&session, &ally_id, &enemy_id),
        "a grounded minion cannot strike an Airborne enemy"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-grant"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert!(public_airborne(&session, &ally_id));
    assert!(
        can_strike_minion(&session, &ally_id, &enemy_id),
        "granted Airborne lets the minion strike the Airborne enemy"
    );
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 2);
    assert_exact_replay(&session);
}
