//! Direct proofs for Sparkmage activation (RULE-CATALOG-0149–0150, RULE-CATALOG-0723,
//! RULE-CATALOG-0802, RULE-CATALOG-1137, RULE-CATALOG-2493–2498).
//!
//! 0723 proves rubble-only nearby locations are valid activation targets. Supplemental
//! 2493–2498 bind rubble persistence, same-turn empty repeat, enemy arrival at rubble,
//! hidden multi-unit selection, far-location filtering, and a newly summoned rubble occupant.

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(sparkmage_ability: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    if sparkmage_ability {
        value["tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn"] = json!(true);
    }
    value
}

fn site(elements: &[&str]) -> Value {
    json!({ "cardType": "site", "elements": elements })
}

fn minion(air_threshold: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": air_threshold, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sparkmage-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sparkmage-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(true),
            "north-minion": minion(1),
            "north-site": site(&["air"]),
            "south-avatar": avatar(false),
            "south-minion": minion(0),
            "south-site": site(&["earth"]),
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
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let actions = session.legal_actions().expect("legal actions");
    let action = actions
        .iter()
        .find(|action| predicate(&action.descriptor))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action; available={:?}",
                actions
                    .iter()
                    .map(|action| &action.descriptor)
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

fn offers_kind(session: &Session, kind: &str) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions
            .iter()
            .any(|action| action.descriptor["kind"] == kind)
    })
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn first_main(seed: u32) -> Session {
    let manifest = manifest(seed);
    let mut session = Session::new(&manifest).expect("valid synthetic Sparkmage manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay value")["state"].clone()
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

#[test]
fn rule_catalog_0802_zero_air_threshold_taps_without_rng_or_damage() {
    let mut session = first_main(417);
    let actions = session.legal_actions().expect("Sparkmage actions");
    let activation = actions
        .iter()
        .find(|action| {
            action.descriptor["kind"] == "activate-sparkmage"
                && action.descriptor["targetLocation"]
                    == json!({ "cell": "C4", "region": "surface" })
        })
        .expect("zero-damage activation");
    assert_eq!(
        actions
            .iter()
            .find(|action| action.action_id == activation.action_id)
            .expect("stable issued action")
            .descriptor,
        activation.descriptor
    );
    assert!(activation.descriptor.get("targetInstanceId").is_none());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor == &activation.descriptor
    });
    assert_eq!(event_types(&receipt), ["sparkmage-activated"]);
    assert!(receipt.random_draws.is_empty());
    assert!(receipt.events[0].payload.get("targetInstanceId").is_none());

    let current = state(&session);
    assert_eq!(current["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(current["players"]["north"]["airThresholdsCastThisTurn"], 0);
    assert!(
        !session
            .legal_actions()
            .expect("post-activation actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "activate-sparkmage")
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one replayed scenario proves counter accumulation, random damage, reset, and zero damage"
)]
fn rule_catalog_0865_air_thresholds_select_hidden_candidate_and_reset() {
    let mut session = first_main(7);
    for expected_threshold in [1, 2] {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-minion"
                && descriptor["cell"] == "C4"
        });
        assert_eq!(
            state(&session)["players"]["north"]["airThresholdsCastThisTurn"],
            expected_threshold
        );
    }

    let before = state(&session);
    let mut candidates = before["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| unit["location"] == "C4" && unit["region"] == "surface")
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("unit identity")
                .to_owned()
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    assert_eq!(candidates.len(), 2);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-sparkmage"
            && descriptor["targetLocation"] == json!({ "cell": "C4", "region": "surface" })
    });
    assert!(descriptor.get("targetInstanceId").is_none());
    assert!(candidates.iter().all(|instance_id| {
        !canonical_json(&descriptor)
            .expect("canonical activation")
            .contains(instance_id)
    }));
    assert_eq!(
        event_types(&receipt),
        ["sparkmage-activated", "damage-dealt"]
    );
    assert!(!receipt.random_draws.is_empty());
    assert!(receipt.random_draws.iter().all(|draw| {
        draw["purpose"] == "sparkmage_random_other_unit_at_nearby_location"
            && draw["domain"]["kind"] == "unit_index_candidate"
            && draw["domain"]["exclusiveMaximum"] == 2
    }));
    let accepted_draw = receipt
        .random_draws
        .iter()
        .rev()
        .find(|draw| draw["domain"]["accepted"] == true)
        .expect("accepted random draw");
    let selected_index = usize::try_from(
        accepted_draw["result"].as_u64().expect("random uint32") % candidates.len() as u64,
    )
    .expect("candidate index");
    let selected = &candidates[selected_index];
    assert_eq!(
        receipt.events[0].payload["targetInstanceId"],
        selected.as_str()
    );
    assert_eq!(receipt.events[1].payload["instanceId"], selected.as_str());

    let damaged = state(&session);
    assert_eq!(
        damaged["realm"]["units"]
            .as_array()
            .expect("damaged units")
            .iter()
            .filter(|unit| unit["damage"] == 2)
            .count(),
        1
    );
    assert_eq!(damaged["players"]["north"]["airThresholdsCastThisTurn"], 2);

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        state(&session)["players"]["north"]["airThresholdsCastThisTurn"],
        0
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let damage_before_zero = state(&session)["realm"]["units"].clone();
    let (_, zero_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-sparkmage"
            && descriptor["targetLocation"] == json!({ "cell": "C4", "region": "surface" })
    });
    assert_eq!(event_types(&zero_receipt), ["sparkmage-activated"]);
    assert_eq!(zero_receipt.random_draws.len(), 1);
    assert!(
        zero_receipt.events[0]
            .payload
            .get("targetInstanceId")
            .is_some()
    );
    assert_eq!(state(&session)["realm"]["units"], damage_before_zero);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0723_rubble_at_nearby_location_is_valid_activate_sparkmage_target() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0723_rubble_at_nearby_location_is_valid_activate_sparkmage_target();
}

fn rain() -> Value {
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

fn deathrite_sparkmage_manifest(seed: u32) -> String {
    let fixture = "sparkmage-deathrite-withheld";
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(true),
            "north-rain": rain(),
            "north-site": site(&["air"]),
            "south-avatar": avatar(false),
            "south-minion": deathrite_minion(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-rain"; 6],
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
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

struct PendingDeathriteSparkmageSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_activate_sparkmage(
    encoded: &str,
) -> Option<PendingDeathriteSparkmageSetup> {
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
    if !offers_kind(&session, "activate-sparkmage") {
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
    Some(PendingDeathriteSparkmageSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_sparkmage_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_sparkmage_manifest)
        .find(|candidate| try_pending_deathrite_with_activate_sparkmage(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with legal activate-sparkmage")
}

#[test]
fn rule_catalog_1137_activate_sparkmage_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_sparkmage_seed_with(1137);
    let mut setup = try_pending_deathrite_with_activate_sparkmage(&encoded)
        .expect("complete activate-sparkmage Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["players"]["north"]["avatar"]["tapped"], false);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "activate-sparkmage")
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
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(resumed["players"]["north"]["avatar"]["tapped"], false);
    assert!(offers_kind(session, "activate-sparkmage"));
    assert_exact_replay(session);
}

fn harness_avatar() -> Value {
    let mut value = avatar(true);
    value["earthSitePlayCreatesAdjacentRubble"] = json!(true);
    value
}

fn fragile_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn sturdy_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn distant_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn visitor_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn destroy_spell() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn sparkmage_harness_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sparkmage-rubble-harness" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sparkmage-rubble-harness-v1",
        },
        "cards": {
            "north-avatar": harness_avatar(),
            "north-destroy": destroy_spell(),
            "north-minion": minion(1),
            "north-site": site(&["air"]),
            "south-avatar": avatar(false),
            "south-distant": distant_minion(),
            "south-fragile": fragile_minion(),
            "south-sturdy": sturdy_minion(),
            "south-site": site(&["earth"]),
            "south-visitor": visitor_minion(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-minion",
                    "north-destroy",
                    "north-minion",
                    "north-destroy",
                    "north-minion",
                    "north-destroy",
                    "north-minion",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-sturdy",
                    "south-sturdy",
                    "south-fragile",
                    "south-distant",
                    "south-visitor",
                    "south-sturdy",
                    "south-fragile",
                    "south-distant",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
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

fn seed_has_opening(encoded: &str, required_south: &[&str]) -> bool {
    let north = opening_spell_ids(encoded, "north");
    let south = opening_spell_ids(encoded, "south");
    north.iter().any(|card| card == "north-destroy")
        && north.iter().any(|card| card == "north-minion")
        && required_south
            .iter()
            .all(|id| south.iter().any(|card| card == id))
}

fn opening_count(encoded: &str, seat: &str, card_id: &str) -> usize {
    opening_spell_ids(encoded, seat)
        .iter()
        .filter(|card| *card == card_id)
        .count()
}

fn seed_with_start(start: u32, c3_card: &str, c1_card: &str) -> String {
    (start..start + 2048)
        .chain(723..723 + 2048)
        .map(sparkmage_harness_manifest)
        .find(|candidate| {
            seed_has_opening(candidate, &[c3_card, c1_card]) && {
                let mut session = opening_harness_main(candidate);
                try_setup_c3_and_c1(&mut session, c3_card, c1_card).is_some()
            }
        })
        .expect("bounded seed reaching C3 rubble harness setup")
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn rubble_at(snapshot: &Value, cell: &str) -> bool {
    snapshot["realm"]["sites"][cell]["rubble"] == json!(true)
}

fn opening_harness_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid Sparkmage harness session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn offers_play_site_at(session: &Session, cell: &str) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "play-site" && action.descriptor["cell"] == cell
        })
    })
}

fn end_turn_if_offered(session: &mut Session) {
    if offers_kind(session, "end-turn") {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    }
}

fn sparkmage_target_cells(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("Sparkmage actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "activate-sparkmage")
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn activate_sparkmage_at(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-sparkmage"
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetLocation"]["region"] == "surface"
    });
    receipt
}

fn try_summon_south_at(session: &mut Session, card_id: &str, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn try_build_air_threshold(session: &mut Session) -> Option<()> {
    if state(session)["players"]["north"]["airThresholdsCastThisTurn"].as_u64()? >= 1 {
        return Some(());
    }
    for cell in ["C4", "C1", "C2", "C3"] {
        if try_accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-minion"
                && descriptor["cell"] == cell
                && descriptor["region"].is_null()
        })
        .is_some()
        {
            return Some(());
        }
    }
    None
}

fn try_south_play_c3_occupant(session: &mut Session, c3_card: &str) -> Option<String> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_summon_south_at(session, c3_card, "C3")
}

fn try_south_draw_step(session: &mut Session) -> Option<()> {
    if try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })
    .is_some()
    {
        return Some(());
    }
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    Some(())
}

fn try_south_places_c3_occupant(session: &mut Session, c3_card: &str) -> Option<String> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_south_draw_step(session)?;
    if !offers_play_site_at(session, "C3") {
        try_accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        })?;
    }
    if !offers_play_site_at(session, "C3") && offers_play_site_at(session, "C2") {
        try_accept_where(session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
        })?;
        try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
        try_south_draw_step(session)?;
        if !offers_play_site_at(session, "C3") {
            try_accept_where(session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
            })?;
        }
    }
    try_south_play_c3_occupant(session, c3_card)
}

fn try_north_destroy_c3_site(session: &mut Session) -> Option<()> {
    let c3_site_id = state(session)["realm"]["sites"]["C3"]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "C3"
            && descriptor["targetSiteInstanceId"] == c3_site_id
    })?;
    rubble_at(&state(session), "C3").then_some(())
}

fn try_prepare_sparkmage_at_c3(session: &mut Session, c3_id: &str) -> Option<()> {
    try_north_destroy_c3_site(session)?;
    unit(&state(session), c3_id)?;
    try_build_air_threshold(session)?;
    sparkmage_target_cells(session)
        .contains(&"C3".to_owned())
        .then_some(())
}

fn south_turn_cycle(session: &mut Session) -> Option<()> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    Some(())
}

fn south_plays_site_at(session: &mut Session, cell: &str) -> Option<()> {
    south_turn_cycle(session)?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    })?;
    Some(())
}

fn try_opening_south_site_at(session: &mut Session, cell: &str) -> Option<()> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    })?;
    Some(())
}

fn try_setup_c3_and_c1(
    session: &mut Session,
    c3_card: &str,
    c1_card: &str,
) -> Option<(String, String)> {
    try_opening_south_site_at(session, "C1")?;
    let c1_id = try_summon_south_at(session, c1_card, "C1")?;
    south_plays_site_at(session, "C2")?;
    south_plays_site_at(session, "C3")?;
    let c3_id = try_summon_south_at(session, c3_card, "C3")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_prepare_sparkmage_at_c3(session, &c3_id)?;
    Some((c3_id, c1_id))
}

fn setup_c3_and_c1(session: &mut Session, c3_card: &str, c1_card: &str) -> (String, String) {
    try_setup_c3_and_c1(session, c3_card, c1_card).expect("C3 and C1 rubble harness setup")
}

fn pass_turn_to_north_sparkmage(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    });
}

fn south_hand_has(session: &Session, card_id: &str) -> bool {
    state(session)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == card_id))
}

fn try_second_sparkmage_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    if !seed_has_opening(
        encoded,
        &["south-fragile", "south-distant", "south-visitor"],
    ) || !opening_spell_ids(encoded, "south")
        .iter()
        .any(|card| card == "south-visitor")
    {
        return None;
    }
    let mut session = opening_harness_main(encoded);
    let (_, _) = try_setup_c3_and_c1(&mut session, "south-fragile", "south-distant")?;
    let first = activate_sparkmage_at(&mut session, "C3");
    if !event_types(&first).contains(&"minion-died") {
        return None;
    }
    pass_turn_to_north_sparkmage(&mut session);
    if !south_hand_has(&session, "south-visitor") {
        return None;
    }
    let visitor_id = try_south_places_c3_occupant(&mut session, "south-visitor")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_prepare_sparkmage_at_c3(&mut session, &visitor_id)?;
    Some((session, visitor_id))
}

fn seed_for_second_sparkmage_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(723..723 + 8192)
        .find_map(|seed| {
            let encoded = sparkmage_harness_manifest(seed);
            try_second_sparkmage_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Sparkmage enemy-arrival setup")
}

fn try_second_sparkmage_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    if !seed_has_opening(encoded, &["south-fragile", "south-distant", "south-sturdy"])
        || !opening_spell_ids(encoded, "south")
            .iter()
            .any(|card| card == "south-sturdy")
    {
        return None;
    }
    let mut session = opening_harness_main(encoded);
    let (fragile_id, _) = try_setup_c3_and_c1(&mut session, "south-fragile", "south-distant")?;
    let first = activate_sparkmage_at(&mut session, "C3");
    if !event_types(&first).contains(&"minion-died") {
        return None;
    }
    if unit(&state(&session), &fragile_id).is_some() {
        return None;
    }
    pass_turn_to_north_sparkmage(&mut session);
    if !south_hand_has(&session, "south-sturdy") {
        return None;
    }
    let new_id = try_south_places_c3_occupant(&mut session, "south-sturdy")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_prepare_sparkmage_at_c3(&mut session, &new_id)?;
    Some((session, new_id))
}

fn seed_for_second_sparkmage_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(723..723 + 8192)
        .find_map(|seed| {
            let encoded = sparkmage_harness_manifest(seed);
            try_second_sparkmage_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Sparkmage new-summon setup")
}

fn try_setup_two_at_c3(session: &mut Session) -> Option<(String, String)> {
    try_opening_south_site_at(session, "C1")?;
    south_plays_site_at(session, "C2")?;
    south_plays_site_at(session, "C3")?;
    let first_id = try_summon_south_at(session, "south-sturdy", "C3")?;
    let second_id = try_summon_south_at(session, "south-sturdy", "C3")?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_prepare_sparkmage_at_c3(session, &first_id)?;
    Some((first_id, second_id))
}

fn setup_two_at_c3(session: &mut Session) -> (String, String) {
    try_setup_two_at_c3(session).expect("C3 rubble with two occupants")
}

#[test]
fn rule_catalog_2493_rubble_target_and_damaged_unit_persist_after_turns_pass() {
    let encoded = seed_with_start(2493, "south-sturdy", "south-distant");
    let mut session = opening_harness_main(&encoded);
    let (sturdy_id, _) = setup_c3_and_c1(&mut session, "south-sturdy", "south-distant");
    let receipt = activate_sparkmage_at(&mut session, "C3");
    assert_eq!(
        event_types(&receipt),
        ["sparkmage-activated", "damage-dealt"]
    );
    let after = state(&session);
    assert!(rubble_at(&after, "C3"));
    let damaged = unit(&after, &sturdy_id).expect("surviving C3 occupant");
    assert_eq!(damaged["location"], "C3");
    assert_eq!(damaged["damage"], 1);
    pass_turn_to_north_sparkmage(&mut session);
    let later = state(&session);
    assert!(rubble_at(&later, "C3"));
    let stayed = unit(&later, &sturdy_id).expect("C3 occupant after turns");
    assert_eq!(stayed["location"], "C3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2494_second_activate_sparkmage_stays_unoffered_after_avatar_taps() {
    let encoded = seed_with_start(2494, "south-fragile", "south-distant");
    let mut session = opening_harness_main(&encoded);
    let (fragile_id, distant_id) = setup_c3_and_c1(&mut session, "south-fragile", "south-distant");
    let receipt = activate_sparkmage_at(&mut session, "C3");
    assert_eq!(
        event_types(&receipt),
        ["sparkmage-activated", "damage-dealt", "minion-died"]
    );
    assert!(unit(&state(&session), &fragile_id).is_none());
    assert!(unit(&state(&session), &distant_id).is_some());
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["tapped"],
        true
    );
    assert!(!offers_kind(&session, "activate-sparkmage"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2495_second_sparkmage_damages_a_newly_arrived_unit_at_rubble_c3() {
    let encoded = seed_for_second_sparkmage_enemy_arrival(2495);
    let (mut session, visitor_id) = try_second_sparkmage_enemy_arrival_prefix(&encoded)
        .expect("second Sparkmage enemy-arrival prefix");
    let receipt = activate_sparkmage_at(&mut session, "C3");
    assert_eq!(
        event_types(&receipt),
        ["sparkmage-activated", "damage-dealt"]
    );
    let after = state(&session);
    let arrived = unit(&after, &visitor_id).expect("arrived C3 occupant");
    assert_eq!(arrived["location"], "C3");
    assert_eq!(arrived["damage"], 1);
    assert!(rubble_at(&after, "C3"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2496_sparkmage_offers_one_hidden_activation_for_multiple_c3_occupants() {
    let encoded = (2496..2496 + 2048)
        .chain(723..723 + 2048)
        .map(sparkmage_harness_manifest)
        .find(|candidate| {
            seed_has_opening(candidate, &["south-sturdy"])
                && opening_count(candidate, "south", "south-sturdy") >= 2
                && {
                    let mut session = opening_harness_main(candidate);
                    try_setup_two_at_c3(&mut session).is_some()
                }
        })
        .expect("bounded seed with two sturdy opening minions");
    let mut session = opening_harness_main(&encoded);
    let (first_id, second_id) = setup_two_at_c3(&mut session);
    let activations: Vec<_> = session
        .legal_actions()
        .expect("Sparkmage actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "activate-sparkmage"
                && action.descriptor["targetLocation"]["cell"] == "C3"
        })
        .collect();
    assert_eq!(activations.len(), 1);
    assert!(activations[0].descriptor.get("targetInstanceId").is_none());
    let receipt = activate_sparkmage_at(&mut session, "C3");
    assert_eq!(
        event_types(&receipt),
        ["sparkmage-activated", "damage-dealt"]
    );
    assert!(!receipt.random_draws.is_empty());
    let selected = receipt.events[0].payload["targetInstanceId"]
        .as_str()
        .expect("hidden target");
    assert!(selected == first_id || selected == second_id);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2497_sparkmage_at_rubble_c3_leaves_a_far_c1_minion_untouched() {
    let encoded = seed_with_start(2497, "south-sturdy", "south-distant");
    let mut session = opening_harness_main(&encoded);
    let (c3_id, distant_id) = setup_c3_and_c1(&mut session, "south-sturdy", "south-distant");
    let receipt = activate_sparkmage_at(&mut session, "C3");
    assert_eq!(
        event_types(&receipt),
        ["sparkmage-activated", "damage-dealt"]
    );
    let after = state(&session);
    let lashed = unit(&after, &c3_id).expect("C3 occupant");
    assert_eq!(lashed["location"], "C3");
    assert_eq!(lashed["damage"], 1);
    let far = unit(&after, &distant_id).expect("far C1 minion");
    assert_eq!(far["location"], "C1");
    assert!(far["damage"] == 0 || far["damage"].is_null());
    assert!(!sparkmage_target_cells(&session).contains(&"C1".to_owned()));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2498_second_sparkmage_damages_a_newly_summoned_unit_at_rubble_c3() {
    let encoded = seed_for_second_sparkmage_new_summon(2498);
    let (mut session, new_id) = try_second_sparkmage_new_summon_prefix(&encoded)
        .expect("second Sparkmage new-summon prefix");
    let receipt = activate_sparkmage_at(&mut session, "C3");
    assert_eq!(
        event_types(&receipt),
        ["sparkmage-activated", "damage-dealt"]
    );
    let after = state(&session);
    let summoned = unit(&after, &new_id).expect("new C3 occupant");
    assert_eq!(summoned["location"], "C3");
    assert_eq!(summoned["damage"], 1);
    assert!(rubble_at(&after, "C3"));
    assert_exact_replay(&session);
}
