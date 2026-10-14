//! A transition must not commit a decision whose strike cannot be enumerated.
use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::game::{Game, GameError};
use sorcery_engine::game_record::game_record_from_session;
use sorcery_engine::session::{Session, SessionError, StepResult};
use sorcery_engine::session_json::{RpcRequest, SessionJsonService};
const DOUBLE_UNSUPPORTED: &str =
    "additive and doubling strike modifiers require a damage-order choice";
fn finish(mut manifest: Value) -> String {
    manifest["manifestId"] = json!(identity_hash(&manifest).unwrap());
    canonical_json(&manifest).unwrap()
}

fn try_accept(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    match session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    {
        StepResult::Accepted(receipt) => Some((descriptor, receipt)),
        StepResult::Rejected(_) => None,
    }
}

fn keep(session: &mut Session) -> Option<()> {
    try_accept(session, |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    })
    .map(|_| ())
}

fn rpc(service: &mut SessionJsonService, method: &str, params: &Value) -> Value {
    let request: RpcRequest = serde_json::from_value(json!({
        "schemaVersion": 1, "id": 1, "method": method, "params": params,
    }))
    .unwrap();
    serde_json::to_value(service.handle(&request)).unwrap()
}

fn is_unsupported<T>(result: &Result<T, SessionError>, expected: &str) -> bool {
    matches!(
        result,
        Err(SessionError::Game(GameError::UnsupportedMechanic(reason))) if reason == expected
    )
}

fn lance_double_manifest(seed: u32, doubled: bool) -> String {
    let avatar = json!({
        "attack": 1, "cardType": "avatar", "defense": 1, "drawSpell": false, "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let earth = json!({ "air": 0, "earth": 1, "fire": 0, "water": 0 });
    let none = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "audit-lance-double" })).unwrap(),
            "mode": "synthetic",
            "revisionId": "synthetic-audit-lance-double-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-defender": {
                "attack": 1, "cardType": "minion", "defense": 5, "manaCost": 1,
                "movementBonus": 1, "movesOnlySideways": false, "thresholds": earth,
            },
            "north-site": site,
            "south-attacker": {
                "attack": 1, "cardType": "minion", "defense": 4, "lanceCount": 1,
                "manaCost": 1, "thresholds": earth,
            },
            "south-avatar": avatar,
            "south-grant": {
                "cardType": "magic", "grantDoubleDamageToAllyNextStrikeThisTurn": true,
                "manaCost": 0, "thresholds": none,
            },
            "south-site": site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 4],
                "avatar": "north-avatar",
                "spellbook": vec!["north-defender"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-attacker", "south-grant", "south-attacker", "south-grant",
                    "south-attacker", "south-grant", "south-attacker", "south-grant",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    if !doubled {
        manifest["cards"]["south-attacker"]
            .as_object_mut()
            .unwrap()
            .remove("lanceCount");
    }
    finish(manifest)
}

fn id_of(descriptor: &Value) -> String {
    descriptor["cardInstanceId"].as_str().unwrap().to_owned()
}

/// Mirrors `defend_rules.rs` `declared_attack`, plus a Grant cast before the attack.
fn before_allocation(seed: u32, doubled: bool) -> Option<Session> {
    let mut s = Session::new(&lance_double_manifest(seed, doubled)).ok()?;
    keep(&mut s)?;
    keep(&mut s)?;
    try_accept(&mut s, |d| d["kind"] == "play-site" && d["cell"] == "C4")?;
    let (summon, _) = try_accept(&mut s, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "north-defender" && d["cell"] == "C4"
    })?;
    let defender_id = id_of(&summon);
    try_accept(&mut s, |d| d["kind"] == "end-turn")?;

    try_accept(&mut s, |d| d["kind"] == "draw" && d["zone"] == "spellbook")?;
    try_accept(&mut s, |d| d["kind"] == "play-site" && d["cell"] == "C1")?;
    let (summon, _) = try_accept(&mut s, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "south-attacker" && d["cell"] == "C1"
    })?;
    let attacker_id = id_of(&summon);
    try_accept(&mut s, |d| d["kind"] == "end-turn")?;

    try_accept(&mut s, |d| d["kind"] == "draw" && d["zone"] == "spellbook")?;
    try_accept(&mut s, |d| d["kind"] == "play-site" && d["cell"] == "C3")?;
    try_accept(&mut s, |d| d["kind"] == "end-turn")?;

    try_accept(&mut s, |d| d["kind"] == "draw" && d["zone"] == "spellbook")?;
    try_accept(&mut s, |d| d["kind"] == "end-turn")?;

    try_accept(&mut s, |d| d["kind"] == "draw" && d["zone"] == "spellbook")?;
    try_accept(&mut s, |d| d["kind"] == "play-site" && d["cell"] == "C2")?;
    let (summon, _) = try_accept(&mut s, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "north-defender" && d["cell"] == "C2"
    })?;
    let target_id = id_of(&summon);
    try_accept(&mut s, |d| d["kind"] == "end-turn")?;

    try_accept(&mut s, |d| d["kind"] == "draw" && d["zone"] == "spellbook")?;
    try_accept(&mut s, |d| {
        d["kind"] == "cast-magic"
            && d["cardId"] == "south-grant"
            && d["ally"]["instanceId"] == attacker_id.as_str()
    })?;
    try_accept(&mut s, |d| {
        d["kind"] == "move-and-attack"
            && d["unitInstanceId"] == attacker_id.as_str()
            && d["path"]
                == json!([
                    { "cell": "C1", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    })?;
    try_accept(&mut s, |d| {
        d["kind"] == "declare-attack"
            && d["target"]["kind"] == "minion"
            && d["target"]["instanceId"] == target_id.as_str()
    })?;
    try_accept(&mut s, |d| {
        d["kind"] == "defend"
            && d["unitInstanceId"] == defender_id.as_str()
            && d["to"]["cell"] == "C2"
    })?;
    Some(s)
}

fn close_request(session: &Session) -> ActionRequest {
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| {
            a.descriptor["kind"] == "close-defend"
                && a.descriptor["originalTargetParticipates"] == true
        })
        .unwrap();
    ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    }
}

#[test]
fn unsupported_allocation_aborts_atomically_and_never_certifies() {
    let root = before_allocation(0, true).expect("fixed synthetic seed");
    let checkpoint = create_game_checkpoint(&root).unwrap();
    let request = close_request(&root);
    let before_state = root.state_hash().unwrap();
    let before_version = root.state_version();
    let before_view = root.public_view(Seat::South).unwrap();
    let before_attempts = root.attempts().to_vec();
    let before_transcript = root.transcript().to_vec();
    let mut branch = root.clone();
    assert!(is_unsupported(
        &branch.step(request.clone()),
        DOUBLE_UNSUPPORTED
    ));
    assert_eq!(branch.unsupported_mechanic(), Some(DOUBLE_UNSUPPORTED));
    assert_eq!(branch.state_hash().unwrap(), before_state);
    assert_eq!(branch.state_version(), before_version);
    assert_eq!(branch.public_view(Seat::South).unwrap(), before_view);
    assert_eq!(branch.attempts(), before_attempts);
    assert_eq!(branch.transcript(), before_transcript);
    assert!(is_unsupported(&branch.legal_actions(), DOUBLE_UNSUPPORTED));
    assert!(is_unsupported(
        &branch.clone().legal_actions(),
        DOUBLE_UNSUPPORTED
    ));
    assert!(is_unsupported(&branch.verify_replay(), DOUBLE_UNSUPPORTED));
    assert!(is_unsupported(&branch.replay_value(), DOUBLE_UNSUPPORTED));
    assert!(is_unsupported(&branch.session_hash(), DOUBLE_UNSUPPORTED));
    assert!(game_record_from_session(&branch).is_err());
    assert!(create_game_checkpoint(&branch).is_err());
    assert_eq!(root.unsupported_mechanic(), None);
    assert_eq!(create_game_checkpoint(&root).unwrap(), checkpoint);
    let resumed = resume_game_checkpoint(&checkpoint).unwrap();
    assert_eq!(
        resumed.session_hash().unwrap(),
        root.session_hash().unwrap()
    );

    let mut raw = Game::from_manifest_json(&lance_double_manifest(0, true)).unwrap();
    for receipt in root.transcript() {
        let action = raw
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|a| a.to_legal_action().unwrap().action_id == receipt.action_id)
            .unwrap();
        raw.apply_action(&action).unwrap();
    }
    let action = raw
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| a.to_legal_action().unwrap().action_id.as_str() == request.action_id)
        .unwrap();
    let position = raw.position().clone();
    assert!(matches!(
        raw.apply_action(&action),
        Err(GameError::UnsupportedMechanic(_))
    ));
    assert_eq!(raw.position(), &position);

    let mut service = SessionJsonService::new();
    assert!(rpc(&mut service, "resume", &json!({ "checkpoint": checkpoint }))["error"].is_null());
    let failed = rpc(&mut service, "step", &json!(request));
    assert!(
        failed["error"]["message"]
            .as_str()
            .unwrap()
            .contains(DOUBLE_UNSUPPORTED)
    );
    for seat in ["north", "south"] {
        assert_eq!(
            rpc(&mut service, "legalActions", &json!({"seat":seat}))["error"],
            failed["error"]
        );
    }
    for method in ["checkpoint", "exportSession", "verifyReplay", "replaySteps"] {
        assert_eq!(
            rpc(&mut service, method, &json!({}))["error"],
            failed["error"]
        );
    }
}

#[test]
fn supported_multi_combatant_allocation_remains_replayable() {
    let mut session = before_allocation(0, false).expect("fixed supported control");
    let request = close_request(&session);
    assert!(matches!(
        session.step(request).unwrap(),
        StepResult::Accepted(_)
    ));
    let actions = session.legal_actions().unwrap();
    assert!(!actions.is_empty());
    assert!(
        actions
            .iter()
            .all(|a| a.descriptor["kind"] == "allocate-strike")
    );
    while let Some(action) = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| a.descriptor["kind"] == "allocate-strike")
    {
        assert!(matches!(
            session
                .step(ActionRequest {
                    action_id: action.action_id.to_string(),
                    seat: action.seat,
                    state_version: action.state_version
                })
                .unwrap(),
            StepResult::Accepted(_)
        ));
    }
    assert_eq!(session.unsupported_mechanic(), None);
    assert!(session.verify_replay().unwrap());
    let checkpoint = create_game_checkpoint(&session).unwrap();
    assert_eq!(
        resume_game_checkpoint(&checkpoint)
            .unwrap()
            .session_hash()
            .unwrap(),
        session.session_hash().unwrap()
    );
}
