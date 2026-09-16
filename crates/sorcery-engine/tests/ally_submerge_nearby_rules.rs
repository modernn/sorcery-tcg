//! Direct proofs for ally-submerges-target-nearby-minion Magic
//! (RULE-CATALOG-0555–0556).
//!
//! Ordinary Magic chooses a controlled ally, then submerges one other
//! minion nearby that ally. Nearby is measured from the ally, not the
//! caster. A far minion is not offered. An Earth site is a paid no-op.

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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["water"],
    })
}

fn grounded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn trial() -> Value {
    json!({
        "allySubmergesTargetNearbyMinion": true,
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

fn trial_manifest(seed: u32, water: bool) -> String {
    let site = if water { water_site() } else { earth_site() };
    let fixture = if water {
        "ally-submerge-nearby-water"
    } else {
        "ally-submerge-nearby-earth"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-site": site,
            "north-trial": trial(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-site": site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-trial",
                    "north-trial",
                    "north-trial",
                    "north-trial",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-raider"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    try_accept_where(session, predicate).expect("expected engine-issued action")
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid ally-submerge session");
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

fn opening_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("opening Spellbook hand")
        .iter()
        .map(|card| {
            card["cardId"]
                .as_str()
                .expect("hand card identity")
                .to_owned()
        })
        .collect()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn trial_pairs(session: &Session) -> Vec<(String, String)> {
    session
        .legal_actions()
        .expect("trial actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-trial"
        })
        .filter_map(|action| {
            Some((
                action.descriptor["ally"]["instanceId"].as_str()?.to_owned(),
                action.descriptor["target"]["instanceId"]
                    .as_str()?
                    .to_owned(),
            ))
        })
        .collect()
}

fn seed_with(water: bool, start: u32, required: &[&str]) -> String {
    (start..start + 256)
        .map(|seed| trial_manifest(seed, water))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn south_plays_c1_and_raids_c4(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let (nearby, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (
        far["cardInstanceId"]
            .as_str()
            .expect("far enemy identity")
            .to_owned(),
        nearby["cardInstanceId"]
            .as_str()
            .expect("nearby enemy identity")
            .to_owned(),
    )
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

fn assert_offered_pairs(session: &Session, ally_id: &str, nearby_id: &str, far_id: &str) {
    let avatar_id = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar identity")
        .to_owned();
    let pairs = trial_pairs(session);
    assert!(pairs.contains(&(avatar_id.clone(), ally_id.to_owned())));
    assert!(pairs.contains(&(avatar_id.clone(), nearby_id.to_owned())));
    assert!(pairs.contains(&(ally_id.to_owned(), nearby_id.to_owned())));
    assert!(!pairs.iter().any(|(_, target)| target == far_id));
    assert!(!pairs.contains(&(ally_id.to_owned(), ally_id.to_owned())));
    assert!(!pairs.contains(&(avatar_id, far_id.to_owned())));
}

#[test]
fn rule_catalog_0555_ally_submerges_a_nearby_minion_on_water() {
    let encoded = seed_with(true, 555, &["north-ally", "north-trial"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    let (far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "surface");
    assert_offered_pairs(&session, &ally_id, &nearby_id, &far_id);

    let (cast, submerged) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-trial"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&submerged),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(submerged.events[1].payload["cell"], "C4");
    assert_eq!(submerged.events[1].payload["instanceId"], nearby_id);
    assert_eq!(submerged.events[1].payload["seat"], "south");
    assert_eq!(
        submerged.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "underwater");
    assert_eq!(unit(&state(&session), &ally_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0556_ally_submerge_is_a_paid_noop_on_earth() {
    let encoded = seed_with(false, 556, &["north-ally", "north-trial"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    let (far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    assert_offered_pairs(&session, &ally_id, &nearby_id, &far_id);

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-trial"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &ally_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_exact_replay(&session);
}
