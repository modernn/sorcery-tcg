//! Direct proofs for summon-random-minion-from-any-cemetery Magic
//! (RULE-CATALOG-0159, 0583–0584, 0707–0708, RULE-CATALOG-1054).
//!
//! Ordinary Raise Dead draws one random minion from either cemetery and
//! opens free placement. An empty cemetery pool still resolves the spell as a
//! paid no-op. While Deathrites wait for ordering, raise Magic stays withheld
//! until the chain drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::game::{Game, IssuedAction};
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

fn victim() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raise_dead() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "summonRandomMinionFromAnyCemetery": true,
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

fn visitor() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
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

fn raise_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "raise-dead" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-raise-dead-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-raise": raise_dead(),
            "north-site": site(),
            "north-zap": zap(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-victim": victim(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-zap",
                    "north-raise",
                    "north-zap",
                    "north-raise",
                    "north-zap",
                    "north-raise",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-victim"; 6],
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
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid raise-dead session");
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

fn seed_with(required: &[&str]) -> String {
    (583..583 + 512)
        .map(raise_manifest)
        .find(|candidate| {
            required
                .iter()
                .all(|id| opening_spell_ids(candidate).iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn end_and_draw_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn south_establishes_domain_if_required(session: &mut Session) {
    if session
        .legal_actions()
        .expect("south opening actions")
        .iter()
        .any(|action| action.descriptor["kind"] == "play-site" && action.descriptor["cell"] == "C1")
    {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
    }
}

fn raise_ready(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("raise actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-raise"
        })
}

fn deathrite_raise_manifest(seed: u32) -> String {
    let fixture = "raise-dead-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-raise": raise_dead(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-raise",
                    "north-rain",
                    "north-rain",
                    "north-raise",
                    "north-rain",
                    "north-raise",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-visitor", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_raise_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-raise", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteRaiseSetup {
    cemetery_ids: Vec<String>,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_raise_ready(encoded: &str) -> Option<PendingDeathriteRaiseSetup> {
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
    let visitor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    })?;
    let visitor_id = visitor.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_raise_and_rain(&state(&session)) {
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
    let mut cemetery_ids = deathrite_ids.to_vec();
    cemetery_ids.push(visitor_id);
    cemetery_ids.sort_unstable();
    Some(PendingDeathriteRaiseSetup {
        cemetery_ids,
        deathrite_ids,
        session,
    })
}

fn deathrite_raise_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_raise_manifest)
        .find(|candidate| try_pending_deathrite_with_raise_ready(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with raise Magic in hand")
}

fn setup_south_victim_in_cemetery(session: &mut Session) -> String {
    end_and_draw_spellbook(session);
    south_establishes_domain_if_required(session);
    // Co-locate on north's C4 site so Zap can reach the victim on north's next turn.
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-victim"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let victim_id = summoned["cardInstanceId"]
        .as_str()
        .expect("victim identity")
        .to_owned();
    end_and_draw_spellbook(session);
    let (_, kill) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == victim_id
    });
    assert!(event_types(&kill).contains(&"minion-died"));
    assert!(
        state(session)["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["instanceId"] == victim_id)
    );
    victim_id
}

#[test]
fn rule_catalog_0583_raise_dead_summons_a_random_cemetery_minion_to_a_legal_site() {
    let encoded = seed_with(&["north-zap", "north-raise"]);
    let mut session = opening_main(&encoded);
    let victim_id = setup_south_victim_in_cemetery(&mut session);

    let (cast, cast_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert_eq!(
        event_types(&cast_receipt),
        ["magic-cast", "dead-minion-selected"]
    );
    assert_eq!(cast_receipt.events[1].payload["instanceId"], victim_id);
    assert_eq!(cast_receipt.events[1].payload["cardId"], "south-victim");

    let (_, summon_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-victim"
            && descriptor["cell"] == "C4"
            && descriptor["manaCost"] == 0
            && descriptor["region"].is_null()
    });
    assert!(event_types(&summon_receipt).contains(&"minion-summoned"));
    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == victim_id && unit["location"] == "C4")
    );
    assert_eq!(
        summon_receipt
            .events
            .iter()
            .find(|event| event.event_type == "minion-summoned")
            .expect("minion-summoned")
            .payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0584_raise_dead_with_empty_cemetery_is_a_paid_noop() {
    let encoded = seed_with(&["north-raise"]);
    let mut session = opening_main(&encoded);

    assert!(
        state(&session)["players"]["north"]["cemetery"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0707_raise_dead_selects_random_cemetery_minion_before_free_placement() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0707_raise_dead_selects_random_cemetery_minion_before_free_placement();
}

#[test]
fn rule_catalog_0708_raise_dead_blocked_footprint_summon() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0708_raise_dead_blocked_footprint_summon();
}

#[test]
fn rule_catalog_1054_raise_dead_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_raise_seed_with(1054);
    let mut setup = try_pending_deathrite_with_raise_ready(&encoded)
        .expect("complete raise-dead Deathrite withheld setup");
    let cemetery_ids = setup.cemetery_ids.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(cemetery_ids.iter().all(|instance_id| {
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
    assert!(!raise_ready(session));

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
    assert!(raise_ready(session));

    let (cast, cast_receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert_eq!(
        event_types(&cast_receipt),
        ["magic-cast", "dead-minion-selected"]
    );
    let raised_id = cast_receipt.events[1].payload["instanceId"]
        .as_str()
        .expect("raised minion identity")
        .to_owned();
    assert!(
        cemetery_ids.contains(&raised_id),
        "Raise Dead must draw from the rain-killed cemetery pool"
    );

    let (_, summon_receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == raised_id
            && descriptor["manaCost"] == 0
            && descriptor["region"].is_null()
    });
    assert!(event_types(&summon_receipt).contains(&"minion-summoned"));
    assert!(
        state(session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == raised_id)
    );
    assert_eq!(
        summon_receipt
            .events
            .iter()
            .find(|event| event.event_type == "minion-summoned")
            .expect("minion-summoned")
            .payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_exact_replay(session);
}

fn power_bonus_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "otherNearbyAlliesPowerBonus": 1,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_cemetery_summon_interrupt_manifest(seed: u32) -> String {
    let fixture = "raise-dead-cemetery-summon-deathrite-interrupt";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-raise": raise_dead(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-aura": power_bonus_minion(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-raise",
                    "north-rain",
                    "north-rain",
                    "north-raise",
                    "north-rain",
                    "north-raise",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-visitor", 2))
                    .chain(std::iter::repeat_n("south-aura", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn issued_descriptor(action: &IssuedAction) -> Value {
    serde_json::to_value(action.descriptor()).expect("typed descriptor JSON")
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

fn apply_where(game: &mut Game, predicate: impl Fn(&Value) -> bool) {
    let action = game
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&issued_descriptor(action)))
        .expect("expected engine-issued action");
    game.apply_action(&action).expect("authoritative Game step");
}

struct PendingCemeterySummonInterruptSetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_ready_cemetery_summon_interrupt(
    encoded: &str,
) -> Option<PendingCemeterySummonInterruptSetup> {
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
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
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
    let aura = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-aura"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_raise_and_rain(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    let snapshot = state(&session);
    if snapshot["phase"] != "main" {
        return None;
    }
    let aura_id = aura.0["cardInstanceId"].as_str()?.to_owned();
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    if deathrite_ids.iter().any(|instance_id| {
        realm_unit(&snapshot, instance_id).is_none_or(|unit| unit["damage"] != 1)
    }) || realm_unit(&snapshot, &aura_id).is_none_or(|unit| unit["damage"] != 1)
    {
        return None;
    }
    Some(PendingCemeterySummonInterruptSetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_cemetery_summon_interrupt_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cemetery_summon_interrupt_manifest)
        .find(|candidate| try_ready_cemetery_summon_interrupt(candidate).is_some())
        .expect("bounded seed that reaches wounded Deathrites after Rain in main")
}

#[test]
fn rule_catalog_1177_cemetery_summon_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cemetery_summon_interrupt_seed_with(1177);
    let setup = try_ready_cemetery_summon_interrupt(&encoded)
        .expect("complete cemetery-summon Deathrite interrupt setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    assert_exact_replay(&setup.session);

    let mut control = setup.session.clone();
    let (_, cast_receipt) = accept_where(&mut control, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert_eq!(cast_receipt.events[1].event_type, "dead-minion-selected");
    let raised_id = cast_receipt.events[1].payload["instanceId"]
        .as_str()
        .expect("raised minion identity")
        .to_owned();
    assert_eq!(state(&control)["phase"], "cemetery-summon");

    let mut branched = replay_game(&setup.session);
    assert!(
        branched.test_remove_realm_unit(&aura_id),
        "checkpoint branch must drop the power-bonus ally so wounded Deathrites settle"
    );
    apply_where(&mut branched, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });

    let paused = branched.authoritative_state();
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(
        paused["pendingDeathrites"]["returnPhase"],
        "cemetery-summon"
    );
    assert_eq!(paused["pendingCemeterySummon"]["cardInstanceId"], raised_id);
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        branched
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| issued_descriptor(action)["kind"] != "summon-minion"),
        "trigger-order must issue no cemetery summon while cemetery-summon stays pending"
    );

    let order_sources: Vec<_> = branched
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| issued_descriptor(action)["kind"] == "order-triggers")
        .map(|action| {
            issued_descriptor(&action)["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);
    apply_where(&mut branched, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = branched.authoritative_state();
    assert_eq!(resumed["phase"], "cemetery-summon");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(
        resumed["pendingCemeterySummon"]["cardInstanceId"],
        raised_id
    );
    assert!(
        branched
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                issued_descriptor(action)["kind"] == "summon-minion"
                    && issued_descriptor(action)["cardInstanceId"] == raised_id
            }),
        "cemetery summon-minion must return once trigger-order clears"
    );
}

fn victim_b() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raise_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "raise-dead-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-raise-dead-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-raise": raise_dead(),
            "north-site": site(),
            "north-zap": zap(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-victim": victim(),
            "south-victim-b": victim_b(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-zap",
                    "north-raise",
                    "north-zap",
                    "north-raise",
                    "north-zap",
                    "north-raise",
                    "north-zap",
                    "north-raise",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-victim"; 6]
                    .into_iter()
                    .chain(vec!["south-victim-b"; 4])
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn assert_kill_victim_prefix(encoded: &str) {
    let mut session = opening_main(encoded);
    kill_south_minion_at_c4(&mut session, "south-victim");
}

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(583..583 + 2048)
        .map(raise_supplemental_manifest)
        .find(|candidate| {
            required
                .iter()
                .all(|id| opening_spell_ids(candidate).iter().any(|card| card == id))
        })
        .inspect(|candidate| assert_kill_victim_prefix(candidate))
        .expect("bounded seed with required opening cards")
}

fn raise_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-raise")
                .count()
        })
        .unwrap_or_default()
}

fn cemetery_instance_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
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

fn kill_south_minion_at(session: &mut Session, card_id: &str, cell: &str) -> String {
    end_and_draw_spellbook(session);
    south_establishes_domain_if_required(session);
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned minion identity")
        .to_owned();
    end_and_draw_spellbook(session);
    let (_, kill) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == minion_id
    });
    assert!(event_types(&kill).contains(&"minion-died"));
    assert!(cemetery_instance_ids(&state(session), "south").contains(&minion_id));
    minion_id
}

fn kill_south_minion_at_c4(session: &mut Session, card_id: &str) -> String {
    kill_south_minion_at(session, card_id, "C4")
}

fn cast_raise_select(session: &mut Session) -> String {
    let (_, cast_receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert_eq!(
        event_types(&cast_receipt),
        ["magic-cast", "dead-minion-selected"]
    );
    cast_receipt.events[1].payload["instanceId"]
        .as_str()
        .expect("raised minion identity")
        .to_owned()
}

fn complete_cemetery_summon(session: &mut Session, raised_id: &str, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == raised_id
            && descriptor["cell"] == cell
            && descriptor["manaCost"] == 0
            && descriptor["region"].is_null()
    });
    assert!(event_types(&receipt).contains(&"minion-summoned"));
    receipt
}

fn cast_raise_and_summon(session: &mut Session, cell: &str) -> String {
    let raised_id = cast_raise_select(session);
    complete_cemetery_summon(session, &raised_id, cell);
    raised_id
}

fn seed_with_two_corpses(start: u32) -> String {
    (start..start + 2048)
        .chain(583..583 + 2048)
        .map(raise_supplemental_manifest)
        .find(|candidate| {
            ["north-zap", "north-raise"]
                .iter()
                .all(|id| opening_spell_ids(candidate).iter().any(|card| card == id))
        })
        .inspect(|candidate| assert_two_corpses_prefix(candidate))
        .expect("bounded seed with two cemetery corpses")
}

fn assert_two_corpses_prefix(encoded: &str) {
    let mut session = opening_main(encoded);
    kill_south_minion_at_c4(&mut session, "south-victim");
    pass_turn_to_north_spellbook(&mut session);
    kill_south_minion_at_c4(&mut session, "south-victim-b");
}

fn setup_two_corpses_in_south_cemetery(session: &mut Session) -> [String; 2] {
    let first = kill_south_minion_at_c4(session, "south-victim");
    pass_turn_to_north_spellbook(session);
    let second = kill_south_minion_at_c4(session, "south-victim-b");
    [first, second]
}

fn try_second_raise_new_kill_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    kill_south_minion_at_c4(&mut session, "south-victim");
    let raised_id = cast_raise_and_summon(&mut session, "C4");
    pass_turn_to_north_spellbook(&mut session);
    if raise_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let (_, first_kill) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == raised_id
    })?;
    if !event_types(&first_kill).contains(&"minion-died") {
        return None;
    }
    let new_id = kill_south_minion_at_c4(&mut session, "south-victim-b");
    cemetery_instance_ids(&state(&session), "south")
        .contains(&new_id)
        .then_some((session, new_id))
}

fn seed_for_second_raise_new_kill(start: u32) -> String {
    (start..start + 8192)
        .chain(583..583 + 8192)
        .find_map(|seed| {
            let encoded = raise_supplemental_manifest(seed);
            if !opening_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-raise")
            {
                return None;
            }
            assert_kill_victim_prefix(&encoded);
            try_second_raise_new_kill_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Raise Dead new-kill setup")
}

fn try_second_raise_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    kill_south_minion_at_c4(&mut session, "south-victim");
    let raised_id = cast_raise_and_summon(&mut session, "C4");
    pass_turn_to_north_spellbook(&mut session);
    if raise_spells_in_hand(&state(&session)) < 1 {
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
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, kill) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-zap"
            && descriptor["target"]["instanceId"] == raised_id
    })?;
    if !event_types(&kill).contains(&"minion-died") {
        return None;
    }
    cemetery_instance_ids(&state(&session), "south")
        .contains(&raised_id)
        .then_some((session, raised_id))
}

fn seed_for_second_raise_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(583..583 + 8192)
        .find_map(|seed| {
            let encoded = raise_supplemental_manifest(seed);
            if !opening_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-raise")
            {
                return None;
            }
            assert_kill_victim_prefix(&encoded);
            try_second_raise_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Raise Dead enemy-arrival setup")
}

#[test]
fn rule_catalog_1893_raised_minion_stays_on_board_after_turns_pass() {
    let encoded = seed_with_start(1893, &["north-zap", "north-raise"]);
    let mut session = opening_main(&encoded);
    kill_south_minion_at_c4(&mut session, "south-victim");
    let raised_id = cast_raise_and_summon(&mut session, "C4");
    pass_turn_to_north_spellbook(&mut session);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == raised_id && unit["location"] == "C4")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1894_second_raise_dead_without_a_cemetery_minion_is_a_paid_noop() {
    let encoded = seed_with_start(1894, &["north-zap", "north-raise"]);
    let mut session = opening_main(&encoded);
    let _ = kill_south_minion_at_c4(&mut session, "south-victim");
    let _ = cast_raise_and_summon(&mut session, "C4");
    assert!(cemetery_instance_ids(&state(&session), "south").is_empty());
    assert!(raise_spells_in_hand(&state(&session)) >= 1);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-raise"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "dead-minion-selected")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1895_second_raise_dead_summons_a_newly_killed_minion_after_enemy_arrival() {
    let encoded = seed_for_second_raise_enemy_arrival(1895);
    let (mut session, victim_id) = try_second_raise_enemy_arrival_prefix(&encoded)
        .expect("second Raise Dead enemy-arrival prefix");
    let selected = cast_raise_select(&mut session);
    assert_eq!(selected, victim_id);
    complete_cemetery_summon(&mut session, &victim_id, "C4");
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == victim_id && unit["location"] == "C4")
    );
    assert_eq!(
        state(&session)["realm"]["sites"]["C2"]["controller"],
        "south"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1896_raise_dead_selects_from_a_multi_minion_cemetery_pool() {
    let encoded = seed_with_two_corpses(1896);
    let mut session = opening_main(&encoded);
    let corpses = setup_two_corpses_in_south_cemetery(&mut session);
    let selected = cast_raise_select(&mut session);
    assert!(
        corpses.contains(&selected),
        "expected one of {corpses:?}, got {selected}"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1897_raise_dead_leaves_an_unselected_cemetery_minion_untouched() {
    let encoded = seed_with_two_corpses(1897);
    let mut session = opening_main(&encoded);
    let corpses = setup_two_corpses_in_south_cemetery(&mut session);
    let selected = cast_raise_select(&mut session);
    let unselected = corpses
        .iter()
        .find(|id| *id != &selected)
        .expect("unselected corpse")
        .clone();
    complete_cemetery_summon(&mut session, &selected, "C4");
    let remaining = cemetery_instance_ids(&state(&session), "south");
    assert_eq!(remaining, vec![unselected]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1898_second_raise_dead_summons_a_newly_killed_minion() {
    let encoded = seed_for_second_raise_new_kill(1898);
    let (mut session, victim_id) =
        try_second_raise_new_kill_prefix(&encoded).expect("second Raise Dead new-kill prefix");
    let selected = cast_raise_select(&mut session);
    assert_eq!(selected, victim_id);
    complete_cemetery_summon(&mut session, &victim_id, "C4");
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == victim_id && unit["location"] == "C4")
    );
    assert_exact_replay(&session);
}
