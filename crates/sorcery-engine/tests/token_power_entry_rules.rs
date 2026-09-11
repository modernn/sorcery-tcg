//! Direct proofs that token placement uses power-threshold entry (RULE-CATALOG-0331–0332).
//!
//! Tokens are units. Creating one on a site is an entry, so Genesis paid tokens
//! use the same prospective-power bar as summons. A printed-3 token cannot be
//! paid onto a threshold-3 site. A printed-1 token that would become 3 atop the
//! Tower it is spawning on cannot be paid onto that threshold-3 Tower.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
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

fn token(attack: u8, tower_bonus: bool) -> Value {
    let mut value = json!({
        "attack": attack,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    });
    if tower_bonus {
        value["gainsPowerRangedAndSpellcasterAtopTower"] = json!(2);
    }
    value
}

fn genesis_site(token_card_id: &str, threshold: Option<u8>, tower: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisPayOneManaToSummonToken": token_card_id,
    });
    if let Some(threshold) = threshold {
        value["preventsUnitsWithPowerAtLeastFromEntering"] = json!(threshold);
    }
    if tower {
        value["isTower"] = json!(true);
    }
    value
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn printed_manifest(seed: u32, heavy_token: bool) -> String {
    let token_id = if heavy_token {
        "heavy-scout"
    } else {
        "light-scout"
    };
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-gate": genesis_site(token_id, Some(3), false),
        "south-avatar": avatar(),
        "south-dummy": dummy(),
        "south-site": site(),
    });
    cards[token_id] = token(if heavy_token { 3 } else { 2 }, false);
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "token-power-entry-printed",
                "heavy": heavy_token,
            }))
            .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": if heavy_token {
                "synthetic-token-power-entry-heavy-v1"
            } else {
                "synthetic-token-power-entry-light-v1"
            },
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-gate"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn tower_manifest(seed: u32, threshold_tower: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "token-power-entry-tower",
                "threshold": threshold_tower,
            }))
            .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": if threshold_tower {
                "synthetic-token-power-entry-tower-bar-v1"
            } else {
                "synthetic-token-power-entry-tower-open-v1"
            },
        },
        "cards": {
            "north-avatar": avatar(),
            "north-gate": genesis_site("watcher", threshold_tower.then_some(3), true),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
            "watcher": token(1, true),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-gate"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
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

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn pays_genesis_at(cell: &str) -> impl Fn(&Value) -> bool + '_ {
    move |descriptor: &Value| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == cell
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    }
}

fn declines_genesis_at(cell: &str) -> impl Fn(&Value) -> bool + '_ {
    move |descriptor: &Value| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == cell
            && descriptor["genesisTokenChoice"] == "decline"
    }
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn opening(manifest: impl Fn(u32) -> String) -> Session {
    let session = Session::new(&manifest(1)).expect("token power-entry candidate");
    let mut session = session;
    keep(&mut session);
    keep(&mut session);
    session
}

#[test]
fn rule_catalog_0331_genesis_token_respects_a_printed_power_threshold() {
    let mut heavy = opening(|seed| printed_manifest(seed, true));
    assert!(
        offers(&heavy, declines_genesis_at("C4")),
        "Decline stays legal when the token is too strong to enter"
    );
    assert!(
        !offers(&heavy, pays_genesis_at("C4")),
        "a printed-3 token cannot be paid onto a threshold-3 site"
    );
    accept_where(&mut heavy, declines_genesis_at("C4"));

    let mut light = opening(|seed| printed_manifest(seed, false));
    assert!(
        offers(&light, pays_genesis_at("C4")),
        "a printed-2 token can still be paid onto a threshold-3 site"
    );
    let (_, paid) = accept_where(&mut light, pays_genesis_at("C4"));
    assert_eq!(
        paid.events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["site-played", "minion-summoned"]
    );
    assert_exact_replay(&heavy);
    assert_exact_replay(&light);
}

#[test]
fn rule_catalog_0332_genesis_token_uses_prospective_power_on_a_tower() {
    let mut barred = opening(|seed| tower_manifest(seed, true));
    assert!(offers(&barred, declines_genesis_at("C4")));
    assert!(
        !offers(&barred, pays_genesis_at("C4")),
        "printed 1 plus Tower bonus is 3, so the threshold-3 Tower stays illegal"
    );
    accept_where(&mut barred, declines_genesis_at("C4"));

    let mut open = opening(|seed| tower_manifest(seed, false));
    assert!(
        offers(&open, pays_genesis_at("C4")),
        "the same printed-1 Tower-bonus token can still be paid onto a Tower without a bar"
    );
    accept_where(&mut open, pays_genesis_at("C4"));
    assert_exact_replay(&barred);
    assert_exact_replay(&open);
}
