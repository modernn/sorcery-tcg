//! Direct proofs for grant-Lethal-this-turn Magic (RULE-CATALOG-0278–0279,
//! RULE-CATALOG-1106).
//!
//! Official Magic can grant Lethal for the current turn. The grant uses the
//! same ally choice as Charge, persists only on minions, and expires through
//! the shared End Phase temporary-effect cleanup. One point of Lethal damage
//! destroys a tougher minion; the same strike without Lethal only wounds it.
//! Grant-Lethal Magic stays withheld while Deathrites wait for ordering.

use serde_json::{json, Value};
use sorcery_engine::canonical::{canonical_json, identity_hash, IdentityHash};
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

fn striker() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn tough_any_site() -> Value {
    json!({
        "attack": 0,
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
        "grantLethalToAllyThisTurn": true,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest() -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-lethal" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-lethal-v1",
        },
        "cards": {
            "north-ally": striker(),
            "north-avatar": avatar(),
            "north-grant": grant(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-tough": tough_any_site(),
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
                "spellbook": vec!["south-tough"; 6],
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
    try_accept_where(session, predicate).expect("expected engine-issued action")
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

fn cemetery_has(session: &Session, seat: &str, instance_id: &str) -> bool {
    state(session)["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn opening_main() -> Session {
    let mut session = Session::new(&manifest()).expect("grant Lethal");
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

fn grant_lethal(session: &mut Session, ally_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-grant"
            && descriptor["ally"]["instanceId"] == ally_id
    })
}

fn grant_ally_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("grant actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-grant"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn south_summons_tough_at_c4(session: &mut Session) -> String {
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
            && descriptor["cardId"] == "south-tough"
            && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
}

fn strike_minion(session: &mut Session, attacker_id: &str, enemy_id: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C4"
    });
    while state(session)["phase"] == "movement" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
    }
    accept_where(session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == enemy_id
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(session)["phase"] == "intercept" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
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
fn rule_catalog_0278_grant_lethal_lasts_only_until_end_of_turn() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let before = state(&session);
    assert!(unit(&before, &ally_id)
        .get("temporaryLethalSources")
        .is_none());

    let (descriptor, receipt) = grant_lethal(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "lethal-granted", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], ally_id);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    let granted = state(&session);
    assert_eq!(
        unit(&granted, &ally_id)["temporaryLethalSources"],
        json!([descriptor["cardInstanceId"]])
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "lethal-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == descriptor["cardInstanceId"]
    }));
    let after = state(&session);
    assert!(unit(&after, &ally_id)
        .get("temporaryLethalSources")
        .is_none());
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("grant-lethal checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized grant-lethal");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed grant-lethal");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed grant-lethal session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0279_granted_lethal_is_required_to_kill_a_tougher_minion() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_tough_at_c4(&mut session);
    let ready = state(&session);
    assert_eq!(unit(&ready, &ally_id)["summoningSickness"], false);
    assert_eq!(
        session.public_view(Seat::North).expect("North public view")["realm"]["units"]
            .as_array()
            .expect("public units")
            .iter()
            .find(|unit| unit["instanceId"] == enemy_id)
            .expect("public enemy")["defense"],
        2
    );

    let mut wounded = session.clone();
    strike_minion(&mut wounded, &ally_id, &enemy_id);
    let after_wound = state(&wounded);
    assert_eq!(unit(&after_wound, &enemy_id)["damage"], 1);
    assert!(!cemetery_has(&wounded, "south", &enemy_id));

    grant_lethal(&mut session, &ally_id);
    strike_minion(&mut session, &ally_id, &enemy_id);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert!(!state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .any(|unit| unit["instanceId"] == enemy_id));
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 2);
    assert_exact_replay(&session);
}

fn deathrite_grant_lethal_manifest(seed: u32) -> String {
    let fixture = "grant-lethal-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": striker(),
            "north-avatar": avatar(),
            "north-grant": grant(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-grant",
                    "north-rain",
                    "north-rain",
                    "north-grant",
                    "north-grant",
                ],
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
        "seed": seed,
    }))
}

fn north_has_grant_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-grant", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteGrantLethalSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_allied_minion(
    encoded: &str,
) -> Option<PendingDeathriteGrantLethalSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
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
    if !north_has_grant_and_rain(&state(&session)) {
        return None;
    }
    if !grant_ally_ids(&session).contains(&ally_id) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteGrantLethalSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_grant_lethal_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_grant_lethal_manifest)
        .find(|candidate| try_pending_deathrite_with_allied_minion(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with grant-lethal Magic in hand")
}

#[test]
fn rule_catalog_1106_grant_lethal_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_grant_lethal_seed_with(1106);
    let mut setup = try_pending_deathrite_with_allied_minion(&encoded)
        .expect("complete grant-lethal Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(unit(&paused, &ally_id).is_object());
    assert!(session
        .legal_actions()
        .expect("paused legal actions")
        .iter()
        .all(|action| action.descriptor["kind"] != "cast-magic"));
    assert!(grant_ally_ids(session).is_empty());

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(unit(&resumed, &ally_id).is_object());
    assert!(grant_ally_ids(session).contains(&ally_id));

    let (descriptor, receipt) = grant_lethal(session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "lethal-granted", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], ally_id);
    assert_eq!(
        unit(&state(session), &ally_id)["temporaryLethalSources"],
        json!([descriptor["cardInstanceId"]])
    );
    assert_exact_replay(session);
}
