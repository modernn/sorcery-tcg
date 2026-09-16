//! Direct proofs for 2×2 top/bottom wraparound occupancy (RULE-CATALOG-0373–0374).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
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

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn minion(extra: &Value) -> Value {
    let mut value = json!({
        "attack": 4,
        "cardType": "minion",
        "defense": 10,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("minion facts")
        .extend(extra.as_object().expect("extra minion facts").clone());
    value
}

fn wrapped_giant() -> Value {
    minion(&json!({
        "charge": true,
        "connectsTopBottom": true,
        "occupiesSquareArea": 2,
        "summonToAnySite": true,
    }))
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "square-wrap-footprint-rules" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-square-wrap-footprint-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-giant": wrapped_giant(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": minion(&json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 16],
                "avatar": "north-avatar",
                "spellbook": vec!["north-giant"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical manifest")
}

fn accept_where(
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
                "{label}: expected engine-issued action; available={:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
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
    accept_where(session, "keep-mulligan", |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, &format!("play-site-{cell}"), |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn end_and_draw_zone(session: &mut Session, zone: &str) {
    accept_where(session, "end-turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, &format!("draw-{zone}"), |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn end_and_draw(session: &mut Session) {
    end_and_draw_zone(session, "spellbook");
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay value")["state"].clone()
}

fn realm_unit<'a>(state: &'a Value, instance_id: &str) -> Option<&'a Value> {
    state["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn path_cells(descriptor: &Value) -> Vec<String> {
    descriptor["path"]
        .as_array()
        .expect("movement path")
        .iter()
        .map(|location| location["cell"].as_str().expect("path cell").to_owned())
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
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

fn play_first_domains(session: &mut Session) {
    keep(session);
    keep(session);
    play_site(session, "C4");
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);
}

fn establish_north_wrap_square(session: &mut Session) {
    play_first_domains(session);
    play_site(session, "B4");
    end_and_draw(session);
    play_site(session, "B1");
    end_and_draw(session);
}

fn establish_north_wrap_square_with_canonical_interior(session: &mut Session) {
    play_first_domains(session);
    play_site(session, "B4");
    end_and_draw(session);
    play_site(session, "B1");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "C2");
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "B3");
    end_and_draw(session);
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "B2");
    end_and_draw(session);
}

fn is_wrapped_summon_at(cell: &'static str) -> impl Fn(&Value) -> bool + use<> {
    move |descriptor: &Value| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == cell
            && descriptor["cells"] == json!(["B1", "B4", "C1", "C4"])
    }
}

#[test]
fn rule_catalog_0373_square_wrap_summons_onto_a_wrapped_footprint() {
    let mut session = Session::new(&manifest(401)).expect("valid wrapped 2x2 summon scenario");
    establish_north_wrap_square(&mut session);

    assert!(
        session
            .legal_actions()
            .expect("summon actions")
            .iter()
            .any(|action| is_wrapped_summon_at("B1")(&action.descriptor))
    );

    let (summoned, receipt) =
        accept_where(&mut session, "summon-wrapped", is_wrapped_summon_at("B1"));
    let giant = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();
    assert_eq!(receipt.events[0].event_type, "minion-summoned");

    let after = state(&session);
    let placed = realm_unit(&after, &giant).expect("wrapped occupant");
    assert_eq!(
        (&placed["location"], &placed["occupiedCells"]),
        (&json!("B1"), &json!(["B1", "B4", "C1", "C4"]))
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0374_square_wrap_steps_from_canonical_to_wrapped_footprint() {
    let mut session = Session::new(&manifest(402)).expect("valid wrapped 2x2 movement scenario");
    establish_north_wrap_square_with_canonical_interior(&mut session);

    let (summoned, _) = accept_where(&mut session, "summon-canonical", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B1"
            && descriptor["cells"] == json!(["B1", "B2", "C1", "C2"])
    });
    let giant = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned();

    let wrap_step = |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == giant.as_str()
            && path_cells(descriptor) == ["B1", "B4"]
    };
    assert!(
        session
            .legal_actions()
            .expect("movement actions")
            .iter()
            .any(|action| wrap_step(&action.descriptor))
    );

    accept_where(&mut session, "wrap-step", wrap_step);
    accept_where(&mut session, "decline-attack", |descriptor| {
        descriptor["kind"] == "decline-attack"
    });

    let after = state(&session);
    let stepped = realm_unit(&after, &giant).expect("wrapped walker");
    assert_eq!(
        (&stepped["location"], &stepped["occupiedCells"]),
        (&json!("B4"), &json!(["B4", "B1", "C4", "C1"]))
    );
    assert_exact_replay(&session);
}
