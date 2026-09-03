use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest_with_seed(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "genesis-terrain-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-genesis-terrain-rules-v1",
        },
        "cards": {
            "deathrite-1": deathrite_minion(),
            "deathrite-2": deathrite_minion(),
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
                "replaceAdjacentRubbleWithTopAtlasSite": true,
            },
            "north-site": {
                "cardType": "site",
                "elements": ["earth"],
                "sacrificeToDestroyNearbySite": true,
            },
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-site": { "cardType": "site", "elements": ["water"] },
            "south-spell": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "water-site": {
                "cardType": "site",
                "elements": ["water"],
                "genesisGainMana": 1,
            },
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-site",
                    "north-site",
                    "north-site",
                    "north-site",
                    "water-site",
                    "north-site",
                    "north-site",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "deathrite-1",
                    "deathrite-2",
                    "deathrite-1",
                    "deathrite-2",
                    "deathrite-1",
                    "deathrite-2",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-spell"; 6],
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

fn manifest() -> String {
    for seed in 244..512 {
        let manifest = manifest_with_seed(seed);
        let Ok(mut session) = Session::new(&manifest) else {
            continue;
        };
        if setup_if_water_on_pile(&mut session, 2).is_some() {
            return manifest;
        }
    }
    panic!("no seed left water-site atop the Atlas pile after terrain setup");
}

fn setup_if_water_on_pile(
    session: &mut Session,
    deathrite_count: usize,
) -> Option<(Value, Vec<Value>)> {
    keep(session);
    keep(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    for ordinal in 0..deathrite_count {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cell"] == "C3"
                && descriptor["region"] == "underground"
                && descriptor["cardId"] == format!("deathrite-{}", ordinal + 1)
        });
    }
    end_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    end_turn_and_draw_spellbook(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-site-destruction" && descriptor["targetCell"] == "C3"
    });
    let before = state(session);
    let top = before["players"]["north"]["atlas"].get(0)?;
    if top["cardId"] != "water-site" {
        return None;
    }
    if before["players"]["north"]["atlas"].as_array()?.len() < 3 {
        return None;
    }
    let deathrites = before["realm"]["units"]
        .as_array()?
        .iter()
        .filter(|unit| unit["cardId"] == "deathrite-1" || unit["cardId"] == "deathrite-2")
        .cloned()
        .collect::<Vec<_>>();
    if deathrites.len() != deathrite_count {
        return None;
    }
    Some((top.clone(), deathrites))
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
    session.replay_value().expect("replay value")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn assert_exact_replay(session: &Session) {
    assert!(session.verify_replay().expect("verified exact replay"));
}

fn end_turn_and_draw_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn setup(session: &mut Session, deathrite_count: usize) -> (Value, Vec<Value>) {
    setup_if_water_on_pile(session, deathrite_count)
        .unwrap_or_else(|| panic!("expected water-site atop the Atlas pile after terrain setup"))
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct continuation proof keeps terrain replacement, ordered Deathrites, and Genesis together"
)]
fn site_genesis_should_resume_after_ordered_terrain_replacement_deathrites() {
    let manifest = manifest();
    let mut session = Session::new(&manifest).expect("valid terrain Genesis scenario");
    let (top, deathrites) = setup(&mut session, 2);
    assert!(
        state(&session)["players"]["north"]["atlas"]
            .as_array()
            .expect("north atlas")
            .len()
            >= 3,
        "terrain setup must leave enough Atlas cards for two Deathrite draws"
    );
    let mana_before = state(&session)["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let atlas_before = state(&session)["players"]["north"]["atlas"]
        .as_array()
        .expect("north atlas")
        .len();
    let atlas_hand_before = state(&session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north hand atlas")
        .len();

    let (_, interrupted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "replace-rubble-with-top-atlas-site"
            && descriptor["targetCell"] == "C3"
    });
    assert_eq!(
        event_types(&interrupted),
        ["rubble-replaced", "site-played"]
    );
    assert!(interrupted.random_draws.is_empty());
    assert_eq!(state(&session)["phase"], "deathrite-order");
    assert_eq!(state(&session)["decisionSeat"], "north");
    assert_eq!(
        state(&session)["realm"]["sites"]["C3"]["instanceId"],
        top["instanceId"]
    );
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["tapped"],
        true
    );
    assert_eq!(
        state(&session)["players"]["north"]["mana"]
            .as_u64()
            .expect("mana"),
        mana_before + 1
    );
    assert_eq!(
        state(&session)["players"]["north"]["atlas"]
            .as_array()
            .expect("atlas")
            .len(),
        atlas_before - 1
    );
    assert!(deathrites.iter().all(|unit| {
        !state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|candidate| candidate["instanceId"] == unit["instanceId"])
    }));
    assert!(deathrites.iter().all(|unit| {
        !state(&session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("cemetery")
            .iter()
            .any(|card| card["instanceId"] == unit["instanceId"])
    }));
    assert_eq!(
        state(&session)["pendingDeathrites"]["continuation"]["kind"],
        "site-genesis"
    );
    assert_eq!(
        state(&session)["pendingDeathrites"]["continuation"]["genesisGainMana"],
        1
    );

    let checkpoint = create_game_checkpoint(&session).expect("captured checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let restored =
        resume_game_checkpoint(&parse_game_checkpoint(&serialized).expect("parsed checkpoint"))
            .expect("restored checkpoint");
    assert_eq!(state(&restored), state(&session));
    let orders = restored
        .legal_actions()
        .expect("restored actions")
        .into_iter()
        .filter(|action| {
            action.seat == sorcery_engine::contract::Seat::North
                && action.descriptor["kind"] == "order-deathrites"
        })
        .collect::<Vec<_>>();
    assert_eq!(orders.len(), 2);

    let mut branch_hashes = Vec::new();
    for order in &orders {
        let mut branch = restored.clone();
        let chosen = order.descriptor["sourceInstanceId"]
            .as_str()
            .expect("chosen source");
        let other = deathrites
            .iter()
            .find(|unit| unit["instanceId"].as_str() != Some(chosen))
            .expect("other corpse")["instanceId"]
            .as_str()
            .expect("other instance ID");
        let (_, resolved) = accept_where(&mut branch, |descriptor| {
            descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == chosen
        });
        assert_eq!(
            event_types(&resolved),
            [
                "deathrite-order-committed",
                "site-drawn",
                "site-drawn",
                "minion-died",
                "minion-died",
                "mana-gained",
            ]
        );
        assert_eq!(
            resolved
                .events
                .iter()
                .filter(|event| event.event_type == "site-drawn")
                .map(|event| event.payload["sourceInstanceId"]
                    .as_str()
                    .unwrap_or_default())
                .collect::<Vec<_>>(),
            [chosen, other]
        );
        assert_eq!(
            resolved.events.last().expect("mana event").payload,
            json!({
                "amount": 1,
                "seat": "north",
                "sourceInstanceId": top["instanceId"],
            })
        );
        assert_eq!(state(&branch)["phase"], "main");
        assert_eq!(state(&branch)["pendingDeathrites"], Value::Null);
        assert_eq!(
            state(&branch)["realm"]["sites"]["C3"]["instanceId"],
            top["instanceId"]
        );
        assert!(deathrites.iter().all(|unit| {
            state(&branch)["players"]["north"]["cemetery"]
                .as_array()
                .expect("cemetery")
                .iter()
                .any(|card| card["instanceId"] == unit["instanceId"])
        }));
        assert_eq!(
            state(&branch)["players"]["north"]["atlas"]
                .as_array()
                .expect("atlas")
                .len(),
            atlas_before - 3
        );
        assert_eq!(
            state(&branch)["players"]["north"]["hand"]["atlas"]
                .as_array()
                .expect("hand atlas")
                .len(),
            atlas_hand_before + 2
        );
        assert_eq!(
            state(&branch)["players"]["north"]["mana"]
                .as_u64()
                .expect("mana"),
            mana_before + 2
        );
        assert!(resolved.random_draws.is_empty());
        assert_exact_replay(&branch);
        branch_hashes.push(canonical_json(&state(&branch)).expect("branch state hash"));
    }
    assert_eq!(branch_hashes[0], branch_hashes[1]);

    let mut immediate = Session::new(&manifest).expect("immediate scenario");
    let (_, immediate_deathrites) = setup(&mut immediate, 1);
    let (_, immediate_result) = accept_where(&mut immediate, |descriptor| {
        descriptor["kind"] == "replace-rubble-with-top-atlas-site"
            && descriptor["targetCell"] == "C3"
    });
    assert_eq!(
        event_types(&immediate_result),
        [
            "rubble-replaced",
            "site-played",
            "site-drawn",
            "minion-died",
            "mana-gained",
        ]
    );
    assert_eq!(state(&immediate)["phase"], "main");
    assert!(immediate_deathrites.iter().all(|unit| {
        state(&immediate)["players"]["north"]["cemetery"]
            .as_array()
            .expect("cemetery")
            .iter()
            .any(|card| card["instanceId"] == unit["instanceId"])
    }));
    assert_exact_replay(&immediate);
}
