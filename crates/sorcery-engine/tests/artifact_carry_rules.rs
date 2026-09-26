//! Direct proofs for Pick Up and Drop locality, Disable filters, oversized drop placement,
//! Pick Up withheld during trigger-order, and the Pick Up filter-matrix harness
//! (RULE-CATALOG-0140, RULE-CATALOG-0727, RULE-CATALOG-0920, RULE-CATALOG-1130,
//! RULE-CATALOG-2503–2508).
//!
//! 0727 proves non-local and Disable filters on a programmatic board. 2503–2508
//! bind persistence, empty-repeat, enemy-arrival, multi-subset, far-cell, and
//! new-summon branches on the session Pick Up matrix without re-proving ownership.

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

fn offers_kind(session: &Session, kind: &str) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions
            .iter()
            .any(|action| action.descriptor["kind"] == kind)
    })
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

fn deathrite_pick_up_manifest(seed: u32) -> String {
    let fixture = "artifact-carry-pick-up-deathrite-withheld";
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-rain": {
                "cardType": "magic",
                "damageEachAbovegroundMinion": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-site": {
                "cardType": "site",
                "elements": ["earth"],
            },
            "north-sword": power_artifact(),
            "south-avatar": avatar(),
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "deathriteDrawSite": true,
                "defense": 1,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": {
                "cardType": "site",
                "elements": ["earth"],
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-minion",
                    "north-sword",
                    "north-rain",
                    "north-rain",
                    "north-minion",
                    "north-sword",
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn north_has_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
}

struct PendingDeathritePickUpSetup {
    deathrite_ids: [String; 2],
    minion_id: String,
    session: Session,
    sword_id: String,
}

fn try_pending_deathrite_with_uncarried_artifact(
    encoded: &str,
) -> Option<PendingDeathritePickUpSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let minion = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let minion_id = minion.0["cardInstanceId"].as_str()?.to_owned();
    let (cast, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-sword"
            && descriptor["cell"] == "C4"
            && descriptor["bearer"].is_null()
    })?;
    let sword_id = cast["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if !offers_kind(&session, "pick-up-artifacts") {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    realm_unit(&state(&session), &minion_id)?;
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathritePickUpSetup {
        deathrite_ids,
        minion_id,
        session,
        sword_id,
    })
}

fn deathrite_pick_up_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_pick_up_manifest)
        .find(|candidate| try_pending_deathrite_with_uncarried_artifact(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with an uncarried Artifact at a minion cell",
        )
}

#[test]
fn rule_catalog_1130_pick_up_artifacts_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_pick_up_seed_with(1130);
    let mut setup = try_pending_deathrite_with_uncarried_artifact(&encoded)
        .expect("complete pick-up-artifacts Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let minion_id = setup.minion_id.clone();
    let sword_id = setup.sword_id.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        deathrite_ids
            .iter()
            .all(|instance_id| realm_unit(&paused, instance_id).is_none())
    );
    assert_eq!(
        realm_unit(&paused, &minion_id).expect("surviving bearer")["location"],
        "C4"
    );
    let loose = paused["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == sword_id.as_str())
        .expect("uncarried sword");
    assert!(loose["bearer"].is_null());
    assert_eq!(loose["location"], "C4");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "pick-up-artifacts")
    );

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
    assert!(offers_kind(session, "pick-up-artifacts"));

    let (_, picked) = accept_where(session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["unit"]["instanceId"] == minion_id.as_str()
            && descriptor["artifactInstanceIds"] == json!([sword_id.as_str()])
    });
    assert_eq!(
        picked
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-picked-up"]
    );
    assert_exact_replay(session);
}
fn drop_withheld_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn drop_withheld_bearer() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drop_withheld_rain() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drop_withheld_deathrite() -> Value {
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

fn deathrite_drop_artifacts_manifest(seed: u32) -> String {
    let fixture = "artifact-drop-deathrite-withheld";
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bearer": drop_withheld_bearer(),
            "north-rain": drop_withheld_rain(),
            "north-site": drop_withheld_site(),
            "north-sword": power_artifact(),
            "south-avatar": avatar(),
            "south-minion": drop_withheld_deathrite(),
            "south-site": drop_withheld_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-bearer",
                    "north-sword",
                    "north-rain",
                    "north-rain",
                    "north-bearer",
                    "north-sword",
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn drop_artifacts_offered(session: &Session, bearer_id: &str, sword_id: &str) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "drop-artifacts"
                && action.descriptor["unit"]["instanceId"] == bearer_id
                && action.descriptor["artifactInstanceIds"] == json!([sword_id])
        })
    })
}

fn carried_sword<'a>(current: &'a Value, sword_id: &str, bearer_id: &str) -> Option<&'a Value> {
    current["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| {
            artifact["instanceId"] == sword_id && artifact["bearer"]["instanceId"] == bearer_id
        })
}

struct PendingDeathriteDropSetup {
    bearer_id: String,
    deathrite_ids: [String; 2],
    session: Session,
    sword_id: String,
}

fn try_summon_drop_bearer(session: &mut Session) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-bearer"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    summoned["cardInstanceId"].as_str().map(ToOwned::to_owned)
}

fn try_cast_carried_sword(session: &mut Session, bearer_id: &str) -> Option<String> {
    let (cast, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-sword"
            && descriptor["bearer"]["instanceId"] == bearer_id
    })?;
    cast["cardInstanceId"].as_str().map(ToOwned::to_owned)
}

fn try_pending_deathrite_with_carried_artifact(encoded: &str) -> Option<PendingDeathriteDropSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let mut bearer_id = try_summon_drop_bearer(&mut session);
    let mut sword_id = bearer_id
        .as_deref()
        .and_then(|bearer_id| try_cast_carried_sword(&mut session, bearer_id));
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
    if bearer_id.is_none() {
        bearer_id = try_summon_drop_bearer(&mut session);
    }
    let bearer_id = bearer_id?;
    if sword_id.is_none() {
        sword_id = try_cast_carried_sword(&mut session, &bearer_id);
    }
    let sword_id = sword_id?;
    if !drop_artifacts_offered(&session, &bearer_id, &sword_id) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    let paused = state(&session);
    if paused["phase"] != "trigger-order" {
        return None;
    }
    realm_unit(&paused, &bearer_id)?;
    carried_sword(&paused, &sword_id, &bearer_id)?;
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteDropSetup {
        bearer_id,
        deathrite_ids,
        session,
        sword_id,
    })
}

fn deathrite_drop_artifacts_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_drop_artifacts_manifest)
        .find(|candidate| try_pending_deathrite_with_carried_artifact(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with a carried Artifact ready to Drop",
        )
}

#[test]
fn rule_catalog_1131_drop_artifacts_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_drop_artifacts_seed_with(1131);
    let mut setup = try_pending_deathrite_with_carried_artifact(&encoded)
        .expect("complete Drop Artifacts Deathrite withheld setup");
    let bearer_id = setup.bearer_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let sword_id = setup.sword_id.clone();
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
    assert!(realm_unit(&paused, &bearer_id).is_some());
    assert!(carried_sword(&paused, &sword_id, &bearer_id).is_some());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "drop-artifacts")
    );

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
    assert!(realm_unit(&resumed, &bearer_id).is_some());
    assert!(carried_sword(&resumed, &sword_id, &bearer_id).is_some());
    assert!(drop_artifacts_offered(session, &bearer_id, &sword_id));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "drop-artifacts"
            && descriptor["unit"]["instanceId"] == bearer_id
            && descriptor["artifactInstanceIds"] == json!([sword_id])
    });
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-dropped"]
    );
    let loose = state(session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == sword_id)
        .expect("dropped sword")
        .clone();
    assert!(loose["bearer"].is_null());
    assert_eq!(loose["location"], "C4");
    assert_eq!(loose["region"], "surface");
    assert_exact_replay(session);
}

fn bury_magic() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn pickup_matrix_manifest(seed: u32) -> String {
    let fixture = "artifact-carry-pickup-matrix";
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury_magic(),
            "north-earth": earth_site(),
            "north-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-sword": power_artifact(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-sword": power_artifact(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-minion",
                    "north-sword",
                    "north-sword",
                    "north-bury",
                    "north-sword",
                    "north-sword",
                    "north-bury",
                    "north-minion",
                    "north-sword",
                    "north-sword",
                    "north-sword",
                    "north-sword",
                ],
            },
            "south": {
                "atlas": vec!["south-earth"; 12],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-sword",
                    "south-sword",
                    "south-minion",
                    "south-sword",
                    "south-sword",
                    "south-minion",
                    "south-sword",
                    "south-sword",
                    "south-minion",
                    "south-sword",
                    "south-sword",
                ],
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

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn opening_pickup_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid pickup-matrix session");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    session
}

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
}

fn loose_artifact_ids_at(snapshot: &Value, cell: &str, region: &str) -> Vec<String> {
    let mut ids: Vec<_> = snapshot["realm"]["artifacts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|artifact| {
            artifact["bearer"].is_null()
                && artifact["location"] == cell
                && artifact["region"] == region
        })
        .filter_map(|artifact| artifact["instanceId"].as_str().map(str::to_owned))
        .collect();
    ids.sort_unstable();
    ids
}

fn pick_up_actions(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "pick-up-artifacts")
        .map(|action| action.descriptor)
        .collect()
}

fn pick_up_keys_for_unit(session: &Session, kind: &str) -> Vec<String> {
    let mut keys: Vec<_> = pick_up_actions(session)
        .into_iter()
        .filter(|descriptor| descriptor["unit"]["kind"] == kind)
        .filter_map(|descriptor| {
            let mut ids: Vec<_> = descriptor["artifactInstanceIds"]
                .as_array()?
                .iter()
                .filter_map(|id| id.as_str().map(str::to_owned))
                .collect();
            ids.sort_unstable();
            Some(ids.join(","))
        })
        .collect();
    keys.sort_unstable();
    keys
}

fn try_cast_loose_artifact(session: &mut Session, card_id: &str, cell: &str) -> Option<String> {
    let (cast, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["bearer"].is_null()
    })?;
    Some(
        cast["cardInstanceId"]
            .as_str()
            .expect("cast artifact identity")
            .to_owned(),
    )
}

fn try_bury_artifact(session: &mut Session, instance_id: &str) -> Option<Receipt> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["targetArtifactInstanceId"] == instance_id
    })
    .map(|(_, receipt)| receipt)
}

fn try_draw_spellbook(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn try_draw_any(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    Some(())
}

fn try_end_turn(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn").map(|_| ())
}

fn try_south_main_turn(session: &mut Session) -> Option<()> {
    try_draw_any(session)?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-earth"
    });
    try_end_turn(session)?;
    Some(())
}

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    try_end_turn(session)?;
    try_south_main_turn(session)?;
    try_draw_spellbook(session)?;
    Some(())
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    try_pass_turn_to_north_spellbook(session).expect("pass back to North spellbook");
}

fn opening_hand_ids(encoded: &str, seat: &str, zone: &str, card_id: &str) -> usize {
    let mut session = Session::new(encoded).expect("candidate session");
    keep(&mut session);
    keep(&mut session);
    state(&session)["players"][seat]["hand"][zone]
        .as_array()
        .map(|hand| hand.iter().filter(|card| card["cardId"] == card_id).count())
        .unwrap_or_default()
}

fn opening_spellbook_ids(encoded: &str, seat: &str, card_id: &str) -> usize {
    opening_hand_ids(encoded, seat, "spellbook", card_id)
}

fn opening_atlas_ids(encoded: &str, seat: &str, card_id: &str) -> usize {
    opening_hand_ids(encoded, seat, "atlas", card_id)
}

fn spellbook_has(snapshot: &Value, card_id: &str) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == card_id))
}

struct PickupMatrixSetup {
    bearer_id: String,
    local_surface_ids: [String; 2],
    remote_id: String,
    session: Session,
    underground_id: String,
}

fn try_complete_cast_and_bury(session: &mut Session) -> Option<String> {
    for _ in 0..10 {
        try_draw_spellbook(session)?;
        if spellbook_has(&state(session), "north-sword")
            && let Some(cast_id) = try_cast_loose_artifact(session, "north-sword", "C4")
        {
            if spellbook_has(&state(session), "north-bury") {
                let buried = try_bury_artifact(session, &cast_id)?;
                if event_types(&buried).contains(&"artifact-burrowed") {
                    return Some(cast_id);
                }
            } else {
                try_end_turn(session)?;
                try_south_main_turn(session)?;
                try_draw_spellbook(session)?;
                let buried = try_bury_artifact(session, &cast_id)?;
                if event_types(&buried).contains(&"artifact-burrowed") {
                    return Some(cast_id);
                }
            }
        }
        try_end_turn(session)?;
        try_south_main_turn(session)?;
    }
    None
}

fn try_opening_pickup_matrix(encoded: &str) -> Option<PickupMatrixSetup> {
    if opening_spellbook_ids(encoded, "north", "north-sword") < 2
        || opening_spellbook_ids(encoded, "north", "north-minion") < 1
        || opening_spellbook_ids(encoded, "south", "south-sword") < 1
        || opening_atlas_ids(encoded, "north", "north-earth") < 1
    {
        return None;
    }

    let mut session = opening_pickup_main(encoded);
    let bearer = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let bearer_id = bearer.0["cardInstanceId"].as_str()?.to_owned();

    let surface_one = try_cast_loose_artifact(&mut session, "north-sword", "C4")?;
    let surface_two = try_cast_loose_artifact(&mut session, "north-sword", "C4")?;
    try_end_turn(&mut session)?;
    try_south_main_turn(&mut session)?;
    try_draw_spellbook(&mut session)?;
    if !spellbook_has(&state(&session), "north-sword") {
        try_end_turn(&mut session)?;
        try_south_main_turn(&mut session)?;
        try_draw_spellbook(&mut session)?;
    }
    if !spellbook_has(&state(&session), "north-sword") {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    })?;
    let remote_id = try_cast_loose_artifact(&mut session, "north-sword", "C3")?;
    try_end_turn(&mut session)?;
    try_south_main_turn(&mut session)?;
    let buried_source = try_complete_cast_and_bury(&mut session)?;

    let snapshot = state(&session);
    let local_surface_ids = loose_artifact_ids_at(&snapshot, "C4", "surface");
    if local_surface_ids.len() != 2
        || !local_surface_ids.contains(&surface_one)
        || !local_surface_ids.contains(&surface_two)
    {
        return None;
    }
    let underground_id = snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| artifact["location"] == "C4" && artifact["region"] == "underground")?
        .get("instanceId")?
        .as_str()?
        .to_owned();
    if underground_id != buried_source {
        return None;
    }
    if realm_artifact(&snapshot, &remote_id)?.get("location")? != "C3" {
        return None;
    }

    let mut expected = local_surface_ids.clone();
    expected.sort_unstable();
    let mut expected_keys = vec![
        expected[0].clone(),
        expected[1].clone(),
        format!("{},{}", expected[0], expected[1]),
    ];
    expected_keys.sort_unstable();
    let offered = pick_up_actions(&session);
    if offered.len() != 6 {
        return None;
    }
    if pick_up_keys_for_unit(&session, "avatar") != expected_keys {
        return None;
    }
    if pick_up_keys_for_unit(&session, "minion") != expected_keys {
        return None;
    }
    if offered.iter().any(|descriptor| {
        descriptor["artifactInstanceIds"]
            .as_array()
            .is_some_and(|ids| {
                ids.iter().any(|id| {
                    id.as_str() == Some(underground_id.as_str())
                        || id.as_str() == Some(remote_id.as_str())
                })
            })
    }) {
        return None;
    }

    let local_pair = [local_surface_ids[0].clone(), local_surface_ids[1].clone()];
    Some(PickupMatrixSetup {
        bearer_id,
        local_surface_ids: local_pair,
        remote_id,
        session,
        underground_id,
    })
}

fn pickup_matrix_seed_with(start: u32) -> String {
    (start..start + 2048)
        .chain(727..727 + 2048)
        .map(pickup_matrix_manifest)
        .find(|candidate| try_opening_pickup_matrix(candidate).is_some())
        .expect("bounded seed reaching Pick Up matrix setup")
}

fn pickup_matrix_seed_for<F>(start: u32, prefix: F) -> String
where
    F: Fn(&str) -> bool,
{
    (start..start + 8192)
        .chain(727..727 + 8192)
        .find_map(|seed| {
            let encoded = pickup_matrix_manifest(seed);
            prefix(&encoded).then_some(encoded)
        })
        .expect("bounded seed reaching Pick Up matrix prefix")
}

fn try_empty_repeat_prefix(encoded: &str) -> Option<(Session, String)> {
    let setup = try_opening_pickup_matrix(encoded)?;
    let mut session = setup.session;
    let bearer_id = setup.bearer_id;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["unit"]["instanceId"] == bearer_id
            && descriptor["artifactInstanceIds"]
                .as_array()
                .is_some_and(|ids| ids.len() == 2)
    })?;
    pick_up_actions(&session)
        .is_empty()
        .then_some((session, bearer_id))
}

fn carried_artifact_on_bearer(snapshot: &Value, bearer_id: &str) -> Option<String> {
    snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| {
            artifact["bearer"]["instanceId"] == bearer_id && artifact["bearer"]["kind"] == "minion"
        })?
        .get("instanceId")?
        .as_str()
        .map(str::to_owned)
}

fn try_summon_south_minion_at_c4(session: &mut Session) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    summoned["cardInstanceId"].as_str().map(str::to_owned)
}

fn try_cast_artifact_on_bearer(
    session: &mut Session,
    card_id: &str,
    bearer_id: &str,
) -> Option<String> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == card_id
            && descriptor["bearer"]["instanceId"] == bearer_id
    })?;
    carried_artifact_on_bearer(&state(session), bearer_id)
}

fn try_drop_artifact_from_bearer(
    session: &mut Session,
    bearer_id: &str,
    artifact_id: &str,
) -> Option<()> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "drop-artifacts"
            && descriptor["unit"]["instanceId"] == bearer_id
            && descriptor["artifactInstanceIds"]
                .as_array()
                .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(artifact_id)))
    })?;
    Some(())
}

fn try_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let setup = try_opening_pickup_matrix(encoded)?;
    if opening_spellbook_ids(encoded, "south", "south-sword") < 1
        || opening_spellbook_ids(encoded, "south", "south-minion") < 1
    {
        return None;
    }
    let bearer_id = setup.bearer_id.clone();
    let remote_id = setup.remote_id.clone();
    let mut session = setup.session;
    try_end_turn(&mut session)?;
    try_draw_any(&mut session)?;
    let south_id = try_summon_south_minion_at_c4(&mut session)?;
    let arrival_id = try_cast_artifact_on_bearer(&mut session, "south-sword", &south_id)?;
    try_drop_artifact_from_bearer(&mut session, &south_id, &arrival_id)?;
    try_end_turn(&mut session)?;
    try_draw_spellbook(&mut session)?;
    let offered = pick_up_actions(&session);
    offered
        .iter()
        .any(|descriptor| {
            descriptor["unit"]["instanceId"] == bearer_id
                && descriptor["artifactInstanceIds"]
                    .as_array()
                    .is_some_and(|ids| {
                        ids.iter()
                            .any(|id| id.as_str() == Some(arrival_id.as_str()))
                    })
        })
        .then_some((session, arrival_id, remote_id))
}

fn try_new_summon_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let setup = try_opening_pickup_matrix(encoded)?;
    let mut session = setup.session;
    let bearer_id = setup.bearer_id;
    try_pass_turn_to_north_spellbook(&mut session)?;
    let newcomer = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let newcomer_id = newcomer.0["cardInstanceId"].as_str()?.to_owned();
    if newcomer_id == bearer_id {
        return None;
    }
    let fresh_id = try_cast_loose_artifact(&mut session, "north-sword", "C4")?;
    pick_up_actions(&session)
        .iter()
        .any(|descriptor| {
            descriptor["unit"]["instanceId"] == newcomer_id
                && descriptor["artifactInstanceIds"]
                    .as_array()
                    .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(fresh_id.as_str())))
        })
        .then_some((session, newcomer_id, fresh_id))
}

#[test]
fn rule_catalog_2503_local_surface_artifacts_stay_pickable_after_turns_pass() {
    let encoded = pickup_matrix_seed_with(2503);
    let setup = try_opening_pickup_matrix(&encoded).expect("Pick Up matrix persistence prefix");
    let mut session = setup.session;
    let snapshot = state(&session);
    assert_eq!(loose_artifact_ids_at(&snapshot, "C4", "surface").len(), 2);
    assert!(
        realm_artifact(&snapshot, &setup.underground_id)
            .is_some_and(|artifact| artifact["region"] == "underground")
    );
    pass_turn_to_north_spellbook(&mut session);
    let later = state(&session);
    assert_eq!(loose_artifact_ids_at(&later, "C4", "surface").len(), 2);
    assert_eq!(pick_up_actions(&session).len(), 6);
    assert!(
        realm_artifact(&later, &setup.remote_id)
            .is_some_and(|artifact| artifact["location"] == "C3")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2504_second_pick_up_stays_unoffered_after_carrying_all_local_surface_artifacts() {
    let encoded = pickup_matrix_seed_for(2504, |candidate| {
        try_empty_repeat_prefix(candidate).is_some()
    });
    let (session, bearer_id) =
        try_empty_repeat_prefix(&encoded).expect("Pick Up matrix empty-repeat prefix");
    assert!(realm_unit(&state(&session), &bearer_id).is_some());
    assert!(pick_up_actions(&session).is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2505_pick_up_offers_a_newly_arrived_surface_artifact_after_enemy_cast() {
    let encoded = pickup_matrix_seed_for(2505, |candidate| {
        try_enemy_arrival_prefix(candidate).is_some()
    });
    let (mut session, arrival_id, remote_id) =
        try_enemy_arrival_prefix(&encoded).expect("Pick Up matrix enemy-arrival prefix");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["artifactInstanceIds"]
                .as_array()
                .is_some_and(|ids| {
                    ids.iter()
                        .any(|id| id.as_str() == Some(arrival_id.as_str()))
                })
    });
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-picked-up"]
    );
    let snapshot = state(&session);
    assert!(
        realm_artifact(&snapshot, &arrival_id)
            .is_some_and(|artifact| !artifact["bearer"].is_null())
    );
    assert!(
        realm_artifact(&snapshot, &remote_id).is_some_and(|artifact| artifact["bearer"].is_null())
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2506_pick_up_offers_every_local_surface_subset_at_c4() {
    let encoded = pickup_matrix_seed_with(2506);
    let setup = try_opening_pickup_matrix(&encoded).expect("Pick Up matrix multi-subset prefix");
    let offered = pick_up_actions(&setup.session);
    assert_eq!(offered.len(), 6);
    let mut minion_keys = pick_up_keys_for_unit(&setup.session, "minion");
    minion_keys.sort_unstable();
    let mut expected = setup.local_surface_ids.to_vec();
    expected.sort_unstable();
    let mut expected_keys = vec![
        expected[0].clone(),
        expected[1].clone(),
        format!("{},{}", expected[0], expected[1]),
    ];
    expected_keys.sort_unstable();
    assert_eq!(minion_keys, expected_keys);
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_2507_pick_up_leaves_a_far_cell_artifact_untouched() {
    let encoded = pickup_matrix_seed_with(2507);
    let setup = try_opening_pickup_matrix(&encoded).expect("Pick Up matrix far-cell prefix");
    let mut session = setup.session;
    let target = setup.local_surface_ids[0].clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["unit"]["instanceId"] == setup.bearer_id
            && descriptor["artifactInstanceIds"] == json!([target.as_str()])
    });
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-picked-up"]
    );
    let snapshot = state(&session);
    assert!(
        realm_artifact(&snapshot, &setup.remote_id)
            .is_some_and(|artifact| artifact["bearer"].is_null() && artifact["location"] == "C3")
    );
    assert!(
        realm_artifact(&snapshot, &setup.underground_id)
            .is_some_and(|artifact| artifact["region"] == "underground")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2508_pick_up_offers_a_newly_summoned_bearer_for_a_fresh_surface_artifact() {
    let encoded =
        pickup_matrix_seed_for(2508, |candidate| try_new_summon_prefix(candidate).is_some());
    let (mut session, newcomer_id, fresh_id) =
        try_new_summon_prefix(&encoded).expect("Pick Up matrix new-summon prefix");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["unit"]["instanceId"] == newcomer_id
            && descriptor["artifactInstanceIds"] == json!([fresh_id.as_str()])
    });
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-picked-up"]
    );
    let snapshot = state(&session);
    assert!(
        realm_artifact(&snapshot, &fresh_id)
            .is_some_and(|artifact| artifact["bearer"]["instanceId"] == newcomer_id)
    );
    assert_exact_replay(&session);
}
