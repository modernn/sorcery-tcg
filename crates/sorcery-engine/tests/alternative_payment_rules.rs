//! Direct proofs for alternate summon payment admission and cost enforcement
//! (RULE-CATALOG-0015, RULE-CATALOG-0016, RULE-CATALOG-0729, RULE-CATALOG-0921,
//! RULE-CATALOG-2523–2528).
//!
//! 2523–2528 add persistence, empty-repeat, enemy-arrival, multi-helper,
//! far-helper, and new-summon branches on the same alternate-payment harness.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn thresholds(element: Option<&str>, required: u64) -> Value {
    let mut value = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    if let Some(element) = element {
        value[element] = json!(required);
    }
    value
}

fn site(element: &str) -> Value {
    json!({ "cardType": "site", "elements": [element] })
}

fn minion(mana_cost: u64, required: &Value) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": mana_cost,
        "thresholds": required,
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn random_discard_manifest(seed: u32, north_site: &Value) -> String {
    let mut aramos = minion(3, &thresholds(Some("earth"), 1));
    aramos["discardRandomCardInsteadOfMana"] = json!(true);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "alternative-payment-thresholds" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-alternative-payment-thresholds-v1",
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-minion": aramos,
            "north-site": north_site,
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-minion": minion(0, &thresholds(None, 0)),
            "south-site": site("earth"),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 4],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 4],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn sacrifice_threshold_manifest(seed: u32, north_site: &Value) -> String {
    let mut gnarled = minion(6, &thresholds(Some("water"), 1));
    gnarled["sacrificeMinionAtSummoningLocationForManaDiscount"] = json!(2);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sacrifice-payment-thresholds" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sacrifice-payment-thresholds-v1",
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-gnarled": gnarled,
            "north-helper": minion(0, &thresholds(None, 0)),
            "north-site": north_site,
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-minion": minion(0, &thresholds(None, 0)),
            "south-site": site("earth"),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-gnarled", "north-helper", "north-helper", "north-helper"],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid alternative-payment scenario");
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

fn summon_actions(session: &Session, card_id: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion" && action.descriptor["cardId"] == card_id
        })
        .map(|action| action.descriptor)
        .collect()
}

fn sacrifice_board(manifest: &str) -> Session {
    sacrifice_board_with_helpers(manifest, 2)
}

fn sacrifice_board_with_helpers(manifest: &str, count: usize) -> Session {
    let mut session = opening_main(manifest);
    for _ in 0..count {
        summon_helper_at(&mut session, "C4");
    }
    session
}

fn payment_supplemental_manifest(seed: u32) -> String {
    let mut gnarled = minion(6, &thresholds(Some("water"), 1));
    gnarled["sacrificeMinionAtSummoningLocationForManaDiscount"] = json!(2);
    let mut aramos = minion(3, &thresholds(Some("earth"), 1));
    aramos["discardRandomCardInsteadOfMana"] = json!(true);
    let mut water_site = site("water");
    water_site["genesisGainMana"] = json!(6);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "alternative-payment-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-alternative-payment-supplemental-v1",
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-gnarled": gnarled,
            "north-helper": minion(0, &thresholds(None, 0)),
            "north-minion": aramos,
            "north-water-site": water_site,
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-minion": minion(0, &thresholds(None, 0)),
            "south-site": site("earth"),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-gnarled",
                    "north-gnarled",
                    "north-gnarled",
                    "north-gnarled",
                    "north-minion",
                    "north-minion",
                    "north-helper",
                    "north-helper",
                    "north-helper",
                    "north-helper",
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

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn assert_exact_replay(session: &Session) {
    assert!(session.verify_replay().expect("verified replay transcript"));
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

fn hand_card_count(snapshot: &Value, seat: &str) -> usize {
    let hand = &snapshot["players"][seat]["hand"];
    hand["atlas"]
        .as_array()
        .map(|cards| cards.len())
        .unwrap_or_default()
        + hand["spellbook"]
            .as_array()
            .map(|cards| cards.len())
            .unwrap_or_default()
}

fn supplemental_seed_sacrifice(start: u32, min_gnarled: usize, min_helper: usize) -> String {
    (start..start + 2048)
        .chain(729..729 + 2048)
        .map(payment_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .filter(|card| *card == "north-gnarled")
                .count()
                >= min_gnarled
                && opening_hand_spell_ids(candidate, "north")
                    .iter()
                    .filter(|card| *card == "north-helper")
                    .count()
                    >= min_helper
        })
        .expect("bounded seed with Gnarled and required helpers")
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
    if offers(session, |descriptor| descriptor["kind"] == "end-turn") {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    }
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

fn summon_helper_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-helper"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("helper identity")
        .to_owned()
}

fn helpers_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-helper")
                .count()
        })
        .unwrap_or_default()
}

fn gnarled_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-gnarled")
                .count()
        })
        .unwrap_or_default()
}

fn gnarled_sacrifice_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Gnarled legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-gnarled"
                && action.descriptor["sacrificedMinionInstanceIds"]
                    .as_array()
                    .is_some_and(|ids| !ids.is_empty())
        })
        .flat_map(|action| {
            action.descriptor["sacrificedMinionInstanceIds"]
                .as_array()
                .expect("sacrifice ids")
                .iter()
                .filter_map(|id| id.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn try_summon_helper_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-helper"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn try_accept_gnarled_sacrifice(
    session: &mut Session,
    sacrificed_id: &str,
) -> Option<(String, Receipt)> {
    let descriptor = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-gnarled"
                && action.descriptor["sacrificedMinionInstanceIds"]
                    .as_array()
                    .is_some_and(|ids| ids.len() == 1 && ids[0] == sacrificed_id)
        })?
        .descriptor
        .clone();
    let gnarled_id = descriptor["cardInstanceId"].as_str()?.to_owned();
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| action.descriptor == descriptor)?;
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
    Some((gnarled_id, receipt))
}

fn try_accept_random_discard_summon(session: &mut Session) -> Option<Receipt> {
    let descriptor = summon_actions(session, "north-minion")
        .into_iter()
        .find(|d| d["paymentMode"] == "random-card-discard")?;
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|a| a.descriptor == descriptor)?;
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
    Some(receipt)
}

fn accept_gnarled_sacrifice(session: &mut Session, sacrificed_id: &str) -> (String, Receipt) {
    let descriptor = session
        .legal_actions()
        .expect("Gnarled legal actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-gnarled"
                && action.descriptor["sacrificedMinionInstanceIds"]
                    .as_array()
                    .is_some_and(|ids| ids.len() == 1 && ids[0] == sacrificed_id)
        })
        .expect("engine-issued Gnarled sacrifice payment")
        .descriptor
        .clone();
    let gnarled_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("Gnarled identity")
        .to_owned();
    let action = session
        .legal_actions()
        .expect("Gnarled legal actions")
        .into_iter()
        .find(|action| action.descriptor == descriptor)
        .expect("engine-issued Gnarled sacrifice payment");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("Gnarled sacrifice summon")
    else {
        panic!("engine-issued Gnarled sacrifice summon must be accepted");
    };
    (gnarled_id, receipt)
}

fn accept_random_discard_summon(session: &mut Session) -> Receipt {
    let descriptor = summon_actions(session, "north-minion")
        .into_iter()
        .find(|descriptor| descriptor["paymentMode"] == "random-card-discard")
        .expect("random-card discard payment");
    let action = session
        .legal_actions()
        .expect("Aramos legal actions")
        .into_iter()
        .find(|action| action.descriptor == descriptor)
        .expect("engine-issued random discard");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("random discard summon")
    else {
        panic!("engine-issued random discard summon must be accepted");
    };
    receipt
}

fn try_second_gnarled_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    if opening_hand_spell_ids(encoded, "north")
        .iter()
        .filter(|card| *card == "north-gnarled")
        .count()
        < 1
        || opening_hand_spell_ids(encoded, "north")
            .iter()
            .filter(|card| *card == "north-helper")
            .count()
            < 1
    {
        return None;
    }
    let mut session = opening_main(encoded);
    let helper_id = try_summon_helper_at(&mut session, "C4")?;
    let (_, first) = try_accept_gnarled_sacrifice(&mut session, &helper_id)?;
    if !event_types(&first).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &helper_id).is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if gnarled_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if helpers_in_hand(&state(&session)) < 1 {
        return None;
    }
    let new_helper = try_summon_helper_at(&mut session, "C4")?;
    gnarled_sacrifice_targets(&session)
        .contains(&new_helper)
        .then_some((session, new_helper))
}

fn seed_for_second_gnarled_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(729..729 + 8192)
        .find_map(|seed| {
            let encoded = payment_supplemental_manifest(seed);
            try_second_gnarled_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Gnarled enemy-arrival setup")
}

fn try_far_helper_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    let near_id = try_summon_helper_at(&mut session, "C4")?;
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let far_id = try_summon_helper_at(&mut session, "C1")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (gnarled_in_hand(&state(&session)) >= 1
        && gnarled_sacrifice_targets(&session).contains(&near_id))
    .then_some((session, near_id, far_id))
}

fn seed_for_far_helper(start: u32) -> String {
    (start..start + 8192)
        .chain(729..729 + 8192)
        .find_map(|seed| {
            let encoded = payment_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "north")
                .iter()
                .filter(|card| *card == "north-helper")
                .count()
                < 2
                || opening_hand_spell_ids(&encoded, "north")
                    .iter()
                    .filter(|card| *card == "north-gnarled")
                    .count()
                    < 1
            {
                return None;
            }
            try_far_helper_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching far-helper setup")
}

fn try_second_gnarled_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    if opening_hand_spell_ids(encoded, "north")
        .iter()
        .filter(|card| *card == "north-gnarled")
        .count()
        < 1
        || opening_hand_spell_ids(encoded, "north")
            .iter()
            .filter(|card| *card == "north-helper")
            .count()
            < 1
    {
        return None;
    }
    let mut session = opening_main(encoded);
    let helper_id = try_summon_helper_at(&mut session, "C4")?;
    let (_, first) = try_accept_gnarled_sacrifice(&mut session, &helper_id)?;
    if !event_types(&first).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &helper_id).is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if gnarled_in_hand(&state(&session)) < 1 || helpers_in_hand(&state(&session)) < 1 {
        return None;
    }
    let new_helper = try_summon_helper_at(&mut session, "C4")?;
    gnarled_sacrifice_targets(&session)
        .contains(&new_helper)
        .then_some((session, new_helper))
}

fn seed_for_second_gnarled_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(729..729 + 8192)
        .find_map(|seed| {
            let encoded = payment_supplemental_manifest(seed);
            try_second_gnarled_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Gnarled new-summon setup")
}

fn discard_supplemental_manifest(seed: u32) -> String {
    let mut aramos = minion(3, &thresholds(Some("earth"), 1));
    aramos["discardRandomCardInsteadOfMana"] = json!(true);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "alternative-payment-discard-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-alternative-payment-discard-supplemental-v1",
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-minion": aramos,
            "north-site": site("earth"),
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-minion": minion(0, &thresholds(None, 0)),
            "south-site": site("earth"),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
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

fn try_second_discard_prefix(encoded: &str) -> Option<String> {
    if opening_hand_spell_ids(encoded, "north")
        .iter()
        .filter(|card| *card == "north-minion")
        .count()
        < 2
    {
        return None;
    }
    let mut session = opening_main(encoded);
    let before = state(&session);
    let atlas_cards = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .map(|hand| hand.len())
        .unwrap_or_default();
    let minions_in_hand = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-minion")
                .count()
        })
        .unwrap_or_default();
    if atlas_cards == 0 || minions_in_hand < 2 {
        return None;
    }
    try_accept_random_discard_summon(&mut session)?;
    let after = state(&session);
    if after["players"]["north"]["hand"]["atlas"]
        .as_array()
        .is_some_and(|hand| !hand.is_empty())
    {
        return None;
    }
    if after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-minion")
                .count()
        })
        != Some(1)
    {
        return None;
    }
    (!summon_actions(&session, "north-minion")
        .iter()
        .any(|descriptor| descriptor["paymentMode"] == "random-card-discard"))
    .then_some(encoded.to_owned())
}

#[test]
fn rule_catalog_0729_alternate_summon_payments_should_be_admitted_and_require_their_costs() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0729_alternate_summon_payments_should_be_admitted_and_require_their_costs(
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "alternate payment threshold gate proof covers blocked and allowed branches"
)]
fn rule_catalog_0921_alternate_summon_payments_require_thresholds_before_offering() {
    let blocked_discard = opening_main(&random_discard_manifest(921, &site("fire")));
    let before_discard = state(&blocked_discard);
    assert!(
        !before_discard["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("Atlas hand")
            .is_empty()
    );
    assert!(
        before_discard["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("Spellbook hand")
            .len()
            > 1
    );
    assert!(summon_actions(&blocked_discard, "north-minion").is_empty());
    assert!(
        blocked_discard
            .verify_replay()
            .expect("verified blocked random-discard replay")
    );

    let mut allowed_discard = opening_main(&random_discard_manifest(922, &site("earth")));
    let discard_summons = summon_actions(&allowed_discard, "north-minion");
    assert!(discard_summons.iter().any(|descriptor| {
        descriptor["manaCost"] == 0 && descriptor["paymentMode"] == "random-card-discard"
    }));
    let discard = discard_summons
        .into_iter()
        .find(|descriptor| descriptor["paymentMode"] == "random-card-discard")
        .expect("random-card discard payment");
    let action = allowed_discard
        .legal_actions()
        .expect("Aramos legal actions")
        .into_iter()
        .find(|action| action.descriptor == discard)
        .expect("engine-issued random discard");
    let StepResult::Accepted(receipt) = allowed_discard
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("random discard summon")
    else {
        panic!("engine-issued random discard summon must be accepted");
    };
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["card-discarded", "minion-summoned"]
    );
    assert!(
        allowed_discard
            .verify_replay()
            .expect("verified random-discard replay")
    );

    let mut mana_site = site("earth");
    mana_site["genesisGainMana"] = json!(6);
    let blocked_sacrifice = sacrifice_board(&sacrifice_threshold_manifest(923, &mana_site));
    let blocked_board = state(&blocked_sacrifice);
    assert_eq!(
        blocked_board["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .filter(|unit| unit["cardId"] == "north-helper" && unit["location"] == "C4")
            .count(),
        2
    );
    assert!(summon_actions(&blocked_sacrifice, "north-gnarled").is_empty());
    assert!(
        blocked_sacrifice
            .verify_replay()
            .expect("verified blocked sacrifice replay")
    );

    let mut water_site = site("water");
    water_site["genesisGainMana"] = json!(6);
    let mut allowed_sacrifice = sacrifice_board(&sacrifice_threshold_manifest(924, &water_site));
    let sacrifice_summons = summon_actions(&allowed_sacrifice, "north-gnarled");
    assert!(sacrifice_summons.iter().any(|descriptor| {
        descriptor["manaCost"] == 4
            && descriptor["sacrificedMinionInstanceIds"]
                .as_array()
                .is_some_and(|ids| ids.len() == 1)
    }));
    let sacrifice = sacrifice_summons
        .into_iter()
        .find(|descriptor| {
            descriptor["manaCost"] == 4
                && descriptor["sacrificedMinionInstanceIds"]
                    .as_array()
                    .is_some_and(|ids| ids.len() == 1)
        })
        .expect("single-sacrifice payment");
    let action = allowed_sacrifice
        .legal_actions()
        .expect("Gnarled legal actions")
        .into_iter()
        .find(|action| action.descriptor == sacrifice)
        .expect("engine-issued sacrifice payment");
    let StepResult::Accepted(receipt) = allowed_sacrifice
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("sacrifice discount summon")
    else {
        panic!("engine-issued sacrifice discount summon must be accepted");
    };
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert!(
        allowed_sacrifice
            .verify_replay()
            .expect("verified sacrifice replay")
    );
}
#[test]
fn rule_catalog_2523_sacrifice_summoned_gnarled_stays_on_the_board_after_turns_pass() {
    let encoded = supplemental_seed_sacrifice(2523, 1, 1);
    let mut session = sacrifice_board_with_helpers(&encoded, 1);
    let helper_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["cardId"] == "north-helper" && unit["location"] == "C4")
        .expect("local helper")["instanceId"]
        .as_str()
        .expect("helper identity")
        .to_owned();
    let (gnarled_id, receipt) = accept_gnarled_sacrifice(&mut session, &helper_id);
    assert_eq!(
        event_types(&receipt),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert!(realm_unit(&state(&session), &helper_id).is_none());
    assert_eq!(unit(&state(&session), &gnarled_id)["location"], "C4");
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &gnarled_id)["location"], "C4");
    assert!(realm_unit(&state(&session), &gnarled_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2524_second_random_discard_offers_no_payment_after_discarding_the_only_hand_card() {
    let encoded = (2524..2524 + 8192)
        .chain(729..729 + 8192)
        .chain(922..922 + 8192)
        .find_map(|seed| try_second_discard_prefix(&discard_supplemental_manifest(seed)))
        .expect("bounded seed with two Aramos and one discard payment card");
    let mut session = opening_main(&encoded);
    accept_random_discard_summon(&mut session);
    assert!(
        !summon_actions(&session, "north-minion")
            .iter()
            .any(|descriptor| descriptor["paymentMode"] == "random-card-discard")
    );
    assert!(summon_actions(&session, "north-minion").is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2525_second_gnarled_sacrifices_a_newly_arrived_helper_after_enemy_site_placement() {
    let encoded = seed_for_second_gnarled_enemy_arrival(2525);
    let (mut session, helper_id) = try_second_gnarled_enemy_arrival_prefix(&encoded)
        .expect("second Gnarled enemy-arrival prefix");
    let (_, receipt) = accept_gnarled_sacrifice(&mut session, &helper_id);
    assert_eq!(
        event_types(&receipt),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert!(realm_unit(&state(&session), &helper_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2526_gnarled_offers_every_local_helper_sacrifice_option() {
    let encoded = supplemental_seed_sacrifice(2526, 1, 2);
    let session = sacrifice_board(&encoded);
    let helper_ids: Vec<_> = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| unit["cardId"] == "north-helper" && unit["location"] == "C4")
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("helper identity")
                .to_owned()
        })
        .collect();
    assert_eq!(helper_ids.len(), 2);
    let offered = gnarled_sacrifice_targets(&session);
    for helper_id in &helper_ids {
        assert!(offered.contains(helper_id));
    }
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2527_gnarled_sacrifice_leaves_a_far_helper_untouched() {
    let encoded = seed_for_far_helper(2527);
    let (mut session, near_id, far_id) =
        try_far_helper_prefix(&encoded).expect("far-helper prefix");
    let (_, receipt) = accept_gnarled_sacrifice(&mut session, &near_id);
    assert_eq!(
        event_types(&receipt),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert!(realm_unit(&state(&session), &near_id).is_none());
    assert!(realm_unit(&state(&session), &far_id).is_some());
    assert_eq!(unit(&state(&session), &far_id)["location"], "C1");
    assert!(!gnarled_sacrifice_targets(&session).contains(&far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2528_second_gnarled_sacrifices_a_newly_summoned_helper() {
    let encoded = seed_for_second_gnarled_new_summon(2528);
    let (mut session, helper_id) =
        try_second_gnarled_new_summon_prefix(&encoded).expect("second Gnarled new-summon prefix");
    let (_, receipt) = accept_gnarled_sacrifice(&mut session, &helper_id);
    assert_eq!(
        event_types(&receipt),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert!(realm_unit(&state(&session), &helper_id).is_none());
    assert_exact_replay(&session);
}
