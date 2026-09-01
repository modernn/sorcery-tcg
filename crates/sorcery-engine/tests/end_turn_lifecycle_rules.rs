use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn magic(extra: Value) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra Magic facts must be an object");
    };
    value.as_object_mut().expect("Magic facts").extend(extra);
    value
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(
    seed: u32,
    cards: &Value,
    north_spellbook: &[&str],
    south_spellbook: &[&str],
    atlas_size: usize,
) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "end-turn-lifecycle" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-end-turn-lifecycle-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; atlas_size],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; atlas_size],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn unit<'a>(position: &'a Value, instance_id: &str) -> &'a Value {
    position["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected unit")
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
    let checkpoint = create_game_checkpoint(session).expect("captured lifecycle checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("original actions")
    );
}

fn lifecycle_cards(malakhim: &Value) -> Value {
    json!({
        "north-avatar": avatar(),
        "north-malakhim": malakhim,
        "north-damager": minion(json!({ "genesisMayDamageTargetAdjacentUnit": 2 })),
        "north-freeze": magic(json!({ "disableTargetNearbyMinionUntilNextTurn": true })),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(),
    })
}

fn malakhim_checkpoint(malakhim: &Value) -> (Session, String) {
    let manifest = manifest(
        108,
        &lifecycle_cards(malakhim),
        &["north-malakhim", "north-damager", "north-freeze"],
        &["south-filler"; 3],
        6,
    );
    let mut session = Session::new(&manifest).expect("valid Malakhim scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-malakhim"
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("Malakhim identity")
        .to_owned();
    (session, instance_id)
}

fn advance_to_tapped_malakhim(session: &mut Session, instance_id: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
}

#[test]
fn malakhim_should_untap_only_when_tapped_and_enabled() {
    let malakhim = minion(json!({
        "airborne": true,
        "attack": 4,
        "defense": 4,
        "untapsAtEndOfControllerTurn": true,
        "ward": true,
    }));
    let mut invalid = lifecycle_cards(&malakhim);
    invalid["north-malakhim"]["untapsAtEndOfControllerTurn"] = json!(false);
    assert!(
        Session::new(&manifest(
            108,
            &invalid,
            &["north-malakhim", "north-damager", "north-freeze"],
            &["south-filler"; 3],
            6,
        ))
        .is_err()
    );

    let (mut session, instance_id) = malakhim_checkpoint(&malakhim);
    let (_, ready_end) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(event_types(&ready_end), ["turn-ended", "turn-started"]);
    assert!(
        ready_end
            .events
            .iter()
            .all(|event| event.event_type != "minion-untapped")
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let (_, active_end) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_types(&active_end),
        ["minion-untapped", "turn-ended", "turn-started"]
    );
    assert_eq!(
        active_end.events[0].payload,
        json!({
            "instanceId": instance_id,
            "seat": "north",
            "sourceInstanceId": instance_id,
        })
    );
    let end_state = state(&session);
    let survivor = unit(&end_state, &instance_id);
    assert_eq!(survivor["tapped"], false);
    assert_eq!(survivor["warded"], true);
    assert_eq!(survivor["damage"], 0);
    assert_exact_replay(&session);
    disabled_malakhim_should_stay_tapped_while_damage_resets();
}

fn disabled_malakhim_should_stay_tapped_while_damage_resets() {
    let (mut session, instance_id) = malakhim_checkpoint(&minion(json!({
        "attack": 4,
        "defense": 4,
        "untapsAtEndOfControllerTurn": true,
    })));
    advance_to_tapped_malakhim(&mut session, &instance_id);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-damager"
            && descriptor["cell"] == "C3"
            && descriptor["genesisDamageTarget"]["instanceId"] == instance_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == instance_id
    });
    let before = state(&session);
    assert_eq!(unit(&before, &instance_id)["damage"], 2);
    assert_eq!(
        unit(&before, &instance_id)["disableEffects"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(event_types(&receipt), ["turn-ended", "turn-started"]);
    let after = state(&session);
    let survivor = unit(&after, &instance_id);
    assert_eq!(survivor["tapped"], true);
    assert_eq!(survivor["damage"], 0);
    assert_eq!(survivor["disableEffects"].as_array().map(Vec::len), Some(1));
    assert_exact_replay(&session);
}

fn ignited_cards(ignited: &Value, north_damager: &Value, south_caster: &Value) -> Value {
    json!({
        "north-avatar": avatar(),
        "north-ignited": ignited,
        "north-damager": north_damager,
        "north-filler": minion(json!({})),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-caster": south_caster,
        "south-freeze": magic(json!({ "disableTargetNearbyMinionUntilNextTurn": true })),
        "south-filler": minion(json!({})),
        "south-site": site(),
    })
}

#[test]
fn ignited_should_die_before_turn_cleanup_unless_disabled() {
    let ignited = minion(json!({
        "attack": 3,
        "charge": true,
        "defense": 3,
        "diesAtEndOfControllerTurn": true,
    }));
    let mut invalid = ignited_cards(&ignited, &minion(json!({})), &minion(json!({})));
    invalid["north-ignited"]["diesAtEndOfControllerTurn"] = json!(false);
    assert!(
        Session::new(&manifest(
            109,
            &invalid,
            &["north-ignited", "north-damager", "north-filler"],
            &["south-caster", "south-freeze", "south-filler"],
            6,
        ))
        .is_err()
    );

    let cards = ignited_cards(&ignited, &minion(json!({})), &minion(json!({})));
    let manifest = manifest(
        109,
        &cards,
        &["north-ignited", "north-damager", "north-filler"],
        &["south-caster", "south-freeze", "south-filler"],
        6,
    );
    let mut session = Session::new(&manifest).expect("valid Ignited scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-ignited"
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("Ignited identity")
        .to_owned();
    let version_before = state(&session)["stateVersion"]
        .as_u64()
        .expect("state version");
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_types(&receipt),
        ["minion-died", "turn-ended", "turn-started"]
    );
    assert_eq!(
        receipt.events[0].payload,
        json!({
            "cardId": "north-ignited",
            "instanceId": instance_id,
            "owner": "north",
        })
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["activeSeat"], "south");
    assert_eq!(after["stateVersion"], version_before + 1);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id)
    );
    assert_exact_replay(&session);
    disabled_ignited_should_survive_cleanup_reset_damage_and_expire_disable();
}

fn disabled_ignited_should_survive_cleanup_reset_damage_and_expire_disable() {
    let cards = ignited_cards(
        &minion(json!({
            "attack": 3,
            "charge": true,
            "defense": 3,
            "diesAtEndOfControllerTurn": true,
            "genesisDisableSelfUntilDamaged": true,
        })),
        &minion(json!({ "genesisMayDamageTargetAdjacentUnit": 2 })),
        &minion(json!({ "spellcaster": true, "summonToAnySite": true })),
    );
    let manifest = manifest(
        110,
        &cards,
        &["north-ignited", "north-damager", "north-filler"],
        &["south-caster", "south-freeze", "south-filler"],
        6,
    );
    let mut session = Session::new(&manifest).expect("valid disabled Ignited scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-ignited"
    });
    let ignited_id = summon["cardInstanceId"]
        .as_str()
        .expect("Ignited identity")
        .to_owned();
    let (_, first_end) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(event_types(&first_end), ["turn-ended", "turn-started"]);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (caster_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-caster"
            && descriptor["cell"] == "C4"
    });
    let caster_id = caster_summon["cardInstanceId"]
        .as_str()
        .expect("caster identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-freeze"
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == ignited_id
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-damager"
            && descriptor["cell"] == "C4"
            && descriptor["genesisDamageTarget"]["instanceId"] == ignited_id
    });
    let damaged = state(&session);
    assert_eq!(unit(&damaged, &ignited_id)["damage"], 2);
    assert_eq!(
        unit(&damaged, &ignited_id)["disableEffects"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_types(&receipt),
        ["turn-ended", "minion-disable-expired", "turn-started"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], ignited_id);
    let after = state(&session);
    let survivor = unit(&after, &ignited_id);
    assert_eq!(survivor["damage"], 0);
    assert!(survivor["disableEffects"].is_null());
    assert_exact_replay(&session);
}

fn end_turn_deathrite_cards(terminal: bool) -> Value {
    let ignited = if terminal {
        minion(json!({
            "deathriteDrawSite": true,
            "diesAtEndOfControllerTurn": true,
        }))
    } else {
        minion(json!({
            "deathriteDamageEachUnitHere": 1,
            "diesAtEndOfControllerTurn": true,
            "defense": 2,
        }))
    };
    let drawrite_a = if terminal {
        minion(json!({
            "defense": 2,
            "genesisMayDamageTargetAdjacentUnit": 2,
        }))
    } else {
        minion(json!({ "deathriteDrawSite": true, "defense": 1 }))
    };
    let drawrite_b = if terminal {
        minion(json!({ "defense": 3 }))
    } else {
        minion(json!({
            "deathriteDrawSite": true,
            "diesAtEndOfControllerTurn": true,
            "defense": 1,
        }))
    };
    json!({
        "north-avatar": avatar(),
        "north-ignited": ignited,
        "north-drawrite-a": drawrite_a,
        "north-drawrite-b": drawrite_b,
        "north-site": site(),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(),
    })
}

#[test]
fn end_turn_deathrite_order_should_resume_the_interrupted_turn_transition() {
    let manifest = manifest(
        111,
        &end_turn_deathrite_cards(false),
        &["north-ignited", "north-drawrite-a", "north-drawrite-b"],
        &["south-filler"; 3],
        6,
    );
    let mut session = Session::new(&manifest).expect("valid end-turn continuation scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let mut remaining_id = None;
    for card_id in ["north-ignited", "north-drawrite-a", "north-drawrite-b"] {
        let (summon, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion" && descriptor["cardId"] == card_id
        });
        if card_id == "north-drawrite-b" {
            remaining_id = Some(summon["cardInstanceId"].clone());
        }
    }
    let version_before = state(&session)["stateVersion"]
        .as_u64()
        .expect("state version");
    let (_, trigger) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let paused = state(&session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["stateVersion"], version_before + 1);
    let remaining_id = remaining_id.expect("second triggered death identity");
    assert_eq!(
        paused["pendingDeathrites"]["continuation"],
        json!({
            "kind": "end-turn",
            "remainingInstanceIds": [remaining_id],
            "seat": "north",
        })
    );
    assert!(
        event_types(&trigger)
            .iter()
            .all(|kind| *kind != "turn-ended" && *kind != "turn-started")
    );
    assert_checkpoint_round_trip(&session);
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
    });
    let types = event_types(&resolved);
    let last_deathrite = types
        .iter()
        .rposition(|kind| *kind == "site-drawn")
        .expect("resolved site-draw Deathrites");
    let first_cemetery = types
        .iter()
        .position(|kind| *kind == "minion-died")
        .expect("simultaneous cemetery entry");
    let turn_ended = types
        .iter()
        .position(|kind| *kind == "turn-ended")
        .expect("resumed turn transition");
    assert!(last_deathrite < first_cemetery && first_cemetery < turn_ended);
    assert_eq!(&types[turn_ended..], ["turn-ended", "turn-started"]);
    let resumed = state(&session);
    assert_eq!(resumed["phase"], "draw");
    assert_eq!(resumed["activeSeat"], "south");
    assert_eq!(resumed["stateVersion"], version_before + 2);
    assert_exact_replay(&session);
}

#[test]
fn terminal_end_turn_deathrite_should_not_start_another_turn() {
    let manifest = manifest(
        112,
        &end_turn_deathrite_cards(true),
        &["north-ignited", "north-drawrite-a", "north-drawrite-b"],
        &["south-filler"; 3],
        3,
    );
    let mut session = Session::new(&manifest).expect("valid terminal end-turn scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (survivor_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-drawrite-b"
    });
    let survivor_id = survivor_summon["cardInstanceId"]
        .as_str()
        .expect("survivor identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-drawrite-a"
            && descriptor["genesisDamageTarget"]["instanceId"] == survivor_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-ignited"
    });
    let before = state(&session);
    assert_eq!(unit(&before, &survivor_id)["damage"], 2);
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let types = event_types(&receipt);
    assert!(types.contains(&"game-ended"));
    assert!(!types.contains(&"turn-ended"));
    assert!(!types.contains(&"turn-started"));
    let terminal = state(&session);
    assert_eq!(terminal["phase"], "terminal");
    assert_eq!(terminal["terminal"]["reason"], "deck_empty");
    assert_eq!(terminal["activeSeat"], "north");
    assert_eq!(terminal["turnNumber"], before["turnNumber"]);
    assert_eq!(unit(&terminal, &survivor_id)["damage"], 2);
    assert_exact_replay(&session);
}
