//! Direct proofs for start-turn occupied-site Aura destruction (RULE-CATALOG-0258–0259,
//! RULE-CATALOG-1241).
//!
//! Official cards such as Hamlet's Ablaze conjure atop an Ordinary or Exceptional site.
//! At the start of the controller's next turn the Aura destroys that site, the minions
//! standing atop it, and itself. Avatars are not minions. Unique or Legendary sites
//! cannot be targeted.

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

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn unique_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "uniqueOrLegendary": true,
    })
}

fn aura() -> Value {
    json!({
        "atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(unique_south: bool) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-site-destroy-aura" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-site-destroy-aura-v1",
        },
        "cards": {
            "north-aura": aura(),
            "north-avatar": avatar(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": if unique_south { unique_site() } else { site() },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-aura"; 6],
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step");
    let StepResult::Accepted(receipt) = result else {
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
    session.replay_value().expect("session value")["state"].clone()
}

fn aura_id(session: &Session) -> Value {
    state(session)["realm"]["auras"][0]["instanceId"].clone()
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

fn after_aura_on_c4(unique_south: bool) -> Session {
    let mut session =
        Session::new(&manifest(unique_south)).expect("valid site-destroy aura session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn resolve_start_turn_destroy(session: &mut Session, source_id: &Value) -> Receipt {
    assert_eq!(state(session)["phase"], "start-turn");
    let legal = session
        .legal_actions()
        .expect("start-turn site-destroy actions");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == *source_id
                && action.descriptor.get("lureTargetInstanceId").is_none()
        }),
        "the occupied-site Aura is the only start-turn source"
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == *source_id
    })
    .1
}

#[test]
fn rule_catalog_0258_start_turn_aura_destroys_the_occupied_site_and_minions_atop_it() {
    let mut session = after_aura_on_c4(false);
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let target_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-minion")
        .expect("south minion")["instanceId"]
        .clone();
    let receipt = resolve_start_turn_destroy(&mut session, &source_id);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed" && event.payload["cell"] == "C4")
    );
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "rubble-created" && event.payload["cell"] == "C4")
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-died" && event.payload["instanceId"] == target_id
    }));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "aura-dispelled" && event.payload["instanceId"] == source_id
    }));
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["realm"]["sites"]["C4"]["rubble"], true);
    assert!(after["realm"].get("auras").is_none());
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .all(|unit| unit["instanceId"] != target_id)
    );
    assert_eq!(after["players"]["north"]["avatar"]["location"], "C4");
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0259_unique_sites_are_illegal_and_empty_sites_still_burn() {
    let mut session = Session::new(&manifest(true)).expect("valid unique-site session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let legal_cells: Vec<_> = session
        .legal_actions()
        .expect("Ablaze offers beside a Unique site")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-aura" && action.descriptor["cardId"] == "north-aura"
        })
        .map(|action| action.descriptor["cells"].clone())
        .collect();
    assert!(
        legal_cells.contains(&json!(["C4"])),
        "an Ordinary site remains a legal conjure target: {legal_cells:?}"
    );
    assert!(
        !legal_cells.contains(&json!(["C1"])),
        "a Unique or Legendary site is not a legal conjure target: {legal_cells:?}"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let receipt = resolve_start_turn_destroy(&mut session, &source_id);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed" && event.payload["cell"] == "C4")
    );
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-dispelled")
    );
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["realm"]["sites"]["C4"]["rubble"], true);
    assert_eq!(after["players"]["north"]["avatar"]["location"], "C4");
    assert!(after["realm"].get("auras").is_none());
    assert_exact_replay(&session);
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn here_pulser() -> Value {
    json!({
        "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn deathrite_site_destroy_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-site-destroy-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-site-destroy-deathrite-withheld-v1",
        },
        "cards": {
            "north-aura": aura(),
            "north-avatar": avatar(),
            "north-pulser": here_pulser(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_minion(),
            "south-site": site(),
            "south-visitor": minion(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-pulser", "north-aura", "north-aura", "north-aura", "north-aura", "north-aura"],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-visitor",
                    "south-deathrite",
                    "south-deathrite",
                    "south-deathrite",
                    "south-deathrite",
                    "south-deathrite",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

struct PendingSiteDestroySetup {
    aura_id: Value,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_during_site_destroy_start_turn(
    encoded: &str,
) -> Option<PendingSiteDestroySetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let pulser = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    if state(&session)["phase"] != "start-turn" {
        return None;
    }
    let aura_id = aura_id(&session);
    let aura_id_str = aura_id.as_str()?.to_owned();
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let offered: Vec<_> = session
        .legal_actions()
        .ok()?
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    if !offered.contains(&pulser_id) || !offered.contains(&aura_id_str) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == pulser_id
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    if session
        .legal_actions()
        .ok()?
        .iter()
        .any(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingSiteDestroySetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

#[test]
fn rule_catalog_1241_start_turn_site_destroy_trigger_withheld_during_pending_deathrite_order() {
    let encoded = (1241..1241 + 256)
        .map(deathrite_site_destroy_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_site_destroy_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during site-destroy start-turn withhold",
        );
    let setup = try_pending_deathrite_during_site_destroy_start_turn(&encoded)
        .expect("complete site-destroy start-turn Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let mut session = setup.session;
    assert_eq!(state(&session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger")
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    assert_eq!(state(&session)["phase"], "start-turn");
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == aura_id
            })
    );
    assert_exact_replay(&session);
}
