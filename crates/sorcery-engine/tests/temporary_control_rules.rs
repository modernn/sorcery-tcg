//! Direct proofs for temporary enemy-minion control (RULE-CATALOG-0513–0514).
//!
//! Official Magic can gain control of a target enemy minion this turn and
//! untap it. The transfer is not Nearby-restricted. Control reverts through
//! the shared End Phase cleanup after that controller's end-turn triggers.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn far() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn betrayal() -> Value {
    json!({
        "cardType": "magic",
        "gainControlOfTargetEnemyMinionThisTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "temporary-control" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-temporary-control-v1",
        },
        "cards": {
            "north-ally": dummy(),
            "north-avatar": avatar(),
            "north-betrayal": betrayal(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": far(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-betrayal",
                    "north-betrayal",
                    "north-betrayal",
                    "north-betrayal"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-far"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_spell_ids(session: &Session, seat: &str) -> Vec<String> {
    state(session)["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("spellbook hand")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
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
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn unit<'a>(after: &'a Value, instance_id: &str) -> &'a Value {
    after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("unit")
}

fn betrayal_targets(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("Betrayal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-betrayal"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn offers_activate_mana(session: &Session, instance_id: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "activate-mana"
                && action.descriptor["unitInstanceId"] == instance_id
        })
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
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

fn end_then_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

/// North dummy at C4, tapped South minion at C1, North ready to cast.
fn betrayal_opening() -> (Session, String, String) {
    let mut session = (1..=4096)
        .map(manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("temporary-control candidate");
            let north = opening_spell_ids(&session, "north");
            let south = opening_spell_ids(&session, "south");
            (north.iter().any(|card| card == "north-ally")
                && north.iter().any(|card| card == "north-betrayal")
                && south.iter().any(|card| card == "south-far"))
            .then_some(session)
        })
        .expect("bounded seed opening with Betrayal, an ally, and a far enemy");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (ally, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    });
    let ally_id = ally["cardInstanceId"]
        .as_str()
        .expect("north ally identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
    });
    let far_id = far["cardInstanceId"]
        .as_str()
        .expect("south far identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "atlas");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == far_id.as_str()
    });
    end_then_draw(&mut session, "spellbook");
    (session, ally_id, far_id)
}

#[test]
fn rule_catalog_0513_temporary_control_takes_a_distant_tapped_enemy_minion_this_turn() {
    let (mut session, ally_id, far_id) = betrayal_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &far_id)["controller"], "south");
    assert_eq!(unit(&before, &far_id)["owner"], "south");
    assert_eq!(unit(&before, &far_id)["tapped"], true);
    assert_eq!(unit(&before, &ally_id)["controller"], "north");

    let targets = betrayal_targets(&session);
    assert!(targets.contains(&far_id), "{targets:?}");
    assert!(!targets.contains(&ally_id), "{targets:?}");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-betrayal"
            && descriptor["target"]["instanceId"] == far_id.as_str()
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-control-changed",
            "minion-untapped",
            "magic-resolved"
        ]
    );
    let changed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-control-changed")
        .expect("control change");
    assert_eq!(changed.payload["fromSeat"], "south");
    assert_eq!(changed.payload["seat"], "north");

    let after = state(&session);
    assert_eq!(unit(&after, &far_id)["controller"], "north");
    assert_eq!(unit(&after, &far_id)["owner"], "south");
    assert_eq!(unit(&after, &far_id)["tapped"], false);
    assert!(offers_activate_mana(&session, &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0514_temporary_control_reverts_at_end_of_turn() {
    let (mut session, _, far_id) = betrayal_opening();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-betrayal"
            && descriptor["target"]["instanceId"] == far_id.as_str()
    });
    assert_eq!(unit(&state(&session), &far_id)["controller"], "north");

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == far_id
    }));
    let after = state(&session);
    assert_eq!(unit(&after, &far_id)["controller"], "south");
    assert_eq!(unit(&after, &far_id)["owner"], "south");
    assert_exact_replay(&session);
}
