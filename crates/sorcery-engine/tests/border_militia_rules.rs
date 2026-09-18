//! Direct proofs for summon-token-to-each-controlled-site-bordering-enemy-site
//! Magic (RULE-CATALOG-0579–0580, 1079, RULE-CATALOG-1873–1878).
//!
//! Ordinary Magic summons one source-linked token onto each controlled site
//! that borders an enemy-controlled site, in stable cell order. When no site
//! qualifies, casting still resolves as a paid no-op. While Deathrites wait
//! for ordering, bordering-site token Magic stays withheld until the chain
//! drains.

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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn foot_soldier_token() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
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

fn border_militia() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "summonTokenToEachControlledSiteBorderingEnemySite": "foot-soldier-token",
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn border_militia_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "border-militia" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-border-militia-v1",
        },
        "cards": {
            "foot-soldier-token": foot_soldier_token(),
            "north-avatar": avatar(),
            "north-militia": border_militia(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid border-militia session");
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

fn opening_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"]["north"]["hand"]["spellbook"]
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

fn seed_with(required: &[&str]) -> String {
    (579..579 + 256)
        .map(border_militia_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn end_turn(session: &mut Session) {
    end_turn_if_offered(session);
}

fn draw_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn summon_at(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned instance identity")
        .to_owned()
}

fn token_units_at<'a>(snapshot: &'a Value, cells: &[&str]) -> Vec<&'a Value> {
    let units = snapshot["realm"]["units"].as_array().expect("realm units");
    cells
        .iter()
        .map(|cell| {
            units
                .iter()
                .find(|unit| unit["location"] == *cell && unit["cardId"] == "foot-soldier-token")
                .unwrap_or_else(|| panic!("expected token at {cell}"))
        })
        .collect()
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

fn end_and_draw_atlas(session: &mut Session) {
    end_turn(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn setup_bordering_sites(session: &mut Session) {
    for cell in ["C1", "C3", "C2", "B3", "B2"] {
        end_and_draw_atlas(session);
        play_site(session, cell);
    }
    summon_at(session, "south-minion", "B2");
    end_turn(session);
    draw_spellbook(session);
}

#[test]
fn rule_catalog_0579_border_militia_summons_tokens_on_bordering_sites_in_order() {
    let encoded = seed_with(&["north-militia"]);
    let mut session = opening_main(&encoded);
    setup_bordering_sites(&mut session);
    let pre_cast_version = state(&session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-summoned",
            "minion-summoned",
            "magic-resolved",
        ]
    );
    assert_eq!(
        receipt.events[1..3]
            .iter()
            .map(|event| event.payload["cell"].as_str().expect("token cell"))
            .collect::<Vec<_>>(),
        ["B3", "C3"]
    );
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    assert!(
        receipt.events[1..3]
            .iter()
            .all(|event| event.payload["sourceInstanceId"] == source_id)
    );
    let after = state(&session);
    let tokens = token_units_at(&after, &["B3", "C3"]);
    assert_eq!(tokens.len(), 2);
    for ((ordinal, cell), token) in ["B3", "C3"].into_iter().enumerate().zip(tokens) {
        let expected_id = identity_hash(&json!({
            "cardId": "foot-soldier-token",
            "cell": cell,
            "ordinal": ordinal,
            "owner": "north",
            "source": "token",
            "sourceInstanceId": source_id,
            "stateVersion": pre_cast_version,
        }))
        .expect("expected token identity");
        assert_eq!(token["instanceId"], json!(expected_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0580_border_militia_without_bordering_sites_is_a_paid_noop() {
    let encoded = seed_with(&["north-militia"]);
    let mut session = opening_main(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-militia"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

fn militia_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("militia actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-militia"
        })
        .count()
}

fn deathrite_border_militia_manifest(seed: u32) -> String {
    let fixture = "border-militia-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "foot-soldier-token": foot_soldier_token(),
            "north-avatar": avatar(),
            "north-militia": border_militia(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-militia",
                    "north-rain",
                    "north-rain",
                    "north-militia",
                    "north-rain",
                    "north-militia",
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

fn north_has_militia_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-militia", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteMilitiaSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_bordering_sites(
    encoded: &str,
) -> Option<PendingDeathriteMilitiaSetup> {
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_militia_and_rain(&state(&session)) {
        return None;
    }
    if militia_casts(&session) == 0 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteMilitiaSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_border_militia_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_border_militia_manifest)
        .find(|candidate| try_pending_deathrite_with_bordering_sites(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Border Militia in hand")
}

#[test]
fn rule_catalog_1079_border_militia_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_border_militia_seed_with(1079);
    let mut setup = try_pending_deathrite_with_bordering_sites(&encoded)
        .expect("complete Border Militia Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert_eq!(paused["realm"]["sites"]["C3"]["controller"], "north");
    assert_eq!(paused["realm"]["sites"]["C2"]["controller"], "south");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert_eq!(militia_casts(session), 0);

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(resumed["realm"]["sites"]["C3"]["controller"], "north");
    assert_eq!(resumed["realm"]["sites"]["C2"]["controller"], "south");
    assert!(militia_casts(session) >= 1);

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-militia"
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-summoned", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["cell"], "C3");
    assert_eq!(receipt.events[1].payload["cardId"], "foot-soldier-token");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(token_units_at(&state(session), &["C3"]).len(), 1);
    assert_exact_replay(session);
}

fn border_militia_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "border-militia-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-border-militia-supplemental-v1",
        },
        "cards": {
            "foot-soldier-token": foot_soldier_token(),
            "north-avatar": avatar(),
            "north-militia": border_militia(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                ],
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

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(579..579 + 2048)
        .map(border_militia_supplemental_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn militia_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-militia")
                .count()
        })
        .unwrap_or_default()
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

fn cast_militia(session: &mut Session) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-militia"
    });
    receipt
}

fn token_cells(snapshot: &Value) -> Vec<String> {
    snapshot["realm"]["units"]
        .as_array()
        .map(|units| {
            units
                .iter()
                .filter(|unit| unit["cardId"] == "foot-soldier-token")
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

fn seed_with_two_militia_spells_in_hand(start: u32) -> String {
    (start..start + 2048)
        .chain(579..579 + 2048)
        .map(border_militia_supplemental_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .filter(|id| *id == "north-militia")
                .count()
                >= 2
        })
        .expect("bounded seed with two Border Militia spells in opening hand")
}

struct SecondMilitiaSummonSetup {
    new_cell: String,
    session: Session,
}

fn try_second_militia_summon_prefix(encoded: &str) -> Option<SecondMilitiaSummonSetup> {
    let mut session = opening_main(encoded);
    setup_bordering_sites(&mut session);
    let first = cast_militia(&mut session);
    if !event_types(&first).contains(&"minion-summoned") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if militia_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D2"
    })?;
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
    })?;
    (militia_casts(&session) >= 1).then_some(SecondMilitiaSummonSetup {
        new_cell: "D3".to_owned(),
        session,
    })
}

fn seed_for_second_militia_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(579..579 + 8192)
        .find_map(|seed| {
            let encoded = border_militia_supplemental_manifest(seed);
            if !opening_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-militia")
            {
                return None;
            }
            try_second_militia_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Border Militia summon setup")
}

fn try_second_militia_enemy_arrival_prefix(encoded: &str) -> Option<Session> {
    let mut session = opening_main(encoded);
    setup_bordering_sites(&mut session);
    cast_militia(&mut session);
    pass_turn_to_north_spellbook(&mut session);
    if militia_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
    })?;
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (militia_casts(&session) >= 1).then_some(session)
}

fn seed_for_second_militia_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(579..579 + 8192)
        .find_map(|seed| {
            let encoded = border_militia_supplemental_manifest(seed);
            if !opening_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-militia")
            {
                return None;
            }
            try_second_militia_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Border Militia enemy-arrival setup")
}

#[test]
fn rule_catalog_1873_summoned_tokens_stay_on_board_after_turns_pass() {
    let encoded = seed_with_start(1873, &["north-militia"]);
    let mut session = opening_main(&encoded);
    setup_bordering_sites(&mut session);
    let receipt = cast_militia(&mut session);
    let token_ids: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-summoned")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("token identity")
                .to_owned()
        })
        .collect();
    assert_eq!(token_ids.len(), 2);
    pass_turn_to_north_spellbook(&mut session);
    let after = state(&session);
    for token_id in &token_ids {
        assert!(
            after["realm"]["units"]
                .as_array()
                .expect("realm units")
                .iter()
                .any(|unit| unit["instanceId"] == *token_id)
        );
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1874_second_border_militia_without_bordering_sites_is_a_paid_noop() {
    let encoded = seed_with_two_militia_spells_in_hand(1874);
    let mut session = opening_main(&encoded);
    let first = cast_militia(&mut session);
    assert_eq!(event_types(&first), ["magic-cast", "magic-resolved"]);
    let second = cast_militia(&mut session);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(
        !second
            .events
            .iter()
            .any(|event| event.event_type == "minion-summoned")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1875_second_border_militia_summons_on_a_newly_bordering_site_after_enemy_arrival() {
    let encoded = seed_for_second_militia_enemy_arrival(1875);
    let mut session = try_second_militia_enemy_arrival_prefix(&encoded)
        .expect("second Border Militia enemy-arrival prefix");
    let before_cells = token_cells(&state(&session));
    let receipt = cast_militia(&mut session);
    let summoned_cells: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-summoned")
        .map(|event| {
            event.payload["cell"]
                .as_str()
                .expect("summoned cell")
                .to_owned()
        })
        .collect();
    assert!(
        !summoned_cells.is_empty(),
        "expected at least one new bordering token"
    );
    assert!(
        summoned_cells
            .iter()
            .any(|cell| !before_cells.contains(cell)),
        "expected at least one newly summoned bordering cell"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1876_border_militia_summons_on_every_controlled_site_bordering_an_enemy_site() {
    let encoded = seed_with_start(1876, &["north-militia"]);
    let mut session = opening_main(&encoded);
    setup_bordering_sites(&mut session);
    let receipt = cast_militia(&mut session);
    let summoned_cells: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-summoned")
        .map(|event| {
            event.payload["cell"]
                .as_str()
                .expect("summoned cell")
                .to_owned()
        })
        .collect();
    assert_eq!(summoned_cells, vec!["B3", "C3"]);
    assert_eq!(token_units_at(&state(&session), &["B3", "C3"]).len(), 2);
    assert!(!token_cells(&state(&session)).contains(&"C1".to_owned()));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1877_border_militia_leaves_a_non_bordering_controlled_site_untouched() {
    let encoded = seed_with_start(1877, &["north-militia"]);
    let mut session = opening_main(&encoded);
    setup_bordering_sites(&mut session);
    cast_militia(&mut session);
    assert!(!token_cells(&state(&session)).contains(&"C1".to_owned()));
    assert_eq!(token_units_at(&state(&session), &["B3", "C3"]).len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1878_second_border_militia_summons_on_a_newly_placed_bordering_site() {
    let encoded = seed_for_second_militia_summon(1878);
    let SecondMilitiaSummonSetup {
        mut session,
        new_cell,
    } = try_second_militia_summon_prefix(&encoded).expect("second Border Militia summon prefix");
    let before = token_cells(&state(&session));
    let receipt = cast_militia(&mut session);
    let summoned_cells: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-summoned")
        .map(|event| {
            event.payload["cell"]
                .as_str()
                .expect("summoned cell")
                .to_owned()
        })
        .collect();
    assert!(
        summoned_cells.contains(&new_cell),
        "expected a token on the newly bordering site {new_cell}, got {summoned_cells:?}"
    );
    let after = token_cells(&state(&session));
    assert!(after.contains(&new_cell));
    assert!(after.len() > before.len());
    assert_exact_replay(&session);
}
