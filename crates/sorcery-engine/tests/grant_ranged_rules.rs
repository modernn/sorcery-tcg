//! Direct proofs for grant-Ranged-this-turn Magic (RULE-CATALOG-0276–0277).
//!
//! Official Magic can grant Ranged for the current turn. The grant uses the
//! same ally choice as Charge, persists only on minions, and expires through
//! the shared End Phase temporary-effect cleanup. Summoning sickness still
//! blocks Ranged: Charge bypasses sickness for Move and Attack, not for shots.

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

fn grant() -> Value {
    json!({
        "cardType": "magic",
        "grantRangedToAllyThisTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest() -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-ranged" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-ranged-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-grant": grant(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": grounded(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-grant", "north-grant"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 6],
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

fn can_shoot(session: &Session, shooter_id: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .any(|action| {
            action.descriptor["kind"] == "shoot-projectile"
                && action.descriptor["shooterInstanceId"] == shooter_id
        })
}

fn opening_main() -> Session {
    let mut session = Session::new(&manifest()).expect("grant Ranged");
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

fn grant_ranged(session: &mut Session, ally_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-grant"
            && descriptor["ally"]["instanceId"] == ally_id
    })
}

fn pass_south_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
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
fn rule_catalog_0276_grant_ranged_lets_a_ready_minion_shoot_until_end_of_turn() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    pass_south_turn(&mut session);
    let before = state(&session);
    assert_eq!(unit(&before, &ally_id)["summoningSickness"], false);
    assert!(
        unit(&before, &ally_id)
            .get("temporaryRangedSources")
            .is_none()
    );
    assert!(!can_shoot(&session, &ally_id));

    let (descriptor, receipt) = grant_ranged(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ranged-granted", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], ally_id);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    let granted = state(&session);
    assert_eq!(
        unit(&granted, &ally_id)["temporaryRangedSources"],
        json!([descriptor["cardInstanceId"]])
    );
    assert!(
        can_shoot(&session, &ally_id),
        "granted Ranged must issue shoot-projectile after sickness clears"
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "ranged-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == descriptor["cardInstanceId"]
    }));
    let after = state(&session);
    assert!(
        unit(&after, &ally_id)
            .get("temporaryRangedSources")
            .is_none()
    );
    assert!(!can_shoot(&session, &ally_id));
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("grant-ranged checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized grant-ranged");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed grant-ranged");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed grant-ranged session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0277_granted_ranged_does_not_bypass_summoning_sickness() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let before = state(&session);
    assert_eq!(unit(&before, &ally_id)["summoningSickness"], true);
    assert!(!can_shoot(&session, &ally_id));

    grant_ranged(&mut session, &ally_id);
    let granted = state(&session);
    assert!(
        unit(&granted, &ally_id)
            .get("temporaryRangedSources")
            .is_some()
    );
    assert_eq!(unit(&granted, &ally_id)["summoningSickness"], true);
    assert!(
        !can_shoot(&session, &ally_id),
        "granted Ranged must not let a sick minion shoot"
    );
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 3);
    assert_exact_replay(&session);
}
