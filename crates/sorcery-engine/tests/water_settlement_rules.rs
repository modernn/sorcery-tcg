//! Direct proofs for Water site terrain settlement: underwater Deathrite source
//! region preservation, Genesis resume after ordered Deathrites
//! (RULE-CATALOG-0713–0714), state-based region settlement when Water floods
//! underground Burrowing minions without Submerge (RULE-CATALOG-0901), the
//! burrow-to-submerge relayer that lets dual-region occupants survive the same
//! flood (RULE-CATALOG-0909), playing Water onto empty rubble
//! (RULE-CATALOG-1314), play-site water-on-rubble withheld during
//! trigger-order (RULE-CATALOG-1315), playing earth onto empty rubble
//! (RULE-CATALOG-1317), play-site earth-on-rubble withheld during
//! trigger-order (RULE-CATALOG-1318), play-site earth-on-flooded-rubble
//! withheld during trigger-order (RULE-CATALOG-1320), play-site
//! water-on-drought-rubble withheld during trigger-order
//! (RULE-CATALOG-1322), play-site water-on-flooded-rubble withheld during
//! trigger-order (RULE-CATALOG-1324), play-site earth-on-drought-rubble
//! withheld during trigger-order (RULE-CATALOG-1326), and play-site on
//! overlay-covered occupied sites withheld during trigger-order
//! (RULE-CATALOG-1340–1341, RULE-CATALOG-1346–1347,
//! RULE-CATALOG-1373–1376, RULE-CATALOG-1382, RULE-CATALOG-1391,
//! RULE-CATALOG-1399–1400, RULE-CATALOG-1403–1406, RULE-CATALOG-1411,
//! RULE-CATALOG-1414, RULE-CATALOG-1435, RULE-CATALOG-1445,
//! RULE-CATALOG-1476, RULE-CATALOG-1489–1490, RULE-CATALOG-1499–1500,
//! RULE-CATALOG-1509–1510, RULE-CATALOG-1515–1518, RULE-CATALOG-1520,
//! RULE-CATALOG-1529–1530).

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::session::{Session, StepResult};

#[test]
fn rule_catalog_0713_water_replacement_preserves_underwater_deathrite_source_region() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0713_water_replacement_preserves_underwater_deathrite_source_region();
}

#[test]
fn rule_catalog_0714_water_replacement_resumes_site_genesis_after_ordered_deathrites() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0714_water_replacement_resumes_site_genesis_after_ordered_deathrites();
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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["water"],
    })
}

fn burrowing_minion() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dual_region_minion() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "submerge": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn submerge_only_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    })
}

fn submerge_draw_site(ward: bool) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": ward,
    })
}

fn destroy_site() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
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

fn flood_aura() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drought_aura() -> Value {
    json!({
        "affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn water_cast_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "mustBeCastToWaterSite": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn flood_settlement_manifest(seed: u32, dual_region: bool) -> String {
    let (south_minion, south_key, fixture) = if dual_region {
        (
            dual_region_minion(),
            "south-dualer",
            "water-flood-dual-region-settlement",
        )
    } else {
        (
            burrowing_minion(),
            "south-burrower",
            "water-flood-burrower-settlement",
        )
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
    "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-earth": earth_site(),
            "south-avatar": avatar(),
            south_key: south_minion,
            "south-earth": earth_site(),
            "south-water": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-destroy"; 6],
            },
            "south": {
                "atlas": ["south-earth", "south-water", "south-earth", "south-earth", "south-earth", "south-earth"],
                "avatar": "south-avatar",
                "spellbook": vec![south_key; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_ward_stranding_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-ward-stranding" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-ward-stranding-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought_aura(),
            "north-water": water_site(),
            "south-avatar": avatar(),
            "south-minion": submerge_only_minion(),
            "south-earth": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-drought"; 6],
            },
            "south": {
                "atlas": vec!["south-earth"; 6],
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

fn mirrored_drought_ward_stranding_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "mirrored-drought-ward-stranding" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-mirrored-drought-ward-stranding-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth_site(),
            "north-minion": submerge_only_minion(),
            "south-avatar": avatar(),
            "south-drought": drought_aura(),
            "south-water": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 6],
            },
            "south": {
                "atlas": vec!["south-water"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-drought"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "south",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn drought_cohort_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-warded-drawsite-cohort" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-warded-drawsite-cohort-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought_aura(),
            "north-water": water_site(),
            "south-avatar": avatar(),
            "south-warded-drawsite": submerge_draw_site(true),
            "south-unwarded-drawsite": submerge_draw_site(false),
            "south-earth": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-drought"; 6],
            },
            "south": {
                "atlas": vec!["south-earth"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-warded-drawsite",
                    "south-warded-drawsite",
                    "south-warded-drawsite",
                    "south-unwarded-drawsite",
                    "south-unwarded-drawsite",
                    "south-unwarded-drawsite",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn mirrored_drought_cohort_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "mirrored-drought-warded-drawsite-cohort" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-mirrored-drought-warded-drawsite-cohort-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth_site(),
            "north-warded-drawsite": submerge_draw_site(true),
            "north-unwarded-drawsite": submerge_draw_site(false),
            "south-avatar": avatar(),
            "south-drought": drought_aura(),
            "south-water": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-warded-drawsite", "north-unwarded-drawsite", "north-warded-drawsite", "north-unwarded-drawsite", "north-warded-drawsite", "north-unwarded-drawsite"],
            },
            "south": {
                "atlas": vec!["south-water"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-drought"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "south",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn seed_with_drought_cohort(start: u32) -> String {
    (start..start + 256)
        .map(drought_cohort_manifest)
        .find(|encoded| {
            let mut session = Session::new(encoded).expect("cohort candidate");
            keep(&mut session);
            keep(&mut session);
            accept_where(&mut session, "cohort North Water", |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
            });
            accept_where(&mut session, "cohort North end-turn", |descriptor| {
                descriptor["kind"] == "end-turn"
            });
            accept_where(&mut session, "cohort South draw", |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            let snapshot = state(&session);
            let hand = snapshot["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            hand.iter()
                .any(|card| card["cardId"] == "south-warded-drawsite")
                && hand
                    .iter()
                    .any(|card| card["cardId"] == "south-unwarded-drawsite")
        })
        .expect("bounded seed with both DrawSite source variants")
}

fn seed_with_mirrored_drought_cohort(start: u32) -> String {
    (start..start + 256)
        .map(mirrored_drought_cohort_manifest)
        .find(|encoded| {
            let mut session = Session::new(encoded).expect("mirrored cohort candidate");
            keep(&mut session);
            keep(&mut session);
            accept_where(&mut session, "cohort South Water", |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
            accept_where(&mut session, "cohort South end-turn", |descriptor| {
                descriptor["kind"] == "end-turn"
            });
            accept_where(&mut session, "cohort North draw", |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            let snapshot = state(&session);
            let hand = snapshot["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            hand.iter()
                .any(|card| card["cardId"] == "north-warded-drawsite")
                && hand
                    .iter()
                    .any(|card| card["cardId"] == "north-unwarded-drawsite")
        })
        .expect("bounded mirrored seed with both DrawSite source variants")
}

fn accept_where(
    session: &mut Session,
    context: &str,
    predicate: impl Fn(&Value) -> bool,
) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!("{context}: expected engine-issued action");
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
    accept_where(session, "keep mulligan", |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid water-settlement session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, "north opening site", |descriptor| {
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
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

fn accept_with_full_checkpoint(
    session: &mut Session,
    context: &str,
    predicate: impl Fn(&Value) -> bool,
) -> (Value, Receipt) {
    let retained_parent = session.clone();
    let parent_fingerprint = full_session_fingerprint(&retained_parent);
    let before = full_session_fingerprint(session);
    let checkpoint = create_game_checkpoint(session).expect("pre-action checkpoint");
    let checkpoint_json = serialize_game_checkpoint(&checkpoint).expect("checkpoint bytes");
    let parsed = parse_game_checkpoint(&checkpoint_json).expect("parsed checkpoint");
    let mut restored = resume_game_checkpoint(&parsed).expect("restored pre-action branch");
    assert_eq!(full_session_fingerprint(&restored), before);

    let (descriptor, receipt) = accept_where(session, context, &predicate);
    let (restored_descriptor, restored_receipt) = accept_where(&mut restored, context, &predicate);
    assert_eq!(descriptor, restored_descriptor);
    assert_eq!(
        serde_json::to_value(&receipt).expect("receipt JSON"),
        serde_json::to_value(&restored_receipt).expect("restored receipt JSON")
    );
    assert_eq!(
        full_session_fingerprint(session),
        full_session_fingerprint(&restored)
    );
    assert_eq!(
        full_session_fingerprint(&retained_parent),
        parent_fingerprint
    );
    assert_exact_replay(session);
    (descriptor, receipt)
}

fn seed_with(start: u32, dual_region: bool) -> String {
    let minion_id = if dual_region {
        "south-dualer"
    } else {
        "south-burrower"
    };
    (start..start + 256)
        .map(|seed| flood_settlement_manifest(seed, dual_region))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("flood settlement candidate");
            let snapshot = state(&preview);
            let south_hand = snapshot["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("South opening hand");
            let south_atlas = snapshot["players"]["south"]["hand"]["atlas"]
                .as_array()
                .expect("South opening atlas");
            south_hand.iter().any(|card| card["cardId"] == minion_id)
                && south_atlas
                    .iter()
                    .any(|card| card["cardId"] == "south-water")
                && snapshot["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-destroy"))
        })
        .expect("bounded seed with Destroy, subsurface minion, and Water site")
}

fn south_draw_spellbook(session: &mut Session) {
    accept_where(session, "south end-turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south spellbook draw", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

#[test]
fn rule_catalog_0901_water_flood_kills_buried_burrower_without_submerge_in_same_receipt() {
    let encoded = seed_with(901, false);
    let mut session = opening_main(&encoded);
    south_draw_spellbook(&mut session);
    accept_where(&mut session, "south play C1", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, "summon burrower underground", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-burrower"
            && descriptor["cell"] == "C1"
            && descriptor["region"] == "underground"
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Burrower identity")
        .to_owned();
    south_draw_spellbook(&mut session);
    let c1_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South earth site identity")
        .to_owned();
    let destroy_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("Destroy in hand")["instanceId"]
        .as_str()
        .expect("Destroy identity")
        .to_owned();
    let (_, destroyed) = accept_where(&mut session, "north destroy C1", |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == destroy_id
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == c1_site_id
    });
    assert_eq!(
        event_types(&destroyed),
        [
            "magic-cast",
            "site-destroyed",
            "rubble-created",
            "magic-resolved",
        ]
    );
    assert_eq!(
        realm_unit(&state(&session), &target_id).expect("buried Burrower")["region"],
        "underground"
    );
    south_draw_spellbook(&mut session);
    if state(&session)["players"]["south"]["hand"]["atlas"]
        .as_array()
        .is_none_or(|hand| !hand.iter().any(|card| card["cardId"] == "south-water"))
    {
        accept_where(&mut session, "south draw-site", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    let (_, flooded) = accept_where(&mut session, "south play water on rubble", |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-water"
            && descriptor["cell"] == "C1"
    });
    assert_eq!(
        event_types(&flooded),
        ["rubble-replaced", "site-played", "minion-died"]
    );
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingDeathrites"].is_null());
    assert!(realm_unit(&after, &target_id).is_none());
    assert!(cemetery_has(&after, "south", &target_id));
    assert_exact_replay(&session);
}

#[test]
fn regional_ward_shared_flood_breaks_ward_before_stranding_death() {
    let mut manifest: Value = serde_json::from_str(&flood_settlement_manifest(901, false))
        .expect("flood settlement fixture");
    manifest.as_object_mut().unwrap().remove("manifestId");
    manifest["cards"]["south-burrower"]["ward"] = json!(true);
    let encoded = finish_manifest(manifest);
    let mut session = opening_main(&encoded);
    south_draw_spellbook(&mut session);
    accept_where(&mut session, "south play C1", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, "summon warded burrower", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-burrower"
            && descriptor["cell"] == "C1"
            && descriptor["region"] == "underground"
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Burrower identity")
        .to_owned();
    south_draw_spellbook(&mut session);
    let target_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South site identity")
        .to_owned();
    let destroy_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .unwrap()["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    accept_where(&mut session, "north destroy C1", |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == destroy_id
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == target_site_id
    });
    south_draw_spellbook(&mut session);
    if state(&session)["players"]["south"]["hand"]["atlas"]
        .as_array()
        .is_none_or(|hand| !hand.iter().any(|card| card["cardId"] == "south-water"))
    {
        accept_where(&mut session, "south draw-site", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    let (_, flooded) = accept_where(&mut session, "south play water on rubble", |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-water"
            && descriptor["cell"] == "C1"
    });
    assert_eq!(
        event_types(&flooded),
        [
            "rubble-replaced",
            "site-played",
            "ward-broken",
            "minion-died"
        ]
    );
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &target_id));
    assert_exact_replay(&session);
}

#[test]
fn drought_warded_submerge_stranding_breaks_ward_then_marks_once() {
    let encoded = drought_ward_stranding_manifest(163);
    assert_drought_single_stranding(&encoded, "north", true);
}

#[test]
fn drought_warded_submerge_stranding_mirrors_south_water_owner() {
    let encoded = mirrored_drought_ward_stranding_manifest(163);
    assert_drought_single_stranding(&encoded, "south", true);
}

#[test]
fn drought_unwarded_submerge_stranding_control_both_owners() {
    for water_owner in ["north", "south"] {
        let source = if water_owner == "north" {
            drought_ward_stranding_manifest(163)
        } else {
            mirrored_drought_ward_stranding_manifest(163)
        };
        let source_owner = if water_owner == "north" {
            "south"
        } else {
            "north"
        };
        let mut manifest: Value = serde_json::from_str(&source).expect("Drought fixture");
        manifest.as_object_mut().unwrap().remove("manifestId");
        manifest["cards"][format!("{source_owner}-minion")]["ward"] = json!(false);
        assert_drought_single_stranding(&finish_manifest(manifest), water_owner, false);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "keeps the mirrored Ward and stranding lifecycle proof together"
)]
fn assert_drought_single_stranding(encoded: &str, water_owner: &str, warded: bool) {
    let source_owner = if water_owner == "north" {
        "south"
    } else {
        "north"
    };
    let water_cell = if water_owner == "north" { "C4" } else { "C1" };
    let earth_cell = if water_owner == "north" { "C1" } else { "C4" };
    let water_card = format!("{water_owner}-water");
    let earth_card = format!("{source_owner}-earth");
    let source_card = format!("{source_owner}-minion");
    let drought_card = format!("{water_owner}-drought");
    let mut session = Session::new(encoded).expect("valid Drought-Ward fixture");
    for _ in 0..2 {
        accept_with_full_checkpoint(&mut session, "keep mulligan", |d| {
            d["kind"] == "mulligan"
                && d["atlasOrder"] == json!([])
                && d["spellbookOrder"] == json!([])
        });
    }
    accept_with_full_checkpoint(&mut session, "play Water home", |d| {
        d["kind"] == "play-site" && d["cardId"] == water_card && d["cell"] == water_cell
    });
    accept_with_full_checkpoint(&mut session, "Water owner end-turn", |d| {
        d["kind"] == "end-turn"
    });
    accept_with_full_checkpoint(&mut session, "source owner draw", |d| {
        d["kind"] == "draw" && d["zone"] == "spellbook"
    });
    accept_with_full_checkpoint(&mut session, "source owner Earth", |d| {
        d["kind"] == "play-site" && d["cardId"] == earth_card && d["cell"] == earth_cell
    });
    let (summoned, _) =
        accept_with_full_checkpoint(&mut session, "summon warded Submerge minion", |d| {
            d["kind"] == "summon-minion"
                && d["cardId"] == source_card
                && d["cell"] == water_cell
                && d["region"] == "underwater"
        });
    let target_id = summoned["cardInstanceId"].as_str().unwrap().to_owned();
    let before = state(&session);
    let before_unit = realm_unit(&before, &target_id).unwrap();
    assert_eq!(before_unit["warded"], warded);
    assert!(before_unit["deathMarked"].is_null());
    assert_eq!(before_unit["region"], "underwater");
    accept_with_full_checkpoint(&mut session, "source owner end-turn", |d| {
        d["kind"] == "end-turn"
    });
    accept_with_full_checkpoint(&mut session, "Water owner draw", |d| {
        d["kind"] == "draw" && d["zone"] == "spellbook"
    });
    let before_cast = state(&session);
    let caster = if water_owner == "north" {
        Seat::North
    } else {
        Seat::South
    };
    let before_view = session
        .public_view(caster)
        .expect("caster view before Drought");
    let source_view = &before_view["players"][source_owner];
    assert_eq!(before_view["players"][water_owner]["affinity"]["water"], 1);
    assert_eq!(before_view["players"][water_owner]["affinity"]["earth"], 0);
    assert_eq!(source_view["affinity"]["earth"], 1);
    assert_eq!(source_view["affinity"]["water"], 0);
    let caster_mana = before_cast["players"][water_owner]["mana"].clone();
    let retained_parent = session.clone();
    let parent_fingerprint = full_session_fingerprint(&retained_parent);
    let (descriptor, receipt) = accept_with_full_checkpoint(&mut session, "cast Drought", |d| {
        d["kind"] == "cast-aura"
            && d["cardId"] == drought_card
            && d["cells"]
                .as_array()
                .is_some_and(|cells| cells.iter().any(|cell| cell == water_cell))
    });
    assert_eq!(descriptor["cardId"], drought_card);
    let aura_id = descriptor["cardInstanceId"].as_str().unwrap();
    assert_eq!(
        event_types(&receipt),
        if warded {
            vec!["aura-conjured", "ward-broken", "minion-died"]
        } else {
            vec!["aura-conjured", "minion-died"]
        }
    );
    let aura_events = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "aura-conjured")
        .collect::<Vec<_>>();
    assert_eq!(aura_events.len(), 1);
    assert_eq!(aura_events[0].payload["instanceId"], aura_id);
    assert_eq!(aura_events[0].payload["cardId"], drought_card);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "ward-broken")
            .count(),
        usize::from(warded)
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "minion-died" && e.payload["instanceId"] == target_id)
            .count(),
        1
    );
    assert!(
        receipt
            .events
            .iter()
            .all(|e| e.event_type != "damage-dealt")
    );
    let after = state(&session);
    let after_view = session
        .public_view(caster)
        .expect("caster view after Drought");
    assert_eq!(after_view["players"][water_owner]["affinity"]["water"], 0);
    assert_eq!(after_view["players"][water_owner]["affinity"]["earth"], 0);
    assert_eq!(after_view["players"][source_owner]["affinity"]["earth"], 1);
    assert_eq!(after_view["players"][source_owner]["affinity"]["water"], 0);
    assert_eq!(after["players"][water_owner]["mana"], caster_mana);
    let realm_auras = after["realm"]["auras"].as_array().unwrap();
    assert_eq!(
        realm_auras
            .iter()
            .filter(|a| a["instanceId"] == aura_id)
            .count(),
        1
    );
    assert!(realm_auras.iter().any(|a| a["instanceId"] == aura_id));
    assert!(
        after["players"][water_owner]["hand"]["spellbook"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert!(
        after["players"][water_owner]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert!(realm_unit(&after, &target_id).is_none());
    assert_eq!(
        after["players"][source_owner]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["instanceId"] == target_id)
            .count(),
        1
    );
    assert!(
        after["realm"]["auras"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["cardId"] == drought_card)
    );
    assert_eq!(
        full_session_fingerprint(&retained_parent),
        parent_fingerprint
    );
    assert_exact_replay(&session);
}

#[test]
fn drought_regional_ward_preserves_healthy_dual_and_surface_controls_both_seats() {
    for (water_owner, control) in [
        ("north", "dual"),
        ("north", "surface"),
        ("south", "dual"),
        ("south", "surface"),
    ] {
        let source = if water_owner == "north" {
            drought_ward_stranding_manifest(171)
        } else {
            mirrored_drought_ward_stranding_manifest(171)
        };
        let encoded = drought_healthy_control_manifest(&source, water_owner, control);
        assert_drought_healthy_control(&encoded, water_owner, control);
    }
}

fn drought_healthy_control_manifest(source: &str, water_owner: &str, control: &str) -> String {
    let source_owner = if water_owner == "north" {
        "south"
    } else {
        "north"
    };
    let source_card = format!("{source_owner}-minion");
    let mut manifest: Value = serde_json::from_str(source).expect("source Drought manifest");
    manifest.as_object_mut().unwrap().remove("manifestId");
    let facts = match control {
        "dual" => {
            let mut facts = dual_region_minion();
            facts["ward"] = json!(true);
            facts["summonToAnySite"] = json!(true);
            facts
        }
        "surface" => json!({
            "attack": 1,
            "cardType": "minion",
            "defense": 1,
            "manaCost": 0,
            "summonToAnySite": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            "ward": true,
        }),
        _ => panic!("known Ward control"),
    };
    manifest["cards"][source_card] = facts;
    finish_manifest(manifest)
}

#[expect(
    clippy::too_many_lines,
    reason = "keeps both healthy Ward controls and mirrored regional settlement explicit"
)]
fn assert_drought_healthy_control(encoded: &str, water_owner: &str, control: &str) {
    let source_owner = if water_owner == "north" {
        "south"
    } else {
        "north"
    };
    let water_cell = if water_owner == "north" { "C4" } else { "C1" };
    let earth_cell = if water_owner == "north" { "C1" } else { "C4" };
    let water_card = format!("{water_owner}-water");
    let earth_card = format!("{source_owner}-earth");
    let source_card = format!("{source_owner}-minion");
    let drought_card = format!("{water_owner}-drought");
    let expected_region = if control == "dual" {
        "underwater"
    } else {
        "surface"
    };
    let settled_region = if control == "dual" {
        "underground"
    } else {
        "surface"
    };
    let mut session = Session::new(encoded).expect("valid Drought control fixture");
    for _ in 0..2 {
        accept_with_full_checkpoint(&mut session, "keep mulligan", |d| {
            d["kind"] == "mulligan"
                && d["atlasOrder"] == json!([])
                && d["spellbookOrder"] == json!([])
        });
    }
    accept_with_full_checkpoint(&mut session, "play Water home", |d| {
        d["kind"] == "play-site" && d["cardId"] == water_card && d["cell"] == water_cell
    });
    accept_with_full_checkpoint(&mut session, "Water owner end-turn", |d| {
        d["kind"] == "end-turn"
    });
    accept_with_full_checkpoint(&mut session, "source owner draw", |d| {
        d["kind"] == "draw" && d["zone"] == "spellbook"
    });
    accept_with_full_checkpoint(&mut session, "source owner Earth", |d| {
        d["kind"] == "play-site" && d["cardId"] == earth_card && d["cell"] == earth_cell
    });
    let summon_context = format!("summon Ward control ({water_owner}, {control})");
    let (summoned, _) = accept_with_full_checkpoint(&mut session, &summon_context, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == source_card && d["cell"] == water_cell
    });
    let target_id = summoned["cardInstanceId"].as_str().unwrap().to_owned();
    let before = state(&session);
    let unit_before = realm_unit(&before, &target_id).expect("live healthy Ward control");
    assert_eq!(unit_before["warded"], true);
    assert_eq!(unit_before["region"], expected_region);
    accept_with_full_checkpoint(&mut session, "source owner end-turn", |d| {
        d["kind"] == "end-turn"
    });
    accept_with_full_checkpoint(&mut session, "Water owner draw", |d| {
        d["kind"] == "draw" && d["zone"] == "spellbook"
    });
    let before_cast = state(&session);
    let caster = if water_owner == "north" {
        Seat::North
    } else {
        Seat::South
    };
    let before_view = session
        .public_view(caster)
        .expect("caster view before Drought");
    assert_eq!(before_view["players"][water_owner]["affinity"]["water"], 1);
    assert_eq!(before_view["players"][water_owner]["affinity"]["earth"], 0);
    assert_eq!(before_view["players"][source_owner]["affinity"]["earth"], 1);
    assert_eq!(before_view["players"][source_owner]["affinity"]["water"], 0);
    let caster_mana = before_cast["players"][water_owner]["mana"].clone();
    let (descriptor, receipt) =
        accept_with_full_checkpoint(&mut session, "cast Drought over control", |d| {
            d["kind"] == "cast-aura"
                && d["cardId"] == drought_card
                && d["cells"]
                    .as_array()
                    .is_some_and(|cells| cells.iter().any(|cell| cell == water_cell))
        });
    assert_eq!(descriptor["cardId"], drought_card);
    let aura_id = descriptor["cardInstanceId"].as_str().unwrap();
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "aura-conjured"
                && event.payload["instanceId"] == aura_id)
            .count(),
        1
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "aura-conjured"
            && event.payload["instanceId"] == aura_id
            && event.payload["cardId"] == drought_card
    }));
    let after_cast = state(&session);
    let after_view = session
        .public_view(caster)
        .expect("caster view after Drought");
    assert_eq!(after_view["players"][water_owner]["affinity"]["water"], 0);
    assert_eq!(after_view["players"][water_owner]["affinity"]["earth"], 0);
    assert_eq!(after_view["players"][source_owner]["affinity"]["earth"], 1);
    assert_eq!(after_view["players"][source_owner]["affinity"]["water"], 0);
    assert_eq!(after_cast["players"][water_owner]["mana"], caster_mana);
    assert_eq!(
        after_cast["realm"]["auras"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|aura| aura["instanceId"] == aura_id)
            .count(),
        1
    );
    assert!(
        after_cast["players"][water_owner]["hand"]["spellbook"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert!(
        after_cast["players"][water_owner]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "ward-broken")
            .count(),
        0
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "minion-died")
            .count(),
        0
    );
    assert!(
        receipt
            .events
            .iter()
            .all(|e| e.event_type != "damage-dealt")
    );
    let after = state(&session);
    let unit_after = realm_unit(&after, &target_id).expect("healthy Ward control survives");
    assert_eq!(unit_after["location"], water_cell);
    assert_eq!(unit_after["region"], settled_region);
    assert_eq!(unit_after["warded"], true);
    assert!(!cemetery_has(&after, source_owner, &target_id));
    assert_eq!(after["phase"], "main");
    assert_exact_replay(&session);
}

#[test]
fn drought_disabled_waterbound_loses_ward_then_dies_without_running_drawsite() {
    let mut manifest: Value = serde_json::from_str(&drought_ward_stranding_manifest(164))
        .expect("Drought Waterbound fixture");
    manifest.as_object_mut().unwrap().remove("manifestId");
    manifest["cards"]["south-minion"]["waterbound"] = json!(true);
    manifest["cards"]["south-minion"]["ward"] = json!(true);
    manifest["cards"]["south-minion"]["deathriteDrawSite"] = json!(true);
    let encoded = finish_manifest(manifest);
    let mut session = Session::new(&encoded).expect("valid Drought Waterbound session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, "north play Water C4", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, "North end turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut session, "South draw", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, "South Earth site", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, "summon warded Waterbound", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underwater"
    });
    let target_id = summoned["cardInstanceId"].as_str().unwrap().to_owned();
    accept_where(&mut session, "South end turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut session, "North draw", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let retained_parent = session.clone();
    let retained_parent_fingerprint = full_session_fingerprint(&retained_parent);
    let (_, receipt) = accept_where(&mut session, "cast Drought", |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && descriptor["cells"]
                .as_array()
                .is_some_and(|cells| cells.iter().any(|cell| cell == "C4"))
    });
    assert_eq!(
        event_types(&receipt),
        ["aura-conjured", "ward-lost", "minion-died"]
    );
    assert!(
        receipt.events.iter().all(|event| {
            event.event_type != "deathrite-draw-site" && event.event_type != "site-drawn"
        }),
        "disabled Waterbound must not run its populated DrawSite clause"
    );
    let after = state(&session);
    assert!(realm_unit(&after, &target_id).is_none());
    assert!(cemetery_has(&after, "south", &target_id));
    assert_eq!(
        full_session_fingerprint(&retained_parent),
        retained_parent_fingerprint
    );
    assert_exact_replay(&session);
}

#[test]
fn drought_enabled_drawsite_clause_runs_on_the_same_stranding_transition() {
    let mut manifest: Value = serde_json::from_str(&drought_ward_stranding_manifest(165))
        .expect("enabled Drought control fixture");
    manifest.as_object_mut().unwrap().remove("manifestId");
    manifest["cards"]["south-minion"]["ward"] = json!(true);
    manifest["cards"]["south-minion"]["deathriteDrawSite"] = json!(true);
    let encoded = finish_manifest(manifest);
    let mut session = Session::new(&encoded).expect("valid enabled DrawSite control");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, "North Water site", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, "North end turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut session, "South draw", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, "South Earth site", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, "summon enabled DrawSite", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underwater"
    });
    let target_id = summoned["cardInstanceId"].as_str().unwrap().to_owned();
    accept_where(&mut session, "South end turn", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut session, "North draw", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let retained_parent = session.clone();
    let retained_parent_fingerprint = full_session_fingerprint(&retained_parent);
    let (_, receipt) = accept_where(&mut session, "cast Drought", |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && descriptor["cells"]
                .as_array()
                .is_some_and(|cells| cells.iter().any(|cell| cell == "C4"))
    });
    assert_eq!(
        event_types(&receipt),
        ["aura-conjured", "ward-broken", "site-drawn", "minion-died"]
    );
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert_eq!(
        full_session_fingerprint(&retained_parent),
        retained_parent_fingerprint
    );
    assert_exact_replay(&session);
}

#[test]
fn drought_regional_ward_cohort_pauses_after_all_prevention() {
    let encoded = seed_with_drought_cohort(163);
    assert_drought_regional_ward_cohort(&encoded, "north");
}

#[test]
fn drought_regional_ward_cohort_mirrors_south_owner_and_source_seat() {
    let encoded = seed_with_mirrored_drought_cohort(163);
    assert_drought_regional_ward_cohort(&encoded, "south");
}

#[expect(
    clippy::too_many_lines,
    reason = "keeps both regional Ward trigger orders, checkpoints, and parent parity explicit"
)]
fn assert_drought_regional_ward_cohort(encoded: &str, water_owner: &str) {
    let source_owner = if water_owner == "north" {
        "south"
    } else {
        "north"
    };
    let water_cell = if water_owner == "north" { "C4" } else { "C1" };
    let earth_cell = if water_owner == "north" { "C1" } else { "C4" };
    let water_card = format!("{water_owner}-water");
    let earth_card = format!("{source_owner}-earth");
    let drought_card = format!("{water_owner}-drought");
    let warded_card = format!("{source_owner}-warded-drawsite");
    let unwarded_card = format!("{source_owner}-unwarded-drawsite");
    let mut session = Session::new(encoded).expect("valid regional Ward cohort");
    accept_with_full_checkpoint(&mut session, "North keep", |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
    accept_with_full_checkpoint(&mut session, "South keep", |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
    accept_with_full_checkpoint(&mut session, "play Water home site", |d| {
        d["kind"] == "play-site" && d["cardId"] == water_card && d["cell"] == water_cell
    });
    accept_with_full_checkpoint(&mut session, "Water owner end-turn", |d| {
        d["kind"] == "end-turn"
    });
    accept_with_full_checkpoint(&mut session, "source owner spellbook draw", |d| {
        d["kind"] == "draw" && d["zone"] == "spellbook"
    });
    accept_with_full_checkpoint(&mut session, "source owner Earth site", |d| {
        d["kind"] == "play-site" && d["cardId"] == earth_card && d["cell"] == earth_cell
    });
    let (warded, _) = accept_with_full_checkpoint(&mut session, "summon warded DrawSite", |d| {
        d["kind"] == "summon-minion"
            && d["cardId"] == warded_card
            && d["cell"] == water_cell
            && d["region"] == "underwater"
    });
    let (unwarded, _) =
        accept_with_full_checkpoint(&mut session, "summon unwarded DrawSite", |d| {
            d["kind"] == "summon-minion"
                && d["cardId"] == unwarded_card
                && d["cell"] == water_cell
                && d["region"] == "underwater"
        });
    let warded_id = warded["cardInstanceId"].as_str().unwrap().to_owned();
    let unwarded_id = unwarded["cardInstanceId"].as_str().unwrap().to_owned();
    accept_with_full_checkpoint(&mut session, "source owner end-turn", |d| {
        d["kind"] == "end-turn"
    });
    accept_with_full_checkpoint(&mut session, "Water owner spellbook draw", |d| {
        d["kind"] == "draw" && d["zone"] == "spellbook"
    });
    let before_cast = state(&session);
    let caster = if water_owner == "north" {
        Seat::North
    } else {
        Seat::South
    };
    let before_view = session
        .public_view(caster)
        .expect("caster view before Drought");
    assert_eq!(before_view["players"][water_owner]["affinity"]["water"], 1);
    assert_eq!(before_view["players"][water_owner]["affinity"]["earth"], 0);
    assert_eq!(before_view["players"][source_owner]["affinity"]["earth"], 1);
    assert_eq!(before_view["players"][source_owner]["affinity"]["water"], 0);
    let caster_mana = before_cast["players"][water_owner]["mana"].clone();
    let (descriptor, receipt) =
        accept_with_full_checkpoint(&mut session, "cast Drought over Water", |d| {
            d["kind"] == "cast-aura"
                && d["cardId"] == drought_card
                && d["cells"]
                    .as_array()
                    .is_some_and(|cells| cells.iter().any(|cell| cell == water_cell))
        });
    assert_eq!(descriptor["cardId"], drought_card);
    let aura_id = descriptor["cardInstanceId"].as_str().unwrap();
    let aura_events = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "aura-conjured")
        .collect::<Vec<_>>();
    assert_eq!(aura_events.len(), 1);
    assert_eq!(aura_events[0].payload["instanceId"], aura_id);
    assert_eq!(aura_events[0].payload["cardId"], drought_card);
    let after_cast = state(&session);
    let after_view = session
        .public_view(caster)
        .expect("caster view after Drought");
    assert_eq!(after_view["players"][water_owner]["affinity"]["water"], 0);
    assert_eq!(after_view["players"][water_owner]["affinity"]["earth"], 0);
    assert_eq!(after_view["players"][source_owner]["affinity"]["earth"], 1);
    assert_eq!(after_view["players"][source_owner]["affinity"]["water"], 0);
    assert_eq!(after_cast["players"][water_owner]["mana"], caster_mana);
    assert_eq!(
        after_cast["realm"]["auras"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|aura| aura["instanceId"] == aura_id)
            .count(),
        1
    );
    assert!(
        after_cast["players"][water_owner]["hand"]["spellbook"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert!(
        after_cast["players"][water_owner]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "ward-broken")
            .count(),
        1
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "minion-died")
            .count(),
        0
    );
    let pause = state(&session);
    assert_eq!(pause["phase"], "trigger-order");
    assert!(pause["pendingDeathrites"].is_object());
    assert!(pause["pendingDeathrites"]["continuation"].is_null());
    for (id, should_be_warded) in [(&warded_id, false), (&unwarded_id, false)] {
        let unit = realm_unit(&pause, id).expect("marked casualty remains live at pause");
        assert_eq!(unit["deathMarked"], true);
        assert_eq!(unit["warded"], should_be_warded);
        assert!(!cemetery_has(&pause, source_owner, id));
    }
    let pause_auras = pause["realm"]["auras"].as_array().unwrap();
    assert_eq!(pause_auras.len(), 1);
    assert_eq!(pause_auras[0]["instanceId"], aura_id);
    assert_eq!(pause_auras[0]["cardId"], drought_card);
    assert!(
        pause["players"][water_owner]["hand"]["spellbook"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert!(
        pause["players"][water_owner]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    let pause_fingerprint = full_session_fingerprint(&session);
    let checkpoint = create_game_checkpoint(&session).expect("pause checkpoint");
    let checkpoint_bytes = serialize_game_checkpoint(&checkpoint).expect("pause checkpoint bytes");
    let parsed = parse_game_checkpoint(&checkpoint_bytes).expect("parsed pause checkpoint");
    let restored_pause = resume_game_checkpoint(&parsed).expect("restored pause");
    assert_eq!(full_session_fingerprint(&restored_pause), pause_fingerprint);
    let frontier = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .map(|action| action.descriptor)
        .collect::<Vec<_>>();
    assert_eq!(frontier.len(), 2);
    assert!(
        frontier
            .iter()
            .all(|descriptor| descriptor["kind"] == "order-triggers")
    );
    let order_ids = frontier
        .iter()
        .filter(|descriptor| descriptor["kind"] == "order-triggers")
        .map(|a| a["sourceInstanceId"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(order_ids.len(), 2);
    assert_ne!(order_ids[0], order_ids[1]);
    assert!(order_ids.contains(&warded_id) && order_ids.contains(&unwarded_id));
    let pause_parent = session.clone();
    for first_id in order_ids.clone() {
        let second_id = order_ids
            .iter()
            .find(|id| *id != &first_id)
            .expect("other DrawSite source")
            .clone();
        let mut branch = resume_game_checkpoint(&parsed).expect("independent order branch");
        let branch_parent = branch.clone();
        let branch_parent_fingerprint = full_session_fingerprint(&branch_parent);
        let (selected, draw) =
            accept_with_full_checkpoint(&mut branch, "choose first DrawSite source", |d| {
                d["kind"] == "order-triggers" && d["sourceInstanceId"] == first_id
            });
        assert_eq!(selected["sourceInstanceId"], first_id);
        let drawn = draw
            .events
            .iter()
            .filter(|e| e.event_type == "site-drawn")
            .collect::<Vec<_>>();
        assert_eq!(drawn.len(), 2, "the final pending source auto-commits");
        let draw_order = drawn
            .iter()
            .map(|event| {
                event.payload["sourceInstanceId"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(draw_order, [first_id.clone(), second_id]);
        assert!(
            draw.events
                .iter()
                .all(|event| event.event_type != "aura-conjured")
        );
        assert!(
            drawn
                .iter()
                .all(|event| event.payload["seat"] == source_owner)
        );
        assert!(draw.events.iter().all(|e| e.event_type != "spell-drawn"));
        let complete = state(&branch);
        assert_eq!(complete["phase"], "main");
        assert!(complete["pendingDeathrites"].is_null());
        let complete_auras = complete["realm"]["auras"].as_array().unwrap();
        assert_eq!(complete_auras.len(), 1);
        assert_eq!(complete_auras[0]["instanceId"], aura_id);
        assert_eq!(complete_auras[0]["cardId"], drought_card);
        assert!(
            complete["players"][water_owner]["hand"]["spellbook"]
                .as_array()
                .unwrap()
                .iter()
                .all(|card| card["instanceId"] != aura_id)
        );
        assert!(
            complete["players"][water_owner]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .all(|card| card["instanceId"] != aura_id)
        );
        assert!(
            !branch
                .legal_actions()
                .unwrap()
                .iter()
                .any(|action| { action.descriptor["kind"] == "order-triggers" })
        );
        assert!(realm_unit(&complete, &warded_id).is_none());
        assert!(realm_unit(&complete, &unwarded_id).is_none());
        for id in [&warded_id, &unwarded_id] {
            assert_eq!(
                draw.events
                    .iter()
                    .filter(|event| event.event_type == "minion-died"
                        && event.payload["instanceId"] == *id)
                    .count(),
                1
            );
            assert_eq!(
                complete["players"][source_owner]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| c["instanceId"] == *id)
                    .count(),
                1
            );
        }
        assert_eq!(
            full_session_fingerprint(&branch_parent),
            branch_parent_fingerprint
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
    assert_eq!(full_session_fingerprint(&pause_parent), pause_fingerprint);
    assert_eq!(full_session_fingerprint(&restored_pause), pause_fingerprint);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0909_water_flood_relayers_dual_region_minion_underwater_without_settlement_death() {
    let encoded = seed_with(909, true);
    let mut session = opening_main(&encoded);
    south_draw_spellbook(&mut session);
    accept_where(&mut session, "south play C1", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, "summon dualer underground", |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-dualer"
            && descriptor["cell"] == "C1"
            && descriptor["region"] == "underground"
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("dual-region identity")
        .to_owned();
    south_draw_spellbook(&mut session);
    let c1_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South earth site identity")
        .to_owned();
    let destroy_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("Destroy in hand")["instanceId"]
        .as_str()
        .expect("Destroy identity")
        .to_owned();
    accept_where(&mut session, "north destroy C1", |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == destroy_id
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == c1_site_id
    });
    assert_eq!(
        realm_unit(&state(&session), &target_id).expect("buried dual-region minion")["region"],
        "underground"
    );
    south_draw_spellbook(&mut session);
    if state(&session)["players"]["south"]["hand"]["atlas"]
        .as_array()
        .is_none_or(|hand| !hand.iter().any(|card| card["cardId"] == "south-water"))
    {
        accept_where(&mut session, "south draw-site", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    let (_, flooded) = accept_where(&mut session, "south play water on rubble", |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-water"
            && descriptor["cell"] == "C1"
    });
    assert_eq!(
        event_types(&flooded),
        ["rubble-replaced", "site-played"],
        "dual-region minion must relayer underwater and survive state-based settlement"
    );
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingDeathrites"].is_null());
    let occupant = realm_unit(&after, &target_id).expect("surviving dual-region minion");
    assert_eq!(occupant["location"], "C1");
    assert_eq!(occupant["region"], "underwater");
    assert!(!cemetery_has(&after, "south", &target_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1314_playing_water_onto_empty_rubble_creates_water_site() {
    let encoded = seed_with(1314, false);
    let mut session = opening_main(&encoded);
    south_draw_spellbook(&mut session);
    accept_where(&mut session, "south play C1", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    south_draw_spellbook(&mut session);
    let c1_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South earth site identity")
        .to_owned();
    let destroy_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("Destroy in hand")["instanceId"]
        .as_str()
        .expect("Destroy identity")
        .to_owned();
    let (_, destroyed) = accept_where(&mut session, "north destroy C1", |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == destroy_id
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == c1_site_id
    });
    assert_eq!(
        event_types(&destroyed),
        [
            "magic-cast",
            "site-destroyed",
            "rubble-created",
            "magic-resolved",
        ]
    );
    assert_eq!(state(&session)["realm"]["sites"]["C1"]["rubble"], true);
    south_draw_spellbook(&mut session);
    if state(&session)["players"]["south"]["hand"]["atlas"]
        .as_array()
        .is_none_or(|hand| !hand.iter().any(|card| card["cardId"] == "south-water"))
    {
        accept_where(&mut session, "south draw-site", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    let (_, flooded) = accept_where(
        &mut session,
        "south play water on empty rubble",
        |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "south-water"
                && descriptor["cell"] == "C1"
        },
    );
    assert_eq!(event_types(&flooded), ["rubble-replaced", "site-played"]);
    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"]["cardId"], "south-water");
    assert_eq!(after["realm"]["sites"]["C1"]["rubble"], Value::Null);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingDeathrites"].is_null());
    assert_exact_replay(&session);
}

fn seed_with_earth_site(start: u32) -> String {
    (start..start + 256)
        .map(|seed| flood_settlement_manifest(seed, false))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("earth rubble candidate");
            let snapshot = state(&preview);
            snapshot["players"]["south"]["hand"]["atlas"]
                .as_array()
                .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-earth"))
                && snapshot["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-destroy"))
        })
        .expect("bounded seed with Destroy and Earth site")
}

#[test]
fn rule_catalog_1317_playing_earth_onto_empty_rubble_creates_earth_site() {
    let encoded = seed_with_earth_site(1317);
    let mut session = opening_main(&encoded);
    south_draw_spellbook(&mut session);
    accept_where(&mut session, "south play C1", |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    south_draw_spellbook(&mut session);
    let c1_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South earth site identity")
        .to_owned();
    let destroy_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("Destroy in hand")["instanceId"]
        .as_str()
        .expect("Destroy identity")
        .to_owned();
    accept_where(&mut session, "north destroy C1", |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == destroy_id
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == c1_site_id
    });
    assert_eq!(state(&session)["realm"]["sites"]["C1"]["rubble"], true);
    south_draw_spellbook(&mut session);
    let (_, replaced) = accept_where(
        &mut session,
        "south play earth on empty rubble",
        |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "south-earth"
                && descriptor["cell"] == "C1"
        },
    );
    assert_eq!(event_types(&replaced), ["rubble-replaced", "site-played"]);
    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"]["cardId"], "south-earth");
    assert_eq!(after["realm"]["sites"]["C1"]["rubble"], Value::Null);
    assert_exact_replay(&session);
}

fn earth_on_rubble_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "earth-rubble-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-earth-rubble-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-destroy",
                    "north-rain",
                    "north-rain",
                    "north-destroy",
                    "north-rain",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                ],
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

fn earth_on_rubble_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(earth_on_rubble_deathrite_manifest)
        .find(|candidate| try_pending_deathrite_with_water_on_rubble_legal(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with legal Earth play on rubble")
}

#[test]
fn rule_catalog_1318_play_earth_on_rubble_withheld_during_pending_deathrite_order() {
    let encoded = earth_on_rubble_deathrite_seed_with(1318);
    let mut setup = try_pending_deathrite_with_water_on_rubble_legal(&encoded)
        .expect("complete earth-on-rubble Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "south-earth"
                && action.descriptor["cell"] == "C1"
        })
    {
        accept_where(session, "south draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-earth"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
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

fn water_on_rubble_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "water-rubble-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-water-rubble-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-water": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-destroy",
                    "north-rain",
                    "north-rain",
                    "north-destroy",
                    "north-rain",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-water",
                    "south-earth",
                    "south-site",
                    "south-site",
                    "south-site",
                ],
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

fn try_accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> bool {
    try_accept_where_pair(session, predicate).is_some()
}

fn try_accept_where_pair(
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

struct PendingDeathriteWaterOnRubbleSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn south_has_water_in_atlas(snapshot: &Value) -> bool {
    snapshot["players"]["south"]["hand"]["atlas"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-water"))
}

fn north_has_earth_in_atlas(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["atlas"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-earth"))
}

fn north_has_water_in_atlas(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["atlas"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water"))
}

fn north_has_destroy_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            hand.iter().any(|card| card["cardId"] == "north-destroy")
                && hand.iter().any(|card| card["cardId"] == "north-rain")
        })
}

fn north_has_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
}

fn try_pending_deathrite_with_water_on_rubble_legal(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
    {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    let snapshot = state(&session);
    if !north_has_destroy_and_rain(&snapshot) {
        return None;
    }
    let c1_site_id = snapshot["realm"]["sites"]["C1"]["instanceId"].as_str()?;
    let destroy_id = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == "north-destroy")?
        .get("instanceId")?
        .as_str()?;
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == destroy_id
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == c1_site_id
    }) || state(&session)["realm"]["sites"]["C1"]["rubble"] != true
    {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn water_on_rubble_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(water_on_rubble_deathrite_manifest)
        .find(|candidate| try_pending_deathrite_with_water_on_rubble_legal(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with legal site play on rubble")
}

#[test]
fn rule_catalog_1315_play_water_on_rubble_withheld_during_pending_deathrite_order() {
    let encoded = water_on_rubble_deathrite_seed_with(1315);
    let mut setup = try_pending_deathrite_with_water_on_rubble_legal(&encoded)
        .expect("complete water-on-rubble Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());

    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !south_has_water_in_atlas(&state(session)) {
        accept_where(session, "south draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(south_has_water_in_atlas(&state(session)));
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-water"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

fn overlay_covers_c1(descriptor: &Value, card_id: &str) -> bool {
    descriptor["kind"] == "cast-aura"
        && descriptor["cardId"] == card_id
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().any(|value| value == "C1"))
}

fn try_pending_deathrite_with_overlay_rubble_legal(
    encoded: &str,
    overlay_card: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
    {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    let snapshot = state(&session);
    if !north_has_destroy_and_rain(&snapshot) {
        return None;
    }
    let c1_site_id = snapshot["realm"]["sites"]["C1"]["instanceId"].as_str()?;
    let destroy_id = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == "north-destroy")?
        .get("instanceId")?
        .as_str()?;
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == destroy_id
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == c1_site_id
    }) || state(&session)["realm"]["sites"]["C1"]["rubble"] != true
    {
        return None;
    }
    if !try_accept_where(&mut session, |descriptor| {
        overlay_covers_c1(descriptor, overlay_card)
    }) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_overlay_occupied_legal(
    encoded: &str,
    overlay_card: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
    {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if !try_accept_where(&mut session, |descriptor| {
        overlay_covers_c1(descriptor, overlay_card)
    }) || state(&session)["realm"]["sites"]["C1"]["rubble"] == true
    {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn flooded_rubble_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-rubble-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-rubble-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-destroy",
                    "north-flood",
                    "north-rain",
                    "north-destroy",
                    "north-flood",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                ],
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

fn drought_rubble_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-rubble-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-rubble-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-drought": drought_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-water": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-destroy",
                    "north-drought",
                    "north-rain",
                    "north-destroy",
                    "north-drought",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-water",
                    "south-water",
                    "south-site",
                    "south-site",
                    "south-site",
                ],
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

fn flooded_rubble_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flooded_rubble_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_rubble_legal(candidate, "north-flood").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with legal Earth play on flooded rubble",
        )
}

fn flooded_water_rubble_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-water-rubble-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-water-rubble-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-water": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-destroy",
                    "north-flood",
                    "north-rain",
                    "north-destroy",
                    "north-flood",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-water",
                    "south-water",
                    "south-water",
                    "south-water",
                    "south-water",
                ],
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

fn drought_earth_rubble_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-earth-rubble-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-earth-rubble-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-drought": drought_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-destroy",
                    "north-drought",
                    "north-rain",
                    "north-destroy",
                    "north-drought",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                ],
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

fn flooded_water_rubble_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flooded_water_rubble_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_rubble_legal(candidate, "north-flood").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with legal Water play on flooded rubble",
        )
}

fn drought_earth_rubble_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_earth_rubble_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_rubble_legal(candidate, "north-drought").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with legal Earth play on drought rubble",
        )
}

fn flooded_occupied_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-occupied-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-occupied-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-destroy",
                    "north-flood",
                    "north-rain",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                ],
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

fn drought_occupied_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-occupied-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-occupied-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-drought": drought_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-water": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-destroy",
                    "north-drought",
                    "north-rain",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-water",
                    "south-water",
                    "south-water",
                    "south-water",
                    "south-water",
                ],
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

fn flooded_occupied_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flooded_occupied_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_legal(candidate, "north-flood").is_some()
        })
        .expect("bounded seed that reaches pending Deathrites with Flood on an occupied Earth site")
}

fn drought_occupied_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_occupied_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_legal(candidate, "north-drought").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Drought on an occupied Water site",
        )
}

fn flooded_occupied_water_cast_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-occupied-water-cast-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-occupied-water-cast-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "north-water-cast": water_cast_minion(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-water-cast",
                    "north-destroy",
                    "north-flood",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                ],
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

fn try_pending_deathrite_with_overlay_occupied_water_cast_summon(
    encoded: &str,
    overlay_card: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let setup = try_pending_deathrite_with_overlay_occupied_legal(encoded, overlay_card)?;
    if !state(&setup.session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water-cast"))
    {
        return None;
    }
    Some(setup)
}

fn flooded_occupied_water_cast_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flooded_occupied_water_cast_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_water_cast_summon(
                candidate,
                "north-flood",
            )
            .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a water-site cast minion on flooded occupied Earth",
        )
}

fn drought_water_c3_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-water-c3-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-water-c3-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought_aura(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "north-water-cast": water_cast_minion(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-water",
                    "north-water",
                    "north-water",
                    "north-water",
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-water-cast",
                    "north-drought",
                    "north-rain",
                    "north-water-cast",
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

fn overlay_covers_c3(descriptor: &Value, card_id: &str) -> bool {
    descriptor["kind"] == "cast-aura"
        && descriptor["cardId"] == card_id
        && descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.iter().any(|value| value == "C3"))
}

fn pass_turn_draw_spellbook(session: &mut Session) -> bool {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")
        && try_accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
}

fn try_pending_deathrite_with_drought_occupied_water_cast_summon(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-drought")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water-cast"))
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn drought_occupied_water_cast_deathrite_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_water_c3_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_water_cast_summon(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a water-site cast minion on drought occupied Water",
        )
}

fn flooded_water_c3_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-water-c3-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-water-c3-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "north-water-cast": water_cast_minion(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-water-cast",
                    "north-flood",
                    "north-rain",
                    "north-water-cast",
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

fn try_pending_deathrite_with_flooded_occupied_water_cast_summon(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-flood")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water-cast"))
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn flooded_water_c3_deathrite_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_water_c3_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_water_cast_summon(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a water-site cast minion on flooded occupied Water",
        )
}

fn flooded_water_occupied_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-water-occupied-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-water-occupied-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-water": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-destroy",
                    "north-flood",
                    "north-rain",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-water",
                    "south-water",
                    "south-water",
                    "south-water",
                    "south-water",
                ],
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

fn drought_earth_occupied_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-earth-occupied-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-earth-occupied-deathrite-withheld-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-drought": drought_aura(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-earth": earth_site(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-destroy",
                    "north-drought",
                    "north-rain",
                    "north-destroy",
                ],
            },
            "south": {
                "atlas": [
                    "south-site",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                    "south-earth",
                ],
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

fn flooded_water_occupied_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(flooded_water_occupied_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_legal(candidate, "north-flood").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Water play on flooded occupied site",
        )
}

fn drought_earth_occupied_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_earth_occupied_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_occupied_legal(candidate, "north-drought").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Earth play on drought occupied site",
        )
}

fn drought_rubble_deathrite_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(drought_rubble_deathrite_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_overlay_rubble_legal(candidate, "north-drought").is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with legal Water play on drought rubble",
        )
}

#[test]
fn rule_catalog_1320_play_earth_on_flooded_rubble_withheld_during_pending_deathrite_order() {
    let encoded = flooded_rubble_deathrite_seed_with(1320);
    let mut setup = try_pending_deathrite_with_overlay_rubble_legal(&encoded, "north-flood")
        .expect("complete earth-on-flooded-rubble Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "south-earth"
                && action.descriptor["cell"] == "C1"
        })
    {
        accept_where(session, "south draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-earth"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1322_play_water_on_drought_rubble_withheld_during_pending_deathrite_order() {
    let encoded = drought_rubble_deathrite_seed_with(1322);
    let mut setup = try_pending_deathrite_with_overlay_rubble_legal(&encoded, "north-drought")
        .expect("complete water-on-drought-rubble Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());

    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !south_has_water_in_atlas(&state(session)) {
        accept_where(session, "south draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-water"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1324_play_water_on_flooded_rubble_withheld_during_pending_deathrite_order() {
    let encoded = flooded_water_rubble_deathrite_seed_with(1324);
    let mut setup = try_pending_deathrite_with_overlay_rubble_legal(&encoded, "north-flood")
        .expect("complete water-on-flooded-rubble Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !south_has_water_in_atlas(&state(session)) {
        accept_where(session, "south draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-water"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1326_play_earth_on_drought_rubble_withheld_during_pending_deathrite_order() {
    let encoded = drought_earth_rubble_deathrite_seed_with(1326);
    let mut setup = try_pending_deathrite_with_overlay_rubble_legal(&encoded, "north-drought")
        .expect("complete earth-on-drought-rubble Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "south-earth"
                && action.descriptor["cell"] == "C1"
        })
    {
        accept_where(session, "south draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-earth"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1340_play_earth_on_flooded_occupied_site_withheld_during_pending_deathrite_order() {
    let encoded = flooded_occupied_deathrite_seed_with(1340);
    let mut setup = try_pending_deathrite_with_overlay_occupied_legal(&encoded, "north-flood")
        .expect("complete earth-on-flooded-occupied-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_ne!(resumed["realm"]["sites"]["C1"]["rubble"], true);
    assert!(resumed["pendingDeathrites"].is_null());
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1341_play_water_on_drought_occupied_site_withheld_during_pending_deathrite_order() {
    let encoded = drought_occupied_deathrite_seed_with(1341);
    let mut setup = try_pending_deathrite_with_overlay_occupied_legal(&encoded, "north-drought")
        .expect("complete water-on-drought-occupied-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_ne!(resumed["realm"]["sites"]["C1"]["rubble"], true);
    assert!(resumed["pendingDeathrites"].is_null());
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1346_play_water_on_flooded_occupied_site_withheld_during_pending_deathrite_order() {
    let encoded = flooded_water_occupied_deathrite_seed_with(1346);
    let mut setup = try_pending_deathrite_with_overlay_occupied_legal(&encoded, "north-flood")
        .expect("complete water-on-flooded-occupied-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_ne!(resumed["realm"]["sites"]["C1"]["rubble"], true);
    assert!(resumed["pendingDeathrites"].is_null());
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1347_play_earth_on_drought_occupied_site_withheld_during_pending_deathrite_order() {
    let encoded = drought_earth_occupied_deathrite_seed_with(1347);
    let mut setup = try_pending_deathrite_with_overlay_occupied_legal(&encoded, "north-drought")
        .expect("complete earth-on-drought-occupied-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_ne!(resumed["realm"]["sites"]["C1"]["rubble"], true);
    assert!(resumed["pendingDeathrites"].is_null());
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1373_play_earth_on_flooded_occupied_site_offered_after_pending_deathrite_order() {
    let encoded = flooded_occupied_deathrite_seed_with(1373);
    let mut setup = try_pending_deathrite_with_overlay_occupied_legal(&encoded, "north-flood")
        .expect("complete earth-on-flooded-occupied-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "south-earth"
                && action.descriptor["cell"] == "C1"
        })
    {
        accept_where(session, "south draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-earth"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1374_play_water_on_drought_occupied_site_offered_after_pending_deathrite_order() {
    let encoded = drought_occupied_deathrite_seed_with(1374);
    let mut setup = try_pending_deathrite_with_overlay_occupied_legal(&encoded, "north-drought")
        .expect("complete water-on-drought-occupied-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");

    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !south_has_water_in_atlas(&state(session)) {
        accept_where(session, "south draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-water"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1375_play_water_on_flooded_occupied_site_offered_after_pending_deathrite_order() {
    let encoded = flooded_water_occupied_deathrite_seed_with(1375);
    let mut setup = try_pending_deathrite_with_overlay_occupied_legal(&encoded, "north-flood")
        .expect("complete water-on-flooded-occupied-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !south_has_water_in_atlas(&state(session)) {
        accept_where(session, "south draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-water"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1376_play_earth_on_drought_occupied_site_offered_after_pending_deathrite_order() {
    let encoded = drought_earth_occupied_deathrite_seed_with(1376);
    let mut setup = try_pending_deathrite_with_overlay_occupied_legal(&encoded, "north-drought")
        .expect("complete earth-on-drought-occupied-site Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    accept_where(session, "north end turn after Deathrites", |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(session, "south draw atlas", |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "south-earth"
                && action.descriptor["cell"] == "C1"
        })
    {
        accept_where(session, "south draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "south-earth"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1382_water_site_cast_minion_withheld_during_pending_deathrite_order_on_flooded_occupied_earth_then_offered()
 {
    let encoded = flooded_occupied_water_cast_deathrite_seed_with(1382);
    let mut setup =
        try_pending_deathrite_with_overlay_occupied_water_cast_summon(&encoded, "north-flood")
            .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water-cast"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C1"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1391_water_site_cast_minion_still_withheld_after_pending_deathrite_order_on_drought_occupied_water()
 {
    let encoded = drought_occupied_water_cast_deathrite_seed_with(1391);
    let mut setup = try_pending_deathrite_with_drought_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_ne!(paused["realm"]["sites"]["C3"]["rubble"], true);
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        paused["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water-cast"))
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        !session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C3"
            }),
        "Drought on an occupied Water site must keep that cell land after Deathrites complete"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1399_water_site_cast_minion_offered_after_pending_deathrite_order_on_flooded_occupied_water()
 {
    let encoded = flooded_water_c3_deathrite_seed_with(1399);
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1489_water_site_cast_withheld_during_pending_deathrite_order_on_flooded_occupied_water_at_c3()
 {
    let encoded = flooded_water_c3_deathrite_seed_with(1489);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_flooded_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    let paused_actions = setup.session.legal_actions().expect("paused legal actions");
    assert!(
        paused_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );
    assert!(
        paused_actions.iter().all(|action| {
            !(action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-water-cast")
        }),
        "cast-magic to water-site minion stays withheld during trigger-order"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1490_water_site_cast_offered_after_pending_deathrite_order_on_flooded_occupied_water_at_c3()
 {
    let encoded = flooded_water_c3_deathrite_seed_with(1490);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C4"
            }),
        "after Deathrites complete, water-site cast must resume on unaffected Water sites"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1400_water_site_cast_minion_withheld_during_pending_deathrite_order_on_flooded_occupied_water()
 {
    let encoded = flooded_water_c3_deathrite_seed_with(1400);
    let setup = try_pending_deathrite_with_flooded_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1433_water_site_cast_minion_withheld_during_pending_deathrite_order_on_drought_occupied_water()
 {
    let encoded = drought_occupied_water_cast_deathrite_seed_with(1433);
    let setup = try_pending_deathrite_with_drought_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1434_water_site_cast_minion_offered_after_pending_deathrite_order_on_drought_occupied_water()
 {
    let encoded = drought_occupied_water_cast_deathrite_seed_with(1434);
    let mut setup = try_pending_deathrite_with_drought_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C4"
            }),
        "after Deathrites complete, water-site cast must resume on unaffected Water sites"
    );
    assert!(
        !session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C3"
            }),
        "Drought on occupied Water at C3 must keep that cell unavailable for water-site cast"
    );
    assert_exact_replay(session);
}

fn flooded_water_c3_play_site_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-water-c3-play-site-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-water-c3-play-site-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth_site(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-earth",
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-flood",
                    "north-rain",
                    "north-flood",
                    "north-rain",
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

fn flooded_earth_c3_play_site_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-earth-c3-play-site-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-earth-c3-play-site-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth_site(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-water",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-flood",
                    "north-rain",
                    "north-flood",
                    "north-rain",
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

fn drought_earth_c3_play_site_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-earth-c3-play-site-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-earth-c3-play-site-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought_aura(),
            "north-earth": earth_site(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-water",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-drought",
                    "north-rain",
                    "north-drought",
                    "north-rain",
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

fn drought_water_c3_play_site_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-water-c3-play-site-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-water-c3-play-site-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought_aura(),
            "north-earth": earth_site(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-earth",
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-drought",
                    "north-rain",
                    "north-drought",
                    "north-rain",
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

fn drought_earth_c3_water_cast_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drought-earth-c3-water-cast-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drought-earth-c3-water-cast-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought_aura(),
            "north-earth": earth_site(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "north-water-cast": water_cast_minion(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-drought",
                    "north-rain",
                    "north-water-cast",
                    "north-drought",
                    "north-rain",
                    "north-water-cast",
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

fn flooded_earth_c3_water_cast_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "flooded-earth-c3-water-cast-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-flooded-earth-c3-water-cast-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-earth": earth_site(),
            "north-flood": flood_aura(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "north-water-cast": water_cast_minion(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-earth",
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-water",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-rain",
                    "north-water-cast",
                    "north-flood",
                    "north-rain",
                    "north-water-cast",
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

fn try_pending_deathrite_with_flooded_occupied_water_c3_play_site(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-flood")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !north_has_earth_in_atlas(&state(&session)) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_flooded_occupied_water_c3_play_water_site(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-flood")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !north_has_water_in_atlas(&state(&session)) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_flooded_occupied_earth_c3_play_site(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-earth"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-flood")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !north_has_earth_in_atlas(&state(&session)) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_flooded_occupied_earth_c3_play_water_site(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-earth"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-flood")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !north_has_water_in_atlas(&state(&session)) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_drought_occupied_earth_c3_play_site(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-earth"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-drought")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !north_has_water_in_atlas(&state(&session)) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_drought_occupied_earth_c3_play_earth_site(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-earth"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-drought")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !north_has_earth_in_atlas(&state(&session)) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_drought_occupied_water_c3_play_site(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-drought")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !north_has_water_in_atlas(&state(&session)) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_drought_occupied_water_c3_play_earth(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-water"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-drought")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-water" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !north_has_earth_in_atlas(&state(&session)) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-earth"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-drought")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water-cast"))
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn try_pending_deathrite_with_flooded_occupied_earth_c3_cast_summon(
    encoded: &str,
) -> Option<PendingDeathriteWaterOnRubbleSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    }) || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "north-earth"
                && descriptor["cell"] == "C3"
        })
        || !pass_turn_draw_spellbook(&mut session)
        || !pass_turn_draw_spellbook(&mut session)
        || !try_accept_where(&mut session, |descriptor| {
            overlay_covers_c3(descriptor, "north-flood")
        })
    {
        return None;
    }
    if state(&session)["realm"]["sites"]["C3"]["cardId"] != "north-earth" {
        return None;
    }
    if !pass_turn_draw_spellbook(&mut session) {
        return None;
    }
    let first = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where_pair(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    if !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
        || !try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
    {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    if state(&session)["phase"] != "trigger-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "trigger-order")
    {
        return None;
    }
    if !state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-water-cast"))
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteWaterOnRubbleSetup {
        deathrite_ids,
        session,
    })
}

fn flooded_water_c3_play_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_water_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_water_c3_play_site(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Earth play on flooded occupied Water",
        )
}

fn flooded_water_c3_play_water_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_water_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_water_c3_play_water_site(candidate)
                .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Water play on flooded occupied Water",
        )
}

fn flooded_water_c3_draw_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_water_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_water_c3_play_water_site(candidate)
                .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with draw-site withheld on flooded occupied Water",
        )
}

fn flooded_water_c3_draw_site_after_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_water_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_water_c3_play_water_site(candidate)
                .is_some_and(|mut setup| {
                    let withheld = setup.session.legal_actions().is_ok_and(|actions| {
                        actions.iter().all(|action| {
                            action.descriptor["kind"] != "draw-site"
                                && action.descriptor["kind"] != "play-site"
                        })
                    });
                    if !withheld {
                        return false;
                    }
                    let deathrite_ids = setup.deathrite_ids.clone();
                    let session = &mut setup.session;
                    try_accept_where(session, |descriptor| {
                        descriptor["kind"] == "order-triggers"
                            && descriptor["sourceInstanceId"] == deathrite_ids[0]
                    }) && {
                        let resumed = state(session);
                        resumed["phase"] == "main"
                            && resumed["decisionSeat"] == "north"
                            && resumed["pendingDeathrites"].is_null()
                            && session.legal_actions().is_ok_and(|actions| {
                                actions
                                    .iter()
                                    .any(|action| action.descriptor["kind"] == "draw-site")
                            })
                    }
                })
        })
        .expect("bounded seed that resumes draw-site after Deathrites on flooded occupied Water")
}

fn flooded_earth_c3_play_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_earth_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_earth_c3_play_site(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Earth play on flooded occupied Earth",
        )
}

fn flooded_earth_c3_play_water_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_earth_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_earth_c3_play_water_site(candidate)
                .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Water play on flooded occupied Earth",
        )
}

fn flooded_earth_c3_draw_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_earth_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_earth_c3_play_water_site(candidate)
                .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with draw-site withheld on flooded occupied Earth",
        )
}

fn flooded_earth_c3_draw_site_after_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_earth_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_earth_c3_play_water_site(candidate)
                .is_some_and(|mut setup| {
                    let withheld = setup.session.legal_actions().is_ok_and(|actions| {
                        actions.iter().all(|action| {
                            action.descriptor["kind"] != "draw-site"
                                && action.descriptor["kind"] != "play-site"
                        })
                    });
                    if !withheld {
                        return false;
                    }
                    let deathrite_ids = setup.deathrite_ids.clone();
                    let session = &mut setup.session;
                    try_accept_where(session, |descriptor| {
                        descriptor["kind"] == "order-triggers"
                            && descriptor["sourceInstanceId"] == deathrite_ids[0]
                    }) && {
                        let resumed = state(session);
                        resumed["phase"] == "main"
                            && resumed["decisionSeat"] == "north"
                            && resumed["pendingDeathrites"].is_null()
                            && session.legal_actions().is_ok_and(|actions| {
                                actions
                                    .iter()
                                    .any(|action| action.descriptor["kind"] == "draw-site")
                            })
                    }
                })
        })
        .expect("bounded seed that resumes draw-site after Deathrites on flooded occupied Earth")
}

fn drought_water_c3_draw_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_water_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_water_c3_play_site(candidate).is_some_and(
                |setup| {
                    setup.session.legal_actions().is_ok_and(|actions| {
                        actions.iter().all(|action| {
                            action.descriptor["kind"] != "draw-site"
                                && action.descriptor["kind"] != "play-site"
                        })
                    })
                },
            )
        })
        .expect(
            "bounded seed that reaches pending Deathrites with draw-site withheld on drought occupied Water",
        )
}

fn drought_water_c3_draw_site_after_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_water_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_water_c3_play_site(candidate).is_some_and(
                |mut setup| {
                    let withheld = setup.session.legal_actions().is_ok_and(|actions| {
                        actions.iter().all(|action| {
                            action.descriptor["kind"] != "draw-site"
                                && action.descriptor["kind"] != "play-site"
                        })
                    });
                    if !withheld {
                        return false;
                    }
                    let deathrite_id = setup.deathrite_ids[0].clone();
                    if !try_accept_where(&mut setup.session, |descriptor| {
                        descriptor["kind"] == "order-triggers"
                            && descriptor["sourceInstanceId"] == deathrite_id
                    }) {
                        return false;
                    }
                    let resumed = state(&setup.session);
                    resumed["phase"] == "main"
                        && resumed["decisionSeat"] == "north"
                        && resumed["pendingDeathrites"].is_null()
                        && setup.session.legal_actions().is_ok_and(|actions| {
                            actions
                                .iter()
                                .any(|action| action.descriptor["kind"] == "draw-site")
                        })
                },
            )
        })
        .expect("bounded seed that resumes draw-site after Deathrites on drought occupied Water")
}

fn drought_earth_c3_play_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_earth_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_earth_c3_play_site(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Water play on drought occupied Earth",
        )
}

fn drought_earth_c3_play_earth_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_earth_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_earth_c3_play_earth_site(candidate)
                .is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Earth play on drought occupied Earth",
        )
}

fn drought_earth_c3_draw_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_earth_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_earth_c3_play_earth_site(candidate)
                .is_some_and(|setup| {
                    setup.session.legal_actions().is_ok_and(|actions| {
                        actions.iter().all(|action| {
                            action.descriptor["kind"] != "draw-site"
                                && action.descriptor["kind"] != "play-site"
                        })
                    })
                })
        })
        .expect(
            "bounded seed that reaches pending Deathrites with draw-site withheld on drought occupied Earth",
        )
}

fn drought_earth_c3_draw_site_after_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_earth_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_earth_c3_play_earth_site(candidate)
                .is_some_and(|mut setup| {
                    let withheld = setup.session.legal_actions().is_ok_and(|actions| {
                        actions.iter().all(|action| {
                            action.descriptor["kind"] != "draw-site"
                                && action.descriptor["kind"] != "play-site"
                        })
                    });
                    if !withheld {
                        return false;
                    }
                    let deathrite_id = setup.deathrite_ids[0].clone();
                    if !try_accept_where(&mut setup.session, |descriptor| {
                        descriptor["kind"] == "order-triggers"
                            && descriptor["sourceInstanceId"] == deathrite_id
                    }) {
                        return false;
                    }
                    let resumed = state(&setup.session);
                    resumed["phase"] == "main"
                        && resumed["decisionSeat"] == "north"
                        && resumed["pendingDeathrites"].is_null()
                        && setup.session.legal_actions().is_ok_and(|actions| {
                            actions
                                .iter()
                                .any(|action| action.descriptor["kind"] == "draw-site")
                        })
                })
        })
        .expect("bounded seed that resumes draw-site after Deathrites on drought occupied Earth")
}

fn drought_water_c3_play_site_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_water_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_water_c3_play_site(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Water play on drought occupied Water",
        )
}

fn drought_water_c3_play_earth_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_water_c3_play_site_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_water_c3_play_earth(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with Earth play on drought occupied Water",
        )
}

fn drought_earth_c3_water_cast_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(drought_earth_c3_water_cast_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a water-site cast minion on drought occupied Earth",
        )
}

fn flooded_earth_c3_water_cast_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(flooded_earth_c3_water_cast_manifest)
        .find(|candidate| {
            try_pending_deathrite_with_flooded_occupied_earth_c3_cast_summon(candidate).is_some()
        })
        .expect(
            "bounded seed that reaches pending Deathrites with a water-site cast minion on flooded occupied Earth",
        )
}

#[test]
fn rule_catalog_1403_play_earth_on_flooded_occupied_water_withheld_during_pending_deathrite_order()
{
    let encoded = flooded_water_c3_play_site_seed_with(1403);
    let setup = try_pending_deathrite_with_flooded_occupied_water_c3_play_site(&encoded)
        .expect("complete earth-on-flooded-occupied-water Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1437_play_earth_on_flooded_occupied_earth_withheld_during_pending_deathrite_order()
{
    let encoded = flooded_earth_c3_play_site_seed_with(1437);
    let setup = try_pending_deathrite_with_flooded_occupied_earth_c3_play_site(&encoded)
        .expect("complete earth-on-flooded-occupied-earth Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1446_play_water_on_flooded_occupied_earth_withheld_during_pending_deathrite_order()
{
    let encoded = flooded_earth_c3_play_water_site_seed_with(1446);
    let setup = try_pending_deathrite_with_flooded_occupied_earth_c3_play_water_site(&encoded)
        .expect("complete water-on-flooded-occupied-earth Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1453_play_water_on_flooded_occupied_water_withheld_during_pending_deathrite_order()
{
    let encoded = flooded_water_c3_play_water_site_seed_with(1453);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_flooded_occupied_water_c3_play_water_site(&encoded)
        .expect("complete water-on-flooded-occupied-water Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1465_draw_site_withheld_during_pending_deathrite_order_on_flooded_occupied_water_at_c3()
 {
    let encoded = flooded_water_c3_draw_site_seed_with(1465);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_flooded_occupied_water_c3_play_water_site(&encoded)
        .expect("complete draw-site-on-flooded-occupied-water Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw-site" && action.descriptor["kind"] != "play-site"
            }),
        "draw-site stays withheld during trigger-order (mutually exclusive with play-site)"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1515_draw_site_withheld_during_pending_deathrite_order_on_flooded_occupied_earth_at_c3()
 {
    let encoded = flooded_earth_c3_draw_site_seed_with(1515);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_flooded_occupied_earth_c3_play_water_site(&encoded)
        .expect("complete draw-site-on-flooded-occupied-earth Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw-site" && action.descriptor["kind"] != "play-site"
            }),
        "draw-site stays withheld during trigger-order (mutually exclusive with play-site)"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1516_draw_site_withheld_during_pending_deathrite_order_on_drought_occupied_water_at_c3()
 {
    let encoded = drought_water_c3_draw_site_seed_with(1516);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_drought_occupied_water_c3_play_site(&encoded)
        .expect("complete draw-site on drought occupied Water Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw-site" && action.descriptor["kind"] != "play-site"
            })
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1517_draw_site_offered_after_pending_deathrite_order_on_flooded_occupied_earth_at_c3()
 {
    let encoded = flooded_earth_c3_draw_site_after_seed_with(1517);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_flooded_occupied_earth_c3_play_water_site(&encoded)
        .expect("complete draw-site-on-flooded-occupied-earth Deathrite resume setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw-site" && action.descriptor["kind"] != "play-site"
            }),
        "draw-site stays withheld during trigger-order (mutually exclusive with play-site)"
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| action.descriptor["kind"] == "draw-site"),
        "draw-site is offered again after Deathrites when atlas lacks the needed site card"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1518_draw_site_offered_after_pending_deathrite_order_on_drought_occupied_water_at_c3()
 {
    let encoded = drought_water_c3_draw_site_after_seed_with(1518);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_drought_occupied_water_c3_play_site(&encoded)
        .expect("complete draw-site after drought occupied Water Deathrite resume setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw-site" && action.descriptor["kind"] != "play-site"
            }),
        "draw-site stays withheld during trigger-order (mutually exclusive with play-site)"
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| action.descriptor["kind"] == "draw-site"),
        "draw-site is offered again after Deathrites when atlas lacks the needed site card"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1475_draw_site_offered_after_pending_deathrite_order_on_flooded_occupied_water_at_c3()
 {
    let encoded = flooded_water_c3_draw_site_after_seed_with(1475);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_c3_play_water_site(&encoded)
        .expect("complete draw-site-on-flooded-occupied-water Deathrite resume setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw-site" && action.descriptor["kind"] != "play-site"
            }),
        "draw-site stays withheld during trigger-order (mutually exclusive with play-site)"
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| action.descriptor["kind"] == "draw-site"),
        "draw-site is offered again after Deathrites when atlas lacks the needed site card"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1404_play_water_on_drought_occupied_earth_withheld_during_pending_deathrite_order()
{
    let encoded = drought_earth_c3_play_site_seed_with(1404);
    let setup = try_pending_deathrite_with_drought_occupied_earth_c3_play_site(&encoded)
        .expect("complete water-on-drought-occupied-earth Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1435_play_water_on_drought_occupied_water_withheld_during_pending_deathrite_order()
{
    let encoded = drought_water_c3_play_site_seed_with(1435);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_drought_occupied_water_c3_play_site(&encoded)
        .expect("complete water-on-drought-occupied-water Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1448_play_earth_on_drought_occupied_water_withheld_during_pending_deathrite_order()
{
    let encoded = drought_water_c3_play_earth_seed_with(1448);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_drought_occupied_water_c3_play_earth(&encoded)
        .expect("complete earth-on-drought-occupied-water Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1455_play_earth_on_drought_occupied_earth_withheld_during_pending_deathrite_order()
{
    let encoded = drought_earth_c3_play_earth_site_seed_with(1455);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_drought_occupied_earth_c3_play_earth_site(&encoded)
        .expect("complete earth-on-drought-occupied-earth Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1456_play_earth_on_drought_occupied_earth_offered_after_pending_deathrite_order() {
    let encoded = drought_earth_c3_play_earth_site_seed_with(1456);
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_play_earth_site(&encoded)
        .expect("complete earth-on-drought-occupied-earth Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-earth"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "north-earth"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1466_draw_site_withheld_during_pending_deathrite_order_on_drought_occupied_earth_at_c3()
 {
    let encoded = drought_earth_c3_draw_site_seed_with(1466);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_drought_occupied_earth_c3_play_earth_site(&encoded)
        .expect("complete draw-site on drought occupied Earth Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw-site" && action.descriptor["kind"] != "play-site"
            })
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1476_draw_site_offered_after_pending_deathrite_order_on_drought_occupied_earth_at_c3()
 {
    let encoded = drought_earth_c3_draw_site_after_seed_with(1476);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_play_earth_site(&encoded)
        .expect("complete draw-site after drought occupied Earth Deathrite resume setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw-site" && action.descriptor["kind"] != "play-site"
            })
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| action.descriptor["kind"] == "draw-site"),
        "draw-site is offered again after Deathrites when a site is still needed"
    );
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-earth"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1405_play_earth_on_flooded_occupied_water_offered_after_pending_deathrite_order() {
    let encoded = flooded_water_c3_play_site_seed_with(1405);
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_c3_play_site(&encoded)
        .expect("complete earth-on-flooded-occupied-water Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-earth"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "north-earth"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1438_play_earth_on_flooded_occupied_earth_offered_after_pending_deathrite_order() {
    let encoded = flooded_earth_c3_play_site_seed_with(1438);
    let mut setup = try_pending_deathrite_with_flooded_occupied_earth_c3_play_site(&encoded)
        .expect("complete earth-on-flooded-occupied-earth Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-earth"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "north-earth"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1447_play_water_on_flooded_occupied_earth_offered_after_pending_deathrite_order() {
    let encoded = flooded_earth_c3_play_water_site_seed_with(1447);
    let mut setup = try_pending_deathrite_with_flooded_occupied_earth_c3_play_water_site(&encoded)
        .expect("complete water-on-flooded-occupied-earth Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-water"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "north-water"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1454_play_water_on_flooded_occupied_water_offered_after_pending_deathrite_order() {
    let encoded = flooded_water_c3_play_water_site_seed_with(1454);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_c3_play_water_site(&encoded)
        .expect("complete water-on-flooded-occupied-water Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-water"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "north-water"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1406_play_water_on_drought_occupied_earth_offered_after_pending_deathrite_order() {
    let encoded = drought_earth_c3_play_site_seed_with(1406);
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_play_site(&encoded)
        .expect("complete water-on-drought-occupied-earth Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-water"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "north-water"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1436_play_water_on_drought_occupied_water_offered_after_pending_deathrite_order() {
    let encoded = drought_water_c3_play_site_seed_with(1436);
    let mut setup = try_pending_deathrite_with_drought_occupied_water_c3_play_site(&encoded)
        .expect("complete water-on-drought-occupied-water Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-water"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for water", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "north-water"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1449_play_earth_on_drought_occupied_water_offered_after_pending_deathrite_order() {
    let encoded = drought_water_c3_play_earth_seed_with(1449);
    let mut setup = try_pending_deathrite_with_drought_occupied_water_c3_play_earth(&encoded)
        .expect("complete earth-on-drought-occupied-water Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    if !session
        .legal_actions()
        .expect("resumed legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-earth"
                && action.descriptor["cell"] == "C3"
        })
    {
        accept_where(session, "north draw-site for earth", |descriptor| {
            descriptor["kind"] == "draw-site"
        });
    }
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "play-site"
                    && action.descriptor["cardId"] == "north-earth"
                    && action.descriptor["cell"] == "C3"
            })
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1411_water_site_cast_minion_still_withheld_after_pending_deathrite_order_on_drought_occupied_earth()
 {
    let encoded = drought_earth_c3_water_cast_seed_with(1411);
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        !session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C3"
            }),
        "Drought on an occupied Earth site must keep that cell land after Deathrites complete"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1414_water_site_cast_minion_withheld_during_pending_deathrite_order_on_drought_occupied_earth()
 {
    let encoded = drought_earth_c3_water_cast_seed_with(1414);
    let setup = try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1499_water_site_cast_withheld_during_pending_deathrite_order_on_drought_occupied_earth_at_c3()
 {
    let encoded = drought_earth_c3_water_cast_seed_with(1499);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    let paused_actions = setup.session.legal_actions().expect("paused legal actions");
    assert!(
        paused_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );
    assert!(
        paused_actions.iter().all(|action| {
            !(action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-water-cast")
        }),
        "cast-magic to water-site minion stays withheld during trigger-order"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1500_water_site_cast_offered_after_pending_deathrite_order_on_drought_occupied_earth_at_c3()
 {
    let encoded = drought_earth_c3_water_cast_seed_with(1500);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C4"
            }),
        "after Deathrites complete, water-site cast must resume on unaffected Water sites"
    );
    assert!(
        !session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C3"
            }),
        "Drought on occupied Earth at C3 must keep that cell unavailable for water-site cast"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1443_water_site_cast_minion_offered_after_pending_deathrite_order_on_drought_occupied_earth()
 {
    let encoded = drought_earth_c3_water_cast_seed_with(1443);
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C4"
            }),
        "after Deathrites complete, water-site cast must resume on unaffected Water sites"
    );
    assert!(
        !session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C3"
            }),
        "Drought on occupied Earth at C3 must keep that cell unavailable for water-site cast"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1445_water_site_cast_minion_offered_after_pending_deathrite_order_on_flooded_occupied_earth()
 {
    let encoded = flooded_earth_c3_water_cast_seed_with(1445);
    let mut setup = try_pending_deathrite_with_flooded_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    let resumed_actions = session.legal_actions().expect("resumed legal actions");
    assert!(
        resumed_actions.iter().any(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-water-cast"
                && action.descriptor["cell"] == "C3"
        }),
        "after Deathrites complete, water-site cast must resume on flooded occupied Earth at C3"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1444_water_site_cast_minion_withheld_during_pending_deathrite_order_on_flooded_occupied_earth()
 {
    let encoded = flooded_earth_c3_water_cast_seed_with(1444);
    let setup = try_pending_deathrite_with_flooded_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1509_water_site_cast_withheld_during_pending_deathrite_order_on_flooded_occupied_earth_at_c3()
 {
    let encoded = flooded_earth_c3_water_cast_seed_with(1509);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_flooded_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    let paused_actions = setup.session.legal_actions().expect("paused legal actions");
    assert!(
        paused_actions
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );
    assert!(
        paused_actions.iter().all(|action| {
            !(action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-water-cast")
        }),
        "cast-magic to water-site minion stays withheld during trigger-order"
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1510_water_site_cast_offered_after_pending_deathrite_order_on_flooded_occupied_earth_at_c3()
 {
    let encoded = flooded_earth_c3_water_cast_seed_with(1510);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_flooded_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-earth"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    let resumed_actions = session.legal_actions().expect("resumed legal actions");
    assert!(
        resumed_actions.iter().any(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-water-cast"
                && action.descriptor["cell"] == "C3"
        }),
        "after Deathrites complete, water-site cast must resume on flooded occupied Earth at C3"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1520_water_site_cast_offered_after_pending_deathrite_order_on_flooded_occupied_water_at_c3()
 {
    let encoded = flooded_water_c3_deathrite_seed_with(1520);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C4"
            }),
        "after Deathrites complete, water-site cast must resume on unaffected Water sites"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1529_water_site_cast_withheld_during_pending_deathrite_order_on_drought_occupied_water_at_c3()
 {
    let encoded = drought_occupied_water_cast_deathrite_seed_with(1529);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let setup = try_pending_deathrite_with_drought_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-water");
    assert!(
        setup
            .session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );
    assert_exact_replay(&setup.session);
}

#[test]
fn rule_catalog_1530_water_site_cast_offered_after_pending_deathrite_order_on_drought_occupied_water_at_c3()
 {
    let encoded = drought_occupied_water_cast_deathrite_seed_with(1530);
    eprintln!(
        "seed={}",
        serde_json::from_str::<Value>(&encoded).expect("manifest json")["seed"]
    );
    let mut setup = try_pending_deathrite_with_drought_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "trigger-order");
    assert_eq!(
        state(session)["realm"]["sites"]["C3"]["cardId"],
        "north-water"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
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
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-water-cast"
                    && action.descriptor["cell"] == "C4"
            }),
        "after Deathrites complete, water-site cast must resume on unaffected Water sites"
    );
    assert_exact_replay(session);
}
