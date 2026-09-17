//! Direct proofs that Lucky Charm extra-random for discard-here uses a 2×2
//! source's whole footprint (RULE-CATALOG-0369–0370), that
//! activate-discard-random-damage stays withheld during deathrite-order
//! (RULE-CATALOG-1147), and that resolve-random-outcome stays withheld while
//! deathrite-order interrupts a pending Lucky Charm random-choice
//! (RULE-CATALOG-1167).
//!
//! Discard-funded random-here already hits every unit sharing any occupied
//! cell. Lucky Charm must offer those same candidates and then honor the
//! chosen outcome. A B3 occupant of an A3-anchored square is therefore a
//! committed extra-random choice, and a minion on C1 is not. Deathrite-order
//! still blocks the activation until the pending chain drains. When a nearby
//! power bonus drops during random-choice, Deathrites settle first and the
//! extra-random actions return only after the order drains.

use serde_json::{Value, json};
use sorcery_engine::action::ActionDescriptor;
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::ActionRequest;
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

fn earth() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn giant() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "discardSpellToDamageRandomOtherUnitHere": 3,
        "manaCost": 0,
        "occupiesSquareArea": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn charm() -> Value {
    json!({
        "bearerControllerChoosesExtraRandomOutcome": true,
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 9,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lucky-charm-footprint" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lucky-charm-footprint-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charm": charm(),
            "north-earth": earth(),
            "north-giant": giant(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-charm",
                    "north-giant",
                    "north-giant",
                    "north-giant",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-raider"; 16],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Value {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            let current = session.replay_value().expect("replay");
            panic!(
                "expected engine-issued action in phase {} among {:?}",
                current["state"]["phase"],
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
        });
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(_) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine-issued action must be accepted");
    };
    descriptor
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn play_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn end_and_draw_zone(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn end_and_draw(session: &mut Session) {
    end_and_draw_zone(session, "spellbook");
}

fn opening_ids(session: &Session, zone: &str) -> Vec<String> {
    session.replay_value().expect("authoritative replay")["state"]["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> Option<&'a Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn unit_at(current: &Value, cell: &str) -> String {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["location"] == cell)
        .and_then(|unit| unit["instanceId"].as_str())
        .expect("minion at cell")
        .to_owned()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
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

fn opening() -> Session {
    (1..=4096)
        .map(manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("lucky charm footprint candidate");
            let atlas = opening_ids(&session, "atlas");
            let spells = opening_ids(&session, "spellbook");
            (atlas.iter().filter(|card| *card == "north-earth").count() >= 3
                && spells.contains(&"north-charm".to_owned())
                && spells.contains(&"north-giant".to_owned()))
            .then_some(session)
        })
        .expect("bounded seed opening with earth, Lucky Charm, and a 2x2 discard source")
}

fn establish_a3_square(session: &mut Session) {
    keep(session);
    keep(session);
    play_site(session, "north-earth", "C4");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-charm"
            && descriptor["bearer"]["kind"] == "avatar"
    });
    end_and_draw(session);
    play_site(session, "south-site", "C1");
    end_and_draw(session);
    play_site(session, "north-earth", "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "north-earth", "B3");
}

fn south_summon(session: &mut Session, cell: &str) -> String {
    let summoned = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == cell
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("south identity")
        .to_owned()
}

fn finish_a3_square_and_summon_giant(session: &mut Session) -> String {
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "A4");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "north-earth", "A3");
    let summoned = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "A3"
            && descriptor["region"].is_null()
    });
    let giant_id = summoned["cardInstanceId"]
        .as_str()
        .expect("2x2 identity")
        .to_owned();
    let current = state(session);
    let occupant = realm_unit(&current, &giant_id).expect("2x2 remains in play");
    assert_eq!(occupant["location"], "A3");
    assert_eq!(occupant["occupiedCells"], json!(["A3", "A4", "B3", "B4"]));
    giant_id
}

fn offered_random_outcomes(session: &Session) -> Vec<String> {
    let mut outcomes: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-random-outcome")
        .map(|action| {
            action.descriptor["outcomeInstanceId"]
                .as_str()
                .expect("outcome identity")
                .to_owned()
        })
        .collect();
    outcomes.sort();
    outcomes.dedup();
    outcomes
}

fn activate_discard_here(session: &mut Session, source_instance_id: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-discard-random-damage"
            && descriptor["sourceInstanceId"] == source_instance_id
    });
}

#[test]
fn rule_catalog_0369_lucky_charm_offers_unit_sharing_occupied_non_anchor_cell() {
    let mut session = opening();
    establish_a3_square(&mut session);
    end_and_draw(&mut session);
    let shared_id = south_summon(&mut session, "B3");
    let giant_id = finish_a3_square_and_summon_giant(&mut session);
    activate_discard_here(&mut session, &giant_id);

    assert_eq!(state(&session)["phase"], "random-choice");
    let commit = session.transcript().last().expect("Lucky Charm commit");
    assert!(commit.events.is_empty());
    assert_eq!(commit.random_draws.len(), 2);
    assert!(
        commit
            .random_draws
            .iter()
            .all(|draw| draw["purpose"] == "discard_spell_random_other_unit_here")
    );
    assert_eq!(
        commit.random_draws[0]["domain"]["exclusiveMaximum"], 1,
        "only the B3 occupant shares the A3 square"
    );
    assert_eq!(
        offered_random_outcomes(&session).as_slice(),
        std::slice::from_ref(&shared_id)
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-random-outcome"
            && descriptor["outcomeInstanceId"] == shared_id
    });
    let resolution = session.transcript().last().expect("honored outcome");
    let allocated = resolution
        .events
        .iter()
        .find(|event| event.event_type == "discard-random-damage-allocated")
        .expect("discard-here allocation");
    assert_eq!(allocated.payload["targetInstanceId"], shared_id);
    assert_eq!(allocated.payload["amount"], 3);
    let current = state(&session);
    assert_eq!(
        realm_unit(&current, &shared_id).expect("survivor")["damage"],
        3
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0370_lucky_charm_excludes_unit_outside_discard_source_footprint() {
    let mut session = opening();
    establish_a3_square(&mut session);
    end_and_draw(&mut session);
    let shared_id = south_summon(&mut session, "B3");
    let outsider_id = south_summon(&mut session, "C1");
    let giant_id = finish_a3_square_and_summon_giant(&mut session);
    activate_discard_here(&mut session, &giant_id);

    assert_eq!(state(&session)["phase"], "random-choice");
    let offered = offered_random_outcomes(&session);
    assert_eq!(offered.as_slice(), std::slice::from_ref(&shared_id));
    assert!(!offered.contains(&outsider_id));
    assert_eq!(unit_at(&state(&session), "C1"), outsider_id);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-random-outcome"
            && descriptor["outcomeInstanceId"] == shared_id
    });
    let current = state(&session);
    assert_eq!(
        realm_unit(&current, &shared_id).expect("chosen target")["damage"],
        3
    );
    assert_eq!(
        realm_unit(&current, &outsider_id).expect("outside the square")["damage"],
        0
    );
    assert_exact_replay(&session);
}

fn rain() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite() -> Value {
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

fn deathrite_giant() -> Value {
    let mut value = giant();
    value["defense"] = json!(9);
    value
}

fn try_accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Option<Value> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(_) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some(descriptor)
}

fn offers_activate_discard(session: &Session, source_instance_id: &str) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "activate-discard-random-damage"
                && action.descriptor["sourceInstanceId"] == source_instance_id
        })
    })
}

fn north_has_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
}

fn north_spell_count(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map_or(0, Vec::len)
}

fn deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lucky-charm-footprint-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lucky-charm-footprint-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charm": charm(),
            "north-earth": earth(),
            "north-giant": deathrite_giant(),
            "north-rain": rain(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-giant",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 16],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct PendingDeathriteActivateDiscardSetup {
    deathrite_ids: [String; 2],
    giant_id: String,
    session: Session,
}

#[expect(
    clippy::too_many_lines,
    reason = "Lucky Charm activate-discard Deathrite withhold setup keeps branch steps inline"
)]
fn try_pending_deathrite_with_activate_discard(
    encoded: &str,
) -> Option<PendingDeathriteActivateDiscardSetup> {
    let mut session = Session::new(encoded).ok()?;
    let atlas = opening_ids(&session, "atlas");
    let spells = opening_ids(&session, "spellbook");
    if atlas.iter().filter(|card| *card == "north-earth").count() < 3
        || !spells.contains(&"north-charm".to_owned())
        || !spells.contains(&"north-giant".to_owned())
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-charm"
            && descriptor["bearer"]["kind"] == "avatar"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "B4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "B3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "A4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "A3"
    })?;
    let summoned = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "A3"
            && descriptor["region"].is_null()
    })?;
    let giant_id = summoned["cardInstanceId"].as_str()?.to_owned();
    let current = state(&session);
    let occupant = realm_unit(&current, &giant_id)?;
    if occupant["location"] != "A3" || occupant["occupiedCells"] != json!(["A3", "A4", "B3", "B4"])
    {
        return None;
    }
    if !north_has_rain(&current) || north_spell_count(&current) < 2 {
        return None;
    }
    if !offers_activate_discard(&session, &giant_id) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first["cardInstanceId"].as_str()?.to_owned(),
        second["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteActivateDiscardSetup {
        deathrite_ids,
        giant_id,
        session,
    })
}

fn deathrite_activate_discard_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_manifest)
        .find(|candidate| try_pending_deathrite_with_activate_discard(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Lucky Charm discard-here ready")
}

#[test]
fn rule_catalog_1147_activate_discard_random_damage_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_activate_discard_seed_with(1147);
    let mut setup = try_pending_deathrite_with_activate_discard(&encoded)
        .expect("complete Lucky Charm activate-discard-random-damage Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let giant_id = setup.giant_id.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    let giant = realm_unit(&paused, &giant_id).expect("2x2 remains in play");
    assert_eq!(giant["location"], "A3");
    assert_eq!(giant["occupiedCells"], json!(["A3", "A4", "B3", "B4"]));
    assert_eq!(giant["damage"], 1);
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
            .all(|action| action.descriptor["kind"] != "activate-discard-random-damage")
    );
    assert!(!offers_activate_discard(session, &giant_id));

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
    let giant = realm_unit(&resumed, &giant_id).expect("2x2 still occupies the square");
    assert_eq!(giant["occupiedCells"], json!(["A3", "A4", "B3", "B4"]));
    assert_eq!(giant["damage"], 1);
    assert!(
        offers_activate_discard(session, &giant_id),
        "activate-discard-random-damage returns after Deathrites drain"
    );
    assert_exact_replay(session);
}

fn power_bonus_aura() -> Value {
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

fn random_choice_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lucky-charm-random-choice-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lucky-charm-random-choice-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charm": charm(),
            "north-earth": earth(),
            "north-giant": deathrite_giant(),
            "north-rain": rain(),
            "south-aura": power_bonus_aura(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": earth(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-charm",
                    "north-giant",
                    "north-rain",
                    "north-giant",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-deathrite",
                    "south-deathrite",
                    "south-aura",
                    "south-deathrite",
                    "south-deathrite",
                    "south-aura",
                    "south-deathrite",
                    "south-deathrite",
                    "south-aura",
                    "south-deathrite",
                    "south-deathrite",
                    "south-aura",
                    "south-deathrite",
                    "south-deathrite",
                    "south-aura",
                    "south-deathrite",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

struct PendingRandomChoiceDeathriteSetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

#[expect(
    clippy::too_many_lines,
    reason = "Lucky Charm random-choice Deathrite withhold setup keeps branch steps inline"
)]
fn try_pending_random_choice_before_deathrite_order(
    encoded: &str,
) -> Option<PendingRandomChoiceDeathriteSetup> {
    let mut session = Session::new(encoded).ok()?;
    let atlas = opening_ids(&session, "atlas");
    let spells = opening_ids(&session, "spellbook");
    if atlas.iter().filter(|card| *card == "north-earth").count() < 3
        || !spells.contains(&"north-charm".to_owned())
        || !spells.contains(&"north-giant".to_owned())
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-charm"
            && descriptor["bearer"]["kind"] == "avatar"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "B4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "B3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "B3"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "B3"
            && descriptor["region"].is_null()
    })?;
    let aura = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-aura"
            && descriptor["cell"] == "B3"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "A4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "A3"
    })?;
    let summoned = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "A3"
            && descriptor["region"].is_null()
    })?;
    let giant_id = summoned["cardInstanceId"].as_str()?.to_owned();
    let current = state(&session);
    let occupant = realm_unit(&current, &giant_id)?;
    if occupant["location"] != "A3" || occupant["occupiedCells"] != json!(["A3", "A4", "B3", "B4"])
    {
        return None;
    }
    if !north_has_rain(&current) || north_spell_count(&current) < 2 {
        return None;
    }
    if !offers_activate_discard(&session, &giant_id) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    let wounded = state(&session);
    if wounded["phase"] != "main" {
        return None;
    }
    let aura_id = aura["cardInstanceId"].as_str()?.to_owned();
    let mut deathrite_ids = [
        first["cardInstanceId"].as_str()?.to_owned(),
        second["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    if deathrite_ids
        .iter()
        .any(|instance_id| realm_unit(&wounded, instance_id).is_none_or(|unit| unit["damage"] != 1))
        || realm_unit(&wounded, &aura_id).is_none_or(|unit| unit["damage"] != 1)
    {
        return None;
    }
    if !offers_activate_discard(&session, &giant_id) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-discard-random-damage"
            && descriptor["sourceInstanceId"] == giant_id
    })?;
    let choosing = state(&session);
    if choosing["phase"] != "random-choice"
        || offered_random_outcomes(&session).is_empty()
        || realm_unit(&choosing, &aura_id).is_none()
        || deathrite_ids
            .iter()
            .any(|instance_id| realm_unit(&choosing, instance_id).is_none())
    {
        return None;
    }
    Some(PendingRandomChoiceDeathriteSetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

fn random_choice_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(random_choice_deathrite_manifest)
        .find(|candidate| try_pending_random_choice_before_deathrite_order(candidate).is_some())
        .expect("bounded seed that reaches Lucky Charm random-choice with wounded Deathrites")
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "random-choice Deathrite withhold scenario proof keeps assertions inline"
)]
fn rule_catalog_1167_random_choice_withheld_during_pending_deathrite_order() {
    let encoded = random_choice_deathrite_seed_with(1167);
    let setup = try_pending_random_choice_before_deathrite_order(&encoded)
        .expect("complete Lucky Charm random-choice Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let choosing = state(&setup.session);
    assert_eq!(choosing["phase"], "random-choice");
    assert_eq!(choosing["decisionSeat"], "north");
    assert!(!offered_random_outcomes(&setup.session).is_empty());
    assert_exact_replay(&setup.session);

    let mut branched = replay_game(&setup.session);
    assert!(
        branched.test_remove_realm_unit(&aura_id),
        "checkpoint branch must drop the power-bonus ally so wounded Deathrites settle"
    );
    let choice = branched
        .legal_actions()
        .expect("Lucky Charm choices after aura removal")
        .into_iter()
        .find(|action| {
            matches!(
                action.descriptor(),
                ActionDescriptor::ResolveRandomOutcome { .. }
            )
        })
        .expect("resolve-random-outcome remains issued before Deathrites");
    branched
        .apply_action(&choice)
        .expect("resolve-random-outcome before Deathrites");

    let paused = branched.authoritative_state();
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["pendingDeathrites"]["returnPhase"], "random-choice");
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
            .all(|action| {
                !matches!(
                    action.descriptor(),
                    ActionDescriptor::ResolveRandomOutcome { .. }
                )
            }),
        "deathrite-order must issue no resolve-random-outcome"
    );

    let order_sources: Vec<_> = branched
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| issued_descriptor(action)["kind"] == "order-deathrites")
        .map(|action| {
            issued_descriptor(&action)["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);
    let order = branched
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .find(|action| {
            let descriptor = issued_descriptor(action);
            descriptor["kind"] == "order-deathrites"
                && descriptor["sourceInstanceId"] == deathrite_ids[0]
        })
        .expect("order first Deathrite");
    branched.apply_action(&order).expect("order Deathrites");

    let resumed = branched.authoritative_state();
    assert_eq!(resumed["phase"], "random-choice");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        branched
            .legal_actions()
            .expect("resumed Lucky Charm choices")
            .iter()
            .any(|action| matches!(
                action.descriptor(),
                ActionDescriptor::ResolveRandomOutcome { .. }
            )),
        "resolve-random-outcome must return once deathrite-order clears"
    );
}
