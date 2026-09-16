//! Direct proofs for destroy-own-artifact-at-location-for-area-damage Magic
//! (RULE-CATALOG-0577–0578).
//!
//! Ordinary Magic offers each Artifact the caster controls, loose or carried,
//! paired with that Artifact's location. Casting destroys the chosen Artifact
//! and deals 3 damage to each other Unit there. The destroyed Artifact's
//! bearer is excluded. No controlled Artifact leaves the spell unoffered.

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

fn relic() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn detonate() -> Value {
    json!({
        "cardType": "magic",
        "destroyOwnArtifactAtLocationForAreaDamage": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn detonate_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "detonate-own-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-detonate-own-artifact-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-detonate": detonate(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-relic": relic(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-relic",
                    "north-relic",
                    "north-relic",
                    "north-detonate",
                    "north-detonate",
                    "north-detonate",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-relic",
                    "south-relic",
                    "south-relic",
                    "south-raider",
                    "south-raider",
                    "south-raider",
                ],
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
    let mut session = Session::new(encoded).expect("valid detonate-own-artifact session");
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

fn seed_with(required: &[&str]) -> String {
    (577..577 + 256)
        .map(detonate_manifest)
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
}

fn south_ends_after_c1(session: &mut Session) {
    south_plays_c1(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
}

fn artifact_at(session: &Session, card_id: &str, cell: &str) -> String {
    state(session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["cardId"] == card_id && artifact["location"] == cell)
        .expect("expected artifact at cell")["instanceId"]
        .as_str()
        .expect("artifact instance identity")
        .to_owned()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected unit")
}

fn detonate_casts(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("detonate actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-detonate"
        })
        .map(|action| action.descriptor)
        .collect()
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
fn rule_catalog_0577_detonate_destroys_own_relic_and_deals_three_to_an_enemy() {
    let encoded = seed_with(&["north-detonate", "north-relic"]);
    let mut session = opening_main(&encoded);
    south_ends_after_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let relic_id = artifact_at(&session, "north-relic", "C3");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    let enemy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("enemy instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let before = state(&session);
    assert_eq!(
        realm_artifact(&before, &relic_id).expect("own relic")["location"],
        "C3"
    );
    assert_eq!(unit(&before, &enemy_id)["location"], "C3");
    assert_eq!(unit(&before, &enemy_id)["damage"], 0);
    let offered = detonate_casts(&session);
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0]["targetArtifactInstanceId"], relic_id);
    assert_eq!(offered[0]["targetLocation"]["cell"], "C3");

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-detonate"
            && descriptor["targetArtifactInstanceId"] == relic_id
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    let types = event_types(&receipt);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(types.contains(&"artifact-destroyed"));
    assert!(types.contains(&"magic-damage-allocated"));
    assert!(types.contains(&"damage-dealt"));
    assert!(!types.contains(&"artifact-banished"));
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "artifact-destroyed")
        .expect("artifact destruction");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], relic_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let allocated = receipt
        .events
        .iter()
        .find(|event| event.event_type == "magic-damage-allocated")
        .expect("area damage allocation");
    assert_eq!(allocated.payload["amount"], 3);
    assert_eq!(allocated.payload["targetInstanceId"], enemy_id);
    assert_eq!(
        allocated.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let dealt = receipt
        .events
        .iter()
        .find(|event| event.event_type == "damage-dealt")
        .expect("area damage");
    assert_eq!(dealt.payload["amount"], 3);
    assert_eq!(dealt.payload["instanceId"], enemy_id);
    assert_eq!(dealt.payload["seat"], "south");
    let after = state(&session);
    assert!(realm_artifact(&after, &relic_id).is_none());
    assert_eq!(unit(&after, &enemy_id)["location"], "C3");
    assert_eq!(unit(&after, &enemy_id)["damage"], 3);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 20);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == relic_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0578_detonate_is_unoffered_without_an_own_artifact() {
    let encoded = seed_with(&["north-detonate"]);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(
        realm_artifact(
            &state(&session),
            &artifact_at(&session, "south-relic", "C1")
        )
        .is_some()
    );
    assert!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north Spellbook hand")
            .iter()
            .any(|card| card["cardId"] == "north-detonate")
    );
    assert!(detonate_casts(&session).is_empty());
    assert_exact_replay(&session);
}
