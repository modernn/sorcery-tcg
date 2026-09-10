//! Direct proofs for start-turn triggers: random teleports (RULE-CATALOG-0158),
//! controller Spellbook draws (RULE-CATALOG-0237–0238), and controller Atlas
//! draws (RULE-CATALOG-0241–0242).

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
