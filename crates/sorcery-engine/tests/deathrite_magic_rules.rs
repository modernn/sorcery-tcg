//! Direct proofs for nearby-control Magic and Deathrite (RULE-CATALOG-0659–0660,
//! RULE-CATALOG-0977).
//!
//! `gainControlOfTargetNearbyMinion` transfers a nearby minion to the caster.
//! Deathrite follows the new controller: targeted Magic is a non-unit source,
//! so killing the stolen minion draws a site for the thief while the corpse
//! still enters the owner's cemetery. A far minion is not a legal steal, and
//! the same Magic damage still resolves Deathrite for the original controller.
//! Distinct from the private Mesmerism fight path, `0977` proves the draw on a
//! later turn via Magic damage rather than immediate same-turn or combat death.

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

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn deathrite() -> Value {
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

fn mesmerism() -> Value {
    json!({
        "cardType": "magic",
        "gainControlOfTargetNearbyMinion": true,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn deathrite_magic_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "deathrite-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-deathrite-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lash": lash(),
            "north-mesmerism": mesmerism(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-mesmerism",
                    "north-lash",
                    "north-mesmerism",
                    "north-lash",
                    "north-mesmerism",
                    "north-lash"
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid Deathrite Magic session");
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
    ["north-mesmerism", "north-lash"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_magic_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_both_spells(&state(&preview)))
        })
        .expect("bounded seed with Mesmerism and Lash in the opening hand")
}

fn stage_south_minion(session: &mut Session, cell: &str) -> String {
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
            && descriptor["cell"] == cell
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

fn mesmerism_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Mesmerism actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-mesmerism"
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
fn rule_catalog_0659_mesmerism_transfers_a_minion_and_its_deathrite_to_the_new_controller() {
    let encoded = seed_with(659);
    let mut session = opening_main(&encoded);
    let nearby_id = stage_south_minion(&mut session, "C4");
    assert_eq!(mesmerism_targets(&session), [nearby_id.as_str()]);

    let (_, stolen) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&stolen),
        ["magic-cast", "minion-control-changed", "magic-resolved"]
    );
    let changed = stolen
        .events
        .iter()
        .find(|event| event.event_type == "minion-control-changed")
        .expect("control change");
    assert_eq!(changed.payload["fromSeat"], "south");
    assert_eq!(changed.payload["seat"], "north");
    let stolen_state = state(&session);
    let transferred = realm_unit(&stolen_state, &nearby_id).expect("stolen minion");
    assert_eq!(transferred["controller"], "north");
    assert_eq!(transferred["owner"], "south");

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == nearby_id
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
    assert_eq!(killed.events[1].payload["targetInstanceId"], nearby_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &nearby_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &nearby_id));
    assert!(!cemetery_has(&finished, "north", &nearby_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0660_mesmerism_does_not_steal_a_far_minion_and_deathrite_stays_with_controller() {
    let encoded = seed_with(660);
    let mut session = opening_main(&encoded);
    let far_id = stage_south_minion(&mut session, "C1");
    assert!(
        mesmerism_targets(&session).is_empty(),
        "a far minion must not be a nearby-control target"
    );

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
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert!(cemetery_has(&finished, "south", &far_id));
    assert_exact_replay(&session);
}

fn end_then_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

#[test]
fn rule_catalog_0977_mesmerism_deathrite_draws_for_new_controller_on_delayed_kill_not_only_fight() {
    let encoded = seed_with(977);
    let mut session = opening_main(&encoded);
    let nearby_id = stage_south_minion(&mut session, "C4");
    assert_eq!(mesmerism_targets(&session), [nearby_id.as_str()]);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    let stolen_state = state(&session);
    let stolen = realm_unit(&stolen_state, &nearby_id).expect("stolen minion");
    assert_eq!(stolen["controller"], "north");
    assert_eq!(stolen["owner"], "south");

    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "spellbook");
    let delayed = state(&session);
    let still_stolen = realm_unit(&delayed, &nearby_id).expect("minion still controlled");
    assert_eq!(still_stolen["controller"], "north");
    assert_eq!(still_stolen["owner"], "south");

    let north_atlas = atlas_len(&delayed, "north");
    let south_atlas = atlas_len(&delayed, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == nearby_id
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
    assert_eq!(killed.events[1].payload["targetInstanceId"], nearby_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");
    assert_eq!(drawn.payload["sourceInstanceId"], nearby_id);

    let finished = state(&session);
    assert!(realm_unit(&finished, &nearby_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &nearby_id));
    assert!(!cemetery_has(&finished, "north", &nearby_id));
    assert_exact_replay(&session);
}
