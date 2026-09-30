#[path = "common/marked_death.rs"]
mod marked_death;

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode};
use sorcery_engine::game::GameError;
use sorcery_engine::session::{Session, SessionError, StepResult};

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

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    try_accept_where(session, predicate).expect("expected engine-issued action")
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
fn rule_catalog_1121_sinkhole_sacrifices_nearby_site_into_neutral_rubble() {
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

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_plain() -> Value {
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

fn deathrite_site_destruction_manifest(seed: u32) -> String {
    let fixture = "sinkhole-site-destruction-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-rain": rain_spell(),
            "north-site": {
                "cardType": "site",
                "elements": ["earth"],
                "sacrificeToDestroyNearbySite": true,
            },
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-deathrite": deathrite_plain(),
            "south-site": { "cardType": "site", "elements": ["earth"] },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-rain",
                    "north-rain",
                    "north-rain",
                    "north-rain",
                    "north-rain",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn terminal_water_site_destruction_manifest(south_atlas_count: usize) -> String {
    let mut value: Value = serde_json::from_str(&manifest()).expect("site destruction manifest");
    value.as_object_mut().unwrap().remove("manifestId");
    value["cards"]["south-spell"]["deathriteDrawSite"] = json!(true);
    value["cards"]["south-spell"]["submerge"] = json!(true);
    value["decks"]["south"]["atlas"] = json!(vec!["south-site"; south_atlas_count]);
    finish_manifest(value)
}

fn north_has_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
}

fn offers_site_destruction(session: &Session) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions
            .iter()
            .any(|action| action.descriptor["kind"] == "activate-site-destruction")
    })
}

fn assert_exact_replay(session: &Session) {
    assert!(session.verify_replay().expect("verified exact replay"));
}

struct PendingDeathriteSiteDestructionSetup {
    deathrite_ids: [String; 2],
    session: Session,
    source_id: String,
}

fn try_pending_deathrite_with_site_destruction_legal(
    encoded: &str,
) -> Option<PendingDeathriteSiteDestructionSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    let played = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let source_id = played.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if !offers_site_destruction(&session) {
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
    Some(PendingDeathriteSiteDestructionSetup {
        deathrite_ids,
        session,
        source_id,
    })
}

fn deathrite_site_destruction_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_site_destruction_manifest)
        .find(|candidate| try_pending_deathrite_with_site_destruction_legal(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with legal Sinkhole site destruction")
}

#[test]
fn rule_catalog_1129_activate_site_destruction_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_site_destruction_seed_with(1129);
    let mut setup = try_pending_deathrite_with_site_destruction_legal(&encoded)
        .expect("complete Sinkhole activate-site-destruction Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let source_id = setup.source_id.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["realm"]["sites"]["C4"]["instanceId"], source_id);
    marked_death::assert_live_marked_before_cemetery(&paused, &deathrite_ids);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "end-turn"
                    && action.descriptor["kind"] != "activate-site-destruction"
            })
    );
    assert!(!offers_site_destruction(session));

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
    assert_eq!(resumed["realm"]["sites"]["C4"]["instanceId"], source_id);
    assert!(offers_site_destruction(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-site-destruction"
            && descriptor["sourceSiteInstanceId"] == source_id
            && descriptor["targetCell"] == "C4"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-sacrificed", "site-destroyed", "rubble-created"]
    );
    assert_eq!(state(session)["realm"]["sites"]["C4"]["rubble"], true);
    assert_exact_replay(session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one Session site-tail guard paired with the populated Atlas control"
)]
fn terminal_sinkhole_site_destruction_rejects_unowned_site_tail_and_accepts_populated_atlas() {
    let run = |south_atlas_count| {
        let mut session =
            Session::new(&terminal_water_site_destruction_manifest(south_atlas_count))
                .expect("valid terminal Sinkhole fixture");
        keep(&mut session);
        keep(&mut session);
        let (_home_site, _) = try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
        })
        .expect("play North home site");
        try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
            .expect("end North turn");
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
        .expect("draw South Spellbook");
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        .expect("play South Water site at C1");
        let (minion, _) = try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-spell"
                && descriptor["cell"] == "C1"
                && descriptor["region"] == "underwater"
        })
        .expect("summon South submerged Deathrite");
        try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
            .expect("end South turn");
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        })
        .expect("draw North Atlas");
        let minion_id = minion["cardInstanceId"].as_str().expect("minion ID");
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
        })
        .expect("extend North sites through C3");
        try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
            .expect("end North site-extension turn");
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
        .expect("draw South Spellbook");
        try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
            .expect("end South Spellbook turn");
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        })
        .expect("draw North Atlas between site plays");
        let (source, _) = try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
        })
        .expect("play North source site at C2");
        let source_id = source["cardInstanceId"].as_str().expect("source ID");
        let action = session
            .legal_actions()
            .expect("site destruction actions")
            .into_iter()
            .find(|action| {
                action.descriptor["kind"] == "activate-site-destruction"
                    && action.descriptor["sourceSiteInstanceId"] == source_id
                    && action.descriptor["targetCell"] == "C1"
            })
            .expect("engine-issued Water destruction action");
        (session, action, source_id.to_owned(), minion_id.to_owned())
    };

    let (mut session, action, _source_id, minion_id) = run(3);
    let before = state(&session);
    assert_eq!(before["realm"]["units"][0]["instanceId"], minion_id);
    assert_eq!(before["realm"]["units"][0]["region"], "underwater");
    assert_eq!(
        before["players"]["south"]["atlas"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let checkpoint = create_game_checkpoint(&session).expect("pre-terminal-action checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let north_view = session
        .public_view(sorcery_engine::contract::Seat::North)
        .unwrap();
    let south_view = session
        .public_view(sorcery_engine::contract::Seat::South)
        .unwrap();
    let state_hash = session.state_hash().unwrap();
    let transcript = session.transcript().to_vec();
    let attempts = session.attempts().to_vec();
    let request = action_request(&action);
    let reason = "terminal site destruction has an unowned local tail";
    assert!(matches!(
        session.step(request.clone()),
        Err(SessionError::Game(GameError::UnsupportedMechanic(actual))) if actual == reason
    ));
    assert_eq!(session.unsupported_mechanic(), Some(reason));
    assert_eq!(
        session
            .public_view(sorcery_engine::contract::Seat::North)
            .unwrap(),
        north_view
    );
    assert_eq!(
        session
            .public_view(sorcery_engine::contract::Seat::South)
            .unwrap(),
        south_view
    );
    assert_eq!(session.state_hash().unwrap(), state_hash);
    assert_eq!(session.transcript(), transcript);
    assert_eq!(session.attempts(), attempts);
    assert!(session.legal_actions().is_err());
    assert!(session.verify_replay().is_err());
    assert!(create_game_checkpoint(&session).is_err());
    let mut restored =
        resume_game_checkpoint(&parse_game_checkpoint(&serialized).expect("parsed checkpoint"))
            .expect("restored pre-attempt checkpoint");
    assert_eq!(restored.replay_value().unwrap()["state"], before);
    assert!(matches!(
        restored.step(request),
        Err(SessionError::Game(GameError::UnsupportedMechanic(actual))) if actual == reason
    ));
    assert_eq!(restored.unsupported_mechanic(), Some(reason));
    assert_eq!(
        restored
            .public_view(sorcery_engine::contract::Seat::North)
            .unwrap(),
        north_view
    );
    assert_eq!(
        restored
            .public_view(sorcery_engine::contract::Seat::South)
            .unwrap(),
        south_view
    );
    assert_eq!(restored.state_hash().unwrap(), state_hash);
    assert_eq!(restored.transcript(), transcript);
    assert_eq!(restored.attempts(), attempts);
    assert!(restored.verify_replay().is_err());
    assert!(create_game_checkpoint(&restored).is_err());
    let valid = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed valid checkpoint"),
    )
    .expect("restore valid branch");
    assert!(valid.verify_replay().unwrap());

    let (mut populated, action, populated_source_id, populated_minion_id) = run(12);
    let receipt = match populated
        .step(action_request(&action))
        .expect("populated Atlas Sinkhole action")
    {
        StepResult::Accepted(receipt) => receipt,
        StepResult::Rejected(_) => panic!("engine-issued Sinkhole action must succeed"),
    };
    assert!(event_types(&receipt).contains(&"site-destroyed"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"site-drawn"));
    assert!(!event_types(&receipt).contains(&"game-ended"));
    let after = state(&populated);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["instanceId"] == populated_source_id)
    );
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["instanceId"] == populated_minion_id)
    );
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["cardId"] == "south-site")
    );
    assert_exact_replay(&populated);
}
