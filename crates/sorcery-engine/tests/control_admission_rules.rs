//! Control Magic admission matrix (RULE-CATALOG-0735) and runtime proofs beyond
//! admission for distant enemy-minion control (RULE-CATALOG-0513–0514 and
//! RULE-CATALOG-0515–0516, and RULE-CATALOG-2583–2588).

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
        "tapForMana": 1,
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
fn rule_catalog_0972_this_turn_control_deathrite_draws_for_original_controller_when_stolen_minion_dies_after_revert()
 {
    let encoded = seed_with(972);
    let mut session = Session::new(&encoded).expect("valid this-turn Deathrite control session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let far_id = stage_far_deathrite(&mut session);
    assert_eq!(betrayal_targets(&session), [far_id.as_str()]);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-betrayal"
            && descriptor["target"]["instanceId"] == far_id
    });
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("stolen minion")["controller"],
        "north"
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == far_id
    }));
    let reverted = state(&session);
    assert_eq!(
        realm_unit(&reverted, &far_id).expect("reverted minion")["controller"],
        "south"
    );
    assert_eq!(
        realm_unit(&reverted, &far_id).expect("reverted minion")["owner"],
        "south"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
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
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &far_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert!(cemetery_has(&finished, "south", &far_id));
    assert!(!cemetery_has(&finished, "north", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0939_stealth_bound_control_transfers_distant_deathrite_to_thief_before_stealth_lost()
 {
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

#[test]
fn rule_catalog_0973_stealth_bound_deathrite_draws_for_original_controller_when_stolen_minion_dies_after_stealth_revert()
 {
    let encoded = stealth_bound_seed_with(973);
    let mut session =
        Session::new(&encoded).expect("valid stealth-bound Deathrite control session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let far_id = stage_far_deathrite(&mut session);
    assert_eq!(infiltrate_targets(&session), [far_id.as_str()]);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-infiltrate"
            && descriptor["target"]["instanceId"] == far_id
    });
    let stolen_state = state(&session);
    let stolen = realm_unit(&stolen_state, &far_id).expect("stolen minion");
    assert_eq!(stolen["controller"], "north");
    assert_eq!(stolen["owner"], "south");
    assert_eq!(stolen["stealthed"], true);

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !ended
            .events
            .iter()
            .any(|event| event.event_type == "minion-control-changed")
    );
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("still stolen")["controller"],
        "north"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let prep_state = state(&session);
    let reverted_prep = realm_unit(&prep_state, &far_id).expect("pre-stealth-loss minion");
    assert_eq!(reverted_prep["controller"], "north");
    assert_eq!(reverted_prep["tapped"], false);

    let (_, interacted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == far_id.as_str()
    });
    assert!(interacted.events.iter().any(|event| {
        event.event_type == "stealth-lost" && event.payload["instanceId"] == far_id
    }));
    assert!(interacted.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == far_id
    }));
    let reverted_state = state(&session);
    let reverted = realm_unit(&reverted_state, &far_id).expect("reverted minion");
    assert_eq!(reverted["controller"], "south");
    assert_eq!(reverted["owner"], "south");
    assert_eq!(reverted["stealthed"], false);

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
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &far_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert!(cemetery_has(&finished, "south", &far_id));
    assert!(!cemetery_has(&finished, "north", &far_id));
    assert_exact_replay(&session);
}

fn burrower() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
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

fn control_admission_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "control-admission-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-control-admission-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-betrayal": betrayal(),
            "north-bury": bury(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-burrower": burrower(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": std::iter::repeat_n("north-betrayal", 8)
                    .chain(std::iter::repeat_n("north-bury", 4))
                    .collect::<Vec<_>>(),
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-burrower"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    realm_unit(snapshot, instance_id).expect("expected realm unit")
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

fn opening_hand_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("opening Spellbook")
        .iter()
        .map(|card| {
            card["cardId"]
                .as_str()
                .expect("hand card identity")
                .to_owned()
        })
        .collect()
}

fn opening_south_burrowers(encoded: &str) -> usize {
    opening_hand_spell_ids(encoded, "south")
        .iter()
        .filter(|card| *card == "south-burrower")
        .count()
}

fn seed_has_control_admission_spells(encoded: &str) -> bool {
    opening_hand_spell_ids(encoded, "north")
        .iter()
        .any(|card| card == "north-betrayal")
        && opening_hand_spell_ids(encoded, "north")
            .iter()
            .any(|card| card == "north-bury")
}

fn supplemental_seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(735..735 + 2048)
        .map(control_admission_supplemental_manifest)
        .find(|candidate| {
            seed_has_control_admission_spells(candidate)
                && opening_south_burrowers(candidate) >= required_south
        })
        .expect("bounded seed with Betrayal, Bury, and required South burrowers")
}

fn opening_main_supplemental(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid control-admission supplemental session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn betrayal_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-betrayal")
                .count()
        })
        .unwrap_or_default()
}

fn cast_betrayal_on(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-betrayal"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn try_bury_minion(session: &mut Session, instance_id: &str) -> Option<()> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == instance_id
    })
    .map(|_| ())
}

fn bury_minion(session: &mut Session, instance_id: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == instance_id
    });
}

fn try_summon_south_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-burrower"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    summoned["cardInstanceId"].as_str().map(str::to_owned)
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    try_summon_south_at(session, cell).expect("summon south burrower")
}

fn stage_enemies(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut enemy_ids = Vec::new();
    for _ in 0..count {
        enemy_ids.push(summon_south_at(session, "C1"));
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_ids
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
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

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    end_turn_if_offered(session);
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn try_second_betrayal_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String, String)> {
    if !seed_has_control_admission_spells(encoded) || opening_south_burrowers(encoded) < 2 {
        return None;
    }
    let mut session = opening_main_supplemental(encoded);
    let enemy_ids = stage_enemies(&mut session, 1);
    let buried_id = enemy_ids[0].clone();
    bury_minion(&mut session, &buried_id);
    if unit(&state(&session), &buried_id)["region"] != "underground" {
        return None;
    }
    if betrayal_targets(&session).contains(&buried_id) {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if betrayal_spells_in_hand(&state(&session)) < 1 {
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
    let visitor_id = try_summon_south_at(&mut session, "C2")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    betrayal_targets(&session)
        .contains(&visitor_id)
        .then_some((session, visitor_id, buried_id))
}

fn seed_for_second_betrayal_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(735..735 + 8192)
        .find_map(|seed| {
            let encoded = control_admission_supplemental_manifest(seed);
            try_second_betrayal_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Betrayal enemy-arrival setup")
}

fn try_second_betrayal_new_summon_prefix(encoded: &str) -> Option<(Session, String, String)> {
    if !seed_has_control_admission_spells(encoded) || opening_south_burrowers(encoded) < 2 {
        return None;
    }
    let mut session = opening_main_supplemental(encoded);
    let enemy_ids = stage_enemies(&mut session, 1);
    let buried_id = enemy_ids[0].clone();
    bury_minion(&mut session, &buried_id);
    if unit(&state(&session), &buried_id)["region"] != "underground" {
        return None;
    }
    try_pass_turn_to_north_spellbook(&mut session)?;
    if betrayal_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    let new_id = try_summon_south_at(&mut session, "C1")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    betrayal_targets(&session)
        .contains(&new_id)
        .then_some((session, new_id, buried_id))
}

fn seed_for_second_betrayal_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(735..735 + 8192)
        .find_map(|seed| {
            let encoded = control_admission_supplemental_manifest(seed);
            try_second_betrayal_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Betrayal new-summon setup")
}

fn try_setup_two_surface_and_one_buried(
    encoded: &str,
) -> Option<(Session, String, String, String)> {
    if opening_south_burrowers(encoded) < 3 {
        return None;
    }
    let mut session = opening_main_supplemental(encoded);
    let enemy_ids = stage_enemies(&mut session, 2);
    let first_id = enemy_ids[0].clone();
    let buried_id = enemy_ids[1].clone();
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let second_id = try_summon_south_at(&mut session, "C2")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_bury_minion(&mut session, &buried_id)?;
    let offered = betrayal_targets(&session);
    (offered.contains(&first_id)
        && offered.contains(&second_id)
        && !offered.contains(&buried_id)
        && offered.len() == 2)
        .then_some((session, first_id, second_id, buried_id))
}

fn seed_for_two_surface_and_one_buried(start: u32) -> String {
    (start..start + 8192)
        .chain(735..735 + 8192)
        .find_map(|seed| {
            let encoded = control_admission_supplemental_manifest(seed);
            try_setup_two_surface_and_one_buried(&encoded).map(|_| encoded)
        })
        .expect("bounded seed with two surface burrowers and one buried copy")
}

#[test]
fn rule_catalog_2583_burrowing_enemy_still_offered_for_betrayal_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2583, 1);
    let mut session = opening_main_supplemental(&encoded);
    let minion_id = stage_enemies(&mut session, 1)[0].clone();
    assert!(betrayal_targets(&session).contains(&minion_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(betrayal_targets(&session).contains(&minion_id));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "south");
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C1");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2584_second_betrayal_on_the_stolen_burrower_is_a_paid_noop() {
    let encoded = (2584..2584 + 8192)
        .chain(513..513 + 8192)
        .chain(735..735 + 8192)
        .find_map(|seed| {
            let candidate = control_admission_supplemental_manifest(seed);
            if !seed_has_control_admission_spells(&candidate) {
                return None;
            }
            let mut session = opening_main_supplemental(&candidate);
            let minion_id = stage_enemies(&mut session, 1)[0].clone();
            let first = cast_betrayal_on(&mut session, &minion_id);
            if !event_types(&first).contains(&"minion-control-changed") {
                return None;
            }
            if unit(&state(&session), &minion_id)["controller"] != "north" {
                return None;
            }
            (betrayal_spells_in_hand(&state(&session)) >= 1
                && betrayal_targets(&session).contains(&minion_id))
            .then_some(candidate)
        })
        .expect("bounded seed with two Betrayal casts after stealing the burrower");
    let mut session = opening_main_supplemental(&encoded);
    let minion_id = stage_enemies(&mut session, 1)[0].clone();
    let first = cast_betrayal_on(&mut session, &minion_id);
    assert!(event_types(&first).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert!(betrayal_spells_in_hand(&state(&session)) >= 1);
    assert!(betrayal_targets(&session).contains(&minion_id));
    let second = cast_betrayal_on(&mut session, &minion_id);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2585_second_betrayal_steals_a_newly_arrived_burrower_after_enemy_site_placement() {
    let encoded = seed_for_second_betrayal_enemy_arrival(2585);
    let (mut session, minion_id, buried_id) = try_second_betrayal_enemy_arrival_prefix(&encoded)
        .expect("second Betrayal enemy-arrival prefix");
    let receipt = cast_betrayal_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C2");
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2586_betrayal_offers_every_surface_burrower_not_underground() {
    let encoded = seed_for_two_surface_and_one_buried(2586);
    let (session, first_id, second_id, buried_id) = try_setup_two_surface_and_one_buried(&encoded)
        .expect("Betrayal admission-matrix multi-target prefix");
    let offered = betrayal_targets(&session);
    assert!(offered.contains(&first_id));
    assert!(offered.contains(&second_id));
    assert!(!offered.contains(&buried_id));
    assert_eq!(offered.len(), 2);
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2587_betrayal_leaves_a_burrowed_underground_minion_untouched() {
    let encoded = supplemental_seed_with_start(2587, 2);
    let mut session = opening_main_supplemental(&encoded);
    let enemy_ids = stage_enemies(&mut session, 2);
    let surface_id = enemy_ids[0].clone();
    let buried_id = enemy_ids[1].clone();
    bury_minion(&mut session, &buried_id);
    let receipt = cast_betrayal_on(&mut session, &surface_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &surface_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &buried_id)["controller"], "south");
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_eq!(unit(&state(&session), &buried_id)["location"], "C1");
    assert!(!betrayal_targets(&session).contains(&buried_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2588_second_betrayal_steals_a_newly_summoned_burrower() {
    let encoded = seed_for_second_betrayal_new_summon(2588);
    let (mut session, minion_id, buried_id) =
        try_second_betrayal_new_summon_prefix(&encoded).expect("second Betrayal new-summon prefix");
    let receipt = cast_betrayal_on(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-control-changed"));
    assert_eq!(unit(&state(&session), &minion_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &minion_id)["owner"], "south");
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C1");
    assert_eq!(unit(&state(&session), &buried_id)["region"], "underground");
    assert_exact_replay(&session);
}
