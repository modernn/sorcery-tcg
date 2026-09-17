//! Direct proofs for ally-strikes-each-enemy-at-its-location Magic
//! (RULE-CATALOG-0557–0558, 1011, 1093).
//!
//! 1011 covers ally strike here killing a Deathrite minion: the controller
//! draws a site and magic-resolved only appears after deathrite settlement.
//! While Deathrites wait for ordering, ally-strike-here Magic stays withheld
//! until the chain drains.
//!
//! Ordinary Magic chooses a controlled ally. That ally strikes every
//! enemy sharing its current cell and region without taking a step. No
//! enemy there is a paid no-op.

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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn fighter() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_raider() -> Value {
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

fn spin() -> Value {
    json!({
        "allyStrikesEachEnemyAtItsLocation": true,
        "cardType": "magic",
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn spin_manifest(seed: u32) -> String {
    spin_manifest_with_raider(seed, &raider())
}

fn spin_deathrite_manifest(seed: u32) -> String {
    spin_manifest_with_raider(seed, &deathrite_raider())
}

fn spin_manifest_with_raider(seed: u32, south_raider: &Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ally-strike-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ally-strike-here-v1",
        },
        "cards": {
            "north-ally": fighter(),
            "north-avatar": avatar(),
            "north-site": earth_site(),
            "north-spin": spin(),
            "south-avatar": avatar(),
            "south-raider": south_raider,
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-spin",
                    "north-spin",
                    "north-spin",
                    "north-spin",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-raider"; 6],
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
    let mut session = Session::new(encoded).expect("valid ally-strike-here session");
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

fn spin_ally_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("spin actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-spin"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    seed_with_manifest(required, spin_manifest)
}

fn seed_with_deathrite(required: &[&str]) -> String {
    seed_with_manifest(required, spin_deathrite_manifest)
}

fn seed_with_manifest(required: &[&str], manifest: impl Fn(u32) -> String) -> String {
    (557..557 + 256)
        .map(manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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

fn south_plays_c1_and_raids_c4(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let (nearby, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (
        far["cardInstanceId"]
            .as_str()
            .expect("far enemy identity")
            .to_owned(),
        nearby["cardInstanceId"]
            .as_str()
            .expect("nearby enemy identity")
            .to_owned(),
    )
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

fn deathrite_spin_manifest(seed: u32) -> String {
    let fixture = "ally-strike-here-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": fighter(),
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "north-spin": spin(),
            "south-avatar": avatar(),
            "south-minion": deathrite_raider(),
            "south-raider": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-spin",
                    "north-rain",
                    "north-spin",
                    "north-rain",
                    "north-spin",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-raider", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_spin_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-spin", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteSpinSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    nearby_id: String,
    session: Session,
}

fn try_pending_deathrite_with_ready_ally(encoded: &str) -> Option<PendingDeathriteSpinSetup> {
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
    let nearby = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let nearby_id = nearby.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_spin_and_rain(&state(&session)) {
        return None;
    }
    if !spin_ally_ids(&session).contains(&ally_id) {
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
    Some(PendingDeathriteSpinSetup {
        ally_id,
        deathrite_ids,
        nearby_id,
        session,
    })
}

fn deathrite_spin_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_spin_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_ally(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with ally-strike-here Magic in hand")
}

#[test]
fn rule_catalog_0557_ally_strikes_each_enemy_at_its_location() {
    let encoded = seed_with(&["north-ally", "north-spin"]);
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
    let (far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar identity")
        .to_owned();
    let offered = spin_ally_ids(&session);
    assert!(offered.contains(&ally_id));
    assert!(offered.contains(&avatar_id));
    assert_eq!(unit(&state(&session), &nearby_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);

    let (cast, struck) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-spin"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    let types = event_types(&struck);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(!types.contains(&"unit-stepped"));
    let allocations: Vec<_> = struck
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .collect();
    assert_eq!(allocations.len(), 1);
    assert_eq!(allocations[0].payload["amount"], 2);
    assert_eq!(allocations[0].payload["strikerInstanceId"], ally_id);
    assert_eq!(allocations[0].payload["targetInstanceId"], nearby_id);
    assert_eq!(
        struck.events[0].payload["instanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(unit(&after, &nearby_id)["damage"], 2);
    assert_eq!(unit(&after, &far_id)["damage"], 0);
    assert_eq!(unit(&after, &ally_id)["damage"], 0);
    assert_eq!(unit(&after, &ally_id)["location"], "C4");
    assert_eq!(unit(&after, &ally_id)["tapped"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0558_ally_strike_here_is_a_paid_noop_without_an_enemy() {
    let encoded = seed_with(&["north-ally", "north-spin"]);
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
    south_plays_c1(&mut session);
    assert!(spin_ally_ids(&session).contains(&ally_id));

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-spin"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C4");
    assert_eq!(unit(&state(&session), &ally_id)["damage"], 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1011_ally_strike_here_deathrite_draws_for_minion_controller_on_kill() {
    let encoded = (1011..1011 + 256)
        .map(spin_deathrite_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|card| card == "north-ally")
                && hand.iter().any(|card| card == "north-spin")
        })
        .unwrap_or_else(|| seed_with_deathrite(&["north-ally", "north-spin"]));
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
    let (_, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (_, struck) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-spin"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&struck),
        [
            "magic-cast",
            "strike-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(struck.events[1].payload["targetInstanceId"], nearby_id);
    let drawn = struck
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], nearby_id);
    let site_drawn = event_types(&struck)
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let magic_resolved = event_types(&struck)
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        site_drawn < magic_resolved,
        "magic-resolved must follow deathrite site-drawn"
    );
    assert_eq!(event_types(&struck).last(), Some(&"magic-resolved"));

    let finished = state(&session);
    assert_eq!(unit(&finished, &ally_id)["damage"], 0);
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == nearby_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1093_ally_strike_here_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_spin_seed_with(1093);
    let mut setup = try_pending_deathrite_with_ready_ally(&encoded)
        .expect("complete ally-strike-here Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
    let nearby_id = setup.nearby_id.clone();
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
    assert_eq!(unit(&paused, &ally_id)["location"], "C4");
    assert_eq!(unit(&paused, &nearby_id)["location"], "C4");
    assert_eq!(unit(&paused, &nearby_id)["damage"], 1);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(spin_ally_ids(session).is_empty());

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
    assert!(spin_ally_ids(session).contains(&ally_id));

    let (cast, struck) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-spin"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
    });
    let types = event_types(&struck);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(!types.contains(&"unit-stepped"));
    let allocations: Vec<_> = struck
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .collect();
    assert_eq!(allocations.len(), 1);
    assert_eq!(allocations[0].payload["amount"], 2);
    assert_eq!(allocations[0].payload["strikerInstanceId"], ally_id);
    assert_eq!(allocations[0].payload["targetInstanceId"], nearby_id);
    assert_eq!(
        struck.events[0].payload["instanceId"],
        cast["cardInstanceId"]
    );
    let after = state(session);
    assert_eq!(unit(&after, &ally_id)["location"], "C4");
    assert_eq!(unit(&after, &ally_id)["tapped"], false);
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == nearby_id)
    );
    assert_exact_replay(session);
}
