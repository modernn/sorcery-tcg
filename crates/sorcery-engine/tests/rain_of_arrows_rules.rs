//! Direct proofs for damage-each-aboveground-minion Magic (RULE-CATALOG-0605–0606,
//! RULE-CATALOG-1008, RULE-CATALOG-1094, RULE-CATALOG-1993–1998).
//!
//! Rain of Arrows simultaneously damages every aboveground minion. Ward absorbs
//! the damage without killing the minion. While Deathrites wait for ordering,
//! Rain of Arrows stays withheld until the chain drains.

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

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn rain() -> Value {
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

fn rain_manifest(seed: u32, ward: bool) -> String {
    let south_target = if ward {
        minion(json!({ "summonToAnySite": true, "ward": true }))
    } else {
        minion(json!({ "summonToAnySite": true }))
    };
    let fixture = if ward { "rain-ward" } else { "rain-lethal" };
    rain_manifest_with_target(seed, fixture, &south_target)
}

fn rain_deathrite_manifest(seed: u32) -> String {
    rain_manifest_with_target(
        seed,
        "rain-deathrite",
        &minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "summonToAnySite": true,
        })),
    )
}

fn rain_manifest_with_target(seed: u32, fixture: &str, south_target: &Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rain": rain(),
            "north-site": site(),
            "north-victim": minion(json!({})),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-target": south_target,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-rain", "north-victim", "north-rain", "north-victim", "north-rain", "north-victim"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-target"; 6],
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
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

fn try_setup_rain(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let (victim_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-victim"
            && descriptor["cell"] == "C4"
    })?;
    let victim_id = victim_summon["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (target_summon, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == "C1"
    })?;
    let target_id = target_summon["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some((session, victim_id, target_id))
}

fn setup_rain(encoded: &str) -> (Session, String, String) {
    try_setup_rain(encoded).expect("complete Rain of Arrows setup")
}

fn seed_with(ward: bool, start: u32) -> String {
    (start..start + 512)
        .map(|seed| rain_manifest(seed, ward))
        .find(|candidate| try_setup_rain(candidate).is_some())
        .expect("bounded seed with complete Rain of Arrows setup")
}

#[test]
fn rule_catalog_0605_rain_of_arrows_damages_every_aboveground_minion() {
    let encoded = seed_with(false, 605);
    let (mut session, victim_id, target_id) = setup_rain(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    assert!(event_types(&receipt).contains(&"damage-dealt"));
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    let after = state(&session);
    assert!(realm_unit(&after, &victim_id).is_none());
    assert!(realm_unit(&after, &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0606_rain_of_arrows_lets_ward_absorb_the_damage() {
    let encoded = seed_with(true, 606);
    let (mut session, victim_id, target_id) = setup_rain(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    assert!(event_types(&receipt).contains(&"ward-broken"));
    assert!(event_types(&receipt).contains(&"magic-resolved"));
    let after = state(&session);
    assert!(realm_unit(&after, &victim_id).is_none());
    assert_eq!(
        realm_unit(&after, &target_id).expect("ward survivor")["warded"],
        false
    );
    assert_eq!(
        realm_unit(&after, &target_id).expect("ward survivor")["damage"],
        0
    );
    assert_exact_replay(&session);
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn deathrite_seed(start: u32) -> String {
    (start..start + 512)
        .map(rain_deathrite_manifest)
        .find(|candidate| try_setup_rain(candidate).is_some())
        .expect("bounded seed with complete Rain of Arrows Deathrite setup")
}

#[test]
fn rule_catalog_1008_area_damage_deathrite_draws_for_minion_controller_on_kill() {
    let encoded = deathrite_seed(1008);
    let (mut session, _victim_id, target_id) = setup_rain(&encoded);
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");
    let receipt = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })
    .1;
    let types = event_types(&receipt);
    assert!(types.starts_with(&["magic-cast", "magic-damage-allocated"][..]));
    assert!(types.contains(&"damage-dealt"));
    let drawn_index = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("Deathrite site draw");
    let first_death_index = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion death");
    assert!(
        drawn_index < first_death_index,
        "Deathrite draw must resolve before corpses settle: {types:?}"
    );
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw event");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], target_id);
    assert!(types.contains(&"magic-resolved"));
    let finished = state(&session);
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

fn deathrite_order_minion() -> Value {
    minion(json!({
        "deathriteDrawSite": true,
        "defense": 1,
        "summonToAnySite": true,
    }))
}

fn visitor() -> Value {
    minion(json!({
        "defense": 3,
        "summonToAnySite": true,
    }))
}

fn deathrite_rain_manifest(seed: u32) -> String {
    let fixture = "rain-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rain": rain(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_order_minion(),
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
    }))
}

fn north_rain_count(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map_or(0, |hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-rain")
                .count()
        })
}

fn rain_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-rain"
        })
        .count()
}

struct PendingDeathriteRainSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteRainSetup> {
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
    if north_rain_count(&state(&session)) < 2 {
        return None;
    }
    if rain_casts(&session) < 2 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    if north_rain_count(&state(&session)) < 1 {
        return None;
    }
    realm_unit(&state(&session), &visitor_id)?;
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteRainSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_rain_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_rain_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Rain of Arrows in hand")
}

#[test]
fn rule_catalog_1094_rain_of_arrows_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_rain_seed_with(1094);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete Rain of Arrows Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
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
    assert_eq!(
        realm_unit(&paused, &visitor_id).expect("wounded visitor")["damage"],
        1
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert_eq!(rain_casts(session), 0);

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
    assert!(realm_unit(&resumed, &visitor_id).is_some());
    assert!(rain_casts(session) >= 1);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "magic-resolved"
        ]
    );
    assert_eq!(
        realm_unit(&state(session), &visitor_id).expect("surviving visitor")["damage"],
        2
    );
    assert_exact_replay(session);
}

fn survivor_minion() -> Value {
    minion(json!({
        "defense": 5,
        "summonToAnySite": true,
    }))
}

fn fragile_minion() -> Value {
    minion(json!({
        "defense": 1,
        "summonToAnySite": true,
    }))
}

fn burrower_minion() -> Value {
    minion(json!({
        "burrowing": true,
        "defense": 5,
        "summonToAnySite": true,
    }))
}

fn bury() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn rain_supplemental_manifest(seed: u32, fragile: bool) -> String {
    let south_target = if fragile {
        fragile_minion()
    } else {
        survivor_minion()
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rain-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rain-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-rain": rain(),
            "north-site": site(),
            "north-victim": survivor_minion(),
            "south-avatar": avatar(),
            "south-burrower": burrower_minion(),
            "south-site": site(),
            "south-target": south_target,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-rain", "north-victim", "north-rain", "north-victim", "north-bury",
                    "north-rain", "north-victim", "north-rain", "north-victim", "north-rain",
                    "north-victim", "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-target"; 10]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-burrower", 2))
                    .collect::<Vec<_>>(),
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

fn supplemental_seed_with_start(
    start: u32,
    required_south: usize,
    fragile: bool,
    summon_north_victim: bool,
) -> String {
    (start..start + 2048)
        .chain(605..605 + 2048)
        .find_map(|seed| {
            let encoded = rain_supplemental_manifest(seed, fragile);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-target")
                .count()
                < required_south
            {
                return None;
            }
            try_setup_rain_supplemental_prefix(&encoded, summon_north_victim).map(|_| encoded)
        })
        .expect("bounded seed with complete Rain supplemental setup")
}

fn rain_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-rain")
                .count()
        })
        .unwrap_or_default()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
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

fn damage_dealt_count(receipt: &Receipt) -> usize {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == "damage-dealt")
        .count()
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

fn advance_full_round(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn cast_rain(session: &mut Session) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    receipt
}

fn try_summon_north_victim_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-victim"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn summon_south_target_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("south target identity")
        .to_owned()
}

fn summon_south_burrower_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-burrower"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("south burrower identity")
        .to_owned()
}

fn try_summon_south_target_at(session: &mut Session, cell: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    })?;
    Some(summoned["cardInstanceId"].as_str()?.to_owned())
}

fn try_setup_rain_supplemental_prefix(
    encoded: &str,
    summon_north_victim: bool,
) -> Option<(Session, Option<String>, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let victim_id = if summon_north_victim {
        Some(try_summon_north_victim_at(&mut session, "C4")?)
    } else {
        None
    };
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let target_id = try_summon_south_target_at(&mut session, "C1")?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some((session, victim_id, target_id))
}

fn try_second_rain_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let (mut session, _, _first_id) = try_setup_rain_supplemental_prefix(encoded, true)?;
    cast_rain(&mut session);
    pass_turn_to_north_spellbook(&mut session);
    if rain_spells_in_hand(&state(&session)) < 1 {
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
    let minion_id = try_summon_south_target_at(&mut session, "C2")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })
    .then_some((session, minion_id))
}

fn seed_for_second_rain_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(605..605 + 8192)
        .find_map(|seed| {
            let encoded = rain_supplemental_manifest(seed, false);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-target")
                .count()
                < 2
            {
                return None;
            }
            try_second_rain_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Rain enemy-arrival setup")
}

fn try_second_rain_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let (mut session, _, first_id) = try_setup_rain_supplemental_prefix(encoded, true)?;
    cast_rain(&mut session);
    if realm_unit(&state(&session), &first_id).is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if rain_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = try_summon_south_target_at(&mut session, "C1")?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })
    .then_some((session, minion_id))
}

fn seed_for_second_rain_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(605..605 + 8192)
        .find_map(|seed| {
            let encoded = rain_supplemental_manifest(seed, true);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-target")
                .count()
                < 2
            {
                return None;
            }
            try_second_rain_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Rain new-summon setup")
}

fn try_burrowed_enemy_prefix(encoded: &str) -> Option<(Session, String, String, String)> {
    let (mut session, victim_id, surface_id) = try_setup_rain_supplemental_prefix(encoded, true)?;
    let victim_id = victim_id?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let burrower_id = summon_south_burrower_at(&mut session, "C3");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let bury_receipt = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["instanceId"] == burrower_id
    })?;
    if !event_types(&bury_receipt.1).contains(&"minion-burrowed") {
        return None;
    }
    if unit(&state(&session), &burrower_id)["region"] != "underground" {
        return None;
    }
    offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })
    .then_some((session, victim_id, surface_id, burrower_id))
}

fn seed_for_burrowed_enemy(start: u32) -> String {
    (start..start + 16384)
        .chain(605..605 + 16384)
        .find_map(|seed| {
            let encoded = rain_supplemental_manifest(seed, false);
            if opening_hand_spell_ids(&encoded, "north")
                .iter()
                .any(|card| card == "north-bury")
                && opening_hand_spell_ids(&encoded, "south")
                    .iter()
                    .any(|card| card == "south-burrower")
            {
                try_burrowed_enemy_prefix(&encoded).map(|_| encoded)
            } else {
                None
            }
        })
        .expect("bounded seed reaching Rain burrowed-enemy setup")
}

#[test]
fn rule_catalog_1993_rain_survivors_stay_on_the_board_after_turns_pass() {
    let encoded = supplemental_seed_with_start(1993, 1, false, true);
    let (mut session, victim_id, target_id) =
        try_setup_rain_supplemental_prefix(&encoded, true).expect("Rain supplemental prefix");
    let victim_id = victim_id.expect("north victim");
    cast_rain(&mut session);
    let victim_location = unit(&state(&session), &victim_id)["location"]
        .as_str()
        .expect("victim location")
        .to_owned();
    let target_location = unit(&state(&session), &target_id)["location"]
        .as_str()
        .expect("target location")
        .to_owned();
    assert_eq!(unit(&state(&session), &victim_id)["damage"], 1);
    assert_eq!(unit(&state(&session), &target_id)["damage"], 1);
    advance_full_round(&mut session);
    assert!(realm_unit(&state(&session), &victim_id).is_some());
    assert!(realm_unit(&state(&session), &target_id).is_some());
    assert_eq!(
        unit(&state(&session), &victim_id)["location"],
        victim_location
    );
    assert_eq!(
        unit(&state(&session), &target_id)["location"],
        target_location
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1994_second_rain_without_aboveground_minions_is_a_paid_noop() {
    let encoded = (1994..1994 + 8192)
        .chain(605..605 + 8192)
        .find_map(|seed| {
            let candidate = rain_supplemental_manifest(seed, true);
            let (mut session, _, target_id) =
                try_setup_rain_supplemental_prefix(&candidate, false)?;
            if rain_spells_in_hand(&state(&session)) < 2 {
                return None;
            }
            let first = cast_rain(&mut session);
            if !event_types(&first).contains(&"minion-died") {
                return None;
            }
            if realm_unit(&state(&session), &target_id).is_some() {
                return None;
            }
            (rain_spells_in_hand(&state(&session)) >= 1).then_some(candidate)
        })
        .expect("bounded seed with two Rain casts after clearing aboveground minions");
    let (mut session, _, target_id) =
        try_setup_rain_supplemental_prefix(&encoded, false).expect("Rain supplemental prefix");
    let first = cast_rain(&mut session);
    assert!(event_types(&first).contains(&"minion-died"));
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert!(rain_spells_in_hand(&state(&session)) >= 1);
    let second = cast_rain(&mut session);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!event_types(&second).contains(&"damage-dealt"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1995_second_rain_damages_a_newly_arrived_enemy_after_enemy_site_placement() {
    let encoded = seed_for_second_rain_enemy_arrival(1995);
    let (mut session, minion_id) =
        try_second_rain_enemy_arrival_prefix(&encoded).expect("second Rain enemy-arrival prefix");
    let receipt = cast_rain(&mut session);
    assert_eq!(damage_dealt_amount(&receipt, &minion_id), 1);
    assert_eq!(unit(&state(&session), &minion_id)["damage"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1996_rain_damages_every_aboveground_minion() {
    let encoded = supplemental_seed_with_start(1996, 2, false, true);
    let (mut session, victim_id, target_id) =
        try_setup_rain_supplemental_prefix(&encoded, true).expect("Rain supplemental prefix");
    let victim_id = victim_id.expect("north victim");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let third_id = summon_south_target_at(&mut session, "C3");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let receipt = cast_rain(&mut session);
    assert_eq!(damage_dealt_count(&receipt), 3);
    assert_eq!(damage_dealt_amount(&receipt, &victim_id), 1);
    assert_eq!(damage_dealt_amount(&receipt, &target_id), 1);
    assert_eq!(damage_dealt_amount(&receipt, &third_id), 1);
    assert_eq!(unit(&state(&session), &victim_id)["damage"], 1);
    assert_eq!(unit(&state(&session), &target_id)["damage"], 1);
    assert_eq!(unit(&state(&session), &third_id)["damage"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1997_rain_leaves_an_underground_minion_untouched() {
    let encoded = seed_for_burrowed_enemy(1997);
    let (mut session, victim_id, surface_id, burrower_id) =
        try_burrowed_enemy_prefix(&encoded).expect("Rain burrowed-enemy prefix");
    let receipt = cast_rain(&mut session);
    assert_eq!(damage_dealt_amount(&receipt, &victim_id), 1);
    assert_eq!(damage_dealt_amount(&receipt, &surface_id), 1);
    assert!(!receipt.events.iter().any(|event| {
        event.event_type == "damage-dealt" && event.payload["instanceId"] == burrower_id
    }));
    assert_eq!(unit(&state(&session), &burrower_id)["damage"], 0);
    assert_eq!(
        unit(&state(&session), &burrower_id)["region"],
        "underground"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1998_second_rain_kills_a_newly_summoned_enemy() {
    let encoded = seed_for_second_rain_new_summon(1998);
    let (mut session, minion_id) =
        try_second_rain_new_summon_prefix(&encoded).expect("second Rain new-summon prefix");
    let receipt = cast_rain(&mut session);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert_eq!(damage_dealt_amount(&receipt, &minion_id), 1);
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(
        state(&session)["players"]["south"]["cemetery"]
            .as_array()
            .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == minion_id))
    );
    assert_exact_replay(&session);
}
