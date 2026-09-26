//! Direct proofs for grant-+2-this-turn then draw-spell Magic (RULE-CATALOG-0529–0530,
//! RULE-CATALOG-1061, RULE-CATALOG-1633–1638).
//!
//! Ordinary Magic can give an allied minion +2 power this turn and then draw
//! one spell. Avatars and enemy minions are not offered. The power uses the
//! shared this-turn source list and expires at End Phase. An empty Spellbook
//! after the grant is a deck-out. Strike damage follows current derived power,
//! including printed attack plus active temporary grants.

#[path = "common/mod.rs"]
mod common;
use common::modifier_sources;

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

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn enemy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn gift() -> Value {
    json!({
        "cardType": "magic",
        "grantPowerTwoToAllyThisTurnThenDrawSpell": true,
        "manaCost": 0,
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
            "contentHash": identity_hash(&json!({ "fixture": "grant-power-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-power-then-draw-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy(),
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
                "spellbook": vec!["south-enemy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn empty_library_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-power-then-draw-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-power-then-draw-empty-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": enemy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-ally", "north-gift", "north-gift"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 6],
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
    let mut session = Session::new(encoded).expect("valid grant-power-then-draw session");
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
        .filter(|action| action.descriptor["kind"] == "choose-ability")
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn grant_power(session: &mut Session, ally_id: &str) -> (Value, Receipt) {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-gift"
    });
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "choose-ability" && descriptor["target"]["instanceId"] == ally_id
    });
    (descriptor, receipt)
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
#[expect(
    clippy::too_many_lines,
    reason = "one scenario checks recipient legality, hidden draw, expiration and replay"
)]
fn rule_catalog_0529_power_grant_offers_allied_minions_then_draws_a_spell() {
    let encoded = (529..529 + 256)
        .map(gift_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate, "north");
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic in the opening hand");
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
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (enemy_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let enemy_id = enemy_summon["cardInstanceId"]
        .as_str()
        .expect("enemy instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

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
    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let (grant_descriptor, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-gift"
    });
    let allies = gift_ally_ids(&session);
    assert!(allies.contains(&ally_id));
    assert!(!allies.contains(&north_avatar));
    assert!(!allies.contains(&enemy_id));

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "choose-ability" && descriptor["target"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "ability-choice-committed",
            "power-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["amount"], 2);
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    let after = state(&session);
    assert_eq!(
        modifier_sources(unit(&after, &ally_id), "power")[0],
        grant_descriptor["cardInstanceId"]
    );
    let hand_after = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand after draw");
    assert_eq!(hand_after.len(), hand_before);
    assert!(
        hand_after
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert!(
        !serde_json::to_string(&south_view)
            .expect("view JSON")
            .contains(&library_top)
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(event_types(&ended).contains(&"power-expired"));
    let expired = state(&session);
    assert_eq!(
        modifier_sources(unit(&expired, &ally_id), "power"),
        json!([])
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0530_power_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = (530..530 + 256)
        .map(empty_library_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate, "north");
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic filling the opening hand");
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
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
    let (_, granted) = grant_power(&mut session, &ally_id);
    assert_eq!(
        event_types(&granted),
        [
            "ability-choice-committed",
            "power-granted",
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
    let after = state(&session);
    assert_eq!(
        modifier_sources(unit(&after, &ally_id), "power")
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}

fn deathrite_grant_power_manifest(seed: u32) -> String {
    let fixture = "grant-power-then-draw-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
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

struct PendingDeathriteGrantPowerSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_allied_minion(
    encoded: &str,
) -> Option<PendingDeathriteGrantPowerSetup> {
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
    if !session.legal_actions().ok()?.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-gift"
    }) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteGrantPowerSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_grant_power_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_grant_power_manifest)
        .find(|candidate| try_pending_deathrite_with_allied_minion(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with grant-power Magic in hand")
}

#[test]
fn rule_catalog_1061_grant_power_then_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_grant_power_seed_with(1061);
    let mut setup = try_pending_deathrite_with_allied_minion(&encoded)
        .expect("complete grant-power Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
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
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(unit(&resumed, &ally_id).is_object());
    let (grant_descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-gift"
    });
    assert!(gift_ally_ids(session).contains(&ally_id));

    let library_top = resumed["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let hand_before = resumed["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();

    let (_, granted) = accept_where(session, |descriptor| {
        descriptor["kind"] == "choose-ability" && descriptor["target"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "ability-choice-committed",
            "power-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["amount"], 2);
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    let after = state(session);
    assert_eq!(
        modifier_sources(unit(&after, &ally_id), "power")[0],
        grant_descriptor["cardInstanceId"]
    );
    let hand_after = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand after draw");
    assert_eq!(hand_after.len(), hand_before);
    assert!(
        hand_after
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(session);
}

fn striker() -> Value {
    json!({
        "attack": 3,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn printed_striker() -> Value {
    json!({
        "attack": 5,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn plain_striker() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn tough_target() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": 10,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn power_combat_manifest_with_north_ally(ally: &Value, seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-power-combat" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-power-combat-v1",
        },
        "cards": {
            "north-ally": ally,
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-enemy": tough_target(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-gift",
                    "north-gift",
                    "north-rain",
                    "north-rain",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-enemy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

struct PowerCombatSetup {
    ally_id: String,
    enemy_id: String,
    session: Session,
}

fn try_power_combat_setup(encoded: &str) -> Option<PowerCombatSetup> {
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
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    let enemy = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C4"
    })?;
    let enemy_id = enemy.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "draw")?;
    Some(PowerCombatSetup {
        ally_id,
        enemy_id,
        session,
    })
}

fn power_combat_setup(ally: &Value, start: u32) -> PowerCombatSetup {
    (start..start + 256)
        .map(|seed| power_combat_manifest_with_north_ally(ally, seed))
        .find_map(|candidate| try_power_combat_setup(&candidate))
        .expect("bounded seed reaching combat setup with ally and enemy on board")
}

fn power_combat_setup_with_gift(ally: &Value, start: u32) -> PowerCombatSetup {
    (start..start + 256)
        .map(|seed| power_combat_manifest_with_north_ally(ally, seed))
        .find_map(|candidate| {
            let setup = try_power_combat_setup(&candidate)?;
            session_has_gift_cast(&setup.session).then_some(setup)
        })
        .expect("bounded seed reaching combat setup with grant-power Magic in hand")
}

fn through_south_pass_to_north_main(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn session_has_gift_cast(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("gift actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-gift"
        })
}

fn expire_grant_power(session: &mut Session, ally_id: &str, grant_source: &str) {
    let (_, ended) = accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "power-expired"
            && event.payload["instanceId"] == ally_id
            && event.payload["sourceInstanceId"] == grant_source
    }));
    assert!(
        modifier_sources(unit(&state(session), ally_id), "power")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn strike_minion(session: &mut Session, attacker_id: &str, enemy_id: &str) -> Receipt {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C4"
    });
    while state(session)["phase"] == "movement" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
    }
    accept_where(session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == enemy_id
    });
    let (_, fight) = accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(session)["phase"] == "intercept" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
    fight
}

fn strike_allocated_to_target(fight: &Receipt, attacker_id: &str, enemy_id: &str) -> u8 {
    u8::try_from(
        fight
            .events
            .iter()
            .find(|event| {
                event.event_type == "strike-damage-allocated"
                    && event.payload["strikerInstanceId"] == attacker_id
                    && event.payload["targetInstanceId"] == enemy_id
            })
            .expect("strike allocation")
            .payload["amount"]
            .as_u64()
            .expect("allocated strike power"),
    )
    .expect("strike power fits u8")
}

#[test]
fn rule_catalog_1633_printed_power_strikes_at_full_attack_without_grant() {
    let PowerCombatSetup {
        mut session,
        ally_id,
        enemy_id,
    } = power_combat_setup(&printed_striker(), 1633);
    let fight = strike_minion(&mut session, &ally_id, &enemy_id);
    assert_eq!(strike_allocated_to_target(&fight, &ally_id, &enemy_id), 5);
    assert_eq!(unit(&state(&session), &enemy_id)["damage"], 5);
    assert!(
        modifier_sources(unit(&state(&session), &ally_id), "power")
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1634_granted_power_strikes_at_boosted_power_before_end_of_turn() {
    let PowerCombatSetup {
        mut session,
        ally_id,
        enemy_id,
    } = power_combat_setup_with_gift(&striker(), 1634);
    let (descriptor, receipt) = grant_power(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        [
            "ability-choice-committed",
            "power-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(receipt.events[1].payload["amount"], 2);
    assert_eq!(
        modifier_sources(unit(&state(&session), &ally_id), "power"),
        json!([descriptor["cardInstanceId"]])
    );
    let fight = strike_minion(&mut session, &ally_id, &enemy_id);
    assert_eq!(strike_allocated_to_target(&fight, &ally_id, &enemy_id), 5);
    assert_eq!(unit(&state(&session), &enemy_id)["damage"], 5);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1635_granted_power_expires_before_ally_strikes_at_base_power_on_later_turn() {
    let PowerCombatSetup {
        mut session,
        ally_id,
        enemy_id,
    } = power_combat_setup_with_gift(&striker(), 1635);
    let (descriptor, _) = grant_power(&mut session, &ally_id);
    expire_grant_power(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    through_south_pass_to_north_main(&mut session);
    let fight = strike_minion(&mut session, &ally_id, &enemy_id);
    assert_eq!(strike_allocated_to_target(&fight, &ally_id, &enemy_id), 3);
    assert_eq!(unit(&state(&session), &enemy_id)["damage"], 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1636_printed_power_still_strikes_at_full_attack_after_grant_expires_on_later_turn()
{
    let PowerCombatSetup {
        mut session,
        ally_id,
        enemy_id,
    } = power_combat_setup_with_gift(&printed_striker(), 1636);
    let (descriptor, _) = grant_power(&mut session, &ally_id);
    expire_grant_power(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );
    through_south_pass_to_north_main(&mut session);
    let fight = strike_minion(&mut session, &ally_id, &enemy_id);
    assert_eq!(strike_allocated_to_target(&fight, &ally_id, &enemy_id), 5);
    assert_eq!(unit(&state(&session), &enemy_id)["damage"], 5);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1637_printed_and_granted_power_compose_while_grant_is_active() {
    let PowerCombatSetup {
        mut session,
        ally_id,
        enemy_id,
    } = power_combat_setup_with_gift(&printed_striker(), 1637);
    let (descriptor, receipt) = grant_power(&mut session, &ally_id);
    assert_eq!(
        event_types(&receipt),
        [
            "ability-choice-committed",
            "power-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(
        modifier_sources(unit(&state(&session), &ally_id), "power"),
        json!([descriptor["cardInstanceId"]])
    );
    let fight = strike_minion(&mut session, &ally_id, &enemy_id);
    assert_eq!(strike_allocated_to_target(&fight, &ally_id, &enemy_id), 7);
    assert_eq!(unit(&state(&session), &enemy_id)["damage"], 7);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1638_printed_power_outlasts_expired_grant_while_plain_ally_strikes_at_base_power() {
    let PowerCombatSetup {
        mut session,
        ally_id,
        enemy_id,
    } = power_combat_setup_with_gift(&printed_striker(), 1638);
    let (descriptor, _) = grant_power(&mut session, &ally_id);
    expire_grant_power(
        &mut session,
        &ally_id,
        descriptor["cardInstanceId"].as_str().expect("grant source"),
    );

    let PowerCombatSetup {
        session: mut plain,
        ally_id: plain_ally,
        enemy_id: plain_enemy,
    } = power_combat_setup_with_gift(&plain_striker(), 1638);
    let (plain_descriptor, _) = grant_power(&mut plain, &plain_ally);
    expire_grant_power(
        &mut plain,
        &plain_ally,
        plain_descriptor["cardInstanceId"]
            .as_str()
            .expect("grant source"),
    );
    through_south_pass_to_north_main(&mut plain);
    let plain_fight = strike_minion(&mut plain, &plain_ally, &plain_enemy);
    assert_eq!(
        strike_allocated_to_target(&plain_fight, &plain_ally, &plain_enemy),
        2
    );

    through_south_pass_to_north_main(&mut session);
    let fight = strike_minion(&mut session, &ally_id, &enemy_id);
    assert_eq!(strike_allocated_to_target(&fight, &ally_id, &enemy_id), 5);
    assert_exact_replay(&session);
}
