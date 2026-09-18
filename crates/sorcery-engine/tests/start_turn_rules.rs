//! Direct proofs for start-turn triggers: random teleports (RULE-CATALOG-0158),
//! controller Spellbook draws (RULE-CATALOG-0237–0238), controller Atlas
//! draws (RULE-CATALOG-0241–0242), stacked library triggers
//! (RULE-CATALOG-0387–0388, RULE-CATALOG-0391–0392, RULE-CATALOG-0393–0394,
//! RULE-CATALOG-0397–0398), library-plus-teleport stacks
//! (RULE-CATALOG-0401–0402), thin-library draw-then-mill edges
//! (RULE-CATALOG-0916), and direct draw-sites-then-teleport ordering
//! (RULE-CATALOG-0936), and direct draw-sites-then-mill ordering
//! (RULE-CATALOG-0948), and thin-Atlas draw-then-mill edges
//! (RULE-CATALOG-0967), and thin-library draw-then-teleport edges
//! (RULE-CATALOG-0980), and thin-Atlas draw-then-teleport edges
//! (RULE-CATALOG-0983), and thin-library draw-sites-then-draw-spells edges
//! (RULE-CATALOG-0995), resolve-start-turn-trigger withheld during
//! deathrite-order (RULE-CATALOG-1163), and stacked start-turn triggers
//! withheld during deathrite-order (RULE-CATALOG-1183–1207).

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode, Seat};
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

fn site(extra: Value) -> Value {
    let mut value = json!({ "cardType": "site", "elements": ["earth"] });
    let Value::Object(extra) = extra else {
        panic!("extra site facts must be an object");
    };
    value.as_object_mut().expect("site facts").extend(extra);
    value
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
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

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-headless-start-turn-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charm": json!({
                "bearerControllerChoosesExtraRandomOutcome": true,
                "cardType": "artifact",
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "north-open-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnTeleportToRandomSiteOrVoid": true,
                "attack": 3,
                "voidwalk": true,
            })),
            "south-avatar": avatar(),
            "south-blocked-site": site(json!({ "preventsUnitsWithPowerAtLeastFromEntering": 3 })),
            "south-blocker": minion(json!({ "attack": 2, "defense": 5 })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-open-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-source",
                    "north-source",
                    "north-source",
                    "north-source",
                    "north-charm",
                    "north-charm",
                ],
            },
            "south": {
                "atlas": vec!["south-blocked-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-blocker"; 6],
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

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action in phase {} among {:?}",
                state(session)["phase"],
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            );
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

fn checkpoint(seed: u32) -> Session {
    let mut session = Session::new(&manifest(seed)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-charm"
            && descriptor["bearer"]["kind"] == "avatar"
    });
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-source"
                && descriptor["cell"] == "C4"
        });
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-blocked-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-blocker"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn source_ids(session: &Session) -> Vec<String> {
    let mut ids = state(session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .filter(|unit| unit["cardId"] == "north-source")
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("source identity")
                .to_owned()
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one replayed scenario proves controller ordering, blocked teleport, and forged rejection"
)]
fn rule_catalog_0158_start_turn_random_teleports_resolve_through_lucky_charm() {
    let checkpoint = checkpoint(10);
    let source_ids = source_ids(&checkpoint);
    assert_eq!(source_ids.len(), 2);
    assert_eq!(state(&checkpoint)["phase"], "start-turn");

    let triggers: Vec<_> = checkpoint
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    let trigger_ids: Vec<_> = triggers
        .iter()
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("source id")
                .to_owned()
        })
        .collect();
    let mut sorted_trigger_ids = trigger_ids.clone();
    sorted_trigger_ids.sort_unstable();
    assert_eq!(sorted_trigger_ids, source_ids);

    let first_trigger = triggers
        .iter()
        .find(|action| action.descriptor["sourceInstanceId"] == source_ids[1])
        .expect("second source trigger")
        .clone();

    let mut committed = checkpoint.clone();
    let StepResult::Accepted(committed_receipt) = committed
        .step(ActionRequest {
            action_id: first_trigger.action_id.to_string(),
            seat: first_trigger.seat,
            state_version: first_trigger.state_version,
        })
        .expect("commit first trigger")
    else {
        panic!("first trigger must commit");
    };
    assert!(committed_receipt.events.is_empty());
    assert_eq!(committed_receipt.random_draws.len(), 2);
    assert!(
        committed_receipt
            .random_draws
            .iter()
            .all(|draw| draw["purpose"] == "start_turn_random_teleport")
    );
    assert_eq!(state(&committed)["phase"], "random-choice");

    let choices: Vec<_> = committed
        .legal_actions()
        .expect("lucky charm choices")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-random-outcome")
        .collect();
    let blocked_choice = choices
        .iter()
        .find(|action| action.label == "Lucky Charm chooses C1 surface")
        .expect("blocked C1 surface choice")
        .clone();

    let mut blocked = committed.clone();
    let StepResult::Accepted(blocked_receipt) = blocked
        .step(ActionRequest {
            action_id: blocked_choice.action_id.to_string(),
            seat: blocked_choice.seat,
            state_version: blocked_choice.state_version,
        })
        .expect("blocked choice")
    else {
        panic!("blocked choice must be accepted");
    };
    assert_eq!(blocked_receipt.events.len(), 1);
    assert_eq!(blocked_receipt.events[0].event_type, "unit-teleport-failed");
    assert_eq!(
        blocked_receipt.events[0].payload["sourceInstanceId"],
        source_ids[1]
    );
    assert_eq!(
        blocked_receipt.events[0].payload["to"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert!(blocked_receipt.random_draws.is_empty());
    assert_eq!(state(&blocked)["phase"], "start-turn");
    assert_eq!(
        state(&blocked)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_ids[1])
            .expect("held source")["location"],
        "C4"
    );

    let second_trigger = blocked
        .legal_actions()
        .expect("remaining trigger")
        .into_iter()
        .find(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .expect("second trigger")
        .clone();
    assert_eq!(second_trigger.descriptor["sourceInstanceId"], source_ids[0]);

    let forged = blocked
        .step(ActionRequest {
            action_id: identity_hash(&json!({
                "descriptor": {
                    "kind": "resolve-start-turn-trigger",
                    "sourceInstanceId": source_ids[1],
                },
                "engineVersion": "sorcery-core-v1",
                "seat": "north",
                "stateVersion": state(&blocked)["stateVersion"],
            }))
            .expect("forged action id")
            .to_string(),
            seat: Seat::North,
            state_version: state(&blocked)["stateVersion"]
                .as_u64()
                .expect("state version"),
        })
        .expect("forged step");
    assert!(matches!(
        forged,
        StepResult::Rejected(rejection) if rejection.code == RejectionCode::UnknownAction
    ));

    let StepResult::Accepted(second_receipt) = blocked
        .step(ActionRequest {
            action_id: second_trigger.action_id.to_string(),
            seat: second_trigger.seat,
            state_version: second_trigger.state_version,
        })
        .expect("second trigger")
    else {
        panic!("second trigger must commit");
    };
    assert!(second_receipt.events.is_empty());
    assert_eq!(second_receipt.random_draws.len(), 2);

    let moved_choice = blocked
        .legal_actions()
        .expect("move choices")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-random-outcome")
        .find(|action| {
            let mut trial = blocked.clone();
            matches!(
                trial.step(ActionRequest {
                    action_id: action.action_id.to_string(),
                    seat: action.seat,
                    state_version: action.state_version,
                }),
                Ok(StepResult::Accepted(receipt))
                    if receipt.events.iter().any(|event| event.event_type == "unit-teleported")
            )
        })
        .expect("teleporting lucky charm choice");

    assert!(moved_choice.label.starts_with("Lucky Charm chooses "));
    accept_where(&mut blocked, |descriptor| {
        descriptor["kind"] == "resolve-random-outcome"
            && descriptor["outcomeInstanceId"] == moved_choice.descriptor["outcomeInstanceId"]
    });
    assert!(
        state(&blocked)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| {
                unit["instanceId"] == source_ids[0] && unit["location"].as_str() != Some("C4")
            })
    );
    assert!(blocked.verify_replay().expect("verified replay"));
}

const NORTH_VOID_SQUARE: [&str; 4] = ["A1", "A2", "B1", "B2"];
const SOUTH_BLOCKED_SQUARE: [&str; 4] = ["C1", "C2", "D1", "D2"];

fn oversized_source() -> Value {
    minion(json!({
        "atStartOfControllerTurnTeleportToRandomSiteOrVoid": true,
        "attack": 3,
        "occupiesSquareArea": 2,
        "voidwalk": true,
    }))
}

fn oversized_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "oversized-start-turn-teleport" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-oversized-start-turn-teleport-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charm": json!({
                "bearerControllerChoosesExtraRandomOutcome": true,
                "cardType": "artifact",
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "north-open-site": site(json!({})),
            "north-source": oversized_source(),
            "south-avatar": avatar(),
            "south-blocked-site": site(json!({ "preventsUnitsWithPowerAtLeastFromEntering": 3 })),
            "south-blocker": minion(json!({ "attack": 2, "defense": 5 })),
            "south-open-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec![
                    "north-open-site"; 12
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-charm",
                    "north-source",
                    "north-source",
                    "north-charm",
                    "north-source",
                    "north-source",
                    "north-charm",
                    "north-source",
                    "north-source",
                    "north-charm",
                    "north-source",
                    "north-source",
                ],
            },
            "south": {
                "atlas": vec![
                    "south-blocked-site",
                    "south-open-site",
                    "south-open-site",
                    "south-open-site",
                    "south-blocked-site",
                    "south-open-site",
                    "south-open-site",
                    "south-open-site",
                    "south-blocked-site",
                    "south-open-site",
                    "south-open-site",
                    "south-open-site",
                ],
                "avatar": "south-avatar",
                "spellbook": vec!["south-blocker"; 12],
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

fn end_and_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn establish_board_for_oversized_teleport(session: &mut Session) {
    accept_where_labeled(session, "north play C4", |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-open-site"
            && descriptor["cell"] == "C4"
    });
    end_and_draw(session, "spellbook");
    accept_where_labeled(session, "south play blocked C1", |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-blocked-site"
            && descriptor["cell"] == "C1"
    });
    end_and_draw(session, "spellbook");
    accept_where_labeled(session, "north play B4", |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-open-site"
            && descriptor["cell"] == "B4"
    });
    end_and_draw(session, "spellbook");
    accept_where_labeled(session, "south play C2", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    end_and_draw(session, "spellbook");
    accept_where_labeled(session, "north play C3", |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-open-site"
            && descriptor["cell"] == "C3"
    });
    end_and_draw(session, "spellbook");
    accept_where_labeled(session, "south play D1", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D1"
    });
    end_and_draw(session, "spellbook");
}

fn accept_where_labeled(
    session: &mut Session,
    label: &str,
    predicate: impl Fn(&Value) -> bool,
) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "{label}: phase={} among {:?}",
                state(session)["phase"],
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            );
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
        panic!("{label} must be accepted");
    };
    (descriptor, receipt)
}

fn oversized_checkpoint(seed: u32) -> Session {
    let mut session = Session::new(&oversized_manifest(seed)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    establish_board_for_oversized_teleport(&mut session);
    if session
        .legal_actions()
        .expect("actions")
        .iter()
        .any(|action| action.descriptor["kind"] == "draw-site")
    {
        accept_where_labeled(&mut session, "draw-site", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    accept_where_labeled(&mut session, "cast charm", |descriptor| {
        descriptor["kind"] == "cast-artifact" && descriptor["cardId"] == "north-charm"
    });
    accept_where_labeled(&mut session, "summon giant", |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "A1"
    });
    accept_where_labeled(&mut session, "north end-turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where_labeled(&mut session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where_labeled(&mut session, "south play D2", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D2"
    });
    accept_where_labeled(&mut session, "south summon blocker", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-blocker"
            && descriptor["cell"] == "C1"
    });
    accept_where_labeled(&mut session, "south end-turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    session
}

fn oversized_source_id(session: &Session) -> String {
    state(session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("oversized source")["instanceId"]
        .as_str()
        .expect("source identity")
        .to_owned()
}

#[test]
fn rule_catalog_0379_oversized_start_turn_random_teleport_moves_whole_footprint() {
    let checkpoint = oversized_checkpoint(10);
    let source_id = oversized_source_id(&checkpoint);
    let before = state(&checkpoint)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == source_id)
        .expect("source before teleport")
        .clone();
    assert_eq!(before["location"], "A1");
    assert_eq!(before["region"], "void");
    assert_eq!(before["occupiedCells"], json!(NORTH_VOID_SQUARE));
    assert_eq!(state(&checkpoint)["phase"], "start-turn");

    let trigger = checkpoint
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("oversized start-turn trigger");

    let mut committed = checkpoint.clone();
    let StepResult::Accepted(committed_receipt) = committed
        .step(ActionRequest {
            action_id: trigger.action_id.to_string(),
            seat: trigger.seat,
            state_version: trigger.state_version,
        })
        .expect("commit oversized trigger")
    else {
        panic!("oversized trigger must commit");
    };
    assert!(committed_receipt.events.is_empty());
    assert_eq!(committed_receipt.random_draws.len(), 2);
    assert_eq!(state(&committed)["phase"], "random-choice");

    let moved_choice = committed
        .legal_actions()
        .expect("lucky charm choices")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-random-outcome")
        .find(|action| {
            let mut trial = committed.clone();
            matches!(
                trial.step(ActionRequest {
                    action_id: action.action_id.to_string(),
                    seat: action.seat,
                    state_version: action.state_version,
                }),
                Ok(StepResult::Accepted(receipt))
                    if receipt.events.iter().any(|event| event.event_type == "unit-teleported")
            )
        })
        .expect("teleporting lucky charm choice");

    accept_where(&mut committed, |descriptor| {
        descriptor["kind"] == "resolve-random-outcome"
            && descriptor["outcomeInstanceId"] == moved_choice.descriptor["outcomeInstanceId"]
    });
    let after = state(&committed)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == source_id)
        .expect("source after teleport")
        .clone();
    assert_ne!(after["location"], before["location"]);
    assert_eq!(
        after["occupiedCells"]
            .as_array()
            .expect("occupied cells")
            .len(),
        4
    );
    assert!(committed.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0380_oversized_start_turn_random_teleport_fails_blocked_footprint_through_lucky_charm()
 {
    let checkpoint = oversized_checkpoint(10);
    let source_id = oversized_source_id(&checkpoint);
    assert_eq!(state(&checkpoint)["phase"], "start-turn");

    let trigger = checkpoint
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("oversized start-turn trigger")
        .clone();

    let mut committed = checkpoint.clone();
    let StepResult::Accepted(committed_receipt) = committed
        .step(ActionRequest {
            action_id: trigger.action_id.to_string(),
            seat: trigger.seat,
            state_version: trigger.state_version,
        })
        .expect("commit oversized trigger")
    else {
        panic!("oversized trigger must commit");
    };
    assert!(committed_receipt.events.is_empty());
    assert_eq!(state(&committed)["phase"], "random-choice");

    let blocked_choice = committed
        .legal_actions()
        .expect("lucky charm choices")
        .into_iter()
        .find(|action| {
            action.label
                == format!(
                    "Lucky Charm chooses C1 surface ({})",
                    SOUTH_BLOCKED_SQUARE.join(", ")
                )
        })
        .expect("blocked C1 surface footprint choice")
        .clone();

    let StepResult::Accepted(blocked_receipt) = committed
        .step(ActionRequest {
            action_id: blocked_choice.action_id.to_string(),
            seat: blocked_choice.seat,
            state_version: blocked_choice.state_version,
        })
        .expect("blocked footprint choice")
    else {
        panic!("blocked footprint choice must be accepted");
    };
    assert_eq!(blocked_receipt.events.len(), 1);
    assert_eq!(blocked_receipt.events[0].event_type, "unit-teleport-failed");
    assert_eq!(
        blocked_receipt.events[0].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        blocked_receipt.events[0].payload["cells"],
        json!(SOUTH_BLOCKED_SQUARE)
    );
    assert_eq!(
        state(&committed)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_id)
            .expect("held source")["location"],
        "A1"
    );
    assert_eq!(
        state(&committed)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_id)
            .expect("held source")["occupiedCells"],
        json!(NORTH_VOID_SQUARE)
    );
    assert!(committed.verify_replay().expect("verified replay"));
}

fn draw_spells_manifest(seed: u32, north_spellbook: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-spells" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-spells-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_spells_start_turn(seed: u32, north_spellbook: &[&str]) -> Session {
    let mut session =
        Session::new(&draw_spells_manifest(seed, north_spellbook)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
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

#[test]
fn rule_catalog_0237_start_turn_draw_spells_draws_a_hidden_spell_before_the_draw_step() {
    let mut session = draw_spells_start_turn(237, &["north-source"; 6]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let drawn_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("next spell")["instanceId"]
        .clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let library_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    let south_observation = session.observe(Seat::South);
    let south_before = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_before["players"]["north"]["hand"]["spellbook"],
        hand_before
    );
    let checkpoint = session.clone();
    let triggers: Vec<_> = session
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].descriptor["sourceInstanceId"], source_id);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["spell-drawn"]);
    assert_eq!(receipt.events[0].payload["seat"], "north");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert!(receipt.events[0].payload.get("instanceId").is_none());
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    let hand = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand");
    assert_eq!(hand.len(), hand_before + 1);
    assert!(hand.iter().any(|card| card["instanceId"] == drawn_id));
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        library_before - 1
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_after = session
        .public_view(Seat::South)
        .expect("South public view after the draw");
    assert_eq!(
        south_after["players"]["north"]["hand"]["spellbook"],
        hand_before + 1
    );
    assert_eq!(
        south_after["players"]["north"]["spellbookCount"],
        library_before - 1
    );
    let south_json = south_after.to_string();
    assert!(
        !south_json.contains(drawn_id.as_str().expect("drawn identity")),
        "the opponent must not see the drawn identity"
    );
    let legal = session.legal_actions().expect("draw-step actions");
    assert!(
        legal
            .iter()
            .any(|action| action.descriptor["kind"] == "draw")
    );
    assert!(
        legal
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger")
    );
    assert_eq!(state(&checkpoint)["phase"], "start-turn");
    let mut resumed = checkpoint;
    accept_where(&mut resumed, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(
        resumed.replay_value().expect("resumed value"),
        session.replay_value().expect("session value")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0238_start_turn_draw_spells_decks_out_on_an_empty_library() {
    let mut session = draw_spells_start_turn(238, &["north-source"; 3]);
    assert_eq!(state(&session)["phase"], "start-turn");
    assert_eq!(state(&session)["players"]["north"]["spellbook"], json!([]));
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["game-ended"]);
    assert_eq!(receipt.events[0].payload["reason"], "deck_empty");
    assert_eq!(receipt.events[0].payload["loser"], "north");
    let after = state(&session);
    assert_eq!(after["phase"], "terminal");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_eq!(after["terminal"]["loser"], "north");
    assert_exact_replay(&session);
}

fn draw_sites_manifest(seed: u32, north_atlas: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": north_atlas,
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_sites_start_turn(seed: u32, north_atlas: &[&str]) -> Session {
    let mut session = Session::new(&draw_sites_manifest(seed, north_atlas)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

#[test]
fn rule_catalog_0241_start_turn_draw_sites_draws_a_hidden_site_before_the_draw_step() {
    let mut session = draw_sites_start_turn(241, &["north-site"; 6]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let drawn_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .first()
        .expect("next site")["instanceId"]
        .clone();
    let hand_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand")
        .len();
    let library_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let south_observation = session.observe(Seat::South);
    let south_before = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_before["players"]["north"]["hand"]["atlas"],
        hand_before
    );
    let checkpoint = session.clone();
    let triggers: Vec<_> = session
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].descriptor["sourceInstanceId"], source_id);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["site-drawn"]);
    assert_eq!(receipt.events[0].payload["seat"], "north");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert!(receipt.events[0].payload.get("instanceId").is_none());
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    let hand = after["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand");
    assert_eq!(hand.len(), hand_before + 1);
    assert!(hand.iter().any(|card| card["instanceId"] == drawn_id));
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        library_before - 1
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_after = session
        .public_view(Seat::South)
        .expect("South public view after the draw");
    assert_eq!(
        south_after["players"]["north"]["hand"]["atlas"],
        hand_before + 1
    );
    assert_eq!(
        south_after["players"]["north"]["atlasCount"],
        library_before - 1
    );
    let south_json = south_after.to_string();
    assert!(
        !south_json.contains(drawn_id.as_str().expect("drawn identity")),
        "the opponent must not see the drawn identity"
    );
    let legal = session.legal_actions().expect("draw-step actions");
    assert!(
        legal
            .iter()
            .any(|action| action.descriptor["kind"] == "draw")
    );
    assert!(
        legal
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger")
    );
    assert_eq!(state(&checkpoint)["phase"], "start-turn");
    let mut resumed = checkpoint;
    accept_where(&mut resumed, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(
        resumed.replay_value().expect("resumed value"),
        session.replay_value().expect("session value")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0242_start_turn_draw_sites_decks_out_on_an_empty_library() {
    let mut session = draw_sites_start_turn(242, &["north-site"; 3]);
    assert_eq!(state(&session)["phase"], "start-turn");
    assert_eq!(state(&session)["players"]["north"]["atlas"], json!([]));
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["game-ended"]);
    assert_eq!(receipt.events[0].payload["reason"], "deck_empty");
    assert_eq!(receipt.events[0].payload["loser"], "north");
    let after = state(&session);
    assert_eq!(after["phase"], "terminal");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_eq!(after["terminal"]["loser"], "north");
    assert_exact_replay(&session);
}

fn draw_mill_stack_manifest(seed: u32, north_spellbook: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-mill-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-mill-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-mill-card": minion(json!({})),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSpells": 1,
                "atStartOfControllerTurnMillSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_mill_stack_start_turn(seed: u32, north_spellbook: &[&str]) -> Session {
    let mut session =
        Session::new(&draw_mill_stack_manifest(seed, north_spellbook)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

fn draw_mill_empty_after_draw_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-mill-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-mill-empty-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSpells": 3,
                "atStartOfControllerTurnMillSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-draw-card",
                    "north-draw-card",
                    "north-draw-card",
                    "north-draw-card",
                    "north-draw-card",
                    "north-source",
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
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_mill_empty_after_draw_start_turn(seed: u32) -> Session {
    let mut session =
        Session::new(&draw_mill_empty_after_draw_manifest(seed)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

#[test]
fn rule_catalog_0387_start_turn_draw_then_mill_spells_resolves_in_order() {
    let mut session = draw_mill_stack_start_turn(
        387,
        &[
            "north-draw-card",
            "north-mill-card",
            "north-source",
            "north-source",
            "north-source",
            "north-source",
        ],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let drawn_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("next spell")["instanceId"]
        .clone();
    let milled_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .get(1)
        .expect("second spell")["instanceId"]
        .clone();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let library_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    let cemetery_before = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let triggers: Vec<_> = session
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].descriptor["sourceInstanceId"], source_id);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["spell-drawn", "spell-discarded"]);
    assert_eq!(receipt.events[1].payload["instanceId"], milled_id);
    assert_ne!(drawn_id, milled_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        library_before - 2
    );
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        cemetery_before + 1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0388_start_turn_mill_spells_is_a_no_op_after_draw_empties_the_library() {
    let mut session = draw_mill_empty_after_draw_start_turn(388);
    assert_eq!(state(&session)["phase"], "start-turn");
    let library_before = state(&session)["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    assert_eq!(library_before, 3);
    let source_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let cemetery_before = state(&session)["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(
        event_types(&receipt),
        ["spell-drawn", "spell-drawn", "spell-drawn"]
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + library_before
    );
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        cemetery_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0916_start_turn_draw_then_mill_spells_mills_no_op_when_library_has_one_card() {
    let mut session = draw_mill_stack_start_turn(
        916,
        &[
            "north-draw-card",
            "north-mill-card",
            "north-draw-card",
            "north-source",
        ],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let drawn_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("only spell")["instanceId"]
        .clone();
    let library_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    assert_eq!(library_before, 1);
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let cemetery_before = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["spell-drawn"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_id)
    );
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        cemetery_before
    );
    assert_exact_replay(&session);
}

fn draw_sites_draw_spells_stack_manifest(
    seed: u32,
    north_atlas: &[&str],
    north_spellbook: &[&str],
) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-spells-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-spells-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-spell-card": minion(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": north_atlas,
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_sites_draw_spells_stack_start_turn(
    seed: u32,
    north_atlas: &[&str],
    north_spellbook: &[&str],
) -> Session {
    let mut session = Session::new(&draw_sites_draw_spells_stack_manifest(
        seed,
        north_atlas,
        north_spellbook,
    ))
    .expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one replayed scenario proves cross-zone drawSites then drawSpells ordering"
)]
fn rule_catalog_0391_start_turn_draw_sites_then_draw_spells_resolves_in_order() {
    let mut session = draw_sites_draw_spells_stack_start_turn(
        391,
        &["north-site"; 6],
        &[
            "north-spell-card",
            "north-source",
            "north-source",
            "north-source",
            "north-source",
            "north-source",
        ],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let drawn_site_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .first()
        .expect("next site")["instanceId"]
        .clone();
    let drawn_spell_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("next spell")["instanceId"]
        .clone();
    let atlas_hand_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand")
        .len();
    let spell_hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let atlas_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let spellbook_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    let triggers: Vec<_> = session
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].descriptor["sourceInstanceId"], source_id);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["site-drawn", "spell-drawn"]);
    assert_eq!(receipt.events[0].payload["seat"], "north");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert!(receipt.events[0].payload.get("instanceId").is_none());
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], source_id);
    assert!(receipt.events[1].payload.get("instanceId").is_none());
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_site_id)
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_spell_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .len(),
        atlas_hand_before + 1
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        spell_hand_before + 1
    );
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        atlas_before - 1
    );
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        spellbook_before - 1
    );
    assert_exact_replay(&session);
}

fn draw_sites_mill_sites_stack_manifest(seed: u32, north_atlas: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-mill-sites-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-mill-sites-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnMillSites": 1,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": north_atlas,
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_sites_mill_sites_stack_start_turn(seed: u32, north_atlas: &[&str]) -> Session {
    let mut session = Session::new(&draw_sites_mill_sites_stack_manifest(seed, north_atlas))
        .expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

#[test]
fn rule_catalog_0392_start_turn_draw_sites_then_mill_sites_resolves_in_order() {
    let mut session = draw_sites_mill_sites_stack_start_turn(392, &["north-site"; 6]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let drawn_site_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .first()
        .expect("next site")["instanceId"]
        .clone();
    let milled_site_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .get(1)
        .expect("second site")["instanceId"]
        .clone();
    let milled_card_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .get(1)
        .expect("second site")["cardId"]
        .clone();
    let atlas_hand_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand")
        .len();
    let atlas_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let cemetery_before = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let triggers: Vec<_> = session
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].descriptor["sourceInstanceId"], source_id);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["site-drawn", "site-discarded"]);
    assert_eq!(receipt.events[1].payload["instanceId"], milled_site_id);
    assert_eq!(receipt.events[1].payload["cardId"], milled_card_id);
    assert_ne!(drawn_site_id, milled_site_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_site_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .len(),
        atlas_hand_before + 1
    );
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        atlas_before - 2
    );
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        cemetery_before + 1
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == milled_site_id)
    );
    assert_exact_replay(&session);
}

fn draw_spells_life_gain_stack_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-life-gain-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-life-gain-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnControllerGainsLife": 2,
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-drain": json!({
                "cardType": "magic",
                "manaCost": 0,
                "targetPlayerLosesLife": 2,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-drain"; 6],
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

fn draw_spells_life_loss_stack_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-life-loss-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-life-loss-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnControllerLosesLife": 1,
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_spells_exclusive_stack_start_turn(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

fn draw_spells_life_gain_stack_start_turn(seed: u32) -> Session {
    let mut session =
        Session::new(&draw_spells_life_gain_stack_manifest(seed)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

#[test]
fn rule_catalog_0393_start_turn_draw_spells_then_life_gain_resolves_in_order() {
    let mut session = draw_spells_life_gain_stack_start_turn(393);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["spell-drawn", "avatar-healed"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert_eq!(receipt.events[1].payload["amount"], 2);
    assert_eq!(receipt.events[1].payload["life"], 20);
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0394_start_turn_draw_spells_then_life_loss_resolves_in_order() {
    let mut session =
        draw_spells_exclusive_stack_start_turn(&draw_spells_life_loss_stack_manifest(394));
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    assert_eq!(before["players"]["north"]["avatar"]["life"], 20);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["spell-drawn", "avatar-life-lost"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(receipt.events[1].payload["life"], 19);
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["players"]["north"]["avatar"]["life"], 19);
    assert_exact_replay(&session);
}

fn draw_spells_mana_gain_stack_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-mana-gain-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-mana-gain-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnControllerGainsMana": 1,
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_sites_here_damage_stack_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-here-damage-stack" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-here-damage-stack-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
                "atStartOfControllerTurnDrawSites": 1,
            })),
            "south-avatar": avatar(),
            "south-site": site(json!({})),
            "south-visitor": minion(json!({
                "summonToAnySite": true,
            })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-visitor"; 6],
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

fn draw_sites_here_damage_stack_start_turn(seed: u32) -> Session {
    let mut session =
        Session::new(&draw_sites_here_damage_stack_manifest(seed)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

#[test]
fn rule_catalog_0397_start_turn_draw_spells_then_mana_gain_resolves_in_order() {
    let mut session =
        draw_spells_exclusive_stack_start_turn(&draw_spells_mana_gain_stack_manifest(397));
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    assert_eq!(before["players"]["north"]["mana"], 1);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["spell-drawn", "mana-gained"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["players"]["north"]["mana"], 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0398_start_turn_draw_sites_then_here_damage_resolves_in_order() {
    let mut session = draw_sites_here_damage_stack_start_turn(398);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let visitor_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-visitor")
        .expect("visitor minion")["instanceId"]
        .clone();
    let north_avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"].clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    let draw_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "site-drawn")
        .expect("site-drawn event");
    let damage_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "start-turn-damage-allocated")
        .expect("start-turn-damage-allocated event");
    assert!(
        draw_index < damage_index,
        "Atlas draw must resolve before here-area damage on the same minion"
    );
    assert_eq!(
        receipt.events[draw_index].payload["sourceInstanceId"],
        source_id
    );
    let mut here_targets: Vec<Value> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "start-turn-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 1);
            assert_eq!(event.payload["sourceInstanceId"], source_id);
            event.payload["targetInstanceId"].clone()
        })
        .collect();
    here_targets.sort_by_key(Value::to_string);
    let mut expected_here_targets = vec![north_avatar_id, visitor_id.clone()];
    expected_here_targets.sort_by_key(Value::to_string);
    assert_eq!(here_targets, expected_here_targets);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["players"]["north"]["avatar"]["life"], 19);
    assert_eq!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == visitor_id)
            .expect("visitor")["damage"],
        1
    );
    assert_exact_replay(&session);
}

fn draw_spells_teleport_manifest(seed: u32, north_spellbook: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-spells-teleport" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-spells-teleport-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSpells": 1,
                "atStartOfControllerTurnTeleportToRandomSiteOrVoid": true,
                "attack": 3,
                "voidwalk": true,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_spells_teleport_start_turn(seed: u32, north_spellbook: &[&str]) -> Session {
    let mut session =
        Session::new(&draw_spells_teleport_manifest(seed, north_spellbook)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

#[test]
fn rule_catalog_0401_start_turn_draw_spells_then_teleport_resolves_in_order() {
    let mut session = draw_spells_teleport_start_turn(401, &["north-source"; 6]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let location_before = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == source_id)
        .expect("source before teleport")["location"]
        .clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    let draw_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "spell-drawn")
        .expect("spell-drawn event");
    let teleport_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "unit-teleported")
        .expect("unit-teleported event");
    assert!(
        draw_index < teleport_index,
        "Spellbook draw must resolve before random teleport on the same minion"
    );
    assert_eq!(
        receipt.events[draw_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        receipt.events[teleport_index].payload["sourceInstanceId"],
        source_id
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_ne!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_id)
            .expect("source after teleport")["location"],
        location_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0980_start_turn_draw_spells_then_teleport_draws_last_spell_before_teleport_on_thin_library()
 {
    let mut session = draw_spells_teleport_start_turn(980, &["north-source"; 4]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let location_before = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == source_id)
        .expect("source before teleport")["location"]
        .clone();
    let drawn_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("only spell")["instanceId"]
        .clone();
    let library_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    assert_eq!(library_before, 1);
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    let draw_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "spell-drawn")
        .expect("spell-drawn event");
    let teleport_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "unit-teleported")
        .expect("unit-teleported event");
    assert!(
        draw_index < teleport_index,
        "Spellbook draw must resolve before random teleport on the same minion Start Phase trigger"
    );
    assert_eq!(
        receipt.events[draw_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        receipt.events[teleport_index].payload["sourceInstanceId"],
        source_id
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_id)
    );
    assert_ne!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_id)
            .expect("source after teleport")["location"],
        location_before
    );
    assert_exact_replay(&session);
}

fn draw_sites_teleport_manifest(seed: u32, north_atlas: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-teleport" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-teleport-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charm": json!({
                "bearerControllerChoosesExtraRandomOutcome": true,
                "cardType": "artifact",
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnTeleportToRandomSiteOrVoid": true,
                "attack": 3,
                "voidwalk": true,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": north_atlas,
                "avatar": "north-avatar",
                "spellbook": [
                    "north-charm",
                    "north-charm",
                    "north-source",
                    "north-source",
                    "north-charm",
                    "north-charm",
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
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_sites_teleport_start_turn(seed: u32, north_atlas: &[&str]) -> Session {
    let mut session =
        Session::new(&draw_sites_teleport_manifest(seed, north_atlas)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-charm"
            && descriptor["bearer"]["kind"] == "avatar"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one replayed scenario proves deferred Atlas draw then Lucky Charm teleport"
)]
fn rule_catalog_0402_start_turn_draw_sites_then_teleport_resolves_through_lucky_charm() {
    let checkpoint = draw_sites_teleport_start_turn(402, &["north-site"; 6]);
    let source_id = state(&checkpoint)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let location_before = state(&checkpoint)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == source_id)
        .expect("source before teleport")["location"]
        .clone();
    assert_eq!(state(&checkpoint)["phase"], "start-turn");

    let trigger = checkpoint
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("draw-sites teleport trigger")
        .clone();

    let mut committed = checkpoint.clone();
    let StepResult::Accepted(committed_receipt) = committed
        .step(ActionRequest {
            action_id: trigger.action_id.to_string(),
            seat: trigger.seat,
            state_version: trigger.state_version,
        })
        .expect("commit start-turn trigger")
    else {
        panic!("start-turn trigger must commit");
    };
    assert!(committed_receipt.events.is_empty());
    assert_eq!(committed_receipt.random_draws.len(), 2);
    assert!(
        committed_receipt
            .random_draws
            .iter()
            .all(|draw| draw["purpose"] == "start_turn_random_teleport")
    );
    assert_eq!(state(&committed)["phase"], "random-choice");

    let moved_choice = committed
        .legal_actions()
        .expect("lucky charm choices")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-random-outcome")
        .find(|action| {
            let mut trial = committed.clone();
            matches!(
                trial.step(ActionRequest {
                    action_id: action.action_id.to_string(),
                    seat: action.seat,
                    state_version: action.state_version,
                }),
                Ok(StepResult::Accepted(receipt))
                    if receipt.events.iter().any(|event| event.event_type == "unit-teleported")
            )
        })
        .expect("teleporting lucky charm choice");

    let StepResult::Accepted(resolved_receipt) = committed
        .step(ActionRequest {
            action_id: moved_choice.action_id.to_string(),
            seat: moved_choice.seat,
            state_version: moved_choice.state_version,
        })
        .expect("resolve lucky charm choice")
    else {
        panic!("lucky charm choice must commit");
    };
    let draw_index = resolved_receipt
        .events
        .iter()
        .position(|event| event.event_type == "site-drawn")
        .expect("site-drawn event");
    let teleport_index = resolved_receipt
        .events
        .iter()
        .position(|event| event.event_type == "unit-teleported")
        .expect("unit-teleported event");
    assert!(
        draw_index < teleport_index,
        "Atlas draw must resolve before random teleport on the deferred start-turn trigger"
    );
    assert_eq!(
        resolved_receipt.events[draw_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        resolved_receipt.events[teleport_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(state(&committed)["phase"], "draw");
    assert_ne!(
        state(&committed)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_id)
            .expect("source after teleport")["location"],
        location_before
    );
    assert!(committed.verify_replay().expect("verified replay"));
}

fn draw_sites_teleport_direct_manifest(seed: u32, north_atlas: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-teleport-direct" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-teleport-direct-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(json!({})),
            "north-source": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnTeleportToRandomSiteOrVoid": true,
                "attack": 3,
                "voidwalk": true,
            })),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": north_atlas,
                "avatar": "north-avatar",
                "spellbook": vec!["north-source"; 6],
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
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn draw_sites_teleport_direct_start_turn(seed: u32, north_atlas: &[&str]) -> Session {
    let mut session = Session::new(&draw_sites_teleport_direct_manifest(seed, north_atlas))
        .expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-source"
            && descriptor["cell"] == "C4"
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
    session
}

#[test]
fn rule_catalog_0936_start_turn_draw_sites_then_teleport_resolves_in_order_without_lucky_charm() {
    let mut session = draw_sites_teleport_direct_start_turn(936, &["north-site"; 6]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let location_before = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == source_id)
        .expect("source before teleport")["location"]
        .clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    let draw_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "site-drawn")
        .expect("site-drawn event");
    let teleport_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "unit-teleported")
        .expect("unit-teleported event");
    assert!(
        draw_index < teleport_index,
        "Atlas draw must resolve before random teleport on the same minion without Lucky Charm deferral"
    );
    assert_eq!(
        receipt.events[draw_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        receipt.events[teleport_index].payload["sourceInstanceId"],
        source_id
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_ne!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_id)
            .expect("source after teleport")["location"],
        location_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0983_start_turn_draw_sites_then_teleport_draws_last_site_before_teleport_on_thin_atlas()
 {
    let mut session = draw_sites_teleport_direct_start_turn(983, &["north-site"; 4]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let location_before = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == source_id)
        .expect("source before teleport")["location"]
        .clone();
    let drawn_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .first()
        .expect("only site")["instanceId"]
        .clone();
    let atlas_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    assert_eq!(atlas_before, 1);
    let hand_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    let draw_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "site-drawn")
        .expect("site-drawn event");
    let teleport_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "unit-teleported")
        .expect("unit-teleported event");
    assert!(
        draw_index < teleport_index,
        "Atlas draw must resolve before random teleport on the same minion Start Phase trigger"
    );
    assert_eq!(
        receipt.events[draw_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        receipt.events[teleport_index].payload["sourceInstanceId"],
        source_id
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .len(),
        hand_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_id)
    );
    assert_ne!(
        after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == source_id)
            .expect("source after teleport")["location"],
        location_before
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "thin-library cross-zone ordering proof keeps setup and assertions together"
)]
fn rule_catalog_0995_start_turn_draw_sites_then_draw_spells_on_thin_libraries() {
    let mut session = draw_sites_draw_spells_stack_start_turn(
        995,
        &["north-site"; 4],
        &[
            "north-spell-card",
            "north-source",
            "north-source",
            "north-source",
        ],
    );
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let drawn_site_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .first()
        .expect("only site")["instanceId"]
        .clone();
    let drawn_spell_id = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("only spell")["instanceId"]
        .clone();
    let atlas_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let spellbook_before = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .len();
    assert_eq!(atlas_before, 1);
    assert_eq!(spellbook_before, 1);
    let atlas_hand_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand")
        .len();
    let spell_hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    let site_draw_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "site-drawn")
        .expect("site-drawn event");
    let spell_draw_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "spell-drawn")
        .expect("spell-drawn event");
    assert!(
        site_draw_index < spell_draw_index,
        "Atlas draw must resolve before Spellbook draw on the same minion Start Phase trigger"
    );
    assert_eq!(
        receipt.events[site_draw_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        receipt.events[spell_draw_index].payload["sourceInstanceId"],
        source_id
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .len(),
        atlas_hand_before + 1
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        spell_hand_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_site_id)
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_spell_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0948_start_turn_draw_sites_then_mill_sites_resolves_in_order_on_same_trigger() {
    let mut session = draw_sites_mill_sites_stack_start_turn(948, &["north-site"; 6]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    let draw_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "site-drawn")
        .expect("site-drawn event");
    let mill_index = receipt
        .events
        .iter()
        .position(|event| event.event_type == "site-discarded")
        .expect("site-discarded event");
    assert!(
        draw_index < mill_index,
        "Atlas draw must resolve before Atlas mill on the same minion Start Phase trigger"
    );
    assert_eq!(
        receipt.events[draw_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        receipt.events[mill_index].payload["sourceInstanceId"],
        source_id
    );
    assert_eq!(state(&session)["phase"], "draw");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0967_start_turn_draw_sites_then_mill_sites_mills_no_op_when_atlas_has_one_card() {
    let mut session = draw_sites_mill_sites_stack_start_turn(967, &["north-site"; 4]);
    assert_eq!(state(&session)["phase"], "start-turn");
    let before = state(&session);
    let source_id = before["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-source")
        .expect("source minion")["instanceId"]
        .clone();
    let drawn_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .first()
        .expect("only site")["instanceId"]
        .clone();
    let atlas_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    assert_eq!(atlas_before, 1);
    let hand_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand")
        .len();
    let cemetery_before = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert_eq!(event_types(&receipt), ["site-drawn"]);
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], source_id);
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .len(),
        hand_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == drawn_id)
    );
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        cemetery_before
    );
    assert_exact_replay(&session);
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

fn deathrite_start_turn_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-trigger-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-trigger-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-gain": minion(json!({
                "atStartOfControllerTurnControllerGainsLife": 2,
            })),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-gain",
                    "north-pulser",
                    "north-gain",
                    "north-pulser",
                    "north-gain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

struct PendingDeathriteStartTurnSetup {
    deathrite_ids: [String; 2],
    gain_id: String,
    session: Session,
}

fn try_pending_deathrite_during_start_turn(
    encoded: &str,
) -> Option<PendingDeathriteStartTurnSetup> {
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
    let gain = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-gain"
            && descriptor["cell"] == "C4"
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
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let gain_id = gain.0["cardInstanceId"].as_str()?.to_owned();
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
    if !offered.contains(&pulser_id) || !offered.contains(&gain_id) {
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
    Some(PendingDeathriteStartTurnSetup {
        deathrite_ids,
        gain_id,
        session,
    })
}

fn deathrite_start_turn_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_start_turn_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_start_turn(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites during Start Phase")
}

#[test]
fn rule_catalog_1163_resolve_start_turn_trigger_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_start_turn_seed_with(1163);
    let mut setup = try_pending_deathrite_during_start_turn(&encoded)
        .expect("complete start-turn Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let gain_id = setup.gain_id.clone();
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
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger"),
        "deathrite-order must issue no resolve-start-turn-trigger"
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
    assert_eq!(resumed["phase"], "start-turn");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == gain_id
            }),
        "resolve-start-turn-trigger must return once deathrite-order clears"
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == gain_id
    });
    assert_eq!(state(session)["phase"], "draw");
    assert_exact_replay(session);
}

fn deathrite_start_turn_teleport_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-teleport-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-teleport-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-teleport": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnTeleportToRandomSiteOrVoid": true,
                "attack": 3,
                "voidwalk": true,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-teleport",
                    "north-pulser",
                    "north-teleport",
                    "north-pulser",
                    "north-teleport",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

struct PendingDeathriteStartTurnTeleportSetup {
    deathrite_ids: [String; 2],
    session: Session,
    teleport_id: String,
}

fn try_pending_deathrite_during_start_turn_teleport(
    encoded: &str,
) -> Option<PendingDeathriteStartTurnTeleportSetup> {
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
    let teleport = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-teleport"
            && descriptor["cell"] == "C4"
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
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let teleport_id = teleport.0["cardInstanceId"].as_str()?.to_owned();
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
    if !offered.contains(&pulser_id) || !offered.contains(&teleport_id) {
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
    Some(PendingDeathriteStartTurnTeleportSetup {
        deathrite_ids,
        session,
        teleport_id,
    })
}

fn deathrite_start_turn_teleport_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_start_turn_teleport_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_start_turn_teleport(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites before start-turn teleport trigger")
}

#[test]
fn rule_catalog_1180_start_turn_teleport_trigger_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_start_turn_teleport_seed_with(1180);
    let mut setup = try_pending_deathrite_during_start_turn_teleport(&encoded)
        .expect("complete start-turn teleport Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let teleport_id = setup.teleport_id.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["pendingDeathrites"]["returnPhase"], "start-turn");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger"),
        "deathrite-order must issue no resolve-start-turn-trigger"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "resolve-start-turn-trigger"
                    || action.descriptor["sourceInstanceId"] != teleport_id
            })
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
    assert_eq!(resumed["phase"], "start-turn");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == teleport_id
            }),
        "draw-sites teleport trigger must return once deathrite-order clears"
    );
    assert_exact_replay(session);
}

fn deathrite_draw_mill_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-mill-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-mill-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-mill-card": minion(json!({})),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnDrawSpells": 1,
                "atStartOfControllerTurnMillSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-draw-card",
                    "north-mill-card",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

struct PendingDeathriteDrawMillSetup {
    deathrite_ids: [String; 2],
    stack_id: String,
    session: Session,
}

fn try_pending_deathrite_during_draw_mill_start_turn(
    encoded: &str,
) -> Option<PendingDeathriteDrawMillSetup> {
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
    let stack = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-stack"
            && descriptor["cell"] == "C4"
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
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let stack_id = stack.0["cardInstanceId"].as_str()?.to_owned();
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
    if !offered.contains(&pulser_id) || !offered.contains(&stack_id) {
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
    Some(PendingDeathriteDrawMillSetup {
        deathrite_ids,
        stack_id,
        session,
    })
}

fn deathrite_draw_mill_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_mill_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites during draw-mill start-turn")
}

#[test]
fn rule_catalog_1183_draw_then_mill_start_turn_trigger_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_draw_mill_seed_with(1183);
    let mut setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-mill start-turn Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let stack_id = setup.stack_id.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger"),
        "deathrite-order must issue no resolve-start-turn-trigger"
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "start-turn");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == stack_id
            }),
        "draw-then-mill start-turn trigger must return once deathrite-order clears"
    );
    assert_exact_replay(session);
}

fn deathrite_draw_sites_mill_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-mill-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-mill-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnMillSites": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-pulser", "north-stack", "north-stack", "north-stack", "north-stack", "north-stack"],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

fn try_pending_deathrite_during_draw_sites_mill_start_turn(
    encoded: &str,
) -> Option<PendingDeathriteDrawMillSetup> {
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
    let stack = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-stack"
            && descriptor["cell"] == "C4"
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
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let stack_id = stack.0["cardInstanceId"].as_str()?.to_owned();
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
    if !offered.contains(&pulser_id) || !offered.contains(&stack_id) {
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
    Some(PendingDeathriteDrawMillSetup {
        deathrite_ids,
        stack_id,
        session,
    })
}

#[test]
fn rule_catalog_1184_draw_sites_then_mill_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = (1184..1184 + 256)
        .map(deathrite_draw_sites_mill_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_during_draw_sites_mill_start_turn(candidate).is_some()
        })
        .expect("bounded seed that reaches pending Deathrites during draw-sites-mill start-turn");
    let mut setup = try_pending_deathrite_during_draw_sites_mill_start_turn(&encoded)
        .expect("complete draw-sites-mill start-turn Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let stack_id = setup.stack_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger")
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    assert_eq!(state(session)["phase"], "start-turn");
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == stack_id
            })
    );
    assert_exact_replay(session);
}

fn deathrite_draw_lure_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-lure-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-lure-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnDrawSpells": 1,
                "atStartOfControllerTurnLureNearbyEnemyMinion": true,
                "defense": 4,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-lure-target": minion(json!({ "summonToAnySite": true })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-draw-card",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": ["south-lure-target", "south-deathrite", "south-deathrite", "south-deathrite", "south-deathrite", "south-deathrite"],
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

fn try_pending_deathrite_during_draw_lure_start_turn(
    encoded: &str,
) -> Option<PendingDeathriteDrawMillSetup> {
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
    let stack = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-stack"
            && descriptor["cell"] == "C4"
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
            && descriptor["cardId"] == "south-lure-target"
            && descriptor["cell"] == "C1"
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
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let stack_id = stack.0["cardInstanceId"].as_str()?.to_owned();
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
    if !offered.contains(&pulser_id) || !offered.contains(&stack_id) {
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
    Some(PendingDeathriteDrawMillSetup {
        deathrite_ids,
        stack_id,
        session,
    })
}

#[test]
fn rule_catalog_1185_draw_spells_then_lure_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = (1185..1185 + 256)
        .map(deathrite_draw_lure_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_lure_start_turn(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites during draw-lure start-turn");
    let mut setup = try_pending_deathrite_during_draw_lure_start_turn(&encoded)
        .expect("complete draw-lure start-turn Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let stack_id = setup.stack_id.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger")
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    assert_eq!(state(session)["phase"], "start-turn");
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == stack_id
            })
    );
    assert_exact_replay(session);
}

fn assert_stacked_start_turn_withheld(setup: PendingDeathriteDrawMillSetup) {
    let deathrite_ids = setup.deathrite_ids.clone();
    let stack_id = setup.stack_id.clone();
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
                    && action.descriptor["sourceInstanceId"] == stack_id
            })
    );
    assert_exact_replay(&session);
}

fn deathrite_draw_spells_teleport_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-spells-teleport-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-spells-teleport-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnDrawSpells": 1,
                "atStartOfControllerTurnTeleportToRandomSiteOrVoid": true,
                "attack": 3,
                "voidwalk": true,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-draw-card",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

fn deathrite_draw_spells_teleport_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_spells_teleport_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-spells-teleport start-turn",
        )
}

#[test]
fn rule_catalog_1186_draw_spells_then_teleport_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = deathrite_draw_spells_teleport_seed_with(1186);
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-spells-teleport start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_sites_lure_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-lure-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-lure-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnLureNearbyEnemyMinion": true,
                "defense": 4,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-lure-target": minion(json!({ "summonToAnySite": true })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-lure-target",
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

fn deathrite_draw_sites_lure_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_sites_lure_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_lure_start_turn(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites during draw-sites-lure start-turn")
}

#[test]
fn rule_catalog_1187_draw_sites_then_lure_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = deathrite_draw_sites_lure_seed_with(1187);
    let setup = try_pending_deathrite_during_draw_lure_start_turn(&encoded)
        .expect("complete draw-sites-lure start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_sites_spells_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-spells-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-spells-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-spell-card": minion(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-spell-card",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

fn deathrite_draw_sites_spells_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_sites_spells_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-sites-draw-spells start-turn",
        )
}

#[test]
fn rule_catalog_1188_draw_sites_then_draw_spells_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = deathrite_draw_sites_spells_seed_with(1188);
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-sites-draw-spells start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_spells_life_gain_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-spells-life-gain-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-spells-life-gain-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerGainsLife": 2,
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-draw-card",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

fn deathrite_draw_spells_life_gain_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_spells_life_gain_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-spells-life-gain start-turn",
        )
}

#[test]
fn rule_catalog_1189_draw_spells_then_life_gain_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = deathrite_draw_spells_life_gain_seed_with(1189);
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-spells-life-gain start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn try_pending_deathrite_during_draw_sites_here_damage_start_turn(
    encoded: &str,
) -> Option<PendingDeathriteDrawMillSetup> {
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
    let stack = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-stack"
            && descriptor["cell"] == "C4"
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
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let stack_id = stack.0["cardInstanceId"].as_str()?.to_owned();
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
    if !offered.contains(&pulser_id) || !offered.contains(&stack_id) {
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
    Some(PendingDeathriteDrawMillSetup {
        deathrite_ids,
        stack_id,
        session,
    })
}

fn deathrite_draw_sites_here_damage_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-here-damage-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-here-damage-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
                "atStartOfControllerTurnDrawSites": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
            "south-visitor": minion(json!({ "summonToAnySite": true })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
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

fn deathrite_draw_sites_here_damage_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_sites_here_damage_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_during_draw_sites_here_damage_start_turn(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites during draw-sites-here-damage start-turn",
        )
}

#[test]
fn rule_catalog_1190_draw_sites_then_here_damage_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = deathrite_draw_sites_here_damage_seed_with(1190);
    let setup = try_pending_deathrite_during_draw_sites_here_damage_start_turn(&encoded)
        .expect("complete draw-sites-here-damage start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_spells_life_loss_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-spells-life-loss-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-spells-life-loss-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerLosesLife": 1,
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-draw-card",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

fn deathrite_draw_spells_life_loss_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_spells_life_loss_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-spells-life-loss start-turn",
        )
}

#[test]
fn rule_catalog_1193_draw_spells_then_life_loss_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = deathrite_draw_spells_life_loss_seed_with(1193);
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-spells-life-loss start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_spells_mana_gain_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-spells-mana-gain-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-spells-mana-gain-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-draw-card": minion(json!({})),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerGainsMana": 1,
                "atStartOfControllerTurnDrawSpells": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-draw-card",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

fn deathrite_draw_spells_mana_gain_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_spells_mana_gain_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-spells-mana-gain start-turn",
        )
}

#[test]
fn rule_catalog_1194_draw_spells_then_mana_gain_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = deathrite_draw_spells_mana_gain_seed_with(1194);
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-spells-mana-gain start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_gain_loss_stack_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-gain-loss-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-gain-loss-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerGainsLife": 3,
                "atStartOfControllerTurnControllerLosesLife": 2,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

fn deathrite_gain_loss_stack_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_gain_loss_stack_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites during gain-loss start-turn")
}

#[test]
fn rule_catalog_1195_gain_then_loss_start_turn_trigger_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_gain_loss_stack_seed_with(1195);
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete gain-loss start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_gain_here_damage_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-gain-here-damage-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-gain-here-damage-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerGainsLife": 2,
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
            "south-visitor": minion(json!({ "summonToAnySite": true })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
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

fn deathrite_gain_here_damage_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_gain_here_damage_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_during_draw_sites_here_damage_start_turn(candidate).is_some()
        })
        .expect("bounded seed that reaches pending Deathrites during gain-here-damage start-turn")
}

#[test]
fn rule_catalog_1196_gain_then_here_damage_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = deathrite_gain_here_damage_seed_with(1196);
    let setup = try_pending_deathrite_during_draw_sites_here_damage_start_turn(&encoded)
        .expect("complete gain-here-damage start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_triple_pulse_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-triple-pulse-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-triple-pulse-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerGainsLife": 3,
                "atStartOfControllerTurnControllerLosesLife": 2,
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
            "south-visitor": minion(json!({ "summonToAnySite": true })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
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

fn deathrite_triple_pulse_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_triple_pulse_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_during_draw_sites_here_damage_start_turn(candidate).is_some()
        })
        .expect("bounded seed that reaches pending Deathrites during triple-pulse start-turn")
}

#[test]
fn rule_catalog_1197_triple_pulse_start_turn_trigger_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_triple_pulse_seed_with(1197);
    let setup = try_pending_deathrite_during_draw_sites_here_damage_start_turn(&encoded)
        .expect("complete triple-pulse start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_sites_teleport_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-teleport-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-teleport-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnDrawSites": 1,
                "atStartOfControllerTurnTeleportToRandomSiteOrVoid": true,
                "attack": 3,
                "voidwalk": true,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

#[test]
fn rule_catalog_1203_draw_sites_then_teleport_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = (1203..1203 + 256)
        .map(deathrite_draw_sites_teleport_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-sites-teleport start-turn",
        );
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-sites-teleport start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_loss_here_damage_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-loss-here-damage-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-loss-here-damage-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerLosesLife": 2,
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
            "south-visitor": minion(json!({ "summonToAnySite": true })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
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

#[test]
fn rule_catalog_1204_loss_then_here_damage_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = (1204..1204 + 256)
        .map(deathrite_loss_here_damage_withheld_manifest)
        .find(|candidate| {
            try_pending_deathrite_during_draw_sites_here_damage_start_turn(candidate).is_some()
        })
        .expect("bounded seed that reaches pending Deathrites during loss-here-damage start-turn");
    let setup = try_pending_deathrite_during_draw_sites_here_damage_start_turn(&encoded)
        .expect("complete loss-here-damage start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_sites_life_gain_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-life-gain-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-life-gain-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerGainsLife": 2,
                "atStartOfControllerTurnDrawSites": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

#[test]
fn rule_catalog_1205_draw_sites_then_life_gain_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = (1205..1205 + 256)
        .map(deathrite_draw_sites_life_gain_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-sites-life-gain start-turn",
        );
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-sites-life-gain start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_sites_life_loss_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-life-loss-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-life-loss-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerLosesLife": 1,
                "atStartOfControllerTurnDrawSites": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

#[test]
fn rule_catalog_1206_draw_sites_then_life_loss_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = (1206..1206 + 256)
        .map(deathrite_draw_sites_life_loss_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-sites-life-loss start-turn",
        );
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-sites-life-loss start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}

fn deathrite_draw_sites_mana_gain_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-draw-sites-mana-gain-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-draw-sites-mana-gain-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-pulser": minion(json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-site": site(json!({})),
            "north-stack": minion(json!({
                "atStartOfControllerTurnControllerGainsMana": 1,
                "atStartOfControllerTurnDrawSites": 1,
            })),
            "south-avatar": avatar(),
            "south-deathrite": minion(json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                    "north-stack",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

#[test]
fn rule_catalog_1207_draw_sites_then_mana_gain_start_turn_trigger_withheld_during_pending_deathrite_order()
 {
    let encoded = (1207..1207 + 256)
        .map(deathrite_draw_sites_mana_gain_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_mill_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during draw-sites-mana-gain start-turn",
        );
    let setup = try_pending_deathrite_during_draw_mill_start_turn(&encoded)
        .expect("complete draw-sites-mana-gain start-turn Deathrite withheld setup");
    assert_stacked_start_turn_withheld(setup);
}
