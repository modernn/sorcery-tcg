//! Direct proofs for burrow-target-minion-or-artifact Magic occupancy of
//! Artifacts (RULE-CATALOG-0679–0680, RULE-CATALOG-2363–2368).
//!
//! These slices complement `bury_magic_rules.rs` 0655–0656, which prove an
//! ordinary minion dies after a forceful burrow or stays put on Water, and
//! `cave_in_rules.rs` 0587–0588, which prove minion burrows at a land site.
//! Targeted Bury also burrows a chosen loose Artifact, and detaches an
//! Avatar-carried Artifact while the Avatar stays on the surface. Area
//! Cave-In Artifact occupancy is 0677–0678, not these targeted casts.
//! 2363–2368 are the 0679 supplemental persistence, empty-repeat,
//! enemy-arrival, multi-artifact, far-artifact, and new-placement proofs.

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

fn bury() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
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

fn bury_artifact_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "burrow-legacy-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-burrow-legacy-artifact-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-site": earth_site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bury"; 6],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid Bury Artifact session");
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

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(bury_artifact_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-bury")
        })
        .expect("bounded seed with Bury in the opening hand")
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

fn cast_bury_on_artifact(session: &mut Session, artifact_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["targetArtifactInstanceId"] == artifact_id
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
fn rule_catalog_0679_bury_burrows_a_loose_artifact() {
    let encoded = seed_with(679);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session, false);
    let before = realm_artifact(&state(&session), &artifact_id).clone();
    assert_eq!(before["location"], "C1");
    assert_eq!(before["region"], "surface");
    assert!(before.get("bearer").is_none());

    let (cast, receipt) = cast_bury_on_artifact(&mut session, &artifact_id);
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
fn rule_catalog_0680_bury_detaches_an_avatar_carried_artifact_and_leaves_the_avatar() {
    let encoded = seed_with(680);
    let mut session = opening_main(&encoded);
    let artifact_id = stage_south_artifact(&mut session, true);
    let before = realm_artifact(&state(&session), &artifact_id).clone();
    assert_eq!(before["bearer"]["kind"], "avatar");
    assert_eq!(before["bearer"]["seat"], "south");
    assert_eq!(
        state(&session)["players"]["south"]["avatar"]["region"],
        "surface"
    );

    let (cast, receipt) = cast_bury_on_artifact(&mut session, &artifact_id);
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

fn bury_artifact_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "bury-artifact-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-bury-artifact-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-site": earth_site(),
            "south-artifact": artifact(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bury"; 8],
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
        .chain(679..679 + 2048)
        .map(bury_artifact_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-bury")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-artifact")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Bury and required South artifacts")
}

fn bury_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-bury")
                .count()
        })
        .unwrap_or_default()
}

fn surface_artifact_id_at(snapshot: &Value, cell: &str) -> String {
    snapshot["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["location"] == cell && artifact["region"] == "surface")
        .expect("surface artifact at cell")["instanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned()
}

fn bury_artifact_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Bury Artifact actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-bury"
        })
        .filter_map(|action| {
            action.descriptor["targetArtifactInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .is_ok_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
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

fn cast_south_artifact_at(session: &mut Session, cell: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == cell
    });
    surface_artifact_id_at(&state(session), cell)
}

fn play_south_site_at(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == cell
    });
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

fn try_second_bury_enemy_artifact_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_south_artifacts_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["targetArtifactInstanceId"] == first_id
    })?;
    pass_turn_to_north_spellbook(&mut session);
    if bury_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C2"
    })?;
    let artifact_id = surface_artifact_id_at(&state(&session), "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    bury_artifact_targets(&session)
        .contains(&artifact_id)
        .then_some((session, artifact_id))
}

fn seed_for_second_bury_enemy_artifact(start: u32) -> String {
    (start..start + 8192)
        .chain(679..679 + 8192)
        .find_map(|seed| {
            let encoded = bury_artifact_supplemental_manifest(seed);
            if !opening_hand_spell_ids(&encoded, "north")
                .iter()
                .any(|card| card == "north-bury")
                || opening_hand_spell_ids(&encoded, "south")
                    .iter()
                    .filter(|card| *card == "south-artifact")
                    .count()
                    < 2
            {
                return None;
            }
            try_second_bury_enemy_artifact_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Bury Artifact enemy-arrival setup")
}

fn try_second_bury_new_artifact_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_south_artifacts_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["targetArtifactInstanceId"] == first_id
    })?;
    if realm_artifact(&state(&session), &first_id)["region"] != "underground" {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if bury_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C1"
    })?;
    let artifact_id = surface_artifact_id_at(&state(&session), "C1");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    bury_artifact_targets(&session)
        .contains(&artifact_id)
        .then_some((session, artifact_id))
}

fn seed_for_second_bury_new_artifact(start: u32) -> String {
    (start..start + 8192)
        .chain(679..679 + 8192)
        .find_map(|seed| {
            let encoded = bury_artifact_supplemental_manifest(seed);
            if !opening_hand_spell_ids(&encoded, "north")
                .iter()
                .any(|card| card == "north-bury")
                || opening_hand_spell_ids(&encoded, "south")
                    .iter()
                    .filter(|card| *card == "south-artifact")
                    .count()
                    < 2
            {
                return None;
            }
            try_second_bury_new_artifact_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Bury Artifact new-placement setup")
}

#[test]
fn rule_catalog_2363_buried_artifact_stays_underground_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2363, 1);
    let mut session = opening_main(&encoded);
    let artifact_id = setup_south_artifacts_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    let (_, receipt) = cast_bury_on_artifact(&mut session, &artifact_id);
    assert!(event_types(&receipt).contains(&"artifact-burrowed"));
    assert_eq!(
        realm_artifact(&state(&session), &artifact_id)["region"],
        "underground"
    );
    pass_turn_to_north_spellbook(&mut session);
    let after_state = state(&session);
    let after = realm_artifact(&after_state, &artifact_id);
    assert_eq!(after["location"], "C1");
    assert_eq!(after["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2364_second_bury_without_a_surface_artifact_stays_unoffered() {
    let encoded = supplemental_seed_with_start(2364, 1);
    let mut session = opening_main(&encoded);
    let artifact_id = setup_south_artifacts_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    cast_bury_on_artifact(&mut session, &artifact_id);
    assert!(bury_spells_in_hand(&state(&session)) >= 1);
    assert!(bury_artifact_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| descriptor["kind"]
        == "cast-magic"
        && descriptor["cardId"] == "north-bury"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2365_second_bury_burrows_a_newly_arrived_artifact_after_enemy_site_placement() {
    let encoded = seed_for_second_bury_enemy_artifact(2365);
    let (mut session, artifact_id) = try_second_bury_enemy_artifact_prefix(&encoded)
        .expect("second Bury Artifact enemy-arrival prefix");
    let (_, receipt) = cast_bury_on_artifact(&mut session, &artifact_id);
    assert!(event_types(&receipt).contains(&"artifact-burrowed"));
    assert_eq!(
        realm_artifact(&state(&session), &artifact_id)["region"],
        "underground"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2366_bury_offers_every_surface_artifact() {
    let encoded = supplemental_seed_with_start(2366, 2);
    let mut session = opening_main(&encoded);
    let artifacts = setup_south_artifacts_at(&mut session, &["C1", "C2"]);
    north_draws_spellbook(&mut session);
    let offered = bury_artifact_targets(&session);
    for (_, artifact_id) in &artifacts {
        assert!(offered.contains(artifact_id));
    }
    assert_eq!(offered.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2367_bury_leaves_a_far_artifact_untouched() {
    let encoded = supplemental_seed_with_start(2367, 2);
    let mut session = opening_main(&encoded);
    let placed = setup_south_artifacts_at(&mut session, &["C1", "C2"]);
    let far_id = placed[0].1.clone();
    let near_id = placed[1].1.clone();
    north_draws_spellbook(&mut session);
    let (_, receipt) = cast_bury_on_artifact(&mut session, &near_id);
    assert!(event_types(&receipt).contains(&"artifact-burrowed"));
    assert_eq!(
        realm_artifact(&state(&session), &near_id)["region"],
        "underground"
    );
    let after_state = state(&session);
    let far = realm_artifact(&after_state, &far_id);
    assert_eq!(far["location"], "C1");
    assert_eq!(far["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2368_second_bury_burrows_a_newly_placed_artifact() {
    let encoded = seed_for_second_bury_new_artifact(2368);
    let (mut session, artifact_id) = try_second_bury_new_artifact_prefix(&encoded)
        .expect("second Bury Artifact new-placement prefix");
    let (_, receipt) = cast_bury_on_artifact(&mut session, &artifact_id);
    assert!(event_types(&receipt).contains(&"artifact-burrowed"));
    assert_eq!(
        realm_artifact(&state(&session), &artifact_id)["region"],
        "underground"
    );
    assert_exact_replay(&session);
}
