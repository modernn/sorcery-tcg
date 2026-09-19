//! Direct proofs for Cave-In Artifact occupancy
//! (RULE-CATALOG-0677–0678, RULE-CATALOG-0740, RULE-CATALOG-2353–2358).
//!
//! These slices complement `cave_in_rules.rs` 0587–0588, which prove minion
//! burrows at a land site and that water-only sites are unoffered. Cave-In
//! also burrows a loose Artifact at the chosen land site, detaches an
//! Avatar-carried Artifact there while the Avatar stays on the surface, and
//! keeps a minion-carried Artifact attached when the bearer burrows.
//! 2353–2358 are the 0677 occupancy persistence, empty-repeat, enemy-arrival,
//! multi-artifact, far-artifact, and new-site proofs.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
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

fn burrower() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn cave_in() -> Value {
    json!({
        "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn cave_in_artifact_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-legacy-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-legacy-artifact-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-site": earth_site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-artifact"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn cave_in_minion_carrier_manifest(seed: u32) -> String {
    let mut south_spellbook = vec!["south-minion"; 8];
    south_spellbook.extend(vec!["south-artifact"; 4]);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-legacy-minion-carrier" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-legacy-minion-carrier-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-site": earth_site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-minion": burrower(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
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
    let mut session = Session::new(encoded).expect("valid Cave-In Artifact session");
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

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(cave_in_artifact_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-cave-in")
        })
        .expect("bounded seed with Cave-In in the opening hand")
}

fn minion_carrier_seed_with(start: u32) -> String {
    (start..start + 1024)
        .map(cave_in_minion_carrier_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-cave-in")
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .any(|card| card == "south-minion")
        })
        .expect("bounded seed with Cave-In and burrowing minion in opening hands")
}

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
        .expect("expected realm artifact")
}

fn stage_south_artifact(session: &mut Session, carried: bool) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && if carried {
                descriptor["bearer"]["kind"] == "avatar"
            } else {
                descriptor["bearer"].is_null() && descriptor["cell"] == "C1"
            }
    });
    let artifact_id = state(session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    artifact_id
}

fn cast_cave_in_at_c1(session: &mut Session) -> (Value, Receipt) {
    let land_site_id = state(session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("Land Site identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == land_site_id
    })
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
fn rule_catalog_0677_cave_in_burrows_a_loose_artifact_at_the_land_site() {
    let encoded = seed_with(677);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session, false);
    let before = realm_artifact(&state(&session), &artifact_id).clone();
    assert_eq!(before["location"], "C1");
    assert_eq!(before["region"], "surface");
    assert!(before.get("bearer").is_none());

    let (cast, receipt) = cast_cave_in_at_c1(&mut session);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-burrowed", "magic-resolved"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": artifact_id,
            "owner": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after_state = state(&session);
    let after = realm_artifact(&after_state, &artifact_id);
    assert_eq!(after["location"], before["location"]);
    assert_eq!(after["region"], "underground");
    assert!(after.get("bearer").is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0678_cave_in_detaches_an_avatar_carried_artifact_and_leaves_the_avatar() {
    let encoded = seed_with(678);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session, true);
    let before = realm_artifact(&state(&session), &artifact_id).clone();
    assert_eq!(before["bearer"]["kind"], "avatar");
    assert_eq!(before["bearer"]["seat"], "south");
    assert_eq!(
        state(&session)["players"]["south"]["avatar"]["region"],
        "surface"
    );

    let (cast, receipt) = cast_cave_in_at_c1(&mut session);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-burrowed", "magic-resolved"]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": artifact_id,
            "owner": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after = state(&session);
    let moved = realm_artifact(&after, &artifact_id);
    assert_eq!(moved["location"], "C1");
    assert_eq!(moved["owner"], "south");
    assert_eq!(moved["region"], "underground");
    assert!(moved.get("bearer").is_none());
    assert_eq!(after["players"]["south"]["avatar"]["region"], "surface");
    assert_exact_replay(&session);
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn stage_south_minion_carried_artifact(session: &mut Session) -> (String, String) {
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
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("burrowing minion identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"]["kind"] == "minion"
            && descriptor["bearer"]["instanceId"] == minion_id
    });
    let artifact_id = state(session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (minion_id, artifact_id)
}

#[test]
fn rule_catalog_0740_cave_in_keeps_minion_carried_artifact_attached_while_burrowing() {
    let encoded = minion_carrier_seed_with(740);
    let mut session = opening_main(&encoded);
    let (minion_id, artifact_id) = stage_south_minion_carried_artifact(&mut session);
    let before = state(&session);
    assert_eq!(realm_unit(&before, &minion_id)["region"], "surface");
    assert_eq!(
        realm_artifact(&before, &artifact_id)["bearer"]["instanceId"],
        minion_id
    );

    let (cast, receipt) = cast_cave_in_at_c1(&mut session);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-burrowed",
            "artifact-burrowed",
            "magic-resolved"
        ]
    );
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": minion_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after = state(&session);
    assert_eq!(realm_unit(&after, &minion_id)["region"], "underground");
    let carried = realm_artifact(&after, &artifact_id);
    assert_eq!(carried["bearer"]["kind"], "minion");
    assert_eq!(carried["bearer"]["instanceId"], minion_id);
    assert!(carried.get("location").is_none());
    assert_exact_replay(&session);
}

fn cave_in_artifact_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-artifact-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-artifact-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-site": earth_site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-artifact"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn supplemental_seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(677..677 + 2048)
        .map(cave_in_artifact_supplemental_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-cave-in")
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-artifact")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Cave-In and required South artifacts")
}

fn cave_in_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-cave-in")
                .count()
        })
        .unwrap_or_default()
}

fn artifact_id_at(snapshot: &Value, cell: &str) -> String {
    snapshot["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["location"] == cell)
        .expect("artifact at cell")["instanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned()
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

fn cave_in_cells(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("Cave-In actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-cave-in"
        })
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

fn cast_cave_in_at(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
}

fn play_south_site_at(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == cell
    });
}

fn cast_south_artifact_at(session: &mut Session, cell: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == cell
    });
    artifact_id_at(&state(session), cell)
}

fn setup_south_artifacts_at(session: &mut Session, cells: &[&str]) -> Vec<(String, String)> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    play_south_site_at(session, cells[0]);
    let mut placed = vec![(
        cells[0].to_string(),
        cast_south_artifact_at(session, cells[0]),
    )];
    for cell in cells.iter().skip(1) {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw"
                && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
        });
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw"
                && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
        });
        play_south_site_at(session, cell);
        placed.push(((*cell).to_string(), cast_south_artifact_at(session, cell)));
    }
    placed
}

fn setup_c1_loose_and_carried(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    play_south_site_at(session, "C1");
    let loose = cast_south_artifact_at(session, "C1");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"]["kind"] == "avatar"
    });
    let carried = state(session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] != loose)
        .expect("avatar-carried artifact")["instanceId"]
        .as_str()
        .expect("carried identity")
        .to_owned();
    (loose, carried)
}

fn is_artifact_underground(snapshot: &Value, instance_id: &str) -> bool {
    realm_artifact(snapshot, instance_id)["region"] == "underground"
}

fn try_second_cave_in_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    setup_south_artifacts_at(&mut session, &["C1"]);
    north_draws_spellbook(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C1"
    })?;
    pass_turn_to_north_spellbook(&mut session);
    if cave_in_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C2"
    })?;
    let artifact_id = artifact_id_at(&state(&session), "C2");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    cave_in_cells(&session)
        .contains(&"C2".to_owned())
        .then_some((session, artifact_id))
}

fn seed_for_second_cave_in_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(677..677 + 8192)
        .find_map(|seed| {
            let encoded = cave_in_artifact_supplemental_manifest(seed);
            if opening_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-artifact")
                .count()
                < 2
            {
                return None;
            }
            try_second_cave_in_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Cave-In enemy-arrival setup")
}

fn try_second_cave_in_new_site_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    setup_south_artifacts_at(&mut session, &["C1"]);
    north_draws_spellbook(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C1"
    })?;
    pass_turn_to_north_spellbook(&mut session);
    if cave_in_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let _ = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] != "C4"
    });
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let new_cell = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find_map(|action| {
            (action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "south-site"
                && action.descriptor["cell"] != "C1")
                .then(|| action.descriptor["cell"].as_str().map(ToOwned::to_owned))?
        })?;
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == new_cell
    });
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == new_cell
    })?;
    let artifact_id = artifact_id_at(&state(&session), &new_cell);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    cave_in_cells(&session)
        .contains(&new_cell)
        .then_some((session, artifact_id, new_cell))
}

fn seed_for_second_cave_in_new_site(start: u32) -> String {
    (start..start + 8192)
        .chain(677..677 + 8192)
        .find_map(|seed| {
            let encoded = cave_in_artifact_supplemental_manifest(seed);
            if opening_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-artifact")
                .count()
                < 2
            {
                return None;
            }
            try_second_cave_in_new_site_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Cave-In new-site setup")
}

#[test]
fn rule_catalog_2353_burrowed_artifact_stays_underground_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2353, 1);
    let mut session = opening_main(&encoded);
    let artifact_id = setup_south_artifacts_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    cast_cave_in_at(&mut session, "C1");
    assert!(is_artifact_underground(&state(&session), &artifact_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(is_artifact_underground(&state(&session), &artifact_id));
    assert_eq!(
        realm_artifact(&state(&session), &artifact_id)["location"],
        "C1"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2354_second_cave_in_without_surface_artifacts_is_a_paid_noop() {
    let encoded = supplemental_seed_with_start(2354, 1);
    let mut session = opening_main(&encoded);
    setup_south_artifacts_at(&mut session, &["C1"]);
    north_draws_spellbook(&mut session);
    cast_cave_in_at(&mut session, "C1");
    assert!(cave_in_spells_in_hand(&state(&session)) >= 1);
    let receipt = cast_cave_in_at(&mut session, "C1");
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "artifact-burrowed")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2355_second_cave_in_burrows_a_newly_arrived_artifact_after_enemy_site_placement() {
    let encoded = seed_for_second_cave_in_enemy_arrival(2355);
    let (mut session, artifact_id) = try_second_cave_in_enemy_arrival_prefix(&encoded)
        .expect("second Cave-In enemy-arrival prefix");
    let receipt = cast_cave_in_at(&mut session, "C2");
    assert!(event_types(&receipt).contains(&"artifact-burrowed"));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "artifact-burrowed" && event.payload["instanceId"] == artifact_id
    }));
    assert!(is_artifact_underground(&state(&session), &artifact_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2356_cave_in_burrows_every_artifact_sharing_the_target_land_site() {
    let encoded = supplemental_seed_with_start(2356, 2);
    let mut session = opening_main(&encoded);
    let (loose, carried) = setup_c1_loose_and_carried(&mut session);
    north_draws_spellbook(&mut session);
    let receipt = cast_cave_in_at(&mut session, "C1");
    let burrowed: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "artifact-burrowed")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("burrowed identity")
                .to_owned()
        })
        .collect();
    assert_eq!(burrowed.len(), 2);
    for artifact_id in [&loose, &carried] {
        assert!(burrowed.contains(artifact_id));
        assert!(is_artifact_underground(&state(&session), artifact_id));
        assert!(
            realm_artifact(&state(&session), artifact_id)
                .get("bearer")
                .is_none()
        );
    }
    assert_eq!(
        state(&session)["players"]["south"]["avatar"]["region"],
        "surface"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2357_cave_in_leaves_a_far_artifact_untouched() {
    let encoded = supplemental_seed_with_start(2357, 2);
    let mut session = opening_main(&encoded);
    let placed = setup_south_artifacts_at(&mut session, &["C1", "C2"]);
    let c1_id = placed[0].1.clone();
    let far_id = placed[1].1.clone();
    north_draws_spellbook(&mut session);
    cast_cave_in_at(&mut session, "C1");
    assert!(is_artifact_underground(&state(&session), &c1_id));
    let after = state(&session);
    let far = realm_artifact(&after, &far_id);
    assert_eq!(far["region"], "surface");
    assert_eq!(far["location"], "C2");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2358_second_cave_in_burrows_an_artifact_at_a_newly_placed_land_site() {
    let encoded = seed_for_second_cave_in_new_site(2358);
    let (mut session, artifact_id, new_cell) =
        try_second_cave_in_new_site_prefix(&encoded).expect("second Cave-In new-site prefix");
    let receipt = cast_cave_in_at(&mut session, &new_cell);
    assert!(event_types(&receipt).contains(&"artifact-burrowed"));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "artifact-burrowed" && event.payload["instanceId"] == artifact_id
    }));
    assert!(is_artifact_underground(&state(&session), &artifact_id));
    assert_exact_replay(&session);
}
