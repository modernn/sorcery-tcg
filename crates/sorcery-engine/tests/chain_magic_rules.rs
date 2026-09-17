//! Direct proofs for 1×1 Chain Magic hops (RULE-CATALOG-0030, 0696, 0709,
//! 0885–0890, 0893–0894, 0896).
//!
//! 0385–0386 already cover oversized Spellcaster footprint hops. 0696 keeps
//! the 0030 leftover: a 1×1 caster stages distinct nearby hops, then damages
//! every chosen unit in one resolve. 0709 is the edge slice: paid Chain Magic
//! is suppressed without enough mana, and hops cannot leave the caster region.
//! 0885–0886 cover pay-life additional costs on Chain Magic resolution.
//! 0887–0890 cover chosen-discard additional costs on Chain Magic.
//! 0894 covers chosen-discard resolve gating when the staged discard leaves hand.
//! 0893 covers staged mana gates on resolve-chain-magic and extend-chain-magic.
//! 0896 covers pay-life resolve gating when life drops after begin.

use serde_json::{Value, json};
use sorcery_engine::action::ActionDescriptor;
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::game::Game;
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

fn avatar_with_life(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    })
}

fn pay_life_chain(life_cost: u8) -> Value {
    json!({
        "cardType": "magic",
        "damageChainNearbyUnits": true,
        "manaCost": 0,
        "payLifeAsAdditionalCost": life_cost,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn discard_chain() -> Value {
    json!({
        "cardType": "magic",
        "damageChainNearbyUnits": true,
        "discardCardAsAdditionalCost": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn fodder() -> Value {
    json!({
        "cardType": "magic",
        "healController": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn discard_chain_manifest(seed: u32, north_spellbook: &[&str]) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-discard" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-discard-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-chain": discard_chain(),
            "north-fodder": fodder(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
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

fn pay_life_chain_manifest(seed: u32, life: u8) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-pay-life" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-pay-life-v1",
        },
        "cards": {
            "north-avatar": avatar_with_life(life),
            "north-chain": pay_life_chain(2),
            "north-site": site(),
            "south-avatar": avatar_with_life(20),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-chain"; 6],
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

fn offers_resolve_chain_magic(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("staged Chain Magic actions")
        .iter()
        .any(|action| action.descriptor["kind"] == "resolve-chain-magic")
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
fn rule_catalog_0709_chain_magic_requires_mana_and_same_region_hops() {
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

fn setup_pay_life_chain(life: u8, seed: u32) -> (Session, String, String) {
    let mut session = Session::new(&pay_life_chain_manifest(seed, life))
        .expect("valid pay-life Chain Magic session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (target, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let before = state(&session);
    let chain_id = hand_instance(&before, "north-chain");
    let target_id = target["cardInstanceId"]
        .as_str()
        .expect("south minion identity")
        .to_owned();
    (session, chain_id, target_id)
}

fn offers_begin_pay_life_chain(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardId"] == "north-chain"
        })
}

fn replay_game(session: &Session) -> Game {
    let mut game = Game::from_manifest_json(session.manifest_json()).expect("valid replay game");
    for receipt in session.transcript() {
        let action = game
            .legal_actions()
            .expect("replay legal actions")
            .into_iter()
            .find(|action| {
                action
                    .to_legal_action()
                    .is_ok_and(|action| action.action_id == receipt.action_id)
            })
            .expect("recorded engine-issued action");
        game.apply_action(&action).expect("replay action");
    }
    game
}

fn branch_without_staged_discard(serialized: &str, zone: &str, discard_id: &str) -> Game {
    let parsed = parse_game_checkpoint(serialized).expect("parsed chain checkpoint");
    let resumed = resume_game_checkpoint(&parsed).expect("resumed chain session");
    let mut checkpoint_json: Value = serde_json::from_str(serialized).expect("checkpoint JSON");
    let mut branched = replay_game(&resumed);
    let mut edited = branched.authoritative_state();
    edited["players"]["north"]["hand"][zone]
        .as_array_mut()
        .expect("north hand zone")
        .retain(|card| card["instanceId"] != discard_id);
    checkpoint_json["editedState"] = edited.clone();
    assert!(
        branched.test_remove_north_hand_card(zone, discard_id),
        "edited checkpoint branch must drop the staged discard from {zone} hand"
    );
    assert_eq!(
        branched.authoritative_state()["players"]["north"]["hand"][zone],
        edited["players"]["north"]["hand"][zone],
        "checkpoint JSON edit must match the branched hand: {}",
        canonical_json(&checkpoint_json).expect("canonical edited checkpoint JSON")
    );
    branched
}

#[test]
fn rule_catalog_0885_chain_magic_pay_life_is_paid_before_the_cast_resolves() {
    let (mut session, chain_id, target_id) = setup_pay_life_chain(20, 885);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
    });
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "life-paid",
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(resolved.events[0].payload["amount"], 2);
    assert_eq!(resolved.events[0].payload["life"], 18);
    assert_eq!(resolved.events[1].payload["lifePaid"], 2);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0886_deaths_door_cannot_begin_pay_life_chain_magic() {
    let blocked = setup_pay_life_chain(1, 886).0;
    assert_eq!(state(&blocked)["players"]["north"]["avatar"]["life"], 1);
    assert!(!offers_begin_pay_life_chain(&blocked));
    assert_exact_replay(&blocked);

    let (mut session, chain_id, target_id) = setup_pay_life_chain(2, 887);
    assert!(offers_begin_pay_life_chain(&session));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
    });
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "life-paid",
            "avatar-reached-deaths-door",
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 0);
    assert!(!state(&session)["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert!(!offers_begin_pay_life_chain(&session));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0896_chain_magic_resolve_is_unoffered_when_pay_life_cost_exceeds_current_life() {
    let (mut session, chain_id, target_id) = setup_pay_life_chain(3, 896);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 3);
    assert!(offers_begin_pay_life_chain(&session));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
    });
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(staged["players"]["north"]["avatar"]["life"], 3);
    assert!(offers_resolve_chain_magic(&session));

    let checkpoint = create_game_checkpoint(&session).expect("staged pay-life checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let resumed = resume_game_checkpoint(&parsed).expect("resumed staged checkpoint");
    assert_eq!(state(&resumed), staged);
    assert!(offers_resolve_chain_magic(&resumed));

    let mut branched = replay_game(&resumed);
    branched.test_set_north_avatar_life(1);
    assert_eq!(
        branched.authoritative_state()["players"]["north"]["avatar"]["life"],
        1
    );
    assert!(
        !branched
            .legal_actions()
            .expect("branched legal actions")
            .iter()
            .any(|action| matches!(action.descriptor(), ActionDescriptor::ResolveChainMagic)),
        "life below the pay-life cost must issue no resolve-chain-magic"
    );
    assert_exact_replay(&session);
}

fn try_setup_discard_chain(encoded: &str) -> Option<(Session, String, String, String)> {
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
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (target, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let before = state(&session);
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let fodder_id = try_hand_instance(&before, "north-fodder")?;
    let target_id = target["cardInstanceId"].as_str()?.to_owned();
    Some((session, chain_id, fodder_id, target_id))
}

fn offers_discard_chain(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardId"] == "north-chain"
        })
}

#[test]
fn rule_catalog_0887_chain_magic_discard_is_paid_before_the_cast_resolves() {
    let spellbook = [
        "north-chain",
        "north-fodder",
        "north-fodder",
        "north-chain",
        "north-fodder",
        "north-fodder",
    ];
    let encoded = (887..887 + 512)
        .map(|seed| discard_chain_manifest(seed, &spellbook))
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-fodder"])
                && try_setup_discard_chain(candidate).is_some()
        })
        .expect("bounded seed with Chain Magic, fodder, and completable setup");
    let (mut session, chain_id, fodder_id, target_id) =
        try_setup_discard_chain(&encoded).expect("discard Chain Magic setup");
    assert!(offers_discard_chain(&session));
    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["discardCardInstanceId"] == fodder_id
    });
    assert_eq!(begin.events.len(), 0);
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "card-discarded",
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(resolved.events[0].payload["instanceId"], fodder_id);
    assert_eq!(resolved.events[0].payload["sourceInstanceId"], chain_id);
    assert_eq!(
        resolved.events[1].payload["discardCardInstanceId"],
        fodder_id
    );
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == fodder_id)
    );
    assert!(realm_unit(&after, &target_id).is_none());
    assert_exact_replay(&session);
}

fn north_spell_card_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn north_hand_ids(snapshot: &Value, zone: &str) -> Vec<String> {
    snapshot["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("hand identity")
                .to_owned()
        })
        .collect()
}

fn cast_all_fodder(session: &mut Session) {
    while session
        .legal_actions()
        .expect("fodder actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fodder"
        })
    {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fodder"
        });
    }
}

fn seed_with_discard_chain(north_spellbook: &[&str], required: &[&str], start: u32) -> String {
    (start..start + 256)
        .map(|seed| discard_chain_manifest(seed, north_spellbook))
        .find(|candidate| opening_has_all(candidate, required))
        .expect("bounded seed with required opening cards")
}

#[test]
fn rule_catalog_0888_chain_magic_discard_is_unoffered_without_another_hand_card() {
    let encoded = seed_with_discard_chain(
        &[
            "north-chain",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
        ],
        &["north-chain"],
        888,
    );
    let mut session = opening_main(&encoded);
    assert!(
        offers_discard_chain(&session),
        "Atlas leftovers still pay the discard cost"
    );
    cast_all_fodder(&mut session);
    assert!(offers_discard_chain(&session));
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        if state(&session)["players"]["south"]["domainEstablished"].as_bool() != Some(true) {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
        cast_all_fodder(&mut session);
    }
    let after = state(&session);
    assert_eq!(north_hand_ids(&after, "atlas").len(), 0);
    assert_eq!(north_spell_card_ids(&after), ["north-chain".to_owned()]);
    assert!(
        !offers_discard_chain(&session),
        "an empty other-hand must issue no discard Chain Magic"
    );
    assert_exact_replay(&session);
}

fn try_setup_atlas_discard_chain(
    encoded: &str,
) -> Option<(Session, String, String, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let opening = state(&session);
    let atlas_id = north_hand_ids(&opening, "atlas").into_iter().next()?;
    let (target, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let before = state(&session);
    if north_hand_ids(&before, "atlas").is_empty() {
        return None;
    }
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let target_id = target["cardInstanceId"].as_str()?.to_owned();
    Some((
        session,
        chain_id,
        atlas_id,
        target_id,
        "north-site".to_owned(),
    ))
}

fn atlas_discard_chain_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-atlas-discard" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-atlas-discard-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-chain": discard_chain(),
            "north-fodder": fodder(),
            "north-ally": minion(json!({})),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-ally",
                    "north-fodder",
                    "north-chain",
                    "north-ally",
                    "north-fodder",
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

fn chain_discard_begin_ids(session: &Session, chain_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardInstanceId"] == chain_id
        })
        .filter_map(|action| {
            action.descriptor["discardCardInstanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn rule_catalog_0889_chain_magic_discard_may_discard_an_atlas_card() {
    let encoded = (889..889 + 512)
        .map(atlas_discard_chain_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally"])
                && try_setup_atlas_discard_chain(candidate).is_some()
        })
        .expect("bounded seed with Chain Magic, ally, and Atlas discard setup");
    let (mut session, chain_id, atlas_id, target_id, site_card_id) =
        try_setup_atlas_discard_chain(&encoded).expect("Atlas discard Chain Magic setup");
    let atlas_before = state(&session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let discard_ids = chain_discard_begin_ids(&session, &chain_id);
    assert!(discard_ids.iter().any(|id| id == &atlas_id));
    assert!(discard_ids.iter().all(|id| id != &chain_id));
    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["discardCardInstanceId"] == atlas_id
    });
    assert_eq!(begin.events.len(), 0);
    let checkpoint = create_game_checkpoint(&session).expect("staged chain checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized chain checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed chain checkpoint");
    let mut session = resume_game_checkpoint(&parsed).expect("resumed chain session");
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "card-discarded",
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(resolved.events[0].payload["cardId"], site_card_id);
    assert_eq!(resolved.events[0].payload["instanceId"], atlas_id);
    assert_eq!(resolved.events[0].payload["zone"], "atlas");
    assert_eq!(resolved.events[0].payload["sourceInstanceId"], chain_id);
    assert_eq!(
        resolved.events[1].payload["discardCardInstanceId"],
        atlas_id
    );
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == atlas_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        atlas_before - 1
    );
    assert!(realm_unit(&after, &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0890_chain_magic_issues_no_begin_without_a_chosen_discard_while_atlas_remains() {
    let encoded = seed_with_discard_chain(
        &[
            "north-chain",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
        ],
        &["north-chain"],
        890,
    );
    let session = opening_main(&encoded);
    let chain_id = hand_instance(&state(&session), "north-chain");
    assert!(!north_hand_ids(&state(&session), "atlas").is_empty());
    assert!(offers_discard_chain(&session));
    let discard_ids = chain_discard_begin_ids(&session, &chain_id);
    assert!(
        !discard_ids.is_empty(),
        "Atlas leftovers still pay the discard cost"
    );
    assert!(
        session
            .legal_actions()
            .expect("legal actions")
            .iter()
            .filter(|action| {
                action.descriptor["kind"] == "begin-chain-magic"
                    && action.descriptor["cardInstanceId"] == chain_id
            })
            .all(|action| action.descriptor["discardCardInstanceId"].is_string()),
        "every issued begin must carry the chosen discard identity"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0894_chain_magic_resolve_is_unoffered_without_staged_discard_in_hand() {
    let spellbook = [
        "north-chain",
        "north-fodder",
        "north-fodder",
        "north-chain",
        "north-fodder",
        "north-fodder",
    ];
    let encoded = (894..894 + 512)
        .map(|seed| discard_chain_manifest(seed, &spellbook))
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-fodder"])
                && try_setup_discard_chain(candidate).is_some()
        })
        .expect("bounded seed with Chain Magic discard setup");
    let (mut session, chain_id, fodder_id, target_id) =
        try_setup_discard_chain(&encoded).expect("discard Chain Magic setup");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["discardCardInstanceId"] == fodder_id
    });
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["discardCardInstanceId"],
        fodder_id
    );
    assert!(offers_resolve_chain_magic(&session));

    let checkpoint = create_game_checkpoint(&session).expect("staged chain checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized chain checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed chain checkpoint");
    let resumed = resume_game_checkpoint(&parsed).expect("resumed chain session");
    assert_eq!(state(&resumed), staged);
    assert!(offers_resolve_chain_magic(&resumed));

    let branched = branch_without_staged_discard(&serialized, "spellbook", &fodder_id);
    assert_eq!(
        branched.authoritative_state()["pendingChainMagic"]["discardCardInstanceId"],
        fodder_id
    );
    assert!(
        !branched
            .legal_actions()
            .expect("branched legal actions")
            .iter()
            .any(|action| matches!(action.descriptor(), ActionDescriptor::ResolveChainMagic)),
        "a staged discard that left hand must issue no resolve-chain-magic"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0893_chain_magic_staged_mana_gates_resolve_and_extend_independently() {
    const EXTRA_TARGET_MANA: u64 = 2;
    let encoded = (893..893 + 256)
        .map(hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
        })
        .expect("bounded seed with Chain Magic and both nearby allies in the opening hand");
    let mut hops = setup_hops(&encoded);
    assert_eq!(hops.mana, EXTRA_TARGET_MANA);
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    assert_eq!(
        state(&hops.session)["pendingChainMagic"]["targets"]
            .as_array()
            .expect("staged targets")
            .len(),
        1
    );
    assert!(offers_resolve_chain_magic(&hops.session));
    assert_eq!(
        sorted(extend_ids(&hops.session)),
        sorted(vec![hops.avatar_id.clone(), hops.second_id.clone()])
    );

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["instanceId"] == hops.second_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["players"]["north"]["mana"], EXTRA_TARGET_MANA);
    assert_eq!(
        staged["pendingChainMagic"]["targets"]
            .as_array()
            .expect("staged targets")
            .len(),
        2
    );
    assert!(
        offers_resolve_chain_magic(&hops.session),
        "two staged hops on a zero-cost chain cost {EXTRA_TARGET_MANA} mana to resolve"
    );
    assert!(
        extend_ids(&hops.session).is_empty(),
        "a third hop would cost {} mana",
        EXTRA_TARGET_MANA * 3
    );

    let mut stuck = replay_game(&hops.session);
    stuck.test_set_north_mana(1);
    assert_eq!(stuck.authoritative_state()["players"]["north"]["mana"], 1);
    assert!(
        !stuck
            .legal_actions()
            .expect("stuck staged chain actions")
            .iter()
            .any(|action| matches!(action.descriptor(), ActionDescriptor::ResolveChainMagic)),
        "two staged hops need {EXTRA_TARGET_MANA} mana to resolve"
    );
    assert!(
        !stuck
            .legal_actions()
            .expect("stuck staged chain actions")
            .iter()
            .any(|action| matches!(
                action.descriptor(),
                ActionDescriptor::ExtendChainMagic { .. }
            )),
        "a third hop would exceed the one-mana pool"
    );

    let mut short = setup_hops_short(&encoded, true);
    assert_eq!(short.mana, 1);
    accept_where(&mut short.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == short.chain_id
            && descriptor["target"]["instanceId"] == short.first_id
    });
    assert!(
        offers_resolve_chain_magic(&short.session),
        "one staged hop on a zero-cost chain resolves for free"
    );
    assert!(
        extend_ids(&short.session).is_empty(),
        "the next hop costs {EXTRA_TARGET_MANA} mana while the caster has one"
    );
    assert_exact_replay(&hops.session);
    assert_exact_replay(&short.session);
}

fn setup_hops_short(encoded: &str, skip_last_site: bool) -> ChainHops {
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
    if !skip_last_site {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
        });
    }
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
