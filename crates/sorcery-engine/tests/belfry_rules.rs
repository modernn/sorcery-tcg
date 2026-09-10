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

fn opponent_turn_manifest(seed: u32) -> String {
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
            "north-near": charger(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-near": charger(),
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
                "spellbook": vec!["south-near"; 6],
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

fn opponent_turn_opening_manifest() -> String {
    (1..=4096)
        .map(opponent_turn_manifest)
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("Belfry opponent-turn candidate"));
            let hand = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("north opening spellbook");
            hand.iter().any(|card| card["cardId"] == "north-belfry")
                && hand.iter().any(|card| card["cardId"] == "north-near")
        })
        .expect("bounded seed opening with Belfry and a nearby ally")
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
    accept_named(session, "expected engine-issued action", predicate)
}

fn accept_named(
    session: &mut Session,
    label: &str,
    predicate: impl Fn(&Value) -> bool,
) -> (Value, Receipt) {
    let legal = session.legal_actions().expect("legal actions");
    let action = legal
        .iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "{label}; legal={:?}",
                legal
                    .iter()
                    .map(|action| action.descriptor["kind"].clone())
                    .collect::<Vec<_>>()
            )
        })
        .clone();
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
    let snapshot = state(session);
    let charger = unit(&snapshot, card_id);
    let instance_id = charger["instanceId"].clone();
    let cell = charger["location"].clone();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["to"]["cell"] == cell
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

fn avatar_only_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "belfry-avatar-only" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-belfry-v1",
        },
        "cards": {
            "belfry-north-artifact": belfry(),
            "belfry-north-avatar": avatar(),
            "belfry-north-site": site(),
            "belfry-south-avatar": avatar(),
            "belfry-south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["belfry-north-site"; 6],
                "avatar": "belfry-north-avatar",
                "spellbook": vec!["belfry-north-artifact"; 6],
            },
            "south": {
                "atlas": vec!["belfry-south-site"; 6],
                "avatar": "belfry-south-avatar",
                "spellbook": vec!["belfry-north-artifact"; 6],
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

#[test]
fn belfry_untaps_the_nearby_avatar_without_a_minion() {
    let mut session = Session::new(&avatar_only_manifest()).expect("valid avatar-only Belfry");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "belfry-north-artifact"
            && descriptor["cell"] == "C4"
    });
    assert_eq!(state(&session)["players"]["north"]["avatar"]["tapped"], true);
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(state(&session)["players"]["north"]["avatar"]["tapped"], false);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-untapped"),
        "{:?}",
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>()
    );
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
    let mut session = Session::new(&opponent_turn_opening_manifest())
        .expect("valid Belfry opponent-turn session");
    keep(&mut session);
    keep(&mut session);
    accept_named(&mut session, "north play-site C4", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_named(&mut session, "north cast Belfry", |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-belfry"
            && descriptor["cell"] == "C4"
    });
    accept_named(&mut session, "north summon ally", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-near"
            && descriptor["cell"] == "C4"
    });
    let ally_id = tap_charger(&mut session, "north-near");
    accept_named(&mut session, "north end T1", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    let after_belfry = state(&session);
    assert_eq!(unit(&after_belfry, "north-near")["tapped"], false);
    accept_named(&mut session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_named(&mut session, "south play-site C1", |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_named(&mut session, "south summon to C4", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-near"
            && descriptor["cell"] == "C4"
    });
    let attacker_id = unit(&state(&session), "south-near")["instanceId"].clone();
    accept_named(&mut session, "south move-and-attack C4", |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C4"
    });
    accept_named(&mut session, "south declare-attack avatar", |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "avatar"
    });
    accept_named(&mut session, "north defend", |descriptor| {
        descriptor["kind"] == "defend" && descriptor["unitInstanceId"] == ally_id
    });
    accept_named(&mut session, "close-defend", |descriptor| {
        descriptor["kind"] == "close-defend"
    });
    if session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| action.descriptor["kind"] == "close-intercept")
    {
        accept_named(&mut session, "close-intercept", |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
    let before = state(&session);
    assert_eq!(unit(&before, "north-near")["tapped"], true);
    assert_eq!(unit(&before, "north-near")["instanceId"], ally_id);
    let (_, south_end) = accept_named(&mut session, "south end-turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    assert!(
        south_end.events.iter().all(|event| {
            event.event_type != "minion-untapped" && event.event_type != "avatar-untapped"
        }),
        "Belfry does not fire at the end of the opponent's turn: {:?}",
        south_end
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>()
    );
    let after = state(&session);
    assert_eq!(unit(&after, "north-near")["instanceId"], ally_id);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}
