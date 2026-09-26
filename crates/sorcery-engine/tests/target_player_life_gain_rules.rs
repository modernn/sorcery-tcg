//! Direct proofs for target-player life-gain Magic (RULE-CATALOG-0651–0652,
//! RULE-CATALOG-1058, RULE-CATALOG-2223–2228).
//!
//! Target-player life-gain Magic pays, offers only both Avatars, never offers
//! a minion, and heals the chosen Avatar through the shared printed-life cap.
//! Healing an Avatar at Death's Door is a paid no-op: life stays at zero and
//! no healing event is emitted. While Deathrites wait for ordering, target-player
//! life-gain Magic stays withheld until the chain drains.

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
        "genesisLoseControllerLife": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn life_gain_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "targetPlayerGainsLife": 2,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn lash_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 2,
        "manaCost": 0,
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

fn life_gain_manifest(seed: u32, north_life: u8, south_spell: &str) -> String {
    let mut cards = json!({
        "north-avatar": avatar(north_life),
        "north-life-gain": life_gain_spell(),
        "north-site": site(),
        "south-avatar": avatar(20),
        "south-site": site(),
    });
    cards[south_spell] = if south_spell == "south-lash" {
        lash_spell()
    } else {
        minion()
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "target-player-life-gain" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-target-player-life-gain-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-life-gain"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec![south_spell; 6],
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
    let mut session = Session::new(encoded).expect("valid life-gain session");
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

fn life_gain_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("life-gain actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-life-gain"
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

fn north_hand_has_life_gain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-life-gain"))
}

fn seed_with(north_life: u8, south_spell: &str, start: u32) -> String {
    (start..start + 256)
        .map(|seed| life_gain_manifest(seed, north_life, south_spell))
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_hand_has_life_gain(&state(&preview)))
        })
        .expect("bounded seed with life-gain Magic in the opening hand")
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

fn deathrite_life_gain_manifest(seed: u32) -> String {
    let fixture = "target-player-life-gain-deathrite-withheld";
    let south_spellbook = ["south-loss"]
        .into_iter()
        .chain(std::iter::repeat_n("south-minion", 5))
        .collect::<Vec<_>>();
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-life-gain": life_gain_spell(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-loss": minion(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-life-gain",
                    "north-rain",
                    "north-rain",
                    "north-life-gain",
                    "north-rain",
                    "north-life-gain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
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

fn north_has_life_gain_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-life-gain", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteLifeGainSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_life_gain_magic(
    encoded: &str,
) -> Option<PendingDeathriteLifeGainSetup> {
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-loss"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if state(&session)["players"]["south"]["avatar"]["life"] != 18 {
        return None;
    }
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
    if !north_has_life_gain_and_rain(&state(&session)) {
        return None;
    }
    if life_gain_targets(&session).is_empty() {
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
    Some(PendingDeathriteLifeGainSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_life_gain_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_life_gain_manifest)
        .find(|candidate| try_pending_deathrite_with_life_gain_magic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with life-gain Magic in hand")
}

#[test]
fn rule_catalog_0651_target_player_life_gain_heals_an_enemy_avatar_to_its_printed_cap() {
    let encoded = seed_with(20, "south-loss", 651);
    let mut session = opening_main(&encoded);
    let minion_id = stage_south_minion_at_c1(&mut session);
    let before = state(&session);
    assert_eq!(before["players"]["south"]["avatar"]["life"], 18);
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert_eq!(
        life_gain_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );
    assert!(
        session
            .legal_actions()
            .expect("life-gain actions")
            .into_iter()
            .filter(|action| action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-life-gain")
            .all(|action| {
                action.descriptor["target"]["kind"] == "avatar"
                    && action.descriptor["target"]["instanceId"] != minion_id
            })
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-gain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "avatar-healed", "magic-resolved"]
    );
    let healed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "avatar-healed")
        .expect("life-gain event");
    assert_eq!(healed.payload["amount"], 2);
    assert_eq!(healed.payload["attemptedAmount"], 2);
    assert_eq!(healed.payload["life"], 20);
    assert_eq!(healed.payload["seat"], "south");
    assert_eq!(
        healed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "damage-dealt"
                || event.event_type == "avatar-life-lost")
    );

    let after = state(&session);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert!(realm_unit(&after, &minion_id).is_some());
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("life-gain checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized life-gain");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed life-gain");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed life-gain session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0652_target_player_life_gain_cannot_leave_deaths_door() {
    let encoded = seed_with(2, "south-lash", 652);
    let mut session = opening_main(&encoded);
    let north_avatar = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (_, damage) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-lash"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
            && descriptor["target"]["instanceId"] == north_avatar
    });
    assert!(event_types(&damage).contains(&"avatar-reached-deaths-door"));
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 0);
    let death_door_turn = state(&session)["players"]["north"]["avatar"]["deathDoorTurn"].clone();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-gain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
            && descriptor["target"]["instanceId"] == north_avatar
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-healed"
                || event.event_type == "damage-dealt"
                || event.event_type == "death-blow"
                || event.event_type == "game-ended")
    );

    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(
        after["players"]["north"]["avatar"]["deathDoorTurn"],
        death_door_turn
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1058_target_player_life_gain_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_life_gain_seed_with(1058);
    let mut setup = try_pending_deathrite_with_life_gain_magic(&encoded)
        .expect("complete target-player life-gain Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["players"]["south"]["avatar"]["life"], 18);
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
    assert!(life_gain_targets(session).is_empty());

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
    assert_eq!(resumed["players"]["south"]["avatar"]["life"], 18);
    let south_avatar = resumed["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert_eq!(
        life_gain_targets(session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-gain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "avatar-healed", "magic-resolved"]
    );
    assert_eq!(state(session)["players"]["south"]["avatar"]["life"], 20);
    assert_exact_replay(session);
}

fn life_gain_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "target-player-life-gain-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-target-player-life-gain-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-life-gain": life_gain_spell(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-loss": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-life-gain"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-loss"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn life_gain_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-life-gain")
                .count()
        })
        .unwrap_or_default()
}

fn seed_for_one_genesis(start: u32, required_gains: usize) -> String {
    (start..start + 8192)
        .chain(651..651 + 8192)
        .find_map(|seed| {
            let encoded = life_gain_supplemental_manifest(seed);
            let session = try_south_genesis_then_north_main(&encoded, 1)?;
            (life_gain_spells_in_hand(&state(&session)) >= required_gains).then_some(encoded)
        })
        .expect("bounded seed reaching target-player life-gain after south genesis")
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .ok()
        .is_some_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
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

fn try_end_turn_if_offered(session: &mut Session) -> Option<()> {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        try_accept_where(session, |descriptor| descriptor["kind"] == "decline-attack")?;
    }
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    Some(())
}

fn try_draw_any(session: &mut Session) -> Option<(Value, Receipt)> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })
}

fn try_south_genesis_then_north_main(encoded: &str, summons: usize) -> Option<Session> {
    let mut session = opening_main(encoded);
    if life_gain_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_end_turn_if_offered(&mut session)?;
    try_draw_any(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    for _ in 0..summons {
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-loss"
                && descriptor["cell"] == "C1"
                && descriptor["region"].is_null()
        })?;
    }
    let expected_life = 20 - i64::try_from(summons).ok()? * 2;
    if state(&session)["players"]["south"]["avatar"]["life"] != expected_life {
        return None;
    }
    try_end_turn_if_offered(&mut session)?;
    try_draw_any(&mut session)?;
    Some(session)
}

fn cast_life_gain_on(session: &mut Session, seat: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-life-gain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == seat
    });
    receipt
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    });
}

fn try_second_life_gain_enemy_arrival_prefix(encoded: &str) -> Option<Session> {
    let mut session = try_south_genesis_then_north_main(encoded, 1)?;
    if life_gain_spells_in_hand(&state(&session)) < 2 {
        return None;
    }
    let first = cast_life_gain_on(&mut session, "south");
    if !event_types(&first).contains(&"avatar-healed") {
        return None;
    }
    if state(&session)["players"]["south"]["avatar"]["life"] != 20 {
        return None;
    }
    try_end_turn_if_offered(&mut session)?;
    try_draw_any(&mut session)?;
    let _ = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-loss"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if state(&session)["players"]["south"]["avatar"]["life"] != 18 {
        return None;
    }
    try_end_turn_if_offered(&mut session)?;
    try_draw_any(&mut session)?;
    (life_gain_spells_in_hand(&state(&session)) >= 1).then_some(session)
}

fn seed_for_second_life_gain_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(651..651 + 8192)
        .find_map(|seed| {
            let encoded = life_gain_supplemental_manifest(seed);
            try_second_life_gain_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second target-player life-gain enemy-arrival setup")
}

fn seed_for_two_genesis_heals(start: u32) -> String {
    (start..start + 8192)
        .chain(651..651 + 8192)
        .find_map(|seed| {
            let encoded = life_gain_supplemental_manifest(seed);
            let session = try_south_genesis_then_north_main(&encoded, 2)?;
            (life_gain_spells_in_hand(&state(&session)) >= 2).then_some(encoded)
        })
        .expect("bounded seed with two target-player life-gain casts after stacked genesis loss")
}

#[test]
fn rule_catalog_2223_healed_life_stays_after_turns_pass() {
    let encoded = seed_for_one_genesis(2223, 1);
    let mut session = try_south_genesis_then_north_main(&encoded, 1)
        .expect("target-player life-gain persistence setup");
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 18);
    let receipt = cast_life_gain_on(&mut session, "south");
    assert!(event_types(&receipt).contains(&"avatar-healed"));
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2224_second_life_gain_at_the_printed_cap_is_a_paid_noop() {
    let encoded = seed_for_one_genesis(2224, 2);
    let mut session = try_south_genesis_then_north_main(&encoded, 1)
        .expect("target-player life-gain cap-repeat setup");
    assert!(life_gain_spells_in_hand(&state(&session)) >= 2);
    let first = cast_life_gain_on(&mut session, "south");
    assert!(event_types(&first).contains(&"avatar-healed"));
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert!(life_gain_spells_in_hand(&state(&session)) >= 1);
    let second = cast_life_gain_on(&mut session, "south");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"avatar-healed"));
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2225_second_life_gain_heals_newly_lost_life_after_enemy_site_placement() {
    let encoded = seed_for_second_life_gain_enemy_arrival(2225);
    let mut session = try_second_life_gain_enemy_arrival_prefix(&encoded)
        .expect("second target-player life-gain enemy-arrival prefix");
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 18);
    let receipt = cast_life_gain_on(&mut session, "south");
    assert!(event_types(&receipt).contains(&"avatar-healed"));
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2226_target_player_life_gain_offers_both_avatars() {
    let encoded = seed_for_one_genesis(2226, 1);
    let mut session = try_south_genesis_then_north_main(&encoded, 1)
        .expect("target-player life-gain dual-avatar setup");
    assert_eq!(
        life_gain_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );
    let receipt = cast_life_gain_on(&mut session, "south");
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "avatar-healed", "magic-resolved"]
    );
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2227_target_player_life_gain_leaves_the_other_avatar_untouched() {
    let encoded = seed_for_one_genesis(2227, 1);
    let mut session = try_south_genesis_then_north_main(&encoded, 1)
        .expect("target-player life-gain other-avatar setup");
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 18);
    let receipt = cast_life_gain_on(&mut session, "south");
    assert!(event_types(&receipt).contains(&"avatar-healed"));
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-healed" && event.payload["seat"] == "north")
    );
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2228_second_life_gain_heals_remaining_life_after_stacked_genesis() {
    let encoded = seed_for_two_genesis_heals(2228);
    let mut session = try_south_genesis_then_north_main(&encoded, 2)
        .expect("target-player life-gain stacked-genesis setup");
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 16);
    assert!(life_gain_spells_in_hand(&state(&session)) >= 2);
    let first = cast_life_gain_on(&mut session, "south");
    assert!(event_types(&first).contains(&"avatar-healed"));
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 18);
    let second = cast_life_gain_on(&mut session, "south");
    assert!(event_types(&second).contains(&"avatar-healed"));
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}
