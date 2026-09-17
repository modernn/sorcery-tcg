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
    if state(&session)["phase"] != "deathrite-order" {
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
    assert_eq!(paused["phase"], "deathrite-order");
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
