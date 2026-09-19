//! Direct proofs for mill-site Magic (RULE-CATALOG-0629–0630,
//! RULE-CATALOG-1043, RULE-CATALOG-2113–2118).
//!
//! Mill-site Magic offers only both Avatars and puts top Atlas cards into the
//! owner's cemetery in deck order. An empty Atlas is a paid no-op: no draw,
//! no deck-out, and no discard events. While Deathrites wait for ordering,
//! mill-site Magic stays withheld until the chain drains.

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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn mill_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "millSites": 2,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
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

fn visitor() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
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

fn mill_sites_manifest(seed: u32, south_atlas: usize) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "mill-sites" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-mill-sites-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-mill": mill_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-mill"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; south_atlas],
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
    let mut session = Session::new(encoded).expect("valid mill-sites session");
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

fn mill_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-mill"
        })
        .count()
}

fn mill_player_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("mill actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-mill"
        })
        .filter_map(|action| {
            let target = action.descriptor.get("target")?;
            Some((
                target["kind"].as_str()?.to_owned(),
                target["seat"].as_str()?.to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn mill_south_atlas(session: &Session) -> Vec<Value> {
    state(session)["players"]["south"]["atlas"]
        .as_array()
        .expect("south atlas")
        .clone()
}

fn deathrite_mill_manifest(seed: u32) -> String {
    let fixture = "mill-sites-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-mill": mill_spell(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-mill",
                    "north-rain",
                    "north-rain",
                    "north-mill",
                    "north-rain",
                    "north-mill",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 10],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-visitor", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_mill_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-mill", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteMillSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_mill_ready(encoded: &str) -> Option<PendingDeathriteMillSetup> {
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
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
    if !north_has_mill_and_rain(&state(&session)) {
        return None;
    }
    if mill_player_targets(&session).is_empty() {
        return None;
    }
    let south_atlas = state(&session)["players"]["south"]["atlas"]
        .as_array()
        .map_or(0, std::vec::Vec::len);
    if south_atlas < 4 {
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
    Some(PendingDeathriteMillSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_mill_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_mill_manifest)
        .find(|candidate| try_pending_deathrite_with_mill_ready(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with mill Magic in hand")
}

#[test]
fn rule_catalog_0629_mill_sites_puts_opponent_atlas_cards_in_the_cemetery() {
    let encoded = mill_sites_manifest(629, 6);
    let mut session = opening_main(&encoded);
    let before = mill_south_atlas(&session);
    let expected: Vec<_> = before.iter().take(2).cloned().collect();
    assert_eq!(expected.len(), 2);
    assert_eq!(
        mill_player_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mill"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "site-discarded",
            "site-discarded",
            "magic-resolved"
        ]
    );
    let discarded: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "site-discarded")
        .collect();
    assert_eq!(
        discarded[0].payload["instanceId"],
        expected[0]["instanceId"]
    );
    assert_eq!(discarded[0].payload["cardId"], "south-site");
    assert_eq!(
        discarded[0].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert_eq!(
        discarded[1].payload["instanceId"],
        expected[1]["instanceId"]
    );

    let after = state(&session);
    assert_eq!(
        after["players"]["south"]["atlas"]
            .as_array()
            .expect("remaining")
            .len(),
        1
    );
    let cemetery = after["players"]["south"]["cemetery"]
        .as_array()
        .expect("south cemetery");
    for card in &expected {
        assert!(
            cemetery
                .iter()
                .any(|entry| entry["instanceId"] == card["instanceId"])
        );
    }
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0630_mill_sites_is_a_paid_noop_on_an_empty_atlas() {
    let encoded = mill_sites_manifest(630, 3);
    let mut session = opening_main(&encoded);
    assert_eq!(mill_south_atlas(&session).len(), 0);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mill"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-discarded" || event.event_type == "game-ended")
    );
    let after = state(&session);
    assert_eq!(after["players"]["south"]["atlas"], json!([]));
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1043_mill_sites_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_mill_seed_with(1043);
    let mut setup = try_pending_deathrite_with_mill_ready(&encoded)
        .expect("complete mill-sites Deathrite withheld setup");
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
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(mill_player_targets(session).is_empty());
    assert_eq!(mill_casts(session), 0);

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
    let before = mill_south_atlas(session);
    let expected: Vec<_> = before.iter().take(2).cloned().collect();
    assert_eq!(expected.len(), 2);
    assert_eq!(
        mill_player_targets(session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mill"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "site-discarded",
            "site-discarded",
            "magic-resolved"
        ]
    );
    assert_eq!(
        state(session)["players"]["south"]["atlas"]
            .as_array()
            .expect("remaining")
            .len(),
        before.len() - 2
    );
    assert_exact_replay(session);
}

fn mill_sites_supplemental_manifest(seed: u32, south_atlas: usize) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "mill-sites-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-mill-sites-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-mill": mill_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-mill"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; south_atlas],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
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

fn supplemental_seed_with_start(start: u32, south_atlas: usize) -> String {
    (start..start + 2048)
        .chain(629..629 + 2048)
        .map(|seed| mill_sites_supplemental_manifest(seed, south_atlas))
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-mill")
        })
        .expect("bounded seed with mill-sites Magic in the opening hand")
}

fn mill_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-mill")
                .count()
        })
        .unwrap_or_default()
}

fn mill_atlas(session: &Session, seat: &str) -> Vec<Value> {
    state(session)["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .clone()
}

fn cemetery_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .filter_map(|card| card["instanceId"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn site_instance_at(snapshot: &Value, cell: &str) -> String {
    snapshot["realm"]["sites"][cell]["instanceId"]
        .as_str()
        .expect("site at cell")
        .to_owned()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .is_ok_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
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

fn cast_mill_on(session: &mut Session, seat: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mill"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == seat
    });
    receipt
}

fn discarded_site_ids(receipt: &Receipt) -> Vec<String> {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == "site-discarded")
        .filter_map(|event| event.payload["instanceId"].as_str().map(str::to_owned))
        .collect()
}

fn try_second_mill_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first = cast_mill_on(&mut session, "south");
    if discarded_site_ids(&first).len() != 2 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    let south_c1 = site_instance_at(&state(&session), "C1");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (mill_spells_in_hand(&state(&session)) >= 1 && mill_atlas(&session, "south").len() >= 2)
        .then_some((session, south_c1))
}

fn seed_for_second_mill_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(629..629 + 8192)
        .find_map(|seed| {
            let encoded = mill_sites_supplemental_manifest(seed, 24);
            opening_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-mill")
                .then_some(encoded)
                .and_then(|encoded| try_second_mill_enemy_arrival_prefix(&encoded).map(|_| encoded))
        })
        .expect("bounded seed reaching second mill-sites enemy-arrival setup")
}

#[test]
fn rule_catalog_2113_milled_sites_stay_in_the_cemetery_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2113, 24);
    let mut session = opening_main(&encoded);
    let before = mill_atlas(&session, "south");
    let expected: Vec<_> = before.iter().take(2).cloned().collect();
    assert_eq!(expected.len(), 2);
    let first = cast_mill_on(&mut session, "south");
    assert_eq!(discarded_site_ids(&first).len(), 2);
    let milled: Vec<_> = expected
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("milled identity")
                .to_owned()
        })
        .collect();
    for instance_id in &milled {
        assert!(cemetery_ids(&state(&session), "south").contains(instance_id));
    }
    pass_turn_to_north_spellbook(&mut session);
    for instance_id in &milled {
        assert!(cemetery_ids(&state(&session), "south").contains(instance_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2114_second_mill_is_a_paid_noop_after_the_atlas_empties() {
    let encoded = (2114..2114 + 8192)
        .chain(629..629 + 8192)
        .find_map(|seed| {
            let candidate = mill_sites_supplemental_manifest(seed, 5);
            if !opening_spell_ids(&candidate)
                .iter()
                .any(|card| card == "north-mill")
            {
                return None;
            }
            let mut session = opening_main(&candidate);
            if mill_atlas(&session, "south").len() != 2 {
                return None;
            }
            let first = cast_mill_on(&mut session, "south");
            if discarded_site_ids(&first).len() != 2 {
                return None;
            }
            if !mill_atlas(&session, "south").is_empty() {
                return None;
            }
            (mill_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two mill-sites casts after emptying the Atlas");
    let mut session = opening_main(&encoded);
    assert_eq!(mill_atlas(&session, "south").len(), 2);
    let first = cast_mill_on(&mut session, "south");
    assert_eq!(discarded_site_ids(&first).len(), 2);
    assert_eq!(mill_atlas(&session, "south"), Vec::<Value>::new());
    assert!(mill_spells_in_hand(&state(&session)) >= 1);
    assert_eq!(
        mill_player_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );
    let second = cast_mill_on(&mut session, "south");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(discarded_site_ids(&second).is_empty());
    assert_eq!(mill_atlas(&session, "south"), Vec::<Value>::new());
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2115_second_mill_discards_remaining_atlas_after_enemy_site_placement() {
    let encoded = seed_for_second_mill_enemy_arrival(2115);
    let (mut session, south_c1) = try_second_mill_enemy_arrival_prefix(&encoded)
        .expect("second mill-sites enemy-arrival prefix");
    let remaining = mill_atlas(&session, "south");
    assert!(remaining.len() >= 2);
    let receipt = cast_mill_on(&mut session, "south");
    assert_eq!(discarded_site_ids(&receipt).len(), 2);
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["instanceId"],
        south_c1
    );
    assert_ne!(
        state(&session)["realm"]["sites"]["C1"]["rubble"],
        json!(true)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2116_mill_sites_offers_both_avatars() {
    let encoded = supplemental_seed_with_start(2116, 24);
    let session = opening_main(&encoded);
    assert_eq!(
        mill_player_targets(&session),
        [
            ("avatar".to_owned(), "north".to_owned()),
            ("avatar".to_owned(), "south".to_owned())
        ]
    );
    assert!(mill_atlas(&session, "south").len() >= 2);
    assert!(mill_atlas(&session, "north").len() >= 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2117_mill_sites_leaves_the_other_atlas_untouched() {
    let encoded = supplemental_seed_with_start(2117, 24);
    let mut session = opening_main(&encoded);
    let north_before = mill_atlas(&session, "north");
    let south_before = mill_atlas(&session, "south");
    assert!(south_before.len() >= 2);
    let receipt = cast_mill_on(&mut session, "south");
    assert_eq!(discarded_site_ids(&receipt).len(), 2);
    assert_eq!(mill_atlas(&session, "north"), north_before);
    assert_eq!(mill_atlas(&session, "south").len(), south_before.len() - 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2118_second_mill_discards_newly_remaining_atlas_cards() {
    let encoded = supplemental_seed_with_start(2118, 24);
    let mut session = opening_main(&encoded);
    let before = mill_atlas(&session, "south");
    assert!(before.len() >= 4);
    assert!(mill_spells_in_hand(&state(&session)) >= 2);
    let first_ids: Vec<_> = before
        .iter()
        .take(2)
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("first mill identity")
                .to_owned()
        })
        .collect();
    let second_ids: Vec<_> = before
        .iter()
        .skip(2)
        .take(2)
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("second mill identity")
                .to_owned()
        })
        .collect();
    let first = cast_mill_on(&mut session, "south");
    assert_eq!(discarded_site_ids(&first), first_ids);
    let second = cast_mill_on(&mut session, "south");
    assert_eq!(discarded_site_ids(&second), second_ids);
    let cemetery = cemetery_ids(&state(&session), "south");
    for instance_id in first_ids.iter().chain(second_ids.iter()) {
        assert!(cemetery.contains(instance_id));
    }
    assert_eq!(mill_atlas(&session, "south").len(), before.len() - 4);
    assert_exact_replay(&session);
}
