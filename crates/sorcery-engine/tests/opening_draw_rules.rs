use std::collections::BTreeSet;

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::game::{Game, GameEndReason, GameOutcome, IssuedAction};
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
fn rule_catalog_0801_mulligan_returns_chosen_cards_to_deck_bottom() {
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
fn rule_catalog_0809_first_player_skips_draw_second_chooses_deck() {
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
fn rule_catalog_0810_sites_expand_through_unoccupied_orthogonal_cells() {
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
fn rule_catalog_0811_avatar_draws_private_site_pays_tap_cost() {
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
fn rule_catalog_0812_empty_atlas_avatar_draw_pays_tap_and_loses() {
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
fn rule_catalog_0813_empty_normal_draw_immediately_loses() {
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

fn has_kind(session: &Session, kind: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| action.descriptor["kind"] == kind)
}

fn no_kind(session: &Session, kind: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .all(|action| action.descriptor["kind"] != kind)
}

fn avatar(draw_spell: bool) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": draw_spell,
        "life": 20,
    })
}

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn water_site() -> Value {
    json!({ "cardType": "site", "elements": ["water"] })
}

fn geomancer_avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "earthSitePlayCreatesAdjacentRubble": true,
        "life": 20,
    })
}

fn north_second_main_from_manifest(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid session");
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

fn north_opening_main_from_manifest(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid session");
    keep(&mut session);
    keep(&mut session);
    session
}

fn site_play_manifest(seed: u32, avatar_card: Value, site_card: Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "site-play-create-rubble", "seed": seed }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-site-play-create-rubble-{seed}-v1"),
        },
        "cards": {
            "north-avatar": avatar_card,
            "north-rain": rain(),
            "north-site": site_card,
            "south-avatar": avatar(false),
            "south-rain": rain(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-rain"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-rain"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn rain() -> Value {
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

fn deathrite_avatar_draw_manifest(seed: u32, draw_spell: bool) -> String {
    let fixture = if draw_spell {
        "avatar-spell-draw-deathrite-withheld"
    } else {
        "avatar-site-draw-deathrite-withheld"
    };
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(draw_spell),
            "north-rain": rain(),
            "north-site": site(),
            "south-avatar": avatar(false),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-rain"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
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
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

struct PendingDeathriteAvatarDrawSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_ready_deathrite_with_avatar_draw(
    encoded: &str,
    required_kinds: &[&str],
) -> Option<PendingDeathriteAvatarDrawSetup> {
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
    let main_actions = session.legal_actions().ok()?;
    if !required_kinds.iter().all(|kind| {
        main_actions
            .iter()
            .any(|action| action.descriptor["kind"] == *kind)
    }) {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteAvatarDrawSetup {
        deathrite_ids,
        session,
    })
}

fn try_cast_rain_to_deathrite_order(setup: &mut PendingDeathriteAvatarDrawSetup) -> bool {
    try_accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })
    .is_some()
        && state(&setup.session)["phase"] == "deathrite-order"
}

fn try_pending_deathrite_with_avatar_site_draw(
    encoded: &str,
) -> Option<PendingDeathriteAvatarDrawSetup> {
    let mut setup = try_ready_deathrite_with_avatar_draw(encoded, &["draw-site", "play-site"])?;
    try_cast_rain_to_deathrite_order(&mut setup).then_some(setup)
}

fn try_pending_deathrite_with_avatar_spell_draw(
    encoded: &str,
) -> Option<PendingDeathriteAvatarDrawSetup> {
    let mut setup = try_ready_deathrite_with_avatar_draw(encoded, &["draw-spell"])?;
    try_cast_rain_to_deathrite_order(&mut setup).then_some(setup)
}

fn deathrite_avatar_draw_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(|seed| deathrite_avatar_draw_manifest(seed, false))
        .find(|candidate| try_pending_deathrite_with_avatar_site_draw(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Avatar site draw legal")
}

fn deathrite_avatar_spell_draw_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(|seed| deathrite_avatar_draw_manifest(seed, true))
        .find(|candidate| try_pending_deathrite_with_avatar_spell_draw(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Avatar spell draw legal")
}

#[test]
fn rule_catalog_1132_avatar_site_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_avatar_draw_seed_with(1132);
    let mut setup = try_pending_deathrite_with_avatar_site_draw(&encoded)
        .expect("complete Avatar site-draw Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["players"]["north"]["avatar"]["tapped"], false);
    assert!(
        paused["players"]["north"]["hand"]["atlas"]
            .as_array()
            .is_some_and(|hand| !hand.is_empty())
    );
    assert!(no_kind(session, "draw-site"));
    assert!(no_kind(session, "play-site"));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "draw" || action.descriptor["zone"] != "atlas"
            })
    );

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
    assert_eq!(resumed["players"]["north"]["avatar"]["tapped"], false);
    assert!(has_kind(session, "draw-site"));
    assert!(has_kind(session, "play-site"));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1133_avatar_spell_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_avatar_spell_draw_seed_with(1133);
    let mut setup = try_ready_deathrite_with_avatar_draw(&encoded, &["draw-spell"])
        .expect("complete Avatar spell-draw Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let play_site_offered;
    {
        let session = &setup.session;
        let ready = state(session);
        assert_eq!(ready["phase"], "main");
        assert_eq!(ready["decisionSeat"], "north");
        assert_eq!(ready["players"]["north"]["avatar"]["tapped"], false);
        assert!(has_kind(session, "draw-spell"));
        play_site_offered = has_kind(session, "play-site");
    }

    assert!(try_cast_rain_to_deathrite_order(&mut setup));
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["players"]["north"]["avatar"]["tapped"], false);
    assert!(no_kind(session, "draw-spell"));
    if play_site_offered {
        assert!(no_kind(session, "play-site"));
    }

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
    assert_eq!(resumed["players"]["north"]["avatar"]["tapped"], false);
    assert!(has_kind(session, "draw-spell"));
    assert_exact_replay(session);
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn pulser() -> Value {
    json!({
        "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_draw_step_withheld_manifest(seed: u32) -> String {
    let fixture = "draw-step-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(false),
            "north-pulser": pulser(),
            "north-site": site(),
            "south-avatar": avatar(false),
            "south-deathrite": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-pulser"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
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

struct PendingDeathriteDrawStepSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_during_draw_step(encoded: &str) -> Option<PendingDeathriteDrawStepSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let pulser = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    if state(&session)["phase"] != "start-turn" {
        return None;
    }
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let offered: Vec<_> = session
        .legal_actions()
        .ok()?
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    if offered != [pulser_id.clone()] {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == pulser_id
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    if session
        .legal_actions()
        .ok()?
        .iter()
        .any(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteDrawStepSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_draw_step_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(deathrite_draw_step_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_draw_step(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites during Draw step")
}

#[test]
fn rule_catalog_1173_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_draw_step_seed_with(1173);
    let mut setup = try_pending_deathrite_during_draw_step(&encoded)
        .expect("complete Draw step Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["pendingDeathrites"]["returnPhase"], "draw");
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
            .all(|action| action.descriptor["kind"] != "draw"),
        "deathrite-order must issue no draw while Draw step stays pending"
    );

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
    assert_eq!(resumed["phase"], "draw");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "draw"),
        "draw must return once deathrite-order clears"
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1536_non_geomancer_earth_site_play_omits_create_rubble_at() {
    let mut session = north_second_main(23, false);
    let (second, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert!(second.get("createRubbleAt").is_none());
    let first_play = session
        .transcript()
        .iter()
        .flat_map(|receipt| receipt.events.iter())
        .find(|event| event.event_type == "site-played")
        .expect("first site-played event");
    assert!(first_play.payload.get("createRubbleAt").is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1543_geomancer_earth_site_play_includes_create_rubble_at() {
    let mut session =
        north_second_main_from_manifest(&site_play_manifest(1543, geomancer_avatar(), site()));
    let (second, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C3"
            && descriptor.get("createRubbleAt").is_some()
    });
    let rubble_cell = second["createRubbleAt"]
        .as_str()
        .expect("adjacent rubble cell");
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        vec!["site-played", "rubble-created"]
    );
    assert_eq!(receipt.events[1].payload["cell"], rubble_cell);
    assert_eq!(
        state(&session)["realm"]["sites"][rubble_cell]["rubble"],
        true
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1544_non_geomancer_water_site_play_omits_create_rubble_at() {
    let mut session =
        north_second_main_from_manifest(&site_play_manifest(1544, avatar(false), water_site()));
    let (second, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert!(second.get("createRubbleAt").is_none());
    let second_play = session
        .transcript()
        .iter()
        .flat_map(|receipt| receipt.events.iter())
        .filter(|event| event.event_type == "site-played")
        .nth(1)
        .expect("second site-played event");
    assert!(second_play.payload.get("createRubbleAt").is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1553_geomancer_water_second_main_play_omits_create_rubble_at() {
    let mut session = north_second_main_from_manifest(&site_play_manifest(
        1553,
        geomancer_avatar(),
        water_site(),
    ));
    let (second, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert!(second.get("createRubbleAt").is_none());
    let second_play = session
        .transcript()
        .iter()
        .flat_map(|receipt| receipt.events.iter())
        .filter(|event| event.event_type == "site-played")
        .nth(1)
        .expect("second site-played event");
    assert!(second_play.payload.get("createRubbleAt").is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1554_geomancer_first_earth_play_includes_create_rubble_at() {
    let mut session =
        north_opening_main_from_manifest(&site_play_manifest(1554, geomancer_avatar(), site()));
    let (first, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor.get("createRubbleAt").is_some()
    });
    let rubble_cell = first["createRubbleAt"]
        .as_str()
        .expect("adjacent rubble cell");
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        vec!["site-played", "rubble-created"]
    );
    assert_eq!(receipt.events[1].payload["cell"], rubble_cell);
    assert_eq!(
        state(&session)["realm"]["sites"][rubble_cell]["rubble"],
        true
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1555_non_geomancer_first_earth_play_omits_create_rubble_at() {
    let mut session =
        north_opening_main_from_manifest(&site_play_manifest(1555, avatar(false), site()));
    let (first, _) = accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    assert!(first.get("createRubbleAt").is_none());
    let first_play = session
        .transcript()
        .iter()
        .flat_map(|receipt| receipt.events.iter())
        .find(|event| event.event_type == "site-played")
        .expect("first site-played event");
    assert!(first_play.payload.get("createRubbleAt").is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1558_non_geomancer_water_first_play_omits_create_rubble_at() {
    let mut session =
        north_opening_main_from_manifest(&site_play_manifest(1558, avatar(false), water_site()));
    let (first, _) = accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    assert!(first.get("createRubbleAt").is_none());
    let first_play = session
        .transcript()
        .iter()
        .flat_map(|receipt| receipt.events.iter())
        .find(|event| event.event_type == "site-played")
        .expect("first site-played event");
    assert!(first_play.payload.get("createRubbleAt").is_none());
    assert_exact_replay(&session);
}
