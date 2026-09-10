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

fn special_north_site(giant_extra: &Value) -> Option<(&'static str, Value)> {
    if giant_extra.get("waterbound").and_then(Value::as_bool) == Some(true) {
        Some((
            "north-water",
            json!({ "cardType": "site", "elements": ["water"] }),
        ))
    } else if giant_extra
        .get("gainsPowerRangedAndSpellcasterAtopTower")
        .is_some()
    {
        Some((
            "north-tower",
            json!({ "cardType": "site", "elements": ["earth"], "isTower": true }),
        ))
    } else if giant_extra.get("ordinary").and_then(Value::as_bool) == Some(true) {
        Some((
            "north-hamlet",
            json!({
                "cardType": "site",
                "elements": ["earth"],
                "ordinaryMinionManaDiscount": 1,
            }),
        ))
    } else if giant_extra
        .get("mustBeCastToWaterSite")
        .and_then(Value::as_bool)
        == Some(true)
    {
        Some((
            "north-water",
            json!({ "cardType": "site", "elements": ["water"] }),
        ))
    } else {
        None
    }
}

#[derive(Clone, Copy)]
enum NorthAtlasPlan {
    FromGiantExtra,
    AllWater,
}

fn water_site() -> Value {
    json!({ "cardType": "site", "elements": ["water"] })
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

fn play_named_site(session: &mut Session, card_id: &str, cell: &str) {
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
    atlas_plan: NorthAtlasPlan,
) -> String {
    let mut giant = minion(&json!({ "occupiesSquareArea": 2 }));
    giant
        .as_object_mut()
        .expect("giant facts")
        .extend(giant_extra.as_object().expect("extra giant facts").clone());
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-giant": giant,
        "south-avatar": avatar(),
        "south-site": site(false),
    });
    if !matches!(atlas_plan, NorthAtlasPlan::AllWater) {
        cards["north-site"] = site(false);
    }
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
    if north_spells.contains(&"north-fodder") {
        cards["north-fodder"] = minion(&json!({}));
    }
    if let Some((card_id, definition)) = special_north_site(giant_extra) {
        cards[card_id] = definition;
    }
    let north_atlas = match atlas_plan {
        NorthAtlasPlan::AllWater => {
            cards["north-water"] = water_site();
            vec!["north-water"; 9]
        }
        NorthAtlasPlan::FromGiantExtra => {
            if let Some((card_id, _)) = special_north_site(giant_extra) {
                let mut atlas = vec!["north-site"; 8];
                atlas.insert(0, card_id);
                atlas
            } else {
                vec!["north-site"; 9]
            }
        }
    };
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
                "atlas": north_atlas,
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
    composition_session_with_atlas(
        giant_extra,
        south_extra,
        north_spells,
        south_spells,
        required_north,
        NorthAtlasPlan::FromGiantExtra,
    )
}

fn composition_session_all_water(
    giant_extra: &Value,
    south_extra: &Value,
    north_spells: &[&str],
    south_spells: &[&str],
    required_north: &[&str],
) -> Session {
    composition_session_with_atlas(
        giant_extra,
        south_extra,
        north_spells,
        south_spells,
        required_north,
        NorthAtlasPlan::AllWater,
    )
}

fn composition_session_with_atlas(
    giant_extra: &Value,
    south_extra: &Value,
    north_spells: &[&str],
    south_spells: &[&str],
    required_north: &[&str],
    atlas_plan: NorthAtlasPlan,
) -> Session {
    let manifest = (1u32..=512)
        .map(|seed| {
            composition_manifest(
                seed,
                giant_extra,
                south_extra,
                north_spells,
                south_spells,
                atlas_plan,
            )
        })
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            let preview_state = state(&preview);
            let hand = preview_state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("north Spellbook hand");
            let atlas = preview_state["players"]["north"]["hand"]["atlas"]
                .as_array()
                .expect("north Atlas hand");
            let required_atlas = special_north_site(giant_extra).map(|(card_id, _)| card_id);
            required_north
                .iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
                && required_atlas
                    .is_none_or(|card_id| atlas.iter().any(|card| card["cardId"] == card_id))
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

fn establish_north_square_named_at_c4(session: &mut Session, first_site: &str) {
    keep(session);
    keep(session);
    play_named_site(session, first_site, "C4");
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);
    play_named_site(session, "north-site", "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_named_site(session, "north-site", "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_named_site(session, "north-site", "B3");
}

fn establish_north_square_water_at_c4(session: &mut Session) {
    establish_north_square_named_at_c4(session, "north-water");
}

fn establish_north_square_tower_at_c4(session: &mut Session) {
    establish_north_square_named_at_c4(session, "north-tower");
}

fn establish_north_square_hamlet_at_c4(session: &mut Session) {
    establish_north_square_named_at_c4(session, "north-hamlet");
}

fn establish_north_square_all_named(session: &mut Session, site: &str) {
    keep(session);
    keep(session);
    play_named_site(session, site, "C4");
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);
    play_named_site(session, site, "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_named_site(session, site, "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_named_site(session, site, "B3");
}

fn offers_summon(session: &Session, card_id: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "summon-minion" && action.descriptor["cardId"] == card_id
        })
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
    assert_eq!(struck.as_slice(), [enemy.as_str()]);
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

fn discard_here_candidate_count(receipt: &Receipt) -> usize {
    let draws: Vec<_> = receipt
        .random_draws
        .iter()
        .filter(|draw| draw["purpose"] == "discard_spell_random_other_unit_here")
        .collect();
    assert_eq!(draws.len(), 1, "one hidden random draw per activation");
    assert_eq!(draws[0]["domain"]["kind"], "unit_index_candidate");
    usize::try_from(
        draws[0]["domain"]["exclusiveMaximum"]
            .as_u64()
            .expect("candidate count"),
    )
    .expect("candidate count fits")
}

#[test]
fn rule_catalog_0173_oversized_discard_here_damages_units_sharing_any_footprint_cell() {
    let mut session = composition_session(
        &json!({
            "defense": 10,
            "discardSpellToDamageRandomOtherUnitHere": 1,
        }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    let before = state(&session);
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    assert_eq!(
        unit(&before, &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-discard-random-damage"
            && descriptor["sourceInstanceId"] == giant
    });
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "discard-random-damage-activated",
            "discard-random-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
        ]
    );
    assert_eq!(
        discard_here_candidate_count(&receipt),
        1,
        "the C4 Avatar is the sole other unit on the oversized footprint"
    );
    assert_eq!(
        receipt.events[1].payload["sourceLocation"]["cell"], "B3",
        "the activation still records the canonical anchor"
    );
    assert_eq!(receipt.events[1].payload["targetInstanceId"], avatar_id);
    assert_eq!(receipt.events[1].payload["targetKind"], "avatar");
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["life"],
        19,
        "the C4 Avatar shares the oversized footprint, not the B3 anchor"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0174_oversized_summon_to_any_site_uses_any_surface_cell_in_the_square() {
    let mut session = composition_session(
        &json!({}),
        &json!({
            "occupiesSquareArea": 2,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &[],
    );
    establish_north_square(&mut session);
    end_and_draw(&mut session);
    let (giant, receipt) = summon_at(&mut session, "south-minion", "B3");
    assert_eq!(
        receipt.events.first().expect("summon event").payload["cells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_eq!(
        unit(&state(&session), &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    let realm = &state(&session)["realm"];
    for cell in ["B3", "B4", "C3", "C4"] {
        assert_eq!(
            realm["sites"][cell]["controller"], "north",
            "{cell} is a North site, so South needs summonToAnySite"
        );
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0175_oversized_waterbound_uses_any_occupied_water_site() {
    let mut session = composition_session(
        &json!({ "charge": true, "waterbound": true }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square_water_at_c4(&mut session);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    let current = state(&session);
    assert_eq!(
        unit(&current, &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_eq!(current["realm"]["sites"]["C4"]["cardId"], "north-water");
    assert_eq!(current["realm"]["sites"]["B3"]["cardId"], "north-site");
    assert!(
        session
            .legal_actions()
            .expect("Waterbound actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "move-and-attack"
                    && action.descriptor["unitInstanceId"] == giant
            }),
        "C4 Water shares the oversized footprint, so Waterbound stays enabled at the B3 land anchor"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0176_oversized_threshold_suppression_covers_every_occupied_site() {
    let mut session = composition_session(
        &json!({
            "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
        }),
        &json!({
            "occupiesSquareArea": 2,
            "siteProvidesNoThreshold": true,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    assert!(
        offers_summon(&session, "north-giant"),
        "four Earth sites should meet the oversized minion's threshold before suppression"
    );
    end_and_draw(&mut session);
    let (rats, _) = summon_at(&mut session, "south-minion", "B3");
    assert_eq!(
        unit(&state(&session), &rats)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    end_and_draw(&mut session);
    assert!(
        !offers_summon(&session, "north-giant"),
        "enabled Rats must suppress every occupied Earth site, not only the B3 anchor"
    );
    assert_exact_replay(&session);
}

fn stage_south_on_north_cell(session: &mut Session, cell: &str) -> String {
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, cell);
    end_and_draw(session);
    let (enemy, _) = summon_at(session, "south-minion", cell);
    end_and_draw(session);
    enemy
}

fn stage_south_on_north_d3(session: &mut Session) -> String {
    stage_south_on_north_cell(session, "D3")
}

fn stage_south_on_north_d4(session: &mut Session) -> String {
    stage_south_on_north_cell(session, "D4")
}

#[test]
fn rule_catalog_0177_oversized_ranged_originates_from_every_footprint_cell() {
    let mut session = composition_session(
        &json!({ "ranged": true }),
        &json!({
            "defense": 10,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    let enemy = stage_south_on_north_d3(&mut session);
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let actions = session
        .legal_actions()
        .expect("Ranged actions after the oversized summon");
    assert!(
        !actions.iter().any(|action| {
            action.descriptor["kind"] == "shoot-projectile"
                && action.descriptor["shooterInstanceId"] == avatar_id
        }),
        "the C4 Avatar is not Ranged"
    );
    assert!(
        !actions.iter().any(|action| {
            action.descriptor["kind"] == "shoot-projectile"
                && action.descriptor["shooterInstanceId"] == giant
                && action.descriptor["hit"]["instanceId"] == enemy
                && action.descriptor["path"][0]["cell"] == "B3"
        }),
        "one-step Ranged from the B3 anchor cannot reach D3"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["shooterInstanceId"] == giant
            && descriptor["hit"]["instanceId"] == enemy
            && descriptor["path"]
                == json!([
                    { "cell": "C3", "region": "surface" },
                    { "cell": "D3", "region": "surface" },
                ])
    });
    assert_eq!(
        event_types(&receipt)[0..2],
        ["projectile-shot", "strike-damage-allocated"]
    );
    assert_eq!(unit(&state(&session), &enemy)["damage"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0178_oversized_tower_bonus_uses_any_occupied_tower() {
    let mut session = composition_session(
        &json!({ "gainsPowerRangedAndSpellcasterAtopTower": 2 }),
        &json!({
            "defense": 10,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square_tower_at_c4(&mut session);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    let current = state(&session);
    assert_eq!(current["realm"]["sites"]["C4"]["cardId"], "north-tower");
    assert_eq!(current["realm"]["sites"]["B3"]["cardId"], "north-site");
    let enemy = stage_south_on_north_d3(&mut session);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-projectile"
            && descriptor["shooterInstanceId"] == giant
            && descriptor["hit"]["instanceId"] == enemy
            && descriptor["path"]
                == json!([
                    { "cell": "C3", "region": "surface" },
                    { "cell": "D3", "region": "surface" },
                ])
    });
    assert_eq!(
        event_types(&receipt)[0..2],
        ["projectile-shot", "strike-damage-allocated"]
    );
    assert_eq!(
        receipt.events[1].payload["amount"], 3,
        "the C4 Tower shares the oversized footprint, so derived power is 1+2"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0179_oversized_ordinary_uses_any_occupied_hamlet() {
    let mut session = composition_session(
        &json!({ "manaCost": 1, "ordinary": true }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square_hamlet_at_c4(&mut session);
    let current = state(&session);
    assert_eq!(current["realm"]["sites"]["C4"]["cardId"], "north-hamlet");
    assert_eq!(current["realm"]["sites"]["B3"]["cardId"], "north-site");
    let mut costs = session
        .legal_actions()
        .expect("Ordinary oversized summon actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-giant"
                && action.descriptor["cell"] == "B3"
        })
        .map(|action| action.descriptor["manaCost"].as_u64().expect("summon cost"))
        .collect::<Vec<_>>();
    costs.sort_unstable();
    costs.dedup();
    assert_eq!(
        costs,
        [0],
        "C4 Hamlet shares the oversized footprint, so the B3-anchor summon is free"
    );
    let (giant, receipt) = summon_at(&mut session, "north-giant", "B3");
    assert_eq!(
        unit(&state(&session), &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_eq!(
        receipt.events.first().expect("summon event").payload["manaPaid"],
        0
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0180_oversized_sacrifice_uses_every_occupied_summoning_cell() {
    let mut session = composition_session(
        &json!({
            "manaCost": 6,
            "sacrificeMinionAtSummoningLocationForManaDiscount": 2,
        }),
        &json!({}),
        &[
            "north-giant",
            "north-giant",
            "north-giant",
            "north-giant",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
        ],
        &["south-minion"; 8],
        &["north-giant", "north-fodder"],
    );
    establish_north_square(&mut session);
    let (fodder, _) = summon_at(&mut session, "north-fodder", "C4");
    assert_eq!(
        unit(&state(&session), &fodder)["location"],
        "C4",
        "the sacrifice candidate stands on a non-anchor footprint cell"
    );
    let actions = session
        .legal_actions()
        .expect("sacrifice summon actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-giant"
                && action.descriptor["cell"] == "B3"
        })
        .map(|action| action.descriptor)
        .collect::<Vec<_>>();
    assert!(
        !actions.is_empty()
            && actions.iter().all(|descriptor| {
                descriptor["manaCost"] == 4
                    && descriptor["sacrificedMinionInstanceIds"] == json!([fodder])
            }),
        "four mana cannot pay the printed six without sacrificing the C4 minion on the footprint"
    );
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["sacrificedMinionInstanceIds"] == json!([fodder])
    });
    let giant = descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned instance")
        .to_owned();
    assert_eq!(
        event_types(&receipt),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert_eq!(
        unit(&state(&session), &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == fodder)
    );
    assert!(
        !after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|candidate| candidate["instanceId"] == fodder)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0181_oversized_water_site_cast_rejects_a_mixed_square() {
    let mut session = composition_session(
        &json!({ "mustBeCastToWaterSite": true }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square_water_at_c4(&mut session);
    let current = state(&session);
    assert_eq!(current["realm"]["sites"]["C4"]["cardId"], "north-water");
    assert_eq!(current["realm"]["sites"]["B3"]["cardId"], "north-site");
    assert!(
        !offers_summon(&session, "north-giant"),
        "a 2x2 water-site cast needs every occupied cell to be Water, not only C4"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0182_oversized_water_site_cast_occupies_an_all_water_square() {
    let mut session = composition_session_all_water(
        &json!({}),
        &json!({
            "mustBeCastToWaterSite": true,
            "occupiesSquareArea": 2,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &[],
    );
    establish_north_square_all_named(&mut session, "north-water");
    end_and_draw(&mut session);
    let realm = &state(&session)["realm"];
    for cell in ["B3", "B4", "C3", "C4"] {
        assert_eq!(
            realm["sites"][cell]["cardId"], "north-water",
            "{cell} must be Water before the oversized water-site cast"
        );
        assert_eq!(realm["sites"][cell]["controller"], "north");
    }
    let (giant, receipt) = summon_at(&mut session, "south-minion", "B3");
    assert_eq!(
        receipt.events.first().expect("summon event").payload["cells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_eq!(
        unit(&state(&session), &giant)["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0183_oversized_activated_projectile_originates_from_every_footprint_cell() {
    let mut session = composition_session(
        &json!({ "tapToShootProjectileDamage": 1 }),
        &json!({
            "defense": 10,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    let enemy = stage_south_on_north_d4(&mut session);
    let actions = session
        .legal_actions()
        .expect("activated projectile actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "shoot-damage-projectile"
                && action.descriptor["shooterInstanceId"] == giant
                && action.descriptor["hit"]["instanceId"] == enemy
        })
        .map(|action| action.descriptor)
        .collect::<Vec<_>>();
    assert!(
        !actions
            .iter()
            .any(|descriptor| descriptor["path"][0]["cell"] == "B3"),
        "an east ray from the B3 anchor reaches D3, not D4"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-damage-projectile"
            && descriptor["shooterInstanceId"] == giant
            && descriptor["hit"]["instanceId"] == enemy
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "D4", "region": "surface" },
                ])
    });
    assert_eq!(
        event_types(&receipt)[0..2],
        ["projectile-shot", "projectile-damage-allocated"]
    );
    assert_eq!(receipt.events[1].payload["amount"], 1);
    assert_eq!(unit(&state(&session), &enemy)["damage"], 1);
    assert_eq!(unit(&state(&session), &giant)["tapped"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0184_oversized_drag_projectile_originates_from_every_footprint_cell() {
    let mut session = composition_session(
        &json!({ "shootsDragProjectile": true }),
        &json!({
            "defense": 10,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    let enemy = stage_south_on_north_d4(&mut session);
    let actions = session
        .legal_actions()
        .expect("drag projectile actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "shoot-drag-projectile"
                && action.descriptor["shooterInstanceId"] == giant
                && action.descriptor["hit"]["instanceId"] == enemy
                && action.descriptor["fightOnArrival"] == false
        })
        .map(|action| action.descriptor)
        .collect::<Vec<_>>();
    assert!(
        !actions
            .iter()
            .any(|descriptor| descriptor["path"][0]["cell"] == "B3"),
        "an east ray from the B3 anchor reaches D3, not D4"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-drag-projectile"
            && descriptor["shooterInstanceId"] == giant
            && descriptor["hit"]["instanceId"] == enemy
            && descriptor["fightOnArrival"] == false
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "D4", "region": "surface" },
                ])
    });
    assert_eq!(event_types(&receipt), ["projectile-shot", "unit-dragged"]);
    assert_eq!(
        receipt.events[1].payload["from"],
        json!({ "cell": "D4", "region": "surface" })
    );
    assert_eq!(
        receipt.events[1].payload["to"],
        json!({ "cell": "C4", "region": "surface" }),
        "the haul returns to the C4 origin, not the B3 anchor"
    );
    assert_eq!(unit(&state(&session), &enemy)["location"], "C4");
    assert_eq!(unit(&state(&session), &giant)["tapped"], true);
    assert_exact_replay(&session);
}

fn summon_regions_at(session: &Session, card_id: &str, cell: &str) -> Vec<String> {
    let mut regions = session
        .legal_actions()
        .expect("summon regions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == card_id
                && action.descriptor["cell"] == cell
        })
        .map(|action| {
            action.descriptor["region"]
                .as_str()
                .unwrap_or("surface")
                .to_owned()
        })
        .collect::<Vec<_>>();
    regions.sort();
    regions.dedup();
    regions
}

#[test]
fn rule_catalog_0185_oversized_burrowing_summons_underground_on_an_all_land_square() {
    let mut session = composition_session(
        &json!({ "burrowing": true }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    assert_eq!(
        summon_regions_at(&session, "north-giant", "B3"),
        ["surface", "underground"],
        "four land sites offer both surface and underground 2x2 placements"
    );
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"] == "underground"
    });
    let giant = descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned instance")
        .to_owned();
    assert_eq!(event_types(&receipt), ["minion-summoned"]);
    let after = state(&session);
    let summoned = unit(&after, &giant);
    assert_eq!(summoned["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(summoned["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0186_oversized_submerge_summons_underwater_on_an_all_water_square() {
    let mut session = composition_session_all_water(
        &json!({ "submerge": true }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square_all_named(&mut session, "north-water");
    assert_eq!(
        summon_regions_at(&session, "north-giant", "B3"),
        ["surface", "underwater"],
        "four Water sites offer both surface and underwater 2x2 placements"
    );
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"] == "underwater"
    });
    let giant = descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned instance")
        .to_owned();
    assert_eq!(event_types(&receipt), ["minion-summoned"]);
    let after = state(&session);
    let summoned = unit(&after, &giant);
    assert_eq!(summoned["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(summoned["region"], "underwater");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0187_oversized_burrowed_only_cast_offers_only_underground() {
    let mut session = composition_session(
        &json!({
            "burrowing": true,
            "mustBeCastBurrowed": true,
        }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    assert_eq!(
        summon_regions_at(&session, "north-giant", "B3"),
        ["underground"],
        "a burrowed-only 2x2 cannot use the surface of an all-land square"
    );
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"] == "underground"
    });
    let giant = descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned instance")
        .to_owned();
    assert_eq!(event_types(&receipt), ["minion-summoned"]);
    let after = state(&session);
    let summoned = unit(&after, &giant);
    assert_eq!(summoned["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(summoned["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0188_oversized_submerged_only_cast_offers_only_underwater() {
    let mut session = composition_session_all_water(
        &json!({
            "mustBeCastSubmerged": true,
            "submerge": true,
        }),
        &json!({}),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square_all_named(&mut session, "north-water");
    assert_eq!(
        summon_regions_at(&session, "north-giant", "B3"),
        ["underwater"],
        "a submerged-only 2x2 cannot use the surface of an all-Water square"
    );
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"] == "underwater"
    });
    let giant = descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned instance")
        .to_owned();
    assert_eq!(event_types(&receipt), ["minion-summoned"]);
    let after = state(&session);
    let summoned = unit(&after, &giant);
    assert_eq!(summoned["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));
    assert_eq!(summoned["region"], "underwater");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0189_oversized_area_damage_reaches_cells_adjacent_to_any_footprint_cell() {
    let mut session = composition_session(
        &json!({ "tapToDamageEachUnitAtAdjacentLocation": 2 }),
        &json!({
            "defense": 10,
            "summonToAnySite": true,
        }),
        &["north-giant"; 8],
        &["south-minion"; 8],
        &["north-giant"],
    );
    establish_north_square(&mut session);
    let (giant, _) = summon_at(&mut session, "north-giant", "B3");
    let enemy = stage_south_on_north_d4(&mut session);
    let actions = session
        .legal_actions()
        .expect("area-damage actions after the oversized summon");
    assert!(
        actions.iter().any(|action| {
            action.descriptor["kind"] == "activate-area-damage"
                && action.descriptor["sourceInstanceId"] == giant
                && action.descriptor["targetLocation"]
                    == json!({ "cell": "D4", "region": "surface" })
        }),
        "C4 borders D4, so the B3-anchored 2x2 must offer that adjacent blanket"
    );
    assert!(
        !actions.iter().any(|action| {
            action.descriptor["kind"] == "activate-area-damage"
                && action.descriptor["sourceInstanceId"] == giant
                && ["B3", "B4", "C3", "C4"].contains(
                    &action.descriptor["targetLocation"]["cell"]
                        .as_str()
                        .unwrap_or(""),
                )
        }),
        "occupied footprint cells are not adjacent targets"
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-area-damage"
            && descriptor["sourceInstanceId"] == giant
            && descriptor["targetLocation"] == json!({ "cell": "D4", "region": "surface" })
    });
    assert_eq!(
        event_types(&receipt)[0..2],
        ["area-damage-activated", "area-damage-allocated"]
    );
    assert_eq!(unit(&state(&session), &enemy)["damage"], 2);
    assert_eq!(unit(&state(&session), &giant)["tapped"], true);
    assert_exact_replay(&session);
}
