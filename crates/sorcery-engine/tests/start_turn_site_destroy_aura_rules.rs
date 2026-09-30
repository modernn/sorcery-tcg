//! Direct proofs for start-turn occupied-site Aura destruction (RULE-CATALOG-0258–0259,
//! RULE-CATALOG-1241).
//!
//! Official cards such as Hamlet's Ablaze conjure atop an Ordinary or Exceptional site.
//! At the start of the controller's next turn the Aura destroys that site, the minions
//! standing atop it, and itself. Avatars are not minions. Unique or Legendary sites
//! cannot be targeted.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::game::{Game, GameError};
use sorcery_engine::session::{Session, SessionError, StepResult};

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

fn replay_game_for_session(session: &Session) -> Game {
    let mut game = Game::from_manifest_json(session.manifest_json()).expect("Game fixture");
    for receipt in session.transcript() {
        let action = game
            .legal_actions()
            .expect("Ignore-path legal actions")
            .into_iter()
            .find(|action| {
                action
                    .to_legal_action()
                    .expect("materialized Ignore-path action")
                    .action_id
                    == receipt.action_id
            })
            .expect("Session-issued action exists in Ignore path");
        game.apply_action(&action)
            .expect("Ignore-path setup action");
    }
    game
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

fn all_warded_manifest() -> String {
    let mut value: Value = serde_json::from_str(&manifest(false)).expect("base fixture manifest");
    let mut north_minion = minion();
    north_minion["ward"] = json!(true);
    value["cards"]["north-ward-minion"] = north_minion;
    value["cards"]["south-minion"]["ward"] = json!(true);
    value["decks"]["north"]["spellbook"] = json!([
        "north-ward-minion",
        "north-ward-minion",
        "north-ward-minion",
        "north-aura",
        "north-aura",
        "north-aura",
    ]);
    value["seed"] = json!(1);
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).expect("warded fixture manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical warded manifest")
}

fn protection_lost_manifest() -> String {
    let mut value: Value = serde_json::from_str(&all_warded_manifest()).unwrap();
    value["cards"]["north-freeze"] = json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["decks"]["north"]["spellbook"] = json!([
        "north-aura",
        "north-ward-minion",
        "north-aura",
        "north-aura",
        "north-aura",
        "north-freeze",
    ]);
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).expect("protection-lost manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).unwrap()
}

fn after_aura_disabling_its_warded_minion() -> Session {
    let encoded = protection_lost_manifest();
    let mut session = Session::new(&encoded).expect("valid protection-lost session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (_, summon) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ward-minion"
            && descriptor["cell"] == "C4"
    });
    let minion_id = summon
        .events
        .iter()
        .find(|event| event.event_type == "minion-summoned")
        .expect("issued protected minion summon")
        .payload["instanceId"]
        .clone();
    let (_, disabled) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == minion_id
    });
    assert!(disabled.events.iter().any(|event| {
        event.event_type == "minion-disabled" && event.payload["wardRemoved"] == true
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn after_aura_with_warded_minion_on_c4() -> Session {
    let encoded = all_warded_manifest();
    after_aura_with_manifest(&encoded)
}

fn after_aura_with_manifest(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid protected-recipient session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ward-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
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
fn rule_catalog_0258_start_turn_site_minion_destruction_is_unsupported_and_rolls_back() {
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

    let before_north = session.public_view(Seat::North).unwrap();
    let before_south = session.public_view(Seat::South).unwrap();
    let before_hash = session.state_hash().unwrap();
    let before_draws_hash = session.initial_random_draws_hash().unwrap();
    let before_transcript = session.transcript().to_vec();
    let before_attempts = session.attempts().to_vec();
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("issued occupied-site Aura trigger");
    let result = session.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    });
    assert!(matches!(
        result,
        Err(SessionError::Game(GameError::UnsupportedMechanic(reason)))
            if reason == "start-turn site and minion destruction overlap"
    ));
    assert_eq!(session.public_view(Seat::North).unwrap(), before_north);
    assert_eq!(session.public_view(Seat::South).unwrap(), before_south);
    assert_eq!(session.state_hash().unwrap(), before_hash);
    assert_eq!(
        session.initial_random_draws_hash().unwrap(),
        before_draws_hash
    );
    assert_eq!(session.transcript(), before_transcript);
    assert_eq!(session.attempts(), before_attempts);
    assert!(session.unsupported_mechanic().is_some());
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one protected start-turn destruction and ward interaction proof"
)]
fn protected_start_turn_destruction_consumes_ward_without_marking_minions() {
    let mut session = after_aura_with_warded_minion_on_c4();
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

    let before = state(&session);
    let before_hash = session.state_hash().expect("pre-trigger hash");
    let mut ignored = replay_game_for_session(&session);
    assert_eq!(ignored.authoritative_state(), before);
    assert_eq!(ignored.state_hash().unwrap(), before_hash);
    let checkpoint_before =
        create_game_checkpoint(&session).expect("pre-effect protected trigger checkpoint");
    let encoded_before =
        serialize_game_checkpoint(&checkpoint_before).expect("serialize pre-effect checkpoint");
    let parsed_before =
        parse_game_checkpoint(&encoded_before).expect("parse pre-effect checkpoint");
    let mut restored_before =
        resume_game_checkpoint(&parsed_before).expect("resume pre-effect checkpoint");
    assert_eq!(
        restored_before.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
    assert_eq!(
        restored_before.legal_actions().unwrap(),
        session.legal_actions().unwrap()
    );
    assert_eq!(restored_before.transcript(), session.transcript());
    let action = session
        .legal_actions()
        .expect("protected start-turn actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("issued occupied-site Aura trigger");
    let ignore_action = ignored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.to_legal_action().unwrap().action_id == action.action_id)
        .expect("same issued trigger in Ignore path");
    ignored
        .apply_action(&ignore_action)
        .expect("Ignore-path protected trigger");
    let restored_action = restored_before
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.action_id == action.action_id)
        .expect("same issued trigger after checkpoint resume");
    let StepResult::Accepted(restored_receipt) = restored_before
        .step(ActionRequest {
            action_id: restored_action.action_id.to_string(),
            seat: restored_action.seat,
            state_version: restored_action.state_version,
        })
        .expect("resumed protected trigger")
    else {
        panic!("resumed issued trigger must be accepted");
    };
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("Ward-protected destruction should resolve")
    else {
        panic!("issued start-turn trigger must be accepted");
    };

    assert_eq!(
        serde_json::to_value(&restored_receipt).unwrap(),
        serde_json::to_value(&receipt).unwrap()
    );
    assert_eq!(
        restored_before.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
    assert_eq!(restored_before.transcript(), session.transcript());
    let warded_recipients = before["realm"]["units"]
        .as_array()
        .expect("two actual Surface recipients")
        .iter()
        .map(|unit| (unit["instanceId"].clone(), unit["controller"].clone()))
        .collect::<Vec<_>>();
    assert_eq!(warded_recipients.len(), 2);
    let ward_events = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "ward-broken")
        .map(|event| event.payload.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        ward_events,
        warded_recipients
            .iter()
            .map(|(instance_id, controller)| json!({
                "instanceId": instance_id,
                "seat": controller,
            }))
            .collect::<Vec<_>>()
    );
    assert!(
        receipt.events.iter().all(|event| {
            event.event_type != "minion-died" && event.event_type != "minion-killed"
        })
    );
    let after = state(&session);
    assert_eq!(ignored.authoritative_state(), after);
    assert_eq!(ignored.state_hash().unwrap(), session.state_hash().unwrap());
    assert_eq!(
        ignored
            .legal_actions()
            .unwrap()
            .iter()
            .map(|action| action.to_legal_action().unwrap())
            .collect::<Vec<_>>(),
        session.legal_actions().unwrap()
    );
    for (id, _) in &warded_recipients {
        let unit = after["realm"]["units"]
            .as_array()
            .expect("surviving protected units")
            .iter()
            .find(|unit| unit["instanceId"] == *id)
            .expect("Ward-protected recipient remains live");
        assert_eq!(unit["warded"], false);
        assert_ne!(unit["deathMarked"], true);
    }
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["realm"]["sites"]["C4"]["rubble"], true);
    assert!(after["realm"].get("auras").is_none());
    assert_ne!(
        session.state_hash().expect("post-trigger hash"),
        before_hash
    );
    for seat in ["north", "south"] {
        assert!(
            state(&session)["players"][seat]["cemetery"]
                .as_array()
                .expect("owner cemetery")
                .iter()
                .all(|card| warded_recipients
                    .iter()
                    .all(|(id, _)| card["instanceId"] != *id))
        );
    }
    let checkpoint = create_game_checkpoint(&session).expect("protected trigger checkpoint");
    let encoded_checkpoint =
        serialize_game_checkpoint(&checkpoint).expect("serialize protected trigger checkpoint");
    let parsed = parse_game_checkpoint(&encoded_checkpoint).expect("parse protected checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("resume protected checkpoint");
    assert_eq!(
        restored.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
    assert_eq!(
        serde_json::to_value(restored.legal_actions().unwrap()).unwrap(),
        serde_json::to_value(session.legal_actions().unwrap()).unwrap()
    );
    assert_eq!(restored.transcript(), session.transcript());
    assert_exact_replay(&restored);
    assert_exact_replay(&session);
}

#[test]
fn protected_and_unprotected_start_turn_cohort_rolls_back_before_mutation() {
    let mut value: Value = serde_json::from_str(&all_warded_manifest()).unwrap();
    value["cards"]["south-minion"]
        .as_object_mut()
        .unwrap()
        .remove("ward");
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).expect("mixed fixture identity"));
    let encoded = sorcery_engine::canonical::canonical_json(&value).unwrap();
    let mut session = after_aura_with_manifest(&encoded);
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let before_state = state(&session);
    let mut ignored = replay_game_for_session(&session);
    assert_eq!(ignored.authoritative_state(), before_state);
    let before_north_view = session.public_view(Seat::North).unwrap();
    let before_south_view = session.public_view(Seat::South).unwrap();
    let before_hash = session.state_hash().unwrap();
    let before_draws_hash = session.initial_random_draws_hash().unwrap();
    let before_transcript = session.transcript().to_vec();
    let before_attempts = session.attempts().to_vec();
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("issued occupied-site Aura trigger");
    let ignored_action = ignored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.to_legal_action().unwrap().action_id == action.action_id)
        .expect("same issued mixed trigger in Ignore path");
    let ignored_result = ignored.apply_action(&ignored_action);
    let result = session.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    });
    assert!(matches!(
        result,
        Err(SessionError::Game(GameError::UnsupportedMechanic(reason)))
            if reason == "start-turn site and minion destruction overlap"
    ));
    assert!(
        matches!(ignored_result, Err(GameError::UnsupportedMechanic(reason))
        if reason == "start-turn site and minion destruction overlap")
    );
    assert_eq!(ignored.authoritative_state(), before_state);
    assert_eq!(ignored.state_hash().unwrap(), before_hash);
    assert_eq!(session.public_view(Seat::North).unwrap(), before_north_view);
    assert_eq!(session.public_view(Seat::South).unwrap(), before_south_view);
    assert_eq!(session.state_hash().unwrap(), before_hash);
    assert_eq!(
        session.initial_random_draws_hash().unwrap(),
        before_draws_hash
    );
    assert_eq!(session.transcript(), before_transcript);
    assert_eq!(session.attempts(), before_attempts);
    assert!(session.unsupported_mechanic().is_some());
}

#[test]
fn disabled_ward_recipient_still_hits_the_fresh_unprotected_guard() {
    let mut session = after_aura_disabling_its_warded_minion();
    let source_id = aura_id(&session);
    let protected_id = state(&session)["realm"]["units"][0]["instanceId"].clone();
    assert_eq!(state(&session)["realm"]["units"][0]["warded"], false);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let before_north = session.public_view(Seat::North).unwrap();
    let before_south = session.public_view(Seat::South).unwrap();
    let before_hash = session.state_hash().unwrap();
    let before_transcript = session.transcript().to_vec();
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("issued occupied-site Aura trigger");
    let result = session.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    });
    assert!(matches!(result,
        Err(SessionError::Game(GameError::UnsupportedMechanic(reason)))
            if reason == "start-turn site and minion destruction overlap"));
    assert_eq!(session.public_view(Seat::North).unwrap(), before_north);
    assert_eq!(session.public_view(Seat::South).unwrap(), before_south);
    assert_eq!(session.state_hash().unwrap(), before_hash);
    assert_eq!(session.transcript(), before_transcript);
    assert_eq!(
        session.unsupported_mechanic(),
        Some("start-turn site and minion destruction overlap")
    );
    assert_eq!(
        session.public_view(Seat::North).unwrap()["realm"]["units"][0]["instanceId"],
        protected_id
    );
}

#[test]
fn warded_minion_off_the_aura_cell_keeps_ward_and_is_not_a_recipient() {
    let mut session = after_aura_with_warded_minion_on_c4();
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let before = state(&session);
    let off_cell = before["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["controller"] == "south")
        .expect("off-cell Ward minion")["instanceId"]
        .clone();
    let receipt = resolve_start_turn_destroy(&mut session, &source_id);
    let wards = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "ward-broken")
        .map(|event| event.payload["instanceId"].clone())
        .collect::<Vec<_>>();
    assert_eq!(wards.len(), 1);
    assert_ne!(wards[0], off_cell);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["instanceId"] == off_cell)
        .expect("unrelated minion remains live");
    assert_eq!(unit["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn unwarded_minion_off_the_aura_cell_is_not_a_recipient() {
    let mut value: Value = serde_json::from_str(&all_warded_manifest()).unwrap();
    value["cards"]["south-minion"]
        .as_object_mut()
        .unwrap()
        .remove("ward");
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] =
        json!(identity_hash(&value).expect("unwarded off-cell manifest identity"));
    let encoded = sorcery_engine::canonical::canonical_json(&value).unwrap();
    let mut session = after_aura_with_manifest(&encoded);
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    let off_cell = summon["cardInstanceId"].clone();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["instanceId"] == off_cell)
            .expect("issued off-cell minion")["warded"],
        false
    );

    let receipt = resolve_start_turn_destroy(&mut session, &source_id);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "ward-broken")
            .count(),
        1,
        "only the Ward-protected in-footprint minion consumes Ward"
    );
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["instanceId"] == off_cell)
        .expect("unwarded off-cell minion remains live");
    assert_eq!(unit["warded"], false);
    assert_eq!(unit.get("deathMarked"), None);
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
    if state(&session)["phase"] != "trigger-order" {
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
    assert_eq!(state(&session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger")
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
