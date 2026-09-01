use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

struct LanceSetup {
    attacker_id: String,
    session: Session,
    summon_receipt: Receipt,
    target_id: Option<String>,
}

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
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn manifest(seed: u32, attacker: &Value, target: &Value, north_atlas_len: usize) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lance-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lance-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-target": target,
            "south-attacker": attacker,
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; north_atlas_len],
                "avatar": "north-avatar",
                "spellbook": vec!["north-target"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-attacker"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
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
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn unit<'a>(position: &'a Value, instance_id: &str) -> Option<&'a Value> {
    position["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
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
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("Lance checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn prepare_lance(
    seed: u32,
    attacker: &Value,
    target: &Value,
    summon_target: bool,
    north_atlas_len: usize,
) -> LanceSetup {
    let encoded = manifest(seed, attacker, target, north_atlas_len);
    let mut session = Session::new(&encoded).expect("valid Lance scenario");
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
    let (summon, summon_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-attacker"
            && descriptor["cell"] == "C1"
    });
    let attacker_id = summon["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let target_id = summon_target.then(|| {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-target"
                && descriptor["cell"] == "C2"
        });
        summon["cardInstanceId"]
            .as_str()
            .expect("target identity")
            .to_owned()
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    LanceSetup {
        attacker_id,
        session,
        summon_receipt,
        target_id,
    }
}

fn declare_attack(setup: &mut LanceSetup, target_kind: &str) {
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.attacker_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == target_kind
            && setup.target_id.as_ref().is_none_or(|target_id| {
                target_kind != "minion" || descriptor["target"]["instanceId"] == *target_id
            })
    });
}

fn close_defend(session: &mut Session, original_target_participates: bool) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend"
            && descriptor["originalTargetParticipates"] == original_target_participates
    })
    .1
}

fn fire(
    session: &mut Session,
    kind: &str,
    shooter_id: &str,
    direction: &str,
    target_id: Option<&str>,
) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == kind
            && descriptor["shooterInstanceId"] == shooter_id
            && descriptor["direction"] == direction
            && target_id.map_or_else(
                || descriptor["hit"].is_null(),
                |target_id| descriptor["hit"]["instanceId"] == target_id,
            )
    })
    .1
}

#[test]
fn lance_should_buff_and_break_on_next_unit_strike_but_not_site_strike() {
    let lancer = minion(json!({ "attack": 1, "defense": 1, "lanceCount": 1 }));
    let target = minion(json!({ "attack": 2, "defense": 2 }));
    let mut strike = prepare_lance(194, &lancer, &target, true, 6);
    assert_eq!(
        event_types(&strike.summon_receipt),
        ["minion-summoned", "lance-gained"]
    );
    assert_eq!(
        strike.summon_receipt.events[1].payload,
        json!({
            "bearerInstanceId": strike.attacker_id,
            "count": 1,
            "sourceInstanceId": strike.attacker_id,
        })
    );
    assert_eq!(
        unit(&state(&strike.session), &strike.attacker_id).expect("Lance bearer")["carriedLanceCount"],
        1
    );
    assert_checkpoint_round_trip(&strike.session);

    let target_id = strike.target_id.clone().expect("unit target");
    declare_attack(&mut strike, "minion");
    let receipt = close_defend(&mut strike.session, true);
    assert_eq!(
        event_types(&receipt),
        [
            "defend-window-closed",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "lance-broken",
            "minion-died",
        ]
    );
    assert_eq!(receipt.events[2].payload["amount"], 2);
    assert_eq!(receipt.events[3].payload["instanceId"], target_id);
    assert_eq!(
        receipt.events[4].payload["bearerInstanceId"],
        strike.attacker_id
    );
    let position = state(&strike.session);
    assert_eq!(
        unit(&position, &strike.attacker_id).expect("surviving attacker")["damage"],
        0
    );
    assert!(
        unit(&position, &strike.attacker_id).expect("surviving attacker")["carriedLanceCount"]
            .is_null()
    );
    assert!(unit(&position, &target_id).is_none());
    assert_exact_replay(&strike.session);

    let mut site_strike = prepare_lance(195, &lancer, &target, false, 6);
    declare_attack(&mut site_strike, "site");
    let receipt = close_defend(&mut site_strike.session, false);
    assert_eq!(
        receipt
            .events
            .iter()
            .find(|event| event.event_type == "undefended-site-struck")
            .expect("site strike")
            .payload["amount"],
        1
    );
    assert!(!event_types(&receipt).contains(&"lance-broken"));
    assert_eq!(
        unit(&state(&site_strike.session), &site_strike.attacker_id).expect("site attacker")["carriedLanceCount"],
        1
    );
    assert_exact_replay(&site_strike.session);
}

#[test]
fn opposing_lances_should_strike_early_simultaneously_and_break_in_striker_order() {
    let attacker = minion(json!({ "attack": 1, "defense": 4, "lanceCount": 1 }));
    let defender = minion(json!({ "attack": 1, "defense": 3, "lanceCount": 3 }));
    let mut setup = prepare_lance(196, &attacker, &defender, true, 6);
    let target_id = setup.target_id.clone().expect("Lance defender");
    declare_attack(&mut setup, "minion");
    let receipt = close_defend(&mut setup.session, true);

    let breaks = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "lance-broken")
        .map(|event| event.payload.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        breaks,
        [
            json!({
                "bearerInstanceId": setup.attacker_id,
                "count": 1,
                "sourceInstanceId": setup.attacker_id,
            }),
            json!({
                "bearerInstanceId": target_id,
                "count": 3,
                "sourceInstanceId": target_id,
            }),
        ]
    );
    let position = state(&setup.session);
    assert!(unit(&position, &setup.attacker_id).is_none());
    let defender = unit(&position, &target_id).expect("surviving Lance defender");
    assert_eq!(defender["damage"], 2);
    assert!(defender["carriedLanceCount"].is_null());
    assert_exact_replay(&setup.session);
}

#[test]
fn ranged_lance_should_break_after_ward_and_use_unmodified_current_power() {
    let lancer = minion(json!({ "attack": 1, "lanceCount": 1, "ranged": true }));
    let warded = minion(json!({ "ward": true }));
    let mut ward = prepare_lance(197, &lancer, &warded, true, 6);
    let target_id = ward.target_id.clone().expect("Ward target");
    let receipt = fire(
        &mut ward.session,
        "shoot-projectile",
        &ward.attacker_id,
        "north",
        Some(&target_id),
    );
    assert_eq!(
        event_types(&receipt),
        [
            "projectile-shot",
            "strike-damage-allocated",
            "damage-dealt",
            "ward-broken",
            "lance-broken",
        ]
    );
    assert_eq!(receipt.events[1].payload["amount"], 2);
    let position = state(&ward.session);
    assert_eq!(
        unit(&position, &target_id).expect("Ward target")["damage"],
        0
    );
    assert_eq!(
        unit(&position, &target_id).expect("Ward target")["warded"],
        false
    );
    assert!(
        unit(&position, &ward.attacker_id).expect("Ranged bearer")["carriedLanceCount"].is_null()
    );
    assert_exact_replay(&ward.session);

    let prevention = minion(json!({ "preventsDamageFromUnitsWithPowerAtLeast": 2 }));
    let mut source = prepare_lance(198, &lancer, &prevention, true, 6);
    let target_id = source.target_id.clone().expect("prevention target");
    let receipt = fire(
        &mut source.session,
        "shoot-projectile",
        &source.attacker_id,
        "north",
        Some(&target_id),
    );
    assert_eq!(receipt.events[1].payload["amount"], 2);
    assert_eq!(receipt.events[2].payload["amount"], 2);
    assert!(receipt.events[2].payload.get("prevented").is_none());
    assert_eq!(
        unit(&state(&source.session), &target_id).expect("damaged target")["damage"],
        2
    );
    assert_exact_replay(&source.session);
}

#[test]
fn missed_and_fixed_projectiles_should_retain_lance_without_adding_its_damage() {
    let ranged = minion(json!({ "lanceCount": 1, "ranged": true }));
    let target = minion(json!({}));
    let mut miss = prepare_lance(199, &ranged, &target, true, 6);
    let receipt = fire(
        &mut miss.session,
        "shoot-projectile",
        &miss.attacker_id,
        "south",
        None,
    );
    assert_eq!(event_types(&receipt), ["projectile-shot"]);
    assert_eq!(
        unit(&state(&miss.session), &miss.attacker_id).expect("missed shooter")["carriedLanceCount"],
        1
    );
    assert_exact_replay(&miss.session);

    let fixed = minion(json!({ "lanceCount": 1, "tapToShootProjectileDamage": 1 }));
    let mut shot = prepare_lance(200, &fixed, &target, true, 6);
    let target_id = shot.target_id.clone().expect("fixed target");
    let receipt = fire(
        &mut shot.session,
        "shoot-damage-projectile",
        &shot.attacker_id,
        "north",
        Some(&target_id),
    );
    assert_eq!(
        event_types(&receipt),
        [
            "projectile-shot",
            "projectile-damage-allocated",
            "damage-dealt",
        ]
    );
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(receipt.events[2].payload["amount"], 1);
    let position = state(&shot.session);
    assert_eq!(
        unit(&position, &target_id).expect("fixed target")["damage"],
        1
    );
    assert_eq!(
        unit(&position, &shot.attacker_id).expect("fixed shooter")["carriedLanceCount"],
        1
    );
    assert!(!event_types(&receipt).contains(&"lance-broken"));
    assert_exact_replay(&shot.session);
}

#[test]
fn lance_break_should_precede_deathrite_deck_out_and_terminal_replay() {
    let lancer = minion(json!({ "attack": 1, "defense": 1, "lanceCount": 1 }));
    let deathrite = minion(json!({ "deathriteDrawSite": true, "defense": 2 }));
    let mut setup = prepare_lance(201, &lancer, &deathrite, true, 3);
    declare_attack(&mut setup, "minion");
    let receipt = close_defend(&mut setup.session, true);

    assert_eq!(
        event_types(&receipt),
        [
            "defend-window-closed",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "lance-broken",
            "minion-died",
            "game-ended",
        ]
    );
    assert_eq!(
        state(&setup.session)["terminal"],
        json!({
            "loser": "north",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "south",
        })
    );
    assert!(
        setup
            .session
            .legal_actions()
            .expect("terminal legal actions")
            .is_empty()
    );
    assert_exact_replay(&setup.session);
}
