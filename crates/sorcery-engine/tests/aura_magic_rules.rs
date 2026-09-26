//! Direct proofs for destroy/return target Aura Magic (RULE-CATALOG-0270–0273,
//! RULE-CATALOG-0784), Flood `cast-aura` withheld during trigger-order
//! (RULE-CATALOG-1148), Drought `cast-aura` withheld during trigger-order
//! (RULE-CATALOG-1330), Flood `cast-aura` withheld during trigger-order on an
//! occupied Earth site (RULE-CATALOG-1348), and Drought `cast-aura` withheld during
//! trigger-order on an occupied Water site (RULE-CATALOG-1353), Drought
//! `cast-aura` withheld during trigger-order on an occupied Earth site
//! (RULE-CATALOG-1423), Flood `cast-aura` withheld during trigger-order
//! on an occupied Water site (RULE-CATALOG-1424), Flood `cast-aura` withheld
//! during trigger-order on a flooded occupied Water site at C3
//! (RULE-CATALOG-1457), Drought `cast-aura` withheld during trigger-order on a
//! drought occupied Earth site at C3 (RULE-CATALOG-1458), Flood `cast-aura`
//! withheld during trigger-order on an occupied Earth site at C3
//! (RULE-CATALOG-1463), and Drought `cast-aura` withheld during trigger-order
//! on an occupied Water site at C3 (RULE-CATALOG-1464).
//!
//! Official Magic can destroy a realm Aura or return it to its owner's
//! Spellbook hand. Destroy is not a duration dispel: the Aura leaves through
//! `aura-destroyed` or `aura-returned-to-hand`, matching immobile areas lift,
//! and Flood/Drought water overlays settle off the covered sites. While
//! Deathrites wait for ordering, Flood stays in hand and `cast-aura` is not
//! offered until the pending chain drains.

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

fn drought() -> Value {
    json!({
        "affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold": true,
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
fn rule_catalog_0784_destroy_target_aura_lifts_matching_immobile_area() {
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

fn filler_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn cemetery_aura_manifest(include_setup: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-return": magic("returnTargetAuraFromOwnCemetery"),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-minion": filler_minion(),
        "south-site": site(),
    });
    if include_setup {
        cards["north-destroy"] = magic("destroyTargetAura");
        cards["north-flood"] = flood();
    }
    let north_spellbook = if include_setup {
        vec!["north-flood", "north-destroy", "north-return"]
    } else {
        vec!["north-return"; 3]
    };
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
                "atlas": vec!["north-site"; 3],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 3],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 3],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    }))
}

fn cemetery_aura_cast_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("cemetery Aura actions")
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("cemetery Aura Magic");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the direct proof keeps mixed cemetery filtering, hidden-hand, and replay together"
)]
fn rule_catalog_0272_cemetery_aura_return_restores_own_cemetery_aura_to_hidden_hand() {
    let mut session = opening_main(&cemetery_aura_manifest(true));
    assert_eq!(cemetery_aura_cast_ids(&session), Vec::<String>::new());
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C4")
    });
    let aura_id = state(&session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("Flood identity")
        .to_owned();
    let (_, destroyed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        destroyed
            .events
            .iter()
            .any(|event| event.event_type == "aura-destroyed")
    );
    let before = state(&session);
    let destroy_id = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("North cemetery")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("destroy Magic in cemetery")["instanceId"]
        .as_str()
        .expect("destroy identity")
        .to_owned();
    assert_ne!(destroy_id, aura_id);
    let cemetery_targets = cemetery_aura_cast_ids(&session);
    assert!(!cemetery_targets.is_empty());
    assert!(cemetery_targets.iter().all(|id| id == &aura_id));
    assert!(!cemetery_targets.iter().any(|id| id == &destroy_id));
    let before_hand_count = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook hand")
        .len();
    let south_observation = session.observe(Seat::South);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["cardId"], "north-flood");
    assert_eq!(receipt.events[1].payload["instanceId"], aura_id);
    assert_eq!(receipt.events[1].payload["owner"], "north");
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "magic-returned-to-hand"
                || event.event_type == "minion-returned-to-hand"
                || event.event_type == "aura-destroyed"
                || event.event_type == "aura-dispelled")
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .len(),
        before_hand_count
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == destroy_id)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_view["players"]["north"]["hand"]["spellbook"], 1);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("cemetery-aura checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized cemetery-aura");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed cemetery-aura");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed cemetery-aura session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0273_cemetery_aura_return_is_a_paid_noop_without_cemetery_aura() {
    let mut session = opening_main(&cemetery_aura_manifest(false));
    assert_eq!(cemetery_aura_cast_ids(&session), Vec::<String>::new());
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-returned-to-hand"
                || event.event_type == "magic-returned-to-hand")
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .len(),
        1
    );
    assert_eq!(cemetery_aura_cast_ids(&session), Vec::<String>::new());
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

fn try_accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
}

fn rain() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_cast_aura_manifest(seed: u32) -> String {
    let fixture = "cast-aura-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-flood": flood(),
            "north-rain": rain(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-rain",
                    "north-flood",
                    "north-rain",
                    "north-flood",
                ],
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

fn north_has_flood_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-flood", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn offers_flood_cast(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-aura"
                && action.descriptor["cardId"] == "north-flood"
                && cells_include(&action.descriptor, "C4")
        })
    })
}

struct PendingDeathriteCastAuraSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_cast_aura(encoded: &str) -> Option<PendingDeathriteCastAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_flood_and_rain(&state(&session)) {
        return None;
    }
    if !offers_flood_cast(&session) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCastAuraSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_cast_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cast_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cast_aura(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Flood cast-aura in hand")
}

#[test]
fn rule_catalog_1148_cast_aura_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cast_aura_seed_with(1148);
    let mut setup = try_pending_deathrite_with_cast_aura(&encoded)
        .expect("complete Flood cast-aura Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-flood"))
    );
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_flood_cast(session));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_flood_cast(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C4")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    let after = state(session);
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(north_affinity(session), (1, 1));
    assert_exact_replay(session);
}

fn deathrite_cast_drought_aura_manifest(seed: u32) -> String {
    let fixture = "cast-drought-aura-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-rain": rain(),
            "north-site": site(),
            "north-water": json!({ "cardType": "site", "elements": ["water"] }),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": ["north-water", "north-site", "north-site", "north-site", "north-site", "north-site"],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-rain",
                    "north-drought",
                    "north-rain",
                    "north-drought",
                ],
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

fn deathrite_cast_drought_occupied_aura_manifest(seed: u32) -> String {
    let fixture = "cast-drought-aura-occupied-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-rain": rain(),
            "north-water": json!({ "cardType": "site", "elements": ["water"] }),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-rain",
                    "north-drought",
                    "north-rain",
                    "north-drought",
                ],
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

fn north_has_drought_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-drought", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn offers_drought_cast(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-aura"
                && action.descriptor["cardId"] == "north-drought"
                && cells_include(&action.descriptor, "C4")
        })
    })
}

fn try_pending_deathrite_with_cast_drought_aura(
    encoded: &str,
) -> Option<PendingDeathriteCastAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_drought_and_rain(&state(&session)) {
        return None;
    }
    if !offers_drought_cast(&session) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCastAuraSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_cast_drought_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cast_drought_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cast_drought_aura(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Drought cast-aura in hand")
}

#[test]
fn rule_catalog_1330_cast_drought_aura_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cast_drought_aura_seed_with(1330);
    let mut setup = try_pending_deathrite_with_cast_drought_aura(&encoded)
        .expect("complete Drought cast-aura Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-drought"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_drought_cast(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_drought_cast(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && cells_include(descriptor, "C4")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(
        state(session)["realm"]["auras"][0]["cardId"],
        "north-drought"
    );
    assert_exact_replay(session);
}

fn offers_flood_cast_c1(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-aura"
                && action.descriptor["cardId"] == "north-flood"
                && cells_include(&action.descriptor, "C1")
        })
    })
}

fn try_pending_deathrite_with_cast_flood_occupied(
    encoded: &str,
) -> Option<PendingDeathriteCastAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_flood_and_rain(&state(&session)) {
        return None;
    }
    if !offers_flood_cast_c1(&session) {
        return None;
    }
    if state(&session)["realm"]["sites"]["C1"]["rubble"] == true {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCastAuraSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_cast_flood_occupied_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cast_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cast_flood_occupied(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with Flood cast-aura on occupied Earth site",
        )
}

#[test]
fn rule_catalog_1348_cast_flood_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site()
 {
    let encoded = deathrite_cast_flood_occupied_aura_seed_with(1348);
    let mut setup = try_pending_deathrite_with_cast_flood_occupied(&encoded)
        .expect("complete Flood cast-aura on occupied Earth site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-flood"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_flood_cast_c1(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_flood_cast_c1(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C1")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(state(session)["realm"]["auras"][0]["cardId"], "north-flood");
    assert_exact_replay(session);
}

fn offers_drought_cast_c1(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-aura"
                && action.descriptor["cardId"] == "north-drought"
                && cells_include(&action.descriptor, "C1")
        })
    })
}

fn try_pending_deathrite_with_cast_drought_occupied(
    encoded: &str,
) -> Option<PendingDeathriteCastAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_drought_and_rain(&state(&session)) {
        return None;
    }
    if !offers_drought_cast_c1(&session) {
        return None;
    }
    if state(&session)["realm"]["sites"]["C1"]["rubble"] == true {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCastAuraSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_cast_drought_occupied_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cast_drought_occupied_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cast_drought_occupied(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with Drought cast-aura on occupied Water site",
        )
}

#[test]
fn rule_catalog_1353_cast_drought_aura_withheld_during_pending_deathrite_order_on_occupied_water_site()
 {
    let encoded = deathrite_cast_drought_occupied_aura_seed_with(1353);
    let mut setup = try_pending_deathrite_with_cast_drought_occupied(&encoded)
        .expect("complete Drought cast-aura on occupied Water site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-drought"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_drought_cast_c1(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_drought_cast_c1(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && cells_include(descriptor, "C1")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(
        state(session)["realm"]["auras"][0]["cardId"],
        "north-drought"
    );
    assert_exact_replay(session);
}

fn deathrite_cast_drought_earth_occupied_aura_manifest(seed: u32) -> String {
    let fixture = "cast-drought-aura-earth-occupied-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-earth": json!({ "cardType": "site", "elements": ["earth"] }),
            "north-rain": rain(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-rain",
                    "north-drought",
                    "north-rain",
                    "north-drought",
                ],
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

fn deathrite_cast_flood_water_occupied_aura_manifest(seed: u32) -> String {
    let fixture = "cast-flood-aura-water-occupied-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-flood": flood(),
            "north-rain": rain(),
            "north-water": json!({ "cardType": "site", "elements": ["water"] }),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-rain",
                    "north-flood",
                    "north-rain",
                    "north-flood",
                ],
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

fn deathrite_cast_drought_earth_occupied_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cast_drought_earth_occupied_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cast_drought_occupied(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with Drought cast-aura on occupied Earth site",
        )
}

fn deathrite_cast_flood_water_occupied_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cast_flood_water_occupied_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cast_flood_occupied(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with Flood cast-aura on occupied Water site",
        )
}

#[test]
fn rule_catalog_1423_cast_drought_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site()
 {
    let encoded = deathrite_cast_drought_earth_occupied_aura_seed_with(1423);
    let mut setup = try_pending_deathrite_with_cast_drought_occupied(&encoded)
        .expect("complete Drought cast-aura on occupied Earth site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-drought"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_drought_cast_c1(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_drought_cast_c1(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && cells_include(descriptor, "C1")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(
        state(session)["realm"]["auras"][0]["cardId"],
        "north-drought"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1424_cast_flood_aura_withheld_during_pending_deathrite_order_on_occupied_water_site()
 {
    let encoded = deathrite_cast_flood_water_occupied_aura_seed_with(1424);
    let mut setup = try_pending_deathrite_with_cast_flood_occupied(&encoded)
        .expect("complete Flood cast-aura on occupied Water site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-flood"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_flood_cast_c1(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_flood_cast_c1(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C1")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(state(session)["realm"]["auras"][0]["cardId"], "north-flood");
    assert_exact_replay(session);
}

fn offers_flood_cast_c3(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-aura"
                && action.descriptor["cardId"] == "north-flood"
                && cells_include(&action.descriptor, "C3")
        })
    })
}

fn deathrite_cast_flood_water_occupied_c3_aura_manifest(seed: u32) -> String {
    let fixture = "cast-flood-aura-water-occupied-c3-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-flood": flood(),
            "north-rain": rain(),
            "north-water": json!({ "cardType": "site", "elements": ["water"] }),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-rain",
                    "north-flood",
                    "north-rain",
                    "north-flood",
                ],
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

fn try_pending_deathrite_with_cast_flood_occupied_c3(
    encoded: &str,
) -> Option<PendingDeathriteCastAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_flood_and_rain(&state(&session)) {
        return None;
    }
    if !offers_flood_cast_c3(&session) {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["rubble"] == true {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCastAuraSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_cast_flood_water_occupied_c3_aura_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(deathrite_cast_flood_water_occupied_c3_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cast_flood_occupied_c3(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with Flood cast-aura on flooded occupied Water at C3",
        )
}

#[test]
fn rule_catalog_1457_cast_flood_aura_withheld_during_pending_deathrite_order_on_flooded_occupied_water_site_at_c3()
 {
    let encoded = deathrite_cast_flood_water_occupied_c3_aura_seed_with(1457);
    let mut setup = try_pending_deathrite_with_cast_flood_occupied_c3(&encoded).expect(
        "complete Flood cast-aura on flooded occupied Water site at C3 Deathrite withheld setup",
    );
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert_ne!(paused["realm"]["sites"]["C3"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-flood"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_flood_cast_c3(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_flood_cast_c3(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C3")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(state(session)["realm"]["auras"][0]["cardId"], "north-flood");
    assert_exact_replay(session);
}

fn offers_drought_cast_c3(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-aura"
                && action.descriptor["cardId"] == "north-drought"
                && cells_include(&action.descriptor, "C3")
        })
    })
}

fn deathrite_cast_drought_earth_occupied_c3_aura_manifest(seed: u32) -> String {
    let fixture = "cast-drought-aura-earth-occupied-c3-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-earth": json!({ "cardType": "site", "elements": ["earth"] }),
            "north-rain": rain(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-rain",
                    "north-drought",
                    "north-rain",
                    "north-drought",
                ],
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

fn try_pending_deathrite_with_cast_drought_occupied_c3(
    encoded: &str,
) -> Option<PendingDeathriteCastAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_drought_and_rain(&state(&session)) {
        return None;
    }
    if !offers_drought_cast_c3(&session) {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["rubble"] == true {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCastAuraSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_cast_drought_earth_occupied_c3_aura_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(deathrite_cast_drought_earth_occupied_c3_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cast_drought_occupied_c3(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with Drought cast-aura on drought occupied Earth at C3",
        )
}

#[test]
fn rule_catalog_1458_cast_drought_aura_withheld_during_pending_deathrite_order_on_drought_occupied_earth_site_at_c3()
 {
    let encoded = deathrite_cast_drought_earth_occupied_c3_aura_seed_with(1458);
    let mut setup = try_pending_deathrite_with_cast_drought_occupied_c3(&encoded).expect(
        "complete Drought cast-aura on drought occupied Earth site at C3 Deathrite withheld setup",
    );
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert_ne!(paused["realm"]["sites"]["C3"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-drought"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_drought_cast_c3(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_drought_cast_c3(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && cells_include(descriptor, "C3")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(
        state(session)["realm"]["auras"][0]["cardId"],
        "north-drought"
    );
    assert_exact_replay(session);
}

fn deathrite_cast_drought_water_occupied_c3_aura_manifest(seed: u32) -> String {
    let fixture = "cast-drought-aura-water-occupied-c3-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-rain": rain(),
            "north-water": json!({ "cardType": "site", "elements": ["water"] }),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-rain",
                    "north-drought",
                    "north-rain",
                    "north-drought",
                ],
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

fn try_pending_deathrite_with_cast_drought_water_occupied_c3(
    encoded: &str,
) -> Option<PendingDeathriteCastAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_drought_and_rain(&state(&session)) {
        return None;
    }
    if !offers_drought_cast_c3(&session) {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["rubble"] == true {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCastAuraSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_cast_drought_water_occupied_c3_aura_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(deathrite_cast_drought_water_occupied_c3_aura_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_cast_drought_water_occupied_c3(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Drought cast-aura on occupied Water at C3",
        )
}

#[test]
fn rule_catalog_1464_cast_drought_aura_withheld_during_pending_deathrite_order_on_occupied_water_site_at_c3()
 {
    let encoded = deathrite_cast_drought_water_occupied_c3_aura_seed_with(1464);
    let mut setup = try_pending_deathrite_with_cast_drought_water_occupied_c3(&encoded)
        .expect("complete Drought cast-aura on occupied Water site at C3 Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert_ne!(paused["realm"]["sites"]["C3"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-drought"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_drought_cast_c3(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_drought_cast_c3(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && cells_include(descriptor, "C3")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(
        state(session)["realm"]["auras"][0]["cardId"],
        "north-drought"
    );
    assert_exact_replay(session);
}

fn deathrite_cast_flood_earth_occupied_c3_aura_manifest(seed: u32) -> String {
    let fixture = "cast-flood-aura-earth-occupied-c3-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": json!({ "cardType": "site", "elements": ["earth"] }),
            "north-flood": flood(),
            "north-rain": rain(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-rain",
                    "north-flood",
                    "north-rain",
                    "north-flood",
                ],
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

fn try_pending_deathrite_with_cast_flood_earth_occupied_c3(
    encoded: &str,
) -> Option<PendingDeathriteCastAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_flood_and_rain(&state(&session)) {
        return None;
    }
    if !offers_flood_cast_c3(&session) {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["rubble"] == true {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCastAuraSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_cast_flood_earth_occupied_c3_aura_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(deathrite_cast_flood_earth_occupied_c3_aura_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_cast_flood_earth_occupied_c3(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Flood cast-aura on occupied Earth at C3",
        )
}

#[test]
fn rule_catalog_1463_cast_flood_aura_withheld_during_pending_deathrite_order_on_occupied_earth_site_at_c3()
 {
    let encoded = deathrite_cast_flood_earth_occupied_c3_aura_seed_with(1463);
    let mut setup = try_pending_deathrite_with_cast_flood_earth_occupied_c3(&encoded)
        .expect("complete Flood cast-aura on occupied Earth site at C3 Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert_ne!(paused["realm"]["sites"]["C3"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-flood"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-aura")
    );
    assert!(!offers_flood_cast_c3(session));

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(offers_flood_cast_c3(session));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C3")
    });
    assert!(event_types(&receipt).contains(&"aura-conjured"));
    assert_eq!(state(session)["realm"]["auras"][0]["cardId"], "north-flood");
    assert_exact_replay(session);
}
