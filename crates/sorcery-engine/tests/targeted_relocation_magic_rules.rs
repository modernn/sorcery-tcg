//! Direct proofs for targeted relocation Magic (RULE-CATALOG-0039 Teleport).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, LegalAction, Receipt};
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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn teleport() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "teleportAllyToTargetSite": true,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "targeted-relocation-magic-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-targeted-relocation-magic-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-minion": minion(),
            "north-site": site(),
            "north-teleport": teleport(),
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-teleport",
                    "north-minion",
                    "north-teleport",
                    "north-minion",
                    "north-teleport",
                    "north-minion",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["north-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
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

fn end_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
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

fn teleport_actions(session: &Session, card_instance_id: &str) -> Vec<LegalAction> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == card_instance_id
        })
        .collect()
}

fn first_teleport_in_hand(current: &Value) -> String {
    current["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-teleport")
        .expect("Teleport in hand")["instanceId"]
        .as_str()
        .expect("Teleport identity")
        .to_owned()
}

fn site_instance_id(current: &Value, cell: &str) -> String {
    current["realm"]["sites"][cell]["instanceId"]
        .as_str()
        .expect("site identity")
        .to_owned()
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> &'a Value {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("realm unit")
}

/// Builds a North main phase with three surface sites, one North minion, and Teleport in hand.
fn opening_with_three_sites() -> Session {
    let manifest = (1..=512)
        .map(manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Teleport candidate");
            let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand")
                .clone();
            hand.iter()
                .filter(|card| card["cardId"] == "north-teleport")
                .count()
                >= 2
                && hand.iter().any(|card| card["cardId"] == "north-minion")
        })
        .expect("bounded seed with two Teleports and a minion in the North opening hand");
    let mut session = Session::new(&manifest).expect("valid Teleport scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
    });
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    session
}

#[test]
fn rule_catalog_0039_teleport_should_move_ally_to_any_target_site_surface() {
    let mut session = opening_with_three_sites();
    let current = state(&session);
    let units = current["realm"]["units"].as_array().expect("realm units");
    assert_eq!(units.len(), 1);
    let ally_instance_id = units[0]["instanceId"]
        .as_str()
        .expect("minion identity")
        .to_owned();
    let avatar_instance_id = current["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar identity")
        .to_owned();

    let card_instance_id = first_teleport_in_hand(&current);
    let actions = teleport_actions(&session, &card_instance_id);
    assert_eq!(actions.len(), 6);
    for action in &actions {
        let descriptor = &action.descriptor;
        assert!(descriptor.get("target").is_none());
        assert_eq!(descriptor["targetLocation"]["region"], "surface");
        let cell = descriptor["targetLocation"]["cell"]
            .as_str()
            .expect("target cell");
        assert_eq!(
            descriptor["targetSiteInstanceId"],
            site_instance_id(&current, cell).as_str()
        );
    }
    assert_eq!(
        actions
            .iter()
            .map(|action| (
                action.descriptor["ally"]["instanceId"]
                    .as_str()
                    .expect("ally identity")
                    .to_owned(),
                action.descriptor["targetLocation"]["cell"]
                    .as_str()
                    .expect("target cell")
                    .to_owned(),
            ))
            .collect::<std::collections::BTreeSet<_>>(),
        ["C1", "C3", "C4"]
            .into_iter()
            .flat_map(|cell| [
                (ally_instance_id.clone(), cell.to_owned()),
                (avatar_instance_id.clone(), cell.to_owned()),
            ])
            .collect::<std::collections::BTreeSet<_>>()
    );
    let labelled = actions
        .iter()
        .find(|action| {
            action.descriptor["ally"]["instanceId"] == ally_instance_id.as_str()
                && action.descriptor["targetLocation"]["cell"] == "C1"
        })
        .expect("minion Teleport to the enemy site");
    assert_eq!(
        labelled.label,
        format!(
            "Cast north-teleport to teleport minion {}… to C1",
            &ally_instance_id[..15]
        )
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == card_instance_id.as_str()
            && descriptor["ally"]["instanceId"] == ally_instance_id.as_str()
            && descriptor["targetLocation"]["cell"] == "C1"
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "unit-teleported", "magic-resolved"]
    );
    assert_eq!(
        receipt.events[1].payload["from"],
        json!({ "cell": "C4", "region": "surface" })
    );
    assert_eq!(
        receipt.events[1].payload["to"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert_eq!(
        receipt.events[1].payload["targetSiteInstanceId"],
        site_instance_id(&current, "C1").as_str()
    );
    let teleported = state(&session);
    let unit = realm_unit(&teleported, &ally_instance_id);
    assert_eq!(unit["location"], "C1");
    assert_eq!(unit["region"], "surface");
    assert!(receipt.random_draws.is_empty());
    assert_exact_replay(&session);
}

#[test]
fn teleport_to_the_site_an_ally_already_occupies_should_resolve_without_moving_it() {
    let mut session = opening_with_three_sites();
    let current = state(&session);
    let avatar_instance_id = current["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar identity")
        .to_owned();
    let avatar_cell = current["players"]["north"]["avatar"]["location"]
        .as_str()
        .expect("avatar cell")
        .to_owned();

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["ally"]["instanceId"] == avatar_instance_id.as_str()
            && descriptor["targetLocation"]["cell"] == avatar_cell.as_str()
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["location"],
        avatar_cell.as_str()
    );
    assert_exact_replay(&session);
}
