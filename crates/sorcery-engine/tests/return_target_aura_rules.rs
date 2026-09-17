//! Direct proofs for return-target-aura-to-owner-hand Magic
//! (RULE-CATALOG-0669–0670).
//!
//! Targeted realm Aura bounce returns a Flood in play to its owner's hidden
//! Spellbook hand. It is not cemetery Aura return: a minion in play is not a
//! target, and an empty realm offers no cast.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn flood() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn return_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetAuraToOwnerHand": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn return_aura_manifest(seed: u32, south_plays_flood: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-return": return_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    if south_plays_flood {
        cards["south-flood"] = flood();
    } else {
        cards["south-minion"] = minion();
    }
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "return-target-aura" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-target-aura-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-return"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": if south_plays_flood {
                    vec!["south-flood"; 6]
                } else {
                    vec!["south-minion"; 6]
                },
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
    let mut session = Session::new(encoded).expect("valid return-aura session");
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

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

fn south_affinity(session: &Session) -> (u64, u64) {
    let view = session.public_view(Seat::South).expect("South public view");
    (
        view["players"]["south"]["affinity"]["earth"]
            .as_u64()
            .expect("earth affinity"),
        view["players"]["south"]["affinity"]["water"]
            .as_u64()
            .expect("water affinity"),
    )
}

fn return_aura_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("return-aura actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return"
        })
        .filter_map(|action| {
            action.descriptor["targetAuraInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn stage_south_flood(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "south-flood"
            && cells_include(descriptor, "C1")
    });
    let aura_id = state(session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("Flood identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    aura_id
}

fn stage_south_minion(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    minion_id
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

#[test]
fn rule_catalog_0669_return_target_aura_returns_flood_to_its_owners_hand() {
    let mut session = opening_main(&return_aura_manifest(669, true));
    let aura_id = stage_south_flood(&mut session);
    let before = state(&session);
    let south_hand_before = before["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South Spellbook hand")
        .len();
    assert_eq!(south_affinity(&session), (1, 1));
    assert_eq!(
        return_aura_targets(&session).as_slice(),
        std::slice::from_ref(&aura_id)
    );
    assert_eq!(before["realm"]["auras"][0]["cardId"], "south-flood");
    assert!(
        !session
            .legal_actions()
            .expect("return-aura actions")
            .iter()
            .any(|action| action.descriptor.get("cemeteryMinionInstanceId").is_some())
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    let returned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "aura-returned-to-hand")
        .expect("Aura return");
    assert_eq!(returned.payload["cardId"], "south-flood");
    assert_eq!(returned.payload["instanceId"], aura_id);
    assert_eq!(returned.payload["owner"], "south");
    assert_eq!(returned.payload["seat"], "south");
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
        after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert_eq!(
        after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .len(),
        south_hand_before + 1
    );
    assert!(
        !after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(
        north_view["players"]["south"]["hand"]["spellbook"],
        json!(south_hand_before + 1)
    );
    assert_eq!(south_affinity(&session), (1, 0));
    assert_eq!(return_aura_targets(&session), Vec::<String>::new());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0670_return_target_aura_offers_no_target_without_a_realm_aura() {
    let mut session = opening_main(&return_aura_manifest(670, false));
    let minion_id = stage_south_minion(&mut session);
    let before = state(&session);
    assert!(
        before["realm"]
            .get("auras")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    assert_eq!(return_aura_targets(&session), Vec::<String>::new());
    assert!(
        !session
            .legal_actions()
            .expect("post-staging actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return")
    );

    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == minion_id)
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .iter()
            .any(|card| card["cardId"] == "north-return")
    );
    assert_exact_replay(&session);
}
