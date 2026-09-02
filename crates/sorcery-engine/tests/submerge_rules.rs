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

fn minion(submerge: bool) -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "submerge": submerge,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u64, water: bool) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-site": site(water),
        "north-swimmer": minion(true),
        "south-avatar": avatar(),
        "south-plain": minion(false),
        "south-site": site(water),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "submerge-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-submerge-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": vec!["north-swimmer"; 8],
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

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn unit_region(session: &Session, instance_id: &str) -> String {
    state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("tracked unit")["region"]
        .as_str()
        .expect("unit region")
        .to_owned()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn walks(descriptor: &Value, instance_id: &str, route: &[&str]) -> bool {
    descriptor["kind"] == "move-and-attack"
        && descriptor["unitInstanceId"] == instance_id
        && descriptor["path"].as_array().is_some_and(|path| {
            path.iter()
                .map(|location| {
                    format!(
                        "{}/{}",
                        location["cell"].as_str().expect("path cell"),
                        location["region"].as_str().expect("path region")
                    )
                })
                .eq(route.iter().map(|step| (*step).to_owned()))
        })
}

fn walk(session: &mut Session, instance_id: &str, route: &[&str]) {
    accept_where(session, |descriptor| walks(descriptor, instance_id, route));
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
    let checkpoint = create_game_checkpoint(session).expect("Submerge checkpoint");
    let restored = resume_game_checkpoint(
        &parse_game_checkpoint(
            &serialize_game_checkpoint(&checkpoint).expect("serialized Submerge checkpoint"),
        )
        .expect("parsed Submerge checkpoint"),
    )
    .expect("restored Submerge checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn is_surface_summon(descriptor: &Value) -> bool {
    descriptor["kind"] == "summon-minion" && descriptor["region"].is_null()
}

fn is_underwater_summon(descriptor: &Value) -> bool {
    descriptor["kind"] == "summon-minion" && descriptor["region"] == "underwater"
}

#[test]
fn rule_catalog_0113_submerge_should_summon_move_and_fight_inside_the_water_region() {
    let mut session = Session::new(&manifest(129, true)).expect("valid Submerge scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    assert!(offers(&session, is_surface_summon));
    assert!(offers(&session, is_underwater_summon));

    let (summoned, _) = accept_where(&mut session, is_underwater_summon);
    let swimmer_id = summoned["cardInstanceId"]
        .as_str()
        .expect("submerged identity")
        .to_owned();
    assert_eq!(unit_region(&session, &swimmer_id), "underwater");
    end_turn(&mut session);

    draw_spell(&mut session);
    play_site(&mut session, "C1");
    assert!(!offers(&session, is_underwater_summon));
    let (enemy, _) = accept_where(&mut session, is_surface_summon);
    let enemy_id = enemy["cardInstanceId"]
        .as_str()
        .expect("surface enemy identity")
        .to_owned();
    end_turn(&mut session);

    draw_spell(&mut session);
    play_site(&mut session, "C3");
    assert!(offers(&session, |descriptor| walks(
        descriptor,
        &swimmer_id,
        &["C4/underwater", "C4/surface"]
    )));
    assert!(offers(&session, |descriptor| walks(
        descriptor,
        &swimmer_id,
        &["C4/underwater", "C3/underwater"]
    )));
    assert_checkpoint_round_trip(&session);

    walk(
        &mut session,
        &swimmer_id,
        &["C4/underwater", "C3/underwater"],
    );
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "site"
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    end_turn(&mut session);

    draw_spell(&mut session);
    play_site(&mut session, "C2");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == enemy_id.as_str()
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    end_turn(&mut session);

    let attacks_enemy = |descriptor: &Value| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == enemy_id.as_str()
    };
    draw_spell(&mut session);
    walk(
        &mut session,
        &swimmer_id,
        &["C3/underwater", "C2/underwater"],
    );
    assert!(!offers(&session, attacks_enemy));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    end_turn(&mut session);

    draw_spell(&mut session);
    end_turn(&mut session);

    draw_spell(&mut session);
    walk(&mut session, &swimmer_id, &["C2/underwater", "C2/surface"]);
    assert_eq!(unit_region(&session, &swimmer_id), "surface");
    assert!(offers(&session, attacks_enemy));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_exact_replay(&session);

    let mut land = Session::new(&manifest(130, false)).expect("valid land Submerge scenario");
    keep(&mut land);
    keep(&mut land);
    play_site(&mut land, "C4");
    assert!(offers(&land, is_surface_summon));
    assert!(!offers(&land, is_underwater_summon));
    assert_exact_replay(&land);
}
