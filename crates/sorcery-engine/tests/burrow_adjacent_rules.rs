//! Direct proofs for burrow-target-adjacent-minion Magic
//! (RULE-CATALOG-0559–0560, RULE-CATALOG-1055).
//!
//! Ordinary Magic burrows one minion that borders the caster. Same-cell
//! and far minions are not offered. A Water site is a paid no-op. While
//! Deathrites wait for ordering, burrow-adjacent Magic stays withheld
//! until the chain drains.

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

fn burrower() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
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
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bury() -> Value {
    json!({
        "burrowTargetAdjacentMinion": true,
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

fn bury_manifest(seed: u32, water: bool) -> String {
    let site = if water { water_site() } else { earth_site() };
    let fixture = if water {
        "burrow-adjacent-water"
    } else {
        "burrow-adjacent-earth"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": burrower(),
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-site": site,
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
                    "north-ally",
                    "north-bury",
                    "north-bury",
                    "north-bury",
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
    let mut session = Session::new(encoded).expect("valid burrow-adjacent session");
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

fn bury_target_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("bury actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-bury"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn seed_with(water: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| bury_manifest(seed, water))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().filter(|card| *card == "north-ally").count() >= 2
                && hand.iter().any(|card| card == "north-bury")
        })
        .expect("bounded seed with two allies and bury")
}

fn south_plays_c1_and_raids_far(session: &mut Session) -> String {
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
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    far["cardInstanceId"]
        .as_str()
        .expect("far enemy identity")
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

fn setup_adjacent_board(encoded: &str) -> (Session, String, String, String) {
    let mut session = opening_main(encoded);
    let (here, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let here_id = here["cardInstanceId"]
        .as_str()
        .expect("same-cell ally identity")
        .to_owned();
    let far_id = south_plays_c1_and_raids_far(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let (adjacent, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    let adjacent_id = adjacent["cardInstanceId"]
        .as_str()
        .expect("adjacent ally identity")
        .to_owned();
    let offered = bury_target_ids(&session);
    assert!(offered.contains(&adjacent_id));
    assert!(!offered.contains(&here_id));
    assert!(!offered.contains(&far_id));
    (session, adjacent_id, here_id, far_id)
}

fn deathrite_burrow_adjacent_manifest(seed: u32) -> String {
    let fixture = "burrow-adjacent-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": burrower(),
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-bury",
                    "north-rain",
                    "north-rain",
                    "north-bury",
                    "north-rain",
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

fn north_has_bury_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-bury", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteBurrowAdjacentSetup {
    adjacent_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_adjacent_target(
    encoded: &str,
) -> Option<PendingDeathriteBurrowAdjacentSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .any(|card| card["cardId"] == "north-ally")
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let adjacent = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    let adjacent_id = adjacent.0["cardInstanceId"].as_str()?.to_owned();
    if !north_has_bury_and_rain(&state(&session)) {
        return None;
    }
    let offered = bury_target_ids(&session);
    if offered.is_empty() || !offered.contains(&adjacent_id) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteBurrowAdjacentSetup {
        adjacent_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_burrow_adjacent_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_burrow_adjacent_manifest)
        .find(|candidate| try_pending_deathrite_with_adjacent_target(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with burrow-adjacent Magic in hand")
}

#[test]
fn rule_catalog_0559_burrow_targets_an_adjacent_minion_on_earth() {
    let encoded = seed_with(false, 559);
    let (mut session, adjacent_id, here_id, far_id) = setup_adjacent_board(&encoded);
    assert_eq!(unit(&state(&session), &adjacent_id)["region"], "surface");

    let (cast, burrowed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == adjacent_id
    });
    assert_eq!(
        event_types(&burrowed),
        ["magic-cast", "minion-burrowed", "magic-resolved"]
    );
    assert_eq!(burrowed.events[1].payload["cell"], "C3");
    assert_eq!(burrowed.events[1].payload["instanceId"], adjacent_id);
    assert_eq!(burrowed.events[1].payload["seat"], "north");
    assert_eq!(
        burrowed.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(
        unit(&state(&session), &adjacent_id)["region"],
        "underground"
    );
    assert_eq!(unit(&state(&session), &here_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0560_burrow_adjacent_is_a_paid_noop_on_water() {
    let encoded = seed_with(true, 560);
    let (mut session, adjacent_id, here_id, far_id) = setup_adjacent_board(&encoded);

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == adjacent_id
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert_eq!(unit(&state(&session), &adjacent_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &here_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1055_burrow_adjacent_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_burrow_adjacent_seed_with(1055);
    let mut setup = try_pending_deathrite_with_adjacent_target(&encoded)
        .expect("complete burrow-adjacent Deathrite withheld setup");
    let adjacent_id = setup.adjacent_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert_eq!(unit(&paused, &adjacent_id)["location"], "C3");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(bury_target_ids(session).is_empty());

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(unit(&resumed, &adjacent_id)["location"], "C3");
    assert_eq!(bury_target_ids(session), [adjacent_id.as_str()]);

    let (cast, burrowed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["instanceId"] == adjacent_id
    });
    assert_eq!(
        event_types(&burrowed),
        ["magic-cast", "minion-burrowed", "magic-resolved"]
    );
    assert_eq!(burrowed.events[1].payload["cell"], "C3");
    assert_eq!(burrowed.events[1].payload["instanceId"], adjacent_id);
    assert_eq!(burrowed.events[1].payload["seat"], "north");
    assert_eq!(
        burrowed.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(
        unit(&state(session), &adjacent_id)["region"],
        "underground"
    );
    assert_exact_replay(session);
}
