//! Direct proofs for local carried Artifacts (RULE-CATALOG-0140) and the power an Artifact
//! carries away from a lethally wounded bearer when it is dropped (RULE-CATALOG-0141).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
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

fn power_artifact(mana_cost: u64) -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": mana_cost,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(revision: &str, cards: &Value, decks: &Value, seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "artifact-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": revision,
        },
        "cards": cards,
        "decks": decks,
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
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

fn end_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_artifacts(current: &Value) -> Vec<Value> {
    current["realm"]["artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
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

/// The joined Artifact selections each action offers, keyed by the unit that would act.
fn selections(descriptors: &[Value], unit_kind: &str) -> Vec<String> {
    let mut joined: Vec<_> = descriptors
        .iter()
        .filter(|descriptor| descriptor["unit"]["kind"] == unit_kind)
        .map(|descriptor| {
            descriptor["artifactInstanceIds"]
                .as_array()
                .expect("selected Artifact identities")
                .iter()
                .map(|instance_id| instance_id.as_str().expect("identity").to_owned())
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    joined.sort();
    joined
}

fn play_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn summon(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn carry_scenario() -> String {
    let cards = json!({
        "carry-avatar": avatar(),
        "carry-bearer": minion(json!({ "manaCost": 1 })),
        "carry-raider": minion(json!({ "manaCost": 1, "summonToAnySite": true })),
        "carry-site": { "cardType": "site", "elements": ["earth"], "genesisGainMana": 6 },
        "sword-and-shield": power_artifact(2),
    });
    let decks = json!({
        "north": {
            "atlas": vec!["carry-site"; 6],
            "avatar": "carry-avatar",
            "spellbook": [
                "sword-and-shield",
                "sword-and-shield",
                "carry-bearer",
                "sword-and-shield",
                "carry-bearer",
                "sword-and-shield",
            ],
        },
        "south": {
            "atlas": vec!["carry-site"; 6],
            "avatar": "carry-avatar",
            "spellbook": vec!["carry-raider"; 6],
        },
    });
    (1..=4096)
        .map(|seed| manifest("synthetic-artifact-carry-v1", &cards, &decks, seed))
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("carry candidate"));
            let hand = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("opening spellbook hand")
                .clone();
            hand.iter()
                .filter(|card| card["cardId"] == "sword-and-shield")
                .count()
                >= 2
                && hand.iter().any(|card| card["cardId"] == "carry-bearer")
        })
        .expect("bounded seed opening with two Artifacts and one bearer in hand")
}

/// Opens the carry scenario with a bearer and two uncarried Artifacts sharing North's only site.
fn opened_carry_scenario() -> (Session, String, Vec<String>) {
    let mut session = Session::new(&carry_scenario()).expect("valid carry scenario");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "carry-site", "C4");
    let bearer = summon(&mut session, "carry-bearer", "C4");
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardId"] == "sword-and-shield"
                && descriptor["cell"] == "C4"
                && descriptor["bearer"].is_null()
        });
    }
    let mut loose: Vec<String> = realm_artifacts(&state(&session))
        .iter()
        .map(|artifact| {
            assert_eq!(artifact["location"], "C4");
            assert_eq!(artifact["region"], "surface");
            artifact["instanceId"]
                .as_str()
                .expect("identity")
                .to_owned()
        })
        .collect();
    loose.sort();
    assert_eq!(loose.len(), 2);
    (session, bearer, loose)
}

#[test]
fn rule_catalog_0140_pick_up_and_drop_should_manage_local_carried_artifacts_once_per_unit_turn() {
    let (mut session, bearer, loose) = opened_carry_scenario();
    let every_subset = {
        let mut subsets = vec![
            loose[0].clone(),
            loose[1].clone(),
            format!("{},{}", loose[0], loose[1]),
        ];
        subsets.sort();
        subsets
    };

    // North's Avatar and its bearer both stand on C4, so each may take any nonempty subset.
    let offered = descriptors_of_kind(&session, "pick-up-artifacts");
    assert_eq!(offered.len(), 6);
    assert_eq!(selections(&offered, "avatar"), every_subset);
    assert_eq!(selections(&offered, "minion"), every_subset);

    let (_, picked) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts"
            && descriptor["unit"]["instanceId"] == bearer.as_str()
            && descriptor["artifactInstanceIds"]
                .as_array()
                .expect("selection")
                .len()
                == 2
    });
    assert_eq!(
        picked
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-picked-up"]
    );
    assert!(realm_artifacts(&state(&session)).iter().all(|artifact| {
        artifact["bearer"]["instanceId"] == bearer.as_str()
            && artifact["bearer"]["kind"] == "minion"
            && artifact["location"].is_null()
    }));

    // The bearer has spent its Pick Up for the turn, and nothing is left loose for the Avatar.
    assert!(descriptors_of_kind(&session, "pick-up-artifacts").is_empty());
    let droppable = descriptors_of_kind(&session, "drop-artifacts");
    assert_eq!(selections(&droppable, "minion"), every_subset);
    assert!(selections(&droppable, "avatar").is_empty());

    let (dropped_descriptor, dropped) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "drop-artifacts"
            && descriptor["unit"]["instanceId"] == bearer.as_str()
            && descriptor["artifactInstanceIds"]
                .as_array()
                .expect("selection")
                .len()
                == 1
    });
    assert_eq!(
        dropped
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-dropped"]
    );
    let released = dropped_descriptor["artifactInstanceIds"][0]
        .as_str()
        .expect("released identity")
        .to_owned();

    // One Drop per unit turn: the bearer keeps its second Artifact until its next turn.
    assert!(selections(&descriptors_of_kind(&session, "drop-artifacts"), "minion").is_empty());
    // The released Artifact lands where its bearer stands and is local to the Avatar there.
    assert_eq!(
        selections(
            &descriptors_of_kind(&session, "pick-up-artifacts"),
            "avatar"
        ),
        vec![released]
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "pick-up-artifacts" && descriptor["unit"]["kind"] == "avatar"
    });
    assert!(descriptors_of_kind(&session, "pick-up-artifacts").is_empty());

    // South's Avatar sits at C1, so the Artifacts on C4 are out of reach until a unit stands there.
    end_and_draw(&mut session);
    play_site(&mut session, "carry-site", "C1");
    assert!(descriptors_of_kind(&session, "pick-up-artifacts").is_empty());
    assert_exact_replay(&session);
}

fn lethal_drop_scenario() -> String {
    let cards = json!({
        "drop-avatar": avatar(),
        "drop-bearer": minion(json!({})),
        "drop-servant": minion(json!({ "genesisDamageEachOtherUnitHere": 1 })),
        "drop-site": { "cardType": "site", "elements": [] },
        "drop-sword": power_artifact(0),
    });
    let decks = json!({
        "north": {
            "atlas": vec!["drop-site"; 3],
            "avatar": "drop-avatar",
            "spellbook": ["drop-bearer", "drop-sword", "drop-servant"],
        },
        "south": {
            "atlas": vec!["drop-site"; 3],
            "avatar": "drop-avatar",
            "spellbook": vec!["drop-bearer"; 3],
        },
    });
    (1..=4096)
        .map(|seed| manifest("synthetic-artifact-drop-v1", &cards, &decks, seed))
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("drop candidate"));
            ["drop-bearer", "drop-servant", "drop-sword"]
                .into_iter()
                .all(|card_id| {
                    opening["players"]["north"]["hand"]["spellbook"]
                        .as_array()
                        .expect("opening spellbook hand")
                        .iter()
                        .any(|card| card["cardId"] == card_id)
                })
        })
        .expect("bounded seed opening with the bearer, its sword, and the servant in hand")
}

#[test]
fn rule_catalog_0141_dropping_a_power_artifact_should_kill_a_lethally_wounded_bearer() {
    let mut session = Session::new(&lethal_drop_scenario()).expect("valid lethal Drop scenario");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "drop-site", "C4");
    let bearer = summon(&mut session, "drop-bearer", "C4");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "drop-sword"
            && descriptor["bearer"]["instanceId"] == bearer.as_str()
    });
    // The servant's Genesis wounds the bearer for one, which its carried power keeps survivable.
    let (_, wounded) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "drop-servant"
    });
    assert!(
        !wounded
            .events
            .iter()
            .any(|event| event.event_type == "minion-died")
    );
    let alive = state(&session);
    assert_eq!(
        alive["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == bearer.as_str())
            .expect("wounded bearer")["damage"],
        1
    );

    let (_, released) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "drop-artifacts"
            && descriptor["unit"]["instanceId"] == bearer.as_str()
    });
    assert_eq!(
        released
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["artifacts-dropped", "minion-died"]
    );
    assert!(released.random_draws.is_empty());

    let settled = state(&session);
    assert!(
        !settled["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == bearer.as_str())
    );
    assert!(
        settled["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == bearer.as_str())
    );
    let artifacts = realm_artifacts(&settled);
    assert_eq!(artifacts.len(), 1);
    assert!(artifacts[0]["bearer"].is_null());
    assert_eq!(artifacts[0]["location"], "C4");
    assert_eq!(artifacts[0]["region"], "surface");
    assert_exact_replay(&session);
}
