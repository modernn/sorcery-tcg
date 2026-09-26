//! Direct proofs for grant-Airborne-this-turn Magic (RULE-CATALOG-0274–0275,
//! RULE-CATALOG-0665–0666, RULE-CATALOG-1026, RULE-CATALOG-1603–1608,
//! RULE-CATALOG-2293–2298).
//!
//! Official Magic can grant Airborne for the current turn. The grant uses the
//! same ally choice as Charge, persists only on minions, is lost while the
//! minion is Disabled or grounded, and expires through the shared End Phase
//! temporary-effect cleanup. Grounded attackers cannot strike Airborne minions
//! until they themselves become Airborne. A printed-Airborne ally still takes
//! the temporary source; End Phase expiry removes that source and leaves the
//! printed keyword. Grant-Airborne-to-target-minion Magic stays withheld while
//! Deathrites wait for ordering. Supplemental 2293–2298 keep persistence,
//! stack-repeat, enemy-arrival, multi-minion, far-minion, and new-summon
//! proofs on later IDs.

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

fn printed_airborne() -> Value {
    json!({
        "airborne": true,
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

fn airborne_target_spell() -> Value {
    json!({
        "cardType": "magic",
        "grantAirborneToTargetMinion": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn visitor() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(north_ally: &Value, south_spell: &str) -> String {
    let mut cards = json!({
        "north-ally": north_ally,
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

fn try_accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
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
    opening_with(&grounded(), south_spell)
}

fn opening_with(north_ally: &Value, south_spell: &str) -> Session {
    let mut session = Session::new(&manifest(north_ally, south_spell)).expect("grant Airborne");
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

fn grant_airborne(session: &mut Session, ally_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-grant"
            && descriptor["ally"]["instanceId"] == ally_id
    })
}

fn south_summons_airborne_at_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-airborne"
            && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
}

fn through_south_pass_to_north_main(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn expire_grant_airborne(session: &mut Session, ally_id: &str, grant_source: &str) {
    let (_, ended) = accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "airborne-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == grant_source
    }));
    assert!(
        unit(&state(session), ally_id)
            .get("temporaryAirborneSources")
            .is_none()
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn grant_airborne_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("grant-Airborne actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-airborne"
        })
        .filter_map(|action| {
            let target = action.descriptor.get("target")?;
            Some((
                target["kind"].as_str()?.to_owned(),
                target["instanceId"].as_str()?.to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn deathrite_airborne_manifest(seed: u32) -> String {
    let fixture = "grant-airborne-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-airborne": airborne_target_spell(),
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-airborne",
                    "north-rain",
                    "north-rain",
                    "north-airborne",
                    "north-rain",
                    "north-airborne",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-visitor", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_airborne_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-airborne", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteAirborneSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_grounded_visitor(
    encoded: &str,
) -> Option<PendingDeathriteAirborneSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let visitor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    })?;
    let visitor_id = visitor.0["cardInstanceId"].as_str()?.to_owned();
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_airborne_and_rain(&state(&session)) {
        return None;
    }
    if grant_airborne_targets(&session).is_empty() {
        return None;
    }
    if public_airborne(&session, &visitor_id) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteAirborneSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_airborne_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_airborne_manifest)
        .find(|candidate| try_pending_deathrite_with_grounded_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with grant-Airborne Magic in hand")
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

#[test]
fn rule_catalog_0665_grant_airborne_makes_a_grounded_minion_airborne() {
    let mut session = opening_main("south-grounded");
    let ally_id = summon_north_ally(&mut session);
    assert!(
        unit(&state(&session), &ally_id)
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
    assert_eq!(
        unit(&state(&session), &ally_id)["temporaryAirborneSources"],
        json!([descriptor["cardInstanceId"]])
    );
    assert!(public_airborne(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0666_already_airborne_grant_expires_at_end_phase() {
    let mut session = opening_with(&printed_airborne(), "south-grounded");
    let ally_id = summon_north_ally(&mut session);
    assert!(public_airborne(&session, &ally_id));
    assert!(
        unit(&state(&session), &ally_id)
            .get("temporaryAirborneSources")
            .is_none()
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-grant"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "airborne-granted", "magic-resolved"]
    );
    assert_eq!(
        unit(&state(&session), &ally_id)["temporaryAirborneSources"],
        json!([descriptor["cardInstanceId"]])
    );
    assert!(public_airborne(&session, &ally_id));

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "airborne-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == descriptor["cardInstanceId"]
    }));
    assert!(
        unit(&state(&session), &ally_id)
            .get("temporaryAirborneSources")
            .is_none()
    );
    assert!(
        public_airborne(&session, &ally_id),
        "printed Airborne remains after the temporary grant expires"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1603_printed_airborne_without_grant_strikes_airborne_enemy() {
    let mut session = opening_with(&printed_airborne(), "south-airborne");
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_airborne_at_c4(&mut session);
    assert!(public_airborne(&session, &ally_id));
    assert!(
        unit(&state(&session), &ally_id)
            .get("temporaryAirborneSources")
            .is_none()
    );
    assert!(can_strike_minion(&session, &ally_id, &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1604_granted_airborne_strikes_airborne_enemy_before_end_of_turn() {
    let mut session = opening_main("south-airborne");
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_airborne_at_c4(&mut session);
    let (descriptor, _) = grant_airborne(&mut session, &ally_id);
    assert!(public_airborne(&session, &ally_id));
    assert!(can_strike_minion(&session, &ally_id, &enemy_id));
    assert_eq!(
        unit(&state(&session), &ally_id)["temporaryAirborneSources"],
        json!([descriptor["cardInstanceId"]])
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1605_granted_airborne_expires_before_ally_strikes_on_later_turn() {
    let mut session = opening_main("south-airborne");
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_airborne_at_c4(&mut session);
    let (descriptor, _) = grant_airborne(&mut session, &ally_id);
    expire_grant_airborne(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    through_south_pass_to_north_main(&mut session);
    assert!(!public_airborne(&session, &ally_id));
    assert!(!can_strike_minion(&session, &ally_id, &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1606_printed_airborne_still_strikes_after_grant_expires_on_later_turn() {
    let mut session = opening_with(&printed_airborne(), "south-airborne");
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_airborne_at_c4(&mut session);
    let (descriptor, _) = grant_airborne(&mut session, &ally_id);
    expire_grant_airborne(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    through_south_pass_to_north_main(&mut session);
    assert!(public_airborne(&session, &ally_id));
    assert!(can_strike_minion(&session, &ally_id, &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1607_printed_and_granted_airborne_compose_while_grant_is_active() {
    let mut session = opening_with(&printed_airborne(), "south-airborne");
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_airborne_at_c4(&mut session);
    let (descriptor, receipt) = grant_airborne(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "airborne-granted", "magic-resolved"]
    );
    assert_eq!(
        unit(&state(&session), &ally_id)["temporaryAirborneSources"],
        json!([descriptor["cardInstanceId"]])
    );
    assert!(can_strike_minion(&session, &ally_id, &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1608_printed_airborne_outlasts_expired_grant_while_plain_ally_cannot_strike() {
    let mut session = opening_with(&printed_airborne(), "south-airborne");
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_airborne_at_c4(&mut session);
    let (descriptor, _) = grant_airborne(&mut session, &ally_id);
    expire_grant_airborne(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );

    let mut plain = opening_main("south-airborne");
    let plain_ally = summon_north_ally(&mut plain);
    let plain_enemy = south_summons_airborne_at_c4(&mut plain);
    let (plain_descriptor, _) = grant_airborne(&mut plain, &plain_ally);
    expire_grant_airborne(
        &mut plain,
        &plain_ally,
        plain_descriptor["cardInstanceId"]
            .as_str()
            .expect("grant source"),
    );
    through_south_pass_to_north_main(&mut plain);
    assert!(!can_strike_minion(&plain, &plain_ally, &plain_enemy));

    through_south_pass_to_north_main(&mut session);
    assert!(can_strike_minion(&session, &ally_id, &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1026_grant_airborne_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_airborne_seed_with(1026);
    let mut setup = try_pending_deathrite_with_grounded_visitor(&encoded)
        .expect("complete grant-Airborne Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(!public_airborne(session, &visitor_id));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(grant_airborne_targets(session).is_empty());

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(!public_airborne(session, &visitor_id));
    assert_eq!(
        grant_airborne_targets(session),
        [("minion".to_owned(), visitor_id.clone())]
    );

    let (descriptor, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-airborne"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "airborne-granted", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], visitor_id);
    assert_eq!(receipt.events[1].payload["seat"], "south");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(public_airborne(session, &visitor_id));
    assert_exact_replay(session);
}

fn supplemental_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn airborne_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-airborne-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-airborne-supplemental-v1",
        },
        "cards": {
            "north-airborne": airborne_target_spell(),
            "north-avatar": avatar(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": supplemental_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-airborne"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_hand_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("opening Spellbook hand")
        .iter()
        .map(|card| {
            card["cardId"]
                .as_str()
                .expect("hand card identity")
                .to_owned()
        })
        .collect()
}

fn supplemental_seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(665..665 + 2048)
        .map(airborne_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-airborne")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with grant-Airborne Magic and required South minions")
}

fn airborne_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-airborne")
                .count()
        })
        .unwrap_or_default()
}

fn temporary_airborne_len(snapshot: &Value, instance_id: &str) -> usize {
    unit(snapshot, instance_id)
        .get("temporaryAirborneSources")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .ok()
        .is_some_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn decline_attack_if_needed(session: &mut Session) {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

fn end_turn_if_offered(session: &mut Session) {
    decline_attack_if_needed(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn opening_supplemental(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid grant-Airborne session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("grant-Airborne target identity")
        .to_owned()
}

fn setup_c2_with_south_minions(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    (0..count).map(|_| summon_south_at(session, "C2")).collect()
}

fn cast_grant_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-airborne"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn grant_target_ids(session: &Session) -> Vec<String> {
    grant_airborne_targets(session)
        .into_iter()
        .map(|(_, instance_id)| instance_id)
        .collect()
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_supplemental(encoded);
    let c2_ids = setup_c2_with_south_minions(&mut session, 2);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let far_id = summon_south_at(&mut session, "C4");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (airborne_spells_in_hand(&state(&session)) >= 1).then_some((session, c2_ids[0].clone(), far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(665..665 + 2048)
        .find_map(|seed| {
            let encoded = airborne_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching grant-Airborne far-minion setup")
}

fn try_second_grant_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_supplemental(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_grant_on(&mut session, &first_id);
    if !event_types(&first).contains(&"airborne-granted") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if airborne_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let minion_id = summon_south_at(&mut session, "C3");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    grant_target_ids(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_grant_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(665..665 + 8192)
        .find_map(|seed| {
            let encoded = airborne_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_grant_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second grant-Airborne enemy-arrival setup")
}

fn try_second_grant_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_supplemental(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_grant_on(&mut session, &first_id);
    if !event_types(&first).contains(&"airborne-granted") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if airborne_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    grant_target_ids(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_grant_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(665..665 + 8192)
        .find_map(|seed| {
            let encoded = airborne_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_grant_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second grant-Airborne new-summon setup")
}

#[test]
fn rule_catalog_2293_granted_minion_stays_at_the_location_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2293, 1);
    let mut session = opening_supplemental(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let receipt = cast_grant_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"airborne-granted"));
    assert!(public_airborne(&session, &minion_id));
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C2");
    assert_eq!(temporary_airborne_len(&state(&session), &minion_id), 0);
    assert!(!public_airborne(&session, &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2294_second_grant_on_an_already_granted_minion_records_another_source() {
    let encoded = (2294..2294 + 8192)
        .chain(665..665 + 8192)
        .find_map(|seed| {
            let candidate = airborne_supplemental_manifest(seed);
            let mut session = opening_supplemental(&candidate);
            let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
            north_draws_spellbook(&mut session);
            let first = cast_grant_on(&mut session, &minion_id);
            if !event_types(&first).contains(&"airborne-granted") {
                return None;
            }
            if temporary_airborne_len(&state(&session), &minion_id) != 1 {
                return None;
            }
            (airborne_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two grant-Airborne casts after granting the first minion");
    let mut session = opening_supplemental(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_grant_on(&mut session, &minion_id);
    assert!(event_types(&first).contains(&"airborne-granted"));
    assert_eq!(temporary_airborne_len(&state(&session), &minion_id), 1);
    assert!(airborne_spells_in_hand(&state(&session)) >= 1);
    let second = cast_grant_on(&mut session, &minion_id);
    assert!(event_types(&second).contains(&"airborne-granted"));
    assert_eq!(temporary_airborne_len(&state(&session), &minion_id), 2);
    assert!(public_airborne(&session, &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2295_second_grant_marks_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_grant_enemy_arrival(2295);
    let (mut session, minion_id) = try_second_grant_enemy_arrival_prefix(&encoded)
        .expect("second grant-Airborne enemy-arrival prefix");
    let receipt = cast_grant_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"airborne-granted"));
    assert!(public_airborne(&session, &minion_id));
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2296_grant_airborne_offers_every_same_region_minion_in_the_caster_region() {
    let encoded = supplemental_seed_with_start(2296, 2);
    let mut session = opening_supplemental(&encoded);
    let minion_ids = setup_c2_with_south_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = grant_target_ids(&session);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_eq!(offered.len(), 2);
    let snapshot = state(&session);
    let north_avatar = snapshot["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = snapshot["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert!(
        grant_airborne_targets(&session)
            .iter()
            .all(|(kind, instance_id)| {
                kind == "minion" && *instance_id != north_avatar && *instance_id != south_avatar
            })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2297_grant_airborne_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(2297);
    let (mut session, granted_id, far_id) =
        try_far_minion_prefix(&encoded).expect("grant-Airborne far-minion prefix");
    let receipt = cast_grant_on(&mut session, &granted_id);
    assert!(event_types(&receipt).contains(&"airborne-granted"));
    assert!(public_airborne(&session, &granted_id));
    assert!(!public_airborne(&session, &far_id));
    assert_eq!(temporary_airborne_len(&state(&session), &far_id), 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2298_second_grant_marks_a_newly_summoned_minion() {
    let encoded = seed_for_second_grant_new_summon(2298);
    let (mut session, minion_id) = try_second_grant_new_summon_prefix(&encoded)
        .expect("second grant-Airborne new-summon prefix");
    let receipt = cast_grant_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"airborne-granted"));
    assert!(public_airborne(&session, &minion_id));
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C2");
    assert_exact_replay(&session);
}
