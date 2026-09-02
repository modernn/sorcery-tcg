//! Direct proofs for Pudge drag projectile (RULE-CATALOG-0092 / 0093).

use serde_json::{Value, json};
use sorcery_engine::action::ActionDescriptor;
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, LegalAction, Receipt, Seat, opaque_action_id};
use sorcery_engine::session::{Session, StepResult};

const FIXTURE: &str = include_str!("../../../tests/engine/fixtures/drag-projectile-action-v1.json");

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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn pudge() -> Value {
    minion(json!({
        "attack": 5,
        "defense": 5,
        "immobile": true,
        "shootsDragProjectile": true,
    }))
}

fn manifest(seed: u32, cards: &Value, north: &[&str], south: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drag-projectile-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drag-projectile-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": south,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
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

fn end_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn summon(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
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

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("captured drag checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn unit_by_card<'a>(current: &'a Value, card_id: &str) -> &'a Value {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("realm unit")
}

fn drag_actions(session: &Session) -> Vec<LegalAction> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "shoot-drag-projectile")
        .collect()
}

fn seeded<'a>(
    cards: &Value,
    north: &'a [&'a str],
    south: &'a [&'a str],
    wanted: &[&str],
) -> String {
    (1..=512)
        .map(|seed| manifest(seed, cards, north, south))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("drag projectile candidate");
            let current = state(&preview);
            wanted.iter().all(|entry| {
                let (seat, card_id) = entry.split_once(':').expect("seat-qualified card");
                current["players"][seat]["hand"]["spellbook"]
                    .as_array()
                    .expect("opening hand")
                    .iter()
                    .any(|card| card["cardId"] == card_id)
            })
        })
        .expect("bounded seed with the required drag projectile opening hands")
}

/// Builds a North main phase with Pudge at C4, a stealthed ally at C3, and a South minion at C2.
fn hook_position() -> Session {
    let cards = json!({
        "north-avatar": avatar(),
        "north-blocker": minion(json!({ "stealth": true })),
        "north-filler": minion(json!({})),
        "north-pudge": pudge(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(),
        "south-target": minion(json!({ "attack": 3, "defense": 6, "ward": true })),
    });
    let north = [
        "north-pudge",
        "north-blocker",
        "north-filler",
        "north-filler",
        "north-filler",
        "north-filler",
    ];
    let south = [
        "south-target",
        "south-filler",
        "south-filler",
        "south-filler",
        "south-filler",
        "south-filler",
    ];
    let manifest = seeded(
        &cards,
        &north,
        &south,
        &[
            "north:north-pudge",
            "north:north-blocker",
            "south:south-target",
        ],
    );
    let mut session = Session::new(&manifest).expect("valid drag projectile scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    summon(&mut session, "north-pudge", "C4");
    end_and_draw(&mut session);
    play_site(&mut session, "C1");
    end_and_draw(&mut session);
    play_site(&mut session, "C3");
    summon(&mut session, "north-blocker", "C3");
    end_and_draw(&mut session);
    play_site(&mut session, "C2");
    summon(&mut session, "south-target", "C2");
    end_and_draw(&mut session);
    session
}

/// Builds a North main phase where hauling the South power source away kills two fragile allies.
fn movement_deathrite_position() -> Session {
    let cards = json!({
        "north-avatar": avatar(),
        "north-filler": minion(json!({})),
        "north-pudge": pudge(),
        "north-rain": json!({
            "cardType": "magic",
            "damageEachAbovegroundMinion": 1,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        }),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-fragile-a": minion(json!({ "deathriteDrawSite": true, "stealth": true })),
        "south-fragile-b": minion(json!({ "deathriteDrawSite": true, "stealth": true })),
        "south-filler": minion(json!({})),
        "south-power": minion(json!({
            "attack": 3,
            "defense": 10,
            "otherNearbyAlliesPowerBonus": 1,
        })),
        "south-site": site(),
    });
    let north = [
        "north-pudge",
        "north-rain",
        "north-filler",
        "north-filler",
        "north-filler",
        "north-filler",
    ];
    let south = [
        "south-fragile-a",
        "south-fragile-b",
        "south-power",
        "south-filler",
        "south-filler",
        "south-filler",
    ];
    let manifest = seeded(
        &cards,
        &north,
        &south,
        &[
            "north:north-pudge",
            "north:north-rain",
            "south:south-fragile-a",
            "south:south-fragile-b",
            "south:south-power",
        ],
    );
    let mut session = Session::new(&manifest).expect("valid drag Deathrite scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    summon(&mut session, "north-pudge", "C4");
    end_and_draw(&mut session);
    play_site(&mut session, "C1");
    summon(&mut session, "south-fragile-a", "C1");
    summon(&mut session, "south-fragile-b", "C1");
    end_and_draw(&mut session);
    play_site(&mut session, "C3");
    end_and_draw(&mut session);
    play_site(&mut session, "C2");
    summon(&mut session, "south-power", "C2");
    end_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    session
}

/// The ray passes over the stealthed ally at C3 and stops at the first visible unit on C2.
fn assert_hook_enumeration(session: &Session, target_id: &str) {
    let actions = drag_actions(session);
    let southward: Vec<_> = actions
        .iter()
        .filter(|action| action.descriptor["direction"] == "south")
        .collect();
    assert_eq!(southward.len(), 2);
    for action in &southward {
        assert_eq!(action.descriptor["hit"]["instanceId"], target_id);
        assert_eq!(
            action.descriptor["path"],
            json!([
                { "cell": "C4", "region": "surface" },
                { "cell": "C3", "region": "surface" },
                { "cell": "C2", "region": "surface" },
            ])
        );
    }
    assert_eq!(
        southward
            .iter()
            .map(|action| action.label.as_str())
            .collect::<Vec<_>>(),
        [
            format!("Hook south at minion {}…", &target_id[..15]),
            format!("Hook south at minion {}… and fight", &target_id[..15]),
        ]
    );
    // Every other direction leaves the ray empty, so only the no-fight choice is offered.
    for direction in ["east", "north", "west"] {
        let empty: Vec<_> = actions
            .iter()
            .filter(|action| action.descriptor["direction"] == direction)
            .collect();
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].descriptor["hit"], Value::Null);
        assert_eq!(empty[0].descriptor["fightOnArrival"], false);
    }
}

#[test]
fn rule_catalog_0092_drag_projectile_should_stop_at_the_first_visible_unit_and_may_fight() {
    let session = hook_position();
    let current = state(&session);
    let pudge_id = unit_by_card(&current, "north-pudge")["instanceId"]
        .as_str()
        .expect("Pudge identity")
        .to_owned();
    let target_id = unit_by_card(&current, "south-target")["instanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    assert_eq!(unit_by_card(&current, "north-blocker")["location"], "C3");
    assert_eq!(unit_by_card(&current, "south-target")["location"], "C2");

    assert_hook_enumeration(&session, &target_id);

    let mut hauled = session.clone();
    let (_, receipt) = accept_where(&mut hauled, |descriptor| {
        descriptor["kind"] == "shoot-drag-projectile"
            && descriptor["direction"] == "south"
            && descriptor["fightOnArrival"] == false
    });
    assert_eq!(event_types(&receipt), ["projectile-shot", "unit-dragged"]);
    let dragged = &receipt.events[1].payload;
    assert_eq!(
        dragged["from"],
        json!({ "cell": "C2", "region": "surface" })
    );
    assert_eq!(
        dragged["path"],
        json!([
            { "cell": "C2", "region": "surface" },
            { "cell": "C3", "region": "surface" },
            { "cell": "C4", "region": "surface" },
        ])
    );
    assert_eq!(dragged["seat"], "north");
    assert_eq!(dragged["sourceInstanceId"], pudge_id.as_str());
    assert_eq!(dragged["steps"], 2);
    assert_eq!(dragged["targetInstanceId"], target_id.as_str());
    assert_eq!(dragged["to"], json!({ "cell": "C4", "region": "surface" }));
    let after = state(&hauled);
    assert_eq!(unit_by_card(&after, "south-target")["location"], "C4");
    assert_eq!(unit_by_card(&after, "south-target")["warded"], true);
    assert_eq!(unit_by_card(&after, "north-pudge")["tapped"], true);
    assert_eq!(unit_by_card(&after, "north-pudge")["damage"], 0);
    assert!(receipt.random_draws.is_empty());
    assert_exact_replay(&hauled);

    let mut fought = session;
    let (_, receipt) = accept_where(&mut fought, |descriptor| {
        descriptor["kind"] == "shoot-drag-projectile"
            && descriptor["direction"] == "south"
            && descriptor["fightOnArrival"] == true
    });
    assert_eq!(
        event_types(&receipt),
        [
            "projectile-shot",
            "unit-dragged",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "damage-dealt",
            "ward-broken",
        ]
    );
    let after = state(&fought);
    assert_eq!(unit_by_card(&after, "south-target")["location"], "C4");
    assert_eq!(unit_by_card(&after, "south-target")["warded"], false);
    assert_eq!(unit_by_card(&after, "south-target")["damage"], 0);
    assert_eq!(unit_by_card(&after, "north-pudge")["damage"], 3);
    assert_exact_replay(&fought);
}

#[test]
fn rule_catalog_0093_drag_projectile_should_resume_after_ordered_movement_deathrites() {
    let mut session = movement_deathrite_position();
    let current = state(&session);
    let power_id = unit_by_card(&current, "south-power")["instanceId"]
        .as_str()
        .expect("power source identity")
        .to_owned();
    // Rain wounded both fragile allies, which only survive while the power source stays nearby.
    for card_id in ["south-fragile-a", "south-fragile-b"] {
        assert_eq!(unit_by_card(&current, card_id)["damage"], 1);
        assert_eq!(unit_by_card(&current, card_id)["location"], "C1");
    }

    let (_, interrupted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-drag-projectile"
            && descriptor["direction"] == "south"
            && descriptor["hit"]["instanceId"] == power_id.as_str()
            && descriptor["fightOnArrival"] == true
    });
    assert_eq!(
        event_types(&interrupted),
        ["projectile-shot", "unit-dragged"]
    );
    assert_eq!(
        interrupted.events[1].payload["path"],
        json!([
            { "cell": "C2", "region": "surface" },
            { "cell": "C3", "region": "surface" },
        ])
    );
    let paused = state(&session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(unit_by_card(&paused, "south-power")["location"], "C3");
    assert_eq!(
        paused["pendingDeathrites"]["continuation"],
        json!({
            "fightOnArrival": true,
            "kind": "drag-projectile",
            "path": [
                { "cell": "C2", "region": "surface" },
                { "cell": "C3", "region": "surface" },
                { "cell": "C4", "region": "surface" },
            ],
            "pathIndex": 1,
            "shooter": {
                "instanceId": unit_by_card(&paused, "north-pudge")["instanceId"],
                "kind": "minion",
                "seat": "north",
            },
            "target": { "instanceId": power_id, "kind": "minion", "seat": "south" },
        })
    );
    assert_checkpoint_round_trip(&session);

    let orders: Vec<_> = session
        .legal_actions()
        .expect("Deathrite ordering actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .collect();
    assert_eq!(orders.len(), 2);
    let first = orders[0].descriptor["sourceInstanceId"]
        .as_str()
        .expect("first Deathrite source")
        .to_owned();
    let (_, resumed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == first.as_str()
    });
    assert_eq!(
        event_types(&resumed),
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
            "minion-died",
            "unit-dragged",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "damage-dealt",
        ]
    );
    let finished = state(&session);
    assert_eq!(finished["phase"], "main");
    assert_eq!(unit_by_card(&finished, "south-power")["location"], "C4");
    // One from Rain before the haul, then five from Pudge's strike after arrival.
    assert_eq!(unit_by_card(&finished, "south-power")["damage"], 6);
    // Rain hit Pudge too, so its own strike back from the hauled minion adds to that wound.
    assert_eq!(unit_by_card(&finished, "north-pudge")["damage"], 4);
    assert_eq!(finished["pendingDeathrites"], Value::Null);
    assert_exact_replay(&session);
}

#[test]
fn drag_projectile_descriptors_labels_order_and_ids_should_match_typescript() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("valid drag projectile fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");
    let contract = fixture["contract"].as_str().expect("action contract");
    let seat: Seat = serde_json::from_value(fixture["seat"].clone()).expect("fixture seat");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let mut ordered = Vec::new();
    let mut labels = Vec::new();

    for action in fixture["actions"].as_array().expect("fixture actions") {
        let descriptor: ActionDescriptor = serde_json::from_value(action["descriptor"].clone())
            .expect("typed drag projectile descriptor");
        assert!(matches!(
            descriptor,
            ActionDescriptor::ShootDragProjectile { .. }
        ));
        let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
        assert_eq!(serialized, action["descriptor"]);
        let expected_id =
            IdentityHash::parse(action["actionId"].as_str().expect("TypeScript action ID"))
                .expect("valid TypeScript action ID");
        assert_eq!(
            opaque_action_id(contract, seat, state_version, &serialized)
                .expect("Rust action identity"),
            expected_id
        );
        labels.push(
            descriptor
                .state_independent_label()
                .expect("state-independent drag projectile label"),
        );
        ordered.push((
            canonical_json(&serialized).expect("canonical descriptor"),
            expected_id,
        ));
    }

    ordered.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        ordered
            .into_iter()
            .map(|(_, action_id)| action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical TypeScript order")
            .iter()
            .map(|action_id| action_id.as_str().expect("action ID").to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        labels,
        fixture["actions"]
            .as_array()
            .expect("fixture actions")
            .iter()
            .map(|action| action["label"].as_str().expect("label").to_owned())
            .collect::<Vec<_>>()
    );

    for invalid in [
        json!({
            "direction": "south",
            "hit": null,
            "kind": "shoot-drag-projectile",
            "path": [{ "cell": "C4", "region": "surface" }],
            "shooterInstanceId": fixture["shooterInstanceId"],
        }),
        json!({
            "direction": "south",
            "fightOnArrival": false,
            "kind": "shoot-drag-projectile",
            "path": [{ "cell": "C4", "region": "surface" }],
            "shooterInstanceId": fixture["shooterInstanceId"],
        }),
        json!({
            "direction": "diagonal",
            "fightOnArrival": false,
            "hit": null,
            "kind": "shoot-drag-projectile",
            "path": [{ "cell": "C4", "region": "surface" }],
            "shooterInstanceId": fixture["shooterInstanceId"],
        }),
    ] {
        assert!(serde_json::from_value::<ActionDescriptor>(invalid).is_err());
    }
}
