//! Direct proofs for damage-each-unit-at-location-within-two-steps Magic
//! (RULE-CATALOG-0589–0590, RULE-CATALOG-0716, 1039, 1099).
//!
//! Ordinary Magic offers existing locations within two measured cardinal steps
//! of the caster footprint and deals 3 damage to every Unit there. Ward
//! absorbs. Location offers are not unit targets, so Stealth does not filter
//! them. An empty offered location is a paid no-op.
//!
//! 1039 covers minor explosion killing a Deathrite minion: the controller draws
//! a site and magic-resolved only appears after deathrite settlement. 1099
//! covers minor explosion withheld while Deathrites wait for ordering, until
//! the chain drains.

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

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn warded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
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

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn visitor() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 5,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn explosion() -> Value {
    json!({
        "cardType": "magic",
        "damageEachUnitAtLocationWithinTwoSteps": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn explosion_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "minor-explosion-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-minor-explosion-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-explosion": explosion(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-explosion"; 6],
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

fn explosion_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "minor-explosion" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-minor-explosion-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-explosion": explosion(),
            "north-raider": raider(),
            "north-site": earth_site(),
            "north-warded": warded(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-raider",
                    "north-warded",
                    "north-explosion",
                    "north-raider",
                    "north-warded",
                    "north-explosion",
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
    let mut session = Session::new(encoded).expect("valid minor-explosion session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn explosion_locations(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("explosion actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-explosion"
        })
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

fn seed_with(required: &[&str]) -> String {
    (589..589 + 256)
        .map(explosion_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn seed_with_deathrite(start: u32) -> String {
    (start..start + 256)
        .map(explosion_deathrite_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-explosion")
        })
        .expect("bounded seed with minor explosion Deathrite setup")
}

fn one_south_deathrite_at_c3(session: &mut Session) -> String {
    south_ends_after_c1(session);
    play_c3(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned minion identity")
        .to_owned()
}

fn summon_at(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned instance identity")
        .to_owned()
}

fn south_ends_after_c1(session: &mut Session) {
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

fn play_c3(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
}

fn damage_dealt_amount(receipt: &Receipt, instance_id: &str) -> u64 {
    receipt
        .events
        .iter()
        .find(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == instance_id
        })
        .expect("damage-dealt")
        .payload["amount"]
        .as_u64()
        .expect("damage amount")
}

fn allocated_targets(receipt: &Receipt) -> Vec<(String, u64)> {
    let mut targets: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "magic-damage-allocated")
        .map(|event| {
            (
                event.payload["targetInstanceId"]
                    .as_str()
                    .expect("allocated target")
                    .to_owned(),
                event.payload["amount"].as_u64().expect("allocated amount"),
            )
        })
        .collect();
    targets.sort();
    targets
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
fn rule_catalog_0589_explosion_damages_each_unit_at_location_and_ward_absorbs() {
    let encoded = seed_with(&["north-explosion", "north-raider", "north-warded"]);
    let mut session = opening_main(&encoded);
    south_ends_after_c1(&mut session);
    play_c3(&mut session);
    let raider_id = summon_at(&mut session, "north-raider", "C3");
    let warded_id = summon_at(&mut session, "north-warded", "C3");

    let before = state(&session);
    assert_eq!(unit(&before, &raider_id)["location"], "C3");
    assert_eq!(unit(&before, &raider_id)["damage"], 0);
    assert_eq!(unit(&before, &warded_id)["location"], "C3");
    assert_eq!(unit(&before, &warded_id)["damage"], 0);
    assert_eq!(unit(&before, &warded_id)["warded"], true);
    assert_eq!(explosion_locations(&session), ["C3", "C4"]);
    assert!(!explosion_locations(&session).contains(&"C1".to_owned()));

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-explosion"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    let types = event_types(&receipt);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    let mut expected = vec![(raider_id.clone(), 3), (warded_id.clone(), 3)];
    expected.sort();
    assert_eq!(allocated_targets(&receipt), expected);
    assert_eq!(damage_dealt_amount(&receipt, &raider_id), 3);
    assert_eq!(damage_dealt_amount(&receipt, &warded_id), 0);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken"
                && event.payload["instanceId"] == warded_id)
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed"
                || event.event_type == "minion-died"
                || event.event_type == "stealth-lost")
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .find(|event| event.event_type == "magic-damage-allocated")
            .expect("allocation")
            .payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );

    let after = state(&session);
    assert_eq!(unit(&after, &raider_id)["location"], "C3");
    assert_eq!(unit(&after, &raider_id)["damage"], 3);
    assert_eq!(unit(&after, &warded_id)["location"], "C3");
    assert_eq!(unit(&after, &warded_id)["damage"], 0);
    assert_eq!(unit(&after, &warded_id)["warded"], false);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0590_empty_offered_location_is_a_paid_noop() {
    let encoded = seed_with(&["north-explosion"]);
    let mut session = opening_main(&encoded);
    south_ends_after_c1(&mut session);
    play_c3(&mut session);
    assert_eq!(explosion_locations(&session), ["C3", "C4"]);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-explosion"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "magic-damage-allocated"
                || event.event_type == "damage-dealt"
                || event.event_type == "minion-killed"
                || event.event_type == "minion-died")
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0716_minor_explosion_measures_connected_locations_and_hits_only_target_region() {
    sorcery_engine::game::catalog_proofs::rule_catalog_0716_minor_explosion_measures_connected_locations_and_hits_only_target_region();
}

#[test]
fn rule_catalog_1039_minor_explosion_deathrite_draws_for_controller_on_kill() {
    let encoded = seed_with_deathrite(1039);
    let mut session = opening_main(&encoded);
    let target_id = one_south_deathrite_at_c3(&mut session);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-explosion"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(receipt.events[1].payload["targetInstanceId"], target_id);
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], target_id);
    let types = event_types(&receipt);
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
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        damage_dealt < site_drawn && site_drawn < minion_died && minion_died < magic_resolved,
        "expected damage-dealt, deathrite site-drawn, minion-died, then magic-resolved; got {types:?}"
    );
    assert_eq!(types.last(), Some(&"magic-resolved"));

    let finished = state(&session);
    assert!(realm_unit(&finished, &target_id).is_none());
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}

fn deathrite_explosion_manifest(seed: u32) -> String {
    let fixture = "minor-explosion-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-explosion": explosion(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-explosion",
                    "north-rain",
                    "north-rain",
                    "north-explosion",
                    "north-rain",
                    "north-explosion",
                ],
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
    }))
}

fn north_has_explosion_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-explosion", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteExplosionSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(
    encoded: &str,
) -> Option<PendingDeathriteExplosionSetup> {
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
    let visitor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    })?;
    let visitor_id = visitor.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_explosion_and_rain(&state(&session)) {
        return None;
    }
    if explosion_locations(&session) != ["C4"] {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    realm_unit(&state(&session), &visitor_id)?;
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteExplosionSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_explosion_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_explosion_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with minor explosion Magic in hand")
}

#[test]
fn rule_catalog_1099_minor_explosion_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_explosion_seed_with(1099);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete minor explosion Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
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
    assert_eq!(unit(&paused, &visitor_id)["location"], "C4");
    assert_eq!(unit(&paused, &visitor_id)["damage"], 1);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(explosion_locations(session).is_empty());

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
    assert_eq!(unit(&resumed, &visitor_id)["location"], "C4");
    assert_eq!(unit(&resumed, &visitor_id)["damage"], 1);
    assert_eq!(explosion_locations(session), ["C4"]);
    let avatar_id = resumed["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar")
        .to_owned();

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-explosion"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    let types = event_types(&receipt);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    let mut expected = vec![(avatar_id.clone(), 3), (visitor_id.clone(), 3)];
    expected.sort();
    assert_eq!(allocated_targets(&receipt), expected);
    assert_eq!(damage_dealt_amount(&receipt, &visitor_id), 3);
    assert_eq!(damage_dealt_amount(&receipt, &avatar_id), 3);
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "avatar-life-lost"
            && event.payload["seat"] == "north"
            && event.payload["amount"] == 3
    }));
    let after = state(session);
    assert_eq!(unit(&after, &visitor_id)["damage"], 4);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 17);
    assert_exact_replay(session);
}

fn supplemental_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn fragile_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn explosion_supplemental_manifest(seed: u32, fragile: bool) -> String {
    let south_minion = if fragile {
        fragile_minion()
    } else {
        supplemental_minion()
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "minor-explosion-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-minor-explosion-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-explosion": explosion(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": south_minion,
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-explosion"; 8],
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

fn opening_hand_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
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

fn seed_with_start(start: u32, required_south: usize, fragile: bool) -> String {
    (start..start + 2048)
        .chain(589..589 + 2048)
        .map(|seed| explosion_supplemental_manifest(seed, fragile))
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-explosion")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with minor explosion and required South minions")
}

fn explosion_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-explosion")
                .count()
        })
        .unwrap_or_default()
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

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    summon_at(session, "south-minion", cell)
}

fn setup_c3_with_south_minions(session: &mut Session, count: usize) -> Vec<String> {
    south_ends_after_c1(session);
    play_c3(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (0..count).map(|_| summon_south_at(session, "C3")).collect()
}

fn cast_explosion_at(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-explosion"
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = opening_main(encoded);
    let c3_ids = setup_c3_with_south_minions(&mut session, 2);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let far_id = summon_south_at(&mut session, "C4");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    explosion_locations(&session)
        .contains(&"C3".to_owned())
        .then_some((session, c3_ids, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(589..589 + 2048)
        .find_map(|seed| {
            let encoded = explosion_supplemental_manifest(seed, false);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching minor explosion far-minion setup")
}

fn try_second_explosion_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    setup_c3_with_south_minions(&mut session, 1);
    north_draws_spellbook(&mut session);
    cast_explosion_at(&mut session, "C3");
    pass_turn_to_north_spellbook(&mut session);
    if explosion_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let minion_id = summon_south_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    explosion_locations(&session)
        .contains(&"C2".to_owned())
        .then_some((session, minion_id))
}

fn seed_for_second_explosion_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(589..589 + 8192)
        .find_map(|seed| {
            let encoded = explosion_supplemental_manifest(seed, false);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_explosion_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second minor explosion enemy-arrival setup")
}

fn try_second_explosion_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c3_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_explosion_at(&mut session, "C3");
    if realm_unit(&state(&session), &first_id).is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if explosion_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C3");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    explosion_locations(&session)
        .contains(&"C3".to_owned())
        .then_some((session, minion_id))
}

fn seed_for_second_explosion_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(589..589 + 8192)
        .find_map(|seed| {
            let encoded = explosion_supplemental_manifest(seed, true);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_explosion_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second minor explosion new-summon setup")
}

#[test]
fn rule_catalog_1923_damaged_minion_stays_at_the_location_after_turns_pass() {
    let encoded = seed_with_start(1923, 1, false);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c3_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_explosion_at(&mut session, "C3");
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 3);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(unit(&state(&session), &minion_id)["location"], "C3");
    assert!(realm_unit(&state(&session), &minion_id).is_some());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1924_second_explosion_at_an_empty_location_is_a_paid_noop() {
    let encoded = seed_with_start(1924, 1, true);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c3_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_explosion_at(&mut session, "C3");
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(explosion_spells_in_hand(&state(&session)) >= 1);
    let receipt = cast_explosion_at(&mut session, "C3");
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "magic-damage-allocated")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1925_second_explosion_damages_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_explosion_enemy_arrival(1925);
    let (mut session, minion_id) = try_second_explosion_enemy_arrival_prefix(&encoded)
        .expect("second minor explosion enemy-arrival prefix");
    let receipt = cast_explosion_at(&mut session, "C2");
    assert_eq!(damage_dealt_amount(&receipt, &minion_id), 3);
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1926_explosion_damages_every_unit_at_the_target_location() {
    let encoded = seed_with_start(1926, 2, false);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c3_with_south_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let receipt = cast_explosion_at(&mut session, "C3");
    let mut expected: Vec<_> = minion_ids.iter().map(|id| (id.clone(), 3)).collect();
    expected.sort();
    assert_eq!(allocated_targets(&receipt), expected);
    for minion_id in &minion_ids {
        assert_eq!(unit(&state(&session), minion_id)["damage"], 3);
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1927_explosion_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(1927);
    let (mut session, c3_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("minor explosion far-minion prefix");
    let receipt = cast_explosion_at(&mut session, "C3");
    assert_eq!(damage_dealt_amount(&receipt, &c3_ids[0]), 3);
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1928_second_explosion_damages_a_newly_summoned_minion() {
    let encoded = seed_for_second_explosion_new_summon(1928);
    let (mut session, minion_id) = try_second_explosion_new_summon_prefix(&encoded)
        .expect("second minor explosion new-summon prefix");
    let receipt = cast_explosion_at(&mut session, "C3");
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert_eq!(damage_dealt_amount(&receipt, &minion_id), 3);
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == minion_id))
    );
    assert_exact_replay(&session);
}
