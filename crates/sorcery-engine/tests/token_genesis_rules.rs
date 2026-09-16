//! Direct proofs for Genesis effects printed on token minions (RULE-CATALOG-0381–0384).

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

#[test]
fn rule_catalog_0479_token_genesis_disable_strips_stealth_on_entry() {
    let token = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDisableSelfUntilDamaged": true,
        "manaCost": 0,
        "stealth": true,
        "token": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-genesis-disable-stealth" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-genesis-disable-stealth-v1",
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
        "seed": 479,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session =
        Session::new(&manifest).expect("valid stealthed disable token genesis manifest");
    keep(&mut session);
    keep(&mut session);
    let (_, paid) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&paid),
        [
            "site-played",
            "minion-summoned",
            "minion-disabled",
            "stealth-lost"
        ]
    );
    let token_id = paid.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity");
    assert_eq!(paid.events[2].payload["instanceId"], token_id);
    assert_eq!(paid.events[3].payload["instanceId"], token_id);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == token_id)
        .expect("summoned token");
    assert_eq!(unit["stealthed"], false);
    assert_eq!(unit["disabledUntilDamaged"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0480_spellbook_summon_genesis_disable_strips_stealth_on_entry() {
    let scout = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDisableSelfUntilDamaged": true,
        "manaCost": 0,
        "stealth": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "spellbook-genesis-disable-stealth" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-spellbook-genesis-disable-stealth-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-scout": scout,
            "north-site": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-scout"; 8],
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
        "seed": 480,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session =
        Session::new(&manifest).expect("valid spellbook disable-stealth genesis manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
    });
    assert_eq!(
        event_types(&receipt),
        ["minion-summoned", "minion-disabled", "stealth-lost"]
    );
    let minion_id = receipt.events[0].payload["instanceId"]
        .as_str()
        .expect("minion identity");
    assert_eq!(receipt.events[1].payload["instanceId"], minion_id);
    assert_eq!(receipt.events[2].payload["instanceId"], minion_id);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == minion_id)
        .expect("summoned minion");
    assert_eq!(unit["stealthed"], false);
    assert_eq!(unit["disabledUntilDamaged"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0483_token_genesis_disable_then_draws_site() {
    let token = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDisableSelfUntilDamaged": true,
        "genesisDrawSite": true,
        "manaCost": 0,
        "token": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut session =
        Session::new(&manifest(483, &token)).expect("valid disable draw token manifest");
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
        [
            "site-played",
            "minion-summoned",
            "minion-disabled",
            "site-drawn"
        ]
    );
    let token_id = paid.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity");
    assert_eq!(paid.events[2].payload["instanceId"], token_id);
    assert_eq!(paid.events[3].payload["sourceInstanceId"], token_id);
    let atlas_after = state(&session)["players"]["north"]["atlas"]
        .as_array()
        .expect("north atlas")
        .len();
    assert_eq!(atlas_after, atlas_before - 1);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == token_id)
        .expect("summoned token");
    assert_eq!(unit["disabledUntilDamaged"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0484_spellbook_summon_genesis_disable_then_draws_site() {
    let scout = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDisableSelfUntilDamaged": true,
        "genesisDrawSite": true,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "spellbook-genesis-disable-draw" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-spellbook-genesis-disable-draw-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-scout": scout,
            "north-site": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-scout"; 8],
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
        "seed": 484,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session =
        Session::new(&manifest).expect("valid spellbook disable-draw genesis manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    let atlas_before = state(&session)["players"]["north"]["atlas"]
        .as_array()
        .expect("north atlas")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
    });
    assert_eq!(
        event_types(&receipt),
        ["minion-summoned", "minion-disabled", "site-drawn"]
    );
    let minion_id = receipt.events[0].payload["instanceId"]
        .as_str()
        .expect("minion identity");
    assert_eq!(receipt.events[1].payload["instanceId"], minion_id);
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], minion_id);
    let atlas_after = state(&session)["players"]["north"]["atlas"]
        .as_array()
        .expect("north atlas")
        .len();
    assert_eq!(atlas_after, atlas_before - 1);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == minion_id)
        .expect("summoned minion");
    assert_eq!(unit["disabledUntilDamaged"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0485_token_genesis_disable_then_draws_spell() {
    let token = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDisableSelfUntilDamaged": true,
        "genesisDrawSpells": 1,
        "manaCost": 0,
        "token": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut session =
        Session::new(&manifest(485, &token)).expect("valid disable draw-spell token manifest");
    keep(&mut session);
    keep(&mut session);
    let spellbook_before = state(&session)["players"]["north"]["spellbook"]
        .as_array()
        .expect("north spellbook")
        .len();
    let (_, paid) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C4"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(
        event_types(&paid),
        [
            "site-played",
            "minion-summoned",
            "minion-disabled",
            "spell-drawn"
        ]
    );
    let token_id = paid.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity");
    assert_eq!(paid.events[2].payload["instanceId"], token_id);
    assert_eq!(paid.events[3].payload["sourceInstanceId"], token_id);
    let spellbook_after = state(&session)["players"]["north"]["spellbook"]
        .as_array()
        .expect("north spellbook")
        .len();
    assert_eq!(spellbook_after, spellbook_before - 1);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == token_id)
        .expect("summoned token");
    assert_eq!(unit["disabledUntilDamaged"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0486_spellbook_summon_genesis_disable_then_draws_spell() {
    let scout = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisDisableSelfUntilDamaged": true,
        "genesisDrawSpells": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "spellbook-genesis-disable-draw-spell" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-spellbook-genesis-disable-draw-spell-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-scout": scout,
            "north-site": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-scout"; 8],
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
        "seed": 486,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session =
        Session::new(&manifest).expect("valid spellbook disable-draw-spell genesis manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    let spellbook_before = state(&session)["players"]["north"]["spellbook"]
        .as_array()
        .expect("north spellbook")
        .len();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
    });
    assert_eq!(
        event_types(&receipt),
        ["minion-summoned", "minion-disabled", "spell-drawn"]
    );
    let minion_id = receipt.events[0].payload["instanceId"]
        .as_str()
        .expect("minion identity");
    assert_eq!(receipt.events[1].payload["instanceId"], minion_id);
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], minion_id);
    let spellbook_after = state(&session)["players"]["north"]["spellbook"]
        .as_array()
        .expect("north spellbook")
        .len();
    assert_eq!(spellbook_after, spellbook_before - 1);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == minion_id)
        .expect("summoned minion");
    assert_eq!(unit["disabledUntilDamaged"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0487_token_genesis_disable_declines_adjacent_damage() {
    let token = disable_damage_scout();
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-genesis-disable-damage-decline" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-genesis-disable-damage-decline-v1",
        },
        "cards": {
            "damage-scout": token,
            "north-avatar": avatar(),
            "north-gate": genesis_site("damage-scout"),
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
        "seed": 487,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session =
        Session::new(&manifest).expect("valid disable-damage decline token genesis manifest");
    keep(&mut session);
    keep(&mut session);
    let source_id = state(&session)["players"]["north"]["hand"]["atlas"][0]["instanceId"]
        .as_str()
        .expect("site identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
            && descriptor["genesisDamageChoice"] == "decline"
    });
    assert_eq!(
        event_types(&receipt),
        ["site-played", "minion-summoned", "minion-disabled"]
    );
    let token_id = receipt.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity");
    assert_eq!(receipt.events[2].payload["instanceId"], token_id);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == token_id)
        .expect("summoned token");
    assert_eq!(unit["disabledUntilDamaged"], true);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one catalog proof keeps setup, target damage, disable state, and replay together"
)]
fn rule_catalog_0488_spellbook_summon_genesis_disable_targets_adjacent_damage() {
    let scout = json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "genesisDisableSelfUntilDamaged": true,
        "genesisMayDamageTargetAdjacentUnit": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let mut enemy = dummy();
    enemy["summonToAnySite"] = json!(true);
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "spellbook-genesis-disable-damage" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-spellbook-genesis-disable-damage-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-scout": scout,
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy,
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-scout"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 488,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session =
        Session::new(&manifest).expect("valid spellbook disable-damage genesis manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let before = state(&session);
    let source_id = before["players"]["north"]["hand"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("scout identity")
        .to_owned();
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["genesisDamageTarget"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "minion-summoned",
            "minion-disabled",
            "genesis-damage-allocated",
            "damage-dealt",
            "avatar-life-lost"
        ]
    );
    assert_eq!(
        receipt.events[2].payload,
        json!({
            "amount": 2,
            "sourceInstanceId": source_id,
            "targetInstanceId": avatar_id,
        })
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    let minion_id = receipt.events[0].payload["instanceId"]
        .as_str()
        .expect("minion identity");
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["instanceId"] == minion_id)
        .expect("summoned minion");
    assert_eq!(unit["disabledUntilDamaged"], true);
    assert_exact_replay(&session);
}

fn damage_scout() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisMayDamageTargetAdjacentUnit": 2,
        "manaCost": 0,
        "token": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn disable_damage_scout() -> Value {
    let mut scout = damage_scout();
    scout["genesisDisableSelfUntilDamaged"] = json!(true);
    scout
}

fn magic(effect: (&str, Value), mana_cost: u8) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": mana_cost,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("Magic facts")
        .insert(effect.0.to_owned(), effect.1);
    value
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof covers issued branches, damage, checkpoint, and replay parity"
)]
fn rule_catalog_0383_token_genesis_damage_should_issue_decline_and_nearby_targets_on_site_entry() {
    let token = damage_scout();
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-genesis-damage-site" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-genesis-damage-site-v1",
        },
        "cards": {
            "damage-scout": token,
            "north-avatar": avatar(),
            "north-gate": genesis_site("damage-scout"),
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
        "seed": 383,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session = Session::new(&manifest).expect("valid token genesis damage manifest");
    keep(&mut session);
    keep(&mut session);

    let before = state(&session);
    let source_id = before["players"]["north"]["hand"]["atlas"][0]["instanceId"]
        .as_str()
        .expect("site identity")
        .to_owned();
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let origin_state_version = before["stateVersion"].clone();
    let expected_token_id = identity_hash(&json!({
        "cardId": "damage-scout",
        "cell": "C4",
        "ordinal": 0,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_id,
        "stateVersion": origin_state_version,
    }))
    .expect("deterministic token identity")
    .to_string();
    let choices: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardInstanceId"] == source_id
                && action.descriptor["cell"] == "C4"
                && action.descriptor["genesisTokenChoice"] == "pay-one-mana"
        })
        .collect();
    assert_eq!(choices.len(), 3);
    assert_eq!(choices[0].descriptor["genesisDamageChoice"], "decline");
    assert!(choices[0].descriptor.get("genesisDamageTarget").is_none());
    let mut expected_target_ids = [expected_token_id.clone(), avatar_id.clone()];
    expected_target_ids.sort();
    assert_eq!(
        choices[1..]
            .iter()
            .map(|action| {
                assert_eq!(action.descriptor["genesisDamageChoice"], "target");
                action.descriptor["genesisDamageTarget"]["instanceId"]
                    .as_str()
                    .expect("target identity")
            })
            .collect::<Vec<_>>(),
        expected_target_ids
    );

    let mut declined = session.clone();
    let (_, declined_receipt) = accept_where(&mut declined, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
            && descriptor["genesisDamageChoice"] == "decline"
    });
    assert_eq!(
        event_types(&declined_receipt),
        ["site-played", "minion-summoned"]
    );
    assert_exact_replay(&declined);

    let mut targeted = session;
    let (_, targeted_receipt) = accept_where(&mut targeted, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardInstanceId"] == source_id
            && descriptor["cell"] == "C4"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
            && descriptor["genesisDamageTarget"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&targeted_receipt),
        [
            "site-played",
            "minion-summoned",
            "genesis-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
        ]
    );
    assert_exact_replay(&targeted);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof covers issued branches, damage, and replay parity"
)]
fn rule_catalog_0384_magic_token_summon_should_issue_and_apply_adjacent_genesis_damage() {
    let token_id = "damage-scout";
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-genesis-damage-magic" }))
                .expect("authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-genesis-damage-magic-v1",
        },
        "cards": {
            token_id: damage_scout(),
            "north-avatar": avatar(),
            "north-magic": magic(("summonTokenToEachControlledSiteBorderingEnemySite", json!(token_id)), 0),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-magic"; 8],
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
        "seed": 384,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    let manifest = canonical_json(&value).expect("canonical manifest");
    let mut session = Session::new(&manifest).expect("valid magic token genesis manifest");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    for (draw_zone, site_cell) in [
        (Some("atlas"), Some("C1")),
        (Some("atlas"), Some("C3")),
        (Some("atlas"), Some("C2")),
        (Some("atlas"), Some("B3")),
        (Some("atlas"), Some("B2")),
    ] {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        if let Some(zone) = draw_zone {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == zone
            });
        }
        if let Some(cell) = site_cell {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == cell
            });
        }
    }
    let (south_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-dummy"
            && descriptor["cell"] == "B2"
    });
    let south_minion_id = south_summon["cardInstanceId"]
        .as_str()
        .expect("South minion identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let before = state(&session);
    let magic_id = before["players"]["north"]["hand"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("Magic identity")
        .to_owned();
    let pre_cast_version = before["stateVersion"].clone();
    let expected_token_ids: Vec<_> = ["B3", "C3"]
        .into_iter()
        .enumerate()
        .map(|(ordinal, cell)| {
            identity_hash(&json!({
                "cardId": token_id,
                "cell": cell,
                "ordinal": ordinal,
                "owner": "north",
                "source": "token",
                "sourceInstanceId": magic_id,
                "stateVersion": pre_cast_version,
            }))
            .expect("deterministic token identity")
            .to_string()
        })
        .collect();
    let choices: Vec<_> = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == magic_id
                && action.descriptor.get("tokenGenesisDamage").is_some()
        })
        .collect();
    assert_eq!(choices.len(), 9);
    assert!(choices.iter().any(|action| {
        action.descriptor["tokenGenesisDamage"]
            .as_array()
            .is_some_and(|resolutions| {
                resolutions.len() == 2
                    && resolutions
                        .iter()
                        .all(|resolution| resolution["genesisDamageChoice"] == "decline")
            })
    }));
    assert!(choices.iter().any(|action| {
        action.descriptor["tokenGenesisDamage"]
            .as_array()
            .is_some_and(|resolutions| {
                resolutions.len() == 2
                    && resolutions[0]["genesisDamageChoice"] == "target"
                    && resolutions[0]["genesisDamageTarget"]["instanceId"] == south_minion_id
                    && resolutions[0]["tokenInstanceId"] == expected_token_ids[0]
                    && resolutions[1]["genesisDamageChoice"] == "decline"
            })
    }));

    let mut targeted = session;
    let (_, targeted_receipt) = accept_where(&mut targeted, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == magic_id
            && descriptor["tokenGenesisDamage"]
                .as_array()
                .is_some_and(|resolutions| {
                    resolutions.len() == 2
                        && resolutions[0]["genesisDamageTarget"]["instanceId"] == south_minion_id
                        && resolutions[1]["genesisDamageChoice"] == "decline"
                })
    });
    assert_eq!(
        event_types(&targeted_receipt),
        [
            "magic-cast",
            "minion-summoned",
            "genesis-damage-allocated",
            "damage-dealt",
            "minion-died",
            "minion-summoned",
            "magic-resolved",
        ]
    );
    assert_exact_replay(&targeted);
}
