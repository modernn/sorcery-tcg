//! Direct proofs for official Belfry (RULE-CATALOG-0312–0313).
//!
//! Belfry is a Monument: "At the end of your turn, untap all nearby allies."
//! Nearby is the Artifact's square plus the eight surrounding squares in the
//! same region. Allies are units the conjurer controls, including the Avatar.
//! The trigger fires only at the end of that player's turn, not each turn.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn belfry() -> Value {
    json!({
        "atEndOfControllerTurnUntapNearbyAllies": true,
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn charger() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn nearby_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "belfry-nearby" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-belfry-nearby-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-belfry": belfry(),
            "north-near": charger(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-belfry",
                    "north-belfry",
                    "north-belfry",
                    "north-near",
                    "north-near",
                    "north-near"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn opponent_turn_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "belfry-opponent-turn" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-belfry-opponent-turn-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-belfry": belfry(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-near": charger(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-belfry"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-near"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn nearby_opening_manifest() -> String {
    (1..=4096)
        .map(nearby_manifest)
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("Belfry candidate"));
            let hand = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("north opening spellbook");
            hand.iter().any(|card| card["cardId"] == "north-belfry")
                && hand.iter().any(|card| card["cardId"] == "north-near")
        })
        .expect("bounded seed opening with Belfry and a nearby ally")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step");
    let StepResult::Accepted(receipt) = result else {
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn unit<'a>(state: &'a Value, card_id: &str) -> &'a Value {
    state["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("expected unit")
}

fn artifact_id(session: &Session, card_id: &str) -> Value {
    state(session)["realm"]["artifacts"]
        .as_array()
        .expect("artifacts")
        .iter()
        .find(|artifact| artifact["cardId"] == card_id)
        .expect("expected artifact")["instanceId"]
        .clone()
}

fn tap_charger(session: &mut Session, card_id: &str) -> Value {
    let instance_id = unit(&state(session), card_id)["instanceId"].clone();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["to"]["cell"] == unit(&state(session), card_id)["location"]
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    instance_id
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
fn rule_catalog_0312_belfry_untaps_nearby_allies_at_end_of_your_turn() {
    let mut session = Session::new(&nearby_opening_manifest()).expect("valid Belfry session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-belfry"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-near"
            && descriptor["cell"] == "C4"
    });
    let minion_id = tap_charger(&mut session, "north-near");
    let before = state(&session);
    assert_eq!(before["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(unit(&before, "north-near")["tapped"], true);
    let source_id = artifact_id(&session, "north-belfry");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["tapped"], false);
    assert_eq!(unit(&after, "north-near")["tapped"], false);
    assert_eq!(unit(&after, "north-near")["instanceId"], minion_id);
    let events: Vec<_> = receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    assert!(
        events.contains(&"avatar-untapped"),
        "Belfry untaps the nearby Avatar: {events:?}"
    );
    assert!(
        events.contains(&"minion-untapped"),
        "Belfry untaps the nearby minion: {events:?}"
    );
    assert!(
        receipt.events.iter().any(|event| {
            event.event_type == "minion-untapped"
                && event.payload["instanceId"] == minion_id
                && event.payload["sourceInstanceId"] == source_id
        }),
        "the minion untap is sourced from Belfry"
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0313_belfry_does_not_untap_on_the_opponents_turn() {
    let mut session =
        Session::new(&opponent_turn_manifest()).expect("valid Belfry opponent-turn session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-belfry"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-near"
            && descriptor["cell"] == "C4"
    });
    let minion_id = tap_charger(&mut session, "south-near");
    let before = state(&session);
    assert_eq!(unit(&before, "south-near")["tapped"], true);
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let after = state(&session);
    assert_eq!(unit(&after, "south-near")["tapped"], true);
    assert_eq!(unit(&after, "south-near")["instanceId"], minion_id);
    assert!(
        receipt.events.iter().all(|event| {
            event.event_type != "minion-untapped" && event.event_type != "avatar-untapped"
        }),
        "Belfry does not fire at the end of the opponent's turn: {:?}",
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}
