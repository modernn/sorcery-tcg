//! Direct proofs for target-player discard Magic (RULE-CATALOG-0643–0644,
//! RULE-CATALOG-0919, RULE-CATALOG-1050).
//!
//! Target-player discard offers only both Avatars. After the cast is
//! announced, the targeted player chooses one of their own Atlas or
//! Spellbook hand cards. An empty hand is a paid no-op, never a random
//! discard and never a deck-out. A count above one keeps the pending
//! Storyline open until each sequential choice is taken. While Deathrites
//! wait for ordering, target-player discard Magic stays withheld until
//! the chain drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode, Seat};
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
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter_map(|action| {
            if action.descriptor["kind"] != "discard-card" {
                return None;
            }
            Some((
                action.descriptor["cardInstanceId"]
                    .as_str()
                    .expect("discard identity")
                    .to_owned(),
                action.descriptor["zone"]
                    .as_str()
                    .expect("discard zone")
                    .to_owned(),
            ))
        })
        .collect()
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
        (
            south_sites[0].clone(),
            "atlas",
        )
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
