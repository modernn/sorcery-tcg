//! Direct proofs for grant-Lethal-this-turn then draw-spell Magic (RULE-CATALOG-0531–0532).
//!
//! Ordinary Magic can give an allied minion Lethal this turn and then draw
//! one spell. Avatars and enemy minions are not offered. The Lethal mark uses
//! the shared this-turn source list, expires at End Phase, and lets one point
//! of damage destroy a tougher minion.

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

fn tough() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn gift() -> Value {
    json!({
        "cardType": "magic",
        "grantLethalToAllyThisTurnThenDrawSpell": true,
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

fn tough_deathrite() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 2,
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
            "contentHash": identity_hash(&json!({ "fixture": "grant-lethal-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-lethal-then-draw-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-tough": tough(),
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
                "spellbook": vec!["south-tough"; 6],
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
    let mut session = Session::new(encoded).expect("valid grant-lethal-then-draw session");
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

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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

fn cemetery_has(session: &Session, seat: &str, instance_id: &str) -> bool {
    state(session)["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
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

fn strike_minion(session: &mut Session, attacker_id: &str, enemy_id: &str) {
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
    accept_where(session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    if state(session)["phase"] == "intercept" {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "close-intercept"
        });
    }
}

fn seed_with_ally_and_gift() -> String {
    (531..531 + 256)
        .map(gift_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic in the opening hand")
}

fn opening_with_ally(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
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
    (session, ally_id)
}

fn south_summons_at_c4(session: &mut Session, card_id: &str) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == "C4"
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

fn south_summons_tough_at_c4(session: &mut Session) -> String {
    south_summons_at_c4(session, "south-tough")
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
fn rule_catalog_0531_lethal_grant_offers_allied_minions_then_draws_a_spell() {
    let encoded = seed_with_ally_and_gift();
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let enemy_id = south_summons_tough_at_c4(&mut session);
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
    let allies = gift_ally_ids(&session);
    assert!(allies.contains(&ally_id));
    assert!(!allies.contains(&north_avatar));
    assert!(!allies.contains(&enemy_id));

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "lethal-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    let after = state(&session);
    assert_eq!(
        unit(&after, &ally_id)["temporaryLethalSources"][0],
        granted.events[0].payload["instanceId"]
    );
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

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "lethal-expired" && event.payload["instanceId"] == ally_id
    }));
    assert!(unit(&state(&session), &ally_id)["temporaryLethalSources"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0532_granted_lethal_then_draw_kills_a_tougher_minion() {
    let encoded = seed_with_ally_and_gift();
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let enemy_id = south_summons_tough_at_c4(&mut session);
    let mut wounded = session.clone();
    strike_minion(&mut wounded, &ally_id, &enemy_id);
    assert_eq!(unit(&state(&wounded), &enemy_id)["damage"], 1);
    assert!(!cemetery_has(&wounded, "south", &enemy_id));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    strike_minion(&mut session, &ally_id, &enemy_id);
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert!(
        !state(&session)["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["instanceId"] == enemy_id)
    );
    assert_exact_replay(&session);
}

fn deathrite_grant_lethal_manifest(seed: u32) -> String {
    let fixture = "grant-lethal-then-draw-deathrite-withheld";
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

struct PendingDeathriteGrantLethalSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_allied_minion(
    encoded: &str,
) -> Option<PendingDeathriteGrantLethalSetup> {
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
    Some(PendingDeathriteGrantLethalSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_grant_lethal_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_grant_lethal_manifest)
        .find(|candidate| try_pending_deathrite_with_allied_minion(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with grant-lethal Magic in hand")
}

#[test]
fn rule_catalog_1064_grant_lethal_then_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_grant_lethal_seed_with(1064);
    let mut setup = try_pending_deathrite_with_allied_minion(&encoded)
        .expect("complete grant-lethal Deathrite withheld setup");
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
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "lethal-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    let after = state(session);
    assert_eq!(
        unit(&after, &ally_id)["temporaryLethalSources"][0],
        granted.events[0].payload["instanceId"]
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

fn deathrite_kill_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-lethal-then-draw-deathrite-kill" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-lethal-then-draw-deathrite-kill-v1",
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-gift": gift(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": tough_deathrite(),
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
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn seed_with_ally_gift_and_deathrite() -> String {
    (1081..1081 + 256)
        .map(deathrite_kill_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-gift")
        })
        .expect("bounded seed with ally and gift Magic in the opening hand")
}

#[test]
fn rule_catalog_1081_grant_lethal_then_draw_deathrite_draws_for_controller_on_kill() {
    let encoded = seed_with_ally_gift_and_deathrite();
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let enemy_id = south_summons_at_c4(&mut session, "south-deathrite");
    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-gift"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "lethal-granted",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert!(
        !event_types(&granted)
            .iter()
            .any(|event_type| *event_type == "site-drawn")
    );

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    strike_minion(&mut session, &ally_id, &enemy_id);
    let killed = session
        .transcript()
        .last()
        .expect("lethal Deathrite strike receipt")
        .clone();
    let types = event_types(&killed);
    let damage_dealt = types
        .iter()
        .position(|event_type| *event_type == "damage-dealt")
        .expect("damage-dealt index");
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let minion_died = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion-died index");
    assert!(
        damage_dealt < site_drawn && site_drawn < minion_died,
        "expected damage-dealt, deathrite site-drawn, then minion-died before strike completes; got {types:?}"
    );
    if let Some(magic_resolved) = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
    {
        assert!(
            site_drawn < magic_resolved,
            "magic-resolved must follow deathrite site-drawn; got {types:?}"
        );
        assert_eq!(types.last(), Some(&"magic-resolved"));
    }
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], enemy_id);

    let finished = state(&session);
    assert!(
        !finished["realm"]["units"]
            .as_array()
            .expect("units")
            .iter()
            .any(|unit| unit["instanceId"] == enemy_id)
    );
    assert!(cemetery_has(&session, "south", &enemy_id));
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}
