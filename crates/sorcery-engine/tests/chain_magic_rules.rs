//! Direct proofs for 1×1 Chain Magic hops (RULE-CATALOG-0030, 0696).
//!
//! 0385–0386 already cover oversized Spellcaster footprint hops. This slice
//! keeps the 0030 leftover: a 1×1 caster stages distinct nearby hops, then
//! damages every chosen unit in one resolve. Extra hops cost 2 mana, and hops
//! cannot leave the caster's region.

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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn chain(mana_cost: u8) -> Value {
    json!({
        "cardType": "magic",
        "damageChainNearbyUnits": true,
        "manaCost": mana_cost,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn hops_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-hops" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-hops-v1",
        },
        "cards": {
            "north-ally-a": minion(json!({})),
            "north-ally-b": minion(json!({})),
            "north-avatar": avatar(),
            "north-chain": chain(0),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-ally-a",
                    "north-ally-b",
                    "north-chain",
                    "north-ally-a",
                    "north-ally-b",
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

fn filter_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-filter" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-filter-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-burrower": minion(json!({ "burrowing": true })),
            "north-chain": chain(2),
            "north-free-chain": chain(0),
            "north-site": site(),
            "north-target": minion(json!({})),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-free-chain",
                    "north-target",
                    "north-burrower",
                    "north-chain",
                    "north-free-chain",
                    "north-target",
                    "north-burrower",
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
    let mut session = Session::new(encoded).expect("valid Chain Magic session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn opening_has_all(encoded: &str, card_ids: &[&str]) -> bool {
    Session::new(encoded).ok().is_some_and(|preview| {
        state(&preview)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| {
                card_ids
                    .iter()
                    .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
            })
    })
}

fn hand_instance(snapshot: &Value, card_id: &str) -> String {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == card_id)
        .expect("card in hand")["instanceId"]
        .as_str()
        .expect("card identity")
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

fn chain_ids(session: &Session, card_instance_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("Chain Magic actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardInstanceId"] == card_instance_id
        })
        .map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .expect("start target identity")
                .to_owned()
        })
        .collect()
}

fn extend_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("staged Chain Magic actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "extend-chain-magic")
        .map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .expect("extension target identity")
                .to_owned()
        })
        .collect()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort_unstable();
    ids
}

struct ChainHops {
    avatar_id: String,
    chain_id: String,
    first_id: String,
    mana: u64,
    second_id: String,
    session: Session,
}

fn setup_hops(encoded: &str) -> ChainHops {
    let mut session = opening_main(encoded);
    let (first, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally-a"
            && descriptor["cell"] == "C4"
    });
    let (second, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally-b"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let before = state(&session);
    ChainHops {
        avatar_id: before["players"]["north"]["avatar"]["card"]["instanceId"]
            .as_str()
            .expect("North Avatar identity")
            .to_owned(),
        chain_id: hand_instance(&before, "north-chain"),
        first_id: first["cardInstanceId"]
            .as_str()
            .expect("first hop identity")
            .to_owned(),
        mana: before["players"]["north"]["mana"]
            .as_u64()
            .expect("North mana"),
        second_id: second["cardInstanceId"]
            .as_str()
            .expect("second hop identity")
            .to_owned(),
        session,
    }
}

#[test]
fn rule_catalog_0696_chain_magic_stages_distinct_nearby_hops_and_resolves_simultaneously() {
    let encoded = (696..696 + 256)
        .map(hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
        })
        .expect("bounded seed with Chain Magic and both nearby allies in the opening hand");
    let mut hops = setup_hops(&encoded);
    assert_eq!(
        sorted(chain_ids(&hops.session, &hops.chain_id)),
        sorted(vec![
            hops.avatar_id.clone(),
            hops.first_id.clone(),
            hops.second_id.clone(),
        ])
    );

    let (_, begin) = accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    assert!(begin.events.is_empty());
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(staged["players"]["north"]["mana"], hops.mana);
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": hops.first_id,
            "kind": "minion",
            "seat": "north",
        }])
    );
    assert_eq!(
        sorted(extend_ids(&hops.session)),
        sorted(vec![hops.avatar_id.clone(), hops.second_id.clone()])
    );
    assert!(!extend_ids(&hops.session).contains(&hops.first_id));

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["instanceId"] == hops.second_id
    });
    let (_, resolved) = accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    let damaged: Vec<_> = resolved
        .events
        .iter()
        .filter(|event| event.event_type == "magic-damage-allocated")
        .map(|event| event.payload.clone())
        .collect();
    assert_eq!(
        damaged,
        [&hops.first_id, &hops.second_id]
            .into_iter()
            .map(|target_instance_id| json!({
                "amount": 2,
                "sourceInstanceId": hops.chain_id,
                "targetInstanceId": target_instance_id,
            }))
            .collect::<Vec<_>>()
    );
    let first_death = event_types(&resolved)
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("first chained death");
    assert!(
        resolved
            .events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.event_type == "damage-dealt")
            .all(|(index, _)| index < first_death)
    );
    assert_eq!(event_types(&resolved).last(), Some(&"magic-resolved"));
    let after = state(&hops.session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingChainMagic"].is_null());
    assert_eq!(after["players"]["north"]["mana"], hops.mana - 2);
    assert!(realm_unit(&after, &hops.first_id).is_none());
    assert!(realm_unit(&after, &hops.second_id).is_none());
    assert_exact_replay(&hops.session);
}

fn try_hand_instance(snapshot: &Value, card_id: &str) -> Option<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == card_id)?["instanceId"]
        .as_str()
        .map(str::to_owned)
}

fn try_setup_filter(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let opening = state(&session);
    let paid_id = try_hand_instance(&opening, "north-chain")?;
    if opening["players"]["north"]["mana"].as_u64() != Some(1)
        || !chain_ids(&session, &paid_id).is_empty()
    {
        return None;
    }
    let target = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-target"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let burrower = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-burrower"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    })?;
    let snapshot = state(&session);
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == "north-free-chain")?;
    Some((
        session,
        target.0["cardInstanceId"].as_str()?.to_owned(),
        burrower.0["cardInstanceId"].as_str()?.to_owned(),
    ))
}

#[test]
fn chain_magic_requires_mana_and_same_region_hops() {
    let encoded = (1696..1696 + 512)
        .map(filter_manifest)
        .find(|candidate| try_setup_filter(candidate).is_some())
        .expect("bounded seed with paid Chain Magic, free hops, and an underground burrower");
    let preview = opening_main(&encoded);
    let paid_id = hand_instance(&state(&preview), "north-chain");
    assert_eq!(state(&preview)["players"]["north"]["mana"], 1);
    assert!(chain_ids(&preview, &paid_id).is_empty());

    let (mut session, target_id, burrower_id) =
        try_setup_filter(&encoded).expect("complete Chain Magic filter setup");
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let free_id = hand_instance(&state(&session), "north-free-chain");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == free_id
            && descriptor["target"]["instanceId"] == target_id
    });
    let extensions = extend_ids(&session);
    assert!(extensions.contains(&avatar_id));
    assert!(!extensions.contains(&burrower_id));
    assert!(!extensions.contains(&target_id));
    assert_exact_replay(&session);
}
