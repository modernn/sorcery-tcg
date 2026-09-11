//! Direct proofs that Lucky Charm extra-random for discard-here uses a 2×2
//! source's whole footprint (RULE-CATALOG-0369–0370).
//!
//! Discard-funded random-here already hits every unit sharing any occupied
//! cell. Lucky Charm must offer those same candidates and then honor the
//! chosen outcome. A B3 occupant of an A3-anchored square is therefore a
//! committed extra-random choice, and a minion on C1 is not.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::ActionRequest;
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
    assert_eq!(offered_random_outcomes(&session), [shared_id.clone()]);

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
    assert_eq!(offered, [shared_id.clone()]);
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
