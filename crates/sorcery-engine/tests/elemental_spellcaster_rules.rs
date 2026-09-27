//! Public integration proofs for elemental Spellcaster restrictions.

use serde_json::{Value, json};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::selfplay_manifest_with;

fn thresholds(element: &str, amount: u8) -> Value {
    let mut value = json!({"air": 0, "earth": 0, "fire": 0, "water": 0});
    value[element] = json!(amount);
    value
}

fn manifest(seed: u32, spellbook: &[&str], allowed: Option<&[&str]>) -> String {
    selfplay_manifest_with(seed, |manifest| {
        manifest["cards"] = json!({
            "north-avatar": {"attack": 1, "cardType": "avatar", "defense": 1, "drawSpell": false, "life": 20},
            "north-site": {"cardType": "site", "elements": ["earth", "fire", "water", "air"]},
            "fire-chain": {"cardType":"magic", "manaCost":0, "thresholds": thresholds("fire",1), "damageChainNearbyUnits":true},
            "fire-magic": {"cardType": "magic", "manaCost": 0, "thresholds": thresholds("fire", 1), "damageTargetUnit": 1},
            "air-magic": {"cardType": "magic", "manaCost": 0, "thresholds": thresholds("air", 1), "damageTargetUnit": 1},
            "zero-magic": {"cardType": "magic", "manaCost": 0, "thresholds": thresholds("fire", 0), "damageTargetUnit": 1},
            "mixed-magic": {"cardType": "magic", "manaCost": 0, "thresholds": {"air": 1, "earth": 0, "fire": 1, "water": 0}, "damageTargetUnit": 1},
            "fire-minion": {"attack": 1, "cardType": "minion", "defense": 1, "manaCost": 0, "thresholds": thresholds("fire", 1)},
            "air-minion": {"attack": 1, "cardType": "minion", "defense": 1, "manaCost": 0, "thresholds": thresholds("air", 1)},
            "fire-aura": {"cardType": "aura", "manaCost": 0, "thresholds": thresholds("fire", 1), "affectedSitesAreFlooded": true},
            "air-aura": {"cardType": "aura", "manaCost": 0, "thresholds": thresholds("air", 1), "affectedSitesAreFlooded": true},
            "fire-artifact": {"cardType": "artifact", "manaCost": 0, "thresholds": thresholds("fire", 1), "grantsBearerPower": 2},
            "air-artifact": {"cardType": "artifact", "manaCost": 0, "thresholds": thresholds("air", 1), "grantsBearerPower": 2},
            "south-avatar": {"attack": 1, "cardType": "avatar", "defense": 1, "drawSpell": false, "life": 20},
            "south-site": {"cardType": "site", "elements": ["water"]},
        });
        manifest["cards"]["fire-caster"] = json!({
            "attack": 1, "cardType": "minion", "defense": 3, "manaCost": 0,
            "spellcaster": true, "spellcasterElements": ["fire"], "thresholds": thresholds("fire", 0),
        });
        manifest["cards"]["fire-caster"]["elements"] = json!(["fire"]);
        match allowed {
            Some(elements) => {
                manifest["cards"]["fire-caster"]["spellcasterElements"] = json!(elements);
            }
            None => {
                manifest["cards"]["fire-caster"]
                    .as_object_mut()
                    .unwrap()
                    .remove("spellcasterElements");
            }
        }
        for element in ["earth", "water"] {
            manifest["cards"][format!("{element}-magic")] = json!({"cardType":"magic", "manaCost":0, "thresholds":thresholds(element,1), "damageTargetUnit":1});
        }
        let mut deck: Vec<Value> = spellbook.iter().map(|card| json!(card)).collect();
        for card in [
            "fire-chain",
            "earth-magic",
            "water-magic",
            "fire-magic",
            "air-magic",
            "zero-magic",
            "mixed-magic",
            "fire-minion",
            "air-minion",
            "fire-aura",
            "air-aura",
            "fire-artifact",
            "air-artifact",
            "fire-caster",
        ] {
            if !spellbook.contains(&card) {
                deck.push(json!(card));
            }
        }
        while deck.len() < 8 {
            deck.push(json!(spellbook[0]));
        }
        manifest["decks"]["north"] = json!({
            "avatar": "north-avatar", "atlas": vec!["north-site"; 12],
            "spellbook": deck,
        });
        manifest["decks"]["south"] = json!({
            "avatar": "south-avatar", "atlas": vec!["south-site"; 12],
            "spellbook": vec!["air-magic"; 10],
        });
    })
}

fn accept(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
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
        .expect("accepted action")
    else {
        panic!("engine-issued action must be accepted");
    };
    (descriptor, receipt)
}

fn ready(spellbook: &[&str]) -> Session {
    ready_for(spellbook, Some(&["fire"]))
}

fn ready_for(spellbook: &[&str], allowed: Option<&[&str]>) -> Session {
    let encoded = (1..=4096)
        .map(|seed| manifest(seed, spellbook, allowed))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("manifest");
            let replay = preview.replay_value().expect("replay");
            let hand = replay["state"]["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("hand");
            spellbook
                .iter()
                .all(|card| hand.iter().any(|entry| entry["cardId"] == *card))
        })
        .expect("seed with requested opening hand");
    let mut session = Session::new(&encoded).expect("manifest");
    accept(&mut session, |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
    accept(&mut session, |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
    accept(&mut session, |d| {
        d["kind"] == "play-site" && d["cardId"] == "north-site" && d["cell"] == "C4"
    });
    accept(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "fire-caster" && d["cell"] == "C4"
    });
    session
}

fn elemental_actions<'a>(
    actions: &'a [sorcery_engine::contract::LegalAction],
    caster_id: &str,
) -> impl Iterator<Item = &'a sorcery_engine::contract::LegalAction> {
    actions
        .iter()
        .filter(move |action| action.descriptor["casterInstanceId"] == caster_id)
}

fn caster_id(session: &Session) -> String {
    let replay = session.replay_value().expect("replay");
    replay["state"]["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "fire-caster")
        .and_then(|unit| unit["instanceId"].as_str())
        .expect("elemental caster")
        .to_owned()
}

#[test]
fn fire_spellcaster_offers_matching_threshold_and_rejects_other_elements() {
    let session = ready(&["fire-caster", "fire-magic", "air-magic"]);
    let actions = session.legal_actions().expect("main actions");
    let elemental = caster_id(&session);
    assert!(elemental_actions(&actions, &elemental).any(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "fire-magic"));
    assert!(
        !elemental_actions(&actions, &elemental)
            .any(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "air-magic")
    );

    // The avatar remains unrestricted, so the same nonmatching card may still
    // be issued with the avatar as caster.
    assert!(
        actions
            .iter()
            .any(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "air-magic")
    );

    let mixed = ready(&["fire-caster", "mixed-magic", "air-magic"]);
    let mixed_id = caster_id(&mixed);
    assert!(
        elemental_actions(&mixed.legal_actions().expect("mixed actions"), &mixed_id).any(|a| {
            a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "mixed-magic"
        })
    );

    let zero = ready(&["fire-caster", "zero-magic"]);
    let zero_actions = zero.legal_actions().expect("zero actions");
    let zero_id = caster_id(&zero);
    assert!(!elemental_actions(&zero_actions, &zero_id)
        .any(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "zero-magic"));
}

#[test]
fn fire_spellcaster_filters_minion_aura_and_artifact_summaries() {
    for (fire, air) in [
        ("fire-minion", "air-minion"),
        ("fire-aura", "air-aura"),
        ("fire-artifact", "air-artifact"),
    ] {
        let session = ready(&["fire-caster", fire, air]);
        let actions = session.legal_actions().expect("main actions");
        let elemental = caster_id(&session);
        assert!(
            elemental_actions(&actions, &elemental).any(|a| a.descriptor["cardId"] == fire),
            "matching card {fire}"
        );
        assert!(
            !elemental_actions(&actions, &elemental).any(|a| a.descriptor["cardId"] == air),
            "forbidden card {air}"
        );
    }
}

#[test]
fn matching_casts_of_every_kind_resume_and_replay() {
    for card in ["fire-magic", "fire-minion", "fire-aura", "fire-artifact"] {
        let mut session = ready(&["fire-caster", card]);
        let caster = caster_id(&session);
        let checkpoint = create_game_checkpoint(&session).unwrap();
        let mut resumed = resume_game_checkpoint(&checkpoint).unwrap();
        let predicate = |d: &Value| d["cardId"] == card && d["casterInstanceId"] == caster;
        let (_, receipt) = accept(&mut session, predicate);
        let (_, repeated) = accept(&mut resumed, predicate);
        assert_eq!(receipt, repeated);
        assert_eq!(
            session.replay_value().unwrap(),
            resumed.replay_value().unwrap()
        );
        assert!(session.verify_replay().unwrap());
    }
}

#[test]
fn all_elements_and_element_sets_filter_independently_of_printed_identity() {
    for allowed in [
        vec!["earth"],
        vec!["fire"],
        vec!["water"],
        vec!["air"],
        vec!["earth", "water"],
    ] {
        for element in ["earth", "fire", "water", "air"] {
            let card = format!("{element}-magic");
            let session = ready_for(&["fire-caster", &card], Some(&allowed));
            let caster = caster_id(&session);
            let actions = session.legal_actions().unwrap();
            assert!(
                actions.iter().any(|a| a.descriptor["cardId"] == card
                    && a.descriptor["casterInstanceId"] != caster)
            );
            assert_eq!(
                elemental_actions(&actions, &caster).any(|a| a.descriptor["cardId"] == card),
                allowed.contains(&element),
                "allowed {allowed:?}, spell {element}"
            );
        }
    }
    for card in ["earth-magic", "air-magic", "zero-magic"] {
        let session = ready_for(&["fire-caster", card], None);
        assert!(
            elemental_actions(&session.legal_actions().unwrap(), &caster_id(&session))
                .any(|a| a.descriptor["cardId"] == card)
        );
    }
}

#[test]
fn malformed_elemental_spellcaster_facts_are_rejected() {
    for allowed in [
        vec![],
        vec!["fire", "fire"],
        vec!["air", "fire"],
        vec!["unknown"],
    ] {
        assert!(Session::new(&manifest(1, &["fire-caster"], Some(&allowed))).is_err());
    }
}

#[test]
fn staged_chain_cast_preserves_elemental_eligibility_through_checkpoint() {
    let mut session = ready(&["fire-caster", "fire-chain"]);
    let caster = caster_id(&session);
    accept(&mut session, |d| {
        d["kind"] == "begin-chain-magic" && d["casterInstanceId"] == caster
    });
    let mut resumed = resume_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
    let (_, receipt) = accept(&mut session, |d| d["kind"] == "resolve-chain-magic");
    let (_, repeated) = accept(&mut resumed, |d| d["kind"] == "resolve-chain-magic");
    assert_eq!(receipt, repeated);
    assert!(session.verify_replay().unwrap());
    assert_eq!(
        session.replay_value().unwrap(),
        resumed.replay_value().unwrap()
    );
    let blocked = ready_for(&["fire-caster", "fire-chain"], Some(&["air"]));
    let actions = blocked.legal_actions().unwrap();
    assert!(
        actions
            .iter()
            .any(|a| a.descriptor["kind"] == "begin-chain-magic")
    );
    assert!(
        !elemental_actions(&actions, &caster_id(&blocked))
            .any(|a| a.descriptor["kind"] == "begin-chain-magic")
    );
}
