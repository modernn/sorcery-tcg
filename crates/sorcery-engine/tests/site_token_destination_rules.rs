//! Non-target site destinations use issued choices and replayable Session history.
use serde_json::{Value, json};
use sorcery_engine::ability::AbilityProgram;
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::ActionRequest;
use sorcery_engine::session::{Session, StepResult};

#[test]
fn site_destination_schema_is_bounded_and_conjuring_is_rejected() {
    let valid = json!({"effects":[{"op":"summon-token","token":"synthetic-token","count":1,
        "destination":{"choose-site":{"area":"realm","controller":"allied"}}}]});
    let program: AbilityProgram = serde_json::from_value(valid.clone()).unwrap();
    program.validate().unwrap();
    assert_eq!(
        serde_json::to_value(program).unwrap()["effects"],
        valid["effects"]
    );
    for destination in [
        json!({"choose-site":{"area":"realm"}}),
        json!({"choose-site":{"area":"realm","controller":"allied","target":true}}),
        json!({"each-site":{"area":"anywhere","controller":"any"}}),
    ] {
        let mut invalid = valid.clone();
        invalid["effects"][0]["destination"] = destination;
        assert!(serde_json::from_value::<AbilityProgram>(invalid).is_err());
    }
    let mut invalid = valid;
    invalid["effects"][0]["op"] = json!("conjure-token");
    let program: AbilityProgram = serde_json::from_value(invalid).unwrap();
    assert!(program.validate().is_err());
}

fn state(session: &Session) -> Value {
    session.replay_value().unwrap()["state"].clone()
}

fn checkpoint(session: &Session) {
    let saved = create_game_checkpoint(session).unwrap();
    let saved = parse_game_checkpoint(&serialize_game_checkpoint(&saved).unwrap()).unwrap();
    let resumed = resume_game_checkpoint(&saved).unwrap();
    let mut mismatched = saved.clone();
    mismatched.manifest["authority"]["contentHash"] =
        json!(identity_hash(&json!({"mismatched":true})).unwrap());
    assert!(resume_game_checkpoint(&mismatched).is_err());
    assert_eq!(
        resumed.session_hash().unwrap(),
        session.session_hash().unwrap()
    );
    assert_eq!(
        resumed.legal_actions().unwrap(),
        session.legal_actions().unwrap()
    );
    assert_eq!(resumed.transcript(), session.transcript());
}

#[expect(
    clippy::too_many_lines,
    reason = "one bounded issued-action recipe retains setup, payment, choices and replay assertions together"
)]
fn run_case(case: &Value) -> Value {
    let mut manifest = case["manifest"].clone();
    manifest["manifestId"] = json!(identity_hash(&manifest).unwrap());
    let manifest_json = canonical_json(&manifest).unwrap();
    let mut session = Session::new(&manifest_json).unwrap();
    let tested = case["seat"].as_str().unwrap();
    let magic = case["magic"].as_str().unwrap();
    let caster_mode = case["caster"].as_str().unwrap();
    let mut played = [0_usize; 2];
    let mut resolving = false;
    let mut checkpoints = 0;
    let mut selected = 0;
    let mut caster_summoned_at = None;
    let mut cast_turn = 0;
    let mut cast_mana = 0;
    let mut cast_instance = Value::Null;
    let mut cast_receipt_start = 0;
    let mut checkpoint_states = Vec::new();
    for _ in 0..800 {
        let snapshot = state(&session);
        let seat = snapshot["decisionSeat"].as_str().unwrap();
        let index = usize::from(seat == "south");
        let plan = case["sitePlans"][seat].as_array().unwrap();
        let actions = session.legal_actions().unwrap();
        let find = |predicate: &dyn Fn(&Value) -> bool| {
            actions.iter().find(|a| predicate(&a.descriptor)).cloned()
        };
        let avatar = snapshot["players"][tested]["avatar"]["card"]["instanceId"].clone();
        let caster = snapshot["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u["cardId"] == "syn:caster" && u["controller"] == tested)
            .map(|u| u["instanceId"].clone());
        let action = if snapshot["phase"] == "mulligan" {
            find(&|d| {
                d["kind"] == "mulligan"
                    && d["atlasOrder"] == json!([])
                    && d["spellbookOrder"] == json!([])
            })
        } else if let Some(action) = find(&|d| d["kind"] == "draw") {
            let zone = if played[index] < plan.len()
                && snapshot["players"][seat]["hand"]["atlas"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            {
                "atlas"
            } else {
                "spellbook"
            };
            find(&|d| d["kind"] == "draw" && d["zone"] == zone).or(Some(action))
        } else if resolving && snapshot["phase"] == "ability-choice" {
            find(&|d| {
                d["kind"] == "choose-ability-location"
                    && d["location"]["region"]
                        == *case.get("placementRegion").unwrap_or(&json!("surface"))
                    && case
                        .get("placementCell")
                        .is_none_or(|cell| d["location"]["cell"] == *cell)
            })
            .or_else(|| find(&|d| d["kind"] == "choose-ability" && d["target"].is_null()))
        } else if resolving && snapshot["pendingTriggerOrder"].is_object() {
            actions.first().cloned()
        } else if !resolving
            && seat == tested
            && played[index] >= usize::try_from(case["requiredSites"].as_u64().unwrap()).unwrap()
        {
            let chosen_caster = if caster_mode == "avatar" {
                Some(avatar.clone())
            } else {
                caster.clone()
            };
            let ready = caster_mode == "avatar"
                || caster_summoned_at
                    .is_some_and(|turn| snapshot["turnNumber"].as_u64().unwrap() > turn);
            if ready {
                find(&|d| {
                    d["kind"] == "cast-magic"
                        && d["cardId"] == magic
                        && Some(&d["casterInstanceId"]) == chosen_caster.as_ref()
                })
            } else {
                None
            }
        } else {
            None
        };
        let action = action
            .or_else(|| {
                if !resolving && played[index] < plan.len() {
                    let entry = &plan[played[index]];
                    find(&|d| {
                        d["kind"] == "play-site"
                            && d["cell"] == entry["cell"]
                            && (entry["rubble"].is_null() || d["createRubbleAt"] == entry["rubble"])
                    })
                } else {
                    None
                }
            })
            .or_else(|| {
                if !resolving
                    && seat == tested
                    && caster_mode == "minion"
                    && caster.is_none()
                    && played[index] >= plan.len()
                {
                    find(&|d| {
                        d["kind"] == "summon-minion"
                            && d["cardId"] == "syn:caster"
                            && d["cell"] == case["casterCell"]
                    })
                } else {
                    None
                }
            })
            .or_else(|| find(&|d| d["kind"] == "end-turn"));
        let action = action.unwrap_or_else(|| {
            panic!(
                "setup has no requested action: seat={seat} phase={} kinds={:?}",
                snapshot["phase"],
                actions
                    .iter()
                    .map(|a| &a.descriptor["kind"])
                    .collect::<Vec<_>>()
            )
        });
        let descriptor = action.descriptor.clone();
        if descriptor["kind"] == "play-site" {
            played[index] += 1;
        }
        if descriptor["kind"] == "summon-minion" {
            caster_summoned_at = snapshot["turnNumber"].as_u64();
        }
        if descriptor["kind"] == "cast-magic" {
            cast_receipt_start = session.transcript().len();
            cast_mana = snapshot["players"][tested]["mana"].as_u64().unwrap();
            cast_instance = descriptor["cardInstanceId"].clone();
            resolving = true;
            cast_turn = snapshot["turnNumber"].as_u64().unwrap();
            assert!(descriptor.get("target").is_none());
            assert!(descriptor.get("targetLocation").is_none());
        }
        let request = ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        };
        let result = session.step(request);
        if descriptor["kind"] == "cast-magic" && case["expectUnsupported"] == true {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("multi-token site layer assignment")
            );
            return json!({"status":"BLOCKED_R3_AS_EXPECTED","manifestHash":identity_hash(&manifest).unwrap(),"castDescriptor":descriptor,"unsupported":session.unsupported_mechanic()});
        }
        assert!(matches!(result.unwrap(), StepResult::Accepted(_)));
        if resolving && descriptor["kind"] == "choose-ability-location" {
            selected += 1;
        }
        if resolving
            && (selected == 0 || selected == 2)
            && state(&session)["phase"] == "ability-choice"
        {
            checkpoint(&session);
            checkpoints += 1;
            checkpoint_states.push(state(&session));
        }
        if resolving && state(&session)["phase"] == "main" {
            let after = state(&session);
            let units = after["realm"]["units"].as_array().unwrap();
            let token = case["token"].as_str().unwrap();
            let mut cells: Vec<_> = units
                .iter()
                .filter(|u| u["cardId"] == token)
                .map(|u| u["location"].clone())
                .collect();
            cells.sort_by_key(Value::to_string);
            let mut expected = case["expectedCells"].as_array().unwrap().clone();
            expected.sort_by_key(Value::to_string);
            assert_eq!(cells, expected);
            assert_eq!(
                after["players"][tested]["mana"].as_u64().unwrap() + case["cost"].as_u64().unwrap(),
                cast_mana
            );
            assert_eq!(
                after["players"][tested]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|card| card["instanceId"] == cast_instance)
                    .count(),
                1
            );
            assert!(
                units
                    .iter()
                    .filter(|unit| unit["cardId"] == token)
                    .all(|unit| unit["owner"] == tested
                        && unit["controller"] == tested
                        && unit["summoningSickness"] == true
                        && unit["tapped"] == false)
            );
            if let Some(draws) = case["expectedDraws"].as_u64() {
                assert_eq!(
                    session.transcript()[cast_receipt_start..]
                        .iter()
                        .flat_map(|receipt| &receipt.events)
                        .filter(|event| event.event_type == "spell-drawn")
                        .count() as u64,
                    draws
                );
            }
            if let Some(entries) = case["expectedEntries"].as_u64() {
                assert_eq!(
                    session.transcript()[cast_receipt_start..]
                        .iter()
                        .flat_map(|receipt| &receipt.events)
                        .filter(|event| event.event_type == "minion-summoned")
                        .count() as u64,
                    entries
                );
            }
            assert!(session.verify_replay().unwrap());
            checkpoint(&session);
            return json!({"status":"PASS","manifestHash":identity_hash(&manifest).unwrap(),"sessionHash":session.session_hash().unwrap(),"transcriptHash":session.transcript_hash().unwrap(),"checkpointCount":checkpoints+1,"castTurn":cast_turn,"checkpointStates":checkpoint_states,"state":after,"transcript":session.transcript()});
        }
        assert!(
            snapshot["turnNumber"].as_u64().unwrap() < 49,
            "bounded setup exhausted"
        );
    }
    panic!("bounded action limit exhausted")
}

#[test]
#[ignore = "requires source-guarded private candidate input; no authority bytes in repository"]
fn private_source_guarded_site_token_candidates() {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let input = std::env::var("SORCERY_SITE_TOKEN_INPUT").expect("explicit ignored input path");
    let value: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    for guard in value["sourceGuards"].as_array().unwrap() {
        let bytes = std::fs::read(guard["path"].as_str().unwrap()).unwrap();
        let mut digest = String::new();
        for byte in Sha256::digest(&bytes) {
            write!(digest, "{byte:02x}").unwrap();
        }
        assert_eq!(digest, guard["sha256"].as_str().unwrap());
    }
    let mut results = Vec::new();
    for case in value["cases"].as_array().unwrap() {
        let result = std::panic::catch_unwind(|| run_case(case)).unwrap_or_else(|error| {
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap_or("candidate panic");
            json!({"status":"FAIL", "error":message})
        });
        results.push(json!({"id":case["id"],"result":result}));
    }
    std::fs::write(
        value["output"].as_str().unwrap(),
        serde_json::to_vec_pretty(&results).unwrap(),
    )
    .unwrap();
    assert!(
        results
            .iter()
            .all(|result| result["result"]["status"] != "FAIL"),
        "candidate failures retained in private output"
    );
}
