//! Direct proofs for Lash damage-then-untap Magic (RULE-CATALOG-0024, 0699).
//!
//! `damageTargetUnit` with `targetNearby` and `untapTargetMinionAfterDamage`
//! offers only a nearby minion, deals printed damage, and untaps the target
//! only if it survives. Distinct from 0595–0596, which have no nearby filter
//! and do not untap.

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
fn rule_catalog_0699_lash_lethal_damage_does_not_untap() {
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
