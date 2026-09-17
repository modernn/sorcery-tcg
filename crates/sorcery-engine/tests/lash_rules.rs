//! Direct proofs for Lash damage-then-untap Magic (RULE-CATALOG-0024, 0699,
//! RULE-CATALOG-1062, RULE-CATALOG-1120).
//!
//! `damageTargetUnit` with `targetNearby` and `untapTargetMinionAfterDamage`
//! offers only a nearby minion, deals printed damage, and untaps the target
//! only if it survives. Distinct from 0595–0596, which have no nearby filter
//! and do not untap. While Deathrites wait for ordering, Lash Magic stays
//! withheld until the chain drains.

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

fn nearby(defense: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 0,
        "summonToAnySite": true,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn distant() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "targetNearby": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "untapTargetMinionAfterDamage": true,
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

fn visitor() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn lash_manifest(seed: u32, defense: u8) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lash-nearby" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lash-nearby-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-distant": distant(),
            "south-nearby": nearby(defense),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lash"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-nearby",
                    "south-distant",
                    "south-nearby",
                    "south-distant",
                    "south-nearby",
                    "south-distant",
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
    let mut session = Session::new(encoded).expect("valid Lash session");
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn lash_targets(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-lash"
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

fn seed_with(defense: u8, start: u32) -> String {
    (start..start + 512)
        .map(|seed| lash_manifest(seed, defense))
        .find(|candidate| {
            let north = opening_spell_ids(candidate, "north");
            let south = opening_spell_ids(candidate, "south");
            north.iter().any(|id| id == "north-lash")
                && south.iter().any(|id| id == "south-nearby")
                && south.iter().any(|id| id == "south-distant")
        })
        .expect("bounded seed with Lash and both South minions")
}

fn deathrite_lash_manifest(seed: u32) -> String {
    let fixture = "lash-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lash": lash(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-lash",
                    "north-rain",
                    "north-rain",
                    "north-lash",
                    "north-rain",
                    "north-lash",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-visitor", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_lash_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-lash", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteLashSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteLashSetup> {
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
    let visitor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    })?;
    let visitor_id = visitor.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_lash_and_rain(&state(&session)) {
        return None;
    }
    if lash_targets(&session).is_empty() {
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
    Some(PendingDeathriteLashSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_lash_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_lash_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Lash Magic in hand")
}

fn summon_south(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned instance identity")
        .to_owned()
}

fn setup_nearby_and_distant(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let nearby_id = summon_south(session, "south-nearby", "C4");
    let distant_id = summon_south(session, "south-distant", "C1");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (nearby_id, distant_id)
}

#[test]
fn rule_catalog_0699_lash_damages_then_untaps_only_a_surviving_nearby_minion() {
    let encoded = seed_with(2, 699);
    let mut session = opening_main(&encoded);
    let (nearby_id, distant_id) = setup_nearby_and_distant(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == nearby_id
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let targets = lash_targets(&session);
    assert!(targets.contains(&nearby_id));
    assert!(!targets.contains(&distant_id));
    assert!(targets.iter().all(|id| id == &nearby_id));
    assert_eq!(
        unit(&state(&session), &nearby_id).expect("tapped nearby")["tapped"],
        json!(true)
    );

    let (_, survived) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&survived),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-untapped",
            "magic-resolved",
        ]
    );
    let after = state(&session);
    let survivor = unit(&after, &nearby_id).expect("surviving nearby");
    assert_eq!(survivor["damage"], 1);
    assert_eq!(survivor["tapped"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1120_lash_lethal_damage_does_not_untap() {
    let encoded = seed_with(1, 1699);
    let mut session = opening_main(&encoded);
    let (nearby_id, distant_id) = setup_nearby_and_distant(&mut session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let targets = lash_targets(&session);
    assert!(targets.contains(&nearby_id));
    assert!(!targets.contains(&distant_id));

    let (_, died) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&died),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert!(unit(&state(&session), &nearby_id).is_none());
    assert!(unit(&state(&session), &distant_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1062_lash_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_lash_seed_with(1062);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete Lash Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
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
    assert!(unit(&paused, &visitor_id).is_some());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(lash_targets(session).is_empty());

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
    let targets = lash_targets(session);
    assert!(targets.contains(&visitor_id));
    assert!(targets.iter().all(|id| id == &visitor_id));

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "magic-resolved",
        ]
    );
    let after = state(session);
    let survivor = unit(&after, &visitor_id).expect("surviving nearby visitor");
    assert_eq!(survivor["damage"], 2);
    assert_exact_replay(session);
}
