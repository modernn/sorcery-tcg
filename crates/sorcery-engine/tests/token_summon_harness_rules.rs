//! Direct proofs for token Magic summon identity and banishment
//! (RULE-CATALOG-0014, RULE-CATALOG-0687–0688, 2403–2408).
//!
//! 0579–0580 prove bordering-site placement and the empty no-op. 1873–1878
//! prove persistence/repeat/enemy-arrival/multi/far/new-site on the Border
//! Militia fixture without the identity recipe or cemetery-banish check.
//! 0372 proves a 2x2 Genesis token banishes. These proofs keep the 0014
//! harness slice: identities hash from cell, ordinal, source, and state
//! version with no random draws, and a Magic-summoned 1x1 token that dies
//! in combat is banished instead of entering a cemetery.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

const TOKEN_ID: &str = "foot-soldier-token";

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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn token() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    })
}

fn token_magic() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "summonTokenToEachControlledSiteBorderingEnemySite": TOKEN_ID,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn token_summon_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-summon-harness" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-summon-harness-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-magic": token_magic(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
            TOKEN_ID: token(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-magic"; 6],
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid token-summon harness session");
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

fn north_magic_count(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map_or(0, |hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-magic")
                .count()
        })
}

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(token_summon_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_magic_count(&state(&preview)) >= 2)
        })
        .expect("bounded seed with two token Magic cards in the opening hand")
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

fn setup_bordering_sites(session: &mut Session) -> String {
    for cell in ["C1", "C3", "C2", "B3", "B2"] {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == cell
        });
    }
    let (summoned_attacker, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "B2"
    });
    let attacker_id = summoned_attacker["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    attacker_id
}

fn play_to_token_summon(session: &mut Session) -> (Value, Receipt, String, u64) {
    let (_, empty) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor.get("cemeteryMinionInstanceId").is_none()
    });
    assert_eq!(event_types(&empty), ["magic-cast", "magic-resolved"]);
    assert!(
        state(session)["realm"]["units"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );

    let attacker_id = setup_bordering_sites(session);
    let pre_cast_version = state(session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, summoned) = accept_where(session, |descriptor| descriptor["kind"] == "cast-magic");
    (cast, summoned, attacker_id, pre_cast_version)
}

fn expected_token_ids(source_id: &str, pre_cast_version: u64) -> Vec<String> {
    ["B3", "C3"]
        .into_iter()
        .enumerate()
        .map(|(ordinal, cell)| {
            identity_hash(&json!({
                "cardId": TOKEN_ID,
                "cell": cell,
                "ordinal": ordinal,
                "owner": "north",
                "source": "token",
                "sourceInstanceId": source_id,
                "stateVersion": pre_cast_version,
            }))
            .expect("expected token identity")
            .to_string()
        })
        .collect()
}

#[test]
fn rule_catalog_0687_token_magic_summons_in_cell_order_with_deterministic_identities() {
    let encoded = seed_with(687);
    let mut session = opening_main(&encoded);
    let (cast, summoned, _, pre_cast_version) = play_to_token_summon(&mut session);
    assert_eq!(
        event_types(&summoned),
        [
            "magic-cast",
            "minion-summoned",
            "minion-summoned",
            "magic-resolved",
        ]
    );
    assert_eq!(
        summoned.events[1..3]
            .iter()
            .map(|event| event.payload["cell"].as_str().expect("token cell"))
            .collect::<Vec<_>>(),
        ["B3", "C3"]
    );
    assert!(summoned.random_draws.is_empty());
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    assert!(
        summoned.events[1..3]
            .iter()
            .all(|event| event.payload["sourceInstanceId"] == source_id)
    );
    assert_eq!(
        summoned.events[1..3]
            .iter()
            .map(|event| event.payload["instanceId"]
                .as_str()
                .expect("token identity"))
            .collect::<Vec<_>>(),
        expected_token_ids(source_id, pre_cast_version)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0688_dead_token_banishes_instead_of_entering_a_cemetery() {
    let encoded = seed_with(688);
    let mut session = opening_main(&encoded);
    let (_, summoned, attacker_id, _) = play_to_token_summon(&mut session);
    let killed_id = summoned.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity")
        .to_owned();

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "B3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == killed_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let token_exits: Vec<_> = fight
        .events
        .iter()
        .filter(|event| event.payload["instanceId"] == killed_id)
        .map(|event| event.event_type.as_str())
        .filter(|event_type| matches!(*event_type, "minion-died" | "minion-banished"))
        .collect();
    assert_eq!(token_exits, ["minion-died", "minion-banished"]);
    assert!(
        state(&session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != killed_id)
    );
    assert_exact_replay(&session);
}

fn token_summon_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "token-summon-harness-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-token-summon-harness-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-magic": token_magic(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
            TOKEN_ID: token(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-magic"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
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

fn opening_spell_ids(encoded: &str) -> Vec<String> {
    state(&Session::new(encoded).expect("preview session"))["players"]["north"]["hand"]["spellbook"]
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

fn seed_supplemental_with(start: u32, min_magic: usize) -> String {
    (start..start + 2048)
        .chain(687..687 + 2048)
        .map(token_summon_supplemental_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .filter(|id| *id == "north-magic")
                .count()
                >= min_magic
        })
        .expect("bounded seed with required token Magic in the opening hand")
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

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .ok()
        .is_some_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn decline_attack_if_needed(session: &mut Session) {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

fn end_turn_if_offered(session: &mut Session) {
    decline_attack_if_needed(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn cast_token_magic(session: &mut Session) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-magic"
    })
}

fn token_magic_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("token Magic actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-magic"
        })
        .count()
}

fn magic_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map_or(0, |hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-magic")
                .count()
        })
}

fn token_cells(snapshot: &Value) -> Vec<String> {
    snapshot["realm"]["units"]
        .as_array()
        .map(|units| {
            units
                .iter()
                .filter(|unit| unit["cardId"] == TOKEN_ID)
                .map(|unit| {
                    unit["location"]
                        .as_str()
                        .expect("token location")
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn summoned_token_cells(receipt: &Receipt) -> Vec<String> {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-summoned")
        .map(|event| {
            event.payload["cell"]
                .as_str()
                .expect("summoned cell")
                .to_owned()
        })
        .collect()
}

fn cemetery_lacks(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .all(|card| card["instanceId"] != instance_id)
}

fn unit_id_at(snapshot: &Value, cell: &str) -> Option<String> {
    snapshot["realm"]["units"].as_array().and_then(|units| {
        units.iter().find_map(|unit| {
            (unit["location"] == cell && unit["cardId"] == TOKEN_ID)
                .then(|| unit["instanceId"].as_str().expect("token id").to_owned())
        })
    })
}

fn assert_hashed_tokens_on_board(session: &Session, source_id: &str, pre_cast_version: u64) {
    let expected = expected_token_ids(source_id, pre_cast_version);
    let snapshot = state(session);
    assert_eq!(
        unit_id_at(&snapshot, "B3").as_deref(),
        Some(expected[0].as_str())
    );
    assert_eq!(
        unit_id_at(&snapshot, "C3").as_deref(),
        Some(expected[1].as_str())
    );
    assert!(unit_id_at(&snapshot, "C1").is_none());
    assert!(unit_id_at(&snapshot, "C4").is_none());
    for token_id in &expected {
        assert!(cemetery_lacks(&snapshot, "north", token_id));
    }
}

fn identity_for_cast(source_id: &str, cell: &str, ordinal: usize, pre_cast_version: u64) -> String {
    identity_hash(&json!({
        "cardId": TOKEN_ID,
        "cell": cell,
        "ordinal": ordinal,
        "owner": "north",
        "source": "token",
        "sourceInstanceId": source_id,
        "stateVersion": pre_cast_version,
    }))
    .expect("expected token identity")
    .to_string()
}

fn try_second_token_enemy_arrival_prefix(encoded: &str) -> Option<Session> {
    let mut session = opening_main(encoded);
    setup_bordering_sites(&mut session);
    let first = cast_token_magic(&mut session).1;
    if !event_types(&first).contains(&"minion-summoned") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if magic_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D2"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "D2"
            && descriptor["region"].is_null()
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (token_magic_casts(&session) >= 1).then_some(session)
}

fn seed_for_second_token_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(687..687 + 8192)
        .find_map(|seed| {
            let encoded = token_summon_supplemental_manifest(seed);
            if opening_spell_ids(&encoded)
                .iter()
                .filter(|card| *card == "north-magic")
                .count()
                < 1
            {
                return None;
            }
            try_second_token_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second token Magic enemy-arrival setup")
}

struct SecondTokenSummonSetup {
    new_cell: String,
    session: Session,
}

fn try_second_token_summon_prefix(encoded: &str) -> Option<SecondTokenSummonSetup> {
    let mut session = opening_main(encoded);
    setup_bordering_sites(&mut session);
    let first = cast_token_magic(&mut session).1;
    if !event_types(&first).contains(&"minion-summoned") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if magic_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D2"
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
    })?;
    (token_magic_casts(&session) >= 1).then_some(SecondTokenSummonSetup {
        new_cell: "D3".to_owned(),
        session,
    })
}

fn seed_for_second_token_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(687..687 + 8192)
        .find_map(|seed| {
            let encoded = token_summon_supplemental_manifest(seed);
            if !opening_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-magic")
            {
                return None;
            }
            try_second_token_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second token Magic new-site setup")
}

fn assert_new_token_identity(cast: &Value, receipt: &Receipt, pre_cast_version: u64, cell: &str) {
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    let cells = summoned_token_cells(receipt);
    let ordinal = cells
        .iter()
        .position(|summoned| summoned == cell)
        .expect("expected a token on the new cell");
    let summoned_id = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-summoned")
        .nth(ordinal)
        .expect("summoned event")
        .payload["instanceId"]
        .as_str()
        .expect("token identity");
    assert_eq!(
        summoned_id,
        identity_for_cast(source_id, cell, ordinal, pre_cast_version)
    );
    assert!(receipt.random_draws.is_empty());
}

#[test]
fn rule_catalog_2403_summoned_token_identities_stay_on_board_after_turns_pass() {
    let encoded = seed_supplemental_with(2403, 1);
    let mut session = opening_main(&encoded);
    setup_bordering_sites(&mut session);
    let pre_cast_version = state(&session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, receipt) = cast_token_magic(&mut session);
    assert_eq!(summoned_token_cells(&receipt), ["B3", "C3"]);
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    assert_hashed_tokens_on_board(&session, source_id, pre_cast_version);
    pass_turn_to_north_spellbook(&mut session);
    assert_hashed_tokens_on_board(&session, source_id, pre_cast_version);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2404_second_token_magic_without_bordering_sites_is_a_paid_noop() {
    let encoded = seed_supplemental_with(2404, 2);
    let mut session = opening_main(&encoded);
    let first = cast_token_magic(&mut session).1;
    assert_eq!(event_types(&first), ["magic-cast", "magic-resolved"]);
    let second = cast_token_magic(&mut session).1;
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(
        !second
            .events
            .iter()
            .any(|event| event.event_type == "minion-summoned")
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2405_second_token_magic_summons_a_hashed_identity_after_enemy_arrival() {
    let encoded = seed_for_second_token_enemy_arrival(2405);
    let mut session = try_second_token_enemy_arrival_prefix(&encoded)
        .expect("second token Magic enemy-arrival prefix");
    let before_cells = token_cells(&state(&session));
    let pre_cast_version = state(&session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, receipt) = cast_token_magic(&mut session);
    let summoned_cells = summoned_token_cells(&receipt);
    let new_cell = summoned_cells
        .iter()
        .find(|cell| !before_cells.contains(cell))
        .expect("expected at least one newly summoned bordering cell");
    assert_new_token_identity(&cast, &receipt, pre_cast_version, new_cell);
    assert!(cemetery_lacks(
        &state(&session),
        "north",
        unit_id_at(&state(&session), new_cell)
            .expect("new token")
            .as_str(),
    ));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2406_token_magic_hashes_an_identity_on_every_bordering_site() {
    let encoded = seed_supplemental_with(2406, 1);
    let mut session = opening_main(&encoded);
    setup_bordering_sites(&mut session);
    let pre_cast_version = state(&session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, receipt) = cast_token_magic(&mut session);
    assert_eq!(summoned_token_cells(&receipt), ["B3", "C3"]);
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    assert_hashed_tokens_on_board(&session, source_id, pre_cast_version);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2407_token_magic_leaves_a_non_bordering_site_without_a_token_identity() {
    let encoded = seed_supplemental_with(2407, 1);
    let mut session = opening_main(&encoded);
    setup_bordering_sites(&mut session);
    let pre_cast_version = state(&session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, _) = cast_token_magic(&mut session);
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    assert_hashed_tokens_on_board(&session, source_id, pre_cast_version);
    assert!(!token_cells(&state(&session)).contains(&"C4".to_owned()));
    assert!(!token_cells(&state(&session)).contains(&"C1".to_owned()));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2408_second_token_magic_hashes_an_identity_on_a_newly_placed_bordering_site() {
    let encoded = seed_for_second_token_summon(2408);
    let SecondTokenSummonSetup {
        mut session,
        new_cell,
    } = try_second_token_summon_prefix(&encoded).expect("second token Magic new-site prefix");
    let pre_cast_version = state(&session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, receipt) = cast_token_magic(&mut session);
    assert!(
        summoned_token_cells(&receipt).contains(&new_cell),
        "expected a token on the newly bordering site {new_cell}"
    );
    assert_new_token_identity(&cast, &receipt, pre_cast_version, &new_cell);
    assert!(token_cells(&state(&session)).contains(&new_cell));
    assert!(cemetery_lacks(
        &state(&session),
        "north",
        unit_id_at(&state(&session), &new_cell)
            .expect("new token")
            .as_str(),
    ));
    assert_exact_replay(&session);
}
