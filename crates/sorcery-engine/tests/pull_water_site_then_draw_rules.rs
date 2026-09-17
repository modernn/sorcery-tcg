//! Direct proofs for pull-adjacent-aboveground-unit-to-target-water-site
//! then draw-spell Magic (RULE-CATALOG-0547–0548, RULE-CATALOG-1069).
//!
//! Ordinary Magic can target a Water site, pull one aboveground unit that
//! borders that site onto it, and then draw one spell. A unit occupying the
//! site is not adjacent. Earth sites are not offered. When no aboveground
//! unit borders a Water site, the pull is a paid no-op that still draws.
//! While Deathrites wait for ordering, the pull-then-draw Magic stays
//! withheld until the chain drains.

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

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["water"],
    })
}

fn earth_site() -> Value {
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

fn riptide() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "pullAdjacentAbovegroundUnitToTargetWaterSiteThenDrawSpell": true,
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

fn riptide_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "pull-water-site-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-pull-water-site-then-draw-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-riptide": riptide(),
            "north-site": water_site(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-riptide",
                    "north-riptide",
                    "north-riptide",
                    "north-riptide",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["north-ally"; 6],
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
    let mut session = Session::new(encoded).expect("valid pull-water-site session");
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

fn riptide_offers(session: &Session) -> Vec<(String, Option<String>)> {
    session
        .legal_actions()
        .expect("riptide actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-riptide"
        })
        .map(|action| {
            let cell = action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
                .unwrap_or_default();
            let target = action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned);
            (cell, target)
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (547..547 + 256)
        .map(riptide_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Riptide opening cards")
}

fn empty_library_riptide_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "riptide-then-draw-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-riptide-then-draw-empty-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-riptide": riptide(),
            "north-site": water_site(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-riptide", "north-riptide"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["north-ally"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn seed_with_ally_and_riptide_empty(start: u32) -> String {
    (start..start + 256)
        .map(empty_library_riptide_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-riptide")
        })
        .expect("bounded seed with ally and Riptide filling the opening hand")
}

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn south_plays_c1_then_north_draws_atlas(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
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
fn rule_catalog_0547_riptide_pulls_an_adjacent_unit_onto_a_water_site_then_draws() {
    let encoded = seed_with(&["north-ally", "north-riptide"]);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
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
    assert_eq!(unit(&before, &ally_id)["location"], "C3");
    let offered = riptide_offers(&session);
    assert!(
        offered
            .iter()
            .any(|(cell, target)| cell == "C4" && target.as_deref() == Some(ally_id.as_str()))
    );
    assert!(!offered.iter().any(|(cell, _)| cell == "C1"));
    assert!(!offered.iter().any(|(cell, target)| {
        cell == "C4" && target.as_deref() == Some(north_avatar.as_str())
    }));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-riptide"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["target"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "unit-teleported",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["targetInstanceId"], ally_id);
    assert_eq!(granted.events[1].payload["from"]["cell"], "C3");
    assert_eq!(granted.events[1].payload["to"]["cell"], "C4");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["location"], "C4");
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
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0548_riptide_still_draws_without_an_adjacent_unit() {
    let encoded = seed_with(&["north-riptide"]);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    let before = state(&session);
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let offered = riptide_offers(&session);
    assert!(!offered.is_empty());
    assert!(
        offered
            .iter()
            .all(|(cell, target)| cell.is_empty() && target.is_none())
    );

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-riptide"
            && descriptor["target"].is_null()
            && descriptor["targetLocation"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "unit-teleported")
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

#[test]
fn rule_catalog_0993_riptide_pull_then_empty_spellbook_is_a_deck_out() {
    let encoded = seed_with_ally_and_riptide_empty(993);
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
    south_plays_c1_then_north_draws_atlas(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C3");
    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-riptide"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["target"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "unit-teleported",
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    assert_eq!(granted.events[1].payload["targetInstanceId"], ally_id);
    assert_eq!(granted.events[1].payload["from"]["cell"], "C3");
    assert_eq!(granted.events[1].payload["to"]["cell"], "C4");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["location"], "C4");
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

fn deathrite_riptide_manifest(seed: u32) -> String {
    let fixture = "riptide-deathrite-withheld";
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
            "north-rain": rain_spell(),
            "north-riptide": riptide(),
            "north-site": water_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-riptide",
                    "north-rain",
                    "north-rain",
                    "north-riptide",
                    "north-rain",
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

fn north_has_riptide_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-riptide", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteRiptideSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_adjacent_unit(encoded: &str) -> Option<PendingDeathriteRiptideSetup> {
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
    if !state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .any(|card| card["cardId"] == "north-ally")
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let adjacent = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    })?;
    let ally_id = adjacent.0["cardInstanceId"].as_str()?.to_owned();
    if !north_has_riptide_and_rain(&state(&session)) {
        return None;
    }
    let offered = riptide_offers(&session);
    if !offered
        .iter()
        .any(|(cell, target)| cell == "C4" && target.as_deref() == Some(ally_id.as_str()))
    {
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
    Some(PendingDeathriteRiptideSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_riptide_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_riptide_manifest)
        .find(|candidate| try_pending_deathrite_with_adjacent_unit(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Riptide Magic in hand")
}

#[test]
fn rule_catalog_1069_pull_water_site_then_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_riptide_seed_with(1069);
    let mut setup = try_pending_deathrite_with_adjacent_unit(&encoded)
        .expect("complete Riptide Deathrite withheld setup");
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
    assert_eq!(unit(&paused, &ally_id)["location"], "C3");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(riptide_offers(session).is_empty());

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
    assert_eq!(unit(&resumed, &ally_id)["location"], "C3");
    assert!(
        riptide_offers(session)
            .iter()
            .any(|(cell, target)| cell == "C4" && target.as_deref() == Some(ally_id.as_str()))
    );

    let (_, granted) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-riptide"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["target"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "unit-teleported",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(unit(&state(session), &ally_id)["location"], "C4");
    assert_exact_replay(session);
}
