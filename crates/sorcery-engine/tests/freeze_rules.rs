//! Direct proofs for disable-target-nearby-minion-until-next-turn Magic
//! (RULE-CATALOG-0581–0582).
//!
//! Ordinary Freeze Magic disables a nearby minion until the caster's next Start
//! Phase. Unlike measured disable-until-damaged, the flag expires on that turn
//! boundary rather than on damage.

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

fn nearby() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn far() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn freeze() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn freeze_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "freeze-nearby" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-freeze-nearby-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-freeze": freeze(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": far(),
            "south-nearby": nearby(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-freeze",
                    "north-freeze",
                    "north-freeze",
                    "north-freeze",
                    "north-freeze",
                    "north-freeze",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-far",
                    "south-nearby",
                    "south-far",
                    "south-nearby",
                    "south-far",
                    "south-nearby",
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
    let mut session = Session::new(encoded).expect("valid freeze session");
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

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
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

fn has_activate_mana(session: &Session, instance_id: &str) -> bool {
    session
        .legal_actions()
        .expect("mana actions")
        .into_iter()
        .any(|action| {
            action.descriptor["kind"] == "activate-mana"
                && action.descriptor["unitInstanceId"] == instance_id
        })
}

fn freeze_targets(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("Freeze actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-freeze"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
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

fn seed_with(required: &[&str]) -> String {
    (581..581 + 512)
        .map(freeze_manifest)
        .find(|candidate| {
            let north = opening_spell_ids(candidate, "north");
            let south = opening_spell_ids(candidate, "south");
            required
                .iter()
                .all(|id| north.iter().any(|card| card == id))
                && ["south-far", "south-nearby"]
                    .into_iter()
                    .all(|id| south.iter().any(|card| card == id))
        })
        .expect("bounded seed with Freeze and both South minions")
}

fn south_plays_c1_and_summons(session: &mut Session, card_id: &str) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned instance identity")
        .to_owned()
}

fn setup_nearby_and_far(session: &mut Session) -> (String, String) {
    let far_id = south_plays_c1_and_summons(session, "south-far");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let nearby_id = {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-nearby"
                && descriptor["cell"] == "C2"
                && descriptor["region"].is_null()
        });
        summoned["cardInstanceId"]
            .as_str()
            .expect("nearby instance identity")
            .to_owned()
    };
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let north_avatar = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_avatar
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    (nearby_id, far_id)
}

#[test]
fn rule_catalog_0581_freeze_disables_nearby_minion_until_caster_next_start_phase() {
    let encoded = seed_with(&["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, far_id) = setup_nearby_and_far(&mut session);

    let targets = freeze_targets(&session);
    assert!(targets.contains(&nearby_id));
    assert!(!targets.contains(&far_id));

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    let source_id = cast["cardInstanceId"].as_str().expect("Freeze source");
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "expiresAtSeat": "north",
            "instanceId": nearby_id,
            "seat": "south",
            "sourceInstanceId": source_id,
            "stealthRemoved": false,
            "wardRemoved": false,
        })
    );
    assert!(!has_activate_mana(&session, &nearby_id));

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(!has_activate_mana(&session, &nearby_id));

    let (_, expired) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_types(&expired),
        ["turn-ended", "minion-disable-expired", "turn-started"]
    );
    assert_eq!(expired.events[1].payload["instanceId"], nearby_id);
    assert!(unit(&state(&session), &nearby_id)["disableEffects"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0582_freeze_does_not_offer_a_minion_three_steps_away() {
    let encoded = seed_with(&["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, far_id) = setup_nearby_and_far(&mut session);

    let targets = freeze_targets(&session);
    assert!(targets.contains(&nearby_id));
    assert!(!targets.contains(&far_id));
    assert_exact_replay(&session);
}
