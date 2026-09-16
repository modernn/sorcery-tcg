//! Direct proofs for grant-+2-this-turn then draw-spell Magic (RULE-CATALOG-0529–0530).
//!
//! Ordinary Magic can give an allied minion +2 power this turn and then draw
//! one spell. Avatars and enemy minions are not offered. The power uses the
//! shared this-turn source list and expires at End Phase. An empty Spellbook
//! after the grant is a deck-out.

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

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn enemy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn gift() -> Value {
    json!({
        "cardType": "magic",
        "grantPowerTwoToAllyThisTurnThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn gift_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-power-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-power-then-draw-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-gift",
                    "north-gift",
                    "north-gift",
                    "north-gift",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn empty_library_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-power-then-draw-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-power-then-draw-empty-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-gift", "north-gift"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 6],
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
    let mut session = Session::new(encoded).expect("valid grant-power-then-draw session");
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

fn avatar_instance_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar instance identity")
        .to_owned()
}

fn gift_ally_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("gift actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-gift"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
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

#[test]
fn rule_catalog_0529_power_grant_offers_allied_minions_then_draws_a_spell() {
    let encoded = (529..529 + 256)
        .map(gift_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate, "north");
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic in the opening hand");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (enemy_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let enemy_id = enemy_summon["cardInstanceId"]
        .as_str()
        .expect("enemy instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let before = state(&session);
    let north_avatar = avatar_instance_id(&before, "north");
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let allies = gift_ally_ids(&session);
    assert!(allies.contains(&ally_id));
    assert!(!allies.contains(&north_avatar));
    assert!(!allies.contains(&enemy_id));

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "power-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["amount"], 2);
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    let after = state(&session);
    assert_eq!(
        unit(&after, &ally_id)["temporaryPowerSources"][0],
        granted.events[0].payload["instanceId"]
    );
    let hand_after = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand after draw");
    assert_eq!(hand_after.len(), hand_before);
    assert!(
        hand_after
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert!(
        !serde_json::to_string(&south_view)
            .expect("view JSON")
            .contains(&library_top)
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(event_types(&ended).contains(&"power-expired"));
    let expired = state(&session);
    assert!(unit(&expired, &ally_id)["temporaryPowerSources"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0530_power_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = (530..530 + 256)
        .map(empty_library_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate, "north");
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic filling the opening hand");
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
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
    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "power-granted",
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    let after = state(&session);
    assert_eq!(
        unit(&after, &ally_id)["temporaryPowerSources"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}
