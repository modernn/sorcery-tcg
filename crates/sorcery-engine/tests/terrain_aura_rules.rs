//! Direct proofs for Flood and Drought terrain Auras (RULE-CATALOG-0266–0267,
//! RULE-CATALOG-0775, RULE-CATALOG-0912, RULE-CATALOG-1169).
//!
//! Official Flood is a persistent 2×2 Aura: affected sites are flooded, so they
//! are Water sites and still provide their other elemental affinities. Official
//! Drought is the later-timestamp inverse: affected sites are not Water sites
//! and provide no Water threshold. Neither uses the 3-turn immobilize machine.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn flood() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drought() -> Value {
    json!({
        "affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn landbound() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "landbound": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn submerge_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dual_region_minion() -> Value {
    let mut value = submerge_minion();
    value["burrowing"] = json!(true);
    value
}

fn submerge_magic() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "submergeTargetMinion": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn destroy_site_magic() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn artifact() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bury_artifact_magic() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn square_landbound() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "landbound": true,
        "manaCost": 0,
        "occupiesSquareArea": 2,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(north_spell: &str, south_spell: &str) -> String {
    let north_card = match north_spell {
        "north-flood" => flood(),
        "north-drought" => drought(),
        _ => panic!("unsupported north spell {north_spell}"),
    };
    let south_card = match south_spell {
        "south-drought" => drought(),
        "south-flood" => flood(),
        "south-minion" => minion(),
        _ => panic!("unsupported south spell {south_spell}"),
    };
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    cards[north_spell] = north_card;
    cards[south_spell] = south_card;
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "terrain-aura" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-terrain-aura-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec![north_spell; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec![south_spell; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
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
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
        });
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

fn north_affinity(session: &Session) -> (u64, u64) {
    let view = session.public_view(Seat::North).expect("North public view");
    (
        view["players"]["north"]["affinity"]["earth"]
            .as_u64()
            .expect("earth affinity"),
        view["players"]["north"]["affinity"]["water"]
            .as_u64()
            .expect("water affinity"),
    )
}

fn observed_unit(session: &Session, instance_id: &str) -> Value {
    session.public_view(Seat::North).expect("North public view")["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("named unit")
        .clone()
}

fn opening_spell_ids(session: &Session, seat: &str) -> Vec<String> {
    state(session)["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("spellbook")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn composition_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "terrain-aura-composition" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-terrain-aura-composition-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-flood": flood(),
            "north-site": site(),
            "north-square-landbound": square_landbound(),
            "south-avatar": avatar(),
            "south-drought": drought(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-flood",
                    "north-flood",
                    "north-flood",
                    "north-flood",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-drought",
                    "south-drought",
                    "south-drought",
                    "south-drought",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                ],
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

fn rubble_flood_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "terrain-aura-flooded-rubble" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-terrain-aura-flooded-rubble-v1",
        },
        "cards": {
            "north-artifact": artifact(),
            "north-avatar": avatar(),
            "north-bury": bury_artifact_magic(),
            "north-destroy": destroy_site_magic(),
            "north-drought": drought(),
            "north-flood": flood(),
            "north-landbound": landbound(),
            "north-site": site(),
            "north-submerge": submerge_magic(),
            "north-swimmer": dual_region_minion(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-swimmer", "north-artifact", "north-bury", "north-flood",
                    "north-submerge", "north-destroy", "north-bury", "north-flood",
                    "north-submerge", "north-destroy", "north-bury", "north-swimmer",
                    "north-landbound", "north-drought", "north-flood",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 12],
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

fn swap_seat_label(label: &str) -> String {
    if label == "north" {
        "south".to_owned()
    } else if label == "south" {
        "north".to_owned()
    } else if let Some(suffix) = label.strip_prefix("north-") {
        format!("south-{suffix}")
    } else if let Some(suffix) = label.strip_prefix("south-") {
        format!("north-{suffix}")
    } else {
        label.to_owned()
    }
}

fn mirror_manifest_seats(value: Value) -> Value {
    match value {
        Value::String(label) => Value::String(swap_seat_label(&label)),
        Value::Array(items) => Value::Array(items.into_iter().map(mirror_manifest_seats).collect()),
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (swap_seat_label(&key), mirror_manifest_seats(value)))
                .collect(),
        ),
        value => value,
    }
}

fn rubble_flood_manifest_for(seed: u32, first: &str) -> String {
    let encoded = rubble_flood_manifest(seed);
    if first == "north" {
        return encoded;
    }
    let mut value = mirror_manifest_seats(serde_json::from_str(&encoded).unwrap());
    value.as_object_mut().unwrap().remove("manifestId");
    value["firstSeat"] = json!(first);
    value["manifestId"] = json!(identity_hash(&value).expect("mirrored manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical mirrored manifest")
}

fn composition_opening() -> Session {
    (1..=4096)
        .map(composition_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("terrain Aura composition candidate");
            let north_spells = opening_spell_ids(&session, "north");
            let south_spells = opening_spell_ids(&session, "south");
            (north_spells
                .iter()
                .any(|card| card == "north-square-landbound")
                && north_spells.iter().any(|card| card == "north-flood")
                && south_spells.iter().any(|card| card == "south-drought"))
            .then_some(session)
        })
        .expect("bounded seed opening with 2x2 Landbound, Flood, and Drought")
}

fn landbound_movement_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "terrain-aura-landbound-movement" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-terrain-aura-landbound-movement-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-flood": flood(),
            "north-landbound": landbound(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-drought": drought(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-landbound",
                    "north-landbound",
                    "north-landbound",
                    "north-flood",
                    "north-flood",
                    "north-flood",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-drought",
                    "south-drought",
                    "south-drought",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                ],
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

fn landbound_movement_opening() -> Session {
    (1..=4096)
        .map(landbound_movement_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Landbound movement candidate");
            let north_spells = opening_spell_ids(&session, "north");
            let south_spells = opening_spell_ids(&session, "south");
            (north_spells.iter().any(|card| card == "north-landbound")
                && north_spells.iter().any(|card| card == "north-flood")
                && south_spells.iter().any(|card| card == "south-drought"))
            .then_some(session)
        })
        .expect("bounded seed opening with Landbound, Flood, and Drought")
}

fn play_site_at(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn end_then_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn summon_square_landbound_at_b3(session: &mut Session) -> String {
    keep(session);
    keep(session);
    play_site_at(session, "C4");
    end_then_draw(session, "spellbook");
    play_site_at(session, "C1");
    end_then_draw(session, "atlas");
    play_site_at(session, "B4");
    end_then_draw(session, "spellbook");
    end_then_draw(session, "atlas");
    play_site_at(session, "C3");
    end_then_draw(session, "spellbook");
    end_then_draw(session, "atlas");
    play_site_at(session, "B3");
    end_then_draw(session, "spellbook");
    end_then_draw(session, "atlas");
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-square-landbound"
            && descriptor["cell"] == "B3"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("2x2 Landbound identity")
        .to_owned()
}

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn moves_bound(bound_id: &str) -> impl Fn(&Value) -> bool + '_ {
    move |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack" && descriptor["unitInstanceId"] == bound_id
    }
}

fn moves_bound_to<'a>(bound_id: &'a str, cell: &'a str) -> impl Fn(&Value) -> bool + 'a {
    move |descriptor: &Value| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bound_id
            && descriptor["to"]["cell"] == cell
            && descriptor["to"]["region"] == "surface"
    }
}

fn cast_covering(session: &mut Session, card_id: &str, cell: &str) -> Value {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == card_id
            && cells_include(descriptor, cell)
    })
    .0
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

fn full_session_fingerprint(session: &Session) -> Value {
    let checkpoint = create_game_checkpoint(session).expect("complete Session checkpoint");
    json!({
        "checkpoint": serialize_game_checkpoint(&checkpoint).expect("checkpoint bytes"),
        "stateHash": session.state_hash().expect("state hash"),
        "sessionHash": session.session_hash().expect("session hash"),
        "replay": session.replay_value().expect("replay envelope"),
        "transcript": session.transcript(),
        "northView": session.public_view(Seat::North).unwrap(),
        "southView": session.public_view(Seat::South).unwrap(),
        "legalActions": serde_json::to_value(session.legal_actions().unwrap()).unwrap(),
    })
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keeps the required flooded Rubble Session setup, checkpoint, and replay proof together"
)]
fn flooded_site_to_rubble_keeps_submerged_occupants_and_artifact_region() {
    for caster in ["north", "south"] {
        let target_cell = if caster == "north" { "C4" } else { "C1" };
        let opponent_cell = if caster == "north" { "C1" } else { "C4" };
        let swimmer_card = format!("{caster}-swimmer");
        let artifact_card = format!("{caster}-artifact");
        let bury_card = format!("{caster}-bury");
        let drought_card = format!("{caster}-drought");
        let flood_card = format!("{caster}-flood");
        let landbound_card = format!("{caster}-landbound");
        let submerge_card = format!("{caster}-submerge");
        let destroy_card = format!("{caster}-destroy");
        let mut selected = None;
        for seed in 1..=4096 {
            let mut session = Session::new(&rubble_flood_manifest_for(seed, caster))
                .expect("flooded rubble candidate");
            let hand = opening_spell_ids(&session, caster);
            let has = |card: &str| hand.iter().any(|id| id == card);
            if !(has(&swimmer_card) && has(&artifact_card)) {
                continue;
            }

            keep(&mut session);
            keep(&mut session);
            play_site_at(&mut session, target_cell);
            let (summoned, _) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == swimmer_card
                    && descriptor["cell"] == target_cell
            });
            let swimmer_id = summoned["cardInstanceId"].as_str().unwrap().to_owned();
            let (artifact_cast, _) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == artifact_card
                    && descriptor["cell"] == target_cell
                    && descriptor["bearer"].is_null()
            });
            let artifact_id = artifact_cast["cardInstanceId"].as_str().unwrap().to_owned();
            let mut opponent_site_played = false;
            let mut buried = false;
            let mut flooded = false;
            let mut submerged = false;
            let mut landbound_summoned = false;
            let mut landbound_id = String::new();
            for _ in 0..16 {
                if !buried
                    && offers(&session, |descriptor| {
                        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == bury_card
                    })
                {
                    accept_where(&mut session, |descriptor| {
                        descriptor["kind"] == "cast-magic"
                            && descriptor["cardId"] == bury_card
                            && descriptor["targetArtifactInstanceId"] == artifact_id
                    });
                    buried = true;
                }
                if !landbound_summoned
                    && offers(&session, |descriptor| {
                        descriptor["kind"] == "summon-minion"
                            && descriptor["cardId"] == landbound_card
                            && descriptor["cell"] == target_cell
                    })
                {
                    let (summoned, _) = accept_where(&mut session, |descriptor| {
                        descriptor["kind"] == "summon-minion"
                            && descriptor["cardId"] == landbound_card
                            && descriptor["cell"] == target_cell
                    });
                    landbound_id = summoned["cardInstanceId"].as_str().unwrap().to_owned();
                    landbound_summoned = true;
                }
                if buried
                    && landbound_summoned
                    && !flooded
                    && offers(&session, |descriptor| {
                        descriptor["kind"] == "cast-aura" && descriptor["cardId"] == flood_card
                    })
                {
                    cast_covering(&mut session, &flood_card, target_cell);
                    flooded = true;
                }
                if flooded && !submerged {
                    if observed_unit(&session, &swimmer_id)["region"] == "underwater" {
                        submerged = true;
                    } else if offers(&session, |descriptor| {
                        descriptor["kind"] == "cast-magic"
                            && descriptor["cardId"] == submerge_card
                            && descriptor["target"]["instanceId"] == swimmer_id
                    }) {
                        accept_where(&mut session, |descriptor| {
                            descriptor["kind"] == "cast-magic"
                                && descriptor["cardId"] == submerge_card
                                && descriptor["target"]["instanceId"] == swimmer_id
                        });
                        submerged = true;
                    }
                }
                if flooded
                    && submerged
                    && landbound_summoned
                    && offers(&session, |descriptor| {
                        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == destroy_card
                    })
                {
                    let current_state = state(&session);
                    let hand = current_state["players"][caster]["hand"]["spellbook"]
                        .as_array()
                        .unwrap();
                    if hand.iter().any(|card| card["cardId"] == drought_card)
                        && hand.iter().any(|card| card["cardId"] == flood_card)
                    {
                        selected = Some((seed, session, swimmer_id, artifact_id, landbound_id));
                        break;
                    }
                }

                end_then_draw(&mut session, "spellbook");
                if !opponent_site_played {
                    play_site_at(&mut session, opponent_cell);
                    opponent_site_played = true;
                }
                end_then_draw(&mut session, "spellbook");
            }
            if selected.is_some() {
                break;
            }
        }
        let (seed, mut session, swimmer_id, artifact_id, landbound_id) = selected
            .expect("bounded seed that draws both overlay Auras, Landbound, Submerge, and Destroy");

        let submerged = observed_unit(&session, &swimmer_id);
        assert_eq!(submerged["region"], "underwater");
        let pre_artifact = state(&session)["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["instanceId"] == artifact_id)
            .unwrap()
            .clone();
        assert_eq!(pre_artifact["region"], "underwater");
        assert_eq!(observed_unit(&session, &landbound_id)["disabled"], true);
        assert_eq!(observed_unit(&session, &landbound_id)["region"], "surface");

        let pre_parent = session.clone();
        let pre_parent_fingerprint = full_session_fingerprint(&pre_parent);
        let pre_checkpoint =
            create_game_checkpoint(&session).expect("flooded Site pre-destroy checkpoint");
        let pre_serialized =
            serialize_game_checkpoint(&pre_checkpoint).expect("serialize pre-destroy");
        let mut branch = resume_game_checkpoint(
            &parse_game_checkpoint(&pre_serialized).expect("parse pre-destroy"),
        )
        .expect("resume pre-destroy");
        assert_eq!(
            branch.replay_value().unwrap(),
            session.replay_value().unwrap()
        );
        assert_eq!(
            branch.session_hash().unwrap(),
            session.session_hash().unwrap()
        );
        assert_eq!(
            branch.public_view(Seat::North).unwrap(),
            session.public_view(Seat::North).unwrap()
        );
        assert_eq!(
            branch.public_view(Seat::South).unwrap(),
            session.public_view(Seat::South).unwrap()
        );
        assert_eq!(
            branch.legal_actions().unwrap(),
            session.legal_actions().unwrap()
        );

        let (destroy_descriptor, destroy_receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == destroy_card
                && descriptor["targetLocation"]["cell"] == target_cell
        });
        assert!(
            destroy_receipt
                .events
                .iter()
                .any(|event| event.event_type == "site-destroyed")
        );
        assert!(
            destroy_receipt
                .events
                .iter()
                .any(|event| event.event_type == "rubble-created")
        );
        let after_site = state(&session)["realm"]["sites"][target_cell].clone();
        assert_eq!(after_site["rubble"], true);
        assert_eq!(observed_unit(&session, &swimmer_id)["region"], "underwater");
        assert!(
            offers(&session, |descriptor| {
                descriptor["kind"] == "move-and-attack"
                    && descriptor["unitInstanceId"] == swimmer_id
                    && descriptor["from"]["region"] == "underwater"
                    && descriptor["to"]["cell"] == target_cell
                    && descriptor["to"]["region"] == "surface"
            }),
            "Flooded Rubble keeps an underwater-to-surface move available"
        );
        let after_artifact = state(&session)["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["instanceId"] == artifact_id)
            .unwrap()
            .clone();
        assert_eq!(after_artifact["region"], "underwater");
        let crater_fingerprint = full_session_fingerprint(&session);

        let rubble_parent = session.clone();
        let rubble_parent_fingerprint = full_session_fingerprint(&rubble_parent);
        let rubble_checkpoint =
            create_game_checkpoint(&session).expect("post-Rubble overlay checkpoint");
        let rubble_checkpoint = serialize_game_checkpoint(&rubble_checkpoint).unwrap();
        let rubble_checkpoint = parse_game_checkpoint(&rubble_checkpoint).unwrap();
        let mut rubble_branch =
            resume_game_checkpoint(&rubble_checkpoint).expect("restore Rubble overlay parent");
        let (_, drought_receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-aura"
                && descriptor["cardId"] == drought_card
                && cells_include(descriptor, target_cell)
        });
        let (_, restored_drought_receipt) = accept_where(&mut rubble_branch, |descriptor| {
            descriptor["kind"] == "cast-aura"
                && descriptor["cardId"] == drought_card
                && cells_include(descriptor, target_cell)
        });
        assert_eq!(drought_receipt, restored_drought_receipt);
        assert_eq!(
            observed_unit(&session, &swimmer_id)["region"],
            "underground"
        );
        assert_eq!(observed_unit(&session, &landbound_id)["disabled"], false);
        assert_eq!(observed_unit(&session, &landbound_id)["region"], "surface");
        let drought_state = state(&session);
        let drought_artifact = drought_state["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["instanceId"] == artifact_id)
            .unwrap();
        assert_eq!(drought_artifact["region"], "underground");
        assert_eq!(
            full_session_fingerprint(&rubble_parent),
            rubble_parent_fingerprint
        );
        assert_eq!(
            full_session_fingerprint(&rubble_branch),
            full_session_fingerprint(&session)
        );

        let drought_parent = session.clone();
        let drought_parent_fingerprint = full_session_fingerprint(&drought_parent);
        let drought_checkpoint =
            create_game_checkpoint(&session).expect("Drought Rubble checkpoint");
        let drought_checkpoint = serialize_game_checkpoint(&drought_checkpoint).unwrap();
        let drought_checkpoint = parse_game_checkpoint(&drought_checkpoint).unwrap();
        let mut drought_branch =
            resume_game_checkpoint(&drought_checkpoint).expect("restore Drought Rubble");
        let (_, latest_flood_receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-aura"
                && descriptor["cardId"] == flood_card
                && cells_include(descriptor, target_cell)
        });
        let (_, restored_flood_receipt) = accept_where(&mut drought_branch, |descriptor| {
            descriptor["kind"] == "cast-aura"
                && descriptor["cardId"] == flood_card
                && cells_include(descriptor, target_cell)
        });
        assert_eq!(latest_flood_receipt, restored_flood_receipt);
        assert_eq!(observed_unit(&session, &swimmer_id)["region"], "underwater");
        assert_eq!(observed_unit(&session, &landbound_id)["disabled"], true);
        assert_eq!(observed_unit(&session, &landbound_id)["region"], "surface");
        let latest_flood_state = state(&session);
        let latest_flood_artifact = latest_flood_state["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["instanceId"] == artifact_id)
            .unwrap();
        assert_eq!(latest_flood_artifact["region"], "underwater");
        assert_eq!(
            full_session_fingerprint(&drought_parent),
            drought_parent_fingerprint
        );
        assert_eq!(
            full_session_fingerprint(&drought_branch),
            full_session_fingerprint(&session)
        );

        let (branch_descriptor, branch_receipt) = accept_where(&mut branch, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == destroy_card
                && descriptor["targetLocation"]["cell"] == target_cell
        });
        assert_eq!(branch_descriptor, destroy_descriptor);
        assert_eq!(branch_receipt, destroy_receipt);
        assert_eq!(full_session_fingerprint(&branch), crater_fingerprint);
        assert_eq!(
            full_session_fingerprint(&pre_parent),
            pre_parent_fingerprint
        );
        assert_exact_replay(&session);
        let post_checkpoint =
            create_game_checkpoint(&session).expect("flooded Rubble post-destroy checkpoint");
        let post_serialized =
            serialize_game_checkpoint(&post_checkpoint).expect("serialize post-destroy");
        let post_resume = resume_game_checkpoint(
            &parse_game_checkpoint(&post_serialized).expect("parse post-destroy"),
        )
        .expect("resume post-destroy");
        assert_eq!(
            post_resume.replay_value().unwrap(),
            session.replay_value().unwrap()
        );
        assert_eq!(
            post_resume.session_hash().unwrap(),
            session.session_hash().unwrap()
        );
        assert_eq!(
            post_resume.public_view(Seat::North).unwrap(),
            session.public_view(Seat::North).unwrap()
        );
        assert_eq!(
            post_resume.public_view(Seat::South).unwrap(),
            session.public_view(Seat::South).unwrap()
        );
        assert_eq!(
            post_resume.legal_actions().unwrap(),
            session.legal_actions().unwrap()
        );
        println!(
            "RBL-P6 flooded first={caster} seed={seed} manifest={} preRequests={} postRequests={} swimmer={swimmer_id} artifact={artifact_id}",
            session.manifest_json(),
            serde_json::to_string(&pre_checkpoint.requests).unwrap(),
            serde_json::to_string(&post_checkpoint.requests).unwrap(),
        );
    }
}

fn after_mulligans(north_spell: &str, south_spell: &str) -> Session {
    let mut session = Session::new(&manifest(north_spell, south_spell)).expect("terrain Aura");
    keep(&mut session);
    keep(&mut session);
    session
}

#[test]
fn rule_catalog_0266_flood_adds_water_and_keeps_other_affinities() {
    let mut session = after_mulligans("north-flood", "south-minion");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(north_affinity(&session), (1, 0));
    let flood_casts = session
        .legal_actions()
        .expect("Flood casts")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-aura" && action.descriptor["cardId"] == "north-flood"
        })
        .collect::<Vec<_>>();
    assert!(
        flood_casts.iter().all(|action| action.descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.len() == 4)),
        "Flood uses the default 2×2 footprint"
    );
    assert!(
        flood_casts
            .iter()
            .any(|action| cells_include(&action.descriptor, "C4")),
        "Flood can cover the played earth site"
    );
    let cast = cast_covering(&mut session, "north-flood", "C4");
    assert!(cells_include(&cast, "C4"));
    assert_eq!(north_affinity(&session), (1, 1));
    let after = state(&session);
    assert!(after["realm"].get("immobileAreas").is_none());
    assert_eq!(after["realm"]["auras"].as_array().expect("auras").len(), 1);
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(after["realm"]["auras"][0]["turnCounters"], 0);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    assert_eq!(north_affinity(&session), (1, 1));
    let persisted = state(&session);
    assert!(persisted["realm"].get("immobileAreas").is_none());
    assert_eq!(persisted["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(persisted["realm"]["auras"][0]["turnCounters"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0267_later_drought_wins_over_flood() {
    let mut session = after_mulligans("north-flood", "south-drought");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    cast_covering(&mut session, "north-flood", "C4");
    assert_eq!(north_affinity(&session), (1, 1));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    cast_covering(&mut session, "south-drought", "C4");
    assert_eq!(north_affinity(&session), (1, 0));
    let after = state(&session);
    assert!(after["realm"].get("immobileAreas").is_none());
    assert_eq!(after["realm"]["auras"].as_array().expect("auras").len(), 2);
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(after["realm"]["auras"][1]["cardId"], "south-drought");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0775_later_flood_wins_when_it_enters_after_drought() {
    let mut session = after_mulligans("north-drought", "south-flood");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    cast_covering(&mut session, "north-drought", "C4");
    assert_eq!(north_affinity(&session), (1, 0));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    cast_covering(&mut session, "south-flood", "C4");
    assert_eq!(north_affinity(&session), (1, 1));
    let after = state(&session);
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-drought");
    assert_eq!(after["realm"]["auras"][1]["cardId"], "south-flood");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0912_later_drought_wins_over_flood_and_re_enables_square_landbound() {
    let mut session = composition_opening();
    let bound_id = summon_square_landbound_at_b3(&mut session);
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], false);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "B3")
            && cells_include(descriptor, "C4")
    });
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], true);
    assert_eq!(north_affinity(&session), (4, 4));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "south-drought"
            && cells_include(descriptor, "B3")
            && cells_include(descriptor, "C4")
    });
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], false);
    assert_eq!(observed_unit(&session, &bound_id)["location"], "B3");
    assert_eq!(north_affinity(&session), (4, 0));
    let after = state(&session);
    assert_eq!(after["realm"]["auras"].as_array().expect("auras").len(), 2);
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(after["realm"]["auras"][1]["cardId"], "south-drought");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1169_drought_after_flood_restores_landbound_movement_on_site() {
    let mut session = landbound_movement_opening();
    keep(&mut session);
    keep(&mut session);
    play_site_at(&mut session, "C4");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-landbound"
            && descriptor["cell"] == "C4"
    });
    let bound_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Landbound identity")
        .to_owned();
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], false);
    end_then_draw(&mut session, "spellbook");
    play_site_at(&mut session, "C1");
    end_then_draw(&mut session, "atlas");
    play_site_at(&mut session, "C3");
    assert!(
        offers(&session, moves_bound_to(&bound_id, "C3")),
        "a ready Landbound minion can step from one earth site to the next"
    );
    cast_covering(&mut session, "north-flood", "C4");
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], true);
    assert_eq!(observed_unit(&session, &bound_id)["location"], "C4");
    assert!(
        !offers(&session, moves_bound(&bound_id)),
        "Flood turns the occupied earth site into Water, so Landbound movement is withheld"
    );
    end_then_draw(&mut session, "atlas");
    cast_covering(&mut session, "south-drought", "C4");
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], false);
    assert_eq!(observed_unit(&session, &bound_id)["location"], "C4");
    end_then_draw(&mut session, "spellbook");
    assert!(
        offers(&session, moves_bound_to(&bound_id, "C3")),
        "later Drought restores the occupied earth site, so Landbound movement returns"
    );
    accept_where(&mut session, moves_bound_to(&bound_id, "C3"));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let after = observed_unit(&session, &bound_id);
    assert_eq!(after["disabled"], false);
    assert_eq!(after["location"], "C3");
    let realm = state(&session);
    assert_eq!(realm["realm"]["auras"].as_array().expect("auras").len(), 2);
    assert_eq!(realm["realm"]["auras"][0]["cardId"], "north-flood");
    assert_eq!(realm["realm"]["auras"][1]["cardId"], "south-drought");
    assert_exact_replay(&session);
}
