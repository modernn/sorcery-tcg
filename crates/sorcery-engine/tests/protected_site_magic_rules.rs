//! Direct proofs for protected-site Craterize Magic (RULE-CATALOG-0657–0658, 1082).
//!
//! Plain destroy-site and return-site Magic already have dedicated protected
//! edges at RULE-CATALOG-0624 and RULE-CATALOG-0626 (from `magic_rules.rs` 0208
//! and 0210). Those spells deal no damage. Craterize-style Magic still pays
//! its Atlas discard and applies its damage grid when the target cannot be
//! moved, destroyed, or modified: the site stays, the occupant is still hit,
//! and the spell still resolves.
//!
//! 1082 covers Craterize killing a Deathrite minion on the target site: the
//! controller draws a site and magic-resolved only appears after deathrite
//! settlement.

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

fn site(protected: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
    });
    if protected {
        value["cannotBeMovedDestroyedOrModified"] = json!(true);
    }
    value
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
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

fn bearer_artifact() -> Value {
    json!({
        "atEndOfEachTurnSiteControllerLosesLife": 1,
        "cardType": "artifact",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn crater_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageUnitsAboveAndBelowTargetSiteByManhattanDistance": [1, 1, 1, 1, 1],
        "destroyTargetSite": true,
        "discardSiteAsAdditionalCost": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn ward_nearby_site_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "wardNearbyMinionOrSite": true,
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn crater_manifest(seed: u32, protected: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "protected-site-crater" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-protected-site-crater-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-crater": crater_spell(),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(protected),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-crater"; 6],
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

fn crater_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "protected-site-crater-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-protected-site-crater-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-crater": crater_spell(),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(false),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-crater"; 6],
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

fn crater_carried_artifact_manifest(seed: u32) -> String {
    let mut value: Value = serde_json::from_str(&crater_deathrite_manifest(seed)).unwrap();
    value.as_object_mut().unwrap().remove("manifestId");
    value["cards"]["south-artifact"] = bearer_artifact();
    value["decks"]["south"]["spellbook"] = json!([
        "south-minion",
        "south-minion",
        "south-minion",
        "south-minion",
        "south-artifact",
        "south-artifact",
    ]);
    finish_manifest(value)
}

fn mirrored_crater_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "protected-site-crater-mirrored-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-protected-site-crater-mirrored-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-minion": deathrite_minion(),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-crater": crater_spell(),
            "south-site": site(false),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-crater"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "south",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn mirrored_crater_carried_artifact_manifest(seed: u32) -> String {
    let mut value: Value = serde_json::from_str(&mirrored_crater_deathrite_manifest(seed)).unwrap();
    value.as_object_mut().unwrap().remove("manifestId");
    value["cards"]["north-artifact"] = bearer_artifact();
    value["decks"]["north"]["spellbook"] = json!([
        "north-minion",
        "north-minion",
        "north-minion",
        "north-minion",
        "north-artifact",
        "north-artifact",
    ]);
    finish_manifest(value)
}

fn seed_with_south_crater(start: u32) -> String {
    (start..start + 256)
        .map(mirrored_crater_deathrite_manifest)
        .find(|candidate| {
            let session = Session::new(candidate).expect("mirrored Crater candidate");
            state(&session)["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["cardId"] == "south-crater")
        })
        .expect("bounded seed with South Crater in opening hand")
}

fn crater_site_ward_manifest(seed: u32) -> String {
    let mut value: Value =
        serde_json::from_str(&crater_deathrite_manifest(seed)).expect("base Crater manifest");
    value["cards"]["north-bless"] = ward_nearby_site_spell();
    value["cards"]["south-bless"] = ward_nearby_site_spell();
    value["decks"]["north"]["spellbook"] = json!([
        "north-crater",
        "north-crater",
        "north-crater",
        "north-bless",
        "north-bless",
        "north-bless",
    ]);
    value["decks"]["south"]["spellbook"] = json!([
        "south-minion",
        "south-bless",
        "south-bless",
        "south-bless",
        "south-bless",
        "south-bless",
    ]);
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).expect("warded Crater fixture identity"));
    canonical_json(&value).expect("canonical warded Crater fixture")
}

fn south_crater_site_ward_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "south-crater-site-ward" }))
                .expect("synthetic South Crater identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-south-crater-site-ward-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bless": ward_nearby_site_spell(),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-bless": ward_nearby_site_spell(),
            "south-crater": crater_spell(),
            "south-site": site(false),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bless"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": ["south-crater", "south-crater", "south-crater",
                    "south-bless", "south-bless", "south-bless"],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn crater_water_region_manifest(seed: u32) -> String {
    let mut value: Value =
        serde_json::from_str(&crater_deathrite_manifest(seed)).expect("base Crater manifest");
    value.as_object_mut().unwrap().remove("manifestId");
    value["cards"]["south-site"]["elements"] = json!(["water"]);
    value["cards"]["south-minion"] = json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "ward": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["cards"]["south-dual"] = json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "takesLessDamage": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["cards"]["south-surface"] = json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["cards"]["south-artifact"] = json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["decks"]["south"]["atlas"] = json!(vec!["south-site"; 6]);
    value["decks"]["south"]["spellbook"] = json!([
        "south-minion",
        "south-minion",
        "south-dual",
        "south-dual",
        "south-surface",
        "south-artifact",
        "south-artifact",
    ]);
    value["manifestId"] = json!(identity_hash(&value).expect("Water Crater identity"));
    canonical_json(&value).expect("canonical Water Crater fixture")
}

fn seed_with_water_crater(start: u32) -> String {
    (start..start + 512)
        .map(crater_water_region_manifest)
        .find(|candidate| {
            let mut session = opening_main(candidate);
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            let snapshot = state(&session);
            let south_hand = snapshot["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            let has_opening = opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-crater")
                && south_hand
                    .iter()
                    .any(|card| card["cardId"] == "south-minion")
                && south_hand.iter().any(|card| card["cardId"] == "south-dual")
                && south_hand
                    .iter()
                    .any(|card| card["cardId"] == "south-surface")
                && south_hand
                    .iter()
                    .any(|card| card["cardId"] == "south-artifact");
            if !has_opening {
                return false;
            }
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
            let (submerge, _) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "south-minion"
                    && descriptor["cell"] == "C1"
                    && descriptor["region"] == "underwater"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "south-dual"
                    && descriptor["cell"] == "C1"
                    && descriptor["region"] == "underwater"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "south-surface"
                    && descriptor["cell"] == "C1"
                    && descriptor["region"].is_null()
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == "south-artifact"
                    && descriptor["bearer"]["instanceId"] == submerge["cardInstanceId"]
            });
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            state(&session)["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["cardId"] == "south-artifact")
        })
        .expect("bounded seed with Crater, Submerge, and dual-region minions")
}

fn swap_seat_words(value: &mut Value) {
    match value {
        Value::String(text) => {
            *text = text
                .replace("north", "__seat_swap__")
                .replace("south", "north")
                .replace("__seat_swap__", "south");
        }
        Value::Array(values) => values.iter_mut().for_each(swap_seat_words),
        Value::Object(object) => {
            let old = std::mem::take(object);
            for (key, mut value) in old {
                swap_seat_words(&mut value);
                let key = key
                    .replace("north", "__seat_swap__")
                    .replace("south", "north")
                    .replace("__seat_swap__", "south");
                object.insert(key, value);
            }
        }
        _ => {}
    }
}

fn mirrored_water_crater_manifest(seed: u32) -> String {
    let mut value: Value = serde_json::from_str(&crater_water_region_manifest(seed))
        .expect("base Water Crater manifest");
    value.as_object_mut().unwrap().remove("manifestId");
    swap_seat_words(&mut value);
    finish_manifest(value)
}

fn opening_mirrored_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid mirrored Water Crater session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    session
}

fn seed_with_mirrored_water_crater(start: u32) -> String {
    (start..start + 512)
        .map(mirrored_water_crater_manifest)
        .find(|candidate| {
            let mut session = opening_mirrored_main(candidate);
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            let snapshot = state(&session);
            let north_hand = snapshot["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            let has_opening = [
                "north-minion",
                "north-dual",
                "north-surface",
                "north-artifact",
            ]
            .iter()
            .all(|wanted| north_hand.iter().any(|card| card["cardId"] == *wanted))
                && snapshot["players"]["south"]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["cardId"] == "south-crater");
            if !has_opening {
                return false;
            }
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
            });
            let (submerge, _) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "north-minion"
                    && descriptor["cell"] == "C4"
                    && descriptor["region"] == "underwater"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "north-dual"
                    && descriptor["cell"] == "C4"
                    && descriptor["region"] == "underwater"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "north-surface"
                    && descriptor["cell"] == "C4"
                    && descriptor["region"].is_null()
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == "north-artifact"
                    && descriptor["bearer"]["instanceId"] == submerge["cardInstanceId"]
            });
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            state(&session)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["cardId"] == "north-artifact")
        })
        .expect("bounded seed with mirrored Water Crater cards in hand")
}

fn seed_with_crater_and_ward(start: u32) -> String {
    (start..start + 256)
        .map(crater_site_ward_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|card| card == "north-crater")
                && hand.iter().any(|card| card == "north-bless")
        })
        .expect("bounded seed with Crater and site-Ward Magic")
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid protected-site crater session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn opening_south_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid South-opening Crater session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
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

fn seed_with(protected: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| crater_manifest(seed, protected))
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-crater")
        })
        .expect("bounded seed with Craterize Magic in the opening hand")
}

fn seed_with_deathrite(start: u32) -> String {
    (start..start + 256)
        .map(crater_deathrite_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-crater")
        })
        .expect("bounded seed with Craterize Deathrite setup")
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn stage_south_site_at_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let south_site_id = state(session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South site identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    south_site_id
}

fn one_south_deathrite_at_c1(session: &mut Session) -> (String, String) {
    one_south_deathrite_at_cell(session, "C1")
}

fn one_south_deathrite_at_cell(session: &mut Session, cell: &str) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
    let south_site_id = state(session)["realm"]["sites"][cell]["instanceId"]
        .as_str()
        .expect("South site identity")
        .to_owned();
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (
        south_site_id,
        summoned["cardInstanceId"]
            .as_str()
            .expect("summoned minion identity")
            .to_owned(),
    )
}

fn two_south_deathrites_at_c1(session: &mut Session) -> (String, [String; 2]) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let site_id = state(session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let first = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let second = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (
        site_id,
        [
            first.0["cardInstanceId"].as_str().unwrap().to_owned(),
            second.0["cardInstanceId"].as_str().unwrap().to_owned(),
        ],
    )
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

fn cast_crater_at_c1(session: &mut Session, south_site_id: &str) -> (Value, Receipt) {
    cast_crater_at_cell(session, south_site_id, "C1")
}

fn cast_crater_at_cell(session: &mut Session, south_site_id: &str, cell: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-crater"
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetSiteInstanceId"] == south_site_id
            && descriptor["discardSiteInstanceId"].is_string()
    })
}

fn cast_crater_for(
    session: &mut Session,
    card_id: &str,
    site_id: &str,
    cell: &str,
) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == card_id
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetSiteInstanceId"] == site_id
            && descriptor["discardSiteInstanceId"].is_string()
    })
}

fn south_avatar_id(snapshot: &Value) -> String {
    snapshot["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned()
}

#[test]
fn rule_catalog_0657_craterize_destroys_an_unprotected_site_and_damages_its_occupant() {
    let encoded = seed_with(false, 657);
    let mut session = opening_main(&encoded);
    let south_site_id = stage_south_site_at_c1(&mut session);
    let before = state(&session);
    let south_avatar = south_avatar_id(&before);
    let atlas_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("North Atlas hand")
        .len();
    assert!(atlas_before > 0);
    assert_eq!(before["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(before["players"]["north"]["avatar"]["life"], 20);

    let (descriptor, receipt) = cast_crater_at_c1(&mut session, &south_site_id);
    let discarded = descriptor["discardSiteInstanceId"]
        .as_str()
        .expect("Atlas discard identity")
        .to_owned();
    assert_eq!(
        descriptor["targetLocation"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-destroyed",
            "magic-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
            "rubble-created",
            "magic-resolved"
        ]
    );
    let cost = receipt
        .events
        .iter()
        .find(|event| event.event_type == "card-discarded")
        .expect("Atlas discard cost");
    assert_eq!(cost.payload["instanceId"], discarded);
    assert_eq!(cost.payload["zone"], "atlas");
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-destroyed")
        .expect("site destruction");
    assert_eq!(destroyed.payload["cell"], "C1");
    assert_eq!(destroyed.payload["instanceId"], south_site_id);
    assert_eq!(destroyed.payload["owner"], "south");
    let allocated = receipt
        .events
        .iter()
        .find(|event| event.event_type == "magic-damage-allocated")
        .expect("grid allocation");
    assert_eq!(allocated.payload["targetInstanceId"], south_avatar);
    assert_eq!(allocated.payload["amount"], 1);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "magic-damage-allocated")
            .count(),
        1
    );

    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"]["rubble"], true);
    assert_eq!(after["players"]["south"]["avatar"]["location"], "C1");
    assert_eq!(after["players"]["south"]["avatar"]["life"], 19);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("North Atlas hand")
            .len(),
        atlas_before - 1
    );
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == south_site_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0658_craterize_is_prevented_on_a_protected_site_but_still_deals_damage() {
    let encoded = seed_with(true, 658);
    let mut session = opening_main(&encoded);
    let south_site_id = stage_south_site_at_c1(&mut session);
    let before = state(&session);
    let south_site = before["realm"]["sites"]["C1"].clone();
    let south_avatar = south_avatar_id(&before);
    let atlas_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("North Atlas hand")
        .len();
    assert_eq!(south_site["instanceId"], south_site_id);

    let (_, receipt) = cast_crater_at_c1(&mut session, &south_site_id);
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-destruction-prevented",
            "magic-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
            "magic-resolved"
        ]
    );
    let prevented = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-destruction-prevented")
        .expect("protected-site prevention");
    assert_eq!(prevented.payload["cell"], "C1");
    assert_eq!(prevented.payload["instanceId"], south_site_id);
    assert_eq!(prevented.payload["owner"], "south");
    let allocated = receipt
        .events
        .iter()
        .find(|event| event.event_type == "magic-damage-allocated")
        .expect("grid allocation");
    assert_eq!(allocated.payload["targetInstanceId"], south_avatar);
    assert_eq!(allocated.payload["amount"], 1);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed"
                || event.event_type == "rubble-created")
    );

    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"], south_site);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 19);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("North Atlas hand")
            .len(),
        atlas_before - 1
    );
    assert!(
        !after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == south_site_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn crater_site_ward_dependency_is_checked_for_both_owners() {
    for (owner, cell, seed) in [("north", "C4", 19000), ("south", "C1", 20000)] {
        let encoded = seed_with_crater_and_ward(seed);
        let mut session = opening_main(&encoded);
        if owner == "south" {
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == cell
            });
            let site_id = state(&session)["realm"]["sites"][cell]["instanceId"]
                .as_str()
                .unwrap()
                .to_owned();
            let (_, ward_receipt) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-magic"
                    && descriptor["cardId"] == "south-bless"
                    && descriptor["targetLocation"]["cell"] == cell
                    && descriptor["targetSiteInstanceId"] == site_id
            });
            assert!(
                ward_receipt
                    .events
                    .iter()
                    .any(|event| event.event_type == "site-warded")
            );
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
        }
        let site_before = state(&session)["realm"]["sites"][cell].clone();
        let site_id = site_before["instanceId"].as_str().unwrap().to_owned();
        if owner == "north" {
            let (_, ward_receipt) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-magic"
                    && descriptor["cardId"] == "north-bless"
                    && descriptor["targetLocation"]["cell"] == cell
                    && descriptor["targetSiteInstanceId"] == site_id
            });
            assert!(
                ward_receipt
                    .events
                    .iter()
                    .any(|event| event.event_type == "site-warded")
            );
        }
        let before = state(&session);
        assert_eq!(before["realm"]["sites"][cell]["warded"], true);
        let north_life = before["players"]["north"]["avatar"]["life"].clone();
        let south_life = before["players"]["south"]["avatar"]["life"].clone();
        let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == "north-crater"
                && descriptor["targetLocation"]["cell"] == cell
                && descriptor["targetSiteInstanceId"] == site_id
                && descriptor["discardSiteInstanceId"].is_string()
        });
        assert!(
            receipt
                .events
                .iter()
                .any(|event| event.event_type == "ward-broken")
        );
        assert!(
            receipt
                .events
                .iter()
                .any(|event| event.event_type == "site-destruction-prevented")
        );
        assert!(
            !receipt
                .events
                .iter()
                .any(|event| event.event_type == "magic-damage-allocated")
        );
        let after = state(&session);
        assert_eq!(after["realm"]["sites"][cell]["instanceId"], site_id);
        assert!(after["realm"]["sites"][cell]["warded"].is_null());
        assert_eq!(after["players"]["north"]["avatar"]["life"], north_life);
        assert_eq!(after["players"]["south"]["avatar"]["life"], south_life);
        assert!(
            after["players"]["north"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == descriptor["discardSiteInstanceId"])
        );
        assert!(
            after["players"]["north"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == descriptor["cardInstanceId"])
        );
        assert_exact_replay(&session);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "mirrors issued Crater prevention against own and opponent Site Wards"
)]
fn south_crater_respects_warded_own_and_opponent_sites_after_checkpoint_restore() {
    for (owner, cell, seed) in [("south", "C1", 22100), ("north", "C4", 22200)] {
        let encoded = (seed..seed + 256)
            .map(south_crater_site_ward_manifest)
            .find(|encoded| {
                let candidate = Session::new(encoded).unwrap();
                let opening = state(&candidate);
                let north_hand = opening["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .unwrap();
                let south_hand = opening["players"]["south"]["hand"]["spellbook"]
                    .as_array()
                    .unwrap();
                north_hand
                    .iter()
                    .any(|card| card["cardId"] == "north-bless")
                    && south_hand
                        .iter()
                        .any(|card| card["cardId"] == "south-crater")
                    && (owner == "north"
                        || south_hand
                            .iter()
                            .any(|card| card["cardId"] == "south-bless"))
            })
            .expect("bounded opening for mirrored caster and protection source");
        let mut session = opening_main(&encoded);
        if owner == "north" {
            let site_id = state(&session)["realm"]["sites"][cell]["instanceId"]
                .as_str()
                .unwrap()
                .to_owned();
            let (_, ward) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-magic"
                    && descriptor["cardId"] == "north-bless"
                    && descriptor["targetSiteInstanceId"] == site_id
            });
            assert!(
                ward.events
                    .iter()
                    .any(|event| event.event_type == "site-warded")
            );
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        if owner == "south" {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
            let own_site_id = state(&session)["realm"]["sites"][cell]["instanceId"]
                .as_str()
                .unwrap()
                .to_owned();
            let (_, ward) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-magic"
                    && descriptor["cardId"] == "south-bless"
                    && descriptor["targetSiteInstanceId"] == own_site_id
            });
            assert!(
                ward.events
                    .iter()
                    .any(|event| event.event_type == "site-warded")
            );
        } else {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
        }
        let target_before = state(&session)["realm"]["sites"][cell].clone();
        let site_id = target_before["instanceId"].as_str().unwrap().to_owned();
        assert_eq!(target_before["warded"], true);
        let parent = session.clone();
        let parent_fingerprint = full_session_fingerprint(&parent);
        let checkpoint = create_game_checkpoint(&session).expect("pre-Crater mirrored checkpoint");
        let bytes = serialize_game_checkpoint(&checkpoint).unwrap();
        let checkpoint = parse_game_checkpoint(&bytes).unwrap();
        let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == "south-crater"
                && descriptor["targetLocation"]["cell"] == cell
                && descriptor["targetSiteInstanceId"] == site_id
                && descriptor["discardSiteInstanceId"].is_string()
        });
        assert_eq!(
            event_types(&receipt),
            [
                "card-discarded",
                "magic-cast",
                "ward-broken",
                "site-destruction-prevented",
                "magic-resolved"
            ]
        );
        assert_eq!(
            receipt
                .events
                .iter()
                .filter(|event| event.event_type == "magic-damage-allocated")
                .count(),
            0
        );
        assert!(
            receipt
                .events
                .iter()
                .any(|event| event.event_type == "card-discarded"
                    && event.payload["instanceId"] == descriptor["discardSiteInstanceId"])
        );
        let after = state(&session);
        assert_eq!(after["realm"]["sites"][cell]["instanceId"], site_id);
        assert_eq!(after["realm"]["sites"][cell]["warded"], Value::Null);
        assert!(after["realm"]["sites"][cell]["rubble"].is_null());
        assert!(
            after["players"]["south"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == descriptor["cardInstanceId"])
        );
        let mut restored =
            resume_game_checkpoint(&checkpoint).expect("restore mirrored pre-Crater");
        let (_, restored_receipt) = accept_where(&mut restored, |action| action == &descriptor);
        assert_eq!(restored_receipt, receipt);
        assert_eq!(
            full_session_fingerprint(&restored),
            full_session_fingerprint(&session)
        );
        assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
        assert_exact_replay(&session);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keeps exact Crater event ordering and checkpoint replay in the original catalog case"
)]
fn rule_catalog_1082_craterize_site_minion_deathrite_completes_before_magic() {
    let encoded = seed_with_deathrite(1082);
    let mut session = opening_main(&encoded);
    let (south_site_id, target_id) = one_south_deathrite_at_c1(&mut session);
    let before = state(&session);
    assert!(realm_unit(&before, &target_id).is_some());
    let south_atlas_before = before["players"]["south"]["hand"]["atlas"]
        .as_array()
        .unwrap()
        .len();
    let parent = session.clone();
    let parent_fingerprint = full_session_fingerprint(&parent);
    let parent_checkpoint = create_game_checkpoint(&session).expect("pre-Crater checkpoint");
    let parent_checkpoint = parse_game_checkpoint(
        &serialize_game_checkpoint(&parent_checkpoint).expect("serialize pre-Crater checkpoint"),
    )
    .expect("parse pre-Crater checkpoint");
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-crater"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
            && descriptor["discardSiteInstanceId"].is_string()
    });
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died")
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "magic-resolved")
            .count(),
        1
    );
    let event_position = |wanted: &str| {
        receipt
            .events
            .iter()
            .position(|event| event.event_type == wanted)
            .unwrap_or_else(|| {
                panic!(
                    "missing {wanted} in Crater events: {:?}",
                    event_types(&receipt)
                )
            })
    };
    let discard_position = event_position("card-discarded");
    let site_position = event_position("site-destroyed");
    let rubble_position = event_position("rubble-created");
    let draw_position = event_position("site-drawn");
    let departure_position = event_position("minion-died");
    let completion_position = event_position("magic-resolved");
    assert!(discard_position < site_position && site_position < rubble_position);
    assert!(rubble_position < draw_position && draw_position < departure_position);
    assert!(departure_position < completion_position);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "card-discarded"
                && event.payload["instanceId"] == descriptor["discardSiteInstanceId"])
            .count(),
        1
    );
    let site_draw = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .unwrap();
    assert_eq!(site_draw.payload["sourceInstanceId"], target_id);
    assert_eq!(site_draw.payload["seat"], "south");
    for expected in ["site-destroyed", "rubble-created", "minion-died"] {
        assert!(
            receipt
                .events
                .iter()
                .any(|event| event.event_type == expected)
        );
    }
    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"]["rubble"], true);
    assert!(realm_unit(&after, &target_id).is_none());
    assert_eq!(
        after["players"]["south"]["hand"]["atlas"]
            .as_array()
            .unwrap()
            .len(),
        south_atlas_before + 1
    );
    let north_cemetery = after["players"]["north"]["cemetery"].as_array().unwrap();
    assert_eq!(
        north_cemetery
            .iter()
            .filter(|card| card["instanceId"] == descriptor["discardSiteInstanceId"])
            .count(),
        1
    );
    assert!(
        north_cemetery
            .iter()
            .any(|card| card["instanceId"] == descriptor["cardInstanceId"])
    );
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["instanceId"] == south_site_id)
    );
    assert_eq!(
        north_cemetery
            .iter()
            .filter(|card| card["instanceId"] == descriptor["cardInstanceId"])
            .count(),
        1
    );
    let south_cemetery = after["players"]["south"]["cemetery"].as_array().unwrap();
    assert_eq!(
        south_cemetery
            .iter()
            .filter(|card| card["instanceId"] == target_id)
            .count(),
        1
    );
    assert_eq!(
        south_cemetery
            .iter()
            .filter(|card| card["instanceId"] == south_site_id)
            .count(),
        1
    );
    let rubble_id = after["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("new Crater Rubble identity");
    assert_ne!(rubble_id, south_site_id);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "rubble-created"
                && event.payload["instanceId"] == rubble_id)
            .count(),
        1
    );
    let mut restored_parent =
        resume_game_checkpoint(&parent_checkpoint).expect("restore pre-Crater parent");
    let (_, restored_receipt) =
        accept_where(&mut restored_parent, |candidate| candidate == &descriptor);
    assert_eq!(restored_receipt, receipt);
    assert_eq!(
        full_session_fingerprint(&restored_parent),
        full_session_fingerprint(&session)
    );
    assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
    let completion_fingerprint = full_session_fingerprint(&session);
    let completion_checkpoint =
        create_game_checkpoint(&session).expect("completed catalog Crater checkpoint");
    let completion_checkpoint =
        parse_game_checkpoint(&serialize_game_checkpoint(&completion_checkpoint).unwrap()).unwrap();
    let restored_completion =
        resume_game_checkpoint(&completion_checkpoint).expect("restore completed Crater");
    assert_eq!(
        full_session_fingerprint(&restored_completion),
        completion_fingerprint
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "proves an issued Crater's failed DrawSite leaves its remaining cohort and held Magic inert"
)]
fn crater_empty_atlas_draw_failure_keeps_magic_and_second_deathrite_pending() {
    let encoded = (2182..2438)
        .map(crater_carried_artifact_manifest)
        .find(|candidate| {
            let session = Session::new(candidate).expect("Crater carried candidate");
            let opening = state(&session);
            opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["cardId"] == "north-crater")
                && opening["players"]["south"]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|card| card["cardId"] == "south-minion")
                    .count()
                    >= 2
                && opening["players"]["south"]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["cardId"] == "south-artifact")
        })
        .expect("bounded opening with actual carried Artifact and both Deathrites");
    let mut manifest: Value = serde_json::from_str(&encoded).expect("Crater Deathrite manifest");
    manifest.as_object_mut().unwrap().remove("manifestId");
    manifest["decks"]["south"]["atlas"] = json!(["south-site", "south-site", "south-site"]);
    let encoded = finish_manifest(manifest);
    let mut session = opening_main(&encoded);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let first = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let second = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let source_ids = [
        first.0["cardInstanceId"].as_str().unwrap().to_owned(),
        second.0["cardInstanceId"].as_str().unwrap().to_owned(),
    ];
    let (artifact, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"]["instanceId"] == source_ids[0]
    });
    let artifact_id = artifact["cardInstanceId"].as_str().unwrap().to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(
        session
            .legal_actions()
            .unwrap()
            .iter()
            .any(|action| action.descriptor["kind"] == "play-site"),
        "South site frontier after Atlas draw: state={:?} actions={:?}",
        state(&session)["phase"],
        session.legal_actions().unwrap()
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let south_before = state(&session)["players"]["south"].clone();
    assert_eq!(south_before["atlas"].as_array().unwrap().len(), 0);
    assert_eq!(
        south_before["hand"]["atlas"].as_array().unwrap().len(),
        0,
        "South exhausted all three actual Atlas cards into Sites"
    );
    let parent = session.clone();
    let parent_fingerprint = full_session_fingerprint(&parent);
    let parent_checkpoint = create_game_checkpoint(&session).expect("pre-Crater checkpoint");
    let parent_checkpoint = serialize_game_checkpoint(&parent_checkpoint).unwrap();
    let parent_checkpoint = parse_game_checkpoint(&parent_checkpoint).unwrap();
    let (crater, crater_receipt) = cast_crater_at_c1(&mut session, &site_id);
    let crater_id = crater["cardInstanceId"].as_str().unwrap().to_owned();
    assert!(
        crater_receipt
            .events
            .iter()
            .any(|event| event.event_type == "card-discarded")
    );
    assert!(
        crater_receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed")
    );
    assert!(
        crater_receipt
            .events
            .iter()
            .any(|event| event.event_type == "rubble-created")
    );
    let mut restored_parent =
        resume_game_checkpoint(&parent_checkpoint).expect("restore pre-Crater parent");
    let (_, restored_crater_receipt) = cast_crater_at_c1(&mut restored_parent, &site_id);
    assert_eq!(restored_crater_receipt, crater_receipt);
    assert_eq!(
        full_session_fingerprint(&restored_parent),
        full_session_fingerprint(&session)
    );
    let pause = state(&session);
    assert_eq!(pause["phase"], "trigger-order");
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["kind"],
        "magic-resolved"
    );
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["heldCard"]["instanceId"],
        crater_id
    );
    assert!(
        source_ids.iter().all(
            |source| realm_unit(&pause, source).is_some_and(|unit| unit["deathMarked"] == true)
        )
    );
    assert_eq!(session.legal_actions().unwrap().len(), 2);
    let checkpoint = create_game_checkpoint(&session).expect("Crater order checkpoint");
    let bytes = serialize_game_checkpoint(&checkpoint).expect("serialize Crater order");
    let checkpoint = parse_game_checkpoint(&bytes).expect("parse Crater order");
    let pause_fingerprint = full_session_fingerprint(&session);
    for first_source in &source_ids {
        let mut branch = resume_game_checkpoint(&checkpoint).expect("restore Crater branch");
        let (_, receipt) = accept_where(&mut branch, |descriptor| {
            descriptor["kind"] == "order-triggers"
                && descriptor["sourceInstanceId"] == *first_source
        });
        assert!(
            receipt.events.iter().all(|event| {
                event.event_type != "site-drawn"
                    && event.event_type != "deathrite-draw-site"
                    && event.event_type != "magic-resolved"
                    && event.event_type != "draw"
            }),
            "empty Atlas failure does not finish the source, held Magic, or normal draw"
        );
        assert!(receipt.events.iter().all(|event| {
            !matches!(
                event.event_type.as_str(),
                "artifact-dropped" | "minion-died" | "spell-drawn"
            )
        }));
        let terminal = state(&branch);
        assert!(terminal["terminal"].is_object() || terminal["terminal"].is_string());
        assert_eq!(terminal["realm"]["sites"]["C1"]["rubble"], true);
        assert!(source_ids.iter().all(|source| {
            realm_unit(&terminal, source).is_some_and(|unit| unit["deathMarked"] == true)
        }));
        let batch = &terminal["pendingDeathrites"]["batches"][0];
        assert_eq!(batch["stage"], "resolve");
        assert!(
            batch["resolving"]
                .as_array()
                .unwrap()
                .iter()
                .any(|source| source_ids
                    .iter()
                    .any(|id| source["instanceId"] == id.as_str())),
            "a real source remains captured in the interrupted resolver: {batch}"
        );
        assert!(
            source_ids
                .iter()
                .all(|source| terminal["pendingDeathrites"]["marked"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["instanceId"] == source.as_str())),
            "the sibling source remains marked and inert after the terminal first draw"
        );
        assert_eq!(
            terminal["pendingDeathrites"]["continuation"]["kind"],
            "magic-resolved"
        );
        assert_eq!(
            terminal["pendingDeathrites"]["continuation"]["heldCard"]["instanceId"],
            crater_id
        );
        assert!(
            terminal["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["instanceId"] == artifact_id
                    && artifact["bearer"]["instanceId"] == source_ids[0])
        );
        assert!(branch.legal_actions().unwrap().is_empty());
        assert!(
            terminal["players"]["north"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .all(|card| card["instanceId"] != crater_id)
        );
        let terminal_fingerprint = full_session_fingerprint(&branch);
        let terminal_checkpoint = create_game_checkpoint(&branch).expect("terminal checkpoint");
        let terminal_checkpoint = serialize_game_checkpoint(&terminal_checkpoint).unwrap();
        let terminal_checkpoint = parse_game_checkpoint(&terminal_checkpoint).unwrap();
        let terminal_restored =
            resume_game_checkpoint(&terminal_checkpoint).expect("restore terminal outcome");
        assert_eq!(
            full_session_fingerprint(&terminal_restored),
            terminal_fingerprint
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
    assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "mirrors the public empty-Atlas Crater checkpoint proof"
)]
fn crater_empty_atlas_draw_failure_keeps_magic_and_second_deathrite_pending_mirrored() {
    let encoded = (8100..8356)
        .map(mirrored_crater_carried_artifact_manifest)
        .find(|candidate| {
            let session = Session::new(candidate).expect("mirrored Crater carried candidate");
            let opening = state(&session);
            opening["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["cardId"] == "south-crater")
                && opening["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|card| card["cardId"] == "north-minion")
                    .count()
                    >= 2
                && opening["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["cardId"] == "north-artifact")
        })
        .expect("bounded mirrored opening with carried Artifact and both Deathrites");
    let mut manifest: Value = serde_json::from_str(&encoded).expect("Crater Deathrite manifest");
    manifest.as_object_mut().unwrap().remove("manifestId");
    manifest["decks"]["north"]["atlas"] = json!(["north-site", "north-site", "north-site"]);
    let encoded = finish_manifest(manifest);
    let mut session = opening_south_main(&encoded);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let first = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let second = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let source_ids = [
        first.0["cardInstanceId"].as_str().unwrap().to_owned(),
        second.0["cardInstanceId"].as_str().unwrap().to_owned(),
    ];
    let (artifact, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-artifact"
            && descriptor["bearer"]["instanceId"] == source_ids[0]
    });
    let artifact_id = artifact["cardInstanceId"].as_str().unwrap().to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(
        session
            .legal_actions()
            .unwrap()
            .iter()
            .any(|action| action.descriptor["kind"] == "play-site"),
        "South site frontier after Atlas draw: state={:?} actions={:?}",
        state(&session)["phase"],
        session.legal_actions().unwrap()
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let north_before = state(&session)["players"]["north"].clone();
    assert_eq!(north_before["atlas"].as_array().unwrap().len(), 0);
    assert_eq!(
        north_before["hand"]["atlas"].as_array().unwrap().len(),
        0,
        "South exhausted all three actual Atlas cards into Sites"
    );
    let parent = session.clone();
    let parent_fingerprint = full_session_fingerprint(&parent);
    let parent_checkpoint = create_game_checkpoint(&session).expect("pre-Crater checkpoint");
    let parent_checkpoint = serialize_game_checkpoint(&parent_checkpoint).unwrap();
    let parent_checkpoint = parse_game_checkpoint(&parent_checkpoint).unwrap();
    let (crater, crater_receipt) = cast_crater_for(&mut session, "south-crater", &site_id, "C4");
    let crater_id = crater["cardInstanceId"].as_str().unwrap().to_owned();
    assert!(
        crater_receipt
            .events
            .iter()
            .any(|event| event.event_type == "card-discarded")
    );
    assert!(
        crater_receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed")
    );
    assert!(
        crater_receipt
            .events
            .iter()
            .any(|event| event.event_type == "rubble-created")
    );
    let mut restored_parent =
        resume_game_checkpoint(&parent_checkpoint).expect("restore pre-Crater parent");
    let (_, restored_crater_receipt) =
        cast_crater_for(&mut restored_parent, "south-crater", &site_id, "C4");
    assert_eq!(restored_crater_receipt, crater_receipt);
    assert_eq!(
        full_session_fingerprint(&restored_parent),
        full_session_fingerprint(&session)
    );
    let pause = state(&session);
    assert_eq!(pause["phase"], "trigger-order");
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["kind"],
        "magic-resolved"
    );
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["heldCard"]["instanceId"],
        crater_id
    );
    assert!(
        source_ids.iter().all(
            |source| realm_unit(&pause, source).is_some_and(|unit| unit["deathMarked"] == true)
        )
    );
    assert_eq!(session.legal_actions().unwrap().len(), 2);
    let checkpoint = create_game_checkpoint(&session).expect("Crater order checkpoint");
    let bytes = serialize_game_checkpoint(&checkpoint).expect("serialize Crater order");
    let checkpoint = parse_game_checkpoint(&bytes).expect("parse Crater order");
    let pause_fingerprint = full_session_fingerprint(&session);
    for first_source in &source_ids {
        let mut branch = resume_game_checkpoint(&checkpoint).expect("restore Crater branch");
        let (_, receipt) = accept_where(&mut branch, |descriptor| {
            descriptor["kind"] == "order-triggers"
                && descriptor["sourceInstanceId"] == *first_source
        });
        assert!(
            receipt.events.iter().all(|event| {
                event.event_type != "site-drawn"
                    && event.event_type != "deathrite-draw-site"
                    && event.event_type != "magic-resolved"
                    && event.event_type != "draw"
            }),
            "empty Atlas failure does not finish the source, held Magic, or normal draw"
        );
        assert!(receipt.events.iter().all(|event| {
            !matches!(
                event.event_type.as_str(),
                "artifact-dropped" | "minion-died" | "spell-drawn"
            )
        }));
        let terminal = state(&branch);
        assert!(terminal["terminal"].is_object() || terminal["terminal"].is_string());
        assert_eq!(terminal["realm"]["sites"]["C4"]["rubble"], true);
        assert!(source_ids.iter().all(|source| {
            realm_unit(&terminal, source).is_some_and(|unit| unit["deathMarked"] == true)
        }));
        let batch = &terminal["pendingDeathrites"]["batches"][0];
        assert_eq!(batch["stage"], "resolve");
        assert!(
            batch["resolving"]
                .as_array()
                .unwrap()
                .iter()
                .any(|source| source_ids
                    .iter()
                    .any(|id| source["instanceId"] == id.as_str())),
            "a real source remains captured in the interrupted resolver: {batch}"
        );
        assert!(
            source_ids
                .iter()
                .all(|source| terminal["pendingDeathrites"]["marked"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["instanceId"] == source.as_str())),
            "the sibling source remains marked and inert after the terminal first draw"
        );
        assert_eq!(
            terminal["pendingDeathrites"]["continuation"]["kind"],
            "magic-resolved"
        );
        assert_eq!(
            terminal["pendingDeathrites"]["continuation"]["heldCard"]["instanceId"],
            crater_id
        );
        assert!(
            terminal["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["instanceId"] == artifact_id
                    && artifact["bearer"]["instanceId"] == source_ids[0])
        );
        assert!(branch.legal_actions().unwrap().is_empty());
        assert!(
            terminal["players"]["south"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .all(|card| card["instanceId"] != crater_id)
        );
        let terminal_fingerprint = full_session_fingerprint(&branch);
        let terminal_checkpoint = create_game_checkpoint(&branch).expect("terminal checkpoint");
        let terminal_checkpoint = serialize_game_checkpoint(&terminal_checkpoint).unwrap();
        let terminal_checkpoint = parse_game_checkpoint(&terminal_checkpoint).unwrap();
        let terminal_restored =
            resume_game_checkpoint(&terminal_checkpoint).expect("restore terminal outcome");
        assert_eq!(
            full_session_fingerprint(&terminal_restored),
            terminal_fingerprint
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
    assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
    assert_exact_replay(&session);
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keeps the genuine Crater pause and two continuation branches together"
)]
fn crater_holds_actual_magic_after_complete_grid_and_site_departure() {
    let encoded = seed_with_deathrite(1082);
    let mut session = opening_main(&encoded);
    let (site_id, deathrite_ids) = two_south_deathrites_at_c1(&mut session);
    let pre_action = create_game_checkpoint(&session).expect("pre-Crater checkpoint");
    let pre_action = serialize_game_checkpoint(&pre_action).unwrap();
    let pre_action = parse_game_checkpoint(&pre_action).unwrap();
    let (descriptor, receipt) = cast_crater_at_c1(&mut session, &site_id);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed")
    );
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "rubble-created")
    );
    let pause = state(&session);
    assert_eq!(pause["phase"], "trigger-order");
    assert_eq!(pause["realm"]["sites"]["C1"]["rubble"], true);
    let continuation = &pause["pendingDeathrites"]["continuation"];
    assert_eq!(continuation["kind"], "magic-resolved");
    assert_eq!(
        continuation["heldCard"]["instanceId"],
        descriptor["cardInstanceId"]
    );
    for source in &deathrite_ids {
        assert_eq!(realm_unit(&pause, source).unwrap()["deathMarked"], true);
        assert!(
            !pause["players"]["south"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == *source)
        );
    }
    assert!(
        pause["players"]["north"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["instanceId"] == descriptor["discardSiteInstanceId"])
    );
    assert_eq!(
        pause["players"]["north"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|card| card["instanceId"] == descriptor["cardInstanceId"])
            .count(),
        0
    );
    assert_eq!(
        pause["players"]["south"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|card| card["instanceId"] == site_id)
            .count(),
        1
    );
    let pause_checkpoint = create_game_checkpoint(&session).expect("pause checkpoint");
    let pause_checkpoint = serialize_game_checkpoint(&pause_checkpoint).unwrap();
    let pause_checkpoint = parse_game_checkpoint(&pause_checkpoint).unwrap();
    let parent_fingerprint = full_session_fingerprint(&session);
    for first_source in &deathrite_ids {
        let mut branch = resume_game_checkpoint(&pause_checkpoint).expect("restore pause branch");
        accept_where(&mut branch, |action| {
            action["kind"] == "order-triggers" && action["sourceInstanceId"] == *first_source
        });
        while state(&branch)["phase"] == "trigger-order" {
            accept_where(&mut branch, |action| action["kind"] == "order-triggers");
        }
        let complete = state(&branch);
        for source in &deathrite_ids {
            assert!(realm_unit(&complete, source).is_none());
            assert!(
                complete["players"]["south"]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["instanceId"] == *source)
            );
        }
        assert_eq!(
            complete["players"]["north"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|card| card["instanceId"] == descriptor["cardInstanceId"])
                .count(),
            1
        );
        assert_eq!(
            branch
                .transcript()
                .iter()
                .flat_map(|receipt| &receipt.events)
                .filter(|event| event.event_type == "magic-resolved")
                .count(),
            1
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), parent_fingerprint);
    assert!(resume_game_checkpoint(&pre_action).is_ok());
    assert_eq!(state(&session)["phase"], "trigger-order");
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "proves actual carried custody through both Crater Deathrite orders"
)]
fn crater_carried_artifact_stays_with_marked_source_until_each_order_completes() {
    let encoded = (1120..1376)
        .map(crater_carried_artifact_manifest)
        .find(|encoded| {
            let candidate = Session::new(encoded).unwrap();
            let current = state(&candidate);
            let north = current["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            let south = current["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            north.iter().any(|card| card["cardId"] == "north-crater")
                && south
                    .iter()
                    .filter(|card| card["cardId"] == "south-minion")
                    .count()
                    >= 2
                && south.iter().any(|card| card["cardId"] == "south-artifact")
        })
        .expect("bounded opening with Crater, two Deathrites and carried Artifact");
    let mut session = opening_main(&encoded);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let first = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    let second = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    let source_ids = [
        first.0["cardInstanceId"].as_str().unwrap().to_owned(),
        second.0["cardInstanceId"].as_str().unwrap().to_owned(),
    ];
    let (artifact_cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"]["instanceId"] == source_ids[0]
    });
    let artifact_id = artifact_cast["cardInstanceId"].as_str().unwrap().to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let parent = session.clone();
    let parent_fingerprint = full_session_fingerprint(&parent);
    let (_, receipt) = cast_crater_at_c1(&mut session, &site_id);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "rubble-created")
    );
    let pause = state(&session);
    assert_eq!(
        pause["phase"],
        "trigger-order",
        "Crater events: {:?}",
        event_types(&receipt)
    );
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["heldCard"]["cardId"],
        "north-crater"
    );
    for source in &source_ids {
        assert_eq!(realm_unit(&pause, source).unwrap()["deathMarked"], true);
    }
    let carried = pause["realm"]["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|artifact| artifact["instanceId"] == artifact_id)
        .unwrap();
    assert_eq!(carried["bearer"]["instanceId"], source_ids[0]);
    let pause_checkpoint =
        create_game_checkpoint(&session).expect("carried Crater pause checkpoint");
    let pause_checkpoint = serialize_game_checkpoint(&pause_checkpoint).unwrap();
    let pause_checkpoint = parse_game_checkpoint(&pause_checkpoint).unwrap();
    let pause_fingerprint = full_session_fingerprint(&session);
    for first_source in &source_ids {
        let mut branch = resume_game_checkpoint(&pause_checkpoint).expect("restore carried pause");
        accept_where(&mut branch, |descriptor| {
            descriptor["kind"] == "order-triggers"
                && descriptor["sourceInstanceId"] == *first_source
        });
        while state(&branch)["phase"] == "trigger-order" {
            accept_where(&mut branch, |descriptor| {
                descriptor["kind"] == "order-triggers"
            });
        }
        let complete = state(&branch);
        assert!(
            source_ids
                .iter()
                .all(|source| realm_unit(&complete, source).is_none())
        );
        let dropped = complete["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|artifact| artifact["instanceId"] == artifact_id)
            .unwrap();
        assert_eq!(dropped["location"], "C1");
        assert_eq!(dropped["region"], "surface");
        let receipts = branch.transcript();
        assert_eq!(
            receipts
                .iter()
                .flat_map(|item| &item.events)
                .filter(|event| event.event_type == "artifact-dropped"
                    && event.payload["instanceId"] == artifact_id)
                .count(),
            1
        );
        assert_eq!(
            receipts
                .iter()
                .flat_map(|item| &item.events)
                .filter(|event| event.event_type == "magic-resolved"
                    && event.payload["cardId"] == "north-crater")
                .count(),
            1
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
    assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keeps the mirrored held-Magic pause and order branches together"
)]
fn crater_holds_actual_magic_after_complete_grid_in_mirrored_seat_orientation() {
    let encoded = seed_with_south_crater(1083);
    let mut session = Session::new(&encoded).expect("mirrored Crater session");
    keep(&mut session);
    keep(&mut session);
    let opening_actions = session.legal_actions().unwrap();
    assert!(
        opening_actions
            .iter()
            .any(|action| action.descriptor["kind"] == "play-site"
                && action.descriptor["cell"] == "C1"),
        "mirrored opening phase: {} / {:?}",
        state(&session)["phase"],
        opening_actions
            .iter()
            .map(|action| &action.descriptor)
            .collect::<Vec<_>>()
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"].clone();
    let first = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let second = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let sources = [
        first.0["cardInstanceId"].as_str().unwrap().to_owned(),
        second.0["cardInstanceId"].as_str().unwrap().to_owned(),
    ];
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let cast = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-crater"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
            && descriptor["discardSiteInstanceId"].is_string()
    });
    let before_branch = create_game_checkpoint(&session).expect("mirrored pre-branch checkpoint");
    let pause = state(&session);
    assert_eq!(pause["phase"], "trigger-order");
    assert_eq!(pause["realm"]["sites"]["C4"]["rubble"], true);
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["kind"],
        "magic-resolved"
    );
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["heldCard"]["instanceId"],
        cast.0["cardInstanceId"]
    );
    for source in &sources {
        assert_eq!(realm_unit(&pause, source).unwrap()["deathMarked"], true);
        assert!(
            !pause["players"]["north"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == *source)
        );
    }
    let pause_checkpoint = create_game_checkpoint(&session).expect("mirrored pause checkpoint");
    let pause_checkpoint =
        parse_game_checkpoint(&serialize_game_checkpoint(&pause_checkpoint).unwrap()).unwrap();
    let parent_fingerprint = full_session_fingerprint(&session);
    for first_source in &sources {
        let mut branch =
            resume_game_checkpoint(&pause_checkpoint).expect("restore mirrored branch");
        accept_where(&mut branch, |action| {
            action["kind"] == "order-triggers" && action["sourceInstanceId"] == *first_source
        });
        while state(&branch)["phase"] == "trigger-order" {
            accept_where(&mut branch, |action| action["kind"] == "order-triggers");
        }
        for source in &sources {
            assert!(realm_unit(&state(&branch), source).is_none());
        }
        assert_eq!(
            state(&branch)["players"]["south"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|card| card["instanceId"] == cast.0["cardInstanceId"])
                .count(),
            1
        );
        assert_eq!(
            branch
                .transcript()
                .iter()
                .flat_map(|receipt| &receipt.events)
                .filter(|event| event.event_type == "magic-resolved")
                .count(),
            1
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), parent_fingerprint);
    assert!(resume_game_checkpoint(&before_branch).is_ok());
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keeps the full mixed Water-region casualty and custody proof together"
)]
fn crater_water_to_rubble_combines_regional_death_and_dual_region_survival() {
    let encoded = seed_with_water_crater(6500);
    let mut session = opening_main(&encoded);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    let site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let (submerge, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"] == "underwater"
    });
    let (dual, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-dual"
            && descriptor["cell"] == "C1"
            && descriptor["region"] == "underwater"
    });
    let (surface, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-surface"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let submerge_id = submerge["cardInstanceId"].as_str().unwrap().to_owned();
    let dual_id = dual["cardInstanceId"].as_str().unwrap().to_owned();
    let surface_id = surface["cardInstanceId"].as_str().unwrap().to_owned();
    let (artifact, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["bearer"]["instanceId"] == submerge_id
    });
    let artifact_id = artifact["cardInstanceId"].as_str().unwrap().to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (second_artifact, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && descriptor["cardInstanceId"] != artifact_id
            && descriptor["bearer"]["instanceId"] == dual_id
    });
    let second_artifact_id = second_artifact["cardInstanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "drop-artifacts"
            && descriptor["unit"]["instanceId"] == dual_id
            && descriptor["artifactInstanceIds"] == json!([second_artifact_id])
    });
    let loose_before = state(&session)["realm"]["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["instanceId"] == second_artifact_id)
        .cloned()
        .expect("actual loose Artifact from the underwater dual-region bearer");
    assert_eq!(loose_before["bearer"], Value::Null);
    assert_eq!(loose_before["location"], "C1");
    assert_eq!(loose_before["region"], "underwater");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, receipt) = cast_crater_at_c1(&mut session, &site_id);
    let types = event_types(&receipt);
    let site_departure = types
        .iter()
        .position(|kind| *kind == "site-destroyed")
        .unwrap();
    let rubble_created = types
        .iter()
        .position(|kind| *kind == "rubble-created")
        .unwrap();
    assert!(site_departure < rubble_created);
    assert!(!types.contains(&"minion-died"));
    assert_eq!(
        types.iter().filter(|kind| **kind == "ward-broken").count(),
        1
    );
    let pause = state(&session);
    assert_eq!(pause["phase"], "trigger-order");
    assert_eq!(pause["realm"]["sites"]["C1"]["rubble"], true);
    assert_eq!(
        realm_unit(&pause, &submerge_id).unwrap()["deathMarked"],
        true
    );
    assert_eq!(realm_unit(&pause, &submerge_id).unwrap()["warded"], false);
    assert_eq!(
        realm_unit(&pause, &surface_id).unwrap()["deathMarked"],
        true
    );
    assert!(
        pause["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| {
                artifact["instanceId"] == artifact_id
                    && artifact["bearer"]["instanceId"] == submerge_id
            })
    );
    let loose_after = pause["realm"]["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["instanceId"] == second_artifact_id)
        .expect("loose underwater Artifact remains represented during Crater custody");
    assert_eq!(loose_after["bearer"], Value::Null);
    assert_eq!(loose_after["location"], "C1");
    assert_eq!(loose_after["region"], "underground");
    let dual_after = realm_unit(&pause, &dual_id).unwrap();
    assert_eq!(dual_after["region"], "underground");
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["kind"],
        "magic-resolved"
    );
    let pause_checkpoint = create_game_checkpoint(&session).expect("Water Crater pause checkpoint");
    let pause_checkpoint = serialize_game_checkpoint(&pause_checkpoint).unwrap();
    let pause_checkpoint = parse_game_checkpoint(&pause_checkpoint).unwrap();
    let parent_fingerprint = full_session_fingerprint(&session);
    let trigger_ids: Vec<_> = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| action.descriptor["sourceInstanceId"].clone())
        .collect();
    assert_eq!(trigger_ids.len(), 2);
    for first_source in trigger_ids {
        let mut branch = resume_game_checkpoint(&pause_checkpoint).expect("restore Water branch");
        let first_order = accept_where(&mut branch, |action| {
            action["kind"] == "order-triggers" && action["sourceInstanceId"] == first_source
        });
        let mut ordered_receipts = vec![first_order];
        while state(&branch)["phase"] == "trigger-order" {
            let next_source = branch
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| action.descriptor["kind"] == "order-triggers")
                .unwrap()
                .descriptor["sourceInstanceId"]
                .clone();
            ordered_receipts.push(accept_where(&mut branch, |action| {
                action["kind"] == "order-triggers" && action["sourceInstanceId"] == next_source
            }));
        }
        let site_draws: Vec<_> = ordered_receipts
            .iter()
            .flat_map(|(_, receipt)| &receipt.events)
            .filter(|event| event.event_type == "site-drawn")
            .collect();
        assert_eq!(site_draws.len(), 2);
        assert_eq!(
            site_draws[0].payload["sourceInstanceId"],
            ordered_receipts[0].0["sourceInstanceId"]
        );
        assert!(
            site_draws
                .iter()
                .all(|event| event.payload["seat"] == "south")
        );
        let mut drawn_sources: Vec<_> = site_draws
            .iter()
            .map(|event| {
                event.payload["sourceInstanceId"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        drawn_sources.sort();
        let mut expected_sources = vec![submerge_id.clone(), surface_id.clone()];
        expected_sources.sort();
        assert_eq!(drawn_sources, expected_sources);
        let mut dead_ids: Vec<_> = ordered_receipts
            .iter()
            .flat_map(|(_, receipt)| &receipt.events)
            .filter(|event| event.event_type == "minion-died")
            .map(|event| event.payload["instanceId"].as_str().unwrap().to_owned())
            .collect();
        dead_ids.sort();
        let mut expected_dead_ids = vec![submerge_id.clone(), surface_id.clone()];
        expected_dead_ids.sort();
        assert_eq!(dead_ids, expected_dead_ids);
        let complete = state(&branch);
        assert!(realm_unit(&complete, &submerge_id).is_none());
        assert!(realm_unit(&complete, &surface_id).is_none());
        assert!(
            complete["players"]["south"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == submerge_id)
        );
        assert!(
            complete["players"]["south"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == surface_id)
        );
        assert!(
            complete["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| {
                    artifact["instanceId"] == artifact_id
                        && artifact["bearer"].is_null()
                        && artifact["location"] == "C1"
                        && artifact["region"] == "underground"
                })
        );
        assert_eq!(
            branch
                .transcript()
                .iter()
                .flat_map(|receipt| &receipt.events)
                .filter(|event| event.event_type == "magic-resolved")
                .count(),
            1
        );
        let completion_fingerprint = full_session_fingerprint(&branch);
        let completion_checkpoint =
            create_game_checkpoint(&branch).expect("completed Water Crater checkpoint");
        let completion_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&completion_checkpoint).unwrap())
                .unwrap();
        let restored_completion =
            resume_game_checkpoint(&completion_checkpoint).expect("restore Water completion");
        assert_eq!(
            full_session_fingerprint(&restored_completion),
            completion_fingerprint
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), parent_fingerprint);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keeps the mirrored Crater cohort checkpoint proof together"
)]
fn crater_water_to_rubble_combines_regional_death_and_dual_region_survival_mirrored() {
    let encoded = seed_with_mirrored_water_crater(7100);
    let mut session = opening_mirrored_main(&encoded);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C4"
    });
    let site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let (submerge, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underwater"
    });
    let (dual, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-dual"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underwater"
    });
    let (surface, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-surface"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let submerge_id = submerge["cardInstanceId"].as_str().unwrap().to_owned();
    let dual_id = dual["cardInstanceId"].as_str().unwrap().to_owned();
    let surface_id = surface["cardInstanceId"].as_str().unwrap().to_owned();
    let (artifact, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-artifact"
            && descriptor["bearer"]["instanceId"] == submerge_id
    });
    let artifact_id = artifact["cardInstanceId"].as_str().unwrap().to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (second_artifact, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-artifact"
            && descriptor["cardInstanceId"] != artifact_id
            && descriptor["bearer"]["instanceId"] == dual_id
    });
    let second_artifact_id = second_artifact["cardInstanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "drop-artifacts"
            && descriptor["unit"]["instanceId"] == dual_id
            && descriptor["artifactInstanceIds"] == json!([second_artifact_id])
    });
    let loose_before = state(&session)["realm"]["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["instanceId"] == second_artifact_id)
        .cloned()
        .expect("actual loose Artifact from the underwater dual-region bearer");
    assert_eq!(loose_before["bearer"], Value::Null);
    assert_eq!(loose_before["location"], "C4");
    assert_eq!(loose_before["region"], "underwater");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, receipt) = cast_crater_for(&mut session, "south-crater", &site_id, "C4");
    let types = event_types(&receipt);
    let site_departure = types
        .iter()
        .position(|kind| *kind == "site-destroyed")
        .unwrap();
    let rubble_created = types
        .iter()
        .position(|kind| *kind == "rubble-created")
        .unwrap();
    assert!(site_departure < rubble_created);
    assert!(!types.contains(&"minion-died"));
    assert_eq!(
        types.iter().filter(|kind| **kind == "ward-broken").count(),
        1
    );
    let pause = state(&session);
    assert_eq!(pause["phase"], "trigger-order");
    assert_eq!(pause["realm"]["sites"]["C4"]["rubble"], true);
    assert_eq!(
        realm_unit(&pause, &submerge_id).unwrap()["deathMarked"],
        true
    );
    assert_eq!(realm_unit(&pause, &submerge_id).unwrap()["warded"], false);
    assert_eq!(
        realm_unit(&pause, &surface_id).unwrap()["deathMarked"],
        true
    );
    assert!(
        pause["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| {
                artifact["instanceId"] == artifact_id
                    && artifact["bearer"]["instanceId"] == submerge_id
            })
    );
    let loose_after = pause["realm"]["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["instanceId"] == second_artifact_id)
        .expect("loose underwater Artifact remains represented during Crater custody");
    assert_eq!(loose_after["bearer"], Value::Null);
    assert_eq!(loose_after["location"], "C4");
    assert_eq!(loose_after["region"], "underground");
    let dual_after = realm_unit(&pause, &dual_id).unwrap();
    assert_eq!(dual_after["region"], "underground");
    assert_eq!(
        pause["pendingDeathrites"]["continuation"]["kind"],
        "magic-resolved"
    );
    let pause_checkpoint = create_game_checkpoint(&session).expect("Water Crater pause checkpoint");
    let pause_checkpoint = serialize_game_checkpoint(&pause_checkpoint).unwrap();
    let pause_checkpoint = parse_game_checkpoint(&pause_checkpoint).unwrap();
    let parent_fingerprint = full_session_fingerprint(&session);
    let trigger_ids: Vec<_> = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| action.descriptor["sourceInstanceId"].clone())
        .collect();
    assert_eq!(trigger_ids.len(), 2);
    for first_source in trigger_ids {
        let mut branch = resume_game_checkpoint(&pause_checkpoint).expect("restore Water branch");
        let first_order = accept_where(&mut branch, |action| {
            action["kind"] == "order-triggers" && action["sourceInstanceId"] == first_source
        });
        let mut ordered_receipts = vec![first_order];
        while state(&branch)["phase"] == "trigger-order" {
            let next_source = branch
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| action.descriptor["kind"] == "order-triggers")
                .unwrap()
                .descriptor["sourceInstanceId"]
                .clone();
            ordered_receipts.push(accept_where(&mut branch, |action| {
                action["kind"] == "order-triggers" && action["sourceInstanceId"] == next_source
            }));
        }
        let site_draws: Vec<_> = ordered_receipts
            .iter()
            .flat_map(|(_, receipt)| &receipt.events)
            .filter(|event| event.event_type == "site-drawn")
            .collect();
        assert_eq!(site_draws.len(), 2);
        assert_eq!(
            site_draws[0].payload["sourceInstanceId"],
            ordered_receipts[0].0["sourceInstanceId"]
        );
        assert!(
            site_draws
                .iter()
                .all(|event| event.payload["seat"] == "north")
        );
        let mut drawn_sources: Vec<_> = site_draws
            .iter()
            .map(|event| {
                event.payload["sourceInstanceId"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        drawn_sources.sort();
        let mut expected_sources = vec![submerge_id.clone(), surface_id.clone()];
        expected_sources.sort();
        assert_eq!(drawn_sources, expected_sources);
        let mut dead_ids: Vec<_> = ordered_receipts
            .iter()
            .flat_map(|(_, receipt)| &receipt.events)
            .filter(|event| event.event_type == "minion-died")
            .map(|event| event.payload["instanceId"].as_str().unwrap().to_owned())
            .collect();
        dead_ids.sort();
        let mut expected_dead_ids = vec![submerge_id.clone(), surface_id.clone()];
        expected_dead_ids.sort();
        assert_eq!(dead_ids, expected_dead_ids);
        let complete = state(&branch);
        assert!(realm_unit(&complete, &submerge_id).is_none());
        assert!(realm_unit(&complete, &surface_id).is_none());
        assert!(
            complete["players"]["north"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == submerge_id)
        );
        assert!(
            complete["players"]["north"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == surface_id)
        );
        assert!(
            complete["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| {
                    artifact["instanceId"] == artifact_id
                        && artifact["bearer"].is_null()
                        && artifact["location"] == "C4"
                        && artifact["region"] == "underground"
                })
        );
        assert_eq!(
            branch
                .transcript()
                .iter()
                .flat_map(|receipt| &receipt.events)
                .filter(|event| event.event_type == "magic-resolved")
                .count(),
            1
        );
        let completion_fingerprint = full_session_fingerprint(&branch);
        let completion_checkpoint =
            create_game_checkpoint(&branch).expect("completed mirrored Water Crater checkpoint");
        let completion_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&completion_checkpoint).unwrap())
                .unwrap();
        let restored_completion =
            resume_game_checkpoint(&completion_checkpoint).expect("restore mirrored completion");
        assert_eq!(
            full_session_fingerprint(&restored_completion),
            completion_fingerprint
        );
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), parent_fingerprint);
}
