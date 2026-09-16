//! Direct proofs for teleport-target-one-diagonal Magic (RULE-CATALOG-0563–0564).
//!
//! Ordinary Magic targets a minion, Artifact, or Aura and teleports it one
//! diagonal step onto an existing location. Cardinal cells and stay are not
//! offered. No diagonal site means the spell has no legal cast.

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

fn fighter() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn displace() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "teleportTargetMinionArtifactOrAuraOneDiagonal": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn displace_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "teleport-one-diagonal" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-teleport-one-diagonal-v1",
        },
        "cards": {
            "north-ally": fighter(),
            "north-avatar": avatar(),
            "north-displace": displace(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-displace",
                    "north-displace",
                    "north-displace",
                    "north-displace",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["north-ally"; 6],
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
    let mut session = Session::new(encoded).expect("valid teleport-one-diagonal session");
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

fn displace_destinations(session: &Session, instance_id: &str) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("displace actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-displace"
                && action.descriptor["target"]["instanceId"] == instance_id
        })
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

fn displace_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-displace"
        })
        .count()
}

fn seed_with(required: &[&str]) -> String {
    (563..563 + 256)
        .map(displace_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn opening_with_ally(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
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
    south_plays_c1(&mut session);
    (session, ally_id)
}

fn lay_diagonal_site(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D4"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
    });
}

fn lay_cardinal_site(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
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
fn rule_catalog_0563_teleport_targets_a_minion_one_diagonal() {
    let encoded = seed_with(&["north-ally", "north-displace"]);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    lay_diagonal_site(&mut session);
    assert_eq!(displace_destinations(&session, &ally_id), ["D3"]);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C4");

    let (cast, teleported) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-displace"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == ally_id
            && descriptor["targetLocation"]["cell"] == "D3"
    });
    let types = event_types(&teleported);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(types.contains(&"unit-teleported"));
    assert!(!types.contains(&"unit-stepped"));
    let hop = teleported
        .events
        .iter()
        .find(|event| event.event_type == "unit-teleported")
        .expect("unit-teleported");
    assert_eq!(hop.payload["targetInstanceId"], ally_id);
    assert_eq!(hop.payload["from"]["cell"], "C4");
    assert_eq!(hop.payload["to"]["cell"], "D3");
    assert_eq!(
        teleported.events[0].payload["instanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(unit(&state(&session), &ally_id)["location"], "D3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0564_teleport_one_diagonal_is_unoffered_without_a_diagonal_site() {
    let encoded = seed_with(&["north-ally", "north-displace"]);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    lay_cardinal_site(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C4");
    assert!(displace_destinations(&session, &ally_id).is_empty());
    assert_eq!(displace_casts(&session), 0);
    assert_exact_replay(&session);
}
