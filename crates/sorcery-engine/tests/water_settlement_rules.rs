//! Direct proofs for Water site terrain settlement: underwater Deathrite source
//! region preservation, Genesis resume after ordered Deathrites
//! (RULE-CATALOG-0713–0714), state-based region settlement when Water floods
//! underground Burrowing minions without Submerge (RULE-CATALOG-0901), and the
//! burrow-to-submerge relayer that lets dual-region occupants survive the same
//! flood (RULE-CATALOG-0909).

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
