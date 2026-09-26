//! Direct proofs for target-player life-loss Magic (RULE-CATALOG-0649–0650,
//! RULE-CATALOG-1057, RULE-CATALOG-2213–2218).
//!
//! Target-player life-loss Magic pays, offers only both Avatars, never offers
//! a minion, and reduces the chosen Avatar's life without dealing damage. Life
//! loss that reaches zero hits Death's Door; further loss against that Avatar
//! is a no-op and is never a death blow. While Deathrites wait for ordering,
//! target-player life-loss Magic stays withheld until the chain drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn life_loss_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "targetPlayerLosesLife": 2,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn life_loss_manifest(seed: u32, north_life: u8, south_life: u8) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "target-player-life-loss" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-target-player-life-loss-v1",
        },
        "cards": {
            "north-avatar": avatar(north_life),
            "north-life-loss": life_loss_spell(),
            "north-site": site(),
            "south-avatar": avatar(south_life),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-life-loss"; 6],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid life-loss session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn stage_south_minion_at_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned()
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

fn realm_unit<'a>(value: &'a Value, instance_id: &str) -> Option<&'a Value> {
    value["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn life_loss_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("life-loss actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-life-loss"
        })
        .filter_map(|action| {
            let target = action.descriptor.get("target")?;
            Some((
                target["kind"].as_str()?.to_owned(),
                target["seat"].as_str()?.to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn deathrite_life_loss_manifest(seed: u32) -> String {
    let fixture = "target-player-life-loss-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-life-loss": life_loss_spell(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-life-loss",
                    "north-rain",
                    "north-rain",
                    "north-life-loss",
                    "north-rain",
                    "north-life-loss",
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

fn north_has_life_loss_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-life-loss", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteLifeLossSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_life_loss_magic(
    encoded: &str,
) -> Option<PendingDeathriteLifeLossSetup> {
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
    if !north_has_life_loss_and_rain(&state(&session)) {
        return None;
    }
    if life_loss_targets(&session).is_empty() {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteLifeLossSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_life_loss_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_life_loss_manifest)
        .find(|candidate| try_pending_deathrite_with_life_loss_magic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with life-loss Magic in hand")
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
fn rule_catalog_0649_target_player_life_loss_reduces_avatar_life_without_dealing_damage() {
    let encoded = life_loss_manifest(649, 20, 20);
    let mut session = opening_main(&encoded);
    let minion_id = stage_south_minion_at_c1(&mut session);
    let before = state(&session);
    let north_avatar = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert_eq!(
        life_loss_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );
    assert!(
        session
            .legal_actions()
            .expect("life-loss actions")
            .into_iter()
            .filter(|action| action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-life-loss")
            .all(|action| {
                action.descriptor["target"]["kind"] == "avatar"
                    && action.descriptor["target"]["instanceId"] != minion_id
            })
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-loss"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "avatar-life-lost", "magic-resolved"]
    );
    let lost = receipt
        .events
        .iter()
        .find(|event| event.event_type == "avatar-life-lost")
        .expect("life-loss event");
    assert_eq!(lost.payload["amount"], 2);
    assert_eq!(lost.payload["life"], 18);
    assert_eq!(lost.payload["seat"], "south");
    assert_eq!(
        lost.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "damage-dealt"
                || event.event_type == "death-blow"
                || event.event_type == "avatar-reached-deaths-door")
    );

    let after = state(&session);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 18);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(
        after["players"]["south"]["avatar"]["card"]["instanceId"],
        south_avatar
    );
    assert_eq!(
        after["players"]["north"]["avatar"]["card"]["instanceId"],
        north_avatar
    );
    assert!(realm_unit(&after, &minion_id).is_some());
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("life-loss checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized life-loss");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed life-loss");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed life-loss session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0650_target_player_life_loss_reaches_deaths_door_without_a_death_blow() {
    let encoded = life_loss_manifest(650, 2, 20);
    let mut session = opening_main(&encoded);
    let north_avatar = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();

    let (descriptor, first) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-loss"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
            && descriptor["target"]["instanceId"] == north_avatar
    });
    assert_eq!(
        event_types(&first),
        [
            "magic-cast",
            "avatar-life-lost",
            "avatar-reached-deaths-door",
            "magic-resolved"
        ]
    );
    let lost = first
        .events
        .iter()
        .find(|event| event.event_type == "avatar-life-lost")
        .expect("life-loss event");
    assert_eq!(lost.payload["amount"], 2);
    assert_eq!(lost.payload["life"], 0);
    assert_eq!(lost.payload["seat"], "north");
    assert_eq!(
        lost.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    let door = first
        .events
        .iter()
        .find(|event| event.event_type == "avatar-reached-deaths-door")
        .expect("Death's Door event");
    assert_eq!(door.payload["seat"], "north");
    assert_eq!(
        door.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !first
            .events
            .iter()
            .any(|event| event.event_type == "damage-dealt" || event.event_type == "death-blow")
    );

    let after_first = state(&session);
    assert_eq!(after_first["players"]["north"]["avatar"]["life"], 0);
    let death_door_turn = after_first["players"]["north"]["avatar"]["deathDoorTurn"].clone();
    assert!(!death_door_turn.is_null());
    assert_eq!(after_first["terminal"]["status"], "active");

    let (_, second) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-loss"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
            && descriptor["target"]["instanceId"] == north_avatar
    });
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(
        !second
            .events
            .iter()
            .any(|event| event.event_type == "avatar-life-lost"
                || event.event_type == "avatar-reached-deaths-door"
                || event.event_type == "damage-dealt"
                || event.event_type == "death-blow"
                || event.event_type == "game-ended")
    );

    let after_second = state(&session);
    assert_eq!(after_second["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(
        after_second["players"]["north"]["avatar"]["deathDoorTurn"],
        death_door_turn
    );
    assert_eq!(after_second["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1057_target_player_life_loss_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_life_loss_seed_with(1057);
    let mut setup = try_pending_deathrite_with_life_loss_magic(&encoded)
        .expect("complete target-player life-loss Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(life_loss_targets(session).is_empty());

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    let south_avatar = resumed["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert_eq!(
        life_loss_targets(session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );
    assert_eq!(resumed["players"]["south"]["avatar"]["life"], 20);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-loss"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "avatar-life-lost", "magic-resolved"]
    );
    let lost = receipt
        .events
        .iter()
        .find(|event| event.event_type == "avatar-life-lost")
        .expect("life-loss event");
    assert_eq!(lost.payload["amount"], 2);
    assert_eq!(lost.payload["life"], 18);
    assert_eq!(lost.payload["seat"], "south");
    assert_eq!(state(session)["players"]["south"]["avatar"]["life"], 18);
    assert_exact_replay(session);
}

fn life_loss_supplemental_manifest(seed: u32, north_life: u8, south_life: u8) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "target-player-life-loss-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-target-player-life-loss-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(north_life),
            "north-life-loss": life_loss_spell(),
            "north-site": site(),
            "south-avatar": avatar(south_life),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-life-loss"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

fn supplemental_seed_with_start(start: u32, north_life: u8, south_life: u8) -> String {
    (start..start + 2048)
        .chain(649..649 + 2048)
        .map(|seed| life_loss_supplemental_manifest(seed, north_life, south_life))
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-life-loss")
        })
        .expect("bounded seed with target-player life-loss Magic in the opening hand")
}

fn life_loss_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-life-loss")
                .count()
        })
        .unwrap_or_default()
}

fn avatar_life(snapshot: &Value, seat: &str) -> u64 {
    snapshot["players"][seat]["avatar"]["life"]
        .as_u64()
        .expect("avatar life")
}

fn site_instance_at(snapshot: &Value, cell: &str) -> String {
    snapshot["realm"]["sites"][cell]["instanceId"]
        .as_str()
        .expect("site at cell")
        .to_owned()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .is_ok_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn decline_attack_if_needed(session: &mut Session) {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

fn end_turn_if_offered(session: &mut Session) {
    decline_attack_if_needed(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn cast_life_loss_on(session: &mut Session, seat: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-loss"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == seat
    });
    receipt
}

fn try_second_life_loss_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first = cast_life_loss_on(&mut session, "south");
    if !event_types(&first).contains(&"avatar-life-lost") {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    let south_c1 = site_instance_at(&state(&session), "C1");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (life_loss_spells_in_hand(&state(&session)) >= 1).then_some((session, south_c1))
}

fn seed_for_second_life_loss_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(649..649 + 8192)
        .find_map(|seed| {
            let encoded = life_loss_supplemental_manifest(seed, 20, 20);
            opening_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-life-loss")
                .then_some(encoded)
                .and_then(|encoded| {
                    try_second_life_loss_enemy_arrival_prefix(&encoded).map(|_| encoded)
                })
        })
        .expect("bounded seed reaching second target-player life-loss enemy-arrival setup")
}

#[test]
fn rule_catalog_2213_life_loss_stays_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2213, 20, 20);
    let mut session = opening_main(&encoded);
    assert_eq!(avatar_life(&state(&session), "south"), 20);
    let first = cast_life_loss_on(&mut session, "south");
    assert!(event_types(&first).contains(&"avatar-life-lost"));
    assert_eq!(avatar_life(&state(&session), "south"), 18);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(avatar_life(&state(&session), "south"), 18);
    assert_eq!(avatar_life(&state(&session), "north"), 20);
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2214_second_life_loss_is_a_paid_noop_after_deaths_door() {
    let encoded = (2214..2214 + 8192)
        .chain(649..649 + 8192)
        .find_map(|seed| {
            let candidate = life_loss_supplemental_manifest(seed, 20, 2);
            if opening_spell_ids(&candidate)
                .iter()
                .filter(|card| *card == "north-life-loss")
                .count()
                < 2
            {
                return None;
            }
            let mut session = opening_main(&candidate);
            if life_loss_spells_in_hand(&state(&session)) < 2
                || avatar_life(&state(&session), "south") != 2
            {
                return None;
            }
            let first = cast_life_loss_on(&mut session, "south");
            if !event_types(&first).contains(&"avatar-reached-deaths-door") {
                return None;
            }
            (avatar_life(&state(&session), "south") == 0
                && life_loss_spells_in_hand(&state(&session)) >= 1)
                .then_some(candidate)
        })
        .expect("bounded seed with two target-player life-loss casts after Death's Door");
    let mut session = opening_main(&encoded);
    assert!(life_loss_spells_in_hand(&state(&session)) >= 2);
    let first = cast_life_loss_on(&mut session, "south");
    assert!(event_types(&first).contains(&"avatar-life-lost"));
    assert!(event_types(&first).contains(&"avatar-reached-deaths-door"));
    assert_eq!(avatar_life(&state(&session), "south"), 0);
    assert!(life_loss_spells_in_hand(&state(&session)) >= 1);
    let second = cast_life_loss_on(&mut session, "south");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"avatar-life-lost"));
    assert_eq!(avatar_life(&state(&session), "south"), 0);
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2215_second_life_loss_still_hits_after_enemy_site_placement() {
    let encoded = seed_for_second_life_loss_enemy_arrival(2215);
    let (mut session, south_c1) = try_second_life_loss_enemy_arrival_prefix(&encoded)
        .expect("second target-player life-loss enemy-arrival prefix");
    assert_eq!(avatar_life(&state(&session), "south"), 18);
    let receipt = cast_life_loss_on(&mut session, "south");
    assert!(event_types(&receipt).contains(&"avatar-life-lost"));
    assert_eq!(avatar_life(&state(&session), "south"), 16);
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["instanceId"],
        south_c1
    );
    assert_ne!(
        state(&session)["realm"]["sites"]["C1"]["rubble"],
        json!(true)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2216_target_player_life_loss_offers_both_avatars() {
    let encoded = supplemental_seed_with_start(2216, 20, 20);
    let session = opening_main(&encoded);
    assert_eq!(
        life_loss_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2217_life_loss_leaves_the_other_avatar_untouched() {
    let encoded = supplemental_seed_with_start(2217, 20, 20);
    let mut session = opening_main(&encoded);
    assert_eq!(avatar_life(&state(&session), "north"), 20);
    assert_eq!(avatar_life(&state(&session), "south"), 20);
    let receipt = cast_life_loss_on(&mut session, "south");
    assert!(event_types(&receipt).contains(&"avatar-life-lost"));
    assert_eq!(avatar_life(&state(&session), "north"), 20);
    assert_eq!(avatar_life(&state(&session), "south"), 18);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2218_second_life_loss_reduces_newly_remaining_life() {
    let encoded = supplemental_seed_with_start(2218, 20, 20);
    let mut session = opening_main(&encoded);
    assert!(life_loss_spells_in_hand(&state(&session)) >= 2);
    assert_eq!(avatar_life(&state(&session), "south"), 20);
    let first = cast_life_loss_on(&mut session, "south");
    assert!(event_types(&first).contains(&"avatar-life-lost"));
    assert_eq!(avatar_life(&state(&session), "south"), 18);
    let second = cast_life_loss_on(&mut session, "south");
    assert!(event_types(&second).contains(&"avatar-life-lost"));
    assert_eq!(avatar_life(&state(&session), "south"), 16);
    assert_eq!(avatar_life(&state(&session), "north"), 20);
    assert_exact_replay(&session);
}
