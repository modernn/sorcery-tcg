//! Direct proofs for untap-target-minion Magic (RULE-CATALOG-0613–0614,
//! RULE-CATALOG-1003, RULE-CATALOG-2033–2038).
//!
//! Untap Magic readies a tapped same-region minion without breaking Ward. A
//! second cast on an already-ready minion is a paid no-op. While Deathrites
//! wait for ordering, untap Magic stays withheld until the chain drains.

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

fn charger() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    })
}

fn untap_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "untapTargetMinion": true,
    })
}

fn tap_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "tapTargetMinion": true,
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
        "defense": 3,
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

fn untap_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "untap-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-untap-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-untap": untap_spell(),
            "south-avatar": avatar(),
            "south-charger": charger(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-untap"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-charger"; 6],
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid untap magic session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("realm unit")
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

fn untap_minion_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("untap actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-untap"
        })
        .filter_map(|action| {
            let target = action.descriptor.get("target")?;
            Some((
                target["kind"].as_str()?.to_owned(),
                target["instanceId"].as_str()?.to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn deathrite_untap_manifest(seed: u32) -> String {
    let fixture = "untap-target-minion-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "north-tap": tap_spell(),
            "north-untap": untap_spell(),
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
                    "north-tap",
                    "north-untap",
                    "north-rain",
                    "north-rain",
                    "north-tap",
                    "north-untap",
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

fn north_has_untap_tap_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-untap", "north-tap", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteUntapSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_tapped_visitor(encoded: &str) -> Option<PendingDeathriteUntapSetup> {
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
    if !north_has_untap_tap_and_rain(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-tap"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    })?;
    if realm_unit(&state(&session), &visitor_id)["tapped"] != true {
        return None;
    }
    if untap_minion_targets(&session).is_empty() {
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
    Some(PendingDeathriteUntapSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_untap_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_untap_manifest)
        .find(|candidate| try_pending_deathrite_with_tapped_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with untap Magic in hand")
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

fn stage_south_charger(session: &mut Session, tap: bool) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-charger"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let charger_id = summoned["cardInstanceId"]
        .as_str()
        .expect("South charger identity")
        .to_owned();
    if tap {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "move-and-attack"
                && descriptor["unitInstanceId"] == charger_id
                && descriptor["to"]["cell"] == "C1"
        });
        if session
            .legal_actions()
            .expect("post-activation actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "decline-attack")
        {
            accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
        }
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    charger_id
}

#[test]
fn rule_catalog_0613_untap_magic_readies_a_tapped_minion_without_breaking_ward() {
    let encoded = untap_manifest(613);
    let mut session = opening_main(&encoded);
    let charger_id = stage_south_charger(&mut session, true);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &charger_id)["tapped"], true);
    assert_eq!(realm_unit(&before, &charger_id)["warded"], true);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-untap"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == charger_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-untapped", "magic-resolved"]
    );
    let untapped = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-untapped")
        .expect("untap event");
    assert_eq!(untapped.payload["instanceId"], charger_id);
    assert_eq!(
        untapped.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken")
    );

    let after = state(&session);
    assert_eq!(realm_unit(&after, &charger_id)["tapped"], false);
    assert_eq!(realm_unit(&after, &charger_id)["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0614_untap_magic_is_a_paid_noop_when_already_untapped() {
    let encoded = untap_manifest(614);
    let mut session = opening_main(&encoded);
    let charger_id = stage_south_charger(&mut session, false);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &charger_id)["tapped"], false);
    assert_eq!(realm_unit(&before, &charger_id)["warded"], true);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-untap"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == charger_id
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-untapped"
                || event.event_type == "ward-broken")
    );

    let after = state(&session);
    assert_eq!(realm_unit(&after, &charger_id)["tapped"], false);
    assert_eq!(realm_unit(&after, &charger_id)["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1003_untap_target_minion_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_untap_seed_with(1003);
    let mut setup = try_pending_deathrite_with_tapped_visitor(&encoded)
        .expect("complete untap Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert_eq!(realm_unit(&paused, &visitor_id)["tapped"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(untap_minion_targets(session).is_empty());

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
    assert_eq!(realm_unit(&resumed, &visitor_id)["tapped"], true);
    assert_eq!(
        untap_minion_targets(session),
        [("minion".to_owned(), visitor_id.clone())]
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-untap"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-untapped", "magic-resolved"]
    );
    assert_eq!(realm_unit(&state(session), &visitor_id)["tapped"], false);
    assert_exact_replay(session);
}

fn supplemental_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn untap_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "untap-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-untap-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-tap": tap_spell(),
            "north-untap": untap_spell(),
            "south-avatar": avatar(),
            "south-minion": supplemental_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec![
                    "north-tap", "north-tap", "north-tap", "north-tap", "north-untap",
                    "north-untap", "north-untap", "north-untap",
                ],
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

fn opening_hand_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
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

fn supplemental_seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(613..613 + 2048)
        .map(untap_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-untap")
                && opening_hand_spell_ids(candidate, "north")
                    .iter()
                    .any(|card| card == "north-tap")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with untap Magic, tap Magic, and required South minions")
}

fn untap_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-untap")
                .count()
        })
        .unwrap_or_default()
}

fn tap_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-tap")
                .count()
        })
        .unwrap_or_default()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
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

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("untap target identity")
        .to_owned()
}

fn setup_c2_with_south_minions(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    (0..count).map(|_| summon_south_at(session, "C2")).collect()
}

fn cast_tap_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-tap"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn cast_untap_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-untap"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn untap_target_ids(session: &Session) -> Vec<String> {
    untap_minion_targets(session)
        .into_iter()
        .map(|(_, instance_id)| instance_id)
        .collect()
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    let c2_ids = setup_c2_with_south_minions(&mut session, 2);
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
    (tap_spells_in_hand(&state(&session)) >= 1 && untap_spells_in_hand(&state(&session)) >= 1)
        .then_some((session, c2_ids[0].clone(), far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(613..613 + 2048)
        .find_map(|seed| {
            let encoded = untap_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching untap far-minion setup")
}

fn try_second_untap_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_tap_on(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if untap_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    cast_untap_on(&mut session, &first_id);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let minion_id = summon_south_at(&mut session, "C3");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if tap_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    cast_tap_on(&mut session, &minion_id);
    (untap_spells_in_hand(&state(&session)) >= 1 && untap_target_ids(&session).contains(&minion_id))
        .then_some((session, minion_id))
}

fn seed_for_second_untap_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(613..613 + 8192)
        .find_map(|seed| {
            let encoded = untap_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_untap_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second untap enemy-arrival setup")
}

fn try_second_untap_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_tap_on(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if untap_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    cast_untap_on(&mut session, &first_id);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if tap_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    cast_tap_on(&mut session, &minion_id);
    (untap_spells_in_hand(&state(&session)) >= 1 && untap_target_ids(&session).contains(&minion_id))
        .then_some((session, minion_id))
}

fn seed_for_second_untap_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(613..613 + 8192)
        .find_map(|seed| {
            let encoded = untap_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_untap_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second untap new-summon setup")
}

#[test]
fn rule_catalog_2033_untapped_minion_stays_at_the_location_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2033, 1);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_tap_on(&mut session, &minion_id);
    let receipt = cast_untap_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-untapped"));
    assert_eq!(unit(&state(&session), &minion_id)["tapped"], false);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(realm_unit(&state(&session), &minion_id)["location"], "C2");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2034_second_untap_on_an_already_ready_minion_is_a_paid_noop() {
    let encoded = (2034..2034 + 8192)
        .chain(613..613 + 8192)
        .find_map(|seed| {
            let candidate = untap_supplemental_manifest(seed);
            let mut session = opening_main(&candidate);
            let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
            north_draws_spellbook(&mut session);
            cast_tap_on(&mut session, &minion_id);
            let first = cast_untap_on(&mut session, &minion_id);
            if !event_types(&first).contains(&"minion-untapped") {
                return None;
            }
            if unit(&state(&session), &minion_id)["tapped"]
                .as_bool()
                .unwrap_or(true)
            {
                return None;
            }
            (untap_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two untap casts after readying the first minion");
    let mut session = opening_main(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_tap_on(&mut session, &minion_id);
    let first = cast_untap_on(&mut session, &minion_id);
    assert!(event_types(&first).contains(&"minion-untapped"));
    assert_eq!(unit(&state(&session), &minion_id)["tapped"], false);
    assert!(untap_spells_in_hand(&state(&session)) >= 1);
    let second = cast_untap_on(&mut session, &minion_id);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"minion-untapped"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2035_second_untap_readies_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_untap_enemy_arrival(2035);
    let (mut session, minion_id) =
        try_second_untap_enemy_arrival_prefix(&encoded).expect("second untap enemy-arrival prefix");
    let receipt = cast_untap_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-untapped"));
    assert_eq!(unit(&state(&session), &minion_id)["tapped"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2036_untap_offers_every_tapped_same_region_minion_in_the_caster_region() {
    let encoded = supplemental_seed_with_start(2036, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c2_with_south_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    for minion_id in &minion_ids {
        cast_tap_on(&mut session, minion_id);
    }
    let offered = untap_target_ids(&session);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2037_untap_leaves_a_far_ready_minion_untouched() {
    let encoded = seed_for_far_minion(2037);
    let (mut session, untapped_id, far_id) =
        try_far_minion_prefix(&encoded).expect("untap far-minion prefix");
    cast_tap_on(&mut session, &untapped_id);
    let receipt = cast_untap_on(&mut session, &untapped_id);
    assert!(event_types(&receipt).contains(&"minion-untapped"));
    assert_eq!(unit(&state(&session), &untapped_id)["tapped"], false);
    assert_eq!(unit(&state(&session), &far_id)["tapped"], false);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2038_second_untap_readies_a_newly_summoned_minion() {
    let encoded = seed_for_second_untap_new_summon(2038);
    let (mut session, minion_id) =
        try_second_untap_new_summon_prefix(&encoded).expect("second untap new-summon prefix");
    let receipt = cast_untap_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-untapped"));
    assert_eq!(unit(&state(&session), &minion_id)["tapped"], false);
    assert_exact_replay(&session);
}
