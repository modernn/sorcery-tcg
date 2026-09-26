//! Direct proofs for destroy-target-artifact Magic (RULE-CATALOG-0633–0634,
//! RULE-CATALOG-1048, RULE-CATALOG-2133–2138).
//!
//! Destroy-artifact Magic offers every Artifact in the caster region, whether
//! loose or carried, and moves a real Artifact into its owner's cemetery. It
//! offers no target when the caster region has no Artifact. While Deathrites
//! wait for ordering, destroy-artifact Magic stays withheld until the chain drains.

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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn artifact() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn destroy_spell() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetArtifact": true,
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

fn destroy_artifact_manifest(seed: u32, south_plays_artifact: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-destroy": destroy_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    let south_spell = if south_plays_artifact {
        cards["south-artifact"] = artifact();
        "south-artifact"
    } else {
        cards["south-minion"] = minion();
        "south-minion"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-artifact-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-artifact-magic-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-destroy"; 6],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid destroy-artifact session");
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

fn opening_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"]["north"]["hand"]["spellbook"]
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

fn seed_with(south_plays_artifact: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| destroy_artifact_manifest(seed, south_plays_artifact))
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-destroy")
        })
        .expect("bounded seed with destroy-artifact Magic in the opening hand")
}

fn stage_south_artifact(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C1"
    });
    let artifact_id = state(session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    artifact_id
}

fn stage_south_minion(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    minion_id
}

fn deathrite_destroy_artifact_manifest(seed: u32) -> String {
    let fixture = "destroy-artifact-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_spell(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-destroy",
                    "north-rain",
                    "north-rain",
                    "north-destroy",
                    "north-rain",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-artifact"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-minion", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_destroy_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-destroy", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteDestroyArtifactSetup {
    artifact_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_destroy_artifact_target(
    encoded: &str,
) -> Option<PendingDeathriteDestroyArtifactSetup> {
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C1"
    })?;
    let artifact_id = state(&session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()?
        .to_owned();
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
    if !north_has_destroy_and_rain(&state(&session)) {
        return None;
    }
    if destroy_artifact_targets(&session).is_empty() {
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
    Some(PendingDeathriteDestroyArtifactSetup {
        artifact_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_destroy_artifact_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_destroy_artifact_manifest)
        .find(|candidate| try_pending_deathrite_with_destroy_artifact_target(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with destroy-artifact Magic in hand")
}

fn destroy_artifact_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("destroy-artifact actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-destroy"
        })
        .filter_map(|action| {
            action.descriptor["targetArtifactInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn realm_has_artifact(value: &Value, instance_id: &str) -> bool {
    value["realm"]
        .get("artifacts")
        .and_then(Value::as_array)
        .is_some_and(|artifacts| {
            artifacts
                .iter()
                .any(|artifact| artifact["instanceId"] == instance_id)
        })
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
fn rule_catalog_0633_destroy_target_artifact_moves_a_loose_artifact_to_its_owners_cemetery() {
    let encoded = seed_with(true, 633);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session);
    assert_eq!(
        destroy_artifact_targets(&session).as_slice(),
        std::slice::from_ref(&artifact_id)
    );
    assert!(realm_has_artifact(&state(&session), &artifact_id));

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetArtifactInstanceId"] == artifact_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-destroyed", "magic-resolved"]
    );
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "artifact-destroyed")
        .expect("artifact destruction");
    assert_eq!(destroyed.payload["cardId"], "south-artifact");
    assert_eq!(destroyed.payload["instanceId"], artifact_id);
    assert_eq!(destroyed.payload["owner"], "south");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "artifact-returned-to-hand"
                || event.event_type == "artifact-banished"
                || event.event_type == "minion-died")
    );

    let after = state(&session);
    assert!(!realm_has_artifact(&after, &artifact_id));
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == artifact_id)
    );
    assert!(
        !after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == artifact_id)
    );
    assert_eq!(destroy_artifact_targets(&session), Vec::<String>::new());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0634_destroy_target_artifact_offers_no_target_without_an_artifact() {
    let encoded = seed_with(false, 634);
    let mut session = opening_main(&encoded);
    let minion_id = stage_south_minion(&mut session);
    let before = state(&session);
    assert!(
        before["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    assert_eq!(destroy_artifact_targets(&session), Vec::<String>::new());
    assert!(
        !session
            .legal_actions()
            .expect("post-staging actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-destroy")
    );

    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == minion_id)
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .iter()
            .any(|card| card["cardId"] == "north-destroy")
    );
    assert_exact_replay(&session);
}
#[test]
fn rule_catalog_1048_destroy_artifact_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_destroy_artifact_seed_with(1048);
    let mut setup = try_pending_deathrite_with_destroy_artifact_target(&encoded)
        .expect("complete destroy-artifact Deathrite withheld setup");
    let artifact_id = setup.artifact_id.clone();
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
    assert!(realm_has_artifact(&paused, &artifact_id));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(destroy_artifact_targets(session).is_empty());

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
    assert!(realm_has_artifact(&resumed, &artifact_id));
    assert_eq!(
        destroy_artifact_targets(session).as_slice(),
        std::slice::from_ref(&artifact_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetArtifactInstanceId"] == artifact_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-destroyed", "magic-resolved"]
    );
    assert!(!realm_has_artifact(&state(session), &artifact_id));
    assert_exact_replay(session);
}

fn destroy_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-artifact-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-artifact-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_spell(),
            "north-site": site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-destroy"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-artifact"; 8],
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
        .chain(633..633 + 2048)
        .map(destroy_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-destroy")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-artifact")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with destroy-artifact Magic and required South Artifacts")
}

fn destroy_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-destroy")
                .count()
        })
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

fn artifact_location<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
        .expect("expected realm artifact")
}

fn cast_south_artifact_at(session: &mut Session, cell: &str) -> String {
    let (cast, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == cell
    });
    cast["cardInstanceId"]
        .as_str()
        .expect("destroy-artifact supplemental identity")
        .to_owned()
}

fn setup_c1_with_south_artifacts(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    (0..count)
        .map(|_| cast_south_artifact_at(session, "C1"))
        .collect()
}

fn play_south_site_at(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == cell
    });
}

fn cast_destroy_on(session: &mut Session, artifact_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetArtifactInstanceId"] == artifact_id
    });
    receipt
}

fn try_far_artifact_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    let far_id = setup_c1_with_south_artifacts(&mut session, 1)[0].clone();
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
    play_south_site_at(&mut session, "C2");
    let near_id = cast_south_artifact_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (destroy_spells_in_hand(&state(&session)) >= 1).then_some((session, near_id, far_id))
}

fn seed_for_far_artifact(start: u32) -> String {
    (start..start + 2048)
        .chain(633..633 + 2048)
        .find_map(|seed| {
            let encoded = destroy_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-artifact")
                .count()
                < 2
            {
                return None;
            }
            try_far_artifact_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching destroy-artifact far-artifact setup")
}

fn try_second_destroy_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_south_artifacts(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_destroy_on(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if destroy_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let artifact_id = cast_south_artifact_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    destroy_artifact_targets(&session)
        .contains(&artifact_id)
        .then_some((session, artifact_id))
}

fn seed_for_second_destroy_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(633..633 + 8192)
        .find_map(|seed| {
            let encoded = destroy_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-artifact")
                .count()
                < 2
            {
                return None;
            }
            try_second_destroy_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second destroy-artifact enemy-arrival setup")
}

fn try_second_destroy_new_placement_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_south_artifacts(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_destroy_on(&mut session, &first_id);
    if realm_has_artifact(&state(&session), &first_id) {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if destroy_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let artifact_id = cast_south_artifact_at(&mut session, "C1");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    destroy_artifact_targets(&session)
        .contains(&artifact_id)
        .then_some((session, artifact_id))
}

fn seed_for_second_destroy_new_placement(start: u32) -> String {
    (start..start + 8192)
        .chain(633..633 + 8192)
        .find_map(|seed| {
            let encoded = destroy_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-artifact")
                .count()
                < 2
            {
                return None;
            }
            try_second_destroy_new_placement_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second destroy-artifact new-placement setup")
}

#[test]
fn rule_catalog_2133_artifact_stays_at_the_location_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2133, 1);
    let mut session = opening_main(&encoded);
    let artifact_id = setup_c1_with_south_artifacts(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    assert_eq!(
        artifact_location(&state(&session), &artifact_id)["location"],
        "C1"
    );
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(
        artifact_location(&state(&session), &artifact_id)["location"],
        "C1"
    );
    assert!(realm_has_artifact(&state(&session), &artifact_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2134_second_destroy_offers_no_targets_after_destroying_the_only_artifact() {
    let encoded = (2134..2134 + 8192)
        .chain(633..633 + 8192)
        .find_map(|seed| {
            let candidate = destroy_supplemental_manifest(seed);
            let mut session = opening_main(&candidate);
            let artifact_id = setup_c1_with_south_artifacts(&mut session, 1)[0].clone();
            north_draws_spellbook(&mut session);
            let first = cast_destroy_on(&mut session, &artifact_id);
            if !event_types(&first).contains(&"artifact-destroyed") {
                return None;
            }
            if realm_has_artifact(&state(&session), &artifact_id) {
                return None;
            }
            (destroy_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two destroy-artifact casts after clearing Artifacts");
    let mut session = opening_main(&encoded);
    let artifact_id = setup_c1_with_south_artifacts(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_destroy_on(&mut session, &artifact_id);
    assert!(event_types(&first).contains(&"artifact-destroyed"));
    assert!(!realm_has_artifact(&state(&session), &artifact_id));
    assert!(destroy_spells_in_hand(&state(&session)) >= 1);
    assert_eq!(destroy_artifact_targets(&session), Vec::<String>::new());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-destroy"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2135_second_destroy_destroys_a_newly_arrived_artifact_after_enemy_site_placement() {
    let encoded = seed_for_second_destroy_enemy_arrival(2135);
    let (mut session, artifact_id) = try_second_destroy_enemy_arrival_prefix(&encoded)
        .expect("second destroy-artifact enemy-arrival prefix");
    let receipt = cast_destroy_on(&mut session, &artifact_id);
    assert!(event_types(&receipt).contains(&"artifact-destroyed"));
    assert!(!realm_has_artifact(&state(&session), &artifact_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2136_destroy_artifact_offers_every_artifact_in_the_caster_region() {
    let encoded = supplemental_seed_with_start(2136, 2);
    let mut session = opening_main(&encoded);
    let artifact_ids = setup_c1_with_south_artifacts(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = destroy_artifact_targets(&session);
    for artifact_id in &artifact_ids {
        assert!(offered.contains(artifact_id));
    }
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2137_destroy_artifact_leaves_a_far_artifact_untouched() {
    let encoded = seed_for_far_artifact(2137);
    let (mut session, near_id, far_id) =
        try_far_artifact_prefix(&encoded).expect("destroy-artifact far-artifact prefix");
    let receipt = cast_destroy_on(&mut session, &near_id);
    assert!(event_types(&receipt).contains(&"artifact-destroyed"));
    assert!(!realm_has_artifact(&state(&session), &near_id));
    assert_eq!(
        artifact_location(&state(&session), &far_id)["location"],
        "C1"
    );
    assert!(realm_has_artifact(&state(&session), &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2138_second_destroy_destroys_a_newly_placed_artifact() {
    let encoded = seed_for_second_destroy_new_placement(2138);
    let (mut session, artifact_id) = try_second_destroy_new_placement_prefix(&encoded)
        .expect("second destroy-artifact new-placement prefix");
    let receipt = cast_destroy_on(&mut session, &artifact_id);
    assert!(event_types(&receipt).contains(&"artifact-destroyed"));
    assert!(!realm_has_artifact(&state(&session), &artifact_id));
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == artifact_id))
    );
    assert_exact_replay(&session);
}
