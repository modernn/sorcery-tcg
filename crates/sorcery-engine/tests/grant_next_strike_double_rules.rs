//! Direct proofs for grant-double-damage-next-strike Magic (RULE-CATALOG-0565–0566,
//! RULE-CATALOG-0979).
//!
//! Official Magic can mark an ally so its next unit strike this turn deals
//! double damage. The grant uses the shared ally choice, doubles only unit
//! strikes, is consumed after that strike, and expires through the shared End
//! Phase temporary-effect cleanup. An attack-2 minion kills a defense-3 enemy
//! only while the mark remains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn striker() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn tough_any_site() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_any_site() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn grant() -> Value {
    json!({
        "cardType": "magic",
        "grantDoubleDamageToAllyNextStrikeThisTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest_cards(enemy_card: &str, enemy: Value) -> Value {
    let mut cards = json!({
        "north-ally": striker(),
        "north-avatar": avatar(),
        "north-grant": grant(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    cards[enemy_card] = enemy;
    cards
}

fn manifest() -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-next-strike-double" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-next-strike-double-v1",
        },
        "cards": manifest_cards("south-tough", tough_any_site()),
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-grant", "north-grant"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-tough"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    }))
}

fn deathrite_manifest() -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "grant-next-strike-double-deathrite"
            }))
            .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-next-strike-double-deathrite-v1",
        },
        "cards": manifest_cards("south-deathrite", deathrite_any_site()),
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-grant", "north-grant"],
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
        "seed": 1,
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

fn unit<'a>(after: &'a Value, instance_id: &str) -> &'a Value {
    after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("unit")
}

fn cemetery_has(session: &Session, seat: &str, instance_id: &str) -> bool {
    state(session)["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn opening_main_with(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("grant next-strike double");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn opening_main() -> Session {
    opening_main_with(&manifest())
}

fn summon_north_ally(session: &mut Session) -> String {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned()
}

fn grant_double(session: &mut Session, ally_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-grant"
            && descriptor["ally"]["instanceId"] == ally_id
    })
}

fn south_summons_enemy_at_c4(session: &mut Session, card_id: &str) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
}

fn south_summons_tough_at_c4(session: &mut Session) -> String {
    south_summons_enemy_at_c4(session, "south-tough")
}

fn atlas_len(session: &Session, seat: &str) -> usize {
    state(session)["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn strike_minion(session: &mut Session, attacker_id: &str, enemy_id: &str) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C4"
    });
    while state(session)["phase"] == "movement" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
    }
    accept_where(session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == enemy_id
    });
    let (_, mut fight) = accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(session)["phase"] == "intercept" {
        (_, fight) = accept_where(session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
    fight
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
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0565_granted_next_strike_double_kills_a_tougher_minion() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_tough_at_c4(&mut session);
    let ready = state(&session);
    assert_eq!(unit(&ready, &ally_id)["summoningSickness"], false);
    assert_eq!(
        session.public_view(Seat::North).expect("North public view")["realm"]["units"]
            .as_array()
            .expect("public units")
            .iter()
            .find(|unit| unit["instanceId"] == enemy_id)
            .expect("public enemy")["defense"],
        3
    );

    let mut wounded = session.clone();
    let wounded_fight = strike_minion(&mut wounded, &ally_id, &enemy_id);
    assert!(wounded_fight.events.iter().any(|event| {
        event.event_type == "strike-damage-allocated"
            && event.payload["amount"] == 2
            && event.payload["targetInstanceId"] == enemy_id
    }));
    let after_wound = state(&wounded);
    assert_eq!(unit(&after_wound, &enemy_id)["damage"], 2);
    assert!(!cemetery_has(&wounded, "south", &enemy_id));

    let (descriptor, receipt) = grant_double(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "next-strike-double-granted", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], ally_id);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    let granted = state(&session);
    assert_eq!(
        unit(&granted, &ally_id)["temporaryNextStrikeDoubleSources"],
        json!([descriptor["cardInstanceId"]])
    );

    let doubled = strike_minion(&mut session, &ally_id, &enemy_id);
    assert!(doubled.events.iter().any(|event| {
        event.event_type == "strike-damage-allocated"
            && event.payload["amount"] == 4
            && event.payload["targetInstanceId"] == enemy_id
    }));
    assert!(doubled.events.iter().any(|event| {
        event.event_type == "next-strike-double-consumed"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == descriptor["cardInstanceId"]
    }));
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert!(
        !state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["instanceId"] == enemy_id)
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .find(|unit| unit["instanceId"] == ally_id)
            .expect("surviving ally")
            .get("temporaryNextStrikeDoubleSources")
            .is_none()
    );
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0979_granted_next_strike_double_triggers_deathrite_draw_on_kill() {
    let mut session = opening_main_with(&deathrite_manifest());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_enemy_at_c4(&mut session, "south-deathrite");
    let (_, receipt) = grant_double(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "next-strike-double-granted", "magic-resolved"]
    );

    let north_atlas = atlas_len(&session, "north");
    let south_atlas = atlas_len(&session, "south");
    let doubled = strike_minion(&mut session, &ally_id, &enemy_id);
    assert!(doubled.events.iter().any(|event| {
        event.event_type == "strike-damage-allocated"
            && event.payload["amount"] == 4
            && event.payload["targetInstanceId"] == enemy_id
    }));
    assert!(doubled.events.iter().any(|event| {
        event.event_type == "next-strike-double-consumed"
            && event.payload["instanceId"] == ally_id
    }));
    let drawn = doubled
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");
    assert_eq!(drawn.payload["sourceInstanceId"], enemy_id);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_eq!(atlas_len(&session, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&session, "south"), south_atlas);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0566_next_strike_double_expires_before_a_later_strike() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_tough_at_c4(&mut session);
    let (descriptor, _) = grant_double(&mut session, &ally_id);
    assert_eq!(
        unit(&state(&session), &ally_id)["temporaryNextStrikeDoubleSources"],
        json!([descriptor["cardInstanceId"]])
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "next-strike-double-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == descriptor["cardInstanceId"]
    }));
    assert!(
        unit(&state(&session), &ally_id)
            .get("temporaryNextStrikeDoubleSources")
            .is_none()
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    let later = strike_minion(&mut session, &ally_id, &enemy_id);
    assert!(later.events.iter().any(|event| {
        event.event_type == "strike-damage-allocated"
            && event.payload["amount"] == 2
            && event.payload["targetInstanceId"] == enemy_id
    }));
    assert!(
        !later
            .events
            .iter()
            .any(|event| event.event_type == "next-strike-double-consumed")
    );
    let after = state(&session);
    assert_eq!(unit(&after, &enemy_id)["damage"], 2);
    assert!(!cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("grant-next-strike-double checkpoint");
    let serialized =
        serialize_game_checkpoint(&checkpoint).expect("serialized grant-next-strike-double");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed grant-next-strike-double");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed grant-next-strike-double session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}
