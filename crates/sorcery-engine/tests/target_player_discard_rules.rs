//! Direct proofs for target-player discard Magic (RULE-CATALOG-0643–0644,
//! RULE-CATALOG-0919, RULE-CATALOG-1050, RULE-CATALOG-1166,
//! RULE-CATALOG-2183–2188).
//!
//! Target-player discard offers only both Avatars. After the cast is
//! announced, the targeted player chooses one of their own Atlas or
//! Spellbook hand cards. An empty hand is a paid no-op, never a random
//! discard and never a deck-out. A count above one keeps the pending
//! Storyline open until each sequential choice is taken. While Deathrites
//! wait for ordering, target-player discard Magic stays withheld until
//! the chain drains. 1166 covers the Storyline itself: a discard cast that
//! settles Deathrites before any discard-card choice is issued withholds
//! discard-card until the order drains, then returns the pending choice.
//! Supplemental proofs cover persistence, empty-hand repeat, enemy-arrival,
//! both-Avatar targeting, the unselected player's hand, and a newly drawn card.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode, Seat};
use sorcery_engine::game::{Game, IssuedAction};
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

fn discard_spell(count: u8) -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "targetPlayerDiscardsCards": count,
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

fn power_bonus_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "otherNearbyAlliesPowerBonus": 1,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn discard_cards_manifest(seed: u32, count: u8, north_spells: usize) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "target-player-discard" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-target-player-discard-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-discard": discard_spell(count),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-discard"; north_spells],
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
    let mut session = Session::new(encoded).expect("valid target-player-discard session");
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

fn discard_card_ids(session: &Session) -> Vec<(String, String)> {
    discard_card_ids_from(
        session
            .legal_actions()
            .expect("legal actions")
            .into_iter()
            .map(|action| action.descriptor),
    )
}

fn issued_descriptor(action: &IssuedAction) -> Value {
    serde_json::to_value(action.descriptor()).expect("typed descriptor JSON")
}

fn game_discard_card_ids(game: &Game) -> Vec<(String, String)> {
    discard_card_ids_from(
        game.legal_actions()
            .expect("legal actions")
            .into_iter()
            .map(|action| issued_descriptor(&action)),
    )
}

fn discard_card_ids_from(descriptors: impl IntoIterator<Item = Value>) -> Vec<(String, String)> {
    descriptors
        .into_iter()
        .filter_map(|descriptor| {
            if descriptor["kind"] != "discard-card" {
                return None;
            }
            Some((
                descriptor["cardInstanceId"]
                    .as_str()
                    .expect("discard identity")
                    .to_owned(),
                descriptor["zone"]
                    .as_str()
                    .expect("discard zone")
                    .to_owned(),
            ))
        })
        .collect()
}

fn replay_game(session: &Session) -> Game {
    let mut game = Game::from_manifest_json(session.manifest_json()).expect("valid replay game");
    for receipt in session.transcript() {
        let action = game
            .legal_actions()
            .expect("replay legal actions")
            .into_iter()
            .find(|action| {
                action
                    .to_legal_action()
                    .is_ok_and(|action| action.action_id == receipt.action_id)
            })
            .expect("recorded engine-issued action");
        game.apply_action(&action).expect("replay action");
    }
    game
}

fn apply_where(game: &mut Game, predicate: impl Fn(&Value) -> bool) {
    let action = game
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&issued_descriptor(action)))
        .expect("expected engine-issued action");
    game.apply_action(&action).expect("authoritative Game step");
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn discard_targets(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("cast actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-discard"
        })
        .filter_map(|action| {
            action.descriptor["target"]["seat"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn north_hand_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn seed_with(count: u8, north_spells: usize, start: u32) -> String {
    (start..start + 256)
        .map(|seed| discard_cards_manifest(seed, count, north_spells))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                north_hand_ids(&state(&preview))
                    .iter()
                    .any(|card| card == "north-discard")
            })
        })
        .expect("bounded seed with required opening cards")
}

fn hand_ids(snapshot: &Value, seat: &str, zone: &str) -> Vec<Value> {
    snapshot["players"][seat]["hand"][zone]
        .as_array()
        .expect("hand zone")
        .iter()
        .map(|card| card["instanceId"].clone())
        .collect()
}

fn assert_pending_south_choice(
    session: &Session,
    spell_id: &Value,
    chosen: &Value,
    sites: &[Value],
) {
    let pending = state(session);
    assert_eq!(pending["phase"], "discard-card");
    assert_eq!(pending["decisionSeat"], "south");
    assert_eq!(pending["pendingDiscardCards"]["remaining"], 1);
    assert_eq!(pending["pendingDiscardCards"]["seat"], "south");
    assert_eq!(
        pending["pendingDiscardCards"]["sourceInstanceId"],
        *spell_id
    );
    let offered = discard_card_ids(session);
    assert_eq!(offered.len(), 6);
    assert!(
        offered
            .iter()
            .any(|(id, zone)| { id == chosen.as_str().expect("chosen") && zone == "spellbook" })
    );
    assert!(sites.iter().all(|id| {
        offered
            .iter()
            .any(|(offered_id, zone)| offered_id == id.as_str().expect("site") && zone == "atlas")
    }));
    assert!(
        session
            .legal_actions()
            .expect("pending discard actions")
            .iter()
            .all(|action| action.seat == Seat::South)
    );
    let north_view = session
        .public_view(Seat::North)
        .expect("North public view during the choice");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 3);
    assert_eq!(north_view["players"]["south"]["hand"]["atlas"], 3);
    assert!(
        !north_view
            .to_string()
            .contains(chosen.as_str().expect("chosen identity")),
        "the caster must not see the opponent's hidden hand identities"
    );
}

fn reject_forged_north_discard(session: &mut Session, chosen: &Value, state_version: &Value) {
    let forged = session
        .step(ActionRequest {
            action_id: identity_hash(&json!({
                "descriptor": {
                    "cardInstanceId": chosen,
                    "kind": "discard-card",
                    "zone": "spellbook",
                },
                "engineVersion": "sorcery-core-v1",
                "seat": "north",
                "stateVersion": state_version,
            }))
            .expect("forged action id")
            .to_string(),
            seat: Seat::North,
            state_version: state_version.as_u64().expect("state version"),
        })
        .expect("forged step");
    assert!(matches!(
        forged,
        StepResult::Rejected(rejection)
            if rejection.code == RejectionCode::UnknownAction
                || rejection.code == RejectionCode::WrongSeat
    ));
}

fn assert_stored_checkpoint(session: &Session) {
    let stored = create_game_checkpoint(session).expect("target-player-discard checkpoint");
    let serialized = serialize_game_checkpoint(&stored).expect("serialized discard");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed discard");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed target-player-discard session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

fn deathrite_discard_manifest(seed: u32) -> String {
    let fixture = "target-player-discard-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-discard": discard_spell(1),
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
                    "north-discard",
                    "north-rain",
                    "north-rain",
                    "north-discard",
                    "north-rain",
                    "north-discard",
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

fn north_has_discard_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-discard", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteDiscardSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_discard_magic(encoded: &str) -> Option<PendingDeathriteDiscardSetup> {
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
    if !north_has_discard_and_rain(&state(&session)) {
        return None;
    }
    if !discard_targets(&session).iter().any(|seat| seat == "south") {
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
    Some(PendingDeathriteDiscardSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_discard_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_discard_manifest)
        .find(|candidate| try_pending_deathrite_with_discard_magic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with discard Magic in hand")
}

#[test]
fn rule_catalog_0643_target_player_discard_lets_the_targeted_player_choose() {
    let encoded = seed_with(1, 6, 643);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"].clone();
    let south_spells = hand_ids(&before, "south", "spellbook");
    let south_sites = hand_ids(&before, "south", "atlas");
    assert_eq!(south_spells.len(), 3);
    assert_eq!(south_sites.len(), 3);
    let chosen = south_spells[0].clone();
    let targets = discard_targets(&session);
    assert!(targets.iter().any(|seat| seat == "north"));
    assert!(targets.iter().any(|seat| seat == "south"));
    let south_observation = session.observe(Seat::South);
    let (descriptor, cast) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    let spell_id = descriptor["cardInstanceId"].clone();
    assert_eq!(event_types(&cast), ["magic-cast"]);
    assert_pending_south_choice(&session, &spell_id, &chosen, &south_sites);
    assert_eq!(session.observe(Seat::South), south_observation);
    let checkpoint = session.clone();
    let state_version = state(&session)["stateVersion"].clone();
    reject_forged_north_discard(&mut session, &chosen, &state_version);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "discard-card"
            && descriptor["cardInstanceId"] == chosen
            && descriptor["zone"] == "spellbook"
    });
    assert_eq!(event_types(&receipt), ["card-discarded", "magic-resolved"]);
    assert_eq!(receipt.events[0].payload["cardId"], "south-minion");
    assert_eq!(receipt.events[0].payload["instanceId"], chosen);
    assert_eq!(receipt.events[0].payload["owner"], "south");
    assert_eq!(receipt.events[0].payload["seat"], "south");
    assert_eq!(receipt.events[0].payload["sourceInstanceId"], spell_id);
    assert_eq!(receipt.events[0].payload["zone"], "spellbook");
    assert_eq!(receipt.events[1].payload["instanceId"], spell_id);
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["decisionSeat"], "north");
    assert!(after.get("pendingDiscardCards").is_none());
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["instanceId"] == chosen)
    );
    assert_eq!(
        after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("south hand")
            .len(),
        2
    );
    let mut resumed = checkpoint;
    accept_where(&mut resumed, |descriptor| {
        descriptor["kind"] == "discard-card" && descriptor["cardInstanceId"] == chosen
    });
    assert_eq!(
        resumed.replay_value().expect("resumed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    assert_stored_checkpoint(&session);
}

#[test]
fn rule_catalog_0644_target_player_discard_is_a_paid_noop_without_cards() {
    let encoded = seed_with(6, 3, 644);
    let mut session = opening_main(&encoded);
    let south_avatar = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"].clone();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    let mut discarded = 0;
    while state(&session)["phase"] == "discard-card" {
        let (_, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "discard-card"
        });
        assert_eq!(receipt.events[0].event_type, "card-discarded");
        discarded += 1;
    }
    assert_eq!(discarded, 6);
    let emptied = state(&session);
    assert_eq!(emptied["phase"], "main");
    assert_eq!(
        emptied["players"]["south"]["hand"]["atlas"]
            .as_array()
            .expect("south Atlas")
            .len(),
        0
    );
    assert_eq!(
        emptied["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("south Spellbook")
            .len(),
        0
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "card-discarded" || event.event_type == "game-ended")
    );
    assert_eq!(state(&session)["phase"], "main");
    assert_eq!(state(&session)["decisionSeat"], "north");
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0919_multi_discard_storyline_defers_magic_resolved_until_each_choice() {
    let encoded = seed_with(2, 6, 919);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"].clone();
    let south_spells = hand_ids(&before, "south", "spellbook");
    let first = south_spells[0].clone();
    let second = south_spells[1].clone();
    let (_, cast) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    let spell_id = cast.events[0].payload["instanceId"].clone();
    assert_eq!(event_types(&cast), ["magic-cast"]);
    let pending = state(&session);
    assert_eq!(pending["phase"], "discard-card");
    assert_eq!(pending["decisionSeat"], "south");
    assert_eq!(pending["pendingDiscardCards"]["remaining"], 2);
    assert_eq!(pending["pendingDiscardCards"]["seat"], "south");
    assert_eq!(pending["pendingDiscardCards"]["sourceInstanceId"], spell_id);
    assert!(
        session
            .legal_actions()
            .expect("pending discard actions")
            .iter()
            .all(|action| action.seat == Seat::South)
    );
    assert!(
        session
            .legal_actions()
            .expect("no caster actions during the Storyline")
            .iter()
            .all(|action| action.seat == Seat::South)
    );
    let (_, first_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "discard-card"
            && descriptor["cardInstanceId"] == first
            && descriptor["zone"] == "spellbook"
    });
    assert_eq!(event_types(&first_receipt), ["card-discarded"]);
    assert_eq!(
        first_receipt.events[0].payload["sourceInstanceId"],
        spell_id
    );
    let mid = state(&session);
    assert_eq!(mid["phase"], "discard-card");
    assert_eq!(mid["decisionSeat"], "south");
    assert_eq!(mid["pendingDiscardCards"]["remaining"], 1);
    assert_eq!(mid["pendingDiscardCards"]["sourceInstanceId"], spell_id);
    let (_, second_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "discard-card"
            && descriptor["cardInstanceId"] == second
            && descriptor["zone"] == "spellbook"
    });
    assert_eq!(
        event_types(&second_receipt),
        ["card-discarded", "magic-resolved"]
    );
    assert_eq!(second_receipt.events[1].payload["instanceId"], spell_id);
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["decisionSeat"], "north");
    assert!(after.get("pendingDiscardCards").is_none());
    assert_eq!(
        after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("south hand")
            .len(),
        1
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1050_target_player_discard_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_discard_seed_with(1050);
    let mut setup = try_pending_deathrite_with_discard_magic(&encoded)
        .expect("complete target-player discard Deathrite withheld setup");
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
    assert!(discard_targets(session).is_empty());

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
    let south_avatar = resumed["players"]["south"]["avatar"]["card"]["instanceId"].clone();
    assert!(discard_targets(session).iter().any(|seat| seat == "south"));

    let before = state(session);
    let south_spells = hand_ids(&before, "south", "spellbook");
    let south_sites = hand_ids(&before, "south", "atlas");
    let (chosen, zone) = if let Some(card) = south_spells.first() {
        (card.clone(), "spellbook")
    } else {
        (south_sites[0].clone(), "atlas")
    };

    let (_, cast) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    assert_eq!(event_types(&cast), ["magic-cast"]);
    assert_eq!(state(session)["phase"], "discard-card");

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "discard-card"
            && descriptor["cardInstanceId"] == chosen
            && descriptor["zone"] == zone
    });
    assert_eq!(event_types(&receipt), ["card-discarded", "magic-resolved"]);
    assert_eq!(state(session)["phase"], "main");
    assert!(state(session).get("pendingDiscardCards").is_none());
    assert_exact_replay(session);
}

fn deathrite_interrupt_discard_manifest(seed: u32) -> String {
    let fixture = "target-player-discard-card-deathrite-interrupt";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-discard": discard_spell(2),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-aura": power_bonus_minion(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-discard",
                    "north-rain",
                    "north-rain",
                    "north-discard",
                    "north-rain",
                    "north-discard",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-minion",
                    "south-aura",
                    "south-minion",
                    "south-minion",
                    "south-aura",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct PendingDiscardDeathriteInterruptSetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_wounded_deathrites_with_discard_ready(
    encoded: &str,
) -> Option<PendingDiscardDeathriteInterruptSetup> {
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
    let aura = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-aura"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_discard_and_rain(&state(&session)) {
        return None;
    }
    if !discard_targets(&session).iter().any(|seat| seat == "south") {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    let snapshot = state(&session);
    if snapshot["phase"] != "main" {
        return None;
    }
    let aura_id = aura.0["cardInstanceId"].as_str()?.to_owned();
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    if deathrite_ids.iter().any(|instance_id| {
        realm_unit(&snapshot, instance_id).is_none_or(|unit| unit["damage"] != 1)
    }) || realm_unit(&snapshot, &aura_id).is_none_or(|unit| unit["damage"] != 1)
    {
        return None;
    }
    Some(PendingDiscardDeathriteInterruptSetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_interrupt_discard_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_interrupt_discard_manifest)
        .find(|candidate| try_wounded_deathrites_with_discard_ready(candidate).is_some())
        .expect(
            "bounded seed that wounds Deathrites under a power bonus with discard Magic in hand",
        )
}

#[test]
fn rule_catalog_1166_discard_card_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_interrupt_discard_seed_with(1166);
    let setup = try_wounded_deathrites_with_discard_ready(&encoded)
        .expect("complete target-player discard Deathrite interrupt setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let south_avatar =
        state(&setup.session)["players"]["south"]["avatar"]["card"]["instanceId"].clone();
    assert_exact_replay(&setup.session);

    let mut control = setup.session.clone();
    let (_, cast) = accept_where(&mut control, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    assert_eq!(event_types(&cast), ["magic-cast"]);
    let pending = state(&control);
    assert_eq!(pending["phase"], "discard-card");
    assert_eq!(pending["decisionSeat"], "south");
    assert_eq!(pending["pendingDiscardCards"]["remaining"], 2);
    assert_eq!(pending["pendingDiscardCards"]["seat"], "south");
    assert!(!discard_card_ids(&control).is_empty());
    assert_exact_replay(&control);

    let mut branched = replay_game(&setup.session);
    assert!(
        branched.test_remove_realm_unit(&aura_id),
        "checkpoint branch must drop the power-bonus ally so wounded Deathrites settle"
    );
    apply_where(&mut branched, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });

    let paused = branched.authoritative_state();
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["pendingDeathrites"]["returnPhase"], "discard-card");
    assert_eq!(paused["pendingDiscardCards"]["remaining"], 2);
    assert_eq!(paused["pendingDiscardCards"]["seat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        game_discard_card_ids(&branched).is_empty(),
        "deathrite-order must issue no discard-card while the Storyline stays pending"
    );

    let order_sources: Vec<_> = branched
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| issued_descriptor(action)["kind"] == "order-deathrites")
        .map(|action| {
            issued_descriptor(&action)["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);
    apply_where(&mut branched, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = branched.authoritative_state();
    assert_eq!(resumed["phase"], "discard-card");
    assert_eq!(resumed["decisionSeat"], "south");
    assert!(resumed["pendingDeathrites"].is_null());
    assert_eq!(resumed["pendingDiscardCards"]["remaining"], 2);
    let offered = game_discard_card_ids(&branched);
    assert!(
        !offered.is_empty(),
        "discard-card must return once deathrite-order clears"
    );
    apply_where(&mut branched, |descriptor| {
        descriptor["kind"] == "discard-card"
            && descriptor["cardInstanceId"] == offered[0].0
            && descriptor["zone"] == offered[0].1
    });
    let mid = branched.authoritative_state();
    assert_eq!(mid["phase"], "discard-card");
    assert_eq!(mid["pendingDiscardCards"]["remaining"], 1);
    apply_where(&mut branched, |descriptor| {
        descriptor["kind"] == "discard-card"
    });
    let after = branched.authoritative_state();
    assert_eq!(after["phase"], "main");
    assert_eq!(after["decisionSeat"], "north");
    assert!(after.get("pendingDiscardCards").is_none());
}

fn discard_supplemental_manifest(seed: u32, count: u8) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "target-player-discard-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-target-player-discard-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-discard": discard_spell(count),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-discard"; 8],
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

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
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

fn supplemental_seed_with_start(start: u32) -> String {
    (start..start + 2048)
        .chain(643..643 + 2048)
        .map(|seed| discard_supplemental_manifest(seed, 1))
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-discard")
        })
        .expect("bounded seed with target-player discard Magic in the opening hand")
}

fn discard_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-discard")
                .count()
        })
        .unwrap_or_default()
}

fn hand_len(snapshot: &Value, seat: &str) -> usize {
    ["atlas", "spellbook"]
        .iter()
        .map(|zone| {
            snapshot["players"][seat]["hand"][zone]
                .as_array()
                .map(Vec::len)
                .unwrap_or_default()
        })
        .sum()
}

fn north_hand_instance_ids(snapshot: &Value) -> Vec<String> {
    ["atlas", "spellbook"]
        .iter()
        .flat_map(|zone| {
            snapshot["players"]["north"]["hand"][zone]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
        })
        .collect()
}

fn south_hand_instance_ids(snapshot: &Value) -> Vec<String> {
    ["atlas", "spellbook"]
        .iter()
        .flat_map(|zone| {
            snapshot["players"]["south"]["hand"][zone]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
        })
        .collect()
}

fn cemetery_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
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

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn resolve_pending_discard(session: &mut Session) -> Option<Receipt> {
    if state(session)["phase"] != "discard-card" {
        return None;
    }
    Some(accept_where(session, |descriptor| descriptor["kind"] == "discard-card").1)
}

fn cast_discard_on_south(session: &mut Session) -> Receipt {
    let (_, cast) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["seat"] == "south"
    });
    resolve_pending_discard(session).unwrap_or(cast)
}

fn try_second_discard_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    if discard_spells_in_hand(&state(&session)) < 1 {
        north_draws_spellbook(&mut session);
    }
    if discard_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let first = cast_discard_on_south(&mut session);
    if !event_types(&first).contains(&"card-discarded") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if discard_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    let _ = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C3"
    });
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if discard_spells_in_hand(&state(&session)) < 1 || hand_len(&state(&session), "south") == 0 {
        return None;
    }
    let chosen = south_hand_instance_ids(&state(&session))
        .into_iter()
        .next()?;
    discard_targets(&session)
        .iter()
        .any(|seat| seat == "south")
        .then_some((session, chosen))
}

fn seed_for_second_discard_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(643..643 + 8192)
        .find_map(|seed| {
            let encoded = discard_supplemental_manifest(seed, 1);
            try_second_discard_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second target-player discard enemy-arrival setup")
}

fn try_second_discard_new_draw_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    if discard_spells_in_hand(&state(&session)) < 1 {
        north_draws_spellbook(&mut session);
    }
    if discard_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let before = south_hand_instance_ids(&state(&session));
    let first = cast_discard_on_south(&mut session);
    if !event_types(&first).contains(&"card-discarded") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if discard_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let _ = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if discard_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let new_id = south_hand_instance_ids(&state(&session))
        .into_iter()
        .find(|id| !before.contains(id))?;
    discard_targets(&session)
        .iter()
        .any(|seat| seat == "south")
        .then_some((session, new_id))
}

fn seed_for_second_discard_new_draw(start: u32) -> String {
    (start..start + 8192)
        .chain(643..643 + 8192)
        .find_map(|seed| {
            let encoded = discard_supplemental_manifest(seed, 1);
            try_second_discard_new_draw_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second target-player discard new-draw setup")
}

#[test]
fn rule_catalog_2183_discarded_card_stays_in_the_cemetery_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2183);
    let mut session = opening_main(&encoded);
    if discard_spells_in_hand(&state(&session)) < 1 {
        north_draws_spellbook(&mut session);
    }
    let first = cast_discard_on_south(&mut session);
    assert!(event_types(&first).contains(&"card-discarded"));
    let discarded = first
        .events
        .iter()
        .find(|event| event.event_type == "card-discarded")
        .expect("discard event")
        .payload["instanceId"]
        .as_str()
        .expect("discarded identity")
        .to_owned();
    assert!(cemetery_ids(&state(&session), "south").contains(&discarded));
    pass_turn_to_north_spellbook(&mut session);
    assert!(cemetery_ids(&state(&session), "south").contains(&discarded));
    assert!(!south_hand_instance_ids(&state(&session)).contains(&discarded));
    assert_exact_replay(&session);
}

fn empty_south_hand_with_discard(session: &mut Session) -> Option<Receipt> {
    let (_, cast) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["seat"] == "south"
    })?;
    let mut last = cast;
    while state(session)["phase"] == "discard-card" {
        last = resolve_pending_discard(session)?;
    }
    Some(last)
}

#[test]
fn rule_catalog_2184_second_discard_is_a_paid_noop_after_emptying_the_hand() {
    let encoded = (2184..2184 + 8192)
        .chain(643..643 + 8192)
        .find_map(|seed| {
            let candidate = discard_supplemental_manifest(seed, 6);
            if opening_spell_ids(&candidate, "north")
                .iter()
                .filter(|card| *card == "north-discard")
                .count()
                < 2
            {
                return None;
            }
            let mut session = opening_main(&candidate);
            if discard_spells_in_hand(&state(&session)) < 2
                || hand_len(&state(&session), "south") == 0
            {
                return None;
            }
            let first = empty_south_hand_with_discard(&mut session)?;
            if !event_types(&first).contains(&"card-discarded") {
                return None;
            }
            (hand_len(&state(&session), "south") == 0
                && discard_spells_in_hand(&state(&session)) >= 1)
                .then_some(candidate)
        })
        .expect("bounded seed with two target-player discard casts after emptying the hand");
    let mut session = opening_main(&encoded);
    assert!(discard_spells_in_hand(&state(&session)) >= 2);
    let first = empty_south_hand_with_discard(&mut session).expect("first empty-hand discard");
    assert!(event_types(&first).contains(&"card-discarded"));
    assert_eq!(hand_len(&state(&session), "south"), 0);
    assert!(discard_spells_in_hand(&state(&session)) >= 1);
    let second = cast_discard_on_south(&mut session);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"card-discarded"));
    assert_eq!(state(&session)["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2185_second_discard_takes_a_newly_arrived_card_after_enemy_site_placement() {
    let encoded = seed_for_second_discard_enemy_arrival(2185);
    let (mut session, chosen) = try_second_discard_enemy_arrival_prefix(&encoded)
        .expect("second target-player discard enemy-arrival prefix");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["seat"] == "south"
    });
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "discard-card" && descriptor["cardInstanceId"] == chosen
    });
    assert!(event_types(&receipt).contains(&"card-discarded"));
    assert!(cemetery_ids(&state(&session), "south").contains(&chosen));
    assert!(!south_hand_instance_ids(&state(&session)).contains(&chosen));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2186_target_player_discard_offers_both_avatars() {
    let encoded = supplemental_seed_with_start(2186);
    let session = opening_main(&encoded);
    let mut seats = discard_targets(&session);
    seats.sort();
    seats.dedup();
    assert_eq!(seats, ["north".to_owned(), "south".to_owned()]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2187_target_player_discard_leaves_the_other_player_hand_untouched() {
    let encoded = supplemental_seed_with_start(2187);
    let mut session = opening_main(&encoded);
    let north_before = north_hand_instance_ids(&state(&session));
    let south_before = south_hand_instance_ids(&state(&session));
    assert!(south_before.len() > 1);
    let (descriptor, cast) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["seat"] == "south"
    });
    let spent = descriptor["cardInstanceId"]
        .as_str()
        .expect("spent discard identity")
        .to_owned();
    let receipt = resolve_pending_discard(&mut session).unwrap_or(cast);
    assert!(event_types(&receipt).contains(&"card-discarded"));
    let discarded = receipt
        .events
        .iter()
        .find(|event| event.event_type == "card-discarded")
        .expect("discard event")
        .payload["instanceId"]
        .as_str()
        .expect("discarded identity")
        .to_owned();
    let north_after = north_hand_instance_ids(&state(&session));
    assert!(!north_after.contains(&spent));
    for id in north_before.iter().filter(|id| *id != &spent) {
        assert!(north_after.contains(id));
    }
    let south_after = south_hand_instance_ids(&state(&session));
    assert!(!south_after.contains(&discarded));
    for id in south_before.iter().filter(|id| *id != &discarded) {
        assert!(south_after.contains(id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2188_second_discard_takes_a_newly_drawn_card() {
    let encoded = seed_for_second_discard_new_draw(2188);
    let (mut session, new_id) = try_second_discard_new_draw_prefix(&encoded)
        .expect("second target-player discard new-draw prefix");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-discard"
            && descriptor["target"]["seat"] == "south"
    });
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "discard-card" && descriptor["cardInstanceId"] == new_id
    });
    assert!(event_types(&receipt).contains(&"card-discarded"));
    assert!(cemetery_ids(&state(&session), "south").contains(&new_id));
    assert!(!south_hand_instance_ids(&state(&session)).contains(&new_id));
    assert_exact_replay(&session);
}
