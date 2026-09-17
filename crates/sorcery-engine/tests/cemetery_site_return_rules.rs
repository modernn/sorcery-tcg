//! Direct proofs for cemetery Site return Magic (RULE-CATALOG-0639–0640).
//!
//! Cemetery Site return offers only Sites in the caster's own cemetery and
//! restores the chosen instance to the hidden Atlas hand. An empty own
//! cemetery is a paid no-choice resolution.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
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

fn destroy_spell() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn return_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetSiteFromOwnCemetery": true,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn cemetery_site_manifest(seed: u32, include_destroy: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-return": return_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-minion": minion(),
        "south-site": site(),
    });
    let north_spellbook = if include_destroy {
        cards["north-destroy"] = destroy_spell();
        json!([
            "north-destroy",
            "north-return",
            "north-destroy",
            "north-return",
            "north-destroy",
            "north-return"
        ])
    } else {
        json!(vec!["north-return"; 6])
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-site-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-site-return-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid cemetery-site session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
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

fn cemetery_site_cast_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("cemetery site actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return"
        })
        .filter_map(|action| {
            action.descriptor["cemeteryMinionInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn north_hand_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn seed_with(required: &[&str], include_destroy: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| cemetery_site_manifest(seed, include_destroy))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                let snapshot = state(&preview);
                let hand = north_hand_ids(&snapshot);
                required.iter().all(|id| hand.iter().any(|card| card == id))
            })
        })
        .expect("bounded seed with required opening cards")
}

fn setup_own_cemetery_site(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B4"
    });
    let site_id = state(session)["realm"]["sites"]["B4"]["instanceId"]
        .as_str()
        .expect("B4 site identity")
        .to_owned();
    let (_, destroyed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "B4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert!(
        destroyed
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed")
    );
    let destroy_id = state(session)["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("destroy Magic in cemetery")["instanceId"]
        .as_str()
        .expect("destroy identity")
        .to_owned();
    (site_id, destroy_id)
}

#[test]
fn rule_catalog_0639_cemetery_site_return_restores_own_cemetery_site_to_hidden_atlas() {
    let encoded = seed_with(&["north-destroy", "north-return"], true, 639);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_site_cast_ids(&session), Vec::<String>::new());
    let (site_id, destroy_id) = setup_own_cemetery_site(&mut session);
    assert_ne!(destroy_id, site_id);
    let cemetery_targets = cemetery_site_cast_ids(&session);
    assert!(!cemetery_targets.is_empty());
    assert!(cemetery_targets.iter().all(|id| id == &site_id));
    assert!(!cemetery_targets.iter().any(|id| id == &destroy_id));
    let before = state(&session);
    let before_atlas = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let south_observation = session.observe(Seat::South);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["cardId"], "north-site");
    assert_eq!(receipt.events[1].payload["instanceId"], site_id);
    assert_eq!(receipt.events[1].payload["owner"], "north");
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(receipt.events[1].payload.get("cell").is_none());
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-returned-to-hand"
                || event.event_type == "magic-returned-to-hand"
                || event.event_type == "artifact-returned-to-hand")
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        before_atlas + 1
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .iter()
            .any(|card| card["instanceId"] == site_id)
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .iter()
            .all(|card| card["instanceId"] != site_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != site_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == destroy_id)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_view["players"]["north"]["hand"]["atlas"],
        before_atlas + 1
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("cemetery-site checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized cemetery-site");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed cemetery-site");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed cemetery-site session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0640_cemetery_site_return_is_a_paid_noop_without_cemetery_site() {
    let encoded = seed_with(&["north-return"], false, 640);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_site_cast_ids(&session), Vec::<String>::new());
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-returned-to-hand"
                || event.event_type == "magic-returned-to-hand")
    );
    assert_eq!(cemetery_site_cast_ids(&session), Vec::<String>::new());
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        1
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}
