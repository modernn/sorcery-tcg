//! Direct proofs for Magic target filters by caster region and enemy Stealth
//! (RULE-CATALOG-0023, RULE-CATALOG-0697).
//!
//! Targeted Magic offers only units in the caster region. Enemy Stealth is
//! excluded; own Stealth stays targetable. Distinct from 0617–0618 (Grant-
//! Stealth) and from 0673–0674 (Fatality's wounded-minion region filter).

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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn zap() -> Value {
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

fn region_target_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "magic-region-target" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-magic-region-target-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-magic": zap(),
            "north-minion": minion(json!({ "burrowing": true, "stealth": true })),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-plain": minion(json!({})),
            "south-site": site(),
            "south-stealth": minion(json!({ "stealth": true })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-bury",
                    "north-bury",
                    "north-magic",
                    "north-magic",
                    "north-minion",
                    "north-minion",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-plain",
                    "south-plain",
                    "south-plain",
                    "south-stealth",
                    "south-stealth",
                    "south-stealth",
                ],
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
    let mut session = Session::new(encoded).expect("valid region-target session");
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

fn magic_target_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("targeted Magic actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-magic"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
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

fn seed_with(start: u32, need: &[&str]) -> String {
    (start..start + 512)
        .map(region_target_manifest)
        .find(|candidate| opening_has_all(candidate, need))
        .expect("bounded seed with required Magic targeting cards")
}

fn summon_north_minion(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("friendly minion identity")
        .to_owned()
}

fn south_plays_c1_and_summons(
    session: &mut Session,
    stealth: bool,
    plain: bool,
) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let stealth_id = if stealth {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-stealth"
                && descriptor["region"].is_null()
        });
        summoned["cardInstanceId"]
            .as_str()
            .expect("enemy Stealth identity")
            .to_owned()
    } else {
        String::new()
    };
    let plain_id = if plain {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-plain"
                && descriptor["region"].is_null()
        });
        summoned["cardInstanceId"]
            .as_str()
            .expect("plain enemy identity")
            .to_owned()
    } else {
        String::new()
    };
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (stealth_id, plain_id)
}

#[test]
fn rule_catalog_0697_magic_targets_stay_in_the_caster_region_not_underground() {
    let encoded = seed_with(697, &["north-bury", "north-magic", "north-minion"]);
    let mut session = opening_main(&encoded);
    let friendly_id = summon_north_minion(&mut session);
    let (_, plain_id) = south_plays_c1_and_summons(&mut session, false, true);

    let surface = magic_target_ids(&session);
    assert!(
        surface.contains(&friendly_id),
        "a surface ally stays targetable"
    );
    assert!(
        surface.contains(&plain_id),
        "an exposed surface enemy stays targetable"
    );

    let bury = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();
    let burrowed = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == bury
            && descriptor["target"]["instanceId"] == friendly_id
    })
    .1;
    assert_eq!(
        event_types(&burrowed),
        ["magic-cast", "minion-burrowed", "magic-resolved"]
    );
    assert_eq!(
        realm_unit(&state(&session), &friendly_id).expect("burrowed ally")["region"],
        "underground"
    );

    let confined = magic_target_ids(&session);
    assert!(
        !confined.contains(&friendly_id),
        "a surface caster may not reach its own burrowed ally"
    );
    assert!(
        confined.contains(&plain_id),
        "the surface enemy stays reachable from the surface"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0023_magic_targets_exclude_enemy_stealth() {
    let encoded = seed_with(23, &["north-magic", "north-minion"]);
    let mut session = opening_main(&encoded);
    let friendly_id = summon_north_minion(&mut session);
    let (stealth_id, plain_id) = south_plays_c1_and_summons(&mut session, true, true);

    let surface = magic_target_ids(&session);
    assert!(
        surface.contains(&friendly_id),
        "own Stealth stays targetable"
    );
    assert!(
        surface.contains(&plain_id),
        "an exposed enemy stays targetable"
    );
    assert!(
        !surface.contains(&stealth_id),
        "enemy active Stealth must never be offered"
    );
    assert_exact_replay(&session);
}
