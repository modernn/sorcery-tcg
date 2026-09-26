//! Direct proofs for return-target-site-to-owner-hand Magic (RULE-CATALOG-0625–0626,
//! RULE-CATALOG-1047, RULE-CATALOG-2093–2098).
//!
//! Return-site Magic returns a real site to its owner's Atlas hand, leaves no
//! Rubble, and banishes surface minions that occupied it. A protected site
//! prevents the return without leaving play. While Deathrites wait for ordering,
//! return-site Magic stays withheld until the chain drains.

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

fn site(protected: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
    });
    if protected {
        value["cannotBeMovedDestroyedOrModified"] = json!(true);
    }
    value
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn return_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetSiteToOwnerHand": true,
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

fn return_site_manifest(seed: u32, protected: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "return-site-magic" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-site-magic-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-return": return_spell(),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(protected),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-return"; 6],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid return-site session");
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

fn return_site_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("return-site actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return"
        })
        .filter_map(|action| {
            Some((
                action.descriptor["targetLocation"]["cell"]
                    .as_str()?
                    .to_owned(),
                action.descriptor["targetSiteInstanceId"]
                    .as_str()?
                    .to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn seed_with_return_spell(protected: bool, start: u32) -> String {
    (start..start + 512)
        .map(|seed| return_site_manifest(seed, protected))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                let north = &state(&preview)["players"]["north"]["hand"];
                let south = &state(&preview)["players"]["south"]["hand"];
                north["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-return"))
                    && south["spellbook"].as_array().is_some_and(|hand| {
                        hand.iter().any(|card| card["cardId"] == "south-minion")
                    })
            })
        })
        .expect("bounded seed with return-site Magic and a South minion in the opening hands")
}

fn stage_south_minion(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let enemy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_id
}

fn deathrite_return_manifest(seed: u32) -> String {
    let fixture = "return-site-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-return": return_spell(),
            "north-rain": rain_spell(),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(false),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-return",
                    "north-rain",
                    "north-rain",
                    "north-return",
                    "north-rain",
                    "north-return",
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

fn north_has_return_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-return", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteReturnSetup {
    deathrite_ids: [String; 2],
    session: Session,
    south_site_id: String,
}

fn try_pending_deathrite_with_return_target(encoded: &str) -> Option<PendingDeathriteReturnSetup> {
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
    let south_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()?
        .to_owned();
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
    if !north_has_return_and_rain(&state(&session)) {
        return None;
    }
    if return_site_targets(&session).is_empty() {
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
    Some(PendingDeathriteReturnSetup {
        deathrite_ids,
        session,
        south_site_id,
    })
}

fn deathrite_return_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_return_manifest)
        .find(|candidate| try_pending_deathrite_with_return_target(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with return-site Magic in hand")
}

#[test]
fn rule_catalog_0625_return_target_site_returns_owners_site_and_banishes_surface_minions() {
    let encoded = seed_with_return_spell(false, 625);
    let mut session = opening_main(&encoded);
    let north_site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("North site identity")
        .to_owned();
    let minion_id = stage_south_minion(&mut session);
    let before = state(&session);
    let south_site_id = before["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South site identity")
        .to_owned();
    let south_atlas_before = before["players"]["south"]["hand"]["atlas"]
        .as_array()
        .expect("South Atlas hand")
        .len();
    assert_eq!(
        return_site_targets(&session),
        [
            ("C1".to_owned(), south_site_id.clone()),
            ("C4".to_owned(), north_site_id.clone()),
        ]
    );
    assert!(realm_unit(&before, &minion_id).is_some());

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
    });
    assert_eq!(
        descriptor["targetLocation"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "site-returned-to-hand",
            "minion-banished",
            "magic-resolved"
        ]
    );
    let returned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-returned-to-hand")
        .expect("site return");
    assert_eq!(returned.payload["cardId"], "south-site");
    assert_eq!(returned.payload["cell"], "C1");
    assert_eq!(returned.payload["instanceId"], south_site_id);
    assert_eq!(returned.payload["owner"], "south");
    let banished = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-banished")
        .expect("surface minion banishment");
    assert_eq!(banished.payload["cardId"], "south-minion");
    assert_eq!(banished.payload["instanceId"], minion_id);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died"
                || event.event_type == "rubble-created"
                || event.event_type == "site-destroyed")
    );

    let after = state(&session);
    assert!(after["realm"]["sites"].get("C1").is_none());
    assert_eq!(after["realm"]["sites"]["C4"]["instanceId"], north_site_id);
    assert_eq!(after["players"]["south"]["avatar"]["location"], "C1");
    assert!(realm_unit(&after, &minion_id).is_none());
    assert!(
        after["players"]["south"]["hand"]["atlas"]
            .as_array()
            .expect("South Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == south_site_id)
    );
    assert_eq!(
        after["players"]["south"]["hand"]["atlas"]
            .as_array()
            .expect("South Atlas hand")
            .len(),
        south_atlas_before + 1
    );
    let north_view = session
        .public_view(Seat::North)
        .expect("North public view after the return");
    assert_eq!(
        north_view["players"]["south"]["hand"]["atlas"],
        south_atlas_before + 1
    );
    assert_eq!(
        return_site_targets(&session),
        [("C4".to_owned(), north_site_id)]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0626_return_target_site_is_prevented_on_a_protected_site() {
    let encoded = seed_with_return_spell(true, 626);
    let mut session = opening_main(&encoded);
    let minion_id = stage_south_minion(&mut session);
    let before = state(&session);
    let south_site = before["realm"]["sites"]["C1"].clone();
    let south_site_id = south_site["instanceId"]
        .as_str()
        .expect("South site identity")
        .to_owned();
    let south_atlas_before = before["players"]["south"]["hand"]["atlas"]
        .as_array()
        .expect("South Atlas hand")
        .len();

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-return-prevented", "magic-resolved"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-returned-to-hand"
                || event.event_type == "minion-banished"
                || event.event_type == "minion-died"
                || event.event_type == "rubble-created")
    );

    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"], south_site);
    assert!(realm_unit(&after, &minion_id).is_some());
    assert_eq!(
        after["players"]["south"]["hand"]["atlas"]
            .as_array()
            .expect("South Atlas hand")
            .len(),
        south_atlas_before
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1047_return_site_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_return_seed_with(1047);
    let mut setup = try_pending_deathrite_with_return_target(&encoded)
        .expect("complete return-site Deathrite withheld setup");
    let south_site_id = setup.south_site_id.clone();
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
    assert_eq!(paused["realm"]["sites"]["C1"]["instanceId"], south_site_id);
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(return_site_targets(session).is_empty());

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
    assert_eq!(resumed["realm"]["sites"]["C1"]["instanceId"], south_site_id);
    assert!(
        return_site_targets(session)
            .iter()
            .any(|(cell, id)| cell == "C1" && *id == south_site_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-returned-to-hand", "magic-resolved"]
    );
    assert!(state(session)["realm"]["sites"].get("C1").is_none());
    assert_exact_replay(session);
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

fn return_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "return-site-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-return-site-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-return": return_spell(),
            "north-site": site(false),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(false),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-return"; 8],
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

fn supplemental_seed_with_start(start: u32) -> String {
    (start..start + 2048)
        .chain(625..625 + 2048)
        .map(return_supplemental_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-return")
        })
        .expect("bounded seed with return-site Magic in the opening hand")
}

fn return_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-return")
                .count()
        })
        .unwrap_or_default()
}

fn site_instance_at(snapshot: &Value, cell: &str) -> String {
    snapshot["realm"]["sites"][cell]["instanceId"]
        .as_str()
        .expect("site at cell")
        .to_owned()
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

fn play_south_site_at(session: &mut Session, cell: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == cell
    });
    site_instance_at(&state(session), cell)
}

fn setup_south_sites_at(session: &mut Session, cells: &[&str]) -> Vec<(String, String)> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let mut placed = vec![(cells[0].to_string(), play_south_site_at(session, cells[0]))];
    for cell in cells.iter().skip(1) {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw"
                && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
        });
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw"
                && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
        });
        placed.push(((*cell).to_string(), play_south_site_at(session, cell)));
    }
    placed
}

fn cast_return_on(session: &mut Session, cell: &str, site_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetSiteInstanceId"] == site_id
    });
    receipt
}

fn try_second_return_enemy_site_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    let (c1, south_c1) = setup_south_sites_at(&mut session, &["C1", "C2"])[0].clone();
    north_draws_spellbook(&mut session);
    cast_return_on(&mut session, &c1, &south_c1);
    pass_turn_to_north_spellbook(&mut session);
    if return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    let south_c3 = play_south_site_at(&mut session, "C3");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    return_site_targets(&session)
        .iter()
        .any(|(cell, id)| cell == "C3" && *id == south_c3)
        .then_some((session, "C3".to_owned(), south_c3))
}

fn seed_for_second_return_enemy_site(start: u32) -> String {
    (start..start + 8192)
        .chain(625..625 + 8192)
        .find_map(|seed| {
            let encoded = return_supplemental_manifest(seed);
            try_second_return_enemy_site_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second return-site enemy-arrival setup")
}

fn try_second_return_new_site_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = opening_main(encoded);
    let (c1, south_c1) = setup_south_sites_at(&mut session, &["C1"])[0].clone();
    north_draws_spellbook(&mut session);
    cast_return_on(&mut session, &c1, &south_c1);
    if state(&session)["realm"]["sites"].get("C1").is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let south_c2 = play_south_site_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    return_site_targets(&session)
        .iter()
        .any(|(cell, id)| cell == "C2" && *id == south_c2)
        .then_some((session, "C2".to_owned(), south_c2))
}

fn seed_for_second_return_new_site(start: u32) -> String {
    (start..start + 8192)
        .chain(625..625 + 8192)
        .find_map(|seed| {
            let encoded = return_supplemental_manifest(seed);
            try_second_return_new_site_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second return-site new-placement setup")
}

#[test]
fn rule_catalog_2093_site_stays_at_the_location_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2093);
    let mut session = opening_main(&encoded);
    let site_id = setup_south_sites_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    pass_turn_to_north_spellbook(&mut session);
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["instanceId"],
        site_id
    );
    assert_ne!(
        state(&session)["realm"]["sites"]["C1"]["rubble"],
        json!(true)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2094_second_return_offers_no_targets_after_the_only_real_site_returns() {
    let encoded = (2094..2094 + 8192)
        .chain(625..625 + 8192)
        .find_map(|seed| {
            let candidate = return_supplemental_manifest(seed);
            if !opening_spell_ids(&candidate)
                .iter()
                .any(|card| card == "north-return")
            {
                return None;
            }
            let mut session = opening_main(&candidate);
            let north_site = site_instance_at(&state(&session), "C4");
            let first = cast_return_on(&mut session, "C4", &north_site);
            if !event_types(&first).contains(&"site-returned-to-hand") {
                return None;
            }
            if state(&session)["realm"]["sites"].get("C4").is_some() {
                return None;
            }
            if return_spells_in_hand(&state(&session)) < 1 {
                pass_turn_to_north_spellbook(&mut session);
            }
            (return_spells_in_hand(&state(&session)) >= 1
                && return_site_targets(&session).is_empty())
            .then_some(candidate)
        })
        .expect("bounded seed with two return-site casts after clearing real sites");
    let mut session = opening_main(&encoded);
    let north_site = site_instance_at(&state(&session), "C4");
    let first = cast_return_on(&mut session, "C4", &north_site);
    assert!(event_types(&first).contains(&"site-returned-to-hand"));
    assert!(state(&session)["realm"]["sites"].get("C4").is_none());
    if return_spells_in_hand(&state(&session)) < 1 {
        pass_turn_to_north_spellbook(&mut session);
    }
    assert!(return_spells_in_hand(&state(&session)) >= 1);
    assert!(return_site_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2095_second_return_returns_a_newly_arrived_site_after_enemy_site_placement() {
    let encoded = seed_for_second_return_enemy_site(2095);
    let (mut session, cell, site_id) =
        try_second_return_enemy_site_prefix(&encoded).expect("second return-site prefix");
    let receipt = cast_return_on(&mut session, &cell, &site_id);
    assert!(event_types(&receipt).contains(&"site-returned-to-hand"));
    assert!(state(&session)["realm"]["sites"].get(&cell).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2096_return_site_offers_every_real_site_in_the_realm() {
    let encoded = supplemental_seed_with_start(2096);
    let mut session = opening_main(&encoded);
    let south_sites = setup_south_sites_at(&mut session, &["C1", "C2"]);
    north_draws_spellbook(&mut session);
    let north_site = site_instance_at(&state(&session), "C4");
    let offered = return_site_targets(&session);
    for (cell, site_id) in &south_sites {
        assert!(offered.contains(&(cell.clone(), site_id.clone())));
    }
    assert!(offered.contains(&("C4".to_owned(), north_site)));
    assert_eq!(offered.len(), 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2097_return_site_leaves_a_far_site_untouched() {
    let encoded = supplemental_seed_with_start(2097);
    let mut session = opening_main(&encoded);
    let far_id = setup_south_sites_at(&mut session, &["C1"])[0].1.clone();
    north_draws_spellbook(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C3"
    });
    let near_id = site_instance_at(&state(&session), "C3");
    let receipt = cast_return_on(&mut session, "C3", &near_id);
    assert!(event_types(&receipt).contains(&"site-returned-to-hand"));
    assert!(state(&session)["realm"]["sites"].get("C3").is_none());
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["instanceId"],
        far_id
    );
    assert_ne!(
        state(&session)["realm"]["sites"]["C1"]["rubble"],
        json!(true)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2098_second_return_returns_a_newly_placed_site() {
    let encoded = seed_for_second_return_new_site(2098);
    let (mut session, cell, site_id) = try_second_return_new_site_prefix(&encoded)
        .expect("second return-site new-placement prefix");
    let receipt = cast_return_on(&mut session, &cell, &site_id);
    assert!(event_types(&receipt).contains(&"site-returned-to-hand"));
    assert!(state(&session)["realm"]["sites"].get(&cell).is_none());
    assert!(
        state(&session)["players"]["south"]["hand"]["atlas"]
            .as_array()
            .is_some_and(|cards| cards.iter().any(|card| card["instanceId"] == site_id))
    );
    assert_exact_replay(&session);
}
