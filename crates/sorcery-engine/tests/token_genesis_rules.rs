//! Direct proofs for Genesis effects printed on token minions (RULE-CATALOG-0381–0382).

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
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn genesis_site(token_id: &str) -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisPayOneManaToSummonToken": token_id,
    })
}

fn manifest(seed: u32, token: &Value) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-genesis-rules" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-genesis-rules-v1",
        },
        "cards": {
            "draw-scout": token,
            "north-avatar": avatar(),
            "north-gate": genesis_site("draw-scout"),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-gate"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["south-dummy"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0381_token_genesis_draws_a_hidden_site_on_entry() {
    let token = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDrawSite": true,
        "manaCost": 0,
        "token": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut session = Session::new(&manifest(381, &token)).expect("valid token genesis manifest");
    keep(&mut session);
    keep(&mut session);
    let atlas_before = state(&session)["players"]["north"]["atlas"]
        .as_array()
        .expect("north atlas")
        .len();
    let (_, paid) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&paid),
        ["site-played", "minion-summoned", "site-drawn"]
    );
    let token_id = paid.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity");
    assert_eq!(paid.events[1].payload["token"], true);
    assert_eq!(paid.events[2].payload["sourceInstanceId"], token_id);
    let atlas_after = state(&session)["players"]["north"]["atlas"]
        .as_array()
        .expect("north atlas")
        .len();
    assert_eq!(atlas_after, atlas_before - 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0382_token_genesis_disables_the_token_until_damaged() {
    let token = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDisableSelfUntilDamaged": true,
        "manaCost": 0,
        "token": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-genesis-disable" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-genesis-disable-v1",
        },
        "cards": {
            "disable-scout": token,
            "north-avatar": avatar(),
            "north-gate-disable": genesis_site("disable-scout"),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-gate-disable"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["south-dummy"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 382,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session = Session::new(&manifest).expect("valid disable token genesis manifest");
    keep(&mut session);
    keep(&mut session);
    let (_, paid) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&paid),
        ["site-played", "minion-summoned", "minion-disabled"]
    );
    let token_id = paid.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity");
    assert_eq!(paid.events[2].payload["instanceId"], token_id);
    assert_exact_replay(&session);
}
