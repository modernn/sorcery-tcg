//! Direct proofs for burrow-all-minions-and-artifacts-at-target-land-site
//! Magic (RULE-CATALOG-0587–0588, RULE-CATALOG-1078).
//!
//! Ordinary Magic offers each surface land site (or rubble) and burrows
//! every surface minion there in canonical instance-id order. A water-only
//! site has no Underground layer, so it is not offered. While Deathrites
//! wait for ordering, Cave-In Magic stays withheld until the chain drains.

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

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["water"],
    })
}

fn burrower() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn cave_in() -> Value {
    json!({
        "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
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
        "burrowing": true,
        "cardType": "minion",
        "defense": 3,
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

fn cave_in_manifest(seed: u32, water_south: bool) -> String {
    let south_site = if water_south {
        water_site()
    } else {
        earth_site()
    };
    let fixture = if water_south {
        "cave-in-water-site"
    } else {
        "cave-in-land-site"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": burrower(),
            "south-site": south_site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 6],
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
    let mut session = Session::new(encoded).expect("valid Cave-In session");
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

fn seed_with(water_south: bool, start: u32, required_south: usize) -> String {
    (start..start + 256)
        .map(|seed| cave_in_manifest(seed, water_south))
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-cave-in")
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Cave-In and required South minions")
}

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Cave-In occupant identity")
        .to_owned()
}

fn summon_south_at_c1(session: &mut Session) -> String {
    summon_south_at(session, "C1")
}

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn cave_in_casts(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("Cave-In actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-cave-in"
        })
        .map(|action| action.descriptor)
        .collect()
}

fn cave_in_cells(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = cave_in_casts(session)
        .into_iter()
        .filter_map(|cast| {
            cast["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

fn deathrite_cave_in_manifest(seed: u32) -> String {
    let fixture = "cave-in-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
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
                    "north-cave-in",
                    "north-rain",
                    "north-rain",
                    "north-cave-in",
                    "north-rain",
                    "north-cave-in",
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

fn north_has_cave_in_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-cave-in", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteCaveInSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_ready_visitor(encoded: &str) -> Option<PendingDeathriteCaveInSetup> {
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
            && descriptor["region"].is_null()
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
    if !north_has_cave_in_and_rain(&state(&session)) {
        return None;
    }
    if cave_in_cells(&session).is_empty() {
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
    Some(PendingDeathriteCaveInSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_cave_in_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cave_in_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Cave-In Magic in hand")
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
fn rule_catalog_0587_cave_in_burrows_minions_at_land_site_in_canonical_order() {
    let encoded = seed_with(false, 587, 2);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    let first_id = summon_south_at_c1(&mut session);
    let second_id = summon_south_at_c1(&mut session);
    north_draws_spellbook(&mut session);

    let land_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("Land Site identity")
        .to_owned();
    let offered = cave_in_casts(&session);
    assert!(offered.iter().any(|cast| {
        cast["targetLocation"]["cell"] == "C1" && cast["targetSiteInstanceId"] == land_site_id
    }));
    assert!(
        offered
            .iter()
            .all(|cast| { cast["targetLocation"]["region"] == "surface" })
    );

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == land_site_id
    });
    let mut burrowed_ids = vec![first_id.clone(), second_id.clone()];
    burrowed_ids.sort();
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-burrowed",
            "minion-burrowed",
            "magic-resolved",
        ]
    );
    assert_eq!(
        receipt.events[0].payload["targetLocation"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert_eq!(
        receipt.events[0].payload["targetSiteInstanceId"],
        land_site_id
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "minion-burrowed")
            .map(|event| {
                assert_eq!(event.payload["cell"], "C1");
                assert_eq!(event.payload["seat"], "south");
                assert_eq!(event.payload["sourceInstanceId"], cast["cardInstanceId"]);
                event.payload["instanceId"]
                    .as_str()
                    .expect("burrowed identity")
                    .to_owned()
            })
            .collect::<Vec<_>>(),
        burrowed_ids
    );
    let after = state(&session);
    assert_eq!(unit(&after, &first_id)["region"], "underground");
    assert_eq!(unit(&after, &second_id)["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0588_cave_in_does_not_offer_a_water_only_site() {
    let encoded = seed_with(true, 588, 0);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    north_draws_spellbook(&mut session);

    let cells = cave_in_cells(&session);
    assert!(cells.contains(&"C4".to_owned()));
    assert!(!cells.contains(&"C1".to_owned()));
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["cardId"],
        "south-site"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1078_cave_in_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cave_in_seed_with(1078);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete Cave-In Deathrite withheld setup");
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
    assert_eq!(unit(&paused, &visitor_id)["region"], "surface");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(cave_in_cells(session).is_empty());

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
    assert_eq!(unit(&resumed, &visitor_id)["region"], "surface");
    let land_site_id = resumed["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("Land Site identity")
        .to_owned();
    assert!(cave_in_cells(session).contains(&"C4".to_owned()));

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == land_site_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-burrowed", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["cell"], "C4");
    assert_eq!(receipt.events[1].payload["instanceId"], visitor_id);
    assert_eq!(receipt.events[1].payload["seat"], "south");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(unit(&state(session), &visitor_id)["region"], "underground");
    assert_exact_replay(session);
}

fn cave_in_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": burrower(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 8],
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

fn seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(587..587 + 2048)
        .map(cave_in_supplemental_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-cave-in")
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Cave-In and required South minions")
}

fn cave_in_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-cave-in")
                .count()
        })
        .unwrap_or_default()
}

fn is_underground(snapshot: &Value, instance_id: &str) -> bool {
    unit(snapshot, instance_id)["region"] == "underground"
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

fn cast_cave_in_at(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
}

fn setup_c1_with_minions(session: &mut Session, count: usize) -> Vec<String> {
    south_plays_c1(session);
    (0..count).map(|_| summon_south_at_c1(session)).collect()
}

fn try_second_cave_in_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    setup_c1_with_minions(&mut session, 1);
    north_draws_spellbook(&mut session);
    cast_cave_in_at(&mut session, "C1");
    pass_turn_to_north_spellbook(&mut session);
    if cave_in_spells_in_hand(&state(&session)) < 1 {
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
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C2"
            && descriptor["region"].is_null()
    })?;
    let minion_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    cave_in_cells(&session)
        .contains(&"C2".to_owned())
        .then_some((session, minion_id))
}

fn seed_for_second_cave_in_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(587..587 + 8192)
        .find_map(|seed| {
            let encoded = cave_in_supplemental_manifest(seed);
            if opening_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_cave_in_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Cave-In enemy-arrival setup")
}

fn try_second_cave_in_new_site_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    setup_c1_with_minions(&mut session, 1);
    north_draws_spellbook(&mut session);
    cast_cave_in_at(&mut session, "C1");
    pass_turn_to_north_spellbook(&mut session);
    if cave_in_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let new_cell = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find_map(|action| {
            (action.descriptor["kind"] == "play-site"
                && action.descriptor["cardId"] == "north-site"
                && action.descriptor["cell"] != "C4")
                .then(|| action.descriptor["cell"].as_str().map(ToOwned::to_owned))?
        })?;
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == new_cell
    });
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == new_cell
            && descriptor["region"].is_null()
    })?;
    let minion_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    cave_in_cells(&session)
        .contains(&new_cell)
        .then_some((session, minion_id, new_cell))
}

fn seed_for_second_cave_in_new_site(start: u32) -> String {
    (start..start + 8192)
        .chain(587..587 + 8192)
        .find_map(|seed| {
            let encoded = cave_in_supplemental_manifest(seed);
            if opening_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_cave_in_new_site_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Cave-In new-site setup")
}

#[test]
fn rule_catalog_1903_burrowed_minions_stay_underground_after_turns_pass() {
    let encoded = seed_with_start(1903, 1);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    cast_cave_in_at(&mut session, "C1");
    assert!(is_underground(&state(&session), &minion_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(is_underground(&state(&session), &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1904_second_cave_in_without_surface_minions_is_a_paid_noop() {
    let encoded = seed_with_start(1904, 1);
    let mut session = opening_main(&encoded);
    setup_c1_with_minions(&mut session, 1);
    north_draws_spellbook(&mut session);
    cast_cave_in_at(&mut session, "C1");
    assert!(cave_in_spells_in_hand(&state(&session)) >= 1);
    let receipt = cast_cave_in_at(&mut session, "C1");
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-burrowed")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1905_second_cave_in_burrows_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_cave_in_enemy_arrival(1905);
    let (mut session, minion_id) = try_second_cave_in_enemy_arrival_prefix(&encoded)
        .expect("second Cave-In enemy-arrival prefix");
    let receipt = cast_cave_in_at(&mut session, "C2");
    assert!(event_types(&receipt).contains(&"minion-burrowed"));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-burrowed" && event.payload["instanceId"] == minion_id
    }));
    assert!(is_underground(&state(&session), &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1906_cave_in_burrows_every_minion_sharing_the_target_land_site() {
    let encoded = seed_with_start(1906, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c1_with_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let receipt = cast_cave_in_at(&mut session, "C1");
    let burrowed: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-burrowed")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("burrowed identity")
                .to_owned()
        })
        .collect();
    assert_eq!(burrowed.len(), 2);
    for minion_id in &minion_ids {
        assert!(burrowed.contains(minion_id));
        assert!(is_underground(&state(&session), minion_id));
    }
    assert_exact_replay(&session);
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = opening_main(encoded);
    let c1_ids = setup_c1_with_minions(&mut session, 2);
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
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let far_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (cave_in_cells(&session).contains(&"C1".to_owned())).then_some((session, c1_ids, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(587..587 + 2048)
        .find_map(|seed| {
            let encoded = cave_in_supplemental_manifest(seed);
            if opening_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching Cave-In far-minion setup")
}

#[test]
fn rule_catalog_1907_cave_in_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(1907);
    let (mut session, c1_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("Cave-In far-minion prefix");
    cast_cave_in_at(&mut session, "C1");
    for minion_id in &c1_ids {
        assert!(is_underground(&state(&session), minion_id));
    }
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1908_second_cave_in_burrows_minions_at_a_newly_placed_land_site() {
    let encoded = seed_for_second_cave_in_new_site(1908);
    let (mut session, minion_id, new_cell) =
        try_second_cave_in_new_site_prefix(&encoded).expect("second Cave-In new-site prefix");
    let receipt = cast_cave_in_at(&mut session, &new_cell);
    assert!(event_types(&receipt).contains(&"minion-burrowed"));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-burrowed" && event.payload["instanceId"] == minion_id
    }));
    assert!(is_underground(&state(&session), &minion_id));
    assert_exact_replay(&session);
}
