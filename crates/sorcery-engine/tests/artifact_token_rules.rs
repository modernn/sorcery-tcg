//! Direct proofs for conjured Artifact tokens: location choice, ordinary carrying,
//! checkpoint replay, and token-only exits.

use serde_json::{Value, json};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::selfplay_manifest_with;

fn manifest() -> String {
    selfplay_manifest_with(9107, |m| {
        let thresholds = json!({"air":0,"earth":0,"fire":0,"water":0});
        m["cards"] = json!({
            "avatar": {"cardType":"avatar","attack":1,"defense":1,"life":20,"drawSpell":false},
            "land": {"cardType":"site","elements":["earth"]},
            "water": {"cardType":"site","elements":["water"]},
            "token": {"cardType":"artifact","grantsBearerPower":2,"manaCost":null,"thresholds":thresholds,"token":true},
            "conjure": {"cardType":"magic","manaCost":0,"thresholds":thresholds,"effectProgram":{"effects":[
                {"op":"choose-location","relation":"anywhere"},
                {"op":"conjure-token","token":"token","count":2,"destination":"chosen-location"}
            ]}},
            "destroy": {"cardType":"magic","manaCost":0,"thresholds":thresholds,"destroyTargetArtifact":true},
            "return": {"cardType":"magic","manaCost":0,"thresholds":thresholds,"returnTargetArtifactToOwnerHand":true}
        });
        for (seat, site) in [("north", "land"), ("south", "water")] {
            m["decks"][seat] = json!({
                "avatar":"avatar", "atlas":vec![site;12],
                "spellbook":vec!["conjure","destroy","return","conjure","destroy","return"]
            });
        }
    })
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("engine-issued action");
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine action rejected")
    };
    (descriptor, receipt)
}

fn keep(session: &mut Session) {
    accept_where(session, |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay")["state"].clone()
}

fn ready(seat: &str) -> Session {
    ready_manifest(seat, &manifest())
}

fn ready_manifest(seat: &str, manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C4"
    });
    accept_where(&mut session, |d| d["kind"] == "end-turn");
    accept_where(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept_where(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C1"
    });
    if seat == "north" {
        accept_where(&mut session, |d| d["kind"] == "end-turn");
        accept_where(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "spellbook"
        });
    }
    session
}

fn assert_replay(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("checkpoint");
    let encoded = serialize_game_checkpoint(&checkpoint).expect("serialize checkpoint");
    let restored =
        resume_game_checkpoint(&parse_game_checkpoint(&encoded).expect("parse checkpoint"))
            .expect("resume checkpoint");
    assert_eq!(
        restored.replay_value().expect("restored replay"),
        session.replay_value().expect("replay")
    );
    let actions = session
        .transcript()
        .iter()
        .map(|r| r.action_id.clone())
        .collect::<Vec<_>>();
    let replayed = Session::replay(session.manifest_json(), &actions).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed"),
        session.replay_value().expect("state")
    );
}

fn artifact_ids(value: &Value) -> Vec<String> {
    value["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .map(|a| a["instanceId"].as_str().expect("artifact id").to_owned())
        .collect()
}

#[test]
fn conjured_artifact_tokens_choose_all_regions_for_both_seats_and_resume_exactly() {
    for seat in ["north", "south"] {
        let root = ready(seat);
        let mut pending = root.clone();
        accept_where(&mut pending, |d| {
            d["kind"] == "cast-magic" && d["cardId"] == "conjure"
        });
        assert_eq!(pending.legal_actions().expect("location actions").len(), 22);
        assert_replay(&pending);
        for location in [
            json!({"cell":"C4","region":"surface"}),
            json!({"cell":"C1","region":"underwater"}),
            json!({"cell":"C4","region":"underground"}),
            json!({"cell":"A1","region":"void"}),
        ] {
            let mut branch = pending.clone();
            let (_, receipt) = accept_where(&mut branch, |d| {
                d["kind"] == "choose-ability-location" && d["location"] == location
            });
            assert_eq!(
                receipt
                    .events
                    .iter()
                    .filter(|e| e.event_type == "artifact-conjured")
                    .count(),
                2
            );
            let branch_state = state(&branch);
            assert_eq!(
                branch_state["cards"]["token"].get("manaCost"),
                Some(&Value::Null)
            );
            let artifacts = branch_state["realm"]["artifacts"]
                .as_array()
                .expect("artifacts");
            assert_eq!(artifacts.len(), 2);
            let ids = artifacts
                .iter()
                .map(|a| a["instanceId"].as_str().expect("token identity"))
                .collect::<Vec<_>>();
            assert_ne!(ids[0], ids[1]);
            for artifact in artifacts {
                assert_eq!(artifact["source"], "token");
                assert_eq!(artifact["cardId"], "token");
                assert_eq!(artifact["owner"], seat);
                assert_eq!(artifact["location"], location["cell"]);
                assert_eq!(artifact["region"], location["region"]);
            }
            assert!(
                receipt
                    .events
                    .iter()
                    .filter(|e| e.event_type == "artifact-conjured")
                    .all(|e| {
                        e.payload["cardId"] == "token"
                            && e.payload["owner"] == seat
                            && e.payload["manaPaid"] == 0
                    })
            );
            assert_replay(&branch);
        }
    }
}

#[test]
fn token_artifacts_are_pickup_and_drop_objects_for_both_seats() {
    for seat in ["north", "south"] {
        let mut session = ready(seat);
        accept_where(&mut session, |d| {
            d["kind"] == "cast-magic" && d["cardId"] == "conjure"
        });
        let location = if seat == "north" {
            json!({"cell":"C4","region":"surface"})
        } else {
            json!({"cell":"C1","region":"surface"})
        };
        accept_where(&mut session, |d| {
            d["kind"] == "choose-ability-location" && d["location"] == location
        });
        // The casting avatar interacted this turn; rotate once so pickup is issued.
        accept_where(&mut session, |d| d["kind"] == "end-turn");
        accept_where(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "atlas"
        });
        accept_where(&mut session, |d| d["kind"] == "end-turn");
        accept_where(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "spellbook"
        });
        let (_, picked) = accept_where(&mut session, |d| d["kind"] == "pick-up-artifacts");
        assert!(
            picked
                .events
                .iter()
                .any(|e| e.event_type == "artifacts-picked-up")
        );
        let carried_state = state(&session);
        let carried = carried_state["realm"]["artifacts"]
            .as_array()
            .expect("artifacts");
        assert!(carried.iter().all(|a| a["bearer"].is_object()));
        let (_, dropped) = accept_where(&mut session, |d| d["kind"] == "drop-artifacts");
        assert!(
            dropped
                .events
                .iter()
                .any(|e| e.event_type == "artifacts-dropped")
        );
        let dropped_state = state(&session);
        assert!(
            dropped_state["realm"]["artifacts"]
                .as_array()
                .expect("artifacts")
                .iter()
                .all(|a| a["location"] == location["cell"] && a.get("bearer").is_none())
        );
        assert_replay(&session);
    }
}

#[test]
fn token_artifacts_use_ordinary_pickup_drop_and_targeted_exits_banish_without_zones() {
    let mut session = ready("north");
    accept_where(&mut session, |d| {
        d["kind"] == "cast-magic" && d["cardId"] == "conjure"
    });
    accept_where(&mut session, |d| {
        d["kind"] == "choose-ability-location"
            && d["location"] == json!({"cell":"C4","region":"surface"})
    });
    let ids = artifact_ids(&state(&session));
    assert_eq!(ids.len(), 2);
    accept_where(&mut session, |d| d["kind"] == "end-turn");
    accept_where(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept_where(&mut session, |d| d["kind"] == "end-turn");
    accept_where(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "spellbook"
    });
    let (_, picked) = accept_where(&mut session, |d| d["kind"] == "pick-up-artifacts");
    assert!(
        picked
            .events
            .iter()
            .any(|e| e.event_type == "artifacts-picked-up")
    );
    assert!(
        state(&session)["realm"]["artifacts"]
            .as_array()
            .expect("artifacts")
            .iter()
            .all(|a| a["bearer"].is_object())
    );
    let (_, dropped) = accept_where(&mut session, |d| d["kind"] == "drop-artifacts");
    assert!(
        dropped
            .events
            .iter()
            .any(|e| e.event_type == "artifacts-dropped")
    );

    let destroy_id = ids[0].clone();
    let return_id = ids[1].clone();
    let mut destroy = session.clone();
    let (_, destroyed) = accept_where(&mut destroy, |d| {
        d["kind"] == "cast-magic"
            && d["cardId"] == "destroy"
            && d["targetArtifactInstanceId"] == json!(destroy_id)
    });
    assert!(
        destroyed
            .events
            .iter()
            .any(|e| e.event_type == "artifact-banished")
    );
    assert!(!destroyed.events.iter().any(
        |e| e.event_type == "artifact-destroyed" || e.event_type == "artifact-returned-to-hand"
    ));
    let after_destroy = state(&destroy);
    assert!(!artifact_ids(&after_destroy).contains(&destroy_id));
    assert!(!after_destroy["players"].to_string().contains(&destroy_id));
    assert_replay(&destroy);

    let mut returned = session.clone();
    let (_, bounced) = accept_where(&mut returned, |d| {
        d["kind"] == "cast-magic"
            && d["cardId"] == "return"
            && d["targetArtifactInstanceId"] == json!(return_id)
    });
    assert!(
        bounced
            .events
            .iter()
            .any(|e| e.event_type == "artifact-banished")
    );
    assert!(!bounced.events.iter().any(
        |e| e.event_type == "artifact-returned-to-hand" || e.event_type == "artifact-destroyed"
    ));
    let after_return = state(&returned);
    assert!(!artifact_ids(&after_return).contains(&return_id));
    assert!(!after_return["players"].to_string().contains(&return_id));
    assert_replay(&returned);
}

#[test]
fn artifacts_conjured_carried_attach_without_pickup_and_replay_for_both_seats() {
    let mut input: Value = serde_json::from_str(&manifest()).unwrap();
    input["cards"]["conjure"]["effectProgram"]["effects"] = json!([
        {"op":"conjure-token","token":"token","count":2,"destination":"source","placement":"carried"}
    ]);
    input.as_object_mut().unwrap().remove("manifestId");
    let input = selfplay_manifest_with(9107, |m| *m = input);
    for seat in ["north", "south"] {
        let mut session = ready_manifest(seat, &input);
        let (_, receipt) = accept_where(&mut session, |d| {
            d["kind"] == "cast-magic" && d["cardId"] == "conjure"
        });
        let current = state(&session);
        let artifacts = current["realm"]["artifacts"].as_array().unwrap();
        assert_eq!(artifacts.len(), 2);
        for artifact in artifacts {
            assert_eq!(artifact["owner"], seat);
            assert_eq!(artifact["bearer"]["seat"], seat);
            assert_eq!(artifact["bearer"]["kind"], "avatar");
        }
        assert_eq!(
            receipt
                .events
                .iter()
                .filter(|e| e.event_type == "artifact-conjured")
                .count(),
            2
        );
        assert!(
            !receipt
                .events
                .iter()
                .any(|e| e.event_type == "artifacts-picked-up")
        );
        assert_replay(&session);
        // Casting spent the avatar's ordinary interaction; rotate before dropping.
        for _ in 0..2 {
            accept_where(&mut session, |d| d["kind"] == "end-turn");
            accept_where(&mut session, |d| {
                d["kind"] == "draw" && d["zone"] == "atlas"
            });
        }
        accept_where(&mut session, |d| {
            d["kind"] == "drop-artifacts"
                && d["artifactInstanceIds"]
                    .as_array()
                    .is_some_and(|ids| ids.len() == 2)
        });
        assert!(
            state(&session)["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| a.get("bearer").is_none())
        );
        assert_replay(&session);
    }
}

#[test]
fn carried_strike_sources_modify_one_unit_strike_then_banish_with_exact_replay() {
    let mut input: Value = serde_json::from_str(&manifest()).unwrap();
    input.as_object_mut().unwrap().remove("manifestId");
    input["cards"]["token"]
        .as_object_mut()
        .unwrap()
        .remove("grantsBearerPower");
    input["cards"]["token"]["bearerUnitStrike"] = json!({
        "damageBonus":1,"firstStrike":true,"destroyAfterStrike":true
    });
    input["cards"]["conjure"]["effectProgram"]["effects"] = json!([
        {"op":"conjure-token","token":"token","count":2,"destination":"source","placement":"carried"}
    ]);
    let encoded = selfplay_manifest_with(9107, |m| *m = input);
    let mut session = ready_manifest("north", &encoded);
    accept_where(&mut session, |d| {
        d["kind"] == "cast-magic" && d["cardId"] == "conjure"
    });
    accept_where(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C3"
    });
    accept_where(&mut session, |d| d["kind"] == "end-turn");
    accept_where(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept_where(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C2"
    });
    for cell in ["C3", "C2"] {
        accept_where(&mut session, |d| d["kind"] == "end-turn");
        accept_where(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "atlas"
        });
        accept_where(&mut session, |d| {
            d["kind"] == "move-and-attack" && d["to"]["cell"] == cell
        });
        accept_where(&mut session, |d| d["kind"] == "decline-attack");
    }
    accept_where(&mut session, |d| d["kind"] == "end-turn");
    accept_where(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept_where(&mut session, |d| {
        d["kind"] == "move-and-attack" && d["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |d| {
        d["kind"] == "declare-attack" && d["target"]["kind"] == "avatar"
    });
    assert_replay(&session);
    let (_, receipt) = accept_where(&mut session, |d| {
        d["kind"] == "close-defend" && d["originalTargetParticipates"] == true
    });
    let damage = receipt
        .events
        .iter()
        .filter(|e| e.event_type == "damage-dealt")
        .map(|e| (e.payload["seat"].clone(), e.payload["amount"].clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        damage,
        vec![(json!("south"), json!(3)), (json!("north"), json!(1))]
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "artifact-consumed-after-strike")
            .count(),
        2
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "artifact-banished")
            .count(),
        2
    );
    assert!(state(&session)["realm"].get("artifacts").is_none());
    assert_replay(&session);
}
