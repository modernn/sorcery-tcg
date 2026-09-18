//! Direct proofs for Water site terrain settlement: underwater Deathrite source
//! region preservation, Genesis resume after ordered Deathrites
//! (RULE-CATALOG-0713–0714), state-based region settlement when Water floods
//! underground Burrowing minions without Submerge (RULE-CATALOG-0901), the
//! burrow-to-submerge relayer that lets dual-region occupants survive the same
//! flood (RULE-CATALOG-0909), playing Water onto empty rubble
//! (RULE-CATALOG-1314), play-site water-on-rubble withheld during
//! deathrite-order (RULE-CATALOG-1315), playing earth onto empty rubble
//! (RULE-CATALOG-1317), play-site earth-on-rubble withheld during
//! deathrite-order (RULE-CATALOG-1318), play-site earth-on-flooded-rubble
//! withheld during deathrite-order (RULE-CATALOG-1320), play-site
//! water-on-drought-rubble withheld during deathrite-order
//! (RULE-CATALOG-1322), play-site water-on-flooded-rubble withheld during
//! deathrite-order (RULE-CATALOG-1324), play-site earth-on-drought-rubble
//! withheld during deathrite-order (RULE-CATALOG-1326), and play-site on
//! overlay-covered occupied sites withheld during deathrite-order
//! (RULE-CATALOG-1340–1341, RULE-CATALOG-1346–1347,
//! RULE-CATALOG-1373–1376, RULE-CATALOG-1382, RULE-CATALOG-1391,
//! RULE-CATALOG-1399–1400, RULE-CATALOG-1403–1406, RULE-CATALOG-1411,
//! RULE-CATALOG-1414, RULE-CATALOG-1435).

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
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
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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
    assert_eq!(paused["phase"], "deathrite-order");
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
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
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
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
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
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
    assert_ne!(paused["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
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
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
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
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
fn rule_catalog_1400_water_site_cast_minion_withheld_during_pending_deathrite_order_on_flooded_occupied_water()
 {
    let encoded = flooded_water_c3_deathrite_seed_with(1400);
    let setup = try_pending_deathrite_with_flooded_occupied_water_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "deathrite-order");
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
    assert_eq!(paused["phase"], "deathrite-order");
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
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
            "north-water-cast": water_cast_minion(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-earth"; 6],
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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

fn try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(
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
    if state(&session)["phase"] != "deathrite-order"
        && (!try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
        }) || state(&session)["phase"] != "deathrite-order")
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

#[test]
fn rule_catalog_1403_play_earth_on_flooded_occupied_water_withheld_during_pending_deathrite_order()
{
    let encoded = flooded_water_c3_play_site_seed_with(1403);
    let setup = try_pending_deathrite_with_flooded_occupied_water_c3_play_site(&encoded)
        .expect("complete earth-on-flooded-occupied-water Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "deathrite-order");
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
    assert_eq!(paused["phase"], "deathrite-order");
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
fn rule_catalog_1404_play_water_on_drought_occupied_earth_withheld_during_pending_deathrite_order()
{
    let encoded = drought_earth_c3_play_site_seed_with(1404);
    let setup = try_pending_deathrite_with_drought_occupied_earth_c3_play_site(&encoded)
        .expect("complete water-on-drought-occupied-earth Deathrite withheld setup");
    let paused = state(&setup.session);
    assert_eq!(paused["phase"], "deathrite-order");
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
    assert_eq!(paused["phase"], "deathrite-order");
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
fn rule_catalog_1405_play_earth_on_flooded_occupied_water_offered_after_pending_deathrite_order() {
    let encoded = flooded_water_c3_play_site_seed_with(1405);
    let mut setup = try_pending_deathrite_with_flooded_occupied_water_c3_play_site(&encoded)
        .expect("complete earth-on-flooded-occupied-water Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert_eq!(state(session)["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
fn rule_catalog_1406_play_water_on_drought_occupied_earth_offered_after_pending_deathrite_order() {
    let encoded = drought_earth_c3_play_site_seed_with(1406);
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_play_site(&encoded)
        .expect("complete water-on-drought-occupied-earth Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(state(session)["phase"], "deathrite-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "play-site")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
fn rule_catalog_1411_water_site_cast_minion_still_withheld_after_pending_deathrite_order_on_drought_occupied_earth()
 {
    let encoded = drought_earth_c3_water_cast_seed_with(1411);
    let mut setup = try_pending_deathrite_with_drought_occupied_earth_c3_cast_summon(&encoded)
        .expect("complete water-site cast summon Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["realm"]["sites"]["C3"]["cardId"], "north-earth");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "summon-minion")
    );

    accept_where(session, "order first Deathrite", |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
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
    assert_eq!(paused["phase"], "deathrite-order");
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
