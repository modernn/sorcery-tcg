//! Direct proofs for grant-Stealth-to-allied-minions then draw-spell Magic
//! (RULE-CATALOG-0539–0540, RULE-CATALOG-1067, RULE-CATALOG-1673–1678).
//!
//! Ordinary Magic can give every allied minion Stealth and then draw one
//! spell. The Avatar and enemy minions are not stealthed. Already-stealthed
//! allies stay stealthed without a second event. An empty allied-minion set
//! is a paid no-op grant that still draws. While Deathrites wait for
//! ordering, grant-Stealth-then-draw Magic stays withheld until the chain
//! drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn grounded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn tough() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn vanish() -> Value {
    json!({
        "cardType": "magic",
        "grantStealthToAlliedMinionsThenDrawSpell": true,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn vanish_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-stealth-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-stealth-then-draw-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-site": site(),
            "north-vanish": vanish(),
            "south-avatar": avatar(),
            "south-minion": grounded(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-vanish",
                    "north-vanish",
                    "north-vanish",
                    "north-vanish",
                ],
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
    try_accept_where(session, predicate).expect("expected engine-issued action")
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid grant-stealth-then-draw session");
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

fn avatar_instance_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar instance identity")
        .to_owned()
}

fn vanish_casts(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("vanish actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-vanish"
        })
        .map(|action| action.descriptor.clone())
        .collect()
}

fn assert_targetless_vanish(session: &Session) {
    let casts = vanish_casts(session);
    assert!(!casts.is_empty());
    assert!(
        casts
            .iter()
            .all(|descriptor| { descriptor["ally"].is_null() && descriptor["target"].is_null() }),
        "grant-Stealth-then-draw is targetless"
    );
}

fn seed_with_ally_and_vanish() -> String {
    (539..539 + 256)
        .map(vanish_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-vanish")
        })
        .expect("bounded seed with ally and Stealth-then-draw Magic in the opening hand")
}

fn seed_with_vanish() -> String {
    (540..540 + 256)
        .map(vanish_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|id| id == "north-vanish")
        })
        .expect("bounded seed with Stealth-then-draw Magic in the opening hand")
}

fn opening_with_ally(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
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
    (session, ally_id)
}

fn south_plays_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
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
fn rule_catalog_0539_stealth_then_draw_stealths_allied_minions_not_enemies() {
    let encoded = seed_with_ally_and_vanish();
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let enemy_id = south_plays_c1(&mut session);
    let before = state(&session);
    let north_avatar = avatar_instance_id(&before, "north");
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    assert_targetless_vanish(&session);
    assert_eq!(unit(&before, &ally_id)["stealthed"], false);
    assert_eq!(unit(&before, &enemy_id)["stealthed"], false);

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-vanish"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-stealthed",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    assert_eq!(granted.events[1].payload["seat"], "north");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "minion-stealthed"
                && (event.payload["instanceId"] == enemy_id
                    || event.payload["instanceId"] == north_avatar))
    );
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["stealthed"], true);
    assert_eq!(unit(&after, &enemy_id)["stealthed"], false);
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert!(
        !serde_json::to_string(&south_view)
            .expect("view JSON")
            .contains(&library_top)
    );
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0540_stealth_then_draw_still_draws_without_allied_minions() {
    let encoded = seed_with_vanish();
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    assert_targetless_vanish(&session);
    assert!(
        before["realm"]["units"]
            .as_array()
            .expect("realm units")
            .is_empty()
    );

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-vanish"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "minion-stealthed")
    );
    let after = state(&session);
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(&session);
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn printed_stealth() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "stealth": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn stealth_combat_manifest(ally: &Value, seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-stealth-then-draw-combat" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-stealth-then-draw-combat-v1",
        },
        "cards": {
            "north-ally": ally,
            "north-avatar": avatar(),
            "north-site": site(),
            "north-vanish": vanish(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-tough": tough(),
            "south-zap": zap(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-vanish",
                    "north-vanish",
                    "north-vanish",
                    "north-vanish",
                    "north-vanish",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-tough",
                    "south-zap",
                    "south-zap",
                    "south-zap",
                    "south-zap",
                    "south-zap",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct StealthVisibilitySetup {
    ally_id: String,
    session: Session,
}

fn north_hand_has_vanish(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-vanish"))
}

fn south_hand_has_zap(snapshot: &Value) -> bool {
    snapshot["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-zap"))
}

fn try_stealth_combat_setup(encoded: &str) -> Option<StealthVisibilitySetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    let enemy = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-tough"
            && descriptor["cell"] == "C4"
    })?;
    let _enemy_id = enemy.0["cardInstanceId"].as_str()?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(StealthVisibilitySetup { ally_id, session })
}

fn stealth_combat_setup(ally: &Value, start: u32) -> StealthVisibilitySetup {
    (start..start + 256)
        .map(|seed| stealth_combat_manifest(ally, seed))
        .find_map(|candidate| try_stealth_combat_setup(&candidate))
        .expect("bounded seed reaching stealth combat setup")
}

fn stealth_combat_setup_with_vanish(ally: &Value, start: u32) -> StealthVisibilitySetup {
    (start..start + 256)
        .map(|seed| stealth_combat_manifest(ally, seed))
        .find_map(|candidate| {
            let setup = try_stealth_combat_setup(&candidate)?;
            north_hand_has_vanish(&state(&setup.session)).then_some(setup)
        })
        .expect("bounded seed reaching stealth combat setup with Vanish in hand")
}

fn stealth_combat_setup_with_vanish_and_south_zap(
    ally: &Value,
    start: u32,
) -> StealthVisibilitySetup {
    (start..start + 256)
        .map(|seed| stealth_combat_manifest(ally, seed))
        .find_map(|candidate| {
            let setup = try_stealth_combat_setup(&candidate)?;
            if !north_hand_has_vanish(&state(&setup.session)) {
                return None;
            }
            let mut probe = setup.session.clone();
            advance_to_south_main(&mut probe);
            south_hand_has_zap(&state(&probe)).then_some(setup)
        })
        .expect("bounded seed reaching stealth combat setup with Vanish and south Zap")
}

fn south_can_zap_ally(session: &Session, ally_id: &str) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "south-zap"
                && action.descriptor["target"]["kind"] == "minion"
                && action.descriptor["target"]["instanceId"] == ally_id
        })
    })
}

fn advance_to_south_main(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn advance_to_south_main_with_zap(session: &mut Session) {
    advance_to_south_main(session);
    assert!(south_hand_has_zap(&state(session)));
}

fn cast_vanish(session: &mut Session) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-vanish"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    receipt
}

#[test]
fn rule_catalog_1673_printed_stealth_hides_ally_from_enemy_targeted_damage_without_grant() {
    let StealthVisibilitySetup {
        session, ally_id, ..
    } = stealth_combat_setup(&printed_stealth(), 1673);
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], true);
    let mut session = session;
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_ally(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1674_granted_stealth_then_draw_hides_ally_before_enemy_can_target() {
    let StealthVisibilitySetup {
        mut session,
        ally_id,
        ..
    } = stealth_combat_setup_with_vanish(&grounded(), 1674);
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], false);
    let receipt = cast_vanish(&mut session);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-stealthed",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], true);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_ally(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1675_granted_stealth_then_draw_blocks_damage_that_was_legal_before_grant() {
    let StealthVisibilitySetup {
        mut session,
        ally_id,
        ..
    } = stealth_combat_setup_with_vanish_and_south_zap(&grounded(), 1675);
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], false);
    advance_to_south_main_with_zap(&mut session);
    assert!(south_can_zap_ally(&session, &ally_id));
    advance_to_south_main(&mut session);
    cast_vanish(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], true);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_ally(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1676_printed_stealth_still_hides_ally_after_turn_passes_without_attack() {
    let StealthVisibilitySetup {
        mut session,
        ally_id,
        ..
    } = stealth_combat_setup(&printed_stealth(), 1676);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_ally(&session, &ally_id));
    advance_to_south_main(&mut session);
    advance_to_south_main_with_zap(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], true);
    assert!(!south_can_zap_ally(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1677_printed_and_granted_stealth_then_draw_compose_while_stealth_is_active() {
    let StealthVisibilitySetup {
        mut session,
        ally_id,
        ..
    } = stealth_combat_setup_with_vanish(&printed_stealth(), 1677);
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], true);
    let receipt = cast_vanish(&mut session);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-stealthed")
    );
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], true);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_ally(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1678_printed_stealth_outlasts_grant_while_plain_ally_only_hides_after_vanish() {
    let StealthVisibilitySetup {
        mut session,
        ally_id,
        ..
    } = stealth_combat_setup_with_vanish_and_south_zap(&printed_stealth(), 1678);
    assert_eq!(unit(&state(&session), &ally_id)["stealthed"], true);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_ally(&session, &ally_id));
    advance_to_south_main(&mut session);
    let receipt = cast_vanish(&mut session);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_ally(&session, &ally_id));

    let plain_setup = stealth_combat_setup_with_vanish_and_south_zap(&grounded(), 1678);
    let mut plain_session = plain_setup.session;
    let plain_ally = plain_setup.ally_id;
    advance_to_south_main_with_zap(&mut plain_session);
    assert!(south_can_zap_ally(&plain_session, &plain_ally));
    advance_to_south_main(&mut plain_session);
    cast_vanish(&mut plain_session);
    advance_to_south_main_with_zap(&mut plain_session);
    assert!(!south_can_zap_ally(&plain_session, &plain_ally));

    assert!(!south_can_zap_ally(&session, &ally_id));
    assert_exact_replay(&session);
}

fn deathrite_vanish_manifest(seed: u32) -> String {
    let fixture = "grant-stealth-then-draw-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "north-vanish": vanish(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-vanish",
                    "north-rain",
                    "north-rain",
                    "north-vanish",
                    "north-vanish",
                ],
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

fn north_has_vanish_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-vanish", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteVanishSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_allied_minion(encoded: &str) -> Option<PendingDeathriteVanishSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_vanish_and_rain(&state(&session)) {
        return None;
    }
    if vanish_casts(&session).is_empty() {
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
    Some(PendingDeathriteVanishSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_vanish_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_vanish_manifest)
        .find(|candidate| try_pending_deathrite_with_allied_minion(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with grant-Stealth-then-draw Magic in hand",
        )
}

#[test]
fn rule_catalog_1067_grant_stealth_then_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_vanish_seed_with(1067);
    let mut setup = try_pending_deathrite_with_allied_minion(&encoded)
        .expect("complete grant-Stealth-then-draw Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
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
    assert_eq!(unit(&paused, &ally_id)["stealthed"], false);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(vanish_casts(session).is_empty());

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
    assert_eq!(unit(&resumed, &ally_id)["stealthed"], false);
    assert_targetless_vanish(session);

    let library_top = resumed["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();

    let (_, granted) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-vanish"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-stealthed",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    let after = state(session);
    assert_eq!(unit(&after, &ally_id)["stealthed"], true);
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(session);
}
