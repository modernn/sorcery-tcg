use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::game::GameError;
use sorcery_engine::session::{Session, SessionError, StepResult};

struct MountainPassSetup {
    north_airborne_id: String,
    north_avatar_id: String,
    north_ground_id: String,
    north_occupant_card_id: String,
    session: Session,
    south_airborne_id: String,
    south_disabled_card_id: String,
    south_ground_id: String,
    south_leaver_card_id: String,
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

fn site(mountain_pass: bool) -> Value {
    let mut value = json!({ "cardType": "site", "elements": ["earth"] });
    if mountain_pass {
        value["blocksGroundMinionEntryWhileMinionAtop"] = json!(true);
    }
    value
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn manifest() -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-ground": minion(json!({})),
        "north-airborne": minion(json!({ "airborne": true })),
        "north-occupant": minion(json!({ "summonToAnySite": true })),
        "north-site": site(false),
        "south-avatar": avatar(),
        "south-ground": minion(json!({})),
        "south-airborne": minion(json!({ "airborne": true })),
        "south-disabled": minion(json!({
            "genesisDisableSelfUntilDamaged": true,
            "summonToAnySite": true,
        })),
        "south-leaver": minion(json!({
            "cannotDefendOrIntercept": true,
            "summonToAnySite": true,
        })),
        "south-pass": site(true),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "mountain-pass-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-mountain-pass-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ground", "north-airborne", "north-occupant"],
            },
            "south": {
                "atlas": vec!["south-pass"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-ground",
                    "south-airborne",
                    "south-disabled",
                    "south-leaver",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 161,
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
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

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("Mountain Pass checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn summon(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned()
}

fn move_one_step(session: &mut Session, instance_id: &str, from: &str, to: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["path"].as_array().is_some_and(|path| {
                path.iter()
                    .map(|location| location["cell"].as_str().expect("path cell"))
                    .eq([from, to])
            })
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
}

fn prepare_mountain_pass() -> MountainPassSetup {
    let mut session = Session::new(&manifest()).expect("valid Mountain Pass scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let north_ground_id = summon(&mut session, "north-ground", "C4");
    let north_airborne_id = summon(&mut session, "north-airborne", "C4");
    let north_avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let north_occupant_card_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-occupant")
        .and_then(|card| card["cardId"].as_str())
        .expect("North occupant card")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let south_ground_id = summon(&mut session, "south-ground", "C1");
    let south_airborne_id = summon(&mut session, "south-airborne", "C1");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    move_one_step(&mut session, &north_ground_id, "C4", "C3");
    move_one_step(&mut session, &north_airborne_id, "C4", "C3");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let south_hand = state(&session)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South hand")
        .clone();
    let card_id = |wanted: &str| {
        south_hand
            .iter()
            .find(|card| card["cardId"] == wanted)
            .and_then(|card| card["cardId"].as_str())
            .expect("expected South card")
            .to_owned()
    };
    MountainPassSetup {
        north_airborne_id,
        north_avatar_id,
        north_ground_id,
        north_occupant_card_id,
        session,
        south_airborne_id,
        south_disabled_card_id: card_id("south-disabled"),
        south_ground_id,
        south_leaver_card_id: card_id("south-leaver"),
    }
}

fn begin_north_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn entry_actor_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("movement actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["path"].as_array().is_some_and(|path| {
                    path.iter()
                        .map(|location| location["cell"].as_str().expect("path cell"))
                        .eq(["C3", "C2"])
                })
        })
        .map(|action| {
            action.descriptor["unitInstanceId"]
                .as_str()
                .expect("moving identity")
                .to_owned()
        })
        .collect()
}

#[test]
fn mountain_pass_should_block_only_occupied_ground_minion_entry() {
    let setup = prepare_mountain_pass();

    let mut empty = setup.session.clone();
    begin_north_turn(&mut empty);
    let mut expected_empty = vec![
        setup.north_airborne_id.clone(),
        setup.north_ground_id.clone(),
    ];
    expected_empty.sort();
    assert_eq!(entry_actor_ids(&empty), expected_empty);
    assert_exact_replay(&empty);

    let mut occupied = setup.session;
    let disabled_id = summon(&mut occupied, &setup.south_disabled_card_id, "C2");
    let occupied_state = state(&occupied);
    let disabled = occupied_state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == disabled_id)
        .expect("disabled occupant");
    assert_eq!(disabled["controller"], "south");
    assert_eq!(disabled["region"], "surface");
    assert_eq!(disabled["disabledUntilDamaged"], true);
    begin_north_turn(&mut occupied);
    let mut expected_occupied = vec![setup.north_airborne_id.clone()];
    expected_occupied.sort();
    assert_eq!(entry_actor_ids(&occupied), expected_occupied);
    assert_checkpoint_round_trip(&occupied);

    let mut avatar_entry = occupied.clone();
    move_one_step(&mut avatar_entry, &setup.north_avatar_id, "C4", "C3");
    accept_where(&mut avatar_entry, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut avatar_entry, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut avatar_entry, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut avatar_entry, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    assert!(
        avatar_entry
            .legal_actions()
            .expect("Avatar entry actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == setup.north_avatar_id
                && action.descriptor["from"]["cell"] == "C3"
                && action.descriptor["to"]["cell"] == "C2")
    );
    accept_where(&mut avatar_entry, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.north_avatar_id
            && descriptor["to"]["cell"] == "C2"
    });
    assert_eq!(state(&avatar_entry)["pendingCombat"]["cell"], "C2");
    assert_exact_replay(&avatar_entry);

    accept_where(&mut occupied, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.north_airborne_id
            && descriptor["path"].as_array().is_some_and(|path| {
                path.iter()
                    .map(|location| location["cell"].as_str().expect("path cell"))
                    .eq(["C3", "C2"])
            })
    });
    accept_where(&mut occupied, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "site"
    });
    let defenders: Vec<_> = occupied
        .legal_actions()
        .expect("Defend actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "defend")
        .map(|action| {
            action.descriptor["unitInstanceId"]
                .as_str()
                .expect("defender identity")
                .to_owned()
        })
        .collect();
    assert!(!defenders.contains(&setup.south_ground_id));
    assert!(defenders.contains(&setup.south_airborne_id));
    accept_where(&mut occupied, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == setup.south_airborne_id
            && descriptor["to"]["cell"] == "C2"
    });
    assert_checkpoint_round_trip(&occupied);
    assert_exact_replay(&occupied);
}

#[test]
fn oversized_mountain_pass_manifest_should_fail_closed() {
    let mut oversized: Value = serde_json::from_str(&manifest()).expect("synthetic manifest value");
    oversized["cards"]["north-ground"]["occupiesSquareArea"] = json!(2);
    oversized
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    oversized["manifestId"] =
        json!(identity_hash(&oversized).expect("oversized manifest identity"));
    let encoded = canonical_json(&oversized).expect("canonical oversized manifest");
    let error = Session::new(&encoded).expect_err("oversized units remain unsupported");
    match error {
        SessionError::Game(GameError::UnsupportedManifestFact(field)) => {
            assert_eq!(field, "occupiesSquareArea");
        }
        other => panic!("expected unsupported oversized fact, received {other}"),
    }
}

#[test]
fn mountain_pass_should_block_entry_with_a_friendly_occupant() {
    let setup = prepare_mountain_pass();
    let mut session = setup.session;
    begin_north_turn(&mut session);
    let occupant_id = summon(&mut session, &setup.north_occupant_card_id, "C2");
    let current = state(&session);
    let occupant = current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == occupant_id)
        .expect("friendly occupant");
    assert_eq!(occupant["controller"], "north");
    assert!(!entry_actor_ids(&session).contains(&setup.north_ground_id));
    assert_exact_replay(&session);
}

#[test]
fn ground_minion_should_be_allowed_to_leave_occupied_mountain_pass() {
    let setup = prepare_mountain_pass();
    let mut session = setup.session;
    let leaver_id = summon(&mut session, &setup.south_leaver_card_id, "C2");
    begin_north_turn(&mut session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let paths: Vec<_> = session
        .legal_actions()
        .expect("leaving actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == leaver_id
        })
        .map(|action| action.descriptor["path"].clone())
        .collect();
    assert!(paths.iter().any(|path| {
        path.as_array().is_some_and(|locations| {
            locations
                .iter()
                .map(|location| location["cell"].as_str().expect("path cell"))
                .eq(["C2", "C3"])
        })
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == leaver_id
            && descriptor["to"]["cell"] == "C3"
    });
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == leaver_id)
            .expect("leaver")["location"],
        "C3"
    );
    assert_exact_replay(&session);
}
