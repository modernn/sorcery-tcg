//! Chain Magic hop gating that the TypeScript setup suite used to prove with forged state.
//!
//! `tests/engine/game-setup-02.test.ts` hand-built a low-mana session and an underground
//! target to show that Chain Magic is unavailable below its printed cost and never hops
//! into another region. Both facts are reached here through legal play instead.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    })
}

fn cards() -> Value {
    json!({
        "north-avatar": avatar(20),
        "north-burrower": {
            "attack": 1,
            "burrowing": true,
            "cardType": "minion",
            "defense": 2,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-chain": {
            "cardType": "magic",
            "damageChainNearbyUnits": true,
            "manaCost": 2,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-site": { "cardType": "site", "elements": ["air"] },
        "south-avatar": avatar(20),
        "south-filler": {
            "attack": 1,
            "cardType": "minion",
            "defense": 1,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "south-site": { "cardType": "site", "elements": ["air"] },
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-cutover" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-cutover-v1",
        },
        "cards": cards(),
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-chain", "north-burrower", "north-burrower", "north-chain"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 4],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn chain_instance_id(session: &Session) -> String {
    state(session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook hand")
        .iter()
        .find(|card| card["cardId"] == "north-chain")
        .expect("Chain Magic in the opening hand")["instanceId"]
        .as_str()
        .expect("Chain Magic identity")
        .to_owned()
}

fn starts(session: &Session, chain_id: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("Chain Magic starts")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardInstanceId"] == chain_id
        })
        .map(|action| action.descriptor)
        .collect()
}

fn hop_target_ids(descriptors: &[Value]) -> Vec<String> {
    let mut ids: Vec<String> = descriptors
        .iter()
        .map(|descriptor| {
            descriptor["target"]["instanceId"]
                .as_str()
                .expect("hop target identity")
                .to_owned()
        })
        .collect();
    ids.sort_unstable();
    ids
}

/// Ends North's turn, lets South place one site, and returns with North placing `north_cell`.
fn hand_back_to_north(session: &mut Session, south_cell: &str, north_cell: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == south_cell
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == north_cell
    });
}

/// One North turn one with the Chain Magic in hand, a single site, and no minions yet.
fn opening_main() -> Session {
    let manifest = manifest(1);
    let mut session = Session::new(&manifest).expect("valid Chain Magic scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

#[test]
fn chain_magic_start_requires_the_full_mana_cost() {
    let mut session = opening_main();
    let chain_id = chain_instance_id(&session);
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();

    // One site pays one mana, so the two-mana Chain Magic offers no first hop at all even
    // though the caster Avatar is nearby itself and every threshold is already met.
    assert_eq!(state(&session)["players"]["north"]["mana"], json!(1));
    assert!(starts(&session, &chain_id).is_empty());

    hand_back_to_north(&mut session, "C1", "C3");

    // The second site lifts North to exactly the printed cost and the same hop appears.
    assert_eq!(state(&session)["players"]["north"]["mana"], json!(2));
    assert_eq!(hop_target_ids(&starts(&session, &chain_id)), [avatar_id]);
}

#[test]
fn chain_magic_hops_skip_units_outside_the_caster_region() {
    let mut session = opening_main();
    let chain_id = chain_instance_id(&session);
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let (burrowed_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-burrower"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underground"
    });
    let (surface_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-burrower"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let burrowed_id = burrowed_summon["cardInstanceId"]
        .as_str()
        .expect("burrowed twin identity")
        .to_owned();
    let surface_id = surface_summon["cardInstanceId"]
        .as_str()
        .expect("surface twin identity")
        .to_owned();

    hand_back_to_north(&mut session, "C1", "C3");
    hand_back_to_north(&mut session, "C2", "B4");
    hand_back_to_north(&mut session, "B1", "B3");

    // Both twins are the same card on the same cell; only the region differs.
    let units = state(&session)["realm"]["units"].clone();
    let region_of = |instance_id: &str| {
        units
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == instance_id)
            .expect("summoned twin")["region"]
            .clone()
    };
    assert_eq!(region_of(&burrowed_id), json!("underground"));
    assert_eq!(region_of(&surface_id), json!("surface"));

    let mut expected_starts = vec![avatar_id.clone(), surface_id.clone()];
    expected_starts.sort_unstable();
    let start_descriptors = starts(&session, &chain_id);
    assert_eq!(hop_target_ids(&start_descriptors), expected_starts);

    let begin = start_descriptors
        .iter()
        .find(|descriptor| descriptor["target"]["instanceId"] == avatar_id)
        .expect("first hop onto the surface caster");
    accept_where(&mut session, |descriptor| descriptor == begin);
    assert_eq!(state(&session)["phase"], "chain-magic");

    let extensions: Vec<Value> = session
        .legal_actions()
        .expect("staged Chain Magic actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "extend-chain-magic")
        .map(|action| action.descriptor)
        .collect();
    assert_eq!(hop_target_ids(&extensions), [surface_id]);
    assert!(!hop_target_ids(&extensions).contains(&burrowed_id));
}
