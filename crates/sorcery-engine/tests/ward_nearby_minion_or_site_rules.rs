//! Direct proofs for ward-nearby-minion-or-site Magic
//! (RULE-CATALOG-0551–0552).
//!
//! Ordinary Magic can Ward one nearby minion or one nearby site. Far sites
//! are not offered. Site Ward is a one-shot mark consumed by the next
//! targeted destroy.

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
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bless() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "wardNearbyMinionOrSite": true,
    })
}

fn destroy_site() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn bless_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-nearby-minion-or-site" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-nearby-minion-or-site-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-bless": bless(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-bless",
                    "north-bless",
                    "north-bless",
                    "north-bless",
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

fn destroy_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ward-nearby-site-destroy" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ward-nearby-site-destroy-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bless": bless(),
            "north-destroy": destroy_site(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-dummy": grounded(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-bless",
                    "north-bless",
                    "north-destroy",
                    "north-destroy",
                    "north-destroy",
                    "north-destroy",
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
    let mut session = Session::new(encoded).expect("valid ward-nearby session");
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

fn bless_minion_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("bless actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-bless"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn bless_site_cells(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("bless actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-bless"
        })
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (551..551 + 256)
        .map(bless_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Bless opening cards")
}

fn seed_destroy() -> String {
    (552..552 + 256)
        .map(destroy_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().filter(|card| *card == "north-bless").count() >= 1
                && hand.iter().filter(|card| *card == "north-destroy").count() >= 2
        })
        .expect("bounded seed with Bless and two destroy-site cards")
}

fn south_plays_c1_and_raids_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
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
fn rule_catalog_0551_bless_wards_a_nearby_minion_and_offers_a_nearby_site() {
    let encoded = seed_with(&["north-ally", "north-bless"]);
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
    let enemy_id = south_plays_c1_and_raids_c4(&mut session);
    let before = state(&session);
    assert_eq!(unit(&before, &ally_id)["warded"], false);
    assert_eq!(unit(&before, &enemy_id)["warded"], false);
    let offered_minions = bless_minion_ids(&session);
    assert!(offered_minions.contains(&ally_id));
    assert!(offered_minions.contains(&enemy_id));
    let offered_sites = bless_site_cells(&session);
    assert!(offered_sites.contains(&"C4".to_owned()));
    assert!(!offered_sites.contains(&"C1".to_owned()));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "minion-warded", "magic-resolved"]
    );
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    assert_eq!(granted.events[1].payload["seat"], "north");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["warded"], true);
    assert_eq!(unit(&after, &enemy_id)["warded"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0552_bless_wards_a_nearby_site_until_targeted_destroy() {
    let encoded = seed_destroy();
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    let offered_sites = bless_site_cells(&session);
    assert!(offered_sites.contains(&"C4".to_owned()));
    assert!(!offered_sites.contains(&"C1".to_owned()));
    assert!(bless_minion_ids(&session).is_empty());
    let site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("C4 site identity")
        .to_owned();

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "site-warded", "magic-resolved"]
    );
    assert_eq!(granted.events[1].payload["cell"], "C4");
    assert_eq!(granted.events[1].payload["instanceId"], site_id);
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let warded = state(&session);
    assert_eq!(warded["realm"]["sites"]["C4"]["warded"], true);
    assert_eq!(warded["realm"]["sites"]["C4"]["instanceId"], site_id);

    let (_, consumed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&consumed),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    assert_eq!(consumed.events[1].payload["cell"], "C4");
    assert_eq!(consumed.events[1].payload["instanceId"], site_id);
    let after_ward = state(&session);
    assert_eq!(after_ward["realm"]["sites"]["C4"]["instanceId"], site_id);
    assert!(after_ward["realm"]["sites"]["C4"]["warded"].is_null());

    let (_, destroyed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&destroyed),
        [
            "magic-cast",
            "site-destroyed",
            "rubble-created",
            "magic-resolved"
        ]
    );
    let rubble = state(&session);
    assert_eq!(rubble["realm"]["sites"]["C4"]["rubble"], true);
    assert_exact_replay(&session);
}
