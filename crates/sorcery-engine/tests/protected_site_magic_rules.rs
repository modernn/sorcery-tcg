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

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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
    let (summoned, _) = accept_where(session, |descriptor| {
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
        south_site_id,
        summoned["cardInstanceId"]
            .as_str()
            .expect("summoned minion identity")
            .to_owned(),
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

fn cast_crater_at_c1(session: &mut Session, south_site_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-crater"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
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
fn rule_catalog_1082_craterize_deathrite_draws_for_controller_on_kill() {
    let encoded = seed_with_deathrite(1082);
    let mut session = opening_main(&encoded);
    let (south_site_id, target_id) = one_south_deathrite_at_c1(&mut session);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (_, receipt) = cast_crater_at_c1(&mut session, &south_site_id);
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-destroyed",
            "magic-damage-allocated",
            "magic-damage-allocated",
            "damage-dealt",
            "damage-dealt",
            "avatar-life-lost",
            "rubble-created",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    let allocated = receipt
        .events
        .iter()
        .find(|event| {
            event.event_type == "magic-damage-allocated"
                && event.payload["targetInstanceId"] == target_id
        })
        .expect("grid allocation to Deathrite minion");
    assert_eq!(allocated.payload["amount"], 1);
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], target_id);
    let types = event_types(&receipt);
    let damage_dealt = types
        .iter()
        .position(|event_type| *event_type == "damage-dealt")
        .expect("damage-dealt index");
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let minion_died = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion-died index");
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        damage_dealt < site_drawn && site_drawn < minion_died && minion_died < magic_resolved,
        "expected damage-dealt, deathrite site-drawn, minion-died, then magic-resolved; got {types:?}"
    );
    assert_eq!(types.last(), Some(&"magic-resolved"));

    let finished = state(&session);
    assert!(realm_unit(&finished, &target_id).is_none());
    assert_eq!(finished["realm"]["sites"]["C1"]["rubble"], true);
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}
