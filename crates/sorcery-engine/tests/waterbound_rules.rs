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

fn site(water: bool) -> Value {
    json!({
        "cardType": "site",
        "elements": if water { ["water"] } else { ["earth"] },
    })
}

fn waterbound(lower: &str) -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        lower: true,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "waterbound": true,
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

fn manifest(seed: u64, lower: &str) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-land": site(false),
        "north-water": site(true),
        "north-waterbound": waterbound(lower),
        "south-avatar": avatar(),
        "south-plain": plain(),
        "south-site": site(false),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "waterbound-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-waterbound-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": vec!["north-waterbound"; 8],
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

fn south_turn(session: &mut Session, cell: Option<&str>) {
    draw_spell(session);
    if let Some(cell) = cell {
        play_site(session, "south-site", cell);
    }
    end_turn(session);
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn units(session: &Session) -> Vec<Value> {
    state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .clone()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn event_types(receipt: &Receipt) -> Vec<String> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.clone())
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

#[test]
fn rule_catalog_0049_waterbound_should_derive_disabled_from_terrain_and_die_without_abilities() {
    let mut session = Session::new(&manifest(136, "submerge")).expect("valid Waterbound scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "north-water", "C4");
    assert!(offers(&session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"] == "underwater"
    }));
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"].is_null()
    });
    let bound_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Waterbound identity")
        .to_owned();
    end_turn(&mut session);
    south_turn(&mut session, Some("C1"));

    draw_spell(&mut session);
    play_site(&mut session, "north-land", "C3");
    let activates_mana = |descriptor: &Value| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == bound_id.as_str()
    };
    assert!(offers(&session, activates_mana));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bound_id.as_str()
            && descriptor["to"]["cell"] == "C3"
            && descriptor["to"]["region"] == "surface"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert!(!offers(&session, activates_mana));
    end_turn(&mut session);
    south_turn(&mut session, None);

    draw_spell(&mut session);
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "move-and-attack" && descriptor["unitInstanceId"] == bound_id.as_str()
    }));
    assert!(!offers(&session, activates_mana));
    assert_eq!(units(&session).len(), 1);
    assert_exact_replay(&session);

    let mut land = Session::new(&manifest(137, "burrowing")).expect("valid land Waterbound");
    keep(&mut land);
    keep(&mut land);
    play_site(&mut land, "north-land", "C4");
    let (_, receipt) = accept_where(&mut land, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"] == "underground"
    });
    assert_eq!(event_types(&receipt), ["minion-summoned", "minion-died"]);
    assert!(units(&land).is_empty());
    assert_exact_replay(&land);
}
