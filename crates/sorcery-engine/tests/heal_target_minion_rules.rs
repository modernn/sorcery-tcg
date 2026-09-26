//! Direct proofs for heal-target-minion Magic (RULE-CATALOG-0298–0299,
//! RULE-CATALOG-0663–0664, RULE-CATALOG-0908, RULE-CATALOG-0992,
//! RULE-CATALOG-0997, RULE-CATALOG-1113, RULE-CATALOG-2283–2288).
//!
//! Official Magic can remove damage from a living minion without targeting
//! Avatars or breaking Ward. Healing a healthy minion is a paid no-op. End
//! Phase clears leftover damage, so the wound and the heal must share a turn.
//! Supplemental 0663–0664 keep that slice on high IDs: a same-turn heal, and a
//! Death's Door Avatar that is still excluded while an at-cap minion no-ops.
//! Supplemental 2283–2288 keep persistence, paid-noop-repeat, enemy-arrival,
//! multi-minion, far-minion, and new-summon proofs on later IDs.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn magic(effect: (&str, Value)) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("Magic facts")
        .insert(effect.0.to_owned(), effect.1);
    value
}

fn visitor(ward: bool) -> Value {
    visitor_with_defense(2, ward)
}

fn visitor_with_defense(defense: u8, ward: bool) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    if ward {
        value["ward"] = json!(true);
    }
    value
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

fn manifest(seed: u32, ward: bool) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "heal-target-minion" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-heal-target-minion-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-damage": magic(("damageTargetUnit", json!(1))),
            "north-heal": magic(("healTargetMinion", json!(1))),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-visitor": visitor(ward),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-damage", "north-damage", "north-damage", "north-heal", "north-heal", "north-heal"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-visitor"; 6],
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

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step");
    let StepResult::Accepted(receipt) = result else {
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
    session.replay_value().expect("session value")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn realm_unit<'a>(value: &'a Value, instance_id: &str) -> Option<&'a Value> {
    value["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn heal_minion_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("heal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-heal"
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

fn after_visitor_on_c4_from(encoded: &str) -> (Session, String) {
    let mut session = Session::new(encoded).expect("valid heal-target-minion session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    let visitor_id = summoned["cardInstanceId"]
        .as_str()
        .expect("visitor identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (session, visitor_id)
}

fn after_visitor_on_c4(seed: u32, ward: bool) -> (Session, String) {
    after_visitor_on_c4_from(&manifest(seed, ward))
}

fn avatar_with_life(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    })
}

fn cap_manifest(seed: u32, heal_amount: u8) -> String {
    let fixture = format!("heal-target-minion-cap-{heal_amount}");
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-damage": magic(("damageTargetUnit", json!(1))),
            "north-heal": magic(("healTargetMinion", json!(heal_amount))),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-visitor": visitor(false),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-damage", "north-damage", "north-damage", "north-heal", "north-heal", "north-heal"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-visitor"; 6],
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

fn high_id_manifest(seed: u32, north_life: u8, damage: u8) -> String {
    let fixture = format!("heal-target-minion-{north_life}-{damage}");
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar_with_life(north_life),
            "north-damage": magic(("damageTargetUnit", json!(damage))),
            "north-heal": magic(("healTargetMinion", json!(1))),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-visitor": visitor(false),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-damage", "north-damage", "north-damage", "north-heal", "north-heal", "north-heal"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-visitor"; 6],
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

fn north_opening_has_damage_and_heal(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-damage", "north-heal"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn seed_with(north_life: u8, damage: u8, start: u32) -> String {
    (start..start + 256)
        .map(|seed| high_id_manifest(seed, north_life, damage))
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_opening_has_damage_and_heal(&state(&preview)))
        })
        .expect("bounded seed with heal and damage Magic in the opening hand")
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

fn assert_checkpoint(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("heal checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized heal");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed heal");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed heal session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0298_heal_target_minion_clears_same_turn_damage_without_targeting_avatars() {
    let (mut session, visitor_id) = after_visitor_on_c4(298, false);
    let before = state(&session);
    let visitor = realm_unit(&before, &visitor_id).expect("wounded-to-be visitor");
    assert_eq!(visitor["damage"], 0);
    assert_eq!(visitor["warded"], false);
    let north_avatar = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert_eq!(
        heal_minion_targets(&session),
        [("minion".to_owned(), visitor_id.clone())]
    );
    assert!(
        !heal_minion_targets(&session)
            .iter()
            .any(|(_, instance_id)| *instance_id == north_avatar || *instance_id == south_avatar)
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-damage"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    let wounded = state(&session);
    let visitor = realm_unit(&wounded, &visitor_id).expect("wounded visitor");
    assert_eq!(visitor["damage"], 1);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-healed", "magic-resolved"]
    );
    let healed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-healed")
        .expect("heal event");
    assert_eq!(healed.payload["amount"], 1);
    assert_eq!(healed.payload["attemptedAmount"], 1);
    assert_eq!(healed.payload["damage"], 0);
    assert_eq!(healed.payload["instanceId"], visitor_id);
    assert_eq!(healed.payload["seat"], "south");
    assert_eq!(
        healed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken"
                || event.event_type == "minion-died"
                || event.event_type == "avatar-healed")
    );

    let after = state(&session);
    let visitor = realm_unit(&after, &visitor_id).expect("healed visitor");
    assert_eq!(visitor["damage"], 0);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["decisionSeat"], "north");
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    assert_checkpoint(&session);
}

#[test]
fn rule_catalog_0299_heal_target_minion_is_a_paid_noop_on_an_undamaged_minion() {
    let (mut session, visitor_id) = after_visitor_on_c4(299, true);
    let before = state(&session);
    let visitor = realm_unit(&before, &visitor_id).expect("healthy visitor");
    assert_eq!(visitor["damage"], 0);
    assert_eq!(visitor["warded"], true);
    assert_eq!(
        heal_minion_targets(&session),
        [("minion".to_owned(), visitor_id.clone())]
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-healed"
                || event.event_type == "ward-broken"
                || event.event_type == "minion-died")
    );

    let after = state(&session);
    let visitor = realm_unit(&after, &visitor_id).expect("still-healthy visitor");
    assert_eq!(visitor["damage"], 0);
    assert_eq!(visitor["warded"], true);
    assert_exact_replay(&session);
    assert_checkpoint(&session);
}

#[test]
fn rule_catalog_0663_heal_target_minion_clears_same_turn_damage_without_targeting_avatars() {
    let encoded = seed_with(20, 1, 663);
    let (mut session, visitor_id) = after_visitor_on_c4_from(&encoded);
    let before = state(&session);
    let visitor = realm_unit(&before, &visitor_id).expect("wounded-to-be visitor");
    assert_eq!(visitor["damage"], 0);
    let north_avatar = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert_eq!(
        heal_minion_targets(&session),
        [("minion".to_owned(), visitor_id.clone())]
    );
    assert!(
        heal_minion_targets(&session)
            .iter()
            .all(|(kind, instance_id)| {
                kind == "minion" && *instance_id != north_avatar && *instance_id != south_avatar
            })
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-damage"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        realm_unit(&state(&session), &visitor_id).expect("wounded visitor")["damage"],
        1
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-healed", "magic-resolved"]
    );
    let healed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-healed")
        .expect("heal event");
    assert_eq!(healed.payload["amount"], 1);
    assert_eq!(healed.payload["attemptedAmount"], 1);
    assert_eq!(healed.payload["damage"], 0);
    assert_eq!(healed.payload["instanceId"], visitor_id);
    assert_eq!(healed.payload["seat"], "south");
    assert_eq!(
        healed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken"
                || event.event_type == "minion-died"
                || event.event_type == "avatar-healed")
    );

    let after = state(&session);
    assert_eq!(
        realm_unit(&after, &visitor_id).expect("healed visitor")["damage"],
        0
    );
    assert_eq!(after["phase"], "main");
    assert_eq!(after["decisionSeat"], "north");
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0664_heal_target_minion_excludes_deaths_door_avatar_and_noops_at_cap() {
    let encoded = seed_with(2, 2, 664);
    let (mut session, visitor_id) = after_visitor_on_c4_from(&encoded);
    let before = state(&session);
    let north_avatar = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert_eq!(before["players"]["north"]["avatar"]["life"], 2);
    assert_eq!(
        realm_unit(&before, &visitor_id).expect("at-cap visitor")["damage"],
        0
    );

    let (_, damage) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-damage"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "north"
            && descriptor["target"]["instanceId"] == north_avatar
    });
    assert!(event_types(&damage).contains(&"avatar-reached-deaths-door"));
    let wounded = state(&session);
    assert_eq!(wounded["players"]["north"]["avatar"]["life"], 0);
    let death_door_turn = wounded["players"]["north"]["avatar"]["deathDoorTurn"].clone();
    assert_eq!(
        realm_unit(&wounded, &visitor_id).expect("untouched visitor")["damage"],
        0
    );
    assert_eq!(
        heal_minion_targets(&session),
        [("minion".to_owned(), visitor_id.clone())]
    );
    assert!(
        heal_minion_targets(&session)
            .iter()
            .all(|(kind, instance_id)| {
                kind == "minion" && *instance_id != north_avatar && *instance_id != south_avatar
            })
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-healed"
                || event.event_type == "avatar-healed"
                || event.event_type == "ward-broken"
                || event.event_type == "minion-died"
                || event.event_type == "death-blow"
                || event.event_type == "game-ended")
    );

    let after = state(&session);
    assert_eq!(
        realm_unit(&after, &visitor_id).expect("still at-cap visitor")["damage"],
        0
    );
    assert_eq!(after["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(
        after["players"]["north"]["avatar"]["deathDoorTurn"],
        death_door_turn
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0908_heal_target_minion_caps_at_current_damage_not_printed_amount() {
    let encoded = (908..908 + 256)
        .map(|seed| cap_manifest(seed, 3))
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_opening_has_damage_and_heal(&state(&preview)))
        })
        .expect("bounded seed with heal and damage Magic in the opening hand");
    let (mut session, visitor_id) = after_visitor_on_c4_from(&encoded);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-damage"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        realm_unit(&state(&session), &visitor_id).expect("wounded visitor")["damage"],
        1
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-healed", "magic-resolved"]
    );
    let healed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-healed")
        .expect("heal event");
    assert_eq!(healed.payload["amount"], 1);
    assert_eq!(healed.payload["attemptedAmount"], 3);
    assert_eq!(healed.payload["damage"], 0);
    assert_eq!(healed.payload["instanceId"], visitor_id);
    assert_eq!(healed.payload["seat"], "south");
    assert_eq!(
        healed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );

    let after = state(&session);
    assert_eq!(
        realm_unit(&after, &visitor_id).expect("healed visitor")["damage"],
        0
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
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

fn deathrite_heal_manifest(seed: u32) -> String {
    let fixture = "heal-target-minion-deathrite-withheld";
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-damage": magic(("damageTargetUnit", json!(1))),
            "north-heal": magic(("healTargetMinion", json!(1))),
            "north-rain": magic(("damageEachAbovegroundMinion", json!(1))),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor_with_defense(3, false),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-damage",
                    "north-heal",
                    "north-rain",
                    "north-rain",
                    "north-damage",
                    "north-heal",
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn north_has_damage_heal_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-damage", "north-heal", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteHealSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_wounded_visitor(encoded: &str) -> Option<PendingDeathriteHealSetup> {
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
    if !north_has_damage_heal_and_rain(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-damage"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    })?;
    if heal_minion_targets(&session).is_empty() {
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
    Some(PendingDeathriteHealSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_heal_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_heal_manifest)
        .find(|candidate| try_pending_deathrite_with_wounded_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with heal Magic in hand")
}

#[test]
fn rule_catalog_0992_heal_target_minion_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_heal_seed_with(992);
    let mut setup = try_pending_deathrite_with_wounded_visitor(&encoded)
        .expect("complete heal Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        deathrite_ids
            .iter()
            .all(|instance_id| realm_unit(&paused, instance_id).is_none())
    );
    assert_eq!(
        realm_unit(&paused, &visitor_id).expect("surviving visitor")["damage"],
        2
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(heal_minion_targets(session).is_empty());

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
    assert_eq!(
        realm_unit(&resumed, &visitor_id).expect("still-wounded visitor")["damage"],
        2
    );
    assert_eq!(
        heal_minion_targets(session),
        [("minion".to_owned(), visitor_id.clone())]
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-healed", "magic-resolved"]
    );
    let healed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-healed")
        .expect("heal event");
    assert_eq!(healed.payload["amount"], 1);
    assert_eq!(
        realm_unit(&state(session), &visitor_id).expect("partially healed visitor")["damage"],
        1
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_0997_end_turn_and_summon_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_heal_seed_with(997);
    let mut setup = try_pending_deathrite_with_wounded_visitor(&encoded)
        .expect("complete end-turn/summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "end-turn"
                    && action.descriptor["kind"] != "summon-minion"
            })
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "end-turn")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1113_play_site_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_heal_seed_with(1113);
    let mut setup = try_pending_deathrite_with_wounded_visitor(&encoded)
        .expect("complete play-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        paused["players"]["north"]["hand"]["atlas"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-site"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "play-site")
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1171_end_turn_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_heal_seed_with(1171);
    let mut setup = try_pending_deathrite_with_wounded_visitor(&encoded)
        .expect("complete end-turn Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "end-turn")
    );

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "end-turn")
    );
    assert_exact_replay(session);
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
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

fn heal_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "heal-target-minion-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-heal-target-minion-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-damage": magic(("damageTargetUnit", json!(1))),
            "north-heal": magic(("healTargetMinion", json!(1))),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": supplemental_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-heal",
                    "north-damage",
                    "north-heal",
                    "north-damage",
                    "north-heal",
                    "north-damage",
                    "north-heal",
                    "north-damage",
                    "north-heal",
                    "north-damage",
                    "north-heal",
                    "north-damage",
                    "north-heal",
                    "north-damage",
                    "north-heal",
                    "north-damage",
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
        .chain(663..663 + 2048)
        .map(heal_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-heal")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with heal-target Magic and required South minions")
}

fn heal_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-heal")
                .count()
        })
        .unwrap_or_default()
}

fn damage_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-damage")
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid heal-target session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
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
        .expect("heal-target supplemental identity")
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

fn heal_target_ids(session: &Session) -> Vec<String> {
    heal_minion_targets(session)
        .into_iter()
        .map(|(_, instance_id)| instance_id)
        .collect()
}

fn cast_damage_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-damage"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn cast_heal_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-heal"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn wound_then_heal(session: &mut Session, target_id: &str) -> Option<Receipt> {
    if damage_spells_in_hand(&state(session)) < 1 || heal_spells_in_hand(&state(session)) < 1 {
        return None;
    }
    let damage = cast_damage_on(session, target_id);
    if !event_types(&damage).contains(&"damage-dealt") {
        return None;
    }
    if unit(&state(session), target_id)["damage"] != 1 {
        return None;
    }
    Some(cast_heal_on(session, target_id))
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
    if damage_spells_in_hand(&state(&session)) < 1 || heal_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    Some((session, c2_ids[0].clone(), far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(663..663 + 2048)
        .find_map(|seed| {
            let encoded = heal_supplemental_manifest(seed);
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
        .expect("bounded seed reaching heal-target far-minion setup")
}

fn try_second_heal_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = wound_then_heal(&mut session, &first_id)?;
    if !event_types(&first).contains(&"minion-healed") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
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
    if damage_spells_in_hand(&state(&session)) < 1 || heal_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    heal_target_ids(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_heal_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(663..663 + 8192)
        .find_map(|seed| {
            let encoded = heal_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_heal_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second heal-target enemy-arrival setup")
}

fn try_second_heal_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = wound_then_heal(&mut session, &first_id)?;
    if !event_types(&first).contains(&"minion-healed") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
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
    if damage_spells_in_hand(&state(&session)) < 1 || heal_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    heal_target_ids(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_heal_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(663..663 + 8192)
        .find_map(|seed| {
            let encoded = heal_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_heal_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second heal-target new-summon setup")
}

#[test]
fn rule_catalog_2283_healed_minion_stays_at_the_location_after_turns_pass() {
    let encoded = (2283..2283 + 8192)
        .chain(663..663 + 8192)
        .find_map(|seed| {
            let candidate = heal_supplemental_manifest(seed);
            if opening_hand_spell_ids(&candidate, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 1
            {
                return None;
            }
            let mut session = opening_main(&candidate);
            let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
            north_draws_spellbook(&mut session);
            wound_then_heal(&mut session, &minion_id)
                .filter(|receipt| event_types(receipt).contains(&"minion-healed"))
                .map(|_| candidate)
        })
        .expect("bounded seed with same-turn heal-target persistence setup");
    let mut session = opening_main(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let receipt = wound_then_heal(&mut session, &minion_id).expect("same-turn heal");
    assert!(event_types(&receipt).contains(&"minion-healed"));
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 0);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C2");
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 0);
    assert!(realm_unit(&state(&session), &minion_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2284_second_heal_on_an_already_healed_minion_is_a_paid_noop() {
    let encoded = (2284..2284 + 8192)
        .chain(663..663 + 8192)
        .find_map(|seed| {
            let candidate = heal_supplemental_manifest(seed);
            let mut session = opening_main(&candidate);
            let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
            north_draws_spellbook(&mut session);
            let first = wound_then_heal(&mut session, &minion_id)?;
            if !event_types(&first).contains(&"minion-healed") {
                return None;
            }
            if unit(&state(&session), &minion_id)["damage"] != 0 {
                return None;
            }
            (heal_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two heal-target casts after clearing damage");
    let mut session = opening_main(&encoded);
    let minion_id = setup_c2_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = wound_then_heal(&mut session, &minion_id).expect("same-turn heal");
    assert!(event_types(&first).contains(&"minion-healed"));
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 0);
    assert!(heal_spells_in_hand(&state(&session)) >= 1);
    let second = cast_heal_on(&mut session, &minion_id);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"minion-healed"));
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2285_second_heal_clears_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_heal_enemy_arrival(2285);
    let (mut session, minion_id) = try_second_heal_enemy_arrival_prefix(&encoded)
        .expect("second heal-target enemy-arrival prefix");
    let receipt = wound_then_heal(&mut session, &minion_id).expect("same-turn heal of new arrival");
    assert!(event_types(&receipt).contains(&"minion-healed"));
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2286_heal_target_offers_every_same_region_minion_in_the_caster_region() {
    let encoded = supplemental_seed_with_start(2286, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c2_with_south_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = heal_target_ids(&session);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_eq!(offered.len(), 2);
    let snapshot = state(&session);
    let north_avatar = snapshot["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = snapshot["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    assert!(
        heal_minion_targets(&session)
            .iter()
            .all(|(kind, instance_id)| {
                kind == "minion" && *instance_id != north_avatar && *instance_id != south_avatar
            })
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2287_heal_target_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(2287);
    let (mut session, healed_id, far_id) =
        try_far_minion_prefix(&encoded).expect("heal-target far-minion prefix");
    let receipt = wound_then_heal(&mut session, &healed_id).expect("same-turn heal");
    assert!(event_types(&receipt).contains(&"minion-healed"));
    assert_eq!(unit(&state(&session), &healed_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2288_second_heal_clears_a_newly_summoned_minion() {
    let encoded = seed_for_second_heal_new_summon(2288);
    let (mut session, minion_id) =
        try_second_heal_new_summon_prefix(&encoded).expect("second heal-target new-summon prefix");
    let receipt = wound_then_heal(&mut session, &minion_id).expect("same-turn heal of new summon");
    assert!(event_types(&receipt).contains(&"minion-healed"));
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C2");
    assert_exact_replay(&session);
}
