use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode};
use sorcery_engine::session::{Session, StepResult};

fn manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-destruction-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-site-destruction-rules-v1",
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-site": {
                "cardType": "site",
                "elements": ["earth"],
                "sacrificeToDestroyNearbySite": true,
            },
            "north-spell": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-site": { "cardType": "site", "elements": ["water"] },
            "south-spell": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-spell"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-spell"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 244,
    });
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
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
    session.replay_value().expect("replay value")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn play_site(session: &mut Session, cell: &str) -> Value {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    })
    .0
}

fn end_turn_and_draw_atlas(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn action_request(action: &sorcery_engine::contract::LegalAction) -> ActionRequest {
    ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario keeps action ordering, checkpoint branches, state, and replay together"
)]
fn rule_catalog_0012_sinkhole_sacrifices_sites_into_neutral_rubble() {
    let mut session = Session::new(&manifest()).expect("valid Sinkhole scenario");
    keep(&mut session);
    keep(&mut session);
    let north_c4 = play_site(&mut session, "C4");
    end_turn_and_draw_atlas(&mut session);
    play_site(&mut session, "C1");
    end_turn_and_draw_atlas(&mut session);
    let north_c3 = play_site(&mut session, "C3");
    end_turn_and_draw_atlas(&mut session);
    play_site(&mut session, "C2");
    end_turn_and_draw_atlas(&mut session);

    let source_id = north_c3["cardInstanceId"]
        .as_str()
        .expect("source instance ID");
    let c4_id = north_c4["cardInstanceId"].as_str().expect("C4 instance ID");
    let source_actions = session
        .legal_actions()
        .expect("Sinkhole actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "activate-site-destruction"
                && action.descriptor["sourceSiteInstanceId"] == source_id
        })
        .collect::<Vec<_>>();
    assert_eq!(
        source_actions
            .iter()
            .map(|action| action.descriptor["targetCell"]
                .as_str()
                .expect("target cell"))
            .collect::<Vec<_>>(),
        ["C2", "C3", "C4"]
    );
    assert_eq!(
        source_actions
            .iter()
            .map(|action| action.label.as_str())
            .collect::<Vec<_>>(),
        [
            "Sacrifice site to destroy C2",
            "Sacrifice site to destroy C3",
            "Sacrifice site to destroy C4",
        ]
    );

    let checkpoint = create_game_checkpoint(&session).expect("captured checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let restored =
        resume_game_checkpoint(&parse_game_checkpoint(&serialized).expect("parsed checkpoint"))
            .expect("restored checkpoint");
    assert_eq!(state(&restored), state(&session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );

    let mut self_targeted = restored.clone();
    let (_, self_receipt) = accept_where(&mut self_targeted, |descriptor| {
        descriptor["kind"] == "activate-site-destruction"
            && descriptor["sourceSiteInstanceId"] == source_id
            && descriptor["targetCell"] == "C3"
    });
    assert_eq!(
        event_types(&self_receipt),
        ["site-sacrificed", "site-destroyed", "rubble-created"]
    );
    assert!(self_receipt.random_draws.is_empty());
    let self_state = state(&self_targeted);
    assert_eq!(
        self_state["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .filter(|card| card["instanceId"] == source_id)
            .count(),
        1
    );
    assert_eq!(self_state["realm"]["sites"]["C4"]["instanceId"], c4_id);
    assert!(self_targeted.verify_replay().expect("self-target replay"));

    let mut destroyed = restored;
    let action = destroyed
        .legal_actions()
        .expect("destructive actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "activate-site-destruction"
                && action.descriptor["sourceSiteInstanceId"] == source_id
                && action.descriptor["targetCell"] == "C2"
        })
        .expect("C2 destruction action");
    let request = action_request(&action);
    let before_version = destroyed.state_version();
    let StepResult::Accepted(receipt) = destroyed
        .step(request.clone())
        .expect("accepted destruction")
    else {
        panic!("issued destruction action must be accepted");
    };
    assert_eq!(destroyed.state_version(), before_version + 1);
    assert_eq!(
        event_types(&receipt),
        [
            "site-sacrificed",
            "site-destroyed",
            "rubble-created",
            "rubble-created",
        ]
    );
    assert!(receipt.random_draws.is_empty());
    let target_id = receipt.events[1].payload["instanceId"]
        .as_str()
        .expect("target instance ID");
    let destroyed_state = state(&destroyed);
    for (cell, destroyed_id) in [("C2", target_id), ("C3", source_id)] {
        let expected = identity_hash(&json!({
            "cell": cell,
            "destroyedSiteInstanceId": destroyed_id,
            "kind": "rubble",
            "sourceInstanceId": source_id,
        }))
        .expect("deterministic Rubble identity");
        assert_eq!(
            destroyed_state["realm"]["sites"][cell],
            json!({
                "controller": null,
                "instanceId": expected,
                "rubble": true,
            })
        );
    }
    assert!(
        destroyed_state["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == source_id)
    );
    assert!(
        destroyed_state["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert!(matches!(
        destroyed.step(request).expect("stable stale rejection"),
        StepResult::Rejected(rejection) if rejection.code == RejectionCode::StaleVersion
    ));
    assert!(destroyed.verify_replay().expect("exact Sinkhole replay"));
}
