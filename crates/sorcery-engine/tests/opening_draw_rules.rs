use std::collections::BTreeSet;

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::game::{Game, GameEndReason, GameOutcome};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

fn scenario_manifest(seed: u32, atlas_len: usize, spellbook_len: usize) -> String {
    let manifest_json = synthetic_demo_manifest_json(seed).expect("synthetic manifest");
    let mut manifest: Value = serde_json::from_str(&manifest_json).expect("manifest value");
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId")
        .expect("manifest identity");
    for seat in ["north", "south"] {
        manifest["decks"][seat]["atlas"]
            .as_array_mut()
            .expect("Atlas cards")
            .truncate(atlas_len);
        manifest["decks"][seat]["spellbook"]
            .as_array_mut()
            .expect("Spellbook cards")
            .truncate(spellbook_len);
    }
    let mut referenced = BTreeSet::new();
    for seat in ["north", "south"] {
        referenced.insert(
            manifest["decks"][seat]["avatar"]
                .as_str()
                .expect("Avatar card")
                .to_owned(),
        );
        for zone in ["atlas", "spellbook"] {
            referenced.extend(
                manifest["decks"][seat][zone]
                    .as_array()
                    .expect("deck zone")
                    .iter()
                    .map(|card| card.as_str().expect("card ID").to_owned()),
            );
        }
    }
    manifest["cards"]
        .as_object_mut()
        .expect("manifest cards")
        .retain(|card_id, _| referenced.contains(card_id));
    manifest["manifestId"] =
        serde_json::to_value(identity_hash(&manifest).expect("manifest identity"))
            .expect("manifest identity JSON");
    canonical_json(&manifest).expect("canonical scenario manifest")
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

fn keep(session: &mut Session) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })
    .1
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay value")["state"].clone()
}

fn events(receipt: &Receipt) -> Vec<Value> {
    receipt
        .events
        .iter()
        .map(|event| json!({ "payload": event.payload, "type": event.event_type }))
        .collect()
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

fn north_second_main(seed: u32, short_decks: bool) -> Session {
    let deck_len = if short_decks { 3 } else { 30 };
    let spellbook_len = if short_decks { 4 } else { 50 };
    let mut session =
        Session::new(&scenario_manifest(seed, deck_len, spellbook_len)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    session
}

#[test]
fn mulligan_returns_chosen_card_to_deck_bottom() {
    let mut session = Session::new(&scenario_manifest(11, 30, 50)).expect("valid session");
    let before = state(&session);
    let returned = before["players"]["north"]["hand"]["atlas"][0]["instanceId"]
        .as_str()
        .expect("returned card identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([returned])
            && descriptor["spellbookOrder"] == json!([])
    });
    assert_eq!(
        events(&receipt),
        vec![json!({
            "payload": { "atlasCount": 1, "seat": "north", "spellbookCount": 0 },
            "type": "mulligan-completed",
        })]
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["mulliganComplete"], true);
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .and_then(|cards| cards.last())
            .and_then(|card| card["instanceId"].as_str()),
        Some(returned.as_str())
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("Atlas hand")
            .iter()
            .all(|card| card["instanceId"] != returned)
    );
    assert_eq!(after["activeSeat"], "south");
    assert_exact_replay(&session);
}

#[test]
fn first_player_skips_draw_then_second_player_chooses_deck() {
    let mut session = Session::new(&scenario_manifest(13, 30, 50)).expect("valid session");
    keep(&mut session);
    let second_keep = keep(&mut session);
    assert_eq!(
        events(&second_keep),
        vec![
            json!({
                "payload": { "atlasCount": 0, "seat": "south", "spellbookCount": 0 },
                "type": "mulligan-completed",
            }),
            json!({
                "payload": { "drawSkipped": true, "seat": "north", "turnNumber": 1 },
                "type": "turn-started",
            }),
        ]
    );
    let opening = state(&session);
    assert_eq!(opening["turnNumber"], 1);
    assert_eq!(opening["activeSeat"], "north");
    assert_eq!(opening["phase"], "main");
    let (site, _) = accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    let after_site = state(&session);
    assert_eq!(
        after_site["realm"]["sites"]["C4"]["instanceId"],
        site["cardInstanceId"]
    );
    assert_eq!(after_site["players"]["north"]["domainEstablished"], true);
    assert_eq!(after_site["players"]["north"]["mana"], 1);
    assert_eq!(after_site["players"]["north"]["avatar"]["tapped"], true);
    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        events(&ended),
        vec![
            json!({ "payload": { "seat": "north", "turnNumber": 1 }, "type": "turn-ended" }),
            json!({
                "payload": { "drawSkipped": false, "seat": "south", "turnNumber": 2 },
                "type": "turn-started",
            }),
        ]
    );
    assert_eq!(
        session
            .legal_actions()
            .expect("draw actions")
            .iter()
            .map(|action| action.descriptor["zone"].clone())
            .collect::<Vec<_>>(),
        vec![json!("atlas"), json!("spellbook")]
    );
    let south_before = state(&session);
    let drawn = south_before["players"]["south"]["atlas"][0].clone();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    assert_eq!(
        events(&receipt),
        vec![json!({
            "payload": { "seat": "south", "zone": "atlas" },
            "type": "card-drawn",
        })]
    );
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["players"]["south"]["hand"]["atlas"][3], drawn);
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(drawn["cardId"].as_str().expect("drawn card ID"))
    );
    assert_exact_replay(&session);
}

#[test]
fn sites_expand_through_unoccupied_orthogonal_cells() {
    let mut session = north_second_main(23, false);
    let before = state(&session);
    let card = before["players"]["north"]["hand"]["atlas"][0].clone();
    let cells = session
        .legal_actions()
        .expect("main actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "play-site"
                && action.descriptor["cardInstanceId"] == card["instanceId"]
        })
        .map(|action| action.descriptor["cell"].clone())
        .collect::<Vec<_>>();
    assert_eq!(cells, vec![json!("B4"), json!("C3"), json!("D4")]);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardInstanceId"] == card["instanceId"]
            && descriptor["cell"] == "C3"
    });
    assert_eq!(
        events(&receipt),
        vec![json!({
            "payload": {
                "cardId": card["cardId"],
                "cell": "C3",
                "instanceId": card["instanceId"],
                "seat": "north",
            },
            "type": "site-played",
        })]
    );
    let after = state(&session);
    assert_eq!(
        after["realm"]["sites"]["C3"]["instanceId"],
        card["instanceId"]
    );
    assert_eq!(after["realm"]["sites"]["C3"]["controller"], "north");
    assert_eq!(after["players"]["north"]["mana"], 2);
    assert_exact_replay(&session);
}

#[test]
fn avatar_draws_private_site_and_pays_tap_cost() {
    let mut session = north_second_main(29, false);
    let before = state(&session);
    let drawn = before["players"]["north"]["atlas"][0].clone();
    let opponent_before = replay_game(&session).observe(Seat::South);
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "draw-site");
    assert_eq!(
        events(&receipt),
        vec![json!({ "payload": { "seat": "north" }, "type": "site-drawn" })]
    );
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(drawn["cardId"].as_str().expect("drawn card ID"))
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["atlas"].as_array().map(Vec::len),
        before["players"]["north"]["atlas"]
            .as_array()
            .map(|cards| cards.len() - 1)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .and_then(|cards| cards.last()),
        Some(&drawn)
    );
    assert_eq!(after["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(replay_game(&session).observe(Seat::South), opponent_before);
    let kinds = session
        .legal_actions()
        .expect("post-draw actions")
        .into_iter()
        .map(|action| action.descriptor["kind"].clone())
        .collect::<Vec<_>>();
    assert!(!kinds.contains(&json!("play-site")));
    assert!(!kinds.contains(&json!("draw-site")));
    assert!(kinds.contains(&json!("end-turn")));
    assert_exact_replay(&session);
}

#[test]
fn empty_atlas_avatar_draw_pays_tap_cost_and_loses() {
    let mut session = north_second_main(31, true);
    assert_eq!(state(&session)["players"]["north"]["atlas"], json!([]));
    assert!(
        session
            .legal_actions()
            .expect("main actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "draw-site")
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor["kind"] == "draw-site");
    assert_eq!(
        events(&receipt),
        vec![json!({
            "payload": { "loser": "north", "reason": "deck_empty", "winner": "south" },
            "type": "game-ended",
        })]
    );
    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["tapped"], true);
    assert_eq!(after["phase"], "terminal");
    assert_eq!(
        after["terminal"],
        json!({
            "loser": "north",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "south",
        })
    );
    assert_eq!(
        session.outcome(),
        Some(GameOutcome::Win {
            loser: Seat::North,
            winner: Seat::South,
        })
    );
    assert_eq!(
        replay_game(&session).terminal_reason(),
        Some(GameEndReason::DeckEmpty)
    );
    assert!(
        session
            .legal_actions()
            .expect("terminal actions")
            .is_empty()
    );
    assert_exact_replay(&session);
}

#[test]
fn empty_normal_draw_immediately_loses() {
    let mut session = Session::new(&scenario_manifest(19, 3, 3)).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(state(&session)["players"]["south"]["atlas"], json!([]));
    assert!(
        session
            .legal_actions()
            .expect("draw actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "draw" && action.descriptor["zone"] == "atlas"
            })
    );
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    assert_eq!(
        events(&receipt),
        vec![json!({
            "payload": { "loser": "south", "reason": "deck_empty", "winner": "north" },
            "type": "game-ended",
        })]
    );
    let after = state(&session);
    assert_eq!(after["players"]["south"]["avatar"]["tapped"], false);
    assert_eq!(after["phase"], "terminal");
    assert_eq!(
        after["terminal"],
        json!({
            "loser": "south",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "north",
        })
    );
    assert_eq!(
        session.outcome(),
        Some(GameOutcome::Win {
            loser: Seat::South,
            winner: Seat::North,
        })
    );
    assert!(
        session
            .legal_actions()
            .expect("terminal actions")
            .is_empty()
    );
    assert_exact_replay(&session);
}
