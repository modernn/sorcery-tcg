//! Direct proofs for destroy/return target Aura Magic (RULE-CATALOG-0270–0271).
//!
//! Official Magic can destroy a realm Aura or return it to its owner's
//! Spellbook hand. Destroy is not a duration dispel: the Aura leaves through
//! `aura-destroyed` or `aura-returned-to-hand`, matching immobile areas lift,
//! and Flood/Drought water overlays settle off the covered sites.

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn flood() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn entangle() -> Value {
    json!({
        "cardType": "aura",
        "immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn magic(effect: &str) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("Magic facts")
        .insert(effect.to_owned(), json!(true));
    value
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(north_aura: &str, south_effect: &str) -> String {
    let north_card = match north_aura {
        "north-flood" => flood(),
        "north-entangle" => entangle(),
        _ => panic!("unsupported north Aura {north_aura}"),
    };
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-aura-magic": magic(south_effect),
        "south-site": site(),
    });
    cards[north_aura] = north_card;
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "aura-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-aura-magic-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec![north_aura; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-aura-magic"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
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

fn north_affinity(session: &Session) -> (u64, u64) {
    let view = session.public_view(Seat::North).expect("North public view");
    (
        view["players"]["north"]["affinity"]["earth"]
            .as_u64()
            .expect("earth affinity"),
        view["players"]["north"]["affinity"]["water"]
            .as_u64()
            .expect("water affinity"),
    )
}

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

fn aura_magic_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<String> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter_map(|action| {
            (action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "south-aura-magic")
                .then(|| {
                    action.descriptor["targetAuraInstanceId"]
                        .as_str()
                        .expect("Aura Magic names its target")
                        .to_owned()
                })
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn after_north_aura(north_aura: &str, south_effect: &str) -> (Session, String) {
    let mut session = Session::new(&manifest(north_aura, south_effect)).expect("Aura Magic");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert!(
        aura_magic_targets(&session).is_empty(),
        "South cannot cast Aura Magic on North's turn"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == north_aura
            && cells_include(descriptor, "C4")
    });
    let aura_id = state(&session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("conjured Aura identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    (session, aura_id)
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

#[test]
fn rule_catalog_0270_destroy_target_aura_sends_flood_to_its_owners_cemetery() {
    let (mut session, aura_id) = after_north_aura("north-flood", "destroyTargetAura");
    assert_eq!(north_affinity(&session), (1, 1));
    assert_eq!(
        aura_magic_targets(&session).as_slice(),
        std::slice::from_ref(&aura_id)
    );
    let before = state(&session);
    assert_eq!(before["realm"]["auras"][0]["cardId"], "north-flood");

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-aura-magic"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-destroyed", "magic-resolved"]
    );
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "aura-destroyed")
        .expect("Aura destruction");
    assert_eq!(destroyed.payload["cardId"], "north-flood");
    assert_eq!(destroyed.payload["instanceId"], aura_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-dispelled"
                || event.event_type == "aura-returned-to-hand"
                || event.event_type == "aura-banished")
    );

    let after = state(&session);
    assert!(after["realm"].get("auras").is_none());
    assert!(after["realm"].get("immobileAreas").is_none());
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert!(
        !after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert_eq!(north_affinity(&session), (1, 0));
    assert_eq!(aura_magic_targets(&session), Vec::<String>::new());
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("destroy-aura checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized destroy-aura");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed destroy-aura");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed destroy-aura session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0271_return_target_aura_returns_flood_to_its_owners_hand() {
    let (mut session, aura_id) = after_north_aura("north-flood", "returnTargetAuraToOwnerHand");
    let before = state(&session);
    let north_hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook hand")
        .len();
    assert_eq!(north_affinity(&session), (1, 1));
    assert_eq!(
        aura_magic_targets(&session).as_slice(),
        std::slice::from_ref(&aura_id)
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-aura-magic"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    let returned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "aura-returned-to-hand")
        .expect("Aura return");
    assert_eq!(returned.payload["cardId"], "north-flood");
    assert_eq!(returned.payload["instanceId"], aura_id);
    assert_eq!(returned.payload["owner"], "north");
    assert_eq!(returned.payload["seat"], "north");
    assert_eq!(
        returned.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-destroyed"
                || event.event_type == "aura-dispelled"
                || event.event_type == "aura-banished")
    );

    let after = state(&session);
    assert!(after["realm"].get("auras").is_none());
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .len(),
        north_hand_before + 1
    );
    assert!(
        !after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_view["players"]["north"]["hand"]["spellbook"],
        json!(north_hand_before + 1)
    );
    assert_eq!(north_affinity(&session), (1, 0));
    assert_eq!(aura_magic_targets(&session), Vec::<String>::new());
    assert_exact_replay(&session);
}

#[test]
fn destroy_target_aura_lifts_matching_immobile_area() {
    let (mut session, aura_id) = after_north_aura("north-entangle", "destroyTargetAura");
    let before = state(&session);
    assert_eq!(before["realm"]["auras"][0]["cardId"], "north-entangle");
    assert_eq!(
        before["realm"]["immobileAreas"][0]["sourceInstanceId"],
        aura_id
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-aura-magic"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-destroyed", "magic-resolved"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-dispelled")
    );
    let after = state(&session);
    assert!(after["realm"].get("auras").is_none());
    assert!(after["realm"].get("immobileAreas").is_none());
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert_eq!(aura_magic_targets(&session), Vec::<String>::new());
    assert_exact_replay(&session);
}
