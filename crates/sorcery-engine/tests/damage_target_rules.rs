//! Direct proofs for damage-target-unit Magic (RULE-CATALOG-0595–0596, 1015).
//!
//! Ordinary targeted damage Magic offers same-region minions, deals printed
//! damage through the shared Ward and death pipeline, and enters its owner's
//! cemetery after resolution.
//!
//! 1015 covers damage-target-unit Magic killing a Deathrite minion: the
//! controller draws a site and magic-resolved only appears after deathrite
//! settlement.

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
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn target() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn warded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    })
}

fn deathrite_target() -> Value {
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

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn zap_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "zap-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-zap-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-zap": zap(),
            "south-avatar": avatar(),
            "south-minion": deathrite_target(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-zap"; 6],
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

fn zap_manifest(seed: u32, ward: bool) -> String {
    let south_minion = if ward { warded() } else { target() };
    let fixture = if ward { "zap-ward" } else { "zap-lethal" };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-zap": zap(),
            "south-avatar": avatar(),
            "south-minion": south_minion,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-zap"; 6],
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
    let mut session = Session::new(encoded).expect("valid zap session");
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

fn seed_with(ward: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| zap_manifest(seed, ward))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                state(&preview)["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-zap"))
            })
        })
        .expect("bounded seed with Zap in the opening hand")
}

fn seed_with_deathrite(start: u32) -> String {
    (start..start + 256)
        .map(zap_deathrite_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                state(&preview)["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-zap"))
            })
        })
        .expect("bounded seed with Zap Deathrite setup")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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
        .expect("Zap target identity")
        .to_owned()
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn setup_zap_target(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let target_id = south_plays_c1_and_summons(&mut session);
    (session, target_id)
}

#[test]
fn rule_catalog_0595_zap_deals_lethal_damage_to_a_same_region_minion() {
    let encoded = seed_with(false, 595);
    let (mut session, target_id) = setup_zap_target(&encoded);

    let (_, damaged) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert!(event_types(&damaged).contains(&"minion-died"));
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0596_zap_ward_absorbs_the_damage() {
    let encoded = seed_with(true, 596);
    let (mut session, target_id) = setup_zap_target(&encoded);

    let (_, blocked) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert!(event_types(&blocked).contains(&"ward-broken"));
    assert!(event_types(&blocked).contains(&"magic-resolved"));
    let after = state(&session);
    assert_eq!(unit(&after, &target_id)["warded"], false);
    assert_eq!(unit(&after, &target_id)["damage"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1015_damage_target_minion_deathrite_draws_for_controller_before_magic_resolved()
{
    let encoded = seed_with_deathrite(1015);
    let (mut session, target_id) = setup_zap_target(&encoded);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(receipt.events[1].payload["targetInstanceId"], target_id);
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], target_id);
    let types = event_types(&receipt);
    let damage_dealt = types
        .iter()
        .position(|event_type| *event_type == "damage-dealt")
        .expect("damage-dealt index");
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let minion_died = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion-died index");
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        damage_dealt < site_drawn && site_drawn < minion_died && minion_died < magic_resolved,
        "expected damage-dealt, deathrite site-drawn, minion-died, then magic-resolved; got {types:?}"
    );
    assert_eq!(types.last(), Some(&"magic-resolved"));

    let finished = state(&session);
    assert!(
        !finished["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == target_id)
    );
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}
