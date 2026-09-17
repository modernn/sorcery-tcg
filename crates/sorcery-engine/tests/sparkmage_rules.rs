//! Direct proofs for Sparkmage activation (RULE-CATALOG-0149–0150, RULE-CATALOG-0723,
//! RULE-CATALOG-0802, RULE-CATALOG-1137).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(sparkmage_ability: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    if sparkmage_ability {
        value["tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn"] = json!(true);
    }
    value
}

fn site(elements: &[&str]) -> Value {
    json!({ "cardType": "site", "elements": elements })
}

fn minion(air_threshold: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": air_threshold, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sparkmage-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sparkmage-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(true),
            "north-minion": minion(1),
            "north-site": site(&["air"]),
            "south-avatar": avatar(false),
            "south-minion": minion(0),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 6],
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
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let actions = session.legal_actions().expect("legal actions");
    let action = actions
        .iter()
        .find(|action| predicate(&action.descriptor))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action; available={:?}",
                actions
                    .iter()
                    .map(|action| &action.descriptor)
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

fn first_main(seed: u32) -> Session {
    let manifest = manifest(seed);
    let mut session = Session::new(&manifest).expect("valid synthetic Sparkmage manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay value")["state"].clone()
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
fn rule_catalog_0802_zero_air_threshold_taps_without_rng_or_damage() {
    let mut session = first_main(417);
    let actions = session.legal_actions().expect("Sparkmage actions");
    let activation = actions
        .iter()
        .find(|action| {
            action.descriptor["kind"] == "activate-sparkmage"
                && action.descriptor["targetLocation"]
                    == json!({ "cell": "C4", "region": "surface" })
        })
        .expect("zero-damage activation");
    assert_eq!(
        actions
            .iter()
            .find(|action| action.action_id == activation.action_id)
            .expect("stable issued action")
            .descriptor,
        activation.descriptor
    );
    assert!(activation.descriptor.get("targetInstanceId").is_none());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor == &activation.descriptor
    });
    assert_eq!(event_types(&receipt), ["sparkmage-activated"]);
    assert!(receipt.random_draws.is_empty());
    assert!(receipt.events[0].payload.get("targetInstanceId").is_none());

    let current = state(&session);
    assert_eq!(current["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(current["players"]["north"]["airThresholdsCastThisTurn"], 0);
    assert!(
        !session
            .legal_actions()
            .expect("post-activation actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "activate-sparkmage")
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one replayed scenario proves counter accumulation, random damage, reset, and zero damage"
)]
fn rule_catalog_0865_air_thresholds_select_hidden_candidate_and_reset() {
    let mut session = first_main(7);
    for expected_threshold in [1, 2] {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-minion"
                && descriptor["cell"] == "C4"
        });
        assert_eq!(
            state(&session)["players"]["north"]["airThresholdsCastThisTurn"],
            expected_threshold
        );
    }

    let before = state(&session);
    let mut candidates = before["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| unit["location"] == "C4" && unit["region"] == "surface")
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("unit identity")
                .to_owned()
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    assert_eq!(candidates.len(), 2);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-sparkmage"
            && descriptor["targetLocation"] == json!({ "cell": "C4", "region": "surface" })
    });
    assert!(descriptor.get("targetInstanceId").is_none());
    assert!(candidates.iter().all(|instance_id| {
        !canonical_json(&descriptor)
            .expect("canonical activation")
            .contains(instance_id)
    }));
    assert_eq!(
        event_types(&receipt),
        ["sparkmage-activated", "damage-dealt"]
    );
    assert!(!receipt.random_draws.is_empty());
    assert!(receipt.random_draws.iter().all(|draw| {
        draw["purpose"] == "sparkmage_random_other_unit_at_nearby_location"
            && draw["domain"]["kind"] == "unit_index_candidate"
            && draw["domain"]["exclusiveMaximum"] == 2
    }));
    let accepted_draw = receipt
        .random_draws
        .iter()
        .rev()
        .find(|draw| draw["domain"]["accepted"] == true)
        .expect("accepted random draw");
    let selected_index = usize::try_from(
        accepted_draw["result"].as_u64().expect("random uint32") % candidates.len() as u64,
    )
    .expect("candidate index");
    let selected = &candidates[selected_index];
    assert_eq!(
        receipt.events[0].payload["targetInstanceId"],
        selected.as_str()
    );
    assert_eq!(receipt.events[1].payload["instanceId"], selected.as_str());

    let damaged = state(&session);
    assert_eq!(
        damaged["realm"]["units"]
            .as_array()
            .expect("damaged units")
            .iter()
            .filter(|unit| unit["damage"] == 2)
            .count(),
        1
    );
    assert_eq!(damaged["players"]["north"]["airThresholdsCastThisTurn"], 2);

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        state(&session)["players"]["north"]["airThresholdsCastThisTurn"],
        0
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let damage_before_zero = state(&session)["realm"]["units"].clone();
    let (_, zero_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-sparkmage"
            && descriptor["targetLocation"] == json!({ "cell": "C4", "region": "surface" })
    });
    assert_eq!(event_types(&zero_receipt), ["sparkmage-activated"]);
    assert_eq!(zero_receipt.random_draws.len(), 1);
    assert!(
        zero_receipt.events[0]
            .payload
            .get("targetInstanceId")
            .is_some()
    );
    assert_eq!(state(&session)["realm"]["units"], damage_before_zero);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0723_rubble_at_nearby_location_is_valid_activate_sparkmage_target() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0723_rubble_at_nearby_location_is_valid_activate_sparkmage_target();
}

fn rain() -> Value {
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

fn deathrite_sparkmage_manifest(seed: u32) -> String {
    let fixture = "sparkmage-deathrite-withheld";
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(true),
            "north-rain": rain(),
            "north-site": site(&["air"]),
            "south-avatar": avatar(false),
            "south-minion": deathrite_minion(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-rain"; 6],
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
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

struct PendingDeathriteSparkmageSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_activate_sparkmage(
    encoded: &str,
) -> Option<PendingDeathriteSparkmageSetup> {
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
    if !offers_kind(&session, "activate-sparkmage") {
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
    Some(PendingDeathriteSparkmageSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_sparkmage_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_sparkmage_manifest)
        .find(|candidate| try_pending_deathrite_with_activate_sparkmage(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with legal activate-sparkmage")
}

#[test]
fn rule_catalog_1137_activate_sparkmage_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_sparkmage_seed_with(1137);
    let mut setup = try_pending_deathrite_with_activate_sparkmage(&encoded)
        .expect("complete activate-sparkmage Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["players"]["north"]["avatar"]["tapped"], false);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "activate-sparkmage")
    );

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
    assert_eq!(resumed["players"]["north"]["avatar"]["tapped"], false);
    assert!(offers_kind(session, "activate-sparkmage"));
    assert_exact_replay(session);
}
