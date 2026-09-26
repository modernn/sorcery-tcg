//! Direct proofs for Genesis strike-each-enemy-here (RULE-CATALOG-0057,
//! RULE-CATALOG-0675–0676, 1016, 1101, 2343–2348).
//!
//! On entry, a minion strikes every enemy sharing its location, including the
//! Avatar standing on that site. Allies and the striker are skipped. Ward
//! absorbs a strike. Enemies on a different cell are not reached.
//!
//! 1016 covers genesis strike here killing a Deathrite minion: the controller
//! draws a site and the summon receipt finishes only after deathrite settlement.
//! While Deathrites wait for ordering, summoning a genesis strike-here minion
//! stays withheld until the chain drains.
//!
//! Supplemental 2343–2348 bind Avatar-life persistence, empty-repeat, enemy-arrival,
//! multi-enemy, far-enemy, and a newly summoned enemy. Distinct from 0675–0676,
//! which cover the first-summon hit/skip matrix without later turns or a
//! second titan.

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
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn titan() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 3,
        "genesisStrikeEachEnemyHere": true,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn plain() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn warded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    })
}

fn deathrite_plain() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn strike_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "genesis-strike-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-genesis-strike-here-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-site": site(),
            "north-titan": titan(),
            "south-avatar": avatar(),
            "south-plain": plain(),
            "south-site": site(),
            "south-warded": warded(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-titan",
                    "north-ally",
                    "north-titan",
                    "north-ally",
                    "north-titan",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-plain",
                    "south-warded",
                    "south-plain",
                    "south-warded",
                    "south-plain",
                    "south-warded",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn strike_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "genesis-strike-here-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-genesis-strike-here-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-titan": titan(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_plain(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-titan"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
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

fn try_accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid genesis-strike-here session");
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

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn avatar_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar identity")
        .to_owned()
}

fn summon_at(session: &mut Session, card_id: &str, cell: &str) -> (String, Receipt) {
    let (summoned, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    (
        summoned["cardInstanceId"]
            .as_str()
            .expect("summoned identity")
            .to_owned(),
        receipt,
    )
}

fn seed_with(required_north: &[&str], required_south: &[&str]) -> String {
    (675..675 + 256)
        .map(strike_manifest)
        .find(|candidate| {
            let opening = state(&Session::new(candidate).expect("candidate session"));
            let has = |seat: &str, wanted: &[&str]| {
                let hand = opening["players"][seat]["hand"]["spellbook"]
                    .as_array()
                    .expect("opening hand");
                wanted
                    .iter()
                    .all(|id| hand.iter().any(|card| card["cardId"] == *id))
            };
            has("north", required_north) && has("south", required_south)
        })
        .expect("bounded seed with required opening cards")
}

fn seed_with_deathrite() -> String {
    (1016..1016 + 256)
        .map(strike_deathrite_manifest)
        .next()
        .expect("bounded deathrite genesis strike seed")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn south_deathrite_at_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("deathrite enemy identity")
        .to_owned()
}

fn enemies_at_c1(session: &mut Session) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut enemy_ids = Vec::new();
    for card_id in ["south-plain", "south-warded"] {
        enemy_ids.push(summon_at(session, card_id, "C1").0);
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_ids
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0675_genesis_strike_hits_every_enemy_sharing_the_newcomers_cell() {
    let encoded = seed_with(
        &["north-ally", "north-titan"],
        &["south-plain", "south-warded"],
    );
    let mut session = opening_main(&encoded);
    let enemy_ids = enemies_at_c1(&mut session);
    let ally_id = summon_at(&mut session, "north-ally", "C1").0;
    let (titan_id, receipt) = summon_at(&mut session, "north-titan", "C1");

    let mut struck: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 3);
            assert_eq!(event.payload["strikerInstanceId"], titan_id.as_str());
            event.payload["targetInstanceId"]
                .as_str()
                .expect("struck identity")
                .to_owned()
        })
        .collect();
    let resolved = state(&session);
    let enemy_avatar_id = avatar_id(&resolved, "south");
    let mut expected = enemy_ids.clone();
    expected.push(enemy_avatar_id);
    struck.sort_unstable();
    expected.sort_unstable();
    assert_eq!(struck, expected);
    assert!(!struck.contains(&ally_id));
    assert!(!struck.contains(&titan_id));
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken")
    );

    let plain_id = enemy_ids
        .iter()
        .find(|instance_id| unit(&resolved, instance_id)["cardId"] == "south-plain")
        .expect("plain enemy")
        .clone();
    let warded_id = enemy_ids
        .iter()
        .find(|instance_id| **instance_id != plain_id)
        .expect("warded enemy")
        .clone();
    assert_eq!(unit(&resolved, &plain_id)["damage"], 3);
    let warded = unit(&resolved, &warded_id);
    assert_eq!(warded["damage"], 0);
    assert_eq!(warded["warded"], false);
    assert_eq!(unit(&resolved, &ally_id)["damage"], 0);
    assert_eq!(unit(&resolved, &titan_id)["damage"], 0);
    assert_eq!(resolved["players"]["south"]["avatar"]["life"], 17);
    assert_eq!(resolved["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0676_genesis_strike_skips_allies_and_far_enemies() {
    let encoded = seed_with(
        &["north-ally", "north-titan"],
        &["south-plain", "south-warded"],
    );
    let mut session = opening_main(&encoded);
    let enemy_ids = enemies_at_c1(&mut session);
    let ally_id = summon_at(&mut session, "north-ally", "C4").0;
    let (titan_id, receipt) = summon_at(&mut session, "north-titan", "C4");

    assert_eq!(event_types(&receipt), ["minion-summoned"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "strike-damage-allocated")
    );

    let resolved = state(&session);
    let warded_id = enemy_ids
        .iter()
        .find(|instance_id| unit(&resolved, instance_id)["cardId"] == "south-warded")
        .expect("warded enemy");
    for enemy_id in &enemy_ids {
        assert_eq!(unit(&resolved, enemy_id)["damage"], 0);
        assert_eq!(unit(&resolved, enemy_id)["location"], "C1");
    }
    assert_eq!(unit(&resolved, warded_id)["warded"], true);
    assert_eq!(unit(&resolved, &ally_id)["damage"], 0);
    assert_eq!(unit(&resolved, &titan_id)["damage"], 0);
    assert_eq!(resolved["players"]["south"]["avatar"]["life"], 20);
    assert_eq!(resolved["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1016_genesis_strike_here_deathrite_draws_for_controller_on_kill() {
    let encoded = seed_with_deathrite();
    let mut session = opening_main(&encoded);
    let enemy_id = south_deathrite_at_c1(&mut session);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (titan_id, receipt) = summon_at(&mut session, "north-titan", "C1");
    let types = event_types(&receipt);
    assert_eq!(types.first(), Some(&"minion-summoned"));
    assert_eq!(types.last(), Some(&"minion-died"));
    let deathrite_strike = receipt
        .events
        .iter()
        .find(|event| {
            event.event_type == "strike-damage-allocated"
                && event.payload["targetInstanceId"] == enemy_id
        })
        .expect("Deathrite minion struck");
    assert_eq!(deathrite_strike.payload["amount"], 3);
    assert_eq!(deathrite_strike.payload["strikerInstanceId"], titan_id);
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], enemy_id);
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let minion_died = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion-died index");
    assert!(
        site_drawn < minion_died,
        "summon receipt must finish only after deathrite site-drawn"
    );

    let finished = state(&session);
    assert_eq!(unit(&finished, &titan_id)["damage"], 0);
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == enemy_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}

fn deathrite_genesis_manifest(seed: u32) -> String {
    let fixture = "genesis-strike-here-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "north-titan": titan(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_plain(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-titan",
                    "north-rain",
                    "north-rain",
                    "north-titan",
                    "north-rain",
                    "north-titan",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_titan_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-titan", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn titan_summon_offered(session: &Session) -> bool {
    session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-titan"
        })
    })
}

struct PendingDeathriteGenesisSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_titan_in_hand(encoded: &str) -> Option<PendingDeathriteGenesisSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_titan_and_rain(&state(&session)) {
        return None;
    }
    if !titan_summon_offered(&session) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteGenesisSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_genesis_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_genesis_manifest)
        .find(|candidate| try_pending_deathrite_with_titan_in_hand(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with genesis strike titan in hand")
}

#[test]
fn rule_catalog_1101_genesis_strike_here_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_genesis_seed_with(1101);
    let mut setup = try_pending_deathrite_with_titan_in_hand(&encoded)
        .expect("complete genesis-strike Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "end-turn"
                    && action.descriptor["kind"] != "summon-minion"
            })
    );
    assert!(!titan_summon_offered(session));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(titan_summon_offered(session));

    let (titan_id, receipt) = summon_at(session, "north-titan", "C1");
    let enemy_avatar_id = avatar_id(&state(session), "south");
    let strike = receipt
        .events
        .iter()
        .find(|event| {
            event.event_type == "strike-damage-allocated"
                && event.payload["targetInstanceId"] == enemy_avatar_id
        })
        .expect("genesis strike on the co-located enemy Avatar");
    assert_eq!(strike.payload["amount"], 3);
    assert_eq!(strike.payload["strikerInstanceId"], titan_id);
    assert_eq!(state(session)["players"]["south"]["avatar"]["life"], 17);
    assert_exact_replay(session);
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn strike_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "genesis-strike-here-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-genesis-strike-here-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-titan": titan(),
            "south-avatar": avatar(),
            "south-plain": plain(),
            "south-raider": raider(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-titan"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-plain",
                    "south-raider",
                    "south-plain",
                    "south-raider",
                    "south-plain",
                    "south-raider",
                    "south-plain",
                    "south-raider",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    state(&Session::new(encoded).expect("candidate session"))["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("opening Spellbook hand")
        .iter()
        .map(|card| {
            card["cardId"]
                .as_str()
                .expect("hand card identity")
                .to_owned()
        })
        .collect()
}

fn hand_count(snapshot: &Value, seat: &str, card_id: &str) -> usize {
    snapshot["players"][seat]["hand"]["spellbook"]
        .as_array()
        .map(|hand| hand.iter().filter(|card| card["cardId"] == card_id).count())
        .unwrap_or_default()
}

fn seed_supplemental(
    start: u32,
    north_titan: usize,
    south_plain: usize,
    south_raider: usize,
) -> String {
    (start..start + 8192)
        .chain(675..675 + 8192)
        .map(strike_supplemental_manifest)
        .find(|candidate| {
            let north = opening_spell_ids(candidate, "north");
            let south = opening_spell_ids(candidate, "south");
            north.iter().filter(|card| *card == "north-titan").count() >= north_titan
                && south.iter().filter(|card| *card == "south-plain").count() >= south_plain
                && south.iter().filter(|card| *card == "south-raider").count() >= south_raider
        })
        .expect("bounded supplemental genesis-strike seed")
}

fn strike_targets(receipt: &Receipt) -> Vec<String> {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| {
            event.payload["targetInstanceId"]
                .as_str()
                .expect("struck identity")
                .to_owned()
        })
        .collect()
}

fn decline_attacks(session: &mut Session) {
    while try_accept_where(session, |descriptor| descriptor["kind"] == "decline-attack").is_some() {
    }
}

fn end_turn(session: &mut Session) {
    decline_attacks(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn draw_zone(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn try_draw_zone(session: &mut Session, zone: &str) -> Option<(Value, Receipt)> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    })
}

fn draw_any(session: &mut Session) -> Option<(Value, Receipt)> {
    try_draw_zone(session, "spellbook").or_else(|| try_draw_zone(session, "atlas"))
}

fn south_summons_at(session: &mut Session, cards: &[&str], cell: &str) -> Vec<String> {
    end_turn(session);
    draw_zone(session, "spellbook");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let ids = cards
        .iter()
        .map(|card_id| summon_at(session, card_id, cell).0)
        .collect();
    end_turn(session);
    draw_zone(session, "spellbook");
    ids
}

#[test]
fn rule_catalog_2343_struck_avatar_life_stays_lost_after_turns_pass() {
    let encoded = seed_supplemental(2343, 1, 1, 0);
    let mut session = opening_main(&encoded);
    let enemy_id = south_summons_at(&mut session, &["south-plain"], "C1")[0].clone();
    let (_, receipt) = summon_at(&mut session, "north-titan", "C1");
    assert!(strike_targets(&receipt).contains(&enemy_id));
    assert_eq!(unit(&state(&session), &enemy_id)["damage"], 3);
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 17);
    end_turn(&mut session);
    draw_any(&mut session).expect("south draw after genesis strike");
    assert_eq!(unit(&state(&session), &enemy_id)["location"], "C1");
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 17);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2344_second_titan_on_an_empty_own_site_emits_no_strike() {
    let encoded = seed_supplemental(2344, 2, 1, 0);
    let mut session = opening_main(&encoded);
    let enemy_id = south_summons_at(&mut session, &["south-plain"], "C1")[0].clone();
    assert!(hand_count(&state(&session), "north", "north-titan") >= 2);
    let (_, first) = summon_at(&mut session, "north-titan", "C1");
    assert!(strike_targets(&first).contains(&enemy_id));
    let (_, second) = summon_at(&mut session, "north-titan", "C4");
    assert_eq!(event_types(&second), ["minion-summoned"]);
    assert!(strike_targets(&second).is_empty());
    assert_eq!(unit(&state(&session), &enemy_id)["damage"], 3);
    assert_exact_replay(&session);
}

fn try_second_titan_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_draw_zone(&mut session, "spellbook")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-plain"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_draw_zone(&mut session, "spellbook")?;
    if hand_count(&state(&session), "north", "north-titan") < 2 {
        return None;
    }
    summon_at(&mut session, "north-titan", "C1");
    decline_attacks(&mut session);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    draw_any(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let (visitor, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && (descriptor["cardId"] == "south-plain" || descriptor["cardId"] == "south-raider")
            && descriptor["cell"] == "C2"
            && descriptor["region"].is_null()
    })?;
    let visitor_id = visitor["cardInstanceId"].as_str()?.to_owned();
    decline_attacks(&mut session);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_draw_zone(&mut session, "spellbook")?;
    (hand_count(&state(&session), "north", "north-titan") >= 1).then_some((session, visitor_id))
}

fn seed_for_second_titan_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(675..675 + 8192)
        .map(strike_supplemental_manifest)
        .find(|candidate| try_second_titan_enemy_arrival_prefix(candidate).is_some())
        .expect("bounded seed reaching second genesis-strike enemy-arrival setup")
}

#[test]
fn rule_catalog_2345_second_titan_strikes_a_newly_arrived_enemy_after_enemy_site_placement() {
    let encoded = seed_for_second_titan_enemy_arrival(2345);
    let (mut session, visitor_id) = try_second_titan_enemy_arrival_prefix(&encoded)
        .expect("second genesis-strike enemy-arrival prefix");
    let (_, receipt) = summon_at(&mut session, "north-titan", "C2");
    assert!(strike_targets(&receipt).contains(&visitor_id));
    assert_eq!(unit(&state(&session), &visitor_id)["damage"], 3);
    assert_eq!(unit(&state(&session), &visitor_id)["location"], "C2");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2346_genesis_strike_hits_every_co_located_enemy_minion() {
    let encoded = seed_supplemental(2346, 1, 2, 0);
    let mut session = opening_main(&encoded);
    let enemy_ids = south_summons_at(&mut session, &["south-plain", "south-plain"], "C1");
    let (_, receipt) = summon_at(&mut session, "north-titan", "C1");
    let struck = strike_targets(&receipt);
    for enemy_id in &enemy_ids {
        assert!(struck.contains(enemy_id));
        assert_eq!(unit(&state(&session), enemy_id)["damage"], 3);
    }
    assert_eq!(enemy_ids.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2347_genesis_strike_leaves_a_far_enemy_untouched() {
    let encoded = seed_supplemental(2347, 1, 1, 1);
    let mut session = opening_main(&encoded);
    end_turn(&mut session);
    draw_zone(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let nearby_id = summon_at(&mut session, "south-plain", "C1").0;
    let far_id = summon_at(&mut session, "south-raider", "C4").0;
    end_turn(&mut session);
    draw_zone(&mut session, "spellbook");
    let (_, receipt) = summon_at(&mut session, "north-titan", "C1");
    let struck = strike_targets(&receipt);
    assert!(struck.contains(&nearby_id));
    assert!(!struck.contains(&far_id));
    assert_eq!(unit(&state(&session), &nearby_id)["damage"], 3);
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

fn try_second_titan_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_draw_zone(&mut session, "spellbook")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-plain"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_draw_zone(&mut session, "spellbook")?;
    if hand_count(&state(&session), "north", "north-titan") < 2 {
        return None;
    }
    summon_at(&mut session, "north-titan", "C1");
    decline_attacks(&mut session);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    draw_any(&mut session)?;
    let (newcomer, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-plain"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let new_id = newcomer["cardInstanceId"].as_str()?.to_owned();
    decline_attacks(&mut session);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_draw_zone(&mut session, "spellbook")?;
    (hand_count(&state(&session), "north", "north-titan") >= 1).then_some((session, new_id))
}

fn seed_for_second_titan_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(675..675 + 8192)
        .map(strike_supplemental_manifest)
        .find(|candidate| try_second_titan_new_summon_prefix(candidate).is_some())
        .expect("bounded seed reaching second genesis-strike new-summon setup")
}

#[test]
fn rule_catalog_2348_second_titan_strikes_a_newly_summoned_enemy() {
    let encoded = seed_for_second_titan_new_summon(2348);
    let (mut session, new_id) = try_second_titan_new_summon_prefix(&encoded)
        .expect("second genesis-strike new-summon prefix");
    let (_, receipt) = summon_at(&mut session, "north-titan", "C1");
    assert!(strike_targets(&receipt).contains(&new_id));
    assert_eq!(unit(&state(&session), &new_id)["damage"], 3);
    assert_eq!(unit(&state(&session), &new_id)["location"], "C1");
    assert_exact_replay(&session);
}
