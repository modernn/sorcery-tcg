//! Grant combat modifier Magic self-play admission matrix (RULE-CATALOG-0734)
//! and composed grant-Airborne-this-turn then draw-spell runtime proofs beyond
//! admission (RULE-CATALOG-0533–0534 and RULE-CATALOG-0734).

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

#[test]
fn rule_catalog_0734_grant_combat_modifier_magic_admits_common_minion_slices() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0734_grant_combat_modifier_magic_admits_common_minion_slices(
    );
}

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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn gift() -> Value {
    json!({
        "cardType": "magic",
        "grantAirborneToAllyThisTurnThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lethal_gift() -> Value {
    json!({
        "cardType": "magic",
        "grantLethalToAllyThisTurnThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn movement_gift() -> Value {
    json!({
        "cardType": "magic",
        "grantMovementOneToAllyThisTurnThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn power_gift() -> Value {
    json!({
        "cardType": "magic",
        "grantPowerTwoToAllyThisTurnThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn stealth_gift() -> Value {
    json!({
        "cardType": "magic",
        "grantStealthToAlliedMinionsThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn fade_gift() -> Value {
    json!({
        "cardType": "magic",
        "grantStealthToAlliedMinionOccupyingEnemySiteThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn cemetery_bottom_gift() -> Value {
    json!({
        "cardType": "magic",
        "returnUpToThreeCemeteryCardsToDeckBottomThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
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

fn empty_library_manifest(seed: u32, gift_card: &Value, fixture: &str, revision: &str) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": revision,
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-gift": gift_card.clone(),
            "north-site": site(),
            "south-ally": ally(),
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-gift", "north-gift"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-ally"; 6],
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
    let mut session = Session::new(encoded).expect("valid grant-airborne-then-draw session");
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

fn seed_with_ally_and_gift(start: u32, gift_card: &Value, fixture: &str, revision: &str) -> String {
    (start..start + 256)
        .map(|seed| empty_library_manifest(seed, gift_card, fixture, revision))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic filling the opening hand")
}

fn empty_library_enemy_site_stealth_manifest(seed: u32, fixture: &str, revision: &str) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": revision,
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fade": fade_gift(),
            "north-raider": raider(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": ally(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-fade", "north-fade", "north-raider"],
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

fn seed_with_fade_and_raider(start: u32, fixture: &str, revision: &str) -> String {
    (start..start + 256)
        .map(|seed| empty_library_enemy_site_stealth_manifest(seed, fixture, revision))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-fade") && hand.iter().any(|id| id == "north-raider")
        })
        .expect("bounded seed with fade and raider filling the opening hand")
}

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
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
fn rule_catalog_0941_airborne_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = seed_with_ally_and_gift(
        941,
        &gift(),
        "grant-airborne-then-draw-empty",
        "synthetic-grant-airborne-then-draw-empty-v1",
    );
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
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
    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "airborne-granted",
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    let after = state(&session);
    assert_eq!(
        unit(&after, &ally_id)["temporaryAirborneSources"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0949_lethal_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = seed_with_ally_and_gift(
        949,
        &lethal_gift(),
        "grant-lethal-then-draw-empty",
        "synthetic-grant-lethal-then-draw-empty-v1",
    );
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
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
    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "lethal-granted",
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    let after = state(&session);
    assert_eq!(
        unit(&after, &ally_id)["temporaryLethalSources"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0956_movement_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = seed_with_ally_and_gift(
        956,
        &movement_gift(),
        "grant-movement-then-draw-empty",
        "synthetic-grant-movement-then-draw-empty-v1",
    );
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
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
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    let after = state(&session);
    assert_eq!(
        unit(&after, &ally_id)["temporaryMovementSources"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0957_stealth_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = seed_with_ally_and_gift(
        957,
        &stealth_gift(),
        "grant-stealth-then-draw-empty",
        "synthetic-grant-stealth-then-draw-empty-v1",
    );
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
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
    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    let stealth_idx = granted
        .events
        .iter()
        .position(|event| {
            matches!(
                event.event_type.as_str(),
                "minions-stealthed" | "minion-stealthed" | "stealth-granted"
            )
        })
        .expect("minions-stealthed or stealth-granted event");
    let ended_idx = granted
        .events
        .iter()
        .position(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert!(stealth_idx < ended_idx);
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-stealthed",
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["stealthed"], true);
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0962_power_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = seed_with_ally_and_gift(
        962,
        &power_gift(),
        "grant-power-then-draw-empty",
        "synthetic-grant-power-then-draw-empty-v1",
    );
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
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
    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "power-granted",
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    let after = state(&session);
    assert_eq!(
        unit(&after, &ally_id)["temporaryPowerSources"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0987_stealth_enemy_site_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = seed_with_fade_and_raider(
        987,
        "grant-stealth-enemy-site-then-draw-empty",
        "synthetic-grant-stealth-enemy-site-then-draw-empty-v1",
    );
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
    south_plays_c1(&mut session);
    let (raid, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-raider"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let raid_id = raid["cardInstanceId"]
        .as_str()
        .expect("raid ally identity")
        .to_owned();
    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fade"
            && descriptor["ally"]["instanceId"] == raid_id
    });
    let stealth_idx = granted
        .events
        .iter()
        .position(|event| {
            matches!(
                event.event_type.as_str(),
                "minions-stealthed" | "minion-stealthed" | "stealth-granted"
            )
        })
        .expect("minion-stealthed or stealth-granted event");
    let ended_idx = granted
        .events
        .iter()
        .position(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert!(stealth_idx < ended_idx);
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-stealthed",
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    let after = state(&session);
    assert_eq!(unit(&after, &raid_id)["stealthed"], true);
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0989_cemetery_bottom_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = seed_with_ally_and_gift(
        989,
        &cemetery_bottom_gift(),
        "cemetery-bottom-then-draw-empty",
        "synthetic-cemetery-bottom-then-draw-empty-v1",
    );
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["cemeteryCardInstanceIds"]
                .as_array()
                .is_none_or(Vec::is_empty)
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "magic-resolved", "game-ended"]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "card-returned-to-deck-bottom")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == cast["cardInstanceId"])
    );
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}
