//! Direct proofs for silence-and-tap-nearby-minion then may-draw-spell
//! Magic (RULE-CATALOG-0553–0554).
//!
//! Ordinary Magic can Silence and tap one nearby minion, then may draw one
//! hidden spell. Silence is this-turn ability loss, not Disable. A far
//! minion is not offered. Declining the draw still Silences and taps.

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

fn caster() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "spellcaster": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
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

fn insult() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "silenceAndTapNearbyMinionThenMayDrawSpell": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "cardType": "magic",
        "healController": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn insult_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "silence-tap-then-may-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-silence-tap-then-may-draw-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-caster": caster(),
            "north-dummy": dummy(),
            "north-insult": insult(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-far": grounded(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-caster",
                    "north-dummy",
                    "north-dummy",
                    "north-insult",
                    "north-insult",
                    "north-insult",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-far"; 6],
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
    let mut session = Session::new(encoded).expect("valid silence-tap session");
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

fn insult_targets(session: &Session) -> Vec<(String, bool)> {
    session
        .legal_actions()
        .expect("insult actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-insult"
        })
        .filter_map(|action| {
            Some((
                action.descriptor["target"]["instanceId"]
                    .as_str()?
                    .to_owned(),
                action.descriptor["drawZone"] == "spellbook",
            ))
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (553..553 + 256)
        .map(insult_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Insult opening cards")
}

fn south_plays_c1_and_summons_far(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("far identity")
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

#[test]
fn rule_catalog_0553_insult_silences_and_taps_a_nearby_minion_then_draws() {
    let encoded = seed_with(&["north-caster", "north-dummy", "north-insult"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let caster_id = summoned["cardInstanceId"]
        .as_str()
        .expect("caster instance identity")
        .to_owned();
    let far_id = south_plays_c1_and_summons_far(&mut session);
    let before = state(&session);
    assert_eq!(unit(&before, &caster_id)["tapped"], false);
    assert!(unit(&before, &caster_id)["silenced"].is_null());
    assert!(unit(&before, &caster_id)["disableEffects"].is_null());
    assert!(unit(&before, &caster_id)["disabledUntilDamaged"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("pre-insult actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["cardId"] == "north-dummy"
                    && action.descriptor["casterInstanceId"] == caster_id
            })
    );
    let offered = insult_targets(&session);
    assert!(offered.iter().any(|(id, draw)| *id == caster_id && *draw));
    assert!(offered.iter().any(|(id, draw)| *id == caster_id && !*draw));
    assert!(!offered.iter().any(|(id, _)| *id == far_id));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-insult"
            && descriptor["target"]["instanceId"] == caster_id
            && descriptor["drawZone"] == "spellbook"
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], caster_id);
    assert_eq!(granted.events[1].payload["seat"], "north");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    let silenced = unit(&after, &caster_id);
    assert_eq!(silenced["tapped"], true);
    assert_eq!(silenced["silenced"], true);
    assert!(silenced["disableEffects"].is_null());
    assert!(silenced["disabledUntilDamaged"].is_null());
    assert_eq!(
        silenced["temporarySilenceSources"],
        json!([cast["cardInstanceId"]])
    );
    assert!(
        !session
            .legal_actions()
            .expect("post-insult actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["casterInstanceId"] == caster_id
            })
    );
    assert!(
        session
            .legal_actions()
            .expect("avatar still casts")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["cardId"] == "north-dummy"
                    && action.descriptor["casterInstanceId"]
                        == after["players"]["north"]["avatar"]["card"]["instanceId"]
            })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0554_insult_still_silences_and_taps_when_the_draw_is_declined() {
    let encoded = seed_with(&["north-caster", "north-insult"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let caster_id = summoned["cardInstanceId"]
        .as_str()
        .expect("caster instance identity")
        .to_owned();
    south_plays_c1_and_summons_far(&mut session);
    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-insult"
            && descriptor["target"]["instanceId"] == caster_id
            && descriptor["drawZone"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-silenced",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let silenced = state(&session);
    assert_eq!(unit(&silenced, &caster_id)["tapped"], true);
    assert_eq!(unit(&silenced, &caster_id)["silenced"], true);
    assert!(unit(&silenced, &caster_id)["disableEffects"].is_null());

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "silence-expired"
            && event.payload["instanceId"] == caster_id
            && event.payload["sourceInstanceId"] == cast["cardInstanceId"]
    }));
    let after = state(&session);
    assert_eq!(unit(&after, &caster_id)["tapped"], true);
    assert!(unit(&after, &caster_id)["silenced"].is_null());
    assert!(unit(&after, &caster_id)["temporarySilenceSources"].is_null());
    assert!(unit(&after, &caster_id)["disableEffects"].is_null());
    assert_exact_replay(&session);
}
