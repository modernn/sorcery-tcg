//! Siege Ballista branches the TypeScript setup suite used to prove with forged state.
//!
//! `tests/engine/game-setup-07.test.ts` hand-edited a live session (moved/tapped helper,
//! disabled/uncarried bearer, stealthed/underground units) to show which Siege Ballista
//! activation offers survive each condition. `rule_catalog_0143_siege_ballista_should_tap_bearer_and_ally_for_measured_artifact_damage`,
//! `a_siege_ballista_should_shoot_the_ally_that_paid_its_second_tap`, and
//! `a_siege_ballista_should_require_both_its_bearer_and_a_second_ready_ally` in
//! `artifact_rules.rs` already cover the loose-Ballista, stealthed-target, far-range, and
//! spent-bearer-or-ally branches through real play; these two reach the remaining ones
//! (a disabled bearer, and a bearer/helper/target that all share the underground region)
//! the same way.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

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

fn descriptors_of_kind(session: &Session, kind: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .map(|action| action.descriptor)
        .filter(|descriptor| descriptor["kind"] == kind)
        .collect()
}

fn end_turn_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
}

fn manifest(
    seed: u32,
    extra_cards: &Value,
    north_spellbook: &[&str],
    south_spellbook: &[&str],
) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut cards = json!({
        "north-avatar": avatar,
        "north-site": site,
        "south-avatar": avatar,
        "south-site": site,
    });
    cards.as_object_mut().expect("card definitions").extend(
        extra_cards
            .as_object()
            .expect("extra card definitions")
            .clone(),
    );
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "artifact-cutover" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-artifact-cutover-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn siege_ballista(extra: Value) -> Value {
    let mut value = json!({
        "cardType": "artifact",
        "manaCost": 0,
        "tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps": 3,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra Siege Ballista facts must be an object");
    };
    value
        .as_object_mut()
        .expect("Siege Ballista facts")
        .extend(extra);
    value
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

#[test]
fn disabled_siege_ballista_bearer_offers_no_activation() {
    let manifest = manifest(
        61,
        &json!({
            "north-bearer": minion(json!({ "genesisDisableSelfUntilDamaged": true })),
            "north-helper": minion(json!({})),
            "siege-ballista": siege_ballista(json!({})),
        }),
        &["north-bearer", "north-helper", "siege-ballista"],
        &["north-helper"; 3],
    );
    let mut session = Session::new(&manifest).expect("valid disabled-bearer scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (bearer_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-bearer"
    });
    let bearer_id = bearer_summon["cardInstanceId"]
        .as_str()
        .expect("bearer identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-helper"
    });
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == bearer_id)
            .expect("disabled bearer")["disabledUntilDamaged"],
        json!(true)
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "siege-ballista"
            && descriptor["bearer"]["instanceId"] == bearer_id
    });
    end_turn_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_turn_and_draw(&mut session);

    assert!(
        descriptors_of_kind(&session, "activate-artifact-damage").is_empty(),
        "a disabled bearer must offer no Siege Ballista activation, even beside a ready ally"
    );
}

#[test]
fn siege_ballista_should_work_from_matching_underground_positions() {
    let manifest = manifest(
        62,
        &json!({
            "north-bearer": minion(json!({ "burrowing": true })),
            "north-helper": minion(json!({ "burrowing": true })),
            "siege-ballista": siege_ballista(json!({})),
            "south-target": minion(json!({
                "burrowing": true,
                "defense": 5,
                "summonToAnySite": true,
            })),
        }),
        &["north-bearer", "north-helper", "siege-ballista"],
        &["south-target"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid underground Ballista scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (bearer_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-bearer"
            && descriptor["region"] == "underground"
    });
    let bearer_id = bearer_summon["cardInstanceId"]
        .as_str()
        .expect("bearer identity")
        .to_owned();
    let (helper_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-helper"
            && descriptor["region"] == "underground"
    });
    let helper_id = helper_summon["cardInstanceId"]
        .as_str()
        .expect("helper identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "siege-ballista"
            && descriptor["bearer"]["instanceId"] == bearer_id
    });
    end_turn_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (target_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underground"
    });
    let target_id = target_summon["cardInstanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    end_turn_and_draw(&mut session);

    let offered = descriptors_of_kind(&session, "activate-artifact-damage");
    assert!(
        offered.iter().any(|descriptor| {
            descriptor["helper"]["instanceId"] == helper_id
                && descriptor["target"]["instanceId"] == target_id
        }),
        "a Siege Ballista whose bearer, helper, and target all share the underground region \
         must still offer activation"
    );

    let (_, fired) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-artifact-damage"
            && descriptor["helper"]["instanceId"] == helper_id
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        fired
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "artifact-damage-activated",
            "artifact-damage-allocated",
            "damage-dealt",
        ]
    );
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == target_id)
            .expect("underground target")["damage"],
        json!(3)
    );
}
