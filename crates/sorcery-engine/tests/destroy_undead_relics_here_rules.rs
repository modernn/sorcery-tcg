//! Direct proofs for destroy-undead-minions-and-artifacts-at-location-within-two-steps
//! Magic (RULE-CATALOG-0575–0576, 1052, 1098, RULE-CATALOG-1853–1858).
//!
//! Ordinary Unravel chooses a location within two measured steps of the caster
//! and destroys every Undead minion and Artifact whose cell and region match.
//! A non-Undead minion at another offered location is left unwounded. An empty
//! offered location is a paid no-op. This is one composed fact, exclusive of
//! destroy-artifacts-and-auras-at-location and kill-mortal-minions-at-location.
//!
//! 1052 covers Unravel killing a Deathrite undead minion: the controller draws
//! a site and magic-resolved only appears after deathrite settlement. 1098
//! covers Unravel withheld while Deathrites wait for ordering, until the chain
//! drains.

#[path = "common/marked_death.rs"]
mod marked_death;

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn relic() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn undead() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "undead": true,
    })
}

fn deathrite_undead() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "undead": true,
    })
}

fn beast() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn unravel() -> Value {
    json!({
        "cardType": "magic",
        "destroyUndeadMinionsAndArtifactsAtLocationWithinTwoSteps": true,
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

fn rain_deathrite_minion() -> Value {
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

fn unravel_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-undead-relics-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-undead-relics-here-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-undead": undead(),
            "north-unravel": unravel(),
            "south-avatar": avatar(),
            "south-dummy": beast(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-undead",
                    "north-relic",
                    "north-unravel",
                    "north-beast",
                    "north-beast",
                    "north-beast",
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
    }))
}

fn unravel_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-undead-relics-here-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-undead-relics-here-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-undead": deathrite_undead(),
            "north-unravel": unravel(),
            "south-avatar": avatar(),
            "south-dummy": beast(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-undead",
                    "north-relic",
                    "north-unravel",
                    "north-beast",
                    "north-beast",
                    "north-beast",
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
    }))
}

fn unravel_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-undead-relics-here-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-undead-relics-here-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-undead": undead(),
            "north-unravel": unravel(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
            "south-undead": {
                "attack": 1,
                "cardType": "minion",
                "defense": 3,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                "undead": true,
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-unravel",
                    "north-unravel",
                    "north-unravel",
                    "north-unravel",
                    "north-undead",
                    "north-undead",
                    "north-relic",
                    "north-unravel",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-undead"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn unravel_multi_target_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-undead-relics-here-multi-target" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-undead-relics-here-multi-target-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-undead": undead(),
            "north-unravel": unravel(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
            "south-undead": {
                "attack": 1,
                "cardType": "minion",
                "defense": 3,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                "undead": true,
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-undead",
                    "north-relic",
                    "north-unravel",
                    "north-undead",
                    "north-relic",
                    "north-unravel",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-undead"; 6],
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
    let mut session = Session::new(encoded).expect("valid destroy-undead-relics-here session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn opening_caster_main(encoded: &str, caster: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid destroy-undead-relics-here session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    if caster == "south" {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
    }
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
    opening_spell_ids_for(encoded, "north")
}

fn opening_spell_ids_for(encoded: &str, seat: &str) -> Vec<String> {
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

fn seed_with(required: &[&str]) -> String {
    seed_with_manifest(required, unravel_manifest)
}

fn seed_with_deathrite(required: &[&str]) -> String {
    seed_with_manifest(required, unravel_deathrite_manifest)
}

fn seed_with_manifest(required: &[&str], manifest: impl Fn(u32) -> String) -> String {
    (575..575 + 256)
        .map(manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn south_plays_c1_and_ends(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn summon_at(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned instance identity")
        .to_owned()
}

fn artifact_at(session: &Session, card_id: &str, cell: &str) -> String {
    state(session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["cardId"] == card_id && artifact["location"] == cell)
        .expect("expected artifact at cell")["instanceId"]
        .as_str()
        .expect("artifact instance identity")
        .to_owned()
}

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn unravel_locations(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("unravel actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-unravel"
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

fn deathrite_unravel_manifest(seed: u32) -> String {
    let fixture = "destroy-undead-relics-here-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-unravel": unravel(),
            "south-avatar": avatar(),
            "south-minion": rain_deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-unravel",
                    "north-rain",
                    "north-relic",
                    "north-unravel",
                    "north-rain",
                    "north-relic",
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

fn mixed_unravel_deathrite_manifest(seed: u32) -> String {
    let mut manifest: Value = serde_json::from_str(&unravel_deathrite_manifest(seed))
        .expect("base mixed Unravel manifest");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    manifest["decks"]["north"]["spellbook"] = json!([
        "north-undead",
        "north-undead",
        "north-relic",
        "north-relic",
        "north-unravel",
        "north-beast",
        "north-beast",
    ]);
    finish_manifest(manifest)
}

fn mixed_north_control_manifest(seed: u32) -> String {
    let mut manifest: Value = serde_json::from_str(&mixed_unravel_deathrite_manifest(seed))
        .expect("North mixed Unravel manifest");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    manifest["cards"]["south-relic-control"] = manifest["cards"]["north-relic"].clone();
    manifest["decks"]["south"]["spellbook"] = json!([
        "south-relic-control",
        "south-dummy",
        "south-dummy",
        "south-dummy",
        "south-dummy",
        "south-dummy",
    ]);
    finish_manifest(manifest)
}

fn mixed_south_unravel_deathrite_manifest(seed: u32) -> String {
    let mut manifest: Value = serde_json::from_str(&mixed_unravel_deathrite_manifest(seed))
        .expect("North mixed Unravel manifest");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    for (north, south) in [
        ("north-undead", "south-undead"),
        ("north-relic", "south-relic"),
        ("north-unravel", "south-unravel"),
        ("north-beast", "south-beast"),
    ] {
        manifest["cards"][south] = manifest["cards"][north].clone();
    }
    manifest["cards"]
        .as_object_mut()
        .unwrap()
        .remove("south-dummy");
    manifest["cards"]["south-relic-control"] = manifest["cards"]["north-relic"].clone();
    manifest["decks"]["south"]["spellbook"] = json!([
        "south-undead",
        "south-undead",
        "south-relic",
        "south-relic",
        "south-relic-control",
        "south-unravel",
        "south-beast",
        "south-beast",
    ]);
    finish_manifest(manifest)
}

fn lethal_mixed_unravel_manifest(seed: u32) -> String {
    let mut manifest: Value = serde_json::from_str(&mixed_unravel_deathrite_manifest(seed))
        .expect("base mixed Unravel manifest");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    manifest["cards"]["north-undead"]
        .as_object_mut()
        .unwrap()
        .remove("deathriteDrawSite");
    manifest["cards"]["north-undead"]["deathriteDamageEachUnitHere"] = json!(1);
    manifest["cards"]["north-relic"] = json!({
        "cardType": "artifact",
        "grantsBearerLethal": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    finish_manifest(manifest)
}

fn static_power_loss_manifest(seed: u32) -> String {
    let mut manifest: Value = serde_json::from_str(&mixed_unravel_deathrite_manifest(seed))
        .expect("base mixed Unravel manifest");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    manifest["cards"]["north-bearer"] = json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    manifest["cards"]["north-servant"] = json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 3,
        "genesisDamageEachOtherUnitHere": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "undead": true,
    });
    manifest["cards"]["north-warded-undead"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "undead": true,
        "ward": true,
    });
    manifest["decks"]["north"]["spellbook"] = json!([
        "north-bearer",
        "north-relic",
        "north-unravel",
        "north-servant",
        "north-servant",
        "north-servant",
        "north-warded-undead",
    ]);
    let cards = manifest["cards"].as_object_mut().expect("cards object");
    cards.remove("north-beast");
    cards.remove("north-rain");
    cards.remove("north-undead");
    finish_manifest(manifest)
}

fn terminal_mixed_unravel_manifest(seed: u32) -> String {
    let mut manifest: Value = serde_json::from_str(&mixed_unravel_deathrite_manifest(seed))
        .expect("base mixed Unravel manifest");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    manifest["decks"]["north"]["atlas"] = json!(vec!["north-site"; 3]);
    finish_manifest(manifest)
}

fn north_has_unravel_rain_and_relic(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-unravel", "north-rain", "north-relic"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteUnravelSetup {
    deathrite_ids: [String; 2],
    relic_id: String,
    session: Session,
}

fn try_pending_deathrite_with_nearby_relic(encoded: &str) -> Option<PendingDeathriteUnravelSetup> {
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
    if !north_has_unravel_rain_and_relic(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C4"
    })?;
    let relic_id = artifact_at(&session, "north-relic", "C4");
    if unravel_locations(&session) != ["C4"] {
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
    Some(PendingDeathriteUnravelSetup {
        deathrite_ids,
        relic_id,
        session,
    })
}

fn deathrite_unravel_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_unravel_manifest)
        .find(|candidate| try_pending_deathrite_with_nearby_relic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Unravel Magic in hand")
}

fn mixed_deathrite_seed_with_two_undead_for(start: u32, caster: &str) -> String {
    let cell = if caster == "north" { "C4" } else { "C1" };
    (start..start + 2048)
        .map(|seed| match caster {
            "north" => mixed_north_control_manifest(seed),
            "south" => mixed_south_unravel_deathrite_manifest(seed),
            _ => unreachable!("only seats are valid caster values"),
        })
        .find(|candidate| {
            let undead = format!("{caster}-undead");
            let relic = format!("{caster}-relic");
            let unravel = format!("{caster}-unravel");
            let hand = opening_spell_ids_for(candidate, caster);
            if !hand.iter().any(|card| card == &undead)
                || !hand.iter().any(|card| card == &relic)
                || !hand.iter().any(|card| card == &unravel)
            {
                return false;
            }
            let mut session = opening_caster_main(candidate, caster);
            let Some((summoned, _)) = try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == undead
                    && descriptor["cell"] == cell
                    && descriptor["region"].is_null()
            }) else {
                return false;
            };
            let first = summoned["cardInstanceId"]
                .as_str()
                .expect("summoned instance identity")
                .to_owned();
            if try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == relic
                    && descriptor["bearer"]["instanceId"] == first
            })
            .is_none()
            {
                return false;
            }
            let control_card = if caster == "north" {
                "south-relic-control"
            } else {
                "north-relic"
            };
            let control_cell = if caster == "north" { "C1" } else { "C4" };
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            if caster == "north" {
                accept_where(&mut session, |descriptor| {
                    descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
                });
                accept_where(&mut session, |descriptor| {
                    descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
                });
            } else {
                accept_where(&mut session, |descriptor| {
                    descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
                });
                let _ = try_accept_where(&mut session, |descriptor| {
                    descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
                });
            }
            if try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == control_card
                    && descriptor["bearer"].is_null()
                    && descriptor["cell"] == control_cell
            })
            .is_none()
            {
                return false;
            }
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            for _ in 0..4 {
                let after_draw = opening_spell_ids_from_state_for(&state(&session), caster);
                if after_draw.iter().any(|card| card == &undead)
                    && after_draw.iter().any(|card| card == &relic)
                    && after_draw
                        .iter()
                        .any(|card| card == &format!("{caster}-beast"))
                {
                    return true;
                }
                pass_turn_to_caster_spellbook(&mut session, caster);
            }
            false
        })
        .expect("bounded seed with two Undead, two Artifacts, a control and Unravel")
}

fn lethal_mixed_seed(start: u32) -> String {
    (start..start + 2048)
        .map(lethal_mixed_unravel_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            if !["north-undead", "north-relic", "north-unravel"]
                .iter()
                .all(|required| hand.iter().any(|card| card == required))
            {
                return false;
            }
            let mut session = opening_main(candidate);
            let deathrite = summon_at(&mut session, "north-undead", "C4");
            if try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == "north-relic"
                    && descriptor["bearer"]["instanceId"] == deathrite
            })
            .is_none()
            {
                return false;
            }
            pass_turn_to_north_spellbook(&mut session);
            opening_spell_ids_from_state(&state(&session))
                .iter()
                .any(|card| card == "north-beast")
        })
        .expect("bounded seed with Deathrite, lethal Artifact, witness, and Unravel")
}

fn static_power_loss_seed(start: u32) -> String {
    (start..start + 2048)
        .map(static_power_loss_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            if !["north-bearer", "north-relic", "north-unravel"]
                .iter()
                .all(|required| hand.iter().any(|card| card == required))
            {
                return false;
            }
            let mut session = opening_main(candidate);
            let bearer = summon_at(&mut session, "north-bearer", "C4");
            if try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == "north-relic"
                    && descriptor["bearer"]["instanceId"] == bearer
            })
            .is_none()
            {
                return false;
            }
            pass_turn_to_north_spellbook(&mut session);
            let hand = opening_spell_ids_from_state(&state(&session));
            if !hand.iter().any(|card| card == "north-servant")
                || !hand.iter().any(|card| card == "north-warded-undead")
            {
                pass_turn_to_north_spellbook(&mut session);
            }
            opening_spell_ids_from_state(&state(&session))
                .iter()
                .any(|card| card == "north-servant")
                && opening_spell_ids_from_state(&state(&session))
                    .iter()
                    .any(|card| card == "north-warded-undead")
        })
        .expect("bounded seed for wounded power-granted Deathrite bearer")
}

fn terminal_mixed_seed(start: u32) -> String {
    (start..start + 2048)
        .map(terminal_mixed_unravel_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            if !["north-undead", "north-relic", "north-unravel"]
                .iter()
                .all(|required| hand.iter().any(|card| card == required))
            {
                return false;
            }
            let mut session = opening_main(candidate);
            let first = summon_at(&mut session, "north-undead", "C4");
            if try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == "north-relic"
                    && descriptor["bearer"]["instanceId"] == first
            })
            .is_none()
            {
                return false;
            }
            south_plays_c1_and_ends(&mut session);
            let after_draw = state(&session);
            opening_spell_ids_from_state(&after_draw)
                .iter()
                .any(|card| card == "north-undead")
                && after_draw["players"]["north"]["atlas"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        })
        .expect("bounded empty-Atlas seed with two Undead, Artifact and Unravel")
}

fn opening_spell_ids_from_state(snapshot: &Value) -> Vec<String> {
    opening_spell_ids_from_state_for(snapshot, "north")
}

fn opening_spell_ids_from_state_for(snapshot: &Value, seat: &str) -> Vec<String> {
    snapshot["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("spellbook hand")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(575..575 + 2048)
        .map(unravel_supplemental_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn unravel_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-unravel")
                .count()
        })
        .unwrap_or_default()
}

fn undead_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-undead")
                .count()
        })
        .unwrap_or_default()
}

fn cast_unravel(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unravel"
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
}

fn cast_unravel_with_checkpoint_parity(session: &mut Session, cell: &str) -> Receipt {
    let checkpoint = create_game_checkpoint(session).expect("pre-cast checkpoint");
    let checkpoint_bytes = serialize_game_checkpoint(&checkpoint).expect("checkpoint bytes");
    let before_replay = session.replay_value().expect("pre-cast replay");
    let before_hash = session.state_hash().expect("pre-cast state hash");
    let before_actions = session.legal_actions().expect("pre-cast legal actions");
    let before_north = session
        .public_view(Seat::North)
        .expect("North pre-cast view");
    let before_south = session
        .public_view(Seat::South)
        .expect("South pre-cast view");
    let action = before_actions
        .iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-unravel"
                && action.descriptor["targetLocation"]["cell"] == cell
        })
        .expect("engine-issued pre-cast Unravel action")
        .clone();
    let request = ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    };
    let StepResult::Accepted(receipt) = session.step(request.clone()).expect("Unravel cast") else {
        panic!("Unravel cast should be accepted")
    };

    let mut restored = resume_game_checkpoint(
        &parse_game_checkpoint(&checkpoint_bytes).expect("parsed pre-cast checkpoint"),
    )
    .expect("restore pre-cast checkpoint");
    assert_eq!(restored.replay_value().unwrap(), before_replay);
    assert_eq!(restored.state_hash().unwrap(), before_hash);
    assert_eq!(restored.legal_actions().unwrap(), before_actions);
    assert_eq!(restored.public_view(Seat::North).unwrap(), before_north);
    assert_eq!(restored.public_view(Seat::South).unwrap(), before_south);
    let restored_action = restored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.action_id == action.action_id)
        .expect("same engine-issued cast after restore");
    assert_eq!(restored_action.descriptor, action.descriptor);
    assert_eq!(restored_action.seat, action.seat);
    assert_eq!(restored_action.state_version, action.state_version);
    let StepResult::Accepted(restored_receipt) =
        restored.step(request).expect("restored Unravel cast")
    else {
        panic!("restored Unravel cast should be accepted")
    };
    assert_eq!(restored_receipt, receipt);
    assert_eq!(
        restored.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
    assert_eq!(
        restored.state_hash().unwrap(),
        session.state_hash().unwrap()
    );
    assert_eq!(restored.transcript(), session.transcript());
    assert_eq!(
        restored.legal_actions().unwrap(),
        session.legal_actions().unwrap()
    );
    assert_eq!(
        restored.public_view(Seat::North).unwrap(),
        session.public_view(Seat::North).unwrap()
    );
    assert_eq!(
        restored.public_view(Seat::South).unwrap(),
        session.public_view(Seat::South).unwrap()
    );
    assert!(restored.verify_replay().unwrap());
    receipt
}

fn cast_relic_at(session: &mut Session, cell: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == cell
    });
    artifact_at(session, "north-relic", cell)
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn pass_turn_to_caster_spellbook(session: &mut Session, caster: &str) {
    if caster == "north" {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        let _ = try_accept_where(session, |descriptor| descriptor["kind"] == "play-site");
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    } else {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        let _ = try_accept_where(session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cardId"] == "north-site"
        });
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
}

fn south_raids_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    let (nearby, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-undead"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    nearby["cardInstanceId"]
        .as_str()
        .expect("nearby enemy identity")
        .to_owned()
}

fn south_plays_c1_and_summons_undead(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let far_id = summon_at(session, "south-undead", "C1");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    far_id
}

fn unit_absent(snapshot: &Value, instance_id: &str) -> bool {
    snapshot["realm"]["units"]
        .as_array()
        .is_none_or(|units| units.iter().all(|unit| unit["instanceId"] != instance_id))
}

fn seed_with_two_unravel_spells_in_hand_after_setup(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = unravel_supplemental_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if hand.iter().filter(|card| *card == "north-undead").count() < 1
                || !hand.iter().any(|card| card == "north-unravel")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = summon_at(&mut session, "north-undead", "C4");
            (unravel_spells_in_hand(&state(&session)) >= 2).then_some(encoded)
        })
        .expect("bounded seed with two Unravel spells in hand after setup")
}

struct SecondUnravelKillSetup {
    second_undead: String,
    session: Session,
}

fn try_second_unravel_kill_prefix(encoded: &str) -> Option<SecondUnravelKillSetup> {
    let mut session = opening_main(encoded);
    let first_undead = summon_at(&mut session, "north-undead", "C4");
    cast_unravel(&mut session, "C4");
    if !cemetery_has(&state(&session), "north", &first_undead) {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    let snap = state(&session);
    if unravel_spells_in_hand(&snap) < 1 || undead_in_hand(&snap) < 1 {
        return None;
    }
    let second_undead = summon_at(&mut session, "north-undead", "C4");
    if !unravel_locations(&session).contains(&"C4".to_owned()) {
        return None;
    }
    Some(SecondUnravelKillSetup {
        second_undead,
        session,
    })
}

fn seed_for_second_unravel_kill(start: u32) -> String {
    (start..start + 8192)
        .chain(575..575 + 8192)
        .find_map(|seed| {
            let encoded = unravel_supplemental_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-undead")
                || !hand.iter().any(|card| card == "north-unravel")
            {
                return None;
            }
            try_second_unravel_kill_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Unravel kill setup")
}

fn seed_with_undead_and_relic_at_c4(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = unravel_multi_target_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-undead")
                || !hand.iter().any(|card| card == "north-relic")
                || !hand.iter().any(|card| card == "north-unravel")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = summon_at(&mut session, "north-undead", "C4");
            try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == "north-relic"
                    && descriptor["bearer"].is_null()
                    && descriptor["cell"] == "C4"
            })?;
            Some(encoded)
        })
        .expect("bounded seed reaching Undead and Artifact at C4")
}

#[test]
fn rule_catalog_0575_mixed_undead_artifact_destruction_completes_before_other_units_change() {
    let encoded = seed_with(&["north-unravel", "north-undead", "north-relic"]);
    let mut session = opening_main(&encoded);
    south_plays_c1_and_ends(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    let undead_id = summon_at(&mut session, "north-undead", "C3");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let relic_id = artifact_at(&session, "north-relic", "C3");
    let before = state(&session);
    assert_eq!(unit(&before, &undead_id)["location"], "C3");
    assert_eq!(
        realm_artifact(&before, &relic_id).expect("relic at C3")["location"],
        "C3"
    );
    assert_eq!(unit(&before, &beast_id)["location"], "C4");
    assert_eq!(unit(&before, &beast_id)["damage"], 0);
    assert!(!cemetery_has(&before, "north", &undead_id));
    assert!(!cemetery_has(&before, "north", &relic_id));
    assert_eq!(unravel_locations(&session), ["C3", "C4"]);

    let receipt = cast_unravel_with_checkpoint_parity(&mut session, "C3");
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "artifact-destroyed",
            "minion-died",
            "magic-resolved"
        ]
    );
    let after = state(&session);
    assert!(unit_absent(&after, &undead_id));
    assert!(cemetery_has(&after, "north", &undead_id));
    assert!(cemetery_has(&after, "north", &relic_id));
    assert_eq!(unit(&after, &beast_id)["damage"], 0);
    assert!(unravel_locations(&session).is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0576_destroy_undead_relics_here_empty_location_is_a_paid_noop() {
    let encoded = seed_with(&["north-unravel"]);
    let mut session = opening_main(&encoded);
    south_plays_c1_and_ends(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(unravel_locations(&session), ["C3", "C4"]);
    assert!(
        state(&session)["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unravel"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed"
                || event.event_type == "minion-died"
                || event.event_type == "artifact-destroyed")
    );
    assert!(
        state(&session)["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1052_mixed_destruction_deathrite_draw_precedes_magic_completion() {
    let encoded = (1052..1052 + 256)
        .map(unravel_deathrite_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|card| card == "north-undead")
                && hand.iter().any(|card| card == "north-relic")
                && hand.iter().any(|card| card == "north-unravel")
        })
        .unwrap_or_else(|| seed_with_deathrite(&["north-undead", "north-relic", "north-unravel"]));
    let mut session = opening_main(&encoded);
    south_plays_c1_and_ends(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let undead_id = summon_at(&mut session, "north-undead", "C3");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let relic_id = artifact_at(&session, "north-relic", "C3");
    let before = state(&session);
    assert_eq!(unit(&before, &undead_id)["location"], "C3");
    assert_eq!(
        realm_artifact(&before, &relic_id).expect("relic at C3")["location"],
        "C3"
    );
    assert!(!cemetery_has(&before, "north", &undead_id));
    assert!(!cemetery_has(&before, "north", &relic_id));

    let receipt = cast_unravel_with_checkpoint_parity(&mut session, "C3");
    let types = event_types(&receipt);
    let artifact = types
        .iter()
        .position(|event| *event == "artifact-destroyed")
        .unwrap();
    let death = types
        .iter()
        .position(|event| *event == "minion-died")
        .unwrap();
    let draw = types
        .iter()
        .position(|event| *event == "site-drawn")
        .unwrap();
    let resolved = types
        .iter()
        .position(|event| *event == "magic-resolved")
        .unwrap();
    assert!(
        artifact < draw && draw < death && death < resolved,
        "{types:?}"
    );
    assert_eq!(
        types
            .iter()
            .filter(|event| **event == "magic-resolved")
            .count(),
        1
    );
    let after = state(&session);
    assert!(unit_absent(&after, &undead_id));
    assert!(cemetery_has(&after, "north", &undead_id));
    assert!(cemetery_has(&after, "north", &relic_id));
    let magic_id = receipt.events[0].payload["instanceId"]
        .as_str()
        .expect("cast Magic instance identity");
    assert!(cemetery_has(&after, "north", magic_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1098_destroy_undead_relics_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_unravel_seed_with(1098);
    let mut setup = try_pending_deathrite_with_nearby_relic(&encoded)
        .expect("complete destroy-undead-relics Deathrite withheld setup");
    let relic_id = setup.relic_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    marked_death::assert_live_marked_before_cemetery(&paused, &deathrite_ids);
    assert_eq!(
        realm_artifact(&paused, &relic_id).expect("nearby relic")["location"],
        "C4"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(unravel_locations(session).is_empty());

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
    assert_eq!(
        realm_artifact(&resumed, &relic_id).expect("nearby relic remains")["location"],
        "C4"
    );
    assert_eq!(unravel_locations(session), ["C4"]);

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unravel"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-destroyed", "magic-resolved"]
    );
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "artifact-destroyed")
        .expect("artifact destruction");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], relic_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert!(realm_artifact(&state(session), &relic_id).is_none());
    assert_exact_replay(session);
}

#[test]
#[allow(clippy::too_many_lines)] // Keep both ordering branches and checkpoint comparisons in one proof.
fn mixed_destruction_finishes_artifacts_before_ordered_deathrites_and_holds_magic() {
    for caster in ["north", "south"] {
        let encoded = mixed_deathrite_seed_with_two_undead_for(2600, caster);
        let cell = if caster == "north" { "C4" } else { "C1" };
        let undead_card = format!("{caster}-undead");
        let relic_card = format!("{caster}-relic");
        let magic_card = format!("{caster}-unravel");
        let beast_card = format!("{caster}-beast");
        let control_card = if caster == "north" {
            "south-relic-control"
        } else {
            "north-relic"
        };
        let control_cell = if caster == "north" { "C1" } else { "C4" };
        let mut setup = opening_caster_main(&encoded, caster);
        let first = summon_at(&mut setup, &undead_card, cell);
        accept_where(&mut setup, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == relic_card
                && descriptor["bearer"]["instanceId"] == first
        });
        accept_where(&mut setup, |descriptor| descriptor["kind"] == "end-turn");
        if caster == "north" {
            accept_where(&mut setup, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            accept_where(&mut setup, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
        } else {
            accept_where(&mut setup, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
            });
            let _ = try_accept_where(&mut setup, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
            });
        }
        let (control_descriptor, _) = accept_where(&mut setup, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == control_card
                && descriptor["bearer"].is_null()
                && descriptor["cell"] == control_cell
        });
        let control_artifact_id = control_descriptor["cardInstanceId"]
            .as_str()
            .expect("non-target control Artifact identity")
            .to_owned();
        accept_where(&mut setup, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut setup, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        for _ in 0..4 {
            let hand = opening_spell_ids_from_state_for(&state(&setup), caster);
            if hand.iter().any(|card| card == &undead_card)
                && hand.iter().any(|card| card == &relic_card)
                && hand.iter().any(|card| card == &beast_card)
            {
                break;
            }
            pass_turn_to_caster_spellbook(&mut setup, caster);
        }
        let second = summon_at(&mut setup, &undead_card, cell);
        let control = summon_at(&mut setup, &beast_card, cell);
        accept_where(&mut setup, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == relic_card
                && descriptor["bearer"].is_null()
                && descriptor["cell"] == cell
        });
        let before = state(&setup);
        let control_before = realm_artifact(&before, &control_artifact_id)
            .expect("remote control Artifact remains in the realm")
            .clone();
        let artifact_ids = before["realm"]["artifacts"]
            .as_array()
            .expect("carried and loose Artifacts")
            .iter()
            .filter(|artifact| artifact["instanceId"] != control_artifact_id)
            .map(|artifact| {
                artifact["instanceId"]
                    .as_str()
                    .expect("Artifact identity")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(artifact_ids.len(), 2);
        assert_eq!(before["realm"]["artifacts"].as_array().unwrap().len(), 3);
        assert!(
            before["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["bearer"]["instanceId"] == first)
        );
        assert!(
            before["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["bearer"].is_null())
        );
        let magic_before = before["players"][caster]["hand"]["spellbook"]
            .as_array()
            .expect("caster spellbook")
            .iter()
            .find(|card| card["cardId"] == magic_card)
            .expect("Unravel in hand")["instanceId"]
            .as_str()
            .expect("Unravel identity")
            .to_owned();

        let (_, cast) = accept_where(&mut setup, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == magic_card
                && descriptor["targetLocation"]["cell"] == cell
        });
        assert_eq!(
            event_types(&cast),
            [
                "magic-cast",
                "minion-killed",
                "minion-killed",
                "artifact-destroyed",
                "artifact-destroyed",
            ]
        );
        let paused = state(&setup);
        assert_eq!(paused["phase"], "trigger-order");
        for id in [&first, &second] {
            assert_eq!(unit(&paused, id)["deathMarked"], true);
            assert!(!cemetery_has(&paused, caster, id));
        }
        assert_eq!(unit(&paused, &control), unit(&before, &control));
        for artifact_id in &artifact_ids {
            assert!(realm_artifact(&paused, artifact_id).is_none());
            assert!(cemetery_has(&paused, caster, artifact_id));
        }
        assert!(!cemetery_has(&paused, caster, &magic_before));
        let continuation = &paused["pendingDeathrites"]["continuation"];
        assert_eq!(continuation["kind"], "magic-resolved");
        assert_eq!(continuation["heldCard"]["instanceId"], magic_before);

        let checkpoint = parse_game_checkpoint(
            &serialize_game_checkpoint(&create_game_checkpoint(&setup).unwrap()).unwrap(),
        )
        .unwrap();
        let parent = setup.replay_value().unwrap();
        let mut branches = Vec::new();
        for first_source in [&first, &second] {
            let mut branch = resume_game_checkpoint(&checkpoint).unwrap();
            accept_where(&mut branch, |descriptor| {
                descriptor["kind"] == "order-triggers"
                    && descriptor["sourceInstanceId"] == first_source.as_str()
            });
            while state(&branch)["phase"] == "trigger-order" {
                let action = branch
                    .legal_actions()
                    .unwrap()
                    .into_iter()
                    .find(|action| action.descriptor["kind"] == "order-triggers")
                    .expect("remaining order action");
                branch
                    .step(ActionRequest {
                        action_id: action.action_id.to_string(),
                        seat: action.seat,
                        state_version: action.state_version,
                    })
                    .expect("complete Deathrite ordering");
            }
            let after = state(&branch);
            assert!(after["pendingDeathrites"].is_null());
            assert!(unit_absent(&after, &first));
            assert!(unit_absent(&after, &second));
            assert_eq!(unit(&after, &control), unit(&before, &control));
            assert!(cemetery_has(&after, caster, &first));
            assert!(cemetery_has(&after, caster, &second));
            for artifact_id in &artifact_ids {
                assert!(cemetery_has(&after, caster, artifact_id));
            }
            assert_eq!(
                realm_artifact(&after, &control_artifact_id),
                Some(&control_before)
            );
            assert!(!cemetery_has(
                &after,
                if caster == "north" { "south" } else { "north" },
                &control_artifact_id
            ));
            assert!(cemetery_has(&after, caster, &magic_before));
            let draw_sources = branch
                .transcript()
                .iter()
                .flat_map(|receipt| receipt.events.iter())
                .filter(|event| event.event_type == "site-drawn")
                .map(|event| {
                    event.payload["sourceInstanceId"]
                        .as_str()
                        .unwrap()
                        .to_owned()
                })
                .collect::<Vec<_>>();
            let other_source = if first_source == &first {
                &second
            } else {
                &first
            };
            assert_eq!(draw_sources, [first_source.clone(), other_source.clone()]);
            let resolved = branch
                .transcript()
                .iter()
                .flat_map(|receipt| receipt.events.iter())
                .filter(|event| event.event_type == "magic-resolved")
                .count();
            assert_eq!(resolved, 1);
            assert_exact_replay(&branch);
            branches.push(branch.replay_value().unwrap());
        }
        assert_eq!(setup.replay_value().unwrap(), parent);
        assert_ne!(branches[0], branches[1]);
    }
}

#[test]
fn mixed_destruction_removes_carried_lethal_before_damage_deathrite() {
    let encoded = lethal_mixed_seed(4800);
    let mut session = opening_main(&encoded);
    let deathrite = summon_at(&mut session, "north-undead", "C4");
    let no_artifact_checkpoint = serialize_game_checkpoint(
        &create_game_checkpoint(&session).expect("pre-Artifact control checkpoint"),
    )
    .expect("pre-Artifact control checkpoint bytes");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"]["instanceId"] == deathrite
    });
    let carried_id = state(&session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("carried lethal Artifact")
        .to_owned();
    south_plays_c1_and_ends(&mut session);
    let witness = summon_at(&mut session, "north-beast", "C4");
    let receipt = cast_unravel(&mut session, "C4");
    let types = event_types(&receipt);
    let artifact = types
        .iter()
        .position(|event| *event == "artifact-destroyed")
        .expect("carried Artifact departure");
    let damage = types
        .iter()
        .position(|event| *event == "damage-dealt")
        .expect("Deathrite damage");
    assert!(artifact < damage, "{types:?}");
    let after = state(&session);
    assert!(unit_absent(&after, &deathrite));
    assert!(cemetery_has(&after, "north", &deathrite));
    assert!(cemetery_has(&after, "north", &carried_id));
    assert_eq!(unit(&after, &witness)["damage"], 1);
    assert!(!cemetery_has(&after, "north", &witness));
    assert_exact_replay(&session);

    let mut no_artifact = resume_game_checkpoint(
        &parse_game_checkpoint(&no_artifact_checkpoint).expect("parse control checkpoint"),
    )
    .expect("restore same Deathrite operands without Artifact");
    south_plays_c1_and_ends(&mut no_artifact);
    let control_witness = summon_at(&mut no_artifact, "north-beast", "C4");
    let control_receipt = cast_unravel(&mut no_artifact, "C4");
    assert!(event_types(&control_receipt).contains(&"deathrite-damage-allocated"));
    assert!(unit_absent(&state(&no_artifact), &deathrite));
    assert_eq!(unit(&state(&no_artifact), &control_witness)["damage"], 1);
    assert!(!cemetery_has(
        &state(&no_artifact),
        "north",
        &control_witness
    ));
    assert!(!event_types(&control_receipt).contains(&"artifact-destroyed"));
    assert_exact_replay(&no_artifact);
}

#[test]
#[allow(clippy::too_many_lines)] // Keep both ordering branches and checkpoint comparisons in one proof.
fn mixed_destruction_collects_power_loss_death_before_trigger_dispatch() {
    let encoded = static_power_loss_seed(7200);
    let mut session = opening_main(&encoded);
    let bearer = summon_at(&mut session, "north-bearer", "C4");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"]["instanceId"] == bearer
    });
    south_plays_c1_and_ends(&mut session);
    pass_turn_to_north_spellbook(&mut session);
    let hand = opening_spell_ids_from_state(&state(&session));
    if !hand.iter().any(|card| card == "north-servant")
        || !hand.iter().any(|card| card == "north-warded-undead")
    {
        pass_turn_to_north_spellbook(&mut session);
    }
    let direct = summon_at(&mut session, "north-servant", "C4");
    let warded = summon_at(&mut session, "north-warded-undead", "C4");
    assert_eq!(unit(&state(&session), &bearer)["damage"], 1);
    assert_eq!(unit(&state(&session), &warded)["warded"], true);
    let artifact_id = state(&session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("carried Power Artifact")
        .to_owned();

    let receipt = cast_unravel(&mut session, "C4");
    let types = event_types(&receipt);
    let artifact = types
        .iter()
        .position(|event| *event == "artifact-destroyed")
        .expect("complete Artifact departure");
    assert!(artifact < types.len());
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "ward-broken" && event.payload["instanceId"] == warded
    }));
    let paused = state(&session);
    assert_eq!(paused["phase"], "trigger-order");
    for id in [&direct, &bearer] {
        assert_eq!(unit(&paused, id)["deathMarked"], true);
        assert!(!cemetery_has(&paused, "north", id));
    }
    assert_eq!(unit(&paused, &warded)["warded"], false);
    assert!(!cemetery_has(&paused, "north", &warded));
    assert!(cemetery_has(&paused, "north", &artifact_id));
    let order_sources = session
        .legal_actions()
        .expect("post-event Deathrite sources")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .filter_map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        order_sources,
        [direct.clone(), bearer.clone()].into_iter().collect()
    );

    let checkpoint = parse_game_checkpoint(
        &serialize_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap(),
    )
    .unwrap();
    let parent = session.replay_value().unwrap();
    for first_source in [&direct, &bearer] {
        let mut branch = resume_game_checkpoint(&checkpoint).unwrap();
        accept_where(&mut branch, |descriptor| {
            descriptor["kind"] == "order-triggers"
                && descriptor["sourceInstanceId"] == first_source.as_str()
        });
        while state(&branch)["phase"] == "trigger-order" {
            let order = branch
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| action.descriptor["kind"] == "order-triggers")
                .expect("remaining Deathrite order");
            branch
                .step(ActionRequest {
                    action_id: order.action_id.to_string(),
                    seat: order.seat,
                    state_version: order.state_version,
                })
                .expect("complete branch order");
        }
        let after = state(&branch);
        assert!(unit_absent(&after, &direct));
        assert!(unit_absent(&after, &bearer));
        assert_eq!(unit(&after, &warded)["warded"], false);
        assert!(!cemetery_has(&after, "north", &warded));
        assert!(cemetery_has(&after, "north", &direct));
        assert!(cemetery_has(&after, "north", &bearer));
        assert!(
            branch
                .transcript()
                .iter()
                .flat_map(|receipt| &receipt.events)
                .any(|event| event.event_type == "magic-resolved")
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(session.replay_value().unwrap(), parent);
}

#[test]
#[allow(clippy::too_many_lines)] // Keep both ordering branches and checkpoint comparisons in one proof.
fn mixed_destruction_terminal_deathrite_retains_marked_bodies_and_magic() {
    let encoded = terminal_mixed_seed(9400);
    let mut session = opening_main(&encoded);
    let first = summon_at(&mut session, "north-undead", "C4");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"]["instanceId"] == first
    });
    south_plays_c1_and_ends(&mut session);
    let second = summon_at(&mut session, "north-undead", "C4");
    let before = state(&session);
    assert!(
        before["players"]["north"]["atlas"]
            .as_array()
            .expect("North Atlas")
            .is_empty()
    );
    let magic_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North spellbook hand")
        .iter()
        .find(|card| card["cardId"] == "north-unravel")
        .expect("Unravel in hand")["instanceId"]
        .as_str()
        .expect("Unravel instance identity")
        .to_owned();
    let artifact_id = state(&session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("carried Artifact identity")
        .to_owned();

    let receipt = cast_unravel(&mut session, "C4");
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-killed",
            "artifact-destroyed"
        ]
    );
    let paused = state(&session);
    assert_eq!(paused["phase"], "trigger-order");
    for id in [&first, &second] {
        assert_eq!(unit(&paused, id)["deathMarked"], true);
        assert!(!unit_absent(&paused, id));
        assert!(!cemetery_has(&paused, "north", id));
    }
    assert!(cemetery_has(&paused, "north", &artifact_id));
    assert!(!cemetery_has(&paused, "north", &magic_id));
    assert_eq!(
        paused["pendingDeathrites"]["continuation"]["heldCard"]["instanceId"],
        magic_id
    );
    let first_sources: std::collections::BTreeSet<_> = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .filter_map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect();
    assert_eq!(
        first_sources,
        [first.clone(), second.clone()].into_iter().collect()
    );
    let pause_checkpoint = parse_game_checkpoint(
        &serialize_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap(),
    )
    .unwrap();
    let parent = session.replay_value().unwrap();

    for first_source in [&first, &second] {
        let mut branch = resume_game_checkpoint(&pause_checkpoint).unwrap();
        let pre_order = branch.replay_value().unwrap();
        let pre_order_actions = branch.legal_actions().unwrap();
        let pre_north = branch
            .public_view(sorcery_engine::contract::Seat::North)
            .unwrap();
        let pre_south = branch
            .public_view(sorcery_engine::contract::Seat::South)
            .unwrap();
        let action = pre_order_actions
            .iter()
            .find(|action| {
                action.descriptor["kind"] == "order-triggers"
                    && action.descriptor["sourceInstanceId"] == first_source.as_str()
            })
            .expect("issued selected first Deathrite order")
            .clone();
        let StepResult::Accepted(first_order_receipt) = branch
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .expect("resolve selected first DrawSite source")
        else {
            panic!("first order action should be accepted")
        };
        let terminal = state(&branch);
        assert_eq!(terminal["phase"], "terminal");
        for id in [&first, &second] {
            assert_eq!(unit(&terminal, id)["deathMarked"], true);
            assert!(!unit_absent(&terminal, id));
            assert!(!cemetery_has(&terminal, "north", id));
        }
        assert!(cemetery_has(&terminal, "north", &artifact_id));
        assert!(!cemetery_has(&terminal, "north", &magic_id));
        assert_eq!(
            terminal["pendingDeathrites"]["continuation"]["heldCard"]["instanceId"],
            magic_id
        );
        assert_eq!(
            first_order_receipt
                .events
                .iter()
                .filter(|event| event.event_type == "game-ended")
                .count(),
            1
        );
        assert!(!first_order_receipt.events.iter().any(|event| {
            event.event_type == "site-drawn"
                || event.event_type == "minion-died"
                || event.event_type == "magic-resolved"
        }));
        assert_exact_replay(&branch);

        let terminal_checkpoint = create_game_checkpoint(&branch).expect("terminal checkpoint");
        let terminal_restored = resume_game_checkpoint(
            &parse_game_checkpoint(&serialize_game_checkpoint(&terminal_checkpoint).unwrap())
                .unwrap(),
        )
        .expect("resume terminal checkpoint");
        assert_eq!(
            terminal_restored.replay_value().unwrap(),
            branch.replay_value().unwrap()
        );
        assert_eq!(terminal_restored.transcript(), branch.transcript());
        assert!(terminal_restored.verify_replay().unwrap());

        let mut replay_branch = resume_game_checkpoint(&pause_checkpoint).unwrap();
        let replay_action = replay_branch
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.action_id == action.action_id)
            .expect("same issued order from restored pause checkpoint");
        assert_eq!(replay_action.descriptor, action.descriptor);
        let StepResult::Accepted(replay_receipt) = replay_branch
            .step(ActionRequest {
                action_id: replay_action.action_id.to_string(),
                seat: replay_action.seat,
                state_version: replay_action.state_version,
            })
            .expect("replayed chosen order")
        else {
            panic!("restored order should be accepted")
        };
        assert_eq!(replay_receipt, first_order_receipt);
        assert_eq!(
            replay_branch.replay_value().unwrap(),
            branch.replay_value().unwrap()
        );
        assert_eq!(
            replay_branch.legal_actions().unwrap(),
            branch.legal_actions().unwrap()
        );
        assert_eq!(
            replay_branch
                .public_view(sorcery_engine::contract::Seat::North)
                .unwrap(),
            branch
                .public_view(sorcery_engine::contract::Seat::North)
                .unwrap()
        );
        assert_eq!(
            replay_branch
                .public_view(sorcery_engine::contract::Seat::South)
                .unwrap(),
            branch
                .public_view(sorcery_engine::contract::Seat::South)
                .unwrap()
        );
        assert_eq!(pre_order["state"]["phase"], "trigger-order");
        assert_eq!(
            pre_north,
            session
                .public_view(sorcery_engine::contract::Seat::North)
                .unwrap()
        );
        assert_eq!(
            pre_south,
            session
                .public_view(sorcery_engine::contract::Seat::South)
                .unwrap()
        );
    }
    assert_eq!(session.replay_value().unwrap(), parent);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1853_mixed_destruction_cemetery_persists_across_turns() {
    let encoded = seed_with_start(1853, &["north-undead", "north-relic", "north-unravel"]);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let relic_id = cast_relic_at(&mut session, "C4");
    let before = state(&session);
    assert_eq!(unit(&before, &undead_id)["location"], "C4");
    assert!(realm_artifact(&before, &relic_id).is_some());
    assert!(unravel_locations(&session).contains(&"C4".to_owned()));
    let receipt = cast_unravel_with_checkpoint_parity(&mut session, "C4");
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "artifact-destroyed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1854_second_unravel_without_an_undead_or_artifact_is_still_a_paid_noop() {
    let encoded = seed_with_two_unravel_spells_in_hand_after_setup(1854);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let first = cast_unravel(&mut session, "C4");
    assert!(event_types(&first).contains(&"minion-killed"));
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    let second = cast_unravel(&mut session, "C4");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!second.events.iter().any(|event| {
        event.event_type == "minion-killed"
            || event.event_type == "minion-died"
            || event.event_type == "artifact-destroyed"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1855_second_unravel_kills_a_newly_arrived_undead_at_the_same_cell() {
    let encoded = seed_with_two_unravel_spells_in_hand_after_setup(1855);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    cast_unravel(&mut session, "C4");
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    let nearby_id = south_raids_c4(&mut session);
    let killed = cast_unravel(&mut session, "C4");
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["instanceId"], nearby_id);
    assert!(unit_absent(&state(&session), &nearby_id));
    assert!(cemetery_has(&state(&session), "south", &nearby_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1856_mixed_destruction_includes_the_complete_local_cohort() {
    let encoded = seed_with_undead_and_relic_at_c4(1856);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let relic_id = cast_relic_at(&mut session, "C4");
    let before = state(&session);
    assert_eq!(unit(&before, &undead_id)["location"], "C4");
    assert!(realm_artifact(&before, &relic_id).is_some());
    let receipt = cast_unravel_with_checkpoint_parity(&mut session, "C4");
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "artifact-destroyed",
            "minion-died",
            "magic-resolved"
        ]
    );
    let after = state(&session);
    assert!(unit_absent(&after, &undead_id));
    assert!(cemetery_has(&after, "north", &undead_id));
    assert!(cemetery_has(&after, "north", &relic_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1857_mixed_destruction_preserves_far_undead() {
    let encoded = seed_with_start(1857, &["north-undead", "north-relic", "north-unravel"]);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let relic_id = cast_relic_at(&mut session, "C4");
    let far_id = south_plays_c1_and_summons_undead(&mut session);
    let before = state(&session);
    assert_eq!(unit(&before, &undead_id)["location"], "C4");
    assert!(realm_artifact(&before, &relic_id).is_some());
    assert_eq!(unit(&before, &far_id)["damage"], 0);
    assert!(!cemetery_has(&before, "north", &undead_id));
    assert!(!cemetery_has(&before, "north", &relic_id));
    assert!(!cemetery_has(&before, "south", &far_id));
    let receipt = cast_unravel_with_checkpoint_parity(&mut session, "C4");
    assert!(event_types(&receipt).contains(&"artifact-destroyed"));
    let after = state(&session);
    assert!(unit_absent(&after, &undead_id));
    assert!(cemetery_has(&after, "north", &undead_id));
    assert!(cemetery_has(&after, "north", &relic_id));
    assert_eq!(unit(&after, &far_id), unit(&before, &far_id));
    assert!(!cemetery_has(&after, "south", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1858_second_unravel_kills_a_newly_summoned_undead() {
    let encoded = seed_for_second_unravel_kill(1858);
    let SecondUnravelKillSetup {
        mut session,
        second_undead,
    } = try_second_unravel_kill_prefix(&encoded).expect("second Unravel kill prefix");
    let killed = cast_unravel(&mut session, "C4");
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["instanceId"], second_undead);
    assert!(unit_absent(&state(&session), &second_undead));
    assert!(cemetery_has(&state(&session), "north", &second_undead));
    assert_exact_replay(&session);
}
