//! Direct synthetic proof that strike healing uses damage dealt by its own strike packet.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({"fixture":"strike-healing"})).unwrap(),
            "mode":"synthetic",
            "revisionId":"synthetic-strike-healing-v1",
        },
        "cards": {
            "north-avatar":{"attack":1,"cardType":"avatar","defense":1,"drawSpell":false,"life":20},
            "north-healer":{"attack":1,"cardType":"minion","defense":2,"healsControllerForStrikeDamage":true,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0}},
            "north-damage":{"cardType":"magic","damageTargetUnit":1,"manaCost":0,"payLifeAsAdditionalCost":1,"thresholds":{"air":0,"earth":0,"fire":0,"water":0}},
            "north-ward":{"cardType":"magic","grantWardToTargetMinion":true,"manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0}},
            "north-site":{"cardType":"site","elements":["earth"]},
            "south-avatar":{"attack":1,"cardType":"avatar","defense":1,"drawSpell":false,"life":20},
            "south-target":{"attack":2,"cardType":"minion","defense":2,"deathriteDrawSite":true,"manaCost":0,"summonToAnySite":true,"thresholds":{"air":0,"earth":0,"fire":0,"water":0}},
            "south-site":{"cardType":"site","elements":["earth"]},
        },
        "decks": {
            "north":{"atlas":["north-site","north-site","north-site","north-site","north-site","north-site"],"avatar":"north-avatar","spellbook":["north-healer","north-damage","north-ward","north-healer","north-damage","north-ward"]},
            "south":{"atlas":["south-site","south-site","south-site","south-site","south-site","south-site"],"avatar":"south-avatar","spellbook":["south-target","south-target","south-target","south-target","south-target","south-target"]},
        },
        "engineVersion":"sorcery-core-v1",
        "firstSeat":"north",
        "schemaVersion":1,
        "seed":seed,
    }))
}

fn accept(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let actions = session.legal_actions().unwrap();
    let action = actions
        .iter()
        .find(|a| predicate(&a.descriptor))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "no matching action; legal: {}",
                actions
                    .iter()
                    .map(|a| a.descriptor.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap()
    else {
        panic!("accepted action")
    };
    (descriptor, receipt)
}

fn keep(session: &mut Session) {
    accept(session, |d| {
        d["kind"] == "mulligan" && d["atlasOrder"] == json!([]) && d["spellbookOrder"] == json!([])
    });
}

fn state(session: &Session) -> Value {
    session.replay_value().unwrap()["state"].clone()
}

fn strike_healer(session: &mut Session, healer_id: &str, target_id: &str) -> Receipt {
    accept(session, |d| {
        d["kind"] == "move-and-attack"
            && d["unitInstanceId"] == healer_id
            && d["to"]["cell"] == "C4"
    });
    while state(session)["phase"] == "movement" {
        accept(session, |d| d["kind"] == "continue-basic-movement");
    }
    accept(session, |d| {
        d["kind"] == "declare-attack" && d["target"]["instanceId"] == target_id
    });
    let (_, mut receipt) = accept(session, |d| {
        d["kind"] == "close-defend" && d["originalTargetParticipates"] == true
    });
    if state(session)["phase"] == "intercept" {
        (_, receipt) = accept(session, |d| d["kind"] == "close-intercept");
    }
    receipt
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one session proof checks prevention, source healing, and event order"
)]
fn strike_healing_counts_dealt_damage_after_prevention_and_heals_controller() {
    let encoded = (1..1000)
        .map(manifest)
        .find(|encoded| {
            let session = Session::new(encoded).unwrap();
            let initial = state(&session);
            let hand = initial["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            hand.iter().any(|card| card["cardId"] == "north-healer")
                && hand.iter().any(|card| card["cardId"] == "north-damage")
                && hand.iter().any(|card| card["cardId"] == "north-ward")
        })
        .expect("seed with fixture cards in opening hand");
    let mut session = Session::new(&encoded).unwrap();
    keep(&mut session);
    keep(&mut session);
    accept(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C4"
    });
    let (healer, _) = accept(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "north-healer" && d["cell"] == "C4"
    });
    let healer_id = healer["cardInstanceId"].as_str().unwrap();
    accept(&mut session, |d| d["kind"] == "end-turn");
    accept(&mut session, |d| {
        d["kind"] == "draw" && d["zone"] == "atlas"
    });
    accept(&mut session, |d| {
        d["kind"] == "play-site" && d["cell"] == "C1"
    });
    let (target, _) = accept(&mut session, |d| {
        d["kind"] == "summon-minion" && d["cardId"] == "south-target" && d["cell"] == "C4"
    });
    let target_id = target["cardInstanceId"].as_str().unwrap();
    accept(&mut session, |d| d["kind"] == "end-turn");
    accept(&mut session, |d| d["kind"] == "draw");

    let paid_damage = accept(&mut session, |d| {
        d["kind"] == "cast-magic"
            && d["cardId"] == "north-damage"
            && d["target"]["kind"] == "minion"
            && d["target"]["seat"] == "south"
    });
    assert!(
        !paid_damage
            .1
            .events
            .iter()
            .any(|event| event.event_type == "avatar-healed")
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 19);
    let mut warded = session.clone();
    accept(&mut warded, |d| {
        d["kind"] == "cast-magic"
            && d["cardId"] == "north-ward"
            && d["target"]["instanceId"] == target_id
    });
    let prevented = strike_healer(&mut warded, healer_id, target_id);
    assert!(
        prevented
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken")
    );
    assert!(
        !prevented
            .events
            .iter()
            .any(|event| event.event_type == "avatar-healed")
    );
    assert_eq!(state(&warded)["players"]["north"]["avatar"]["life"], 19);
    let receipt = strike_healer(&mut session, healer_id, target_id);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "avatar-healed"
                && event.payload["sourceInstanceId"] == healer_id
                && event.payload["attemptedAmount"] == 1)
    );
    let healed_at = receipt
        .events
        .iter()
        .position(|event| event.event_type == "avatar-healed")
        .unwrap();
    let death_at = receipt
        .events
        .iter()
        .position(|event| {
            event.event_type == "minion-died" && event.payload["instanceId"] == healer_id
        })
        .unwrap();
    assert!(
        healed_at < death_at,
        "strike healing resolves before its death cohort"
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-died" && event.payload["instanceId"] == healer_id
    }));
    let deathrite_at = receipt
        .events
        .iter()
        .position(|event| event.event_type == "site-drawn")
        .unwrap_or_else(|| {
            panic!(
                "expected Deathrite event in {:?}",
                receipt
                    .events
                    .iter()
                    .map(|event| event.event_type.as_str())
                    .collect::<Vec<_>>()
            )
        });
    assert!(
        healed_at < deathrite_at,
        "strike healing precedes Deathrite"
    );
    assert!(
        death_at > healed_at,
        "the healer dies in the same combat window"
    );
}
