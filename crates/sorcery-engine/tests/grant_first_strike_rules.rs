//! Direct proofs for grant-First-Strike-this-turn Magic (RULE-CATALOG-0282–0283,
//! RULE-CATALOG-1108, RULE-CATALOG-1533–1535, RULE-CATALOG-1537, RULE-CATALOG-1539,
//! RULE-CATALOG-1545–1548, RULE-CATALOG-1564–1568, RULE-CATALOG-1573–1578).
//!
//! Official Magic can grant First Strike for the current turn. The grant uses
//! the same ally choice as Charge, persists only on minions, and expires
//! through the shared End Phase temporary-effect cleanup. Granted First Strike
//! applies while attacking and while defending. Equal 3/3 combat without it is
//! simultaneous; with it the granted attacker kills before taking return damage.
//! While trigger-order is pending, the grant is withheld until the chain
//! completes.

#[path = "common/mod.rs"]
mod common;
use common::modifier_sources;

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

fn fighter() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn printed_first_strike_fighter() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "strikesFirstWhileAttacking": true,
        "strikesFirstWhileDefending": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn visitor() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn grant() -> Value {
    json!({
        "cardType": "magic",
        "grantFirstStrikeToAllyThisTurn": true,
        "manaCost": 0,
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

fn deathrite_minion() -> Value {
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest_with_north_ally(ally: &Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-first-strike" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-first-strike-v1",
        },
        "cards": {
            "north-ally": ally,
            "north-avatar": avatar(),
            "north-grant": grant(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-grant", "north-grant"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-visitor"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    }))
}

fn manifest() -> String {
    manifest_with_north_ally(&fighter())
}

fn opening_main_with_ally(ally: &Value) -> Session {
    let mut session = Session::new(&manifest_with_north_ally(ally)).expect("grant First Strike");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn defending_only_fighter() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "strikesFirstWhileDefending": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn attacking_only_fighter() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "strikesFirstWhileAttacking": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    try_accept_where(session, predicate).expect("expected engine-issued action")
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

fn opening_main() -> Session {
    let mut session = Session::new(&manifest()).expect("grant First Strike");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
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

fn grant_first_strike(session: &mut Session, ally_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-grant"
            && descriptor["ally"]["instanceId"] == ally_id
    })
}

fn grant_ally_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("grant actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-grant"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn south_summons_visitor_at_c4(session: &mut Session) -> String {
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
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
}

fn through_north_pass_to_south_main(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn through_south_pass_to_north_main(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn south_attacks_north_ally(session: &mut Session, attacker_id: &str, defender_id: &str) {
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
            && descriptor["target"]["instanceId"] == defender_id
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(session)["phase"] == "intercept" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
}

fn strike_minion(session: &mut Session, attacker_id: &str, enemy_id: &str) {
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
    accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(session)["phase"] == "intercept" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
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
fn rule_catalog_0282_grant_first_strike_lasts_only_until_end_of_turn() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let before = state(&session);
    assert!(
        modifier_sources(unit(&before, &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );

    let (descriptor, receipt) = grant_first_strike(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "first-strike-granted", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], ally_id);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    let granted = state(&session);
    assert_eq!(
        modifier_sources(unit(&granted, &ally_id), "first-strike"),
        json!([descriptor["cardInstanceId"]])
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "first-strike-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == descriptor["cardInstanceId"]
    }));
    let after = state(&session);
    assert!(
        modifier_sources(unit(&after, &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("grant-first-strike checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized grant-first-strike");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed grant-first-strike");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed grant-first-strike session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0283_granted_first_strike_kills_before_return_damage() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    let ready = state(&session);
    assert_eq!(unit(&ready, &ally_id)["summoningSickness"], false);

    let mut simultaneous = session.clone();
    strike_minion(&mut simultaneous, &ally_id, &enemy_id);
    assert!(cemetery_has(&simultaneous, "north", &ally_id));
    assert!(cemetery_has(&simultaneous, "south", &enemy_id));

    grant_first_strike(&mut session, &ally_id);
    strike_minion(&mut session, &ally_id, &enemy_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert!(
        !after["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["instanceId"] == enemy_id)
    );
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1533_granted_first_strike_kills_before_attacker_strikes_while_defending() {
    let mut session = opening_main_with_ally(&defending_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    grant_first_strike(&mut session, &ally_id);
    through_north_pass_to_south_main(&mut session);
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert!(
        modifier_sources(unit(&after, &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1534_printed_and_granted_first_strike_compose_while_attacking() {
    let mut session = opening_main_with_ally(&printed_first_strike_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    let (descriptor, _) = grant_first_strike(&mut session, &ally_id);
    assert!(
        modifier_sources(unit(&state(&session), &ally_id), "first-strike")
            .as_array()
            .is_some_and(|sources| sources.len() == 1)
    );
    strike_minion(&mut session, &ally_id, &enemy_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_eq!(
        &descriptor["cardInstanceId"],
        modifier_sources(unit(&state(&session), &ally_id), "first-strike")
            .as_array()
            .and_then(|sources| sources.first())
            .expect("grant source")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1535_printed_and_granted_first_strike_compose_while_defending() {
    let mut session = opening_main_with_ally(&printed_first_strike_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    let (descriptor, _) = grant_first_strike(&mut session, &ally_id);
    assert_eq!(
        &descriptor["cardInstanceId"],
        modifier_sources(unit(&state(&session), &ally_id), "first-strike")
            .as_array()
            .and_then(|sources| sources.first())
            .expect("grant source")
    );
    through_north_pass_to_south_main(&mut session);
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1537_defending_only_printed_plus_grant_strikes_first_while_attacking() {
    let mut session = opening_main_with_ally(&defending_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);

    let mut simultaneous = session.clone();
    strike_minion(&mut simultaneous, &ally_id, &enemy_id);
    assert!(cemetery_has(&simultaneous, "north", &ally_id));
    assert!(cemetery_has(&simultaneous, "south", &enemy_id));

    grant_first_strike(&mut session, &ally_id);
    strike_minion(&mut session, &ally_id, &enemy_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1539_granted_first_strike_expires_before_opponent_turn_combat() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    grant_first_strike(&mut session, &ally_id);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        modifier_sources(unit(&state(&session), &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    assert!(cemetery_has(&session, "north", &ally_id));
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1545_attacking_only_printed_plus_grant_trades_after_grant_expires_while_defending()
{
    let mut session = opening_main_with_ally(&attacking_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    grant_first_strike(&mut session, &ally_id);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        modifier_sources(unit(&state(&session), &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    assert!(cemetery_has(&session, "north", &ally_id));
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1546_attacking_only_printed_plus_grant_strikes_first_while_attacking() {
    let mut session = opening_main_with_ally(&attacking_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    grant_first_strike(&mut session, &ally_id);
    strike_minion(&mut session, &ally_id, &enemy_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1547_attacking_only_printed_without_grant_trades_simultaneously_while_defending() {
    let mut session = opening_main_with_ally(&attacking_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    through_north_pass_to_south_main(&mut session);
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    assert!(cemetery_has(&session, "north", &ally_id));
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1548_defending_only_printed_without_grant_strikes_first_while_defending() {
    let mut session = opening_main_with_ally(&defending_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    through_north_pass_to_south_main(&mut session);
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1556_defending_only_printed_without_grant_trades_simultaneously_while_attacking() {
    let mut session = opening_main_with_ally(&defending_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    strike_minion(&mut session, &ally_id, &enemy_id);
    assert!(cemetery_has(&session, "north", &ally_id));
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1557_attacking_only_printed_without_grant_strikes_first_while_attacking() {
    let mut session = opening_main_with_ally(&attacking_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    strike_minion(&mut session, &ally_id, &enemy_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1564_printed_first_strike_without_grant_strikes_first_while_attacking() {
    let mut session = opening_main_with_ally(&printed_first_strike_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    strike_minion(&mut session, &ally_id, &enemy_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1565_printed_first_strike_without_grant_strikes_first_while_defending() {
    let mut session = opening_main_with_ally(&printed_first_strike_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    through_north_pass_to_south_main(&mut session);
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1566_defending_only_printed_plus_grant_trades_after_grant_expires_while_attacking()
{
    let mut session = opening_main_with_ally(&defending_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    grant_first_strike(&mut session, &ally_id);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        modifier_sources(unit(&state(&session), &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    through_south_pass_to_north_main(&mut session);
    strike_minion(&mut session, &ally_id, &enemy_id);
    assert!(cemetery_has(&session, "north", &ally_id));
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1567_attacking_only_printed_plus_grant_strikes_first_after_grant_expires_while_attacking()
 {
    let mut session = opening_main_with_ally(&attacking_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    grant_first_strike(&mut session, &ally_id);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        modifier_sources(unit(&state(&session), &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    through_south_pass_to_north_main(&mut session);
    strike_minion(&mut session, &ally_id, &enemy_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1568_printed_first_strike_plus_grant_strikes_first_after_grant_expires_while_defending()
 {
    let mut session = opening_main_with_ally(&printed_first_strike_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    grant_first_strike(&mut session, &ally_id);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        modifier_sources(unit(&state(&session), &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

fn expire_grant_first_strike(session: &mut Session, ally_id: &str, grant_source: &str) {
    let (_, ended) = accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "first-strike-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == grant_source
    }));
    assert!(
        modifier_sources(unit(&state(session), ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

#[test]
fn rule_catalog_1573_defending_only_printed_plus_grant_strikes_first_after_grant_expires_while_defending()
 {
    let mut session = opening_main_with_ally(&defending_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    let (descriptor, _) = grant_first_strike(&mut session, &ally_id);
    expire_grant_first_strike(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1574_printed_first_strike_plus_grant_strikes_first_after_grant_expires_while_attacking()
 {
    let mut session = opening_main_with_ally(&printed_first_strike_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    let (descriptor, _) = grant_first_strike(&mut session, &ally_id);
    expire_grant_first_strike(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    through_south_pass_to_north_main(&mut session);
    strike_minion(&mut session, &ally_id, &enemy_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1575_granted_first_strike_expires_before_ally_attacks_on_later_turn() {
    let mut session = opening_main();
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    let (descriptor, _) = grant_first_strike(&mut session, &ally_id);
    expire_grant_first_strike(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    through_south_pass_to_north_main(&mut session);
    strike_minion(&mut session, &ally_id, &enemy_id);
    assert!(cemetery_has(&session, "north", &ally_id));
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1576_attacking_only_printed_plus_grant_trades_after_grant_expires_while_defending()
{
    let mut session = opening_main_with_ally(&attacking_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    let (descriptor, _) = grant_first_strike(&mut session, &ally_id);
    expire_grant_first_strike(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    assert!(cemetery_has(&session, "north", &ally_id));
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1577_defending_only_printed_persists_after_grant_expires_while_defending() {
    let mut session = opening_main_with_ally(&defending_only_fighter());
    let ally_id = summon_north_ally(&mut session);
    let enemy_id = south_summons_visitor_at_c4(&mut session);
    let (descriptor, _) = grant_first_strike(&mut session, &ally_id);
    expire_grant_first_strike(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );

    let mut plain = opening_main();
    let plain_ally = summon_north_ally(&mut plain);
    let plain_enemy = south_summons_visitor_at_c4(&mut plain);
    let (plain_descriptor, plain_grant) = grant_first_strike(&mut plain, &plain_ally);
    expire_grant_first_strike(
        &mut plain,
        &plain_ally,
        plain_descriptor["cardInstanceId"]
            .as_str()
            .expect("grant source"),
    );
    south_attacks_north_ally(&mut plain, &plain_enemy, &plain_ally);
    assert!(cemetery_has(&plain, "north", &plain_ally));
    assert!(cemetery_has(&plain, "south", &plain_enemy));
    assert!(
        plain_grant
            .events
            .iter()
            .any(|event| event.event_type == "first-strike-granted")
    );

    south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1578_printed_first_strike_without_grant_strikes_first_in_both_roles() {
    {
        let mut session = opening_main_with_ally(&printed_first_strike_fighter());
        let ally_id = summon_north_ally(&mut session);
        let enemy_id = south_summons_visitor_at_c4(&mut session);
        strike_minion(&mut session, &ally_id, &enemy_id);
        let after = state(&session);
        assert_eq!(unit(&after, &ally_id)["damage"], 0);
        assert!(cemetery_has(&session, "south", &enemy_id));
        assert_exact_replay(&session);
    }
    {
        let mut session = opening_main_with_ally(&printed_first_strike_fighter());
        let ally_id = summon_north_ally(&mut session);
        let enemy_id = south_summons_visitor_at_c4(&mut session);
        through_north_pass_to_south_main(&mut session);
        south_attacks_north_ally(&mut session, &enemy_id, &ally_id);
        let after = state(&session);
        assert_eq!(unit(&after, &ally_id)["damage"], 0);
        assert!(cemetery_has(&session, "south", &enemy_id));
        assert_exact_replay(&session);
    }
}

fn deathrite_grant_first_strike_manifest(seed: u32) -> String {
    let fixture = "grant-first-strike-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": fighter(),
            "north-avatar": avatar(),
            "north-grant": grant(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-grant",
                    "north-rain",
                    "north-rain",
                    "north-grant",
                    "north-grant",
                ],
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

fn north_has_grant_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-grant", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteGrantFirstStrikeSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_allied_minion(
    encoded: &str,
) -> Option<PendingDeathriteGrantFirstStrikeSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_grant_and_rain(&state(&session)) {
        return None;
    }
    if !grant_ally_ids(&session).contains(&ally_id) {
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
    Some(PendingDeathriteGrantFirstStrikeSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_grant_first_strike_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_grant_first_strike_manifest)
        .find(|candidate| try_pending_deathrite_with_allied_minion(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with grant-First-Strike Magic in hand",
        )
}

#[test]
fn rule_catalog_1108_grant_first_strike_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_grant_first_strike_seed_with(1108);
    let mut setup = try_pending_deathrite_with_allied_minion(&encoded)
        .expect("complete grant-First-Strike Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
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
    assert!(unit(&paused, &ally_id).is_object());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(grant_ally_ids(session).is_empty());

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
    assert!(unit(&resumed, &ally_id).is_object());
    assert!(grant_ally_ids(session).contains(&ally_id));
    assert!(
        modifier_sources(unit(&resumed, &ally_id), "first-strike")
            .as_array()
            .is_some_and(Vec::is_empty)
    );

    let (descriptor, receipt) = grant_first_strike(session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "first-strike-granted", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["instanceId"], ally_id);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert_eq!(
        modifier_sources(unit(&state(session), &ally_id), "first-strike"),
        json!([descriptor["cardInstanceId"]])
    );
    assert_exact_replay(session);
}

#[test]
fn avatar_receives_first_strike_from_an_engine_issued_ally_grant_and_replays_expiry() {
    let mut session = opening_main();
    let before = state(&session);
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity");
    assert!(grant_ally_ids(&session).iter().any(|id| id == avatar_id));
    let (descriptor, _) = grant_first_strike(&mut session, avatar_id);
    assert_eq!(
        modifier_sources(
            &state(&session)["players"]["north"]["avatar"],
            "first-strike"
        ),
        json!([descriptor["cardInstanceId"]])
    );
    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        ended
            .events
            .iter()
            .any(|event| event.event_type == "first-strike-expired"
                && event.payload["instanceId"] == avatar_id)
    );
    assert_eq!(
        modifier_sources(
            &state(&session)["players"]["north"]["avatar"],
            "first-strike"
        ),
        json!([])
    );
    assert_exact_replay(&session);
}
