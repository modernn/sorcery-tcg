//! Direct proofs for destroy-minions-at-water-site-within-two-steps Magic
//! (RULE-CATALOG-0571–0572, 1028, 1092).
//!
//! 1028 covers boil killing a Deathrite minion: the controller draws a site
//! and magic-resolved only appears after deathrite settlement. 1092 covers
//! Boil withheld while Deathrites wait for ordering, until the chain drains.
//!
//! Ordinary Magic offers Water sites within two measured cardinal steps of
//! the caster footprint and kills every minion occupying the chosen site.
//! Earth sites are not offered. An empty qualifying Water site is a paid
//! no-op. This is one composed fact, exclusive of area damage and
//! kill-mortal-here.

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

fn mortal() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "mortal": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn beast() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn boil() -> Value {
    json!({
        "cardType": "magic",
        "destroyMinionsAtWaterSiteWithinTwoSteps": true,
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

fn rain_deathrite_minion() -> Value {
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

fn boil_deathrite_manifest(seed: u32) -> String {
    boil_manifest_with_minion(seed, &deathrite_minion())
}

fn boil_manifest(seed: u32) -> String {
    boil_manifest_with_minion(seed, &mortal())
}

fn boil_manifest_with_minion(seed: u32, north_minion: &Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-water-site-minions" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-water-site-minions-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-boil": boil(),
            "north-earth": earth_site(),
            "north-mortal": north_minion.clone(),
            "north-water": water_site(),
            "south-avatar": avatar(),
            "south-mortal": mortal(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-water",
                    "north-earth",
                    "north-earth",
                    "north-water",
                    "north-earth",
                ],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-mortal",
                    "north-beast",
                    "north-boil",
                    "north-mortal",
                    "north-beast",
                    "north-boil",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-mortal"; 6],
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
    let mut session = Session::new(encoded).expect("valid destroy-water-site-minions session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
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

fn opening_atlas_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("opening Atlas hand")
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

fn boil_locations(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("boil actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-boil"
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

fn seed_with(required_spells: &[&str], require_earth: bool) -> String {
    (571..571 + 256)
        .map(boil_manifest)
        .find(|candidate| {
            let spells = opening_spell_ids(candidate);
            let atlas = opening_atlas_ids(candidate);
            required_spells
                .iter()
                .all(|id| spells.iter().any(|card| card == id))
                && atlas.iter().any(|card| card == "north-water")
                && (!require_earth || atlas.iter().any(|card| card == "north-earth"))
        })
        .expect("bounded seed with required opening cards")
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

fn south_plays_c1_and_summons(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let far_id = summon_at(session, "south-mortal", "C1");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    far_id
}

fn seed_with_deathrite(required: &[&str]) -> String {
    (571..571 + 256)
        .map(boil_deathrite_manifest)
        .find(|candidate| {
            let spells = opening_spell_ids(candidate);
            required
                .iter()
                .all(|id| spells.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn deathrite_boil_manifest(seed: u32) -> String {
    let fixture = "boil-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-boil": boil(),
            "north-mortal": mortal(),
            "north-rain": rain_spell(),
            "north-water": water_site(),
            "south-avatar": avatar(),
            "south-minion": rain_deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-water"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-mortal",
                    "north-boil",
                    "north-rain",
                    "north-rain",
                    "north-boil",
                    "north-boil",
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

fn north_has_boil_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-boil", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteBoilSetup {
    deathrite_ids: [String; 2],
    occupant_id: String,
    session: Session,
}

fn try_pending_deathrite_with_ready_occupant(encoded: &str) -> Option<PendingDeathriteBoilSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
    })?;
    let occupant = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-mortal"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let occupant_id = occupant.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_boil_and_rain(&state(&session)) {
        return None;
    }
    if boil_locations(&session) != ["C4"] {
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
    Some(PendingDeathriteBoilSetup {
        deathrite_ids,
        occupant_id,
        session,
    })
}

fn deathrite_boil_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_boil_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_occupant(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Boil Magic in hand")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
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
fn rule_catalog_0571_boil_kills_all_minions_at_water_site_and_spares_far_mortal() {
    let encoded = seed_with(&["north-mortal", "north-beast", "north-boil"], true);
    let mut session = opening_main(&encoded);
    let mortal_id = summon_at(&mut session, "north-mortal", "C4");
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    let far_id = south_plays_c1_and_summons(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-earth"
            && descriptor["cell"] == "C3"
    });

    let before = state(&session);
    assert_eq!(unit(&before, &mortal_id)["location"], "C4");
    assert_eq!(unit(&before, &beast_id)["location"], "C4");
    assert_eq!(unit(&before, &beast_id)["damage"], 0);
    assert_eq!(unit(&before, &far_id)["location"], "C1");
    assert_eq!(boil_locations(&session), ["C4"]);
    assert!(!boil_locations(&session).contains(&"C3".to_owned()));
    assert!(!boil_locations(&session).contains(&"C1".to_owned()));

    let (cast, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-boil"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    let types = event_types(&killed);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert_eq!(
        types
            .iter()
            .filter(|event| **event == "minion-killed")
            .count(),
        2
    );
    assert_eq!(
        types
            .iter()
            .filter(|event| **event == "minion-died")
            .count(),
        2
    );
    assert!(
        !killed
            .events
            .iter()
            .any(|event| event.event_type == "damage-dealt")
    );
    let killed_ids: Vec<_> = killed
        .events
        .iter()
        .filter(|event| event.event_type == "minion-killed")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("killed instance")
                .to_owned()
        })
        .collect();
    assert!(killed_ids.contains(&mortal_id));
    assert!(killed_ids.contains(&beast_id));
    for event in killed
        .events
        .iter()
        .filter(|event| event.event_type == "minion-killed")
    {
        assert_eq!(event.payload["owner"], "north");
        assert_eq!(event.payload["seat"], "north");
        assert_eq!(event.payload["sourceInstanceId"], cast["cardInstanceId"]);
    }

    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != mortal_id && unit["instanceId"] != beast_id)
    );
    assert_eq!(unit(&after, &far_id)["location"], "C1");
    assert_eq!(unit(&after, &far_id)["damage"], 0);
    assert!(cemetery_has(&after, "north", &mortal_id));
    assert!(cemetery_has(&after, "north", &beast_id));
    assert!(!cemetery_has(&after, "south", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0572_empty_water_site_is_a_paid_noop() {
    let encoded = seed_with(&["north-boil"], false);
    let mut session = opening_main(&encoded);
    assert_eq!(boil_locations(&session), ["C4"]);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-boil"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert!(
        !resolved
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed" || event.event_type == "minion-died")
    );
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1028_destroy_water_site_minions_deathrite_draws_for_controller_on_kill() {
    let encoded = (1028..1028 + 256)
        .map(boil_deathrite_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|card| card == "north-mortal")
                && hand.iter().any(|card| card == "north-boil")
        })
        .unwrap_or_else(|| seed_with_deathrite(&["north-mortal", "north-boil"]));
    let mut session = opening_main(&encoded);
    let minion_id = summon_at(&mut session, "north-mortal", "C4");
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (_, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-boil"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "minion-killed",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    let kill = killed
        .events
        .iter()
        .find(|event| event.event_type == "minion-killed")
        .expect("minion-killed");
    assert_eq!(kill.payload["instanceId"], minion_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");
    assert_eq!(drawn.payload["sourceInstanceId"], minion_id);
    let site_drawn = event_types(&killed)
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let magic_resolved = event_types(&killed)
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        site_drawn < magic_resolved,
        "magic-resolved must follow deathrite site-drawn"
    );
    assert_eq!(event_types(&killed).last(), Some(&"magic-resolved"));

    let after = state(&session);
    assert!(cemetery_has(&after, "north", &minion_id));
    assert_eq!(atlas_len(&after, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&after, "south"), south_atlas);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1092_destroy_water_site_minions_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_boil_seed_with(1092);
    let mut setup = try_pending_deathrite_with_ready_occupant(&encoded)
        .expect("complete Boil Deathrite withheld setup");
    let occupant_id = setup.occupant_id.clone();
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
    assert_eq!(unit(&paused, &occupant_id)["location"], "C4");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(boil_locations(session).is_empty());

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
    assert_eq!(unit(&resumed, &occupant_id)["location"], "C4");
    assert_eq!(boil_locations(session), ["C4"]);

    let (cast, killed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-boil"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["instanceId"], occupant_id);
    assert_eq!(killed.events[1].payload["owner"], "north");
    assert_eq!(killed.events[1].payload["seat"], "north");
    assert_eq!(
        killed.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(session);
    assert!(cemetery_has(&after, "north", &occupant_id));
    assert_exact_replay(session);
}
