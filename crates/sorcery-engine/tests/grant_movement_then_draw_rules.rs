//! Direct proofs for grant-+1-movement-this-turn then draw-spell Magic (RULE-CATALOG-0535–0536,
//! RULE-CATALOG-1066, RULE-CATALOG-1613–1618).
//!
//! Ordinary Magic can give an ally +1 movement this turn and then draw one
//! spell. Enemy units are not offered. The movement mark uses a shared
//! this-turn source list, expires at End Phase, and lets a 1-step minion
//! reach a cell two steps away. While Deathrites wait for ordering, the
//! grant stays withheld until the chain drains.

#[path = "common/mod.rs"]
mod common;
use common::modifier_sources;

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn grounded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn printed_mover() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "movementBonus": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn gift() -> Value {
    json!({
        "cardType": "magic",
        "grantMovementOneToAllyThisTurnThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
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

fn gift_manifest_with_ally(seed: u32, north_ally: &Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-movement-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-movement-then-draw-v1",
        },
        "cards": {
            "north-ally": north_ally,
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": grounded(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-gift",
                    "north-gift",
                    "north-gift",
                    "north-gift",
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

fn gift_manifest(seed: u32) -> String {
    gift_manifest_with_ally(seed, &grounded())
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
    let mut session = Session::new(encoded).expect("valid grant-movement-then-draw session");
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn avatar_instance_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar instance identity")
        .to_owned()
}

fn gift_ally_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("gift actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-gift"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn can_move_to(session: &Session, unit_id: &str, cell: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .any(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == unit_id
                && action.descriptor["to"]["cell"] == cell
        })
}

fn seed_with_ally_and_gift() -> String {
    (535..535 + 256)
        .map(gift_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic in the opening hand")
}

fn opening_with_ally(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    (session, ally_id)
}

fn south_summons_at_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
}

fn lay_two_step_sites(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn through_south_pass_to_north_main(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn grant_movement(session: &mut Session, ally_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    })
}

fn expire_grant_movement(session: &mut Session, ally_id: &str, grant_source: &str) {
    let (_, ended) = accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "movement-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == grant_source
    }));
    assert!(
        modifier_sources(unit(&state(session), ally_id), "movement")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn seed_with_ally_card(north_ally: &Value, start: u32) -> String {
    (start..start + 256)
        .map(|seed| gift_manifest_with_ally(seed, north_ally))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic in the opening hand")
}

fn two_step_reach_setup(north_ally: &Value, start: u32) -> (Session, String) {
    let encoded = seed_with_ally_card(north_ally, start);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let _enemy_id = south_summons_at_c1(&mut session);
    lay_two_step_sites(&mut session);
    (session, ally_id)
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
fn rule_catalog_0535_movement_grant_offers_allies_then_draws_a_spell() {
    let encoded = seed_with_ally_and_gift();
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let enemy_id = south_summons_at_c1(&mut session);
    let before = state(&session);
    let north_avatar = avatar_instance_id(&before, "north");
    let south_avatar = avatar_instance_id(&before, "south");
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let allies = gift_ally_ids(&session);
    assert!(allies.contains(&ally_id));
    assert!(allies.contains(&north_avatar));
    assert!(!allies.contains(&south_avatar));
    assert!(!allies.contains(&enemy_id));

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "movement-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    assert_eq!(granted.events[1].payload["amount"], 1);
    let after = state(&session);
    assert_eq!(
        modifier_sources(unit(&after, &ally_id), "movement")[0],
        granted.events[0].payload["instanceId"]
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert!(
        !serde_json::to_string(&south_view)
            .expect("view JSON")
            .contains(&library_top)
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "movement-expired" && event.payload["instanceId"] == ally_id
    }));
    assert!(
        modifier_sources(unit(&state(&session), &ally_id), "movement")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0536_granted_movement_then_draw_reaches_a_two_step_cell() {
    let encoded = seed_with_ally_and_gift();
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let _enemy_id = south_summons_at_c1(&mut session);
    lay_two_step_sites(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["summoningSickness"], false);
    assert!(
        can_move_to(&session, &ally_id, "C3"),
        "a 1-step minion can reach an adjacent site"
    );
    assert!(
        !can_move_to(&session, &ally_id, "C2"),
        "a 1-step minion cannot reach a cell two steps away"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert!(
        can_move_to(&session, &ally_id, "C2"),
        "granted +1 movement lets the minion reach a cell two steps away"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1613_printed_movement_bonus_reaches_two_step_cell_without_grant() {
    let (session, ally_id) = two_step_reach_setup(&printed_mover(), 1613);
    assert_eq!(unit(&state(&session), &ally_id)["summoningSickness"], false);
    assert!(can_move_to(&session, &ally_id, "C3"));
    assert!(
        can_move_to(&session, &ally_id, "C2"),
        "printed +1 movement reaches a cell two steps away"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1614_granted_movement_reaches_two_step_cell_before_end_of_turn() {
    let (mut session, ally_id) = two_step_reach_setup(&grounded(), 1614);
    assert!(!can_move_to(&session, &ally_id, "C2"));
    let (descriptor, receipt) = grant_movement(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "movement-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(
        modifier_sources(unit(&state(&session), &ally_id), "movement"),
        json!([descriptor["cardInstanceId"]])
    );
    assert!(can_move_to(&session, &ally_id, "C2"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1615_granted_movement_expires_before_ally_reaches_two_step_cell_on_later_turn() {
    let (mut session, ally_id) = two_step_reach_setup(&grounded(), 1615);
    let (descriptor, _) = grant_movement(&mut session, &ally_id);
    expire_grant_movement(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    through_south_pass_to_north_main(&mut session);
    assert!(!can_move_to(&session, &ally_id, "C2"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1616_printed_movement_bonus_still_reaches_after_grant_expires_on_later_turn() {
    let (mut session, ally_id) = two_step_reach_setup(&printed_mover(), 1616);
    let (descriptor, _) = grant_movement(&mut session, &ally_id);
    expire_grant_movement(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    through_south_pass_to_north_main(&mut session);
    assert!(can_move_to(&session, &ally_id, "C2"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1617_printed_and_granted_movement_compose_while_grant_is_active() {
    let (mut session, ally_id) = two_step_reach_setup(&printed_mover(), 1617);
    let (descriptor, receipt) = grant_movement(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "movement-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(
        modifier_sources(unit(&state(&session), &ally_id), "movement"),
        json!([descriptor["cardInstanceId"]])
    );
    assert!(can_move_to(&session, &ally_id, "C2"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1618_printed_movement_bonus_outlasts_expired_grant_while_plain_ally_cannot_reach() {
    let (mut session, ally_id) = two_step_reach_setup(&printed_mover(), 1618);
    let (descriptor, _) = grant_movement(&mut session, &ally_id);
    expire_grant_movement(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );

    let (mut plain, plain_ally) = two_step_reach_setup(&grounded(), 1618);
    let (plain_descriptor, _) = grant_movement(&mut plain, &plain_ally);
    expire_grant_movement(
        &mut plain,
        &plain_ally,
        plain_descriptor["cardInstanceId"]
            .as_str()
            .expect("grant source"),
    );
    through_south_pass_to_north_main(&mut plain);
    assert!(!can_move_to(&plain, &plain_ally, "C2"));

    through_south_pass_to_north_main(&mut session);
    assert!(can_move_to(&session, &ally_id, "C2"));
    assert_exact_replay(&session);
}

fn deathrite_grant_movement_manifest(seed: u32) -> String {
    let fixture = "grant-movement-then-draw-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-gift",
                    "north-rain",
                    "north-rain",
                    "north-gift",
                    "north-gift",
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

fn north_has_gift_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-gift", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteGrantMovementSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_allied_minion(
    encoded: &str,
) -> Option<PendingDeathriteGrantMovementSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_gift_and_rain(&state(&session)) {
        return None;
    }
    if !gift_ally_ids(&session).contains(&ally_id) {
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
    Some(PendingDeathriteGrantMovementSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_grant_movement_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_grant_movement_manifest)
        .find(|candidate| try_pending_deathrite_with_allied_minion(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with grant-movement Magic in hand")
}

#[test]
fn rule_catalog_1066_grant_movement_then_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_grant_movement_seed_with(1066);
    let mut setup = try_pending_deathrite_with_allied_minion(&encoded)
        .expect("complete grant-movement Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
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
    assert!(unit(&paused, &ally_id).is_object());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(gift_ally_ids(session).is_empty());

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
    assert!(unit(&resumed, &ally_id).is_object());
    assert!(gift_ally_ids(session).contains(&ally_id));

    let library_top = resumed["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let hand_before = resumed["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();

    let (_, granted) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "movement-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["amount"], 1);
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    let after = state(session);
    assert_eq!(
        modifier_sources(unit(&after, &ally_id), "movement")[0],
        granted.events[0].payload["instanceId"]
    );
    let hand_after = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand after draw");
    assert_eq!(hand_after.len(), hand_before);
    assert!(
        hand_after
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(session);
}
