//! Direct proofs for cemetery Artifact return Magic (RULE-CATALOG-0637–0638,
//! RULE-CATALOG-1060, RULE-CATALOG-2153–2158).
//!
//! Cemetery Artifact return offers only Artifacts in the caster's own cemetery
//! and restores the chosen instance to the hidden Spellbook hand. A cemetery
//! that holds only Magic is still a paid no-choice resolution. While Deathrites
//! wait for ordering, cemetery Artifact return Magic stays withheld until the
//! chain drains. Supplemental 2153–2158 bind persistence, empty-cemetery
//! repeat, enemy-arrival, multi-artifact offer, unselected remainder, and a
//! newly destroyed Artifact.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn relic() -> Value {
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
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
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

fn return_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetArtifactFromOwnCemetery": true,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn cemetery_artifact_manifest(seed: u32, include_setup: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-return": return_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-minion": minion(),
        "south-site": site(),
    });
    let north_spellbook = if include_setup {
        cards["north-destroy"] = destroy_spell();
        cards["north-relic"] = relic();
        json!([
            "north-relic",
            "north-destroy",
            "north-return",
            "north-relic",
            "north-destroy",
            "north-return"
        ])
    } else {
        json!(vec!["north-return"; 6])
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-artifact-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-artifact-return-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
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
    let mut session = Session::new(encoded).expect("valid cemetery-artifact session");
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

fn cemetery_artifact_cast_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("cemetery artifact actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return"
        })
        .filter_map(|action| {
            action.descriptor["cemeteryMinionInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn north_hand_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn seed_with(required: &[&str], include_setup: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| cemetery_artifact_manifest(seed, include_setup))
        .find(|candidate| {
            Session::new(candidate).is_ok_and(|preview| {
                let snapshot = state(&preview);
                let hand = north_hand_ids(&snapshot);
                required.iter().all(|id| hand.iter().any(|card| card == id))
            })
        })
        .expect("bounded seed with required opening cards")
}

fn setup_own_cemetery_artifact(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C4"
    });
    let relic_id = state(session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("relic identity")
        .to_owned();
    let (_, destroyed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetArtifactInstanceId"] == relic_id
    });
    assert!(
        destroyed
            .events
            .iter()
            .any(|event| event.event_type == "artifact-destroyed")
    );
    let destroy_id = state(session)["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("destroy Magic in cemetery")["instanceId"]
        .as_str()
        .expect("destroy identity")
        .to_owned();
    (relic_id, destroy_id)
}

fn deathrite_cemetery_artifact_manifest(seed: u32) -> String {
    let fixture = "cemetery-artifact-return-deathrite-withheld";
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
            "north-relic": relic(),
            "north-return": return_spell(),
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
                    "north-relic",
                    "north-destroy",
                    "north-return",
                    "north-rain",
                    "north-rain",
                    "north-return",
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

fn north_has_return_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-return", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn cemetery_has_card(snapshot: &Value, owner: &str, instance_id: &str) -> bool {
    snapshot["players"][owner]["cemetery"]
        .as_array()
        .is_some_and(|cemetery| {
            cemetery
                .iter()
                .any(|card| card["instanceId"] == instance_id)
        })
}

struct PendingDeathriteCemeteryArtifactSetup {
    deathrite_ids: [String; 2],
    relic_id: String,
    session: Session,
}

fn try_pending_deathrite_with_cemetery_artifact_target(
    encoded: &str,
) -> Option<PendingDeathriteCemeteryArtifactSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let opening_hand = north_hand_ids(&state(&session));
    if !opening_hand.iter().any(|card| card == "north-relic")
        || !opening_hand.iter().any(|card| card == "north-destroy")
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C4"
    })?;
    let relic_id = state(&session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetArtifactInstanceId"] == relic_id
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
    if !north_has_return_and_rain(&state(&session)) {
        return None;
    }
    if cemetery_artifact_cast_ids(&session).is_empty() {
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
    Some(PendingDeathriteCemeteryArtifactSetup {
        deathrite_ids,
        relic_id,
        session,
    })
}

fn deathrite_cemetery_artifact_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cemetery_artifact_manifest)
        .find(|candidate| try_pending_deathrite_with_cemetery_artifact_target(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with cemetery-artifact return Magic in hand")
}

#[test]
fn rule_catalog_0637_cemetery_artifact_return_restores_own_cemetery_artifact_to_hidden_hand() {
    let encoded = seed_with(&["north-relic", "north-destroy", "north-return"], true, 637);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_artifact_cast_ids(&session), Vec::<String>::new());
    let (relic_id, destroy_id) = setup_own_cemetery_artifact(&mut session);
    assert_ne!(destroy_id, relic_id);
    let cemetery_targets = cemetery_artifact_cast_ids(&session);
    assert!(!cemetery_targets.is_empty());
    assert!(cemetery_targets.iter().all(|id| id == &relic_id));
    assert!(!cemetery_targets.iter().any(|id| id == &destroy_id));
    let before = state(&session);
    let before_hand_count = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let south_observation = session.observe(Seat::South);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == relic_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["cardId"], "north-relic");
    assert_eq!(receipt.events[1].payload["instanceId"], relic_id);
    assert_eq!(receipt.events[1].payload["owner"], "north");
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-returned-to-hand"
                || event.event_type == "magic-returned-to-hand")
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        before_hand_count
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == relic_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != relic_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == destroy_id)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_view["players"]["north"]["hand"]["spellbook"], 1);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("cemetery-artifact checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized cemetery-artifact");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed cemetery-artifact");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed cemetery-artifact session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0638_cemetery_artifact_return_is_a_paid_noop_without_cemetery_artifact() {
    let encoded = seed_with(&["north-return"], false, 638);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_artifact_cast_ids(&session), Vec::<String>::new());
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "artifact-returned-to-hand"
                || event.event_type == "magic-returned-to-hand")
    );
    assert_eq!(cemetery_artifact_cast_ids(&session), Vec::<String>::new());
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        1
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1060_cemetery_artifact_return_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cemetery_artifact_seed_with(1060);
    let mut setup = try_pending_deathrite_with_cemetery_artifact_target(&encoded)
        .expect("complete cemetery-artifact return Deathrite withheld setup");
    let relic_id = setup.relic_id.clone();
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
    assert!(cemetery_has_card(&paused, "north", &relic_id));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(cemetery_artifact_cast_ids(session).is_empty());

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
    assert!(cemetery_has_card(&resumed, "north", &relic_id));
    assert_eq!(
        cemetery_artifact_cast_ids(session).as_slice(),
        std::slice::from_ref(&relic_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == relic_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-returned-to-hand", "magic-resolved"]
    );
    assert!(!cemetery_has_card(&state(session), "north", &relic_id));
    assert_exact_replay(session);
}

fn cemetery_artifact_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-artifact-return-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-artifact-return-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_spell(),
            "north-relic": relic(),
            "north-return": return_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-relic",
                    "north-destroy",
                    "north-return",
                    "north-return",
                    "north-relic",
                    "north-destroy",
                    "north-relic",
                    "north-return",
                ],
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

fn opening_hand_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    north_hand_ids(&state(&preview))
}

fn supplemental_seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(637..637 + 2048)
        .map(cemetery_artifact_supplemental_manifest)
        .find(|candidate| {
            required.iter().all(|id| {
                opening_hand_spell_ids(candidate)
                    .iter()
                    .any(|card| card == *id)
            })
        })
        .expect("bounded seed with cemetery Artifact return supplemental opening cards")
}

fn return_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-return")
                .count()
        })
        .unwrap_or_default()
}

fn cemetery_artifact_instance_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .map(|cemetery| {
            cemetery
                .iter()
                .filter(|card| card["cardId"] == "north-relic")
                .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn hand_has_instance(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["instanceId"] == instance_id))
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .is_ok_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn decline_attack_if_needed(session: &mut Session) {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

fn try_opening_main(encoded: &str) -> Option<Session> {
    let mut session = Session::new(encoded).ok()?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    Some(session)
}

fn try_end_turn_if_offered(session: &mut Session) -> Option<()> {
    decline_attack_if_needed(session);
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    Some(())
}

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    try_end_turn_if_offered(session)?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    try_end_turn_if_offered(session)?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    try_pass_turn_to_north_spellbook(session).expect("north Spellbook draw");
}

fn try_advance_north_spellbook_draws(session: &mut Session, draws: usize) -> Option<()> {
    for _ in 0..draws {
        try_pass_turn_to_north_spellbook(session)?;
    }
    Some(())
}

fn try_destroy_own_relic(session: &mut Session, avoid: &[String]) -> Option<String> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C4"
            && !avoid.iter().any(|id| descriptor["cardInstanceId"] == *id)
    })?;
    let relic_id = state(session)["realm"]["artifacts"]
        .as_array()?
        .iter()
        .rev()
        .find(|artifact| artifact["cardId"] == "north-relic")?["instanceId"]
        .as_str()?
        .to_owned();
    let (_, destroyed) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetArtifactInstanceId"] == relic_id
    })?;
    event_types(&destroyed)
        .contains(&"artifact-destroyed")
        .then_some(relic_id)
}

fn destroy_own_relic(session: &mut Session) -> String {
    try_destroy_own_relic(session, &[]).expect("own cemetery Artifact")
}

fn try_cast_return_target(session: &mut Session, relic_id: &str) -> Option<Receipt> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == relic_id
    })
    .map(|(_, receipt)| receipt)
}

fn cast_return_target(session: &mut Session, relic_id: &str) -> Receipt {
    try_cast_return_target(session, relic_id).expect("cemetery Artifact return")
}

fn try_two_cemetery_artifacts_on_session(session: &mut Session) -> Option<[String; 2]> {
    let first = try_destroy_own_relic(session, &[])?;
    let mut second = None;
    for _ in 0..4 {
        if let Some(id) = try_destroy_own_relic(session, std::slice::from_ref(&first)) {
            second = Some(id);
            break;
        }
        try_pass_turn_to_north_spellbook(session)?;
    }
    let second = second?;
    (cemetery_artifact_instance_ids(&state(session), "north").len() == 2).then_some([first, second])
}

fn seed_with_two_cemetery_artifacts(start: u32) -> String {
    (start..start + 8192)
        .chain(637..637 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_artifact_supplemental_manifest(seed);
            if !["north-relic", "north-destroy", "north-return"]
                .iter()
                .all(|id| {
                    opening_hand_spell_ids(&encoded)
                        .iter()
                        .any(|card| card == *id)
                })
            {
                return None;
            }
            let mut session = try_opening_main(&encoded)?;
            try_two_cemetery_artifacts_on_session(&mut session).map(|_| encoded)
        })
        .expect("bounded seed with two own cemetery Artifacts")
}

fn prepare_two_cemetery_artifacts(encoded: &str) -> (Session, [String; 2]) {
    let mut session = try_opening_main(encoded).expect("opening main");
    let relics =
        try_two_cemetery_artifacts_on_session(&mut session).expect("two own cemetery Artifacts");
    (session, relics)
}

fn try_second_return_empty_prefix(encoded: &str) -> Option<Session> {
    let mut session = try_opening_main(encoded)?;
    let relic_id = try_destroy_own_relic(&mut session, &[])?;
    try_cast_return_target(&mut session, &relic_id)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    (return_spells_in_hand(&state(&session)) >= 1
        && cemetery_artifact_instance_ids(&state(&session), "north").is_empty())
    .then_some(session)
}

fn seed_for_second_return_empty(start: u32) -> String {
    (start..start + 8192)
        .chain(637..637 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_artifact_supplemental_manifest(seed);
            try_second_return_empty_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Artifact return empty-cemetery setup")
}

fn try_second_return_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = try_opening_main(encoded)?;
    let first_id = try_destroy_own_relic(&mut session, &[])?;
    try_cast_return_target(&mut session, &first_id)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    if return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_end_turn_if_offered(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    try_end_turn_if_offered(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_advance_north_spellbook_draws(&mut session, 2)?;
    let second_id = try_destroy_own_relic(&mut session, &[first_id])?;
    (return_spells_in_hand(&state(&session)) >= 1
        && cemetery_artifact_cast_ids(&session).contains(&second_id))
    .then_some((session, second_id))
}

fn seed_for_second_return_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(637..637 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_artifact_supplemental_manifest(seed);
            try_second_return_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Artifact return enemy-arrival setup")
}

fn try_second_return_new_destroy_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = try_opening_main(encoded)?;
    let first_id = try_destroy_own_relic(&mut session, &[])?;
    try_cast_return_target(&mut session, &first_id)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    if return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_advance_north_spellbook_draws(&mut session, 2)?;
    let second_id = try_destroy_own_relic(&mut session, &[first_id])?;
    (return_spells_in_hand(&state(&session)) >= 1
        && cemetery_artifact_cast_ids(&session).contains(&second_id))
    .then_some((session, second_id))
}

fn seed_for_second_return_new_destroy(start: u32) -> String {
    (start..start + 8192)
        .chain(637..637 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_artifact_supplemental_manifest(seed);
            try_second_return_new_destroy_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Artifact return new-destroy setup")
}

#[test]
fn rule_catalog_2153_returned_artifact_stays_in_hand_after_turns_pass() {
    let encoded =
        supplemental_seed_with_start(2153, &["north-relic", "north-destroy", "north-return"]);
    let mut session = opening_main(&encoded);
    let relic_id = destroy_own_relic(&mut session);
    cast_return_target(&mut session, &relic_id);
    assert!(hand_has_instance(&state(&session), "north", &relic_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(hand_has_instance(&state(&session), "north", &relic_id));
    assert!(cemetery_artifact_instance_ids(&state(&session), "north").is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2154_second_return_without_a_cemetery_artifact_is_a_paid_noop() {
    let encoded = seed_for_second_return_empty(2154);
    let mut session = try_second_return_empty_prefix(&encoded)
        .expect("second cemetery Artifact return empty prefix");
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "artifact-returned-to-hand")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2155_second_return_restores_a_newly_destroyed_artifact_after_enemy_site_placement()
{
    let encoded = seed_for_second_return_enemy_arrival(2155);
    let (mut session, relic_id) = try_second_return_enemy_arrival_prefix(&encoded)
        .expect("second cemetery Artifact return enemy-arrival prefix");
    let receipt = cast_return_target(&mut session, &relic_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), "north", &relic_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2156_cemetery_artifact_return_offers_every_own_cemetery_artifact() {
    let encoded = seed_with_two_cemetery_artifacts(2156);
    let (session, relic_ids) = prepare_two_cemetery_artifacts(&encoded);
    let mut offered = cemetery_artifact_cast_ids(&session);
    offered.sort();
    offered.dedup();
    assert_eq!(offered.len(), 2);
    for relic_id in &relic_ids {
        assert!(offered.contains(relic_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2157_cemetery_artifact_return_leaves_an_unselected_cemetery_artifact_in_place() {
    let encoded = seed_with_two_cemetery_artifacts(2157);
    let (mut session, relic_ids) = prepare_two_cemetery_artifacts(&encoded);
    let returned_id = &relic_ids[0];
    cast_return_target(&mut session, returned_id);
    assert!(hand_has_instance(&state(&session), "north", returned_id));
    let remaining = cemetery_artifact_instance_ids(&state(&session), "north");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0], relic_ids[1]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2158_second_return_restores_a_newly_destroyed_artifact() {
    let encoded = seed_for_second_return_new_destroy(2158);
    let (mut session, relic_id) = try_second_return_new_destroy_prefix(&encoded)
        .expect("second cemetery Artifact return new-destroy prefix");
    let receipt = cast_return_target(&mut session, &relic_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), "north", &relic_id));
    assert_exact_replay(&session);
}
