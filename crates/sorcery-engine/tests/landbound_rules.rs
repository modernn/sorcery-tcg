//! Direct proofs for official Landbound (RULE-CATALOG-0318–0322).
//!
//! Landbound is the land-site sibling of Waterbound. A minion is Disabled while
//! it occupies no land location. The Landbound ability itself still applies
//! while Disabled. A land site provides zero Water affinity, so mixed Water
//! sites and Flood overlays are not land. Drought is the inverse overlay: a
//! printed Water site under Drought is land again.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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

fn site(elements: &[&str]) -> Value {
    json!({ "cardType": "site", "elements": elements })
}

fn landbound() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "landbound": true,
        "manaCost": 0,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn square_landbound() -> Value {
    let mut value = landbound();
    value["occupiesSquareArea"] = json!(2);
    value
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

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn movement_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "landbound-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-landbound-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-land": site(&["earth"]),
            "north-landbound": landbound(),
            "north-water": site(&["water"]),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                ],
                "avatar": "north-avatar",
                "spellbook": vec!["north-landbound"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
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

fn flood_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "landbound-flood" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-landbound-flood-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-flood": flood(),
            "north-land": site(&["earth"]),
            "north-landbound": landbound(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-land"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-landbound",
                    "north-landbound",
                    "north-landbound",
                    "north-flood",
                    "north-flood",
                    "north-flood"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
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

fn drought_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "landbound-drought" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-landbound-drought-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-landbound": landbound(),
            "north-water": site(&["water"]),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-landbound",
                    "north-landbound",
                    "north-landbound",
                    "north-drought",
                    "north-drought",
                    "north-drought"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
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

fn square_flood_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "landbound-square-flood" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-landbound-square-flood-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-flood": flood(),
            "north-land": site(&["earth"]),
            "north-square-landbound": square_landbound(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-land"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-flood",
                    "north-flood",
                    "north-flood",
                    "north-flood"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
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

fn square_drought_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "landbound-square-drought" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-landbound-square-drought-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-drought": drought(),
            "north-square-landbound": square_landbound(),
            "north-water": site(&["water"]),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 12],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-square-landbound",
                    "north-drought",
                    "north-drought",
                    "north-drought",
                    "north-drought"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn units(session: &Session) -> Vec<Value> {
    state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .clone()
}

fn observed_unit(session: &Session, instance_id: &str) -> Value {
    session.public_view(Seat::North).expect("north observation")["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("named unit")
        .clone()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn opening_atlas_ids(session: &Session) -> Vec<String> {
    state(session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north atlas")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn opening_spell_ids(session: &Session) -> Vec<String> {
    state(session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north spellbook")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn movement_opening() -> Session {
    (1..=4096)
        .map(movement_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Landbound movement candidate");
            let atlas = opening_atlas_ids(&session);
            (atlas.contains(&"north-land".to_owned()) && atlas.contains(&"north-water".to_owned()))
                .then_some(session)
        })
        .expect("bounded seed opening with a land site")
}

fn flood_opening() -> Session {
    (1..=4096)
        .map(flood_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Landbound Flood candidate");
            let spells = opening_spell_ids(&session);
            (spells.iter().any(|card| card == "north-landbound")
                && spells.iter().any(|card| card == "north-flood"))
            .then_some(session)
        })
        .expect("bounded seed opening with Landbound and Flood")
}

fn drought_opening() -> Session {
    (1..=4096)
        .map(drought_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("Landbound Drought candidate");
            let spells = opening_spell_ids(&session);
            (spells.iter().any(|card| card == "north-landbound")
                && spells.iter().any(|card| card == "north-drought"))
            .then_some(session)
        })
        .expect("bounded seed opening with Landbound and Drought")
}

fn activates_mana(bound_id: &str) -> impl Fn(&Value) -> bool + '_ {
    move |descriptor: &Value| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == bound_id
    }
}

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

#[test]
fn rule_catalog_0318_landbound_is_disabled_off_a_land_site() {
    let mut session = movement_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-land"
            && descriptor["cell"] == "C4"
    });
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C3"
    });
    assert!(offers(&session, activates_mana(&bound_id)));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bound_id
            && descriptor["to"]["cell"] == "C3"
            && descriptor["to"]["region"] == "surface"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let after = observed_unit(&session, &bound_id);
    assert_eq!(after["disabled"], true);
    assert_eq!(after["location"], "C3");
    assert!(!offers(&session, activates_mana(&bound_id)));
    assert_eq!(units(&session).len(), 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0319_flood_overlay_disables_landbound_in_place() {
    let mut session = flood_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C4")
    });
    let after = observed_unit(&session, &bound_id);
    assert_eq!(after["disabled"], true);
    assert_eq!(after["location"], "C4");
    assert!(!offers(&session, activates_mana(&bound_id)));
    assert_eq!(units(&session).len(), 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0322_drought_overlay_enables_landbound_in_place() {
    let mut session = drought_opening();
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-landbound"
            && descriptor["cell"] == "C4"
    });
    let bound_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Landbound identity")
        .to_owned();
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], true);
    assert!(!offers(&session, activates_mana(&bound_id)));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && cells_include(descriptor, "C4")
    });
    let after = observed_unit(&session, &bound_id);
    assert_eq!(after["disabled"], false);
    assert_eq!(after["location"], "C4");
    assert_eq!(units(&session).len(), 1);
    assert_exact_replay(&session);
}

fn square_flood_opening() -> Session {
    (1..=4096)
        .map(square_flood_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("2x2 Landbound Flood candidate");
            let spells = opening_spell_ids(&session);
            (spells.iter().any(|card| card == "north-square-landbound")
                && spells.iter().any(|card| card == "north-flood"))
            .then_some(session)
        })
        .expect("bounded seed opening with 2x2 Landbound and Flood")
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

#[test]
fn rule_catalog_0507_square_landbound_stays_enabled_when_flood_covers_only_part_of_its_footprint() {
    let mut session = square_flood_opening();
    let bound_id = summon_square_landbound_at_b3(&mut session);
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], false);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "A3")
            && !cells_include(descriptor, "C3")
    });
    let after = observed_unit(&session, &bound_id);
    assert_eq!(after["disabled"], false);
    assert_eq!(after["location"], "B3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0508_square_landbound_is_disabled_when_flood_covers_its_whole_footprint() {
    let mut session = square_flood_opening();
    let bound_id = summon_square_landbound_at_b3(&mut session);
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], false);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "B3")
            && cells_include(descriptor, "C4")
    });
    let after = observed_unit(&session, &bound_id);
    assert_eq!(after["disabled"], true);
    assert_eq!(after["location"], "B3");
    assert!(!offers(&session, activates_mana(&bound_id)));
    assert_exact_replay(&session);
}

fn square_drought_opening() -> Session {
    (1..=4096)
        .map(square_drought_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("2x2 Landbound Drought candidate");
            let spells = opening_spell_ids(&session);
            (spells.iter().any(|card| card == "north-square-landbound")
                && spells.iter().any(|card| card == "north-drought"))
            .then_some(session)
        })
        .expect("bounded seed opening with 2x2 Landbound and Drought")
}

#[test]
fn rule_catalog_0511_square_landbound_is_enabled_when_drought_covers_its_whole_water_footprint() {
    let mut session = square_drought_opening();
    let bound_id = summon_square_landbound_at_b3(&mut session);
    assert_eq!(observed_unit(&session, &bound_id)["disabled"], true);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-drought"
            && cells_include(descriptor, "B3")
            && cells_include(descriptor, "C4")
    });
    let after = observed_unit(&session, &bound_id);
    assert_eq!(after["disabled"], false);
    assert_eq!(after["location"], "B3");
    assert_exact_replay(&session);
}

fn rain() -> Value {
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

fn deathrite_activate_mana_manifest(seed: u32) -> String {
    let fixture = "landbound-activate-mana-deathrite-withheld";
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-landbound": landbound(),
            "north-rain": rain(),
            "north-site": site(&["earth"]),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_minion(),
            "south-site": site(&["earth"]),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-landbound",
                    "north-rain",
                    "north-rain",
                    "north-landbound",
                    "north-rain",
                    "north-rain"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

struct PendingDeathriteActivateManaSetup {
    deathrite_ids: [String; 2],
    mana_id: String,
    session: Session,
}

fn try_pending_deathrite_with_activate_mana(
    encoded: &str,
) -> Option<PendingDeathriteActivateManaSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let summoned = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-landbound"
            && descriptor["cell"] == "C4"
    })?;
    let mana_id = summoned.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !offers(&session, activates_mana(&mana_id)) {
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
    Some(PendingDeathriteActivateManaSetup {
        deathrite_ids,
        mana_id,
        session,
    })
}

fn deathrite_activate_mana_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_activate_mana_manifest)
        .find(|candidate| try_pending_deathrite_with_activate_mana(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with legal activate-mana")
}

#[test]
fn rule_catalog_1150_activate_mana_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_activate_mana_seed_with(1150);
    let mut setup = try_pending_deathrite_with_activate_mana(&encoded)
        .expect("complete activate-mana Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let mana_id = setup.mana_id.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(observed_unit(session, &mana_id)["tapped"], false);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "activate-mana")
    );
    assert!(!offers(session, activates_mana(&mana_id)));

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
    assert!(offers(session, activates_mana(&mana_id)));
    assert_exact_replay(session);
}
