//! Direct proofs for kill-target-wounded-minion Magic region filtering
//! (RULE-CATALOG-0147, RULE-CATALOG-0673–0674).
//!
//! Fatality offers only wounded minions in the caster region. 0609–0610 already
//! prove the wounded-versus-healthy surface slice in `fatality_rules.rs`; these
//! proofs wound a minion and then burrow it so the region filter is the thing
//! under test.

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

fn burrower() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn fatality() -> Value {
    json!({
        "cardType": "magic",
        "killTargetWoundedMinion": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bury() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn fatality_region_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "fatality-region-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-fatality-region-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-fatality": fatality(),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": burrower(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-lash",
                    "north-lash",
                    "north-bury",
                    "north-fatality",
                    "north-lash",
                    "north-fatality",
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
    let mut session = Session::new(encoded).expect("valid fatality-region session");
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    realm_unit(snapshot, instance_id).expect("expected realm unit")
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

fn fatality_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("fatality actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fatality"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn opening_card_ids(encoded: &str) -> (Vec<String>, Vec<String>) {
    let preview = Session::new(encoded).expect("Fatality region seed candidate");
    let snapshot = state(&preview);
    let ids = |cards: &Value| {
        cards
            .as_array()
            .expect("spell cards")
            .iter()
            .map(|card| card["cardId"].as_str().expect("card identity").to_owned())
            .collect()
    };
    (
        ids(&snapshot["players"]["north"]["hand"]["spellbook"]),
        ids(&snapshot["players"]["north"]["spellbook"]),
    )
}

fn seed_with_region_spells(need_second_lash: bool) -> String {
    (673..673 + 512)
        .map(fatality_region_manifest)
        .find(|candidate| {
            let (hand, library) = opening_card_ids(candidate);
            ["north-fatality", "north-lash", "north-bury"]
                .into_iter()
                .all(|card_id| hand.iter().any(|id| id == card_id))
                && (!need_second_lash || library.first().map(String::as_str) == Some("north-lash"))
        })
        .expect("bounded seed with Fatality region spells in hand")
}

fn stage_enemies(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut enemy_ids = Vec::new();
    for _ in 0..count {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cell"] == "C1"
                && descriptor["region"].is_null()
        });
        enemy_ids.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("summoned enemy identity")
                .to_owned(),
        );
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_ids
}

fn lash_minion(session: &mut Session, instance_id: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == instance_id
    });
}

fn bury_minion(session: &mut Session, instance_id: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == instance_id
    });
}

#[test]
fn rule_catalog_0673_fatality_kills_a_wounded_minion_in_the_caster_region_not_underground() {
    let encoded = seed_with_region_spells(true);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_enemies(&mut session, 2);
    let surface_id = enemy_ids[0].clone();
    let buried_id = enemy_ids[1].clone();

    assert!(fatality_targets(&session).is_empty());
    lash_minion(&mut session, &surface_id);
    lash_minion(&mut session, &buried_id);
    let mut wounded = fatality_targets(&session);
    wounded.sort_unstable();
    let mut expected = [surface_id.as_str(), buried_id.as_str()];
    expected.sort_unstable();
    assert_eq!(wounded, expected);

    bury_minion(&mut session, &buried_id);
    let after_bury = state(&session);
    let buried = unit(&after_bury, &buried_id);
    assert_eq!(buried["region"], "underground");
    assert_eq!(buried["damage"], 1);
    assert_eq!(fatality_targets(&session), [surface_id.as_str()]);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fatality"
    });
    assert_eq!(descriptor["target"]["instanceId"], surface_id.as_str());
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );

    let finished = state(&session);
    assert!(realm_unit(&finished, &surface_id).is_none());
    let survivor = unit(&finished, &buried_id);
    assert_eq!(survivor["region"], "underground");
    assert_eq!(survivor["damage"], 1);
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == surface_id.as_str())
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0674_fatality_offers_no_target_for_a_wounded_underground_minion() {
    let encoded = seed_with_region_spells(false);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_enemies(&mut session, 1);
    let buried_id = enemy_ids[0].clone();

    assert!(fatality_targets(&session).is_empty());
    lash_minion(&mut session, &buried_id);
    assert_eq!(fatality_targets(&session), [buried_id.as_str()]);

    bury_minion(&mut session, &buried_id);
    let after_bury = state(&session);
    let buried = unit(&after_bury, &buried_id);
    assert_eq!(buried["region"], "underground");
    assert_eq!(buried["damage"], 1);
    assert!(fatality_targets(&session).is_empty());
    let fatality_casts: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fatality"
        })
        .collect();
    assert_eq!(fatality_casts.len(), 0);
    assert_exact_replay(&session);
}
