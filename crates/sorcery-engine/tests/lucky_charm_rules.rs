//! Direct proof for Lucky Charm (RULE-CATALOG-0028): random draws commit before outcome choice.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode, Seat};
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
    json!({ "cardType": "site", "elements": ["air"] })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "stealth": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "lucky-charm-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-lucky-charm-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charm": json!({
                "bearerControllerChoosesExtraRandomOutcome": true,
                "cardType": "artifact",
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            }),
            "north-bolt": json!({
                "cardType": "magic",
                "damageRandomUnitAtLocation": 3,
                "manaCost": 1,
                "thresholds": { "air": 1, "earth": 0, "fire": 0, "water": 0 },
            }),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 4],
                "avatar": "north-avatar",
                "spellbook": vec!["north-charm", "north-bolt", "north-bolt", "north-bolt", "north-bolt", "north-bolt"],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 6],
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
    session.replay_value().expect("authoritative replay value")["state"].clone()
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

fn setup(seed: u32) -> Option<Session> {
    let mut session = Session::new(&manifest(seed)).ok()?;
    keep(&mut session);
    keep(&mut session);
    if !state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .any(|card| card["cardId"] == "north-charm")
    {
        return None;
    }
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-charm"
            && descriptor["bearer"]["kind"] == "avatar"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
        });
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    Some(session)
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one replayed scenario proves commit-before-choice and forged rejection"
)]
fn rule_catalog_0028_lucky_charm_commits_random_draws_before_exposing_two_outcomes() {
    let mut before_commit = None;
    let mut session = None;
    let mut choices = Vec::new();

    for seed in 1..=100 {
        let Some(candidate) = setup(seed) else {
            continue;
        };
        let candidate_state = state(&candidate);
        let bolt = candidate_state["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north spellbook")
            .iter()
            .find(|card| card["cardId"] != "north-charm")
            .expect("damage spell");
        let bolt_id = bolt["instanceId"].as_str().expect("bolt identity");
        let casts: Vec<_> = candidate
            .legal_actions()
            .expect("legal actions")
            .into_iter()
            .filter(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["cardInstanceId"] == bolt_id
                    && action.descriptor["targetLocation"]["cell"] == "C1"
            })
            .collect();
        if casts.len() != 1 {
            continue;
        }
        let mut committed = candidate;
        let cast = casts[0].clone();
        let before = committed.clone();
        let StepResult::Accepted(receipt) = committed
            .step(ActionRequest {
                action_id: cast.action_id.to_string(),
                seat: cast.seat,
                state_version: cast.state_version,
            })
            .expect("cast step")
        else {
            continue;
        };
        let candidate_choices: Vec<_> = committed
            .legal_actions()
            .expect("random choices")
            .into_iter()
            .filter(|action| action.descriptor["kind"] == "resolve-random-outcome")
            .collect();
        if candidate_choices.len() == 2 {
            before_commit = Some(before);
            session = Some(committed);
            choices = candidate_choices
                .into_iter()
                .map(|action| (action.descriptor, action.label))
                .collect();
            assert!(receipt.events.is_empty());
            break;
        }
    }

    let before_commit = before_commit.expect("seed with two Lucky Charm outcomes");
    let mut session = session.expect("committed session");
    assert!(descriptors_of_kind(&before_commit, "resolve-random-outcome").is_empty());
    assert_eq!(state(&session)["phase"], "random-choice");
    assert!(choices.iter().all(|(descriptor, label)| {
        descriptor["kind"] == "resolve-random-outcome" && label.contains("Lucky Charm chooses")
    }));

    let committed_receipt = session.transcript().last().expect("committed receipt");
    assert!(committed_receipt.events.is_empty());
    assert_eq!(committed_receipt.random_draws.len(), 2);
    assert!(
        committed_receipt
            .random_draws
            .iter()
            .all(|draw| draw["purpose"] == "magic_random_unit_at_location")
    );

    let (chosen_descriptor, _) = choices[1].clone();
    let chosen_id = chosen_descriptor["outcomeInstanceId"]
        .as_str()
        .expect("chosen outcome")
        .to_owned();

    let mut occupants = vec![
        state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
            .as_str()
            .expect("south avatar")
            .to_owned(),
    ];
    occupants.extend(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .filter(|unit| unit["location"] == "C1" && unit["region"] == "surface")
            .map(|unit| {
                unit["instanceId"]
                    .as_str()
                    .expect("unit identity")
                    .to_owned()
            }),
    );
    occupants.sort_unstable();
    let offered_ids: Vec<_> = choices
        .iter()
        .map(|(descriptor, _)| {
            descriptor["outcomeInstanceId"]
                .as_str()
                .expect("offered outcome")
                .to_owned()
        })
        .collect();
    let unoffered_id = occupants
        .into_iter()
        .find(|id| !offered_ids.contains(id))
        .expect("unoffered occupant");

    let forged = session
        .step(ActionRequest {
            action_id: identity_hash(&json!({
                "descriptor": { "kind": "resolve-random-outcome", "outcomeInstanceId": unoffered_id },
                "engineVersion": "sorcery-core-v1",
                "seat": "north",
                "stateVersion": session.state_version(),
            }))
            .expect("forged action id")
            .to_string(),
            seat: Seat::North,
            state_version: session.state_version(),
        })
        .expect("forged step");
    assert!(matches!(
        forged,
        StepResult::Rejected(rejection) if rejection.code == RejectionCode::UnknownAction
    ));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-random-outcome"
            && descriptor["outcomeInstanceId"] == chosen_id
    });
    let allocation = session
        .transcript()
        .last()
        .expect("resolution receipt")
        .events
        .iter()
        .find(|event| event.event_type == "magic-damage-allocated")
        .expect("damage allocation");
    assert_eq!(
        allocation.payload["targetInstanceId"].as_str(),
        Some(chosen_id.as_str())
    );
    assert!(session.verify_replay().expect("verified replay"));
}
