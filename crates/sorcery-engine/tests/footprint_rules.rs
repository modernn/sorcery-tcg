use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
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

fn site(blocks_ground_entry: bool) -> Value {
    let mut value = json!({ "cardType": "site", "elements": ["earth"] });
    if blocks_ground_entry {
        value["blocksGroundMinionEntryWhileMinionAtop"] = json!(true);
    }
    value
}

fn minion(extra: &Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 10,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("minion facts")
        .extend(extra.as_object().expect("extra minion facts").clone());
    value
}

fn manifest(seed: u32, blocks_ground_entry: bool) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-giant": minion(&json!({
            "attack": 4,
            "charge": true,
            "occupiesSquareArea": 2,
            "stealth": true,
        })),
        "north-site": site(false),
        "south-avatar": avatar(),
        "south-ranger": minion(&json!({
            "charge": true,
            "nearbyEnemiesPermanentlyLoseStealth": true,
            "ranged": true,
        })),
        "south-site": site(blocks_ground_entry),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "footprint-rules" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-footprint-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-giant"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-ranger"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical manifest")
}

/// A realm whose only spells are one oversized minion and one holding Aura.
///
/// The separate fixture keeps the shuffled openings of the interaction proofs untouched.
fn aura_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "footprint-aura-rules" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-footprint-aura-rules-v1",
        },
        "cards": {
            "north-aura": {
                "cardType": "aura",
                "immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns": true,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-avatar": avatar(),
            "north-giant": minion(&json!({ "charge": true, "occupiesSquareArea": 2 })),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-site": site(false),
            "south-spell": minion(&json!({})),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-aura",
                    "north-aura",
                    "north-aura",
                    "north-aura",
                    "north-giant",
                    "north-giant",
                    "north-giant",
                    "north-giant",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-spell"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical manifest")
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
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

fn summon_at(session: &mut Session, card_id: &str, cell: &str) -> (String, Receipt) {
    let (descriptor, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    (
        descriptor["cardInstanceId"]
            .as_str()
            .expect("summoned instance")
            .to_owned(),
        receipt,
    )
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay value")["state"].clone()
}

fn unit<'a>(state: &'a Value, instance_id: &str) -> &'a Value {
    state["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("realm unit")
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
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

fn setup_to_north_fourth_turn(blocks_ground_entry: bool) -> (Session, String, String, String) {
    let mut session =
        Session::new(&manifest(401, blocks_ground_entry)).expect("footprint manifest");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "C4");
    end_and_draw(&mut session);
    play_site(&mut session, "C1");
    let (shooter, _) = summon_at(&mut session, "south-ranger", "C1");

    end_and_draw(&mut session);
    play_site(&mut session, "B4");
    end_and_draw(&mut session);
    play_site(&mut session, "C2");
    let (combat_target, _) = summon_at(&mut session, "south-ranger", "C2");

    end_and_draw(&mut session);
    play_site(&mut session, "C3");
    end_and_draw(&mut session);
    play_site(&mut session, "B2");
    let (interceptor, _) = summon_at(&mut session, "south-ranger", "B2");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == combat_target
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    end_and_draw_zone(&mut session, "atlas");
    play_site(&mut session, "B3");
    (session, shooter, combat_target, interceptor)
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct footprint proof keeps movement, combat, response, checkpoint, and replay together"
)]
fn fixed_two_by_two_footprint_should_drive_the_supported_interaction_core() {
    let (mut session, shooter, combat_target, interceptor) = setup_to_north_fourth_turn(false);
    let giant_actions = session
        .legal_actions()
        .expect("giant summon actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-giant"
        })
        .collect::<Vec<_>>();
    assert!(!giant_actions.is_empty());
    assert_eq!(
        giant_actions
            .iter()
            .map(|action| canonical_json(&action.descriptor["cells"]).expect("cells JSON"))
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            r#"["B2","B3","C2","C3"]"#.to_owned(),
            r#"["B3","B4","C3","C4"]"#.to_owned(),
        ])
    );
    assert!(giant_actions.windows(2).all(|pair| {
        canonical_json(&pair[0].descriptor).expect("left descriptor")
            <= canonical_json(&pair[1].descriptor).expect("right descriptor")
    }));
    assert_eq!(
        giant_actions
            .iter()
            .map(|action| action.action_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        giant_actions.len()
    );

    let (first_giant, first_summon) = summon_at(&mut session, "north-giant", "B3");
    assert_eq!(
        first_summon.events.first().expect("summon event").payload["cells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    let current = state(&session);
    assert_eq!(
        unit(&current, &first_giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_eq!(unit(&current, &first_giant)["stealthed"], false);
    assert!(event_types(&first_summon).contains(&"stealth-lost"));

    let (second_giant, _) = summon_at(&mut session, "north-giant", "B2");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == first_giant
            && descriptor["path"]
                .as_array()
                .is_some_and(|path| path.len() == 1)
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["instanceId"] == combat_target
    });
    assert_eq!(state(&session)["pendingCombat"]["cell"], "C3");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });

    end_and_draw_zone(&mut session, "atlas");
    let (_, projectile) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["shooterInstanceId"] == shooter
            && descriptor["hit"]["instanceId"] == second_giant
            && descriptor["path"]
                == json!([
                    { "cell": "C1", "region": "surface" },
                    { "cell": "C2", "region": "surface" },
                ])
    });
    assert_eq!(event_types(&projectile)[0], "projectile-shot");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == combat_target
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "site"
    });
    let defend = session
        .legal_actions()
        .expect("Defend actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "defend"
                && action.descriptor["unitInstanceId"] == second_giant
                && action.descriptor["to"]["cell"] == "C3"
                && action.descriptor["path"].as_array().is_some_and(|path| {
                    path.last().is_some_and(|location| location["cell"] == "B2")
                })
        })
        .expect("oversized in-place Defend");
    let StepResult::Accepted(defend_receipt) = session
        .step(ActionRequest {
            action_id: defend.action_id.to_string(),
            seat: defend.seat,
            state_version: defend.state_version,
        })
        .expect("Defend step")
    else {
        panic!("issued Defend must be accepted");
    };
    assert_eq!(
        event_types(&defend_receipt),
        ["defender-joined", "original-target-removed"]
    );
    assert_eq!(defend_receipt.events[0].payload["to"]["cell"], "B2");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });

    end_and_draw(&mut session);
    let (movement, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == first_giant
            && descriptor["path"]
                == json!([
                    { "cell": "B3", "region": "surface" },
                    { "cell": "B2", "region": "surface" },
                ])
    });
    assert_eq!(movement["to"]["cell"], "B2");
    let current = state(&session);
    assert_eq!(
        unit(&current, &first_giant)["occupiedCells"],
        json!(["B2", "B3", "C2", "C3"])
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(state(&session)["pendingCombat"]["cell"], "B2");
    let interceptors = session
        .legal_actions()
        .expect("Intercept actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "intercept")
        .map(|action| {
            action.descriptor["unitInstanceId"]
                .as_str()
                .expect("ID")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert!(interceptors.contains(&interceptor));
    assert!(!interceptors.contains(&shooter));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "intercept" && descriptor["unitInstanceId"] == interceptor
    });

    let checkpoint = create_game_checkpoint(&session).expect("footprint checkpoint");
    let resumed = resume_game_checkpoint(&checkpoint).expect("resumed footprint checkpoint");
    assert_eq!(
        resumed.replay_value().expect("resumed value"),
        session.replay_value().expect("session value")
    );
    assert_exact_replay(&session);
}

/// Every destination one unit is offered, as `cell/region` labels.
fn move_destinations(session: &Session, instance_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .map(|action| {
            format!(
                "{}/{}",
                action.descriptor["to"]["cell"]
                    .as_str()
                    .expect("destination cell"),
                action.descriptor["to"]["region"]
                    .as_str()
                    .expect("destination region")
            )
        })
        .collect()
}

#[test]
fn rule_catalog_0066_an_aura_should_hold_an_oversized_footprint_it_barely_overlaps() {
    let mut session = Session::new(&aura_manifest(66)).expect("footprint Aura manifest");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "C4");
    end_and_draw_zone(&mut session, "atlas");
    play_site(&mut session, "C1");
    end_and_draw_zone(&mut session, "atlas");

    play_site(&mut session, "C3");
    end_and_draw_zone(&mut session, "atlas");
    play_site(&mut session, "C2");
    end_and_draw_zone(&mut session, "atlas");

    play_site(&mut session, "B4");
    end_and_draw_zone(&mut session, "atlas");
    play_site(&mut session, "B2");
    end_and_draw(&mut session);

    play_site(&mut session, "B3");
    end_and_draw_zone(&mut session, "atlas");
    end_and_draw(&mut session);

    let (giant, summon) = summon_at(&mut session, "north-giant", "B3");
    assert_eq!(
        summon.events.first().expect("summon event").payload["cells"],
        json!(["B3", "B4", "C3", "C4"]),
        "an oversized minion is summoned onto one whole canonical two-by-two area"
    );
    assert_eq!(
        unit(&state(&session), &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_eq!(
        move_destinations(&session, &giant),
        ["B2/surface", "B3/surface"],
        "an unheld oversized minion translates its footprint onto whole existing terrain"
    );

    let (cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura" && descriptor["cells"] == json!(["C2", "C3", "D2", "D3"])
    });
    let aura = cast["cardInstanceId"].as_str().expect("aura identity");
    assert_eq!(
        state(&session)["realm"]["immobileAreas"],
        json!([{
            "cells": ["C2", "C3", "D2", "D3"],
            "minionsAtSitesOnly": true,
            "sourceInstanceId": aura,
            "suppressesAirborne": true,
        }])
    );
    assert_eq!(
        move_destinations(&session, &giant),
        ["B3/surface"],
        "an Aura covering one footprint cell holds the whole oversized minion"
    );
    assert_exact_replay(&session);
}

#[test]
fn oversized_ground_movement_should_check_every_new_terrain_cell() {
    let (mut session, _, _, _) = setup_to_north_fourth_turn(true);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    end_and_draw_zone(&mut session, "atlas");
    end_and_draw(&mut session);
    assert!(
        !session
            .legal_actions()
            .expect("movement actions")
            .into_iter()
            .any(|action| {
                action.descriptor["kind"] == "move-and-attack"
                    && action.descriptor["unitInstanceId"] == giant
                    && action.descriptor["path"]
                        == json!([
                            { "cell": "B3", "region": "surface" },
                            { "cell": "B2", "region": "surface" },
                        ])
            })
    );
}

fn freeze() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn composition_manifest(
    seed: u32,
    giant_extra: &Value,
    south_extra: &Value,
    north_spells: &[&str],
    south_spells: &[&str],
) -> String {
    let mut giant = minion(&json!({ "occupiesSquareArea": 2 }));
    giant
        .as_object_mut()
        .expect("giant facts")
        .extend(giant_extra.as_object().expect("extra giant facts").clone());
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-giant": giant,
        "north-site": site(false),
        "south-avatar": avatar(),
        "south-site": site(false),
    });
    if north_spells.contains(&"north-freeze") {
        cards["north-freeze"] = freeze();
    }
    if south_spells.contains(&"south-zap") {
        cards["south-zap"] = zap();
    }
    if south_spells.contains(&"south-minion") {
        let mut south = minion(&json!({}));
        south
            .as_object_mut()
            .expect("south minion facts")
            .extend(south_extra.as_object().expect("extra south facts").clone());
        cards["south-minion"] = south;
    }
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "footprint-composition-rules" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-footprint-composition-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": north_spells,
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": south_spells,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical manifest")
}

fn composition_session(
    giant_extra: &Value,
    south_extra: &Value,
    north_spells: &[&str],
    south_spells: &[&str],
    required_north: &[&str],
) -> Session {
    let manifest = (1u32..=512)
        .map(|seed| {
            composition_manifest(seed, giant_extra, south_extra, north_spells, south_spells)
        })
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            let preview_state = state(&preview);
            let hand = preview_state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("north Spellbook hand");
            required_north
                .iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
        })
        .expect("opening hand with the required north spells");
    Session::new(&manifest).expect("composition session")
}

fn play_first_domains(session: &mut Session) {
    keep(session);
    keep(session);
    play_site(session, "C4");
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);
}

fn establish_north_square(session: &mut Session) {
    play_first_domains(session);
    play_site(session, "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "B3");
}

fn establish_north_square_and_south_c2(session: &mut Session) -> String {
    play_first_domains(session);
    play_site(session, "B4");
    end_and_draw(session);
    play_site(session, "C2");
    let (enemy, _) = summon_at(session, "south-minion", "C2");
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "B3");
    enemy
}

fn establish_north_square_and_south_c4(session: &mut Session) -> String {
    keep(session);
    keep(session);
    play_site(session, "C4");
    end_and_draw(session);
    play_site(session, "C1");
    let (enemy, _) = summon_at(session, "south-minion", "C4");
    end_and_draw(session);
    play_site(session, "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "B3");
    enemy
}

fn establish_north_square_and_south_d2(session: &mut Session) -> String {
    play_first_domains(session);
    play_site(session, "B4");
    end_and_draw(session);
    play_site(session, "C2");
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    play_site(session, "D2");
    let (enemy, _) = summon_at(session, "south-minion", "D2");
    end_and_draw_zone(session, "atlas");
    play_site(session, "B3");
    enemy
}

#[test]
fn rule_catalog_0167_oversized_genesis_still_draws_after_summoning() {
    let mut session = composition_session(
        &json!({ "genesisDrawSpells": 1 }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    let before = state(&session);
    let drawn_id = before["players"]["north"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("top Spellbook identity")
        .to_owned();
    let (giant, receipt) = summon_at(&mut session, "north-giant", "B3");
    assert_eq!(
        unit(&state(&session), &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_eq!(event_types(&receipt), ["minion-summoned", "spell-drawn"]);
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(&drawn_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0168_oversized_spellcaster_originates_nearby_magic_from_every_footprint_cell() {
    let mut session = composition_session(
        &json!({ "spellcaster": true }),
        &json!({}),
        &[
            "north-giant",
            "north-giant",
            "north-giant",
            "north-giant",
            "north-freeze",
            "north-freeze",
            "north-freeze",
            "north-freeze",
        ],
        &["south-minion"; 8],
        &["north-giant", "north-freeze"],
    );
    let enemy = establish_north_square_and_south_d2(&mut session);
    let (giant, summon) = summon_at(&mut session, "north-giant", "B3");
    assert_eq!(
        summon.events.first().expect("summon event").payload["cells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let actions = session
        .legal_actions()
        .expect("Freeze actions after the oversized summon");
    assert!(
        !actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-freeze"
                && action.descriptor["casterInstanceId"] == avatar_id
                && action.descriptor["target"]["instanceId"] == enemy
        }),
        "D2 is nearby only to a non-anchor footprint cell, not the C4 Avatar"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["casterInstanceId"] == giant
            && descriptor["target"]["instanceId"] == enemy
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert_eq!(
        unit(&state(&session), &enemy)["disableEffects"][0]["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0169_oversized_deathrite_damages_units_sharing_any_footprint_cell() {
    let mut session = composition_session(
        &json!({
            "deathriteDamageEachUnitHere": 1,
            "defense": 0,
        }),
        &json!({}),
        &["north-giant"; 8],
        &["south-zap"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    end_and_draw(&mut session);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-zap"
            && descriptor["target"]["instanceId"] == giant
    });
    let allocated: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "deathrite-damage-allocated")
        .map(|event| {
            event.payload["targetInstanceId"]
                .as_str()
                .expect("Deathrite target")
                .to_owned()
        })
        .collect();
    assert_eq!(allocated, [avatar_id]);
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["life"],
        19,
        "the C4 Avatar shares the oversized footprint, not the B3 anchor"
    );
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0170_oversized_genesis_here_damages_units_sharing_any_footprint_cell() {
    let mut session = composition_session(
        &json!({
            "defense": 10,
            "genesisDamageEachOtherUnitHere": 1,
        }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let (giant, receipt) = summon_at(&mut session, "north-giant", "B3");
    assert_eq!(
        unit(&state(&session), &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_eq!(
        event_types(&receipt),
        [
            "minion-summoned",
            "genesis-damage-allocated",
            "damage-dealt",
            "avatar-life-lost"
        ]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "amount": 1,
            "sourceInstanceId": giant,
            "targetInstanceId": avatar_id,
        })
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 19);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0171_oversized_genesis_strike_hits_enemies_sharing_any_footprint_cell() {
    let mut session = composition_session(
        &json!({
            "attack": 2,
            "defense": 10,
            "genesisStrikeEachEnemyHere": true,
        }),
        &json!({
            "defense": 10,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    let enemy = establish_north_square_and_south_c4(&mut session);
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let (giant, receipt) = summon_at(&mut session, "north-giant", "B3");
    let struck: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 2);
            assert_eq!(event.payload["strikerInstanceId"], giant.as_str());
            event.payload["targetInstanceId"]
                .as_str()
                .expect("struck identity")
                .to_owned()
        })
        .collect();
    assert_eq!(struck, [enemy.clone()]);
    assert!(!struck.contains(&avatar_id));
    assert_eq!(unit(&state(&session), &enemy)["damage"], 2);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0172_oversized_adjacent_genesis_reaches_units_bordering_any_footprint_cell() {
    let mut session = composition_session(
        &json!({
            "defense": 10,
            "genesisMayDamageTargetAdjacentUnit": 2,
        }),
        &json!({ "defense": 10 }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    let enemy = establish_north_square_and_south_c2(&mut session);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["genesisDamageTarget"]["instanceId"] == enemy
    });
    assert_eq!(
        event_types(&receipt),
        [
            "minion-summoned",
            "genesis-damage-allocated",
            "damage-dealt"
        ]
    );
    assert_eq!(unit(&state(&session), &enemy)["damage"], 2);
    assert_exact_replay(&session);
}
