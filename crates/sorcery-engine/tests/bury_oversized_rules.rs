//! Direct proofs for burrow-target-minion-or-artifact Magic on 2×2 footprints
//! (RULE-CATALOG-0717–0718).
//!
//! Bury eligibility scans every occupied footprint cell. Rubble counts as land.
//! An all-Water square still offers the target, then resolves as a paid no-op.

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

fn oversized_giant() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 10,
        "manaCost": 0,
        "occupiesSquareArea": 2,
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

fn destroy_spell() -> Value {
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

fn north_spellbook() -> Vec<&'static str> {
    std::iter::repeat_n("north-destroy", 4)
        .chain(std::iter::repeat_n("north-bury", 26))
        .collect()
}

fn bury_oversized_manifest(seed: u32, water: bool) -> String {
    let north_site = if water { water_site() } else { earth_site() };
    let fixture = if water {
        "bury-oversized-water"
    } else {
        "bury-oversized-rubble"
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
            "north-bury": bury(),
            "north-destroy": destroy_spell(),
            "north-site": north_site,
            "south-avatar": avatar(),
            "south-giant": oversized_giant(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 12],
                "avatar": "north-avatar",
                "spellbook": north_spellbook(),
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": vec!["south-giant"; 30],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let actions = session.legal_actions().expect("legal actions");
    let action = actions
        .iter()
        .find(|action| predicate(&action.descriptor))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action; phase={} terminal={:?}; available={:?}",
                state(session)["phase"],
                state(session).get("terminal"),
                actions
                    .iter()
                    .map(|action| &action.descriptor)
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn end_and_draw_zone(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn end_and_draw(session: &mut Session) {
    end_and_draw_zone(session, "spellbook");
}

fn establish_north_square(session: &mut Session) {
    keep(session);
    keep(session);
    play_site(session, "C4");
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);
    play_site(session, "B4");
    end_and_draw(session);
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    end_and_draw_zone(session, "atlas");
    play_site(session, "B3");
}

fn destroy_site_at(session: &mut Session, cell: &str) {
    let site_id = state(session)["realm"]["sites"][cell]["instanceId"]
        .as_str()
        .expect("site identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetSiteInstanceId"] == site_id
    });
}

fn draw_until(session: &mut Session, mut ready: impl FnMut(&Session) -> bool) {
    for _ in 0..32 {
        if ready(session) {
            return;
        }
        assert_ne!(
            state(session)["phase"],
            "terminal",
            "setup must stay in play"
        );
        end_and_draw(session);
    }
    panic!("draw_until exhausted without reaching readiness");
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

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn seed_with(water: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| bury_oversized_manifest(seed, water))
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-bury")
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .any(|card| card == "south-giant")
        })
        .expect("bounded seed with Bury and an oversized target")
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

fn setup_oversized_target(encoded: &str, rubble: bool) -> (Session, String) {
    let mut session = Session::new(encoded).expect("valid oversized bury session");
    establish_north_square(&mut session);
    if rubble {
        draw_until(&mut session, |session| {
            state(session)["decisionSeat"] == "north"
                && session.legal_actions().ok().is_some_and(|actions| {
                    actions.iter().any(|action| {
                        action.descriptor["kind"] == "cast-magic"
                            && action.descriptor["cardId"] == "north-destroy"
                            && action.descriptor["targetLocation"]["cell"] == "B4"
                    })
                })
        });
        assert_eq!(
            state(&session)["realm"]["sites"]["B4"]["rubble"],
            Value::Null
        );
        destroy_site_at(&mut session, "B4");
        assert_eq!(state(&session)["realm"]["sites"]["B4"]["rubble"], true);
    }
    end_and_draw(&mut session);
    draw_until(&mut session, |session| {
        state(session)["decisionSeat"] == "south"
            && session.legal_actions().ok().is_some_and(|actions| {
                actions.iter().any(|action| {
                    action.descriptor["kind"] == "summon-minion"
                        && action.descriptor["cardId"] == "south-giant"
                        && action.descriptor["cell"] == "B3"
                })
            })
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-giant"
            && descriptor["cell"] == "B3"
            && descriptor["region"].is_null()
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("oversized target identity")
        .to_owned();
    assert_eq!(
        realm_unit(&state(&session), &target_id).expect("summoned giant")["occupiedCells"],
        json!(["B3", "B4", "C3", "C4"])
    );
    end_and_draw(&mut session);
    draw_until(&mut session, |session| {
        state(session)["decisionSeat"] == "north"
            && session.legal_actions().ok().is_some_and(|actions| {
                actions.iter().any(|action| {
                    action.descriptor["kind"] == "cast-magic"
                        && action.descriptor["cardId"] == "north-bury"
                })
            })
    });
    (session, target_id)
}

#[test]
fn rule_catalog_0717_bury_checks_every_oversized_cell_and_treats_rubble_as_land() {
    let encoded = seed_with(false, 717);
    let (mut session, target_id) = setup_oversized_target(&encoded, true);
    assert!(realm_unit(&state(&session), &target_id).is_some());

    let (cast, settled) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&settled),
        [
            "magic-cast",
            "minion-burrowed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(
        settled.events[1].payload,
        json!({
            "cell": "B3",
            "instanceId": target_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0718_bury_on_all_water_oversized_footprint_is_a_paid_noop() {
    let encoded = seed_with(true, 718);
    let (mut session, target_id) = setup_oversized_target(&encoded, false);
    let before = realm_unit(&state(&session), &target_id)
        .expect("Water footprint target")
        .clone();
    assert_eq!(before["location"], "B3");
    assert_eq!(before["region"], "surface");
    assert_eq!(before["occupiedCells"], json!(["B3", "B4", "C3", "C4"]));

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert_eq!(
        realm_unit(&state(&session), &target_id).expect("unchanged target"),
        &before
    );
    assert_exact_replay(&session);
}
