//! Direct proofs for return-target-site-to-owner-hand Magic (RULE-CATALOG-0625–0626).
//!
//! Return-site Magic returns a real site to its owner's Atlas hand, leaves no
//! Rubble, and banishes surface minions that occupied it. A protected site
//! prevents the return without leaving play.

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

fn site(protected: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
    });
    if protected {
        value["cannotBeMovedDestroyedOrModified"] = json!(true);
    }
    value
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

fn return_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetSiteToOwnerHand": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn return_site_manifest(seed: u32, protected: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "return-site-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-site-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-return": return_spell(),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(protected),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-return"; 6],
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
    let mut session = Session::new(encoded).expect("valid return-site session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
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

fn return_site_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("return-site actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return"
        })
        .filter_map(|action| {
            Some((
                action.descriptor["targetLocation"]["cell"]
                    .as_str()?
                    .to_owned(),
                action.descriptor["targetSiteInstanceId"]
                    .as_str()?
                    .to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn seed_with_return_spell(protected: bool, start: u32) -> String {
    (start..start + 512)
        .map(|seed| return_site_manifest(seed, protected))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                let north = &state(&preview)["players"]["north"]["hand"];
                let south = &state(&preview)["players"]["south"]["hand"];
                north["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-return"))
                    && south["spellbook"].as_array().is_some_and(|hand| {
                        hand.iter().any(|card| card["cardId"] == "south-minion")
                    })
            })
        })
        .expect("bounded seed with return-site Magic and a South minion in the opening hands")
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
    let enemy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_id
}

#[test]
fn rule_catalog_0625_return_target_site_returns_owners_site_and_banishes_surface_minions() {
    let encoded = seed_with_return_spell(false, 625);
    let mut session = opening_main(&encoded);
    let north_site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("North site identity")
        .to_owned();
    let minion_id = stage_south_minion(&mut session);
    let before = state(&session);
    let south_site_id = before["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South site identity")
        .to_owned();
    let south_atlas_before = before["players"]["south"]["hand"]["atlas"]
        .as_array()
        .expect("South Atlas hand")
        .len();
    assert_eq!(
        return_site_targets(&session),
        [
            ("C1".to_owned(), south_site_id.clone()),
            ("C4".to_owned(), north_site_id.clone()),
        ]
    );
    assert!(realm_unit(&before, &minion_id).is_some());

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
    });
    assert_eq!(
        descriptor["targetLocation"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "site-returned-to-hand",
            "minion-banished",
            "magic-resolved"
        ]
    );
    let returned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-returned-to-hand")
        .expect("site return");
    assert_eq!(returned.payload["cardId"], "south-site");
    assert_eq!(returned.payload["cell"], "C1");
    assert_eq!(returned.payload["instanceId"], south_site_id);
    assert_eq!(returned.payload["owner"], "south");
    let banished = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-banished")
        .expect("surface minion banishment");
    assert_eq!(banished.payload["cardId"], "south-minion");
    assert_eq!(banished.payload["instanceId"], minion_id);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died"
                || event.event_type == "rubble-created"
                || event.event_type == "site-destroyed")
    );

    let after = state(&session);
    assert!(after["realm"]["sites"].get("C1").is_none());
    assert_eq!(after["realm"]["sites"]["C4"]["instanceId"], north_site_id);
    assert_eq!(after["players"]["south"]["avatar"]["location"], "C1");
    assert!(realm_unit(&after, &minion_id).is_none());
    assert!(
        after["players"]["south"]["hand"]["atlas"]
            .as_array()
            .expect("South Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == south_site_id)
    );
    assert_eq!(
        after["players"]["south"]["hand"]["atlas"]
            .as_array()
            .expect("South Atlas hand")
            .len(),
        south_atlas_before + 1
    );
    let north_view = session
        .public_view(Seat::North)
        .expect("North public view after the return");
    assert_eq!(
        north_view["players"]["south"]["hand"]["atlas"],
        south_atlas_before + 1
    );
    assert_eq!(
        return_site_targets(&session),
        [("C4".to_owned(), north_site_id)]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0626_return_target_site_is_prevented_on_a_protected_site() {
    let encoded = seed_with_return_spell(true, 626);
    let mut session = opening_main(&encoded);
    let minion_id = stage_south_minion(&mut session);
    let before = state(&session);
    let south_site = before["realm"]["sites"]["C1"].clone();
    let south_site_id = south_site["instanceId"]
        .as_str()
        .expect("South site identity")
        .to_owned();
    let south_atlas_before = before["players"]["south"]["hand"]["atlas"]
        .as_array()
        .expect("South Atlas hand")
        .len();

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-return-prevented", "magic-resolved"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-returned-to-hand"
                || event.event_type == "minion-banished"
                || event.event_type == "minion-died"
                || event.event_type == "rubble-created")
    );

    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"], south_site);
    assert!(realm_unit(&after, &minion_id).is_some());
    assert_eq!(
        after["players"]["south"]["hand"]["atlas"]
            .as_array()
            .expect("South Atlas hand")
            .len(),
        south_atlas_before
    );
    assert_exact_replay(&session);
}
