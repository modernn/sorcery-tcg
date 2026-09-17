//! Control Magic admission matrix (RULE-CATALOG-0735) and runtime proofs beyond
//! admission for distant enemy-minion control (RULE-CATALOG-0513–0514 and
//! RULE-CATALOG-0515–0516).

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

#[test]
fn rule_catalog_0735_control_magic_admits_common_minion_slices() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0735_control_magic_admits_common_minion_slices();
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

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn betrayal() -> Value {
    json!({
        "cardType": "magic",
        "gainControlOfTargetEnemyMinionThisTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn infiltrate() -> Value {
    json!({
        "cardType": "magic",
        "gainControlOfTargetEnemyMinionUntilStealthLost": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_far() -> Value {
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

fn stealth_bound_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "stealth-bound-control-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-stealth-bound-control-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-infiltrate": infiltrate(),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_far(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-infiltrate",
                    "north-lash",
                    "north-infiltrate",
                    "north-lash",
                    "north-infiltrate",
                    "north-lash",
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
    }))
}

fn this_turn_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "this-turn-control-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-this-turn-control-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-betrayal": betrayal(),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_far(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-betrayal",
                    "north-lash",
                    "north-betrayal",
                    "north-lash",
                    "north-betrayal",
                    "north-lash",
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

fn north_has_both_spells(snapshot: &Value) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    ["north-betrayal", "north-lash"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn north_has_infiltrate_and_lash(snapshot: &Value) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    ["north-infiltrate", "north-lash"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(this_turn_deathrite_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_both_spells(&state(&preview)))
        })
        .expect("bounded seed with Betrayal and Lash in the opening hand")
}

fn stealth_bound_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(stealth_bound_deathrite_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_infiltrate_and_lash(&state(&preview)))
        })
        .expect("bounded seed with Infiltrate and Lash in the opening hand")
}

fn stage_far_deathrite(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned()
}

fn betrayal_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Betrayal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-betrayal"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn infiltrate_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Infiltrate actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-infiltrate"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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
fn rule_catalog_0930_this_turn_control_transfers_distant_deathrite_to_thief_before_end_phase() {
    let encoded = seed_with(930);
    let mut session = Session::new(&encoded).expect("valid this-turn Deathrite control session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let far_id = stage_far_deathrite(&mut session);
    assert_eq!(betrayal_targets(&session), [far_id.as_str()]);

    let (_, stolen) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-betrayal"
            && descriptor["target"]["instanceId"] == far_id
    });
    assert_eq!(
        event_types(&stolen),
        ["magic-cast", "minion-control-changed", "magic-resolved"]
    );
    let stolen_state = state(&session);
    let transferred = realm_unit(&stolen_state, &far_id).expect("stolen minion");
    assert_eq!(transferred["controller"], "north");
    assert_eq!(transferred["owner"], "south");

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == far_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], far_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &far_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &far_id));
    assert!(!cemetery_has(&finished, "north", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0939_stealth_bound_control_transfers_distant_deathrite_to_thief_before_stealth_lost() {
    let encoded = stealth_bound_seed_with(939);
    let mut session =
        Session::new(&encoded).expect("valid stealth-bound Deathrite control session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let far_id = stage_far_deathrite(&mut session);
    assert_eq!(infiltrate_targets(&session), [far_id.as_str()]);

    let (_, stolen) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-infiltrate"
            && descriptor["target"]["instanceId"] == far_id
    });
    assert_eq!(
        event_types(&stolen),
        [
            "magic-cast",
            "minion-control-changed",
            "minion-stealthed",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    let stolen_state = state(&session);
    let transferred = realm_unit(&stolen_state, &far_id).expect("stolen minion");
    assert_eq!(transferred["controller"], "north");
    assert_eq!(transferred["owner"], "south");
    assert_eq!(transferred["tapped"], true);
    assert_eq!(transferred["stealthed"], true);

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == far_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], far_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &far_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &far_id));
    assert!(!cemetery_has(&finished, "north", &far_id));
    assert_exact_replay(&session);
}
