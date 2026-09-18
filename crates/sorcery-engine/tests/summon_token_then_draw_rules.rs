//! Direct proofs for summon-token-to-allied-minion then draw-spell Magic
//! (RULE-CATALOG-0543–0544, RULE-CATALOG-1068, RULE-CATALOG-1693–1698).
//!
//! Ordinary Magic can summon one source-linked token to an allied minion's
//! surface cell and then draw one spell. Avatars and enemy minions are not
//! hosts. When no allied minion is in play, the summon is a paid no-op that
//! still draws. While Deathrites wait for ordering, summon-token-then-draw
//! Magic stays withheld until the chain drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn grounded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn frog_token() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": 0,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
    })
}

fn gift() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "summonTokenToAlliedMinionThenDrawSpell": "north-frog",
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

fn gift_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "summon-token-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-summon-token-then-draw-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-frog": frog_token(),
            "north-gift": gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": grounded(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-gift",
                    "north-gift",
                    "north-gift",
                    "north-gift",
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
    let mut session = Session::new(encoded).expect("valid summon-token-then-draw session");
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn avatar_instance_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar instance identity")
        .to_owned()
}

fn gift_ally_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("gift actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-gift"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (543..543 + 256)
        .map(gift_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Gift opening cards")
}

fn south_plays_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
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

#[test]
fn rule_catalog_0543_token_then_draw_summons_on_an_allied_minion() {
    let encoded = seed_with(&["north-ally", "north-gift"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    let enemy_id = south_plays_c1(&mut session);
    let before = state(&session);
    let north_avatar = avatar_instance_id(&before, "north");
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let offered = gift_ally_ids(&session);
    assert!(offered.contains(&ally_id));
    assert!(!offered.contains(&north_avatar));
    assert!(!offered.contains(&enemy_id));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-summoned",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["cardId"], "north-frog");
    assert_eq!(granted.events[1].payload["cell"], "C4");
    assert_eq!(granted.events[1].payload["token"], true);
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let token_id = granted.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity")
        .to_owned();
    let after = state(&session);
    let token = unit(&after, &token_id);
    assert_eq!(token["cardId"], "north-frog");
    assert_eq!(token["location"], "C4");
    assert_eq!(token["source"], "token");
    assert_eq!(token["summoningSickness"], true);
    assert_eq!(after["cards"]["north-frog"]["token"], true);
    assert_eq!(after["cards"]["north-frog"]["attack"], 0);
    assert_eq!(after["cards"]["north-frog"]["defense"], 0);
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert!(
        !serde_json::to_string(&south_view)
            .expect("view JSON")
            .contains(&library_top)
    );
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0544_token_then_draw_still_draws_without_an_allied_minion() {
    let encoded = seed_with(&["north-gift"]);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    assert_eq!(gift_ally_ids(&session), [] as [String; 0]);

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .is_empty()
    );
    assert!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(&session);
}

fn gift_manifest_with_spellbook(seed: u32, spellbook: &[&str]) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "summon-token-then-draw-proof" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-summon-token-then-draw-proof-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-frog": frog_token(),
            "north-gift": gift(),
            "north-second": grounded(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": grounded(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": spellbook,
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

fn summon_north_ally(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned()
}

fn opening_with_ally(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let ally_id = summon_north_ally(&mut session);
    (session, ally_id)
}

fn pass_full_round(session: &mut Session) {
    let _enemy = south_plays_c1(session);
}

fn cast_gift_on(session: &mut Session, ally_id: &str) -> (Receipt, Option<String>) {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    let token_id = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-summoned")
        .and_then(|event| event.payload["instanceId"].as_str())
        .map(ToOwned::to_owned);
    (receipt, token_id)
}

fn token_ids_at_cell(snapshot: &Value, cell: &str) -> Vec<String> {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| unit["location"] == cell && unit["source"] == "token")
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("token identity")
                .to_owned()
        })
        .collect()
}

fn unit_has_move(session: &Session, unit_id: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == unit_id
        })
}

fn lay_site_at(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn seed_with_spellbook(required: &[&str], start: u32) -> String {
    let spellbook = vec![
        "north-ally",
        "north-second",
        "north-gift",
        "north-gift",
        "north-gift",
        "north-gift",
    ];
    (start..start + 256)
        .map(|seed| gift_manifest_with_spellbook(seed, &spellbook))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Gift opening cards")
}

fn host_setup(start: u32) -> (Session, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-gift"], start);
    opening_with_ally(&encoded)
}

fn host_setup_with_two_gifts(start: u32) -> (Session, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-gift", "north-gift"], start);
    opening_with_ally(&encoded)
}

fn host_setup_for_moved_ally(start: u32) -> Option<(Session, String)> {
    let encoded = seed_with_spellbook(&["north-ally", "north-gift"], start);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let _enemy = south_plays_c1(&mut session);
    if unit(&state(&session), &ally_id)["summoningSickness"] != false {
        return None;
    }
    lay_site_at(&mut session, "C3");
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == ally_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    })?;
    while state(&session)["phase"] == "movement" {
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        })?;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    })?;
    Some((session, ally_id))
}

fn host_setup_with_two_allies(start: u32) -> (Session, String, String) {
    let encoded = seed_with_spellbook(&["north-ally", "north-second", "north-gift"], start);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let _enemy = south_plays_c1(&mut session);
    lay_site_at(&mut session, "C3");
    let (second, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-second"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    let second_id = second["cardInstanceId"]
        .as_str()
        .expect("second ally identity")
        .to_owned();
    (session, ally_id, second_id)
}

#[test]
fn rule_catalog_1693_summoned_token_co_locates_with_host_ally_at_cast() {
    let (mut session, ally_id) = host_setup(1693);
    let ally_cell = unit(&state(&session), &ally_id)["location"]
        .as_str()
        .expect("ally cell")
        .to_owned();
    let (_, token_id) = cast_gift_on(&mut session, &ally_id);
    let token_id = token_id.expect("token summoned");
    let after = state(&session);
    let token = unit(&after, &token_id);
    assert_eq!(token["location"], ally_cell);
    assert_eq!(token["cardId"], "north-frog");
    assert_eq!(token["source"], "token");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1694_second_gift_summons_a_second_token_at_the_host_cell() {
    let (mut session, ally_id) = host_setup_with_two_gifts(1694);
    let (_, first_token) = cast_gift_on(&mut session, &ally_id);
    let first_token = first_token.expect("first token");
    let host_cell = unit(&state(&session), &ally_id)["location"]
        .as_str()
        .expect("host cell")
        .to_owned();
    let (_, second_token) = cast_gift_on(&mut session, &ally_id);
    let second_token = second_token.expect("second token");
    assert_ne!(first_token, second_token);
    let tokens = token_ids_at_cell(&state(&session), &host_cell);
    assert_eq!(tokens.len(), 2);
    assert!(tokens.contains(&first_token));
    assert!(tokens.contains(&second_token));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1695_summoned_token_has_summoning_sickness_and_cannot_move() {
    let (mut session, ally_id) = host_setup(1695);
    let (_, token_id) = cast_gift_on(&mut session, &ally_id);
    let token_id = token_id.expect("token summoned");
    assert_eq!(unit(&state(&session), &token_id)["summoningSickness"], true);
    assert!(!unit_has_move(&session, &token_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1696_summoned_token_stays_at_host_cell_after_turns_pass() {
    let (mut session, ally_id) = host_setup(1696);
    let (_, token_id) = cast_gift_on(&mut session, &ally_id);
    let token_id = token_id.expect("token summoned");
    let host_cell = unit(&state(&session), &ally_id)["location"]
        .as_str()
        .expect("host cell")
        .to_owned();
    pass_full_round(&mut session);
    assert_eq!(unit(&state(&session), &token_id)["location"], host_cell);
    assert_eq!(unit(&state(&session), &ally_id)["location"], host_cell);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1697_gift_summons_token_at_host_new_cell_after_ally_moves() {
    let (mut session, ally_id) = (1697..1697 + 256)
        .find_map(host_setup_for_moved_ally)
        .expect("bounded seed reaching moved host setup");
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C3");
    let (_, token_id) = cast_gift_on(&mut session, &ally_id);
    let token_id = token_id.expect("token summoned");
    assert_eq!(unit(&state(&session), &token_id)["location"], "C3");
    assert_eq!(token_ids_at_cell(&state(&session), "C4").len(), 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1698_gift_places_token_only_on_the_chosen_host_cell() {
    let (mut session, home_id, away_id) = host_setup_with_two_allies(1698);
    assert_eq!(unit(&state(&session), &home_id)["location"], "C4");
    assert_eq!(unit(&state(&session), &away_id)["location"], "C3");
    let (_, token_id) = cast_gift_on(&mut session, &away_id);
    let token_id = token_id.expect("token summoned");
    assert_eq!(unit(&state(&session), &token_id)["location"], "C3");
    assert_eq!(token_ids_at_cell(&state(&session), "C3"), vec![token_id]);
    assert!(token_ids_at_cell(&state(&session), "C4").is_empty());
    assert_exact_replay(&session);
}

fn deathrite_summon_token_manifest(seed: u32) -> String {
    let fixture = "summon-token-then-draw-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-frog": frog_token(),
            "north-gift": gift(),
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
                    "north-gift",
                    "north-rain",
                    "north-rain",
                    "north-gift",
                    "north-gift",
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

fn north_has_gift_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-gift", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteSummonTokenSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_allied_minion(
    encoded: &str,
) -> Option<PendingDeathriteSummonTokenSetup> {
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
    if !north_has_gift_and_rain(&state(&session)) {
        return None;
    }
    if !gift_ally_ids(&session).contains(&ally_id) {
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
    Some(PendingDeathriteSummonTokenSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_summon_token_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_summon_token_manifest)
        .find(|candidate| try_pending_deathrite_with_allied_minion(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with summon-token Magic in hand")
}

#[test]
fn rule_catalog_1068_summon_token_then_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_summon_token_seed_with(1068);
    let mut setup = try_pending_deathrite_with_allied_minion(&encoded)
        .expect("complete summon-token Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
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
    assert!(unit(&paused, &ally_id).is_object());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(gift_ally_ids(session).is_empty());

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
    assert!(unit(&resumed, &ally_id).is_object());
    let offered = gift_ally_ids(session);
    assert!(offered.contains(&ally_id));
    assert!(offered.iter().all(|id| id == &ally_id));

    let (cast, granted) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-summoned",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["cardId"], "north-frog");
    assert_eq!(granted.events[1].payload["cell"], "C4");
    assert_eq!(granted.events[1].payload["token"], true);
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let token_id = granted.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity");
    assert_eq!(unit(&state(session), token_id)["cardId"], "north-frog");
    assert_exact_replay(session);
}
