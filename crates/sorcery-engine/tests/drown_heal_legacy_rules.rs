//! Direct proofs for Drown occupancy of a non-Submerge minion and targetless
//! controller healing (RULE-CATALOG-0045–0046, 0693–0694).
//!
//! 0591–0592 already cover a Submerge minion surviving underwater and the
//! earth-only paid no-op. Drown also submerges a minion without Submerge, and
//! that minion dies. Targetless `healController` restores only the caster
//! Avatar through the printed-life cap and cannot leave Death's Door, unlike
//! 0651–0652 which offer a chosen Avatar.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    })
}

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth", "water"],
    })
}

fn lander() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drown() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "submergeTargetMinion": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn loss() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisLoseControllerLife": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn heal() -> Value {
    json!({
        "cardType": "magic",
        "healController": 7,
        "manaCost": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn drown_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drown-heal-legacy-drown" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drown-heal-legacy-drown-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-drown": drown(),
            "north-site": earth_site(),
            "south-avatar": avatar(20),
            "south-minion": lander(),
            "south-site": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-drown"; 6],
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

fn heal_manifest(seed: u32, life: u8) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drown-heal-legacy-heal" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drown-heal-legacy-heal-v1",
        },
        "cards": {
            "north-avatar": avatar(life),
            "north-heal": heal(),
            "north-loss": loss(),
            "north-site": earth_site(),
            "south-avatar": avatar(20),
            "south-minion": lander(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-heal",
                    "north-heal",
                    "north-heal",
                    "north-loss",
                    "north-loss",
                    "north-loss",
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
    let mut session = Session::new(encoded).expect("valid drown-heal session");
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
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn opening_has_all(encoded: &str, card_ids: &[&str]) -> bool {
    Session::new(encoded).ok().is_some_and(|preview| {
        state(&preview)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| {
                card_ids
                    .iter()
                    .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
            })
    })
}

fn seed_drown(start: u32) -> String {
    (start..start + 256)
        .map(drown_manifest)
        .find(|candidate| opening_has_all(candidate, &["north-drown"]))
        .expect("bounded seed with Drown in the opening hand")
}

fn seed_heal(life: u8, start: u32) -> String {
    (start..start + 256)
        .map(|seed| heal_manifest(seed, life))
        .find(|candidate| opening_has_all(candidate, &["north-heal", "north-loss"]))
        .expect("bounded seed with healController and life-loss Genesis in the opening hand")
}

fn south_plays_c1_and_summons(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Drown target identity")
        .to_owned()
}

fn healing_after_genesis(life: u8, start: u32) -> (Session, String) {
    let encoded = seed_heal(life, start);
    let mut session = opening_main(&encoded);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-loss"
            && descriptor["region"].is_null()
    });
    let heal = session
        .legal_actions()
        .expect("healing action")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-heal"
        })
        .expect("targetless healing Magic");
    assert!(heal.descriptor.get("target").is_none());
    assert!(heal.descriptor.get("cemeteryMinionInstanceId").is_none());
    let spell_id = heal.descriptor["cardInstanceId"]
        .as_str()
        .expect("healing Magic identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor == &heal.descriptor);
    (session, spell_id)
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
fn rule_catalog_0693_drown_kills_a_non_submerge_minion_at_a_water_site() {
    let encoded = seed_drown(693);
    let mut session = opening_main(&encoded);
    let target_id = south_plays_c1_and_summons(&mut session);
    assert_eq!(
        realm_unit(&state(&session), &target_id).expect("surface lander")["region"],
        "surface"
    );

    let (cast, drowned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-drown"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&drowned),
        [
            "magic-cast",
            "minion-submerged",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(
        drowned.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": target_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after = state(&session);
    assert!(realm_unit(&after, &target_id).is_none());
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0694_heal_controller_caps_and_cannot_leave_deaths_door() {
    let (capped, spell_id) = healing_after_genesis(20, 694);
    let capped_state = state(&capped);
    assert_eq!(capped_state["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(capped_state["players"]["north"]["mana"], 0);
    assert!(
        capped_state["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    let receipt = capped.transcript().last().expect("healing receipt");
    assert_eq!(
        event_types(receipt),
        ["magic-cast", "avatar-healed", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["amount"], 2);
    assert_eq!(receipt.events[1].payload["attemptedAmount"], 7);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], spell_id);

    let (death_door, _) = healing_after_genesis(2, 1694);
    let death_door_state = state(&death_door);
    assert_eq!(death_door_state["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(death_door_state["terminal"]["status"], "active");
    assert_eq!(
        event_types(
            death_door
                .transcript()
                .last()
                .expect("Death's Door receipt")
        ),
        ["magic-cast", "magic-resolved"]
    );
    assert_exact_replay(&capped);
    assert_exact_replay(&death_door);
}
