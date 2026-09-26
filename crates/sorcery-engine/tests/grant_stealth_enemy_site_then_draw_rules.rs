//! Direct proofs for grant-Stealth-to-an-allied-minion occupying an enemy
//! site then draw-spell Magic (RULE-CATALOG-0541–0542, RULE-CATALOG-1074,
//! RULE-CATALOG-1683–1688).
//!
//! Ordinary Magic can give Stealth to one allied minion that occupies an
//! enemy-controlled site and then draw one spell. Allies on friendly sites,
//! Avatars, and enemy minions are not offered. When no allied minion occupies
//! an enemy site, the grant is a paid no-op that still draws. While Deathrites
//! wait for ordering, Fade Magic stays withheld until the chain drains.

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

fn fade() -> Value {
    json!({
        "cardType": "magic",
        "grantStealthToAlliedMinionOccupyingEnemySiteThenDrawSpell": true,
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

fn fade_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-stealth-enemy-site-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-stealth-enemy-site-then-draw-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-fade": fade(),
            "north-raider": raider(),
            "north-site": site(),
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
                    "north-fade",
                    "north-fade",
                    "north-raider",
                    "north-raider",
                    "north-raider",
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
    let mut session = Session::new(encoded).expect("valid grant-stealth-enemy-site session");
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

fn fade_ally_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("fade actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-fade"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (541..541 + 256)
        .map(fade_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Fade opening cards")
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
fn rule_catalog_0541_fade_offers_only_an_ally_occupying_an_enemy_site() {
    let encoded = seed_with(&["north-ally", "north-fade", "north-raider"]);
    let mut session = opening_main(&encoded);
    let (home, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let home_id = home["cardInstanceId"]
        .as_str()
        .expect("home ally identity")
        .to_owned();
    let enemy_id = south_plays_c1(&mut session);
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
    let offered = fade_ally_ids(&session);
    assert!(offered.contains(&raid_id));
    assert!(!offered.contains(&home_id));
    assert!(!offered.contains(&enemy_id));
    assert!(!offered.contains(&north_avatar));
    assert_eq!(unit(&before, &raid_id)["stealthed"], false);

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fade"
            && descriptor["ally"]["instanceId"] == raid_id
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
    assert_eq!(granted.events[1].payload["instanceId"], raid_id);
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(unit(&after, &raid_id)["stealthed"], true);
    assert_eq!(unit(&after, &home_id)["stealthed"], false);
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
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0542_fade_still_draws_when_no_ally_occupies_an_enemy_site() {
    let encoded = seed_with(&["north-ally", "north-fade"]);
    let mut session = opening_main(&encoded);
    let (home, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let home_id = home["cardInstanceId"]
        .as_str()
        .expect("home ally identity")
        .to_owned();
    south_plays_c1(&mut session);
    let before = state(&session);
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    assert_eq!(fade_ally_ids(&session), [] as [String; 0]);
    assert_eq!(unit(&before, &home_id)["stealthed"], false);

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fade"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    assert_eq!(unit(&state(&session), &home_id)["stealthed"], false);
    assert!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
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

fn printed_stealth_raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "stealth": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn enemy_site_visibility_manifest(raider: &Value, seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-stealth-enemy-site-visibility" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-stealth-enemy-site-visibility-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fade": fade(),
            "north-raider": raider,
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-zap": zap(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-raider",
                    "north-fade",
                    "north-fade",
                    "north-fade",
                    "north-fade",
                    "north-fade",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-zap",
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

struct EnemySiteVisibilitySetup {
    raid_id: String,
    session: Session,
}

fn north_hand_has_fade(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-fade"))
}

fn south_hand_has_zap(snapshot: &Value) -> bool {
    snapshot["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-zap"))
}

fn try_enemy_site_visibility_setup(encoded: &str) -> Option<EnemySiteVisibilitySetup> {
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
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let raid = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-raider"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let raid_id = raid.0["cardInstanceId"].as_str()?.to_owned();
    Some(EnemySiteVisibilitySetup { raid_id, session })
}

fn enemy_site_visibility_setup(raider: &Value, start: u32) -> EnemySiteVisibilitySetup {
    (start..start + 256)
        .map(|seed| enemy_site_visibility_manifest(raider, seed))
        .find_map(|candidate| try_enemy_site_visibility_setup(&candidate))
        .expect("bounded seed reaching enemy-site stealth visibility setup")
}

fn enemy_site_visibility_setup_with_fade(raider: &Value, start: u32) -> EnemySiteVisibilitySetup {
    (start..start + 256)
        .map(|seed| enemy_site_visibility_manifest(raider, seed))
        .find_map(|candidate| {
            let setup = try_enemy_site_visibility_setup(&candidate)?;
            north_hand_has_fade(&state(&setup.session)).then_some(setup)
        })
        .expect("bounded seed reaching enemy-site stealth visibility setup with Fade in hand")
}

fn enemy_site_visibility_setup_with_fade_and_south_zap(
    raider: &Value,
    start: u32,
) -> EnemySiteVisibilitySetup {
    (start..start + 256)
        .map(|seed| enemy_site_visibility_manifest(raider, seed))
        .find_map(|candidate| {
            let setup = try_enemy_site_visibility_setup(&candidate)?;
            if !north_hand_has_fade(&state(&setup.session)) {
                return None;
            }
            let mut probe = setup.session.clone();
            advance_to_south_main(&mut probe);
            south_hand_has_zap(&state(&probe)).then_some(setup)
        })
        .expect("bounded seed reaching enemy-site stealth visibility setup with Fade and south Zap")
}

fn south_can_zap_raid(session: &Session, raid_id: &str) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "south-zap"
                && action.descriptor["target"]["kind"] == "minion"
                && action.descriptor["target"]["instanceId"] == raid_id
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

fn cast_fade(session: &mut Session, raid_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fade"
            && descriptor["ally"]["instanceId"] == raid_id
    });
    receipt
}

#[test]
fn rule_catalog_1683_printed_stealth_on_enemy_site_hides_raid_from_enemy_targeted_damage_without_fade()
 {
    let EnemySiteVisibilitySetup {
        session, raid_id, ..
    } = enemy_site_visibility_setup(&printed_stealth_raider(), 1683);
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], true);
    let mut session = session;
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_raid(&session, &raid_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1684_fade_on_enemy_site_hides_raid_before_enemy_can_target() {
    let EnemySiteVisibilitySetup {
        mut session,
        raid_id,
        ..
    } = enemy_site_visibility_setup_with_fade(&raider(), 1684);
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], false);
    let receipt = cast_fade(&mut session, &raid_id);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-stealthed",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], true);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_raid(&session, &raid_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1685_fade_on_enemy_site_blocks_damage_that_was_legal_before_grant() {
    let EnemySiteVisibilitySetup {
        mut session,
        raid_id,
        ..
    } = enemy_site_visibility_setup_with_fade_and_south_zap(&raider(), 1685);
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], false);
    advance_to_south_main_with_zap(&mut session);
    assert!(south_can_zap_raid(&session, &raid_id));
    advance_to_south_main(&mut session);
    cast_fade(&mut session, &raid_id);
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], true);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_raid(&session, &raid_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1686_printed_stealth_on_enemy_site_still_hides_raid_after_turn_passes_without_attack()
 {
    let EnemySiteVisibilitySetup {
        mut session,
        raid_id,
        ..
    } = enemy_site_visibility_setup(&printed_stealth_raider(), 1686);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_raid(&session, &raid_id));
    advance_to_south_main(&mut session);
    advance_to_south_main_with_zap(&mut session);
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], true);
    assert!(!south_can_zap_raid(&session, &raid_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1687_printed_and_fade_on_enemy_site_compose_while_stealth_is_active() {
    let EnemySiteVisibilitySetup {
        mut session,
        raid_id,
        ..
    } = enemy_site_visibility_setup_with_fade(&printed_stealth_raider(), 1687);
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], true);
    let receipt = cast_fade(&mut session, &raid_id);
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
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], true);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_raid(&session, &raid_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1688_printed_stealth_on_enemy_site_outlasts_fade_while_plain_raid_only_hides_after_fade()
 {
    let EnemySiteVisibilitySetup {
        mut session,
        raid_id,
        ..
    } = enemy_site_visibility_setup_with_fade_and_south_zap(&printed_stealth_raider(), 1688);
    assert_eq!(unit(&state(&session), &raid_id)["stealthed"], true);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_raid(&session, &raid_id));
    advance_to_south_main(&mut session);
    advance_to_south_main_with_zap(&mut session);
    assert!(!south_can_zap_raid(&session, &raid_id));

    let EnemySiteVisibilitySetup {
        session: plain_session,
        raid_id: plain_raid,
        ..
    } = enemy_site_visibility_setup_with_fade_and_south_zap(&raider(), 1688);
    let mut plain_session = plain_session;
    advance_to_south_main_with_zap(&mut plain_session);
    assert!(south_can_zap_raid(&plain_session, &plain_raid));
    advance_to_south_main(&mut plain_session);
    cast_fade(&mut plain_session, &plain_raid);
    advance_to_south_main_with_zap(&mut plain_session);
    assert!(!south_can_zap_raid(&plain_session, &plain_raid));

    assert!(!south_can_zap_raid(&session, &raid_id));
    assert_exact_replay(&session);
}

fn deathrite_fade_manifest(seed: u32) -> String {
    let fixture = "grant-stealth-enemy-site-then-draw-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-fade": fade(),
            "north-raider": raider(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-raider",
                    "north-fade",
                    "north-rain",
                    "north-rain",
                    "north-fade",
                    "north-raider",
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

fn north_has_fade_rain_and_raider(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-fade", "north-rain", "north-raider"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteFadeSetup {
    deathrite_ids: [String; 2],
    raid_id: String,
    session: Session,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteFadeSetup> {
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
    if !north_has_fade_rain_and_raider(&state(&session)) {
        return None;
    }
    let raid = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-raider"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let raid_id = raid.0["cardInstanceId"].as_str()?.to_owned();
    if !fade_ally_ids(&session).contains(&raid_id) {
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
    Some(PendingDeathriteFadeSetup {
        deathrite_ids,
        raid_id,
        session,
    })
}

fn deathrite_fade_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_fade_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Fade Magic in hand")
}

#[test]
fn rule_catalog_1074_grant_stealth_enemy_site_then_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_fade_seed_with(1074);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete Fade Deathrite withheld setup");
    let raid_id = setup.raid_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert_eq!(unit(&paused, &raid_id)["stealthed"], false);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(fade_ally_ids(session).is_empty());

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
    assert_eq!(unit(&resumed, &raid_id)["stealthed"], false);
    let mut offered = fade_ally_ids(session);
    offered.sort_unstable();
    offered.dedup();
    assert_eq!(offered, vec![raid_id.clone()]);

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
            && descriptor["cardId"] == "north-fade"
            && descriptor["ally"]["instanceId"] == raid_id
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
    assert_eq!(granted.events[1].payload["instanceId"], raid_id);
    let after = state(session);
    assert_eq!(unit(&after, &raid_id)["stealthed"], true);
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(session);
}
