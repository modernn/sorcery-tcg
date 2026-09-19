//! Direct proofs for player-chosen additional Magic discard
//! (RULE-CATALOG-0653–0654, 0671–0672, RULE-CATALOG-1087,
//! RULE-CATALOG-2233–2238).
//!
//! A chosen-discard cost is a Storyline choice among every other Atlas or
//! Spellbook hand card. It cannot select the spell being cast, pays
//! `card-discarded` into the owner's cemetery before the cast is announced,
//! then the companion effect resolves. An empty other-hand issues no cast.
//! An Atlas leftover pays the same cost as a spell; every issued cast carries
//! the chosen identity. While Deathrites wait for ordering, chosen-discard
//! Magic stays withheld until the chain drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn fodder() -> Value {
    json!({
        "cardType": "magic",
        "healController": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn cost_spell() -> Value {
    json!({
        "cardType": "magic",
        "discardCardAsAdditionalCost": true,
        "drawSites": 1,
        "manaCost": 0,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn chosen_discard_manifest(seed: u32, north_spellbook: &[&str]) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chosen-discard-cost" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chosen-discard-cost-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cost": cost_spell(),
            "north-fodder": fodder(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
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
    let mut session = Session::new(encoded).expect("valid chosen-discard session");
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

fn north_hand_ids(snapshot: &Value, zone: &str) -> Vec<String> {
    snapshot["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("hand identity")
                .to_owned()
        })
        .collect()
}

fn north_spell_card_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn seed_with(north_spellbook: &[&str], required: &[&str], start: u32) -> String {
    (start..start + 256)
        .map(|seed| chosen_discard_manifest(seed, north_spellbook))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                let hand = north_spell_card_ids(&state(&preview));
                required.iter().all(|id| hand.iter().any(|card| card == id))
            })
        })
        .expect("bounded seed with required opening cards")
}

fn discard_cost_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("chosen-discard actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-cost"
        })
        .filter_map(|action| {
            action.descriptor["discardCardInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn offers_cost(session: &Session) -> bool {
    !discard_cost_ids(session).is_empty()
}

fn cast_all_fodder(session: &mut Session) {
    while session
        .legal_actions()
        .expect("fodder actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fodder"
        })
    {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fodder"
        });
    }
}

fn hand_card(snapshot: &Value, zone: &str, card_id: &str) -> String {
    snapshot["players"]["north"]["hand"][zone]
        .as_array()
        .expect("hand zone")
        .iter()
        .find(|card| card["cardId"] == card_id)
        .expect("required hand card")["instanceId"]
        .as_str()
        .expect("identity")
        .to_owned()
}

#[test]
fn rule_catalog_0653_chosen_discard_cost_discards_another_spell_then_resolves() {
    let encoded = seed_with(
        &["north-cost", "north-fodder", "north-fodder"],
        &["north-cost", "north-fodder"],
        653,
    );
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let cost_id = hand_card(&before, "spellbook", "north-cost");
    let fodder_id = hand_card(&before, "spellbook", "north-fodder");
    let atlas_ids = north_hand_ids(&before, "atlas");
    let drawn_id = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas library")
        .first()
        .expect("next site")["instanceId"]
        .clone();
    let discard_ids = discard_cost_ids(&session);
    assert!(!discard_ids.is_empty());
    assert!(discard_ids.iter().all(|id| id != &cost_id));
    assert!(discard_ids.iter().any(|id| id == &fodder_id));
    assert!(atlas_ids.iter().all(|id| discard_ids.contains(id)));
    let south_observation = session.observe(Seat::South);
    let atlas_before = atlas_ids.len();
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cost"
            && descriptor["discardCardInstanceId"] == fodder_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(receipt.events[0].payload["cardId"], "north-fodder");
    assert_eq!(receipt.events[0].payload["instanceId"], fodder_id);
    assert_eq!(receipt.events[0].payload["owner"], "north");
    assert_eq!(receipt.events[0].payload["seat"], "north");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], cost_id);
    assert_eq!(receipt.events[0].payload["zone"], "spellbook");
    assert_eq!(descriptor["discardCardInstanceId"], fodder_id);
    assert_eq!(
        receipt.events[1].payload["discardCardInstanceId"],
        fodder_id
    );
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], cost_id);
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == fodder_id)
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .iter()
            .any(|card| card["instanceId"] == drawn_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        atlas_before + 1
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .iter()
            .all(|card| card["instanceId"] != fodder_id && card["instanceId"] != cost_id)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_view["players"]["north"]["hand"]["atlas"],
        atlas_before + 1
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("chosen-discard checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized chosen-discard");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed chosen-discard");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed chosen-discard session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0654_chosen_discard_cost_is_unoffered_without_another_hand_card() {
    let encoded = seed_with(
        &[
            "north-cost",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
        ],
        &["north-cost"],
        654,
    );
    let mut session = opening_main(&encoded);
    assert!(offers_cost(&session), "Atlas leftovers still pay the cost");
    cast_all_fodder(&mut session);
    assert!(offers_cost(&session));
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        if state(&session)["players"]["south"]["domainEstablished"].as_bool() != Some(true) {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
        cast_all_fodder(&mut session);
    }
    let after = state(&session);
    assert_eq!(north_hand_ids(&after, "atlas").len(), 0);
    assert_eq!(north_spell_card_ids(&after), ["north-cost".to_owned()]);
    assert!(
        !offers_cost(&session),
        "an empty other-hand must issue no chosen-discard cast"
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0671_chosen_discard_cost_may_discard_an_atlas_card() {
    let encoded = seed_with(
        &["north-cost", "north-fodder", "north-fodder"],
        &["north-cost", "north-fodder"],
        671,
    );
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let cost_id = hand_card(&before, "spellbook", "north-cost");
    let site_id = north_hand_ids(&before, "atlas")
        .into_iter()
        .next()
        .expect("Atlas card");
    let discard_ids = discard_cost_ids(&session);
    assert!(discard_ids.iter().all(|id| id != &cost_id));
    assert!(discard_ids.iter().any(|id| id == &site_id));
    let atlas_before = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cost"
            && descriptor["discardCardInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(receipt.events[0].payload["cardId"], "north-site");
    assert_eq!(receipt.events[0].payload["instanceId"], site_id);
    assert_eq!(receipt.events[0].payload["owner"], "north");
    assert_eq!(receipt.events[0].payload["seat"], "north");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], cost_id);
    assert_eq!(receipt.events[0].payload["zone"], "atlas");
    assert_eq!(descriptor["discardCardInstanceId"], site_id);
    assert_eq!(receipt.events[1].payload["discardCardInstanceId"], site_id);
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == site_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        atlas_before
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("atlas-discard checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized atlas-discard");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed atlas-discard");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed atlas-discard session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0672_chosen_discard_cost_issues_no_choice_free_cast_while_atlas_remains() {
    let encoded = seed_with(
        &["north-cost", "north-fodder", "north-fodder"],
        &["north-cost"],
        672,
    );
    let mut session = opening_main(&encoded);
    cast_all_fodder(&mut session);
    let after = state(&session);
    let cost_id = hand_card(&after, "spellbook", "north-cost");
    let atlas_ids = north_hand_ids(&after, "atlas");
    assert!(!atlas_ids.is_empty(), "Atlas leftovers still pay the cost");
    assert_eq!(north_spell_card_ids(&after), ["north-cost".to_owned()]);
    let cost_casts: Vec<_> = session
        .legal_actions()
        .expect("chosen-discard actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-cost"
        })
        .collect();
    assert!(!cost_casts.is_empty());
    assert!(
        cost_casts
            .iter()
            .all(|action| action.descriptor.get("discardCardInstanceId").is_some()),
        "there is no no-choice cast while another hand card remains"
    );
    let discard_ids = discard_cost_ids(&session);
    assert!(discard_ids.iter().all(|id| id != &cost_id));
    assert!(atlas_ids.iter().all(|id| discard_ids.contains(id)));
    assert!(discard_ids.iter().all(|id| atlas_ids.contains(id)));
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

fn deathrite_chosen_discard_manifest(seed: u32) -> String {
    let fixture = "chosen-discard-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cost": cost_spell(),
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
                    "north-cost",
                    "north-rain",
                    "north-rain",
                    "north-cost",
                    "north-rain",
                    "north-cost",
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

fn north_has_cost_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-cost", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteChosenDiscardSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_chosen_discard_magic(
    encoded: &str,
) -> Option<PendingDeathriteChosenDiscardSetup> {
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
    if !north_has_cost_and_rain(&state(&session)) {
        return None;
    }
    if !offers_cost(&session) {
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
    Some(PendingDeathriteChosenDiscardSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_chosen_discard_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_chosen_discard_manifest)
        .find(|candidate| try_pending_deathrite_with_chosen_discard_magic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with chosen-discard Magic in hand")
}

#[test]
fn rule_catalog_1087_chosen_discard_cost_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_chosen_discard_seed_with(1087);
    let mut setup = try_pending_deathrite_with_chosen_discard_magic(&encoded)
        .expect("complete chosen-discard Deathrite withheld setup");
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
    assert!(!offers_cost(session));

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
    assert!(offers_cost(session));

    let discard_id = discard_cost_ids(session)
        .into_iter()
        .next()
        .expect("chosen-discard cost offered after Deathrites drain");
    let (descriptor, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cost"
            && descriptor["discardCardInstanceId"] == discard_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(descriptor["discardCardInstanceId"], discard_id);
    assert_eq!(receipt.events[0].payload["instanceId"], discard_id);
    assert_eq!(
        receipt.events[1].payload["discardCardInstanceId"],
        discard_id
    );
    assert_exact_replay(session);
}

fn chosen_discard_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chosen-discard-cost-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chosen-discard-cost-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cost": cost_spell(),
            "north-fodder": fodder(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-cost",
                    "north-fodder",
                    "north-cost",
                    "north-fodder",
                    "north-cost",
                    "north-fodder",
                    "north-cost",
                    "north-fodder",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
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
    north_spell_card_ids(&state(&preview))
}

fn supplemental_seed_with_start(start: u32, min_cost: usize, min_fodder: usize) -> String {
    (start..start + 2048)
        .chain(653..653 + 2048)
        .map(chosen_discard_supplemental_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().filter(|card| *card == "north-cost").count() >= min_cost
                && hand.iter().filter(|card| *card == "north-fodder").count() >= min_fodder
        })
        .expect("bounded seed with chosen-discard supplemental opening cards")
}

fn cost_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-cost")
                .count()
        })
        .unwrap_or_default()
}

fn cemetery_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn seat_hand_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    ["atlas", "spellbook"]
        .iter()
        .flat_map(|zone| {
            snapshot["players"][seat]["hand"][zone]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
        })
        .collect()
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
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
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

fn cast_cost_discarding(session: &mut Session, discard_id: &str) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cost"
            && descriptor["discardCardInstanceId"] == discard_id
    })
    .1
}

fn first_offered_discard(session: &Session) -> String {
    discard_cost_ids(session)
        .into_iter()
        .next()
        .expect("chosen-discard cost offered")
}

fn chosen_discard_empty_hand_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chosen-discard-cost-empty-hand" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chosen-discard-cost-empty-hand-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cost": cost_spell(),
            "north-fodder": fodder(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-cost",
                    "north-fodder",
                    "north-fodder",
                    "north-fodder",
                    "north-fodder",
                    "north-fodder",
                    "north-fodder",
                    "north-fodder",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
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

fn try_empty_other_hand_unoffered(encoded: &str) -> Option<Session> {
    let mut session = opening_main(encoded);
    if cost_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    if !offers_cost(&session) {
        return None;
    }
    cast_all_fodder(&mut session);
    for _ in 0..4 {
        let snapshot = state(&session);
        if north_hand_ids(&snapshot, "atlas").is_empty()
            && north_spell_card_ids(&snapshot) == ["north-cost".to_owned()]
        {
            return (!offers_cost(&session)).then_some(session);
        }
        end_turn_if_offered(&mut session);
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw"
                && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
        })?;
        if state(&session)["players"]["south"]["domainEstablished"].as_bool() != Some(true) {
            let _ = try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
        }
        end_turn_if_offered(&mut session);
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })?;
        let _ = try_accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
        cast_all_fodder(&mut session);
    }
    let snapshot = state(&session);
    (north_hand_ids(&snapshot, "atlas").is_empty()
        && north_spell_card_ids(&snapshot) == ["north-cost".to_owned()]
        && !offers_cost(&session))
    .then_some(session)
}

fn seed_for_empty_other_hand_unoffered(start: u32) -> String {
    (start..start + 2048)
        .chain(653..653 + 2048)
        .find_map(|seed| {
            let encoded = chosen_discard_empty_hand_manifest(seed);
            try_empty_other_hand_unoffered(&encoded).map(|_| encoded)
        })
        .expect(
            "bounded seed reaching leftover chosen-discard unoffered after emptying the other-hand",
        )
}

fn try_second_cost_after_enemy_site(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    if cost_spells_in_hand(&state(&session)) < 2 {
        return None;
    }
    let first = first_offered_discard(&session);
    let receipt = cast_cost_discarding(&mut session, &first);
    if !event_types(&receipt).contains(&"card-discarded") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if state(&session)["realm"]["sites"]["C1"].is_null() {
        return None;
    }
    if cost_spells_in_hand(&state(&session)) < 1 || !offers_cost(&session) {
        return None;
    }
    let discard_id = first_offered_discard(&session);
    Some((session, discard_id))
}

fn seed_for_second_cost_after_enemy_site(start: u32) -> String {
    (start..start + 2048)
        .chain(653..653 + 2048)
        .find_map(|seed| {
            let encoded = chosen_discard_supplemental_manifest(seed);
            try_second_cost_after_enemy_site(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second chosen-discard after enemy site placement")
}

fn try_second_cost_new_draw_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    if cost_spells_in_hand(&state(&session)) < 2 {
        return None;
    }
    let before = seat_hand_ids(&state(&session), "north");
    let first = first_offered_discard(&session);
    let receipt = cast_cost_discarding(&mut session, &first);
    if !event_types(&receipt).contains(&"card-discarded") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if cost_spells_in_hand(&state(&session)) < 1 || !offers_cost(&session) {
        return None;
    }
    let new_id = seat_hand_ids(&state(&session), "north")
        .into_iter()
        .find(|id| !before.contains(id) && discard_cost_ids(&session).contains(id))?;
    Some((session, new_id))
}

fn seed_for_second_cost_new_draw(start: u32) -> String {
    (start..start + 2048)
        .chain(653..653 + 2048)
        .find_map(|seed| {
            let encoded = chosen_discard_supplemental_manifest(seed);
            try_second_cost_new_draw_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second chosen-discard of a newly drawn card")
}

#[test]
fn rule_catalog_2233_discarded_card_stays_in_the_cemetery_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2233, 1, 1);
    let mut session = opening_main(&encoded);
    let fodder = hand_card(&state(&session), "spellbook", "north-fodder");
    let receipt = cast_cost_discarding(&mut session, &fodder);
    assert!(event_types(&receipt).contains(&"card-discarded"));
    assert!(cemetery_ids(&state(&session), "north").contains(&fodder));
    pass_turn_to_north_spellbook(&mut session);
    assert!(cemetery_ids(&state(&session), "north").contains(&fodder));
    assert!(!seat_hand_ids(&state(&session), "north").contains(&fodder));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2234_leftover_cost_is_unoffered_after_emptying_the_other_hand() {
    let encoded = seed_for_empty_other_hand_unoffered(2234);
    let session = try_empty_other_hand_unoffered(&encoded)
        .expect("leftover chosen-discard unoffered after emptying the other-hand");
    let after = state(&session);
    assert_eq!(north_hand_ids(&after, "atlas").len(), 0);
    assert_eq!(north_spell_card_ids(&after), ["north-cost".to_owned()]);
    assert!(!offers_cost(&session));
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2235_second_cost_still_discards_after_enemy_site_placement() {
    let encoded = seed_for_second_cost_after_enemy_site(2235);
    let (mut session, discard_id) = try_second_cost_after_enemy_site(&encoded)
        .expect("second chosen-discard enemy-arrival prefix");
    let receipt = cast_cost_discarding(&mut session, &discard_id);
    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "magic-cast",
            "site-drawn",
            "magic-resolved"
        ]
    );
    assert!(cemetery_ids(&state(&session), "north").contains(&discard_id));
    assert!(!seat_hand_ids(&state(&session), "north").contains(&discard_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2236_chosen_discard_cost_offers_every_other_hand_card() {
    let encoded = supplemental_seed_with_start(2236, 1, 1);
    let session = opening_main(&encoded);
    let before = state(&session);
    let cost_ids: Vec<_> = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .iter()
        .filter(|card| card["cardId"] == "north-cost")
        .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
        .collect();
    assert!(!cost_ids.is_empty());
    let other_ids: Vec<_> = seat_hand_ids(&before, "north")
        .into_iter()
        .filter(|id| !cost_ids.contains(id))
        .collect();
    assert!(!other_ids.is_empty());
    let discard_ids = discard_cost_ids(&session);
    for id in &other_ids {
        assert!(discard_ids.contains(id));
    }
    if cost_ids.len() == 1 {
        assert!(!discard_ids.contains(&cost_ids[0]));
    } else {
        for id in &cost_ids {
            assert!(discard_ids.contains(id));
        }
    }
    assert!(discard_ids
        .iter()
        .all(|id| seat_hand_ids(&before, "north").contains(id)));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2237_chosen_discard_cost_leaves_the_opponent_hand_untouched() {
    let encoded = supplemental_seed_with_start(2237, 1, 1);
    let mut session = opening_main(&encoded);
    let south_before = seat_hand_ids(&state(&session), "south");
    assert!(!south_before.is_empty());
    let discard_ids = discard_cost_ids(&session);
    assert!(south_before.iter().all(|id| !discard_ids.contains(id)));
    let fodder = hand_card(&state(&session), "spellbook", "north-fodder");
    let receipt = cast_cost_discarding(&mut session, &fodder);
    assert!(event_types(&receipt).contains(&"card-discarded"));
    assert_eq!(seat_hand_ids(&state(&session), "south"), south_before);
    assert!(!cemetery_ids(&state(&session), "south").contains(&fodder));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2238_second_cost_discards_a_newly_drawn_card() {
    let encoded = seed_for_second_cost_new_draw(2238);
    let (mut session, new_id) =
        try_second_cost_new_draw_prefix(&encoded).expect("second chosen-discard new-draw prefix");
    let receipt = cast_cost_discarding(&mut session, &new_id);
    assert!(event_types(&receipt).contains(&"card-discarded"));
    assert!(cemetery_ids(&state(&session), "north").contains(&new_id));
    assert!(!seat_hand_ids(&state(&session), "north").contains(&new_id));
    assert_exact_replay(&session);
}
