//! Direct proofs for kill-target-wounded-minion Magic (RULE-CATALOG-0609–0610,
//! RULE-CATALOG-0721, RULE-CATALOG-1021, RULE-CATALOG-1091, RULE-CATALOG-2013–2018,
//! RULE-CATALOG-2483–2488).
//!
//! Fatality kills only a wounded minion in the caster region. Healthy minions
//! are never offered as legal targets. Enemy Stealth and underground region
//! filter wounded copies; Ward absorbs without killing. Allied wounded minions
//! remain legal even when stealthed. Supplemental 2483–2488 bind persistence,
//! empty-repeat after legal copies die, enemy-arrival, multi-legal, far Stealth,
//! and a newly summoned unfiltered wounded minion. Distinct from 2013–2018
//! (wounded versus healthy) and 2333–2338 (region only). While Deathrites wait
//! for ordering, Fatality Magic stays withheld until the chain drains.

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

fn minion(deathrite: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if deathrite {
        value["deathriteDrawSite"] = json!(true);
    }
    value
}

fn fatality() -> Value {
    json!({
        "cardType": "magic",
        "killTargetWoundedMinion": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
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

fn fatality_manifest(seed: u32, deathrite: bool) -> String {
    let fixture = if deathrite {
        "fatality-deathrite-draw"
    } else {
        "fatality-rules"
    };
    let south_atlas = if deathrite { 4 } else { 6 };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fatality": fatality(),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(deathrite),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-lash", "north-lash", "north-lash", "north-fatality", "north-fatality", "north-fatality"],
            },
            "south": {
                "atlas": vec!["south-site"; south_atlas],
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

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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
    let mut session = Session::new(encoded).expect("valid fatality session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
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

fn fatality_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("fatality actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fatality"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn stage_two_enemies(session: &mut Session) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut enemy_ids = Vec::new();
    for _ in 0..2 {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cell"] == "C1"
                && descriptor["region"].is_null()
        });
        enemy_ids.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("summoned enemy identity")
                .to_owned(),
        );
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_ids
}

fn seed_with_both_magic_cards(deathrite: bool, start: u32) -> String {
    (start..start + 512)
        .map(|seed| fatality_manifest(seed, deathrite))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Fatality seed candidate");
            let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand")
                .clone();
            ["north-fatality", "north-lash"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("bounded seed with both North Magic cards in hand")
}

#[test]
fn rule_catalog_0609_fatality_kills_a_wounded_minion_in_the_caster_region() {
    let encoded = seed_with_both_magic_cards(false, 609);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_two_enemies(&mut session);
    let wounded_id = enemy_ids[0].clone();
    let healthy_id = enemy_ids[1].clone();

    assert!(fatality_targets(&session).is_empty());
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == wounded_id.as_str()
    });
    let wounded_only = fatality_targets(&session);
    assert_eq!(wounded_only, [wounded_id.as_str()]);
    assert!(!wounded_only.contains(&healthy_id));

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fatality"
    });
    assert_eq!(descriptor["target"]["instanceId"], wounded_id.as_str());
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );

    let finished = state(&session);
    assert!(realm_unit(&finished, &wounded_id).is_none());
    assert_eq!(
        realm_unit(&finished, &healthy_id).expect("survivor")["damage"],
        0
    );
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == wounded_id.as_str())
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0610_fatality_offers_no_target_when_every_minion_is_healthy() {
    let encoded = seed_with_both_magic_cards(false, 609);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_two_enemies(&mut session);

    assert!(fatality_targets(&session).is_empty());
    let fatality_casts: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fatality"
        })
        .collect();
    assert_eq!(fatality_casts.len(), 0);
    for enemy_id in &enemy_ids {
        assert_eq!(
            realm_unit(&state(&session), enemy_id).expect("healthy enemy")["damage"],
            0
        );
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0721_fatality_breaks_ward_and_filters_healthy_stealthed_and_underground_copies() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0721_fatality_breaks_ward_and_filters_healthy_stealthed_and_underground_copies();
}

#[test]
fn rule_catalog_2483_fatality_filter_matrix_persists_after_turns_pass() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2483_fatality_filter_matrix_persists_after_turns_pass();
}

#[test]
fn rule_catalog_2484_second_fatality_offers_no_targets_after_legal_filter_copies_die() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2484_second_fatality_offers_no_targets_after_legal_filter_copies_die();
}

#[test]
fn rule_catalog_2485_second_fatality_kills_a_newly_arrived_unfiltered_wounded_minion() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2485_second_fatality_kills_a_newly_arrived_unfiltered_wounded_minion();
}

#[test]
fn rule_catalog_2486_fatality_offers_every_legal_filter_matrix_copy() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2486_fatality_offers_every_legal_filter_matrix_copy();
}

#[test]
fn rule_catalog_2487_fatality_leaves_a_far_stealthed_wounded_copy_untouched() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2487_fatality_leaves_a_far_stealthed_wounded_copy_untouched();
}

#[test]
fn rule_catalog_2488_second_fatality_kills_a_newly_summoned_unfiltered_wounded_minion() {
    sorcery_engine::game::catalog_proofs::rule_catalog_2488_second_fatality_kills_a_newly_summoned_unfiltered_wounded_minion();
}

#[test]
fn rule_catalog_1021_kill_wounded_minion_deathrite_draws_for_controller_on_kill() {
    let encoded = seed_with_both_magic_cards(true, 1021);
    let mut session = opening_main(&encoded);
    let enemy_ids = stage_two_enemies(&mut session);
    let wounded_id = enemy_ids[0].clone();
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    assert_eq!(
        south_atlas, 1,
        "thin South atlas leaves one site before the Fatality kill"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == wounded_id.as_str()
    });
    assert_eq!(fatality_targets(&session), [wounded_id.as_str()]);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fatality"
    });
    let types = event_types(&receipt);
    assert_eq!(
        types,
        [
            "magic-cast",
            "minion-killed",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        site_drawn < magic_resolved,
        "expected site-drawn before magic-resolved; got {types:?}"
    );

    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], wounded_id);

    let finished = state(&session);
    assert!(realm_unit(&finished, &wounded_id).is_none());
    assert_eq!(
        realm_unit(&finished, &enemy_ids[1]).expect("healthy survivor")["damage"],
        0
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}

fn deathrite_fatality_manifest(seed: u32) -> String {
    let fixture = "fatality-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fatality": fatality(),
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
                    "north-fatality",
                    "north-rain",
                    "north-rain",
                    "north-fatality",
                    "north-rain",
                    "north-fatality",
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

fn north_has_fatality_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-fatality", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteFatalitySetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_wounded_visitor(
    encoded: &str,
) -> Option<PendingDeathriteFatalitySetup> {
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
    if !north_has_fatality_and_rain(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    realm_unit(&state(&session), &visitor_id)?;
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteFatalitySetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_fatality_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_fatality_manifest)
        .find(|candidate| try_pending_deathrite_with_wounded_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Fatality Magic in hand")
}

#[test]
fn rule_catalog_1091_fatality_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_fatality_seed_with(1091);
    let mut setup = try_pending_deathrite_with_wounded_visitor(&encoded)
        .expect("complete Fatality Deathrite withheld setup");
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
    assert_eq!(
        realm_unit(&paused, &visitor_id).expect("wounded visitor")["damage"],
        1
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(fatality_targets(session).is_empty());

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
    assert_eq!(fatality_targets(session), [visitor_id.as_str()]);

    let (descriptor, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fatality"
            && descriptor["target"]["instanceId"] == visitor_id.as_str()
    });
    assert_eq!(descriptor["target"]["instanceId"], visitor_id.as_str());
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert!(realm_unit(&state(session), &visitor_id).is_none());
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

fn fatality_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "fatality-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-fatality-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fatality": fatality(),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": supplemental_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": std::iter::repeat_n("north-lash", 8)
                    .chain(std::iter::repeat_n("north-fatality", 8))
                    .collect::<Vec<_>>(),
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
        .chain(609..609 + 2048)
        .map(fatality_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-fatality")
                && opening_hand_spell_ids(candidate, "north")
                    .iter()
                    .any(|card| card == "north-lash")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Fatality, Lash, and required South minions")
}

fn fatality_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-fatality")
                .count()
        })
        .unwrap_or_default()
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
        .expect("Fatality target identity")
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

fn lash_target(session: &mut Session, target_id: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == target_id
    });
}

fn cast_fatality_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fatality"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
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
    (lash_spells_in_hand(&state(&session)) >= 1).then_some((session, c2_ids[0].clone(), far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(609..609 + 2048)
        .find_map(|seed| {
            let encoded = fatality_supplemental_manifest(seed);
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
        .expect("bounded seed reaching Fatality far-minion setup")
}

fn try_second_fatality_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    lash_target(&mut session, &first_id);
    cast_fatality_on(&mut session, &first_id);
    pass_turn_to_north_spellbook(&mut session);
    if fatality_spells_in_hand(&state(&session)) < 1 || lash_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
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
    lash_target(&mut session, &minion_id);
    fatality_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_fatality_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(609..609 + 8192)
        .find_map(|seed| {
            let encoded = fatality_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_fatality_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Fatality enemy-arrival setup")
}

fn try_second_fatality_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    lash_target(&mut session, &first_id);
    cast_fatality_on(&mut session, &first_id);
    if realm_unit(&state(&session), &first_id).is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if fatality_spells_in_hand(&state(&session)) < 1 || lash_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
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
    lash_target(&mut session, &minion_id);
    fatality_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_fatality_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(609..609 + 8192)
        .find_map(|seed| {
            let encoded = fatality_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_fatality_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Fatality new-summon setup")
}

#[test]
fn rule_catalog_2013_wounded_minion_stays_at_the_location_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2013, 1);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    lash_target(&mut session, &minion_id);
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 1);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C2");
    assert!(realm_unit(&state(&session), &minion_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2014_second_fatality_offers_no_targets_after_killing_the_only_wounded_minion() {
    let encoded = (2014..2014 + 8192)
        .chain(609..609 + 8192)
        .find_map(|seed| {
            let candidate = fatality_supplemental_manifest(seed);
            let mut session = opening_main(&candidate);
            let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
            north_draws_spellbook(&mut session);
            lash_target(&mut session, &minion_id);
            let first = cast_fatality_on(&mut session, &minion_id);
            if !event_types(&first).contains(&"minion-died") {
                return None;
            }
            if realm_unit(&state(&session), &minion_id).is_some() {
                return None;
            }
            (fatality_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two Fatality casts after clearing wounded minions");
    let mut session = opening_main(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    lash_target(&mut session, &minion_id);
    let first = cast_fatality_on(&mut session, &minion_id);
    assert!(event_types(&first).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(fatality_spells_in_hand(&state(&session)) >= 1);
    assert!(fatality_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fatality"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2015_second_fatality_kills_a_newly_arrived_wounded_minion_after_enemy_site_placement()
 {
    let encoded = seed_for_second_fatality_enemy_arrival(2015);
    let (mut session, minion_id) = try_second_fatality_enemy_arrival_prefix(&encoded)
        .expect("second Fatality enemy-arrival prefix");
    let receipt = cast_fatality_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2016_fatality_offers_every_wounded_minion_in_the_caster_region() {
    let encoded = supplemental_seed_with_start(2016, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c2_with_south_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    for minion_id in &minion_ids {
        lash_target(&mut session, minion_id);
    }
    let offered = fatality_targets(&session);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2017_fatality_leaves_a_far_healthy_minion_untouched() {
    let encoded = seed_for_far_minion(2017);
    let (mut session, wounded_id, far_id) =
        try_far_minion_prefix(&encoded).expect("Fatality far-minion prefix");
    lash_target(&mut session, &wounded_id);
    let receipt = cast_fatality_on(&mut session, &wounded_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &wounded_id).is_none());
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2018_second_fatality_kills_a_newly_summoned_wounded_minion() {
    let encoded = seed_for_second_fatality_new_summon(2018);
    let (mut session, minion_id) =
        try_second_fatality_new_summon_prefix(&encoded).expect("second Fatality new-summon prefix");
    let receipt = cast_fatality_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == minion_id))
    );
    assert_exact_replay(&session);
}
