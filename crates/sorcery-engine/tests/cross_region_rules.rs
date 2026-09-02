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

fn crosser() -> Value {
    json!({
        "attack": 2,
        "burrowing": true,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "submerge": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn plain() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u64) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-crosser": crosser(),
        "north-land": site(false),
        "north-water": site(true),
        "south-avatar": avatar(),
        "south-plain": plain(),
        "south-site": site(false),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cross-region-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cross-region-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": [
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                ],
                "avatar": "north-avatar",
                "spellbook": vec!["north-crosser"; 8],
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

fn draw_spell(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn play_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn unit_location(session: &Session, instance_id: &str) -> (String, String) {
    let units = state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .clone();
    let unit = units
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("tracked unit");
    (
        unit["location"].as_str().expect("unit cell").to_owned(),
        unit["region"].as_str().expect("unit region").to_owned(),
    )
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
    let checkpoint = create_game_checkpoint(session).expect("cross-region checkpoint");
    let restored = resume_game_checkpoint(
        &parse_game_checkpoint(
            &serialize_game_checkpoint(&checkpoint).expect("serialized cross-region checkpoint"),
        )
        .expect("parsed cross-region checkpoint"),
    )
    .expect("restored cross-region checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn step_offer<'a>(unit_id: &'a str, path: [&'static str; 2]) -> impl Fn(&Value) -> bool + use<'a> {
    move |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == unit_id
            && path_locations(descriptor) == path
    }
}

#[test]
fn rule_catalog_0118_combined_region_abilities_should_permit_cross_region_adjacency_steps() {
    let mut session = Session::new(&manifest(135)).expect("valid cross-region scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "north-land", "C4");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"] == "underground"
    });
    let crosser_id = summoned["cardInstanceId"]
        .as_str()
        .expect("crosser identity")
        .to_owned();
    end_turn(&mut session);

    draw_spell(&mut session);
    play_site(&mut session, "south-site", "C1");
    end_turn(&mut session);

    draw_spell(&mut session);
    play_site(&mut session, "north-water", "C3");
    let dive = step_offer(&crosser_id, ["C4/underground", "C3/underwater"]);
    assert!(offers(&session, &dive));
    assert_checkpoint_round_trip(&session);
    accept_where(&mut session, &dive);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(
        unit_location(&session, &crosser_id),
        ("C3".to_owned(), "underwater".to_owned())
    );
    end_turn(&mut session);

    draw_spell(&mut session);
    end_turn(&mut session);

    draw_spell(&mut session);
    let burrow_back = step_offer(&crosser_id, ["C3/underwater", "C4/underground"]);
    assert!(offers(&session, &burrow_back));
    accept_where(&mut session, &burrow_back);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(
        unit_location(&session, &crosser_id),
        ("C4".to_owned(), "underground".to_owned())
    );
    assert_exact_replay(&session);
}
