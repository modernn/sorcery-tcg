//! Direct proofs for local carried Artifacts (RULE-CATALOG-0140), the power an Artifact
//! carries away from a lethally wounded bearer when it is dropped (RULE-CATALOG-0141), the
//! Lethal a carried Artifact grants its bearer until the bearer falls (RULE-CATALOG-0142), and the
//! measured damage a Siege Ballista shoots for its bearer's tap plus another ally's
//! (RULE-CATALOG-0143).

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

fn lethal_artifact(mana_cost: u64) -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerLethal": true,
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

fn end_and_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
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

/// The realm unit with this identity, absent once it has died.
fn realm_unit(current: &Value, instance_id: &str) -> Option<Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .cloned()
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
    end_and_draw(&mut session, "spellbook");
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
    assert_eq!(
        realm_unit(&state(&session), &bearer).expect("wounded bearer")["damage"],
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
    assert!(realm_unit(&settled, &bearer).is_none());
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

fn lethal_strike_scenario() -> String {
    let cards = json!({
        "dagger-avatar": avatar(),
        "dagger-bearer": minion(json!({
            "attack": 2,
            "defense": 2,
            "manaCost": 1,
            "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
        })),
        "dagger-enemy": minion(json!({
            "attack": 2,
            "charge": true,
            "defense": 3,
            "manaCost": 1,
            "summonToAnySite": true,
            "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
        })),
        "dagger-site": { "cardType": "site", "elements": ["earth"], "genesisGainMana": 6 },
        "poisonous-dagger": lethal_artifact(2),
    });
    let decks = json!({
        "north": {
            "atlas": vec!["dagger-site"; 6],
            "avatar": "dagger-avatar",
            "spellbook": [
                "poisonous-dagger",
                "dagger-bearer",
                "poisonous-dagger",
                "dagger-bearer",
                "poisonous-dagger",
                "dagger-bearer",
            ],
        },
        "south": {
            "atlas": vec!["dagger-site"; 6],
            "avatar": "dagger-avatar",
            "spellbook": vec!["dagger-enemy"; 6],
        },
    });
    (1..=4096)
        .map(|seed| manifest("synthetic-lethal-artifact-v1", &cards, &decks, seed))
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("lethal strike candidate"));
            ["dagger-bearer", "poisonous-dagger"]
                .into_iter()
                .all(|card_id| {
                    opening["players"]["north"]["hand"]["spellbook"]
                        .as_array()
                        .expect("opening spellbook hand")
                        .iter()
                        .any(|card| card["cardId"] == card_id)
                })
        })
        .expect("bounded seed opening with the bearer and its dagger in hand")
}

/// Walks both seats up to the strike exchange between North's two-power bearer at C3 and the
/// three-defense enemy that charges in to attack it. The dagger is conjured either onto the bearer
/// or loose on the same cell, which is the only difference between the two outcomes.
fn lethal_strike_position(session: &mut Session, carried: bool) -> (String, String) {
    play_site(session, "dagger-site", "C4");
    let bearer = summon(session, "dagger-bearer", "C4");
    end_and_draw(session, "spellbook");
    play_site(session, "dagger-site", "C1");
    end_and_draw(session, "spellbook");

    play_site(session, "dagger-site", "C3");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "poisonous-dagger"
            && if carried {
                descriptor["bearer"]["instanceId"] == bearer.as_str()
            } else {
                descriptor["bearer"].is_null() && descriptor["cell"] == "C3"
            }
    });
    // Stepping to C3 leaves the bearer alone with the dagger, so the exchange is a clean duel.
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bearer.as_str()
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    end_and_draw(session, "spellbook");

    play_site(session, "dagger-site", "C2");
    let enemy = summon(session, "dagger-enemy", "C3");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == enemy.as_str()
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == bearer.as_str()
    });
    (bearer, enemy)
}

#[test]
fn rule_catalog_0142_carried_lethal_should_kill_on_positive_strike_damage_and_drop_with_bearer() {
    let mut session =
        Session::new(&lethal_strike_scenario()).expect("valid lethal strike scenario");
    keep(&mut session);
    keep(&mut session);
    let (bearer, enemy) = lethal_strike_position(&mut session, true);

    let (_, fought) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    // Two power against three defense is short of lethal on its own, so only the carried Lethal
    // can explain the enemy's death; the bearer dies to the ordinary two it takes back.
    let struck: Vec<_> = fought
        .events
        .iter()
        .filter(|event| event.event_type == "damage-dealt")
        .map(|event| {
            (
                event.payload["instanceId"].clone(),
                event.payload["amount"].clone(),
            )
        })
        .collect();
    assert!(struck.contains(&(json!(enemy), json!(2))));
    assert!(struck.contains(&(json!(bearer), json!(2))));

    let event_types: Vec<_> = fought
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    assert_eq!(
        event_types
            .iter()
            .filter(|event_type| **event_type == "minion-died")
            .count(),
        2
    );
    // The dagger leaves its bearer as the bearer falls, before the death is recorded.
    let dropped = event_types
        .iter()
        .position(|event_type| *event_type == "artifact-dropped")
        .expect("the dagger drops with its bearer");
    let bearer_died = fought
        .events
        .iter()
        .position(|event| {
            event.event_type == "minion-died" && event.payload["instanceId"] == bearer.as_str()
        })
        .expect("the bearer dies in the exchange");
    assert!(dropped < bearer_died);

    let settled = state(&session);
    assert!(realm_unit(&settled, &bearer).is_none());
    assert!(realm_unit(&settled, &enemy).is_none());
    let artifacts = realm_artifacts(&settled);
    assert_eq!(artifacts.len(), 1);
    assert!(artifacts[0]["bearer"].is_null());
    assert_eq!(artifacts[0]["location"], "C3");
    assert_eq!(artifacts[0]["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn a_loose_lethal_artifact_should_not_grant_lethal_to_the_unit_standing_on_it() {
    let mut session =
        Session::new(&lethal_strike_scenario()).expect("valid lethal strike scenario");
    keep(&mut session);
    keep(&mut session);
    let (bearer, enemy) = lethal_strike_position(&mut session, false);

    let (_, fought) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    assert_eq!(
        fought
            .events
            .iter()
            .filter(|event| event.event_type == "minion-died")
            .map(|event| event.payload["instanceId"].clone())
            .collect::<Vec<_>>(),
        vec![json!(bearer)]
    );

    // The same two damage only wounds the enemy while the dagger lies loose on its cell.
    let settled = state(&session);
    assert_eq!(
        realm_unit(&settled, &enemy).expect("surviving enemy")["damage"],
        2
    );
    assert!(realm_unit(&settled, &bearer).is_none());
    let artifacts = realm_artifacts(&settled);
    assert_eq!(artifacts.len(), 1);
    assert!(artifacts[0]["bearer"].is_null());
    assert_eq!(artifacts[0]["location"], "C3");
    assert_exact_replay(&session);
}

/// Both spellbooks are small enough that the opening hand plus the seat's spellbook draws is the
/// whole spellbook, so the scenario does not depend on the shuffle.
fn ballista_scenario() -> String {
    let cards = json!({
        "ballista-avatar": avatar(),
        // Lethal and Stealth on the bearer prove the Ballista shoots on its own account: the shot
        // neither borrows the bearer's Lethal nor spends its Stealth.
        "ballista-bearer": minion(json!({
            "attack": 4,
            "defense": 2,
            "lethal": true,
            "stealth": true,
            "tapForMana": 1,
        })),
        "ballista-far-target": minion(json!({ "defense": 5 })),
        "ballista-helper": minion(json!({ "defense": 2, "tapForMana": 1 })),
        "ballista-hidden-target": minion(json!({ "defense": 5, "stealth": true })),
        // Four is exactly the bearer's power, so only a source that is not a unit gets through.
        "ballista-near-target": minion(json!({
            "defense": 5,
            "preventsDamageFromUnitsWithPowerAtLeast": 4,
        })),
        "ballista-north-site": { "cardType": "site", "elements": ["earth"] },
        "ballista-south-site": { "cardType": "site", "elements": ["earth", "water"] },
        "ballista-sunken-target": minion(json!({
            "defense": 5,
            "submerge": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 1 },
        })),
        "siege-ballista": {
            "cardType": "artifact",
            "manaCost": 0,
            "tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps": 3,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "whelm": {
            "cardType": "magic",
            "manaCost": 0,
            "submergeTargetMinion": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 1 },
        },
    });
    let decks = json!({
        "north": {
            "atlas": vec!["ballista-north-site"; 6],
            "avatar": "ballista-avatar",
            "spellbook": [
                "siege-ballista",
                "ballista-bearer",
                "ballista-helper",
                "siege-ballista",
            ],
        },
        "south": {
            "atlas": vec!["ballista-south-site"; 6],
            "avatar": "ballista-avatar",
            "spellbook": [
                "ballista-far-target",
                "ballista-hidden-target",
                "ballista-near-target",
                "ballista-sunken-target",
                "whelm",
            ],
        },
    });
    manifest("synthetic-siege-ballista-v1", &cards, &decks, 11)
}

/// Every identity the Ballista at C4 either reaches or deliberately leaves alone.
struct Ballista {
    bearer: String,
    carried: String,
    far: String,
    helper: String,
    loose: String,
    near: String,
}

/// Walks both seats up to North's ready Ballista bearer and its ready ally on C4, two measured
/// steps from South's minions on C2 and three from its minion on C1. One Ballista is conjured onto
/// the bearer and a second is left loose on the same cell.
fn ballista_position(session: &mut Session) -> Ballista {
    keep(session);
    keep(session);

    play_site(session, "ballista-north-site", "C4");
    end_and_draw(session, "spellbook");
    play_site(session, "ballista-south-site", "C1");
    end_and_draw(session, "spellbook");

    let bearer = summon(session, "ballista-bearer", "C4");
    let helper = summon(session, "ballista-helper", "C4");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "siege-ballista"
            && descriptor["bearer"]["instanceId"] == bearer.as_str()
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "siege-ballista"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C4"
    });
    // A summoning-sick bearer cannot pay the Ballista's first tap yet.
    assert!(descriptors_of_kind(session, "activate-artifact-damage").is_empty());
    play_site(session, "ballista-north-site", "C3");
    end_and_draw(session, "spellbook");

    play_site(session, "ballista-south-site", "C2");
    let far = summon(session, "ballista-far-target", "C1");
    let near = summon(session, "ballista-near-target", "C2");
    let hidden = summon(session, "ballista-hidden-target", "C2");
    let sunken = summon(session, "ballista-sunken-target", "C2");
    // South pulls one minion under its Water site so C2 exposes only its surface layer.
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "whelm"
            && descriptor["target"]["instanceId"] == sunken.as_str()
    });
    let opposed = state(session);
    assert_eq!(
        realm_unit(&opposed, &sunken).expect("sunken target")["region"],
        "underwater"
    );
    assert_eq!(
        realm_unit(&opposed, &hidden).expect("hidden target")["stealthed"],
        true
    );
    end_and_draw(session, "atlas");

    let artifacts = realm_artifacts(&state(session));
    assert_eq!(artifacts.len(), 2);
    let identity = |artifact: &Value| {
        artifact["instanceId"]
            .as_str()
            .expect("Ballista identity")
            .to_owned()
    };
    Ballista {
        carried: artifacts
            .iter()
            .find(|artifact| artifact["bearer"]["instanceId"] == bearer.as_str())
            .map(identity)
            .expect("the conjured Ballista its bearer carries"),
        loose: artifacts
            .iter()
            .find(|artifact| artifact["bearer"].is_null())
            .map(identity)
            .expect("the conjured Ballista lying loose on C4"),
        bearer,
        far,
        helper,
        near,
    }
}

/// The (helper, target) tap pairs one Artifact currently offers, in sorted order.
fn artifact_damage_pairs(session: &Session, artifact_instance_id: &str) -> Vec<(String, String)> {
    let mut pairs: Vec<_> = descriptors_of_kind(session, "activate-artifact-damage")
        .iter()
        .filter(|descriptor| descriptor["artifactInstanceId"] == artifact_instance_id)
        .map(|descriptor| {
            (
                descriptor["helper"]["instanceId"]
                    .as_str()
                    .expect("helper identity")
                    .to_owned(),
                descriptor["target"]["instanceId"]
                    .as_str()
                    .expect("target identity")
                    .to_owned(),
            )
        })
        .collect();
    pairs.sort();
    pairs
}

#[test]
fn rule_catalog_0143_siege_ballista_should_tap_bearer_and_ally_for_measured_artifact_damage() {
    let mut session = Session::new(&ballista_scenario()).expect("valid Siege Ballista scenario");
    let ballista = ballista_position(&mut session);
    let north_avatar = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();

    // Either ready ally on C4 may pay the second tap, and the shot reaches every surface unit
    // within two measured steps of C4: C4 itself, C3, and South's C2. South's minion on C1 is one
    // step too far, its hidden minion cannot be targeted, and its submerged minion is at another
    // location entirely.
    let mut expected: Vec<(String, String)> = [&north_avatar, &ballista.helper]
        .into_iter()
        .flat_map(|helper| {
            [
                &north_avatar,
                &ballista.bearer,
                &ballista.helper,
                &ballista.near,
            ]
            .into_iter()
            .map(|target| (helper.clone(), target.clone()))
        })
        .collect();
    expected.sort();
    assert_eq!(artifact_damage_pairs(&session, &ballista.carried), expected);
    assert!(
        artifact_damage_pairs(&session, &ballista.loose).is_empty(),
        "a loose Ballista has no bearer to tap"
    );

    let (_, fired) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-artifact-damage"
            && descriptor["artifactInstanceId"] == ballista.carried.as_str()
            && descriptor["helper"]["instanceId"] == ballista.helper.as_str()
            && descriptor["target"]["instanceId"] == ballista.near.as_str()
    });
    assert_eq!(
        event_types(&fired),
        [
            "artifact-damage-activated",
            "artifact-damage-allocated",
            "damage-dealt",
        ],
        "the Ballista shoots without opening a strike exchange"
    );
    assert_eq!(
        fired.events[0].payload,
        json!({
            "bearerInstanceId": ballista.bearer,
            "helperInstanceId": ballista.helper,
            "seat": "north",
            "sourceInstanceId": ballista.carried,
            "targetInstanceId": ballista.near,
        })
    );
    assert_eq!(
        fired.events[1].payload,
        json!({
            "amount": 3,
            "sourceInstanceId": ballista.carried,
            "targetInstanceId": ballista.near,
        })
    );
    assert!(fired.random_draws.is_empty());

    let settled = state(&session);
    let bearer = realm_unit(&settled, &ballista.bearer).expect("bearer");
    assert_eq!(
        (&bearer["damage"], &bearer["stealthed"], &bearer["tapped"]),
        (&json!(0), &json!(true), &json!(true)),
        "the bearer pays a tap, keeps its Stealth, and takes nothing back"
    );
    assert_eq!(
        realm_unit(&settled, &ballista.helper).expect("helper")["tapped"],
        true
    );
    // Measured artifact damage is not unit damage, so four-power prevention does not stop it and
    // the bearer's Lethal never reaches the five-defense minion it wounds.
    assert_eq!(
        realm_unit(&settled, &ballista.near).expect("near target")["damage"],
        3
    );
    assert_eq!(
        realm_unit(&settled, &ballista.far).expect("far target")["damage"],
        0
    );
    assert!(
        descriptors_of_kind(&session, "activate-artifact-damage").is_empty(),
        "a spent bearer cannot shoot its Ballista twice in one turn"
    );
    assert_exact_replay(&session);
}

#[test]
fn a_siege_ballista_should_shoot_the_ally_that_paid_its_second_tap() {
    let mut session = Session::new(&ballista_scenario()).expect("valid Siege Ballista scenario");
    let ballista = ballista_position(&mut session);

    let (_, fired) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-artifact-damage"
            && descriptor["artifactInstanceId"] == ballista.carried.as_str()
            && descriptor["helper"]["instanceId"] == ballista.helper.as_str()
            && descriptor["target"]["instanceId"] == ballista.helper.as_str()
    });
    assert_eq!(
        event_types(&fired),
        [
            "artifact-damage-activated",
            "artifact-damage-allocated",
            "damage-dealt",
            "minion-died",
        ]
    );

    let settled = state(&session);
    assert!(realm_unit(&settled, &ballista.helper).is_none());
    assert_eq!(
        realm_unit(&settled, &ballista.bearer).expect("bearer")["tapped"],
        true,
        "the bearer still pays its tap when the shot kills the ally that paid the other"
    );
    assert_exact_replay(&session);
}

#[test]
fn a_siege_ballista_should_require_both_its_bearer_and_a_second_ready_ally() {
    // Tapping the bearer for mana spends the first cost, so the Ballista offers nothing even with
    // two ready allies still standing on its cell.
    let mut spent_bearer =
        Session::new(&ballista_scenario()).expect("valid Siege Ballista scenario");
    let ballista = ballista_position(&mut spent_bearer);
    accept_where(&mut spent_bearer, |descriptor| {
        descriptor["kind"] == "activate-mana"
            && descriptor["unitInstanceId"] == ballista.bearer.as_str()
    });
    assert!(
        descriptors_of_kind(&spent_bearer, "activate-artifact-damage").is_empty(),
        "the bearer itself must be ready to pay the first tap"
    );
    assert_exact_replay(&spent_bearer);

    // With the bearer still ready but every other ally on C4 spent, the second tap cannot be paid.
    let mut spent_allies =
        Session::new(&ballista_scenario()).expect("valid Siege Ballista scenario");
    let ballista = ballista_position(&mut spent_allies);
    accept_where(&mut spent_allies, |descriptor| {
        descriptor["kind"] == "activate-mana"
            && descriptor["unitInstanceId"] == ballista.helper.as_str()
    });
    // Playing a site taps North's Avatar, the only other ally standing on C4.
    play_site(&mut spent_allies, "ballista-north-site", "D4");
    assert!(
        descriptors_of_kind(&spent_allies, "activate-artifact-damage").is_empty(),
        "a lone ready bearer cannot pay the Ballista's second tap by itself"
    );
    assert_exact_replay(&spent_allies);
}
