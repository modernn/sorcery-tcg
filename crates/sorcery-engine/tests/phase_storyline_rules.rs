//! Direct proofs for Phase and Storyline cleanup (RULE-CATALOG-0728,
//! RULE-CATALOG-0731, RULE-CATALOG-0732, RULE-CATALOG-0906,
//! RULE-CATALOG-2513–2518, RULE-CATALOG-2543–2548, RULE-CATALOG-2553–2558).
//!
//! End-turn cleanup resets both players' air-threshold cast counters, not just
//! the ending player. Extends the Sparkmage per-turn counter slice in 0149.
//! Supplemental 2513–2518 bind persistence across turns, repeat end-turn,
//! opponent-turn reset, both-player reset, partial counter tracking, and
//! post-reset cast accumulation.
//! Post-action settlement tails slot before `magic-resolved`, so terminal
//! `game-ended` follows magic completion rather than `magic-cast`.
//! Ordered terminal cleanup omits resolved Chain Magic from authoritative state.
//! Multiple start-of-controller-turn minion triggers resolve as separate Start
//! Phase actions in summon order before the Draw step.

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

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn life_giver(amount: u8) -> Value {
    json!({
        "atStartOfControllerTurnControllerGainsLife": amount,
        "attack": 0,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drain() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "targetPlayerLosesLife": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dual_start_turn_queue_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "phase-storyline-start-turn-queue" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-phase-storyline-start-turn-queue-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-first": life_giver(2),
            "north-second": life_giver(1),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-drain": drain(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-first",
                    "north-second",
                    "north-first",
                    "north-second",
                    "north-first",
                    "north-second",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-drain"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 906,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action in phase {} among {:?}",
                state(session)["phase"],
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            );
        });
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

fn unit_id(session: &Session, card_id: &str) -> Value {
    state(session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == card_id)
        .expect("expected unit")["instanceId"]
        .clone()
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

fn after_north_ready_to_start_turn_with_two_givers() -> Session {
    let mut session =
        Session::new(&dual_start_turn_queue_manifest()).expect("valid dual-trigger session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-first"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-second"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-drain"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

#[test]
fn rule_catalog_0728_end_turn_cleanup_resets_both_players_air_threshold_counts() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0728_end_turn_cleanup_resets_both_players_air_threshold_counts();
}

#[test]
fn rule_catalog_0731_post_action_terminal_event_should_follow_magic_completion() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0731_post_action_terminal_event_should_follow_magic_completion();
}

#[test]
fn rule_catalog_0732_ordered_terminal_cleanup_should_omit_resolved_chain_magic() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0732_ordered_terminal_cleanup_should_omit_resolved_chain_magic();
}

#[test]
fn rule_catalog_2553_resolved_chain_magic_stays_omitted_after_terminal_state_reserializes() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2553_resolved_chain_magic_stays_omitted_after_terminal_state_reserializes();
}

#[test]
fn rule_catalog_2554_terminal_cleanup_without_resolved_chain_magic_keeps_pending_chain_magic_absent()
 {
    sorcery_engine::game::catalog_proofs::rule_catalog_2554_terminal_cleanup_without_resolved_chain_magic_keeps_pending_chain_magic_absent();
}

#[test]
fn rule_catalog_2555_terminal_cleanup_omits_resolved_chain_magic_alongside_other_resolved_continuations()
 {
    sorcery_engine::game::catalog_proofs::rule_catalog_2555_terminal_cleanup_omits_resolved_chain_magic_alongside_other_resolved_continuations();
}

#[test]
fn rule_catalog_2556_terminal_cleanup_omits_resolved_chain_magic_after_multi_hop_staging() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2556_terminal_cleanup_omits_resolved_chain_magic_after_multi_hop_staging();
}

#[test]
fn rule_catalog_2557_terminal_cleanup_omits_resolved_chain_magic_while_other_continuations_stay_pending()
 {
    sorcery_engine::game::catalog_proofs::rule_catalog_2557_terminal_cleanup_omits_resolved_chain_magic_while_other_continuations_stay_pending();
}

#[test]
fn rule_catalog_2558_second_terminal_cleanup_still_omits_newly_resolved_chain_magic() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2558_second_terminal_cleanup_still_omits_newly_resolved_chain_magic();
}

#[test]
fn rule_catalog_0906_two_start_turn_minion_triggers_resolve_separately_before_draw() {
    let mut session = after_north_ready_to_start_turn_with_two_givers();
    let first_id = unit_id(&session, "north-first");
    let second_id = unit_id(&session, "north-second");
    let before = state(&session);
    assert_eq!(before["phase"], "start-turn");
    assert_eq!(before["activeSeat"], "north");
    assert_eq!(before["players"]["north"]["avatar"]["life"], 18);
    let queued_triggers: Vec<_> = session
        .legal_actions()
        .expect("start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    assert_eq!(queued_triggers.len(), 2);
    assert!(
        queued_triggers
            .iter()
            .any(|action| action.descriptor["sourceInstanceId"] == first_id)
    );
    assert!(
        queued_triggers
            .iter()
            .any(|action| action.descriptor["sourceInstanceId"] == second_id)
    );
    let (_, first_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == first_id
    });
    assert_eq!(
        first_receipt.events.len(),
        1,
        "first trigger resolves only its own life gain"
    );
    assert_eq!(first_receipt.events[0].event_type, "avatar-healed");
    assert_eq!(first_receipt.events[0].payload["amount"], 2);
    assert_eq!(
        first_receipt.events[0].payload["sourceInstanceId"],
        first_id
    );
    let mid = state(&session);
    assert_eq!(mid["phase"], "start-turn");
    assert_eq!(mid["players"]["north"]["avatar"]["life"], 20);
    let remaining: Vec<_> = session
        .legal_actions()
        .expect("remaining start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .collect();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].descriptor["sourceInstanceId"], second_id);
    let (_, second_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == second_id
    });
    assert!(
        second_receipt.events.is_empty(),
        "a capped start-turn heal emits no avatar-healed event once the Avatar is full"
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["terminal"]["status"], "active");
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

fn start_turn_minion(extra: &Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("minion facts")
        .extend(extra.as_object().expect("extra minion facts").clone());
    value
}

fn dual_start_turn_deathrite_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "dual-start-turn-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-dual-start-turn-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-first": start_turn_minion(&json!({
                "atStartOfControllerTurnControllerGainsLife": 2,
            })),
            "north-pulser": start_turn_minion(&json!({
                "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
            })),
            "north-second": start_turn_minion(&json!({
                "atStartOfControllerTurnControllerGainsLife": 1,
            })),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-deathrite": start_turn_minion(&json!({
                "deathriteDrawSite": true,
                "defense": 1,
                "summonToAnySite": true,
            })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-pulser",
                    "north-first",
                    "north-second",
                    "north-pulser",
                    "north-first",
                    "north-second",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

struct PendingDualStartTurnDeathriteSetup {
    deathrite_ids: [String; 2],
    first_id: String,
    second_id: String,
    session: Session,
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

fn try_pending_deathrite_during_dual_start_turn(
    encoded: &str,
) -> Option<PendingDualStartTurnDeathriteSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let pulser = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-first"
            && descriptor["cell"] == "C4"
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-second"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    let death1 = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let death2 = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    if state(&session)["phase"] != "start-turn" {
        return None;
    }
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let first_id = first.0["cardInstanceId"].as_str()?.to_owned();
    let second_id = second.0["cardInstanceId"].as_str()?.to_owned();
    let offered: Vec<_> = session
        .legal_actions()
        .ok()?
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    if !offered.contains(&pulser_id)
        || !offered.contains(&first_id)
        || !offered.contains(&second_id)
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == pulser_id
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    if session
        .legal_actions()
        .ok()?
        .iter()
        .any(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
    {
        return None;
    }
    let mut deathrite_ids = [
        death1.0["cardInstanceId"].as_str()?.to_owned(),
        death2.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDualStartTurnDeathriteSetup {
        deathrite_ids,
        first_id,
        second_id,
        session,
    })
}

fn dual_start_turn_deathrite_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(dual_start_turn_deathrite_manifest)
        .find(|candidate| try_pending_deathrite_during_dual_start_turn(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites during dual start-turn triggers")
}

#[test]
fn rule_catalog_1179_dual_start_turn_triggers_withheld_during_pending_deathrite_order() {
    let encoded = dual_start_turn_deathrite_seed_with(1179);
    let mut setup = try_pending_deathrite_during_dual_start_turn(&encoded)
        .expect("complete dual start-turn Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let first_id = setup.first_id.clone();
    let second_id = setup.second_id.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["pendingDeathrites"]["returnPhase"], "start-turn");
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
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger"),
        "deathrite-order must issue no resolve-start-turn-trigger"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "resolve-start-turn-trigger"
                    || !matches!(
                        action.descriptor["sourceInstanceId"].as_str(),
                        Some(id) if id == first_id || id == second_id
                    )
            })
    );

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "start-turn");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    let remaining: Vec<_> = session
        .legal_actions()
        .expect("resumed start-turn triggers")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("trigger source")
                .to_owned()
        })
        .collect();
    assert!(remaining.contains(&first_id));
    assert!(remaining.contains(&second_id));
    assert_exact_replay(session);
}
fn sparkmage_avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
        "tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn": true,
    })
}

fn plain_avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    })
}

fn air_site() -> Value {
    json!({ "cardType": "site", "elements": ["air"] })
}

fn air_minion(threshold: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": threshold, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn air_threshold_harness_manifest(seed: u32, dual_sparkmage: bool) -> String {
    let south_avatar = if dual_sparkmage {
        sparkmage_avatar()
    } else {
        plain_avatar()
    };
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "phase-storyline-air-threshold-harness" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-phase-storyline-air-threshold-harness-v1",
        },
        "cards": {
            "north-avatar": sparkmage_avatar(),
            "north-minion": air_minion(1),
            "north-site": air_site(),
            "south-avatar": south_avatar,
            "south-minion": air_minion(1),
            "south-site": air_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 6],
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn air_threshold_first_main(seed: u32, dual_sparkmage: bool) -> Session {
    let manifest = air_threshold_harness_manifest(seed, dual_sparkmage);
    let mut session = Session::new(&manifest).expect("valid air-threshold harness session");
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
    session
}

fn north_air_threshold(session: &Session) -> u64 {
    session.replay_value().expect("replay value")["state"]["players"]["north"]
        ["airThresholdsCastThisTurn"]
        .as_u64()
        .expect("north air-threshold counter")
}

fn south_air_threshold(session: &Session) -> Option<u64> {
    session.replay_value().expect("replay value")["state"]["players"]["south"]
        .get("airThresholdsCastThisTurn")
        .and_then(Value::as_u64)
}

fn summon_north_air_minion(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
    });
}

fn summon_south_air_minion(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
}

fn end_active_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn draw_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

#[test]
fn rule_catalog_2513_air_threshold_cast_counter_reset_persists_after_turns_pass() {
    let mut session = air_threshold_first_main(2513, false);
    summon_north_air_minion(&mut session);
    summon_north_air_minion(&mut session);
    assert_eq!(north_air_threshold(&session), 2);
    end_active_turn(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    draw_spellbook(&mut session);
    end_active_turn(&mut session);
    draw_spellbook(&mut session);
    summon_north_air_minion(&mut session);
    assert_eq!(north_air_threshold(&session), 1);
    end_active_turn(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2514_repeat_end_turn_leaves_zero_air_threshold_counter() {
    let mut session = air_threshold_first_main(2514, false);
    summon_north_air_minion(&mut session);
    assert_eq!(north_air_threshold(&session), 1);
    end_active_turn(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    draw_spellbook(&mut session);
    end_active_turn(&mut session);
    draw_spellbook(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    end_active_turn(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2515_opponent_end_turn_resets_active_air_threshold_counter() {
    let mut session = air_threshold_first_main(2515, true);
    summon_north_air_minion(&mut session);
    assert_eq!(north_air_threshold(&session), 1);
    end_active_turn(&mut session);
    draw_spellbook(&mut session);
    summon_south_air_minion(&mut session);
    assert_eq!(south_air_threshold(&session), Some(1));
    assert_eq!(north_air_threshold(&session), 0);
    end_active_turn(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    assert_eq!(south_air_threshold(&session), Some(0));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2516_end_turn_resets_both_players_air_threshold_counters() {
    let mut session = air_threshold_first_main(2516, true);
    summon_north_air_minion(&mut session);
    summon_north_air_minion(&mut session);
    assert_eq!(north_air_threshold(&session), 2);
    end_active_turn(&mut session);
    draw_spellbook(&mut session);
    summon_south_air_minion(&mut session);
    summon_south_air_minion(&mut session);
    assert_eq!(south_air_threshold(&session), Some(2));
    assert_eq!(north_air_threshold(&session), 0);
    end_active_turn(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    assert_eq!(south_air_threshold(&session), Some(0));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2517_end_turn_resets_only_tracked_air_threshold_counters() {
    let mut session = air_threshold_first_main(2517, false);
    summon_north_air_minion(&mut session);
    summon_north_air_minion(&mut session);
    assert_eq!(north_air_threshold(&session), 2);
    assert!(south_air_threshold(&session).is_none());
    end_active_turn(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    assert!(south_air_threshold(&session).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2518_post_reset_air_threshold_cast_accumulates_again() {
    let mut session = air_threshold_first_main(2518, false);
    summon_north_air_minion(&mut session);
    summon_north_air_minion(&mut session);
    assert_eq!(north_air_threshold(&session), 2);
    end_active_turn(&mut session);
    assert_eq!(north_air_threshold(&session), 0);
    draw_spellbook(&mut session);
    end_active_turn(&mut session);
    draw_spellbook(&mut session);
    summon_north_air_minion(&mut session);
    assert_eq!(north_air_threshold(&session), 1);
    assert_exact_replay(&session);
}

fn terminal_order_lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn terminal_order_deathrite() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "preventsDamageFromUnitsWithPowerAtLeast": 4,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_terminal_order_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn terminal_order_manifest(seed: u32) -> String {
    finish_terminal_order_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "terminal-order-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-terminal-order-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-lash": terminal_order_lash(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-deathrite": terminal_order_deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lash"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_terminal_order(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid terminal-order session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn assert_post_action_settlement_before_magic_resolved(types: &[&str]) {
    let resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved");
    assert_eq!(types.first(), Some(&"magic-cast"));
    for (index, event_type) in types.iter().enumerate() {
        if *event_type == "game-ended" {
            assert!(
                index > resolved,
                "game-ended must follow magic-resolved; got {types:?}"
            );
        }
        if matches!(
            *event_type,
            "minion-died"
                | "site-drawn"
                | "ward-broken"
                | "avatar-life-lost"
                | "death-blow"
                | "avatar-reached-deaths-door"
        ) {
            assert!(
                index < resolved,
                "{event_type} must precede magic-resolved; got {types:?}"
            );
        }
    }
    if types.contains(&"game-ended") {
        assert_eq!(types.last(), Some(&"game-ended"));
        assert_eq!(types[types.len() - 2], "magic-resolved");
    } else {
        assert_eq!(types.last(), Some(&"magic-resolved"));
    }
}

fn north_has_lash(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-lash"))
}

fn terminal_order_seed_with(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .map(terminal_order_manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            north_has_lash(&state(&preview))
                && state(&preview)["players"]["south"]["hand"]["spellbook"]
                    .as_array()
                    .map(|hand| {
                        hand.iter()
                            .filter(|card| card["cardId"] == "south-deathrite")
                            .count()
                    })
                    .unwrap_or_default()
                    >= required_south
        })
        .expect("bounded seed with Lash and required South Deathrite minions")
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    realm_unit(snapshot, instance_id).expect("expected realm unit")
}

fn lash_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-lash")
                .count()
        })
        .unwrap_or_default()
}

fn decline_attack_if_needed(session: &mut Session) {
    while session.legal_actions().ok().is_some_and(|actions| {
        actions
            .iter()
            .any(|action| action.descriptor["kind"] == "decline-attack")
    }) {
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

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned()
}

fn setup_c1_with_south_minions(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    (0..count).map(|_| summon_south_at(session, "C1")).collect()
}

fn cast_lash_target(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn lash_minion_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-lash"
                && action.descriptor["target"]["kind"] == "minion"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn try_second_lash_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_terminal_order(encoded);
    let first_id = setup_c1_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_lash_target(&mut session, &first_id);
    if !event_types(&first).contains(&"minion-died") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if lash_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let minion_id = summon_south_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    lash_minion_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_lash_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .find_map(|seed| {
            let encoded = terminal_order_manifest(seed);
            if state(&Session::new(&encoded).ok()?)["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .map(|hand| {
                    hand.iter()
                        .filter(|card| card["cardId"] == "south-deathrite")
                        .count()
                })
                .unwrap_or_default()
                < 2
            {
                return None;
            }
            try_second_lash_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching terminal-order enemy-arrival setup")
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = opening_terminal_order(encoded);
    let c1_ids = setup_c1_with_south_minions(&mut session, 2);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let far_id = summon_south_at(&mut session, "C4");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (!lash_minion_targets(&session).is_empty()).then_some((session, c1_ids, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = terminal_order_manifest(seed);
            if state(&Session::new(&encoded).ok()?)["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .map(|hand| {
                    hand.iter()
                        .filter(|card| card["cardId"] == "south-deathrite")
                        .count()
                })
                .unwrap_or_default()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching terminal-order far-minion setup")
}

fn try_second_lash_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_terminal_order(encoded);
    let first_id = setup_c1_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_lash_target(&mut session, &first_id);
    if !event_types(&first).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &first_id).is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if lash_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C1");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    lash_minion_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_lash_new_summon(start: u32) -> String {
    (start..start + 8192)
        .find_map(|seed| {
            let encoded = terminal_order_manifest(seed);
            if state(&Session::new(&encoded).ok()?)["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .map(|hand| {
                    hand.iter()
                        .filter(|card| card["cardId"] == "south-deathrite")
                        .count()
                })
                .unwrap_or_default()
                < 2
            {
                return None;
            }
            try_second_lash_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching terminal-order new-summon setup")
}

#[test]
fn rule_catalog_2543_post_action_settlement_order_persists_after_turns_pass() {
    let encoded = terminal_order_seed_with(2543, 1);
    let mut session = opening_terminal_order(&encoded);
    let minion_id = setup_c1_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_lash_target(&mut session, &minion_id);
    assert_post_action_settlement_before_magic_resolved(&event_types(&first));
    assert!(event_types(&first).contains(&"minion-died"));
    assert!(event_types(&first).contains(&"site-drawn"));
    pass_turn_to_north_spellbook(&mut session);
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert!(lash_spells_in_hand(&state(&session)) >= 1);
    let south_avatar = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    let second = cast_lash_target(&mut session, &south_avatar);
    assert_post_action_settlement_before_magic_resolved(&event_types(&second));
    assert!(!event_types(&second).contains(&"game-ended"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2544_second_lash_without_settlement_tails_ends_on_magic_resolved() {
    let encoded = terminal_order_seed_with(2544, 1);
    let mut session = opening_terminal_order(&encoded);
    let minion_id = setup_c1_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_lash_target(&mut session, &minion_id);
    assert_post_action_settlement_before_magic_resolved(&event_types(&first));
    assert!(lash_spells_in_hand(&state(&session)) >= 1);
    assert!(lash_minion_targets(&session).is_empty());
    let south_avatar = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    let second = cast_lash_target(&mut session, &south_avatar);
    assert_post_action_settlement_before_magic_resolved(&event_types(&second));
    assert!(!event_types(&second).contains(&"minion-died"));
    assert!(!event_types(&second).contains(&"site-drawn"));
    assert!(!event_types(&second).contains(&"game-ended"));
    assert_eq!(event_types(&second).last(), Some(&"magic-resolved"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2545_second_lash_settlement_stays_before_magic_resolved_after_enemy_arrival() {
    let encoded = seed_for_second_lash_enemy_arrival(2545);
    let (mut session, minion_id) = try_second_lash_enemy_arrival_prefix(&encoded)
        .expect("terminal-order enemy-arrival prefix");
    let receipt = cast_lash_target(&mut session, &minion_id);
    assert_post_action_settlement_before_magic_resolved(&event_types(&receipt));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"site-drawn"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2546_lash_settlement_stays_before_magic_resolved_with_two_nearby_minions() {
    let encoded = terminal_order_seed_with(2546, 2);
    let mut session = opening_terminal_order(&encoded);
    let minion_ids = setup_c1_with_south_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    assert_eq!(lash_minion_targets(&session).len(), 2);
    let receipt = cast_lash_target(&mut session, &minion_ids[0]);
    assert_post_action_settlement_before_magic_resolved(&event_types(&receipt));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"site-drawn"));
    assert!(realm_unit(&state(&session), &minion_ids[0]).is_none());
    assert!(realm_unit(&state(&session), &minion_ids[1]).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2547_lash_settlement_stays_before_magic_resolved_and_leaves_far_minion_untouched() {
    let encoded = seed_for_far_minion(2547);
    let (mut session, c1_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("terminal-order far-minion prefix");
    let receipt = cast_lash_target(&mut session, &c1_ids[0]);
    assert_post_action_settlement_before_magic_resolved(&event_types(&receipt));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &c1_ids[0]).is_none());
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2548_second_lash_settlement_stays_before_magic_resolved_on_new_summon() {
    let encoded = seed_for_second_lash_new_summon(2548);
    let (mut session, minion_id) =
        try_second_lash_new_summon_prefix(&encoded).expect("terminal-order new-summon prefix");
    let receipt = cast_lash_target(&mut session, &minion_id);
    assert_post_action_settlement_before_magic_resolved(&event_types(&receipt));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"site-drawn"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert_exact_replay(&session);
}
