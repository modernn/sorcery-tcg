//! Direct proofs for alternate summon payment admission and cost enforcement
//! (RULE-CATALOG-0015, RULE-CATALOG-0016, RULE-CATALOG-0729, RULE-CATALOG-0921).

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn thresholds(element: Option<&str>, required: u64) -> Value {
    let mut value = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    if let Some(element) = element {
        value[element] = json!(required);
    }
    value
}

fn site(element: &str) -> Value {
    json!({ "cardType": "site", "elements": [element] })
}

fn minion(mana_cost: u64, required: &Value) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": mana_cost,
        "thresholds": required,
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn random_discard_manifest(seed: u32, north_site: &Value) -> String {
    let mut aramos = minion(3, &thresholds(Some("earth"), 1));
    aramos["discardRandomCardInsteadOfMana"] = json!(true);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "alternative-payment-thresholds" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-alternative-payment-thresholds-v1",
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-minion": aramos,
            "north-site": north_site,
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-minion": minion(0, &thresholds(None, 0)),
            "south-site": site("earth"),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 4],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 4],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn sacrifice_threshold_manifest(seed: u32, north_site: &Value) -> String {
    let mut gnarled = minion(6, &thresholds(Some("water"), 1));
    gnarled["sacrificeMinionAtSummoningLocationForManaDiscount"] = json!(2);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sacrifice-payment-thresholds" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sacrifice-payment-thresholds-v1",
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-gnarled": gnarled,
            "north-helper": minion(0, &thresholds(None, 0)),
            "north-site": north_site,
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-minion": minion(0, &thresholds(None, 0)),
            "south-site": site("earth"),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-gnarled", "north-helper", "north-helper", "north-helper"],
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

fn opening_main(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid alternative-payment scenario");
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

fn summon_actions(session: &Session, card_id: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion" && action.descriptor["cardId"] == card_id
        })
        .map(|action| action.descriptor)
        .collect()
}

fn sacrifice_board(manifest: &str) -> Session {
    let mut session = opening_main(manifest);
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "north-helper"
                && descriptor["cell"] == "C4"
        });
    }
    session
}

#[test]
fn rule_catalog_0729_alternate_summon_payments_should_be_admitted_and_require_their_costs() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0729_alternate_summon_payments_should_be_admitted_and_require_their_costs(
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "alternate payment threshold gate proof covers blocked and allowed branches"
)]
fn rule_catalog_0921_alternate_summon_payments_require_thresholds_before_offering() {
    let blocked_discard = opening_main(&random_discard_manifest(921, &site("fire")));
    let before_discard = state(&blocked_discard);
    assert!(
        !before_discard["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("Atlas hand")
            .is_empty()
    );
    assert!(
        before_discard["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("Spellbook hand")
            .len()
            > 1
    );
    assert!(summon_actions(&blocked_discard, "north-minion").is_empty());
    assert!(
        blocked_discard
            .verify_replay()
            .expect("verified blocked random-discard replay")
    );

    let mut allowed_discard = opening_main(&random_discard_manifest(922, &site("earth")));
    let discard_summons = summon_actions(&allowed_discard, "north-minion");
    assert!(discard_summons.iter().any(|descriptor| {
        descriptor["manaCost"] == 0 && descriptor["paymentMode"] == "random-card-discard"
    }));
    let discard = discard_summons
        .into_iter()
        .find(|descriptor| descriptor["paymentMode"] == "random-card-discard")
        .expect("random-card discard payment");
    let action = allowed_discard
        .legal_actions()
        .expect("Aramos legal actions")
        .into_iter()
        .find(|action| action.descriptor == discard)
        .expect("engine-issued random discard");
    let StepResult::Accepted(receipt) = allowed_discard
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("random discard summon")
    else {
        panic!("engine-issued random discard summon must be accepted");
    };
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["card-discarded", "minion-summoned"]
    );
    assert!(
        allowed_discard
            .verify_replay()
            .expect("verified random-discard replay")
    );

    let mut mana_site = site("earth");
    mana_site["genesisGainMana"] = json!(6);
    let blocked_sacrifice = sacrifice_board(&sacrifice_threshold_manifest(923, &mana_site));
    let blocked_board = state(&blocked_sacrifice);
    assert_eq!(
        blocked_board["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .filter(|unit| unit["cardId"] == "north-helper" && unit["location"] == "C4")
            .count(),
        2
    );
    assert!(summon_actions(&blocked_sacrifice, "north-gnarled").is_empty());
    assert!(
        blocked_sacrifice
            .verify_replay()
            .expect("verified blocked sacrifice replay")
    );

    let mut water_site = site("water");
    water_site["genesisGainMana"] = json!(6);
    let mut allowed_sacrifice = sacrifice_board(&sacrifice_threshold_manifest(924, &water_site));
    let sacrifice_summons = summon_actions(&allowed_sacrifice, "north-gnarled");
    assert!(sacrifice_summons.iter().any(|descriptor| {
        descriptor["manaCost"] == 4
            && descriptor["sacrificedMinionInstanceIds"]
                .as_array()
                .is_some_and(|ids| ids.len() == 1)
    }));
    let sacrifice = sacrifice_summons
        .into_iter()
        .find(|descriptor| {
            descriptor["manaCost"] == 4
                && descriptor["sacrificedMinionInstanceIds"]
                    .as_array()
                    .is_some_and(|ids| ids.len() == 1)
        })
        .expect("single-sacrifice payment");
    let action = allowed_sacrifice
        .legal_actions()
        .expect("Gnarled legal actions")
        .into_iter()
        .find(|action| action.descriptor == sacrifice)
        .expect("engine-issued sacrifice payment");
    let StepResult::Accepted(receipt) = allowed_sacrifice
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("sacrifice discount summon")
    else {
        panic!("engine-issued sacrifice discount summon must be accepted");
    };
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert!(
        allowed_sacrifice
            .verify_replay()
            .expect("verified sacrifice replay")
    );
}
