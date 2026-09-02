use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
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

fn site(water: bool) -> Value {
    json!({
        "cardType": "site",
        "elements": if water { ["water"] } else { ["earth"] },
    })
}

fn minion(burrowing: bool) -> Value {
    json!({
        "attack": 2,
        "burrowing": burrowing,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u64, north_water: bool) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-burrower": minion(true),
        "north-site": site(north_water),
        "south-avatar": avatar(),
        "south-plain": minion(false),
        "south-site": site(false),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "burrowing-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-burrowing-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": vec!["north-burrower"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-plain"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
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

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn path_locations(descriptor: &Value) -> Vec<String> {
    descriptor["path"]
        .as_array()
        .expect("movement path")
        .iter()
        .map(|location| {
            format!(
                "{}/{}",
                location["cell"].as_str().expect("path cell"),
                location["region"].as_str().expect("path region")
            )
        })
        .collect()
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
    let checkpoint = create_game_checkpoint(session).expect("Burrowing checkpoint");
    let restored = resume_game_checkpoint(
        &parse_game_checkpoint(
            &serialize_game_checkpoint(&checkpoint).expect("serialized Burrowing checkpoint"),
        )
        .expect("parsed Burrowing checkpoint"),
    )
    .expect("restored Burrowing checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

#[test]
fn rule_catalog_0114_burrowing_should_summon_and_move_underground_only_at_land_sites() {
    let mut session = Session::new(&manifest(131, false)).expect("valid Burrowing scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });

    let is_surface_summon = |descriptor: &Value| {
        descriptor["kind"] == "summon-minion" && descriptor["region"].is_null()
    };
    let is_burrowed_summon = |descriptor: &Value| {
        descriptor["kind"] == "summon-minion" && descriptor["region"] == "underground"
    };
    assert!(offers(&session, is_surface_summon));
    assert!(offers(&session, is_burrowed_summon));

    let (summoned, _) = accept_where(&mut session, is_burrowed_summon);
    let burrower_id = summoned["cardInstanceId"]
        .as_str()
        .expect("burrowed identity")
        .to_owned();
    let units = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .clone();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["instanceId"], burrower_id.as_str());
    assert_eq!(units[0]["region"], "underground");

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    assert!(!offers(&session, is_burrowed_summon));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let burrowed_step = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == burrower_id.as_str()
            && path_locations(descriptor) == ["C4/underground", "C3/underground"]
    };
    assert!(offers(&session, burrowed_step));
    assert!(offers(&session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == burrower_id.as_str()
            && path_locations(descriptor) == ["C4/underground", "C4/surface"]
    }));

    assert_checkpoint_round_trip(&session);
    accept_where(&mut session, burrowed_step);
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "site"
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let moved = state(&session)["realm"]["units"]
        .as_array()
        .expect("moved realm units")
        .clone();
    assert_eq!(moved[0]["location"], "C3");
    assert_eq!(moved[0]["region"], "underground");
    assert_exact_replay(&session);

    let mut water = Session::new(&manifest(132, true)).expect("valid water Burrowing scenario");
    keep(&mut water);
    keep(&mut water);
    accept_where(&mut water, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert!(offers(&water, is_surface_summon));
    assert!(!offers(&water, is_burrowed_summon));
    assert_exact_replay(&water);
}
