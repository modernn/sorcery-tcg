//! Direct proofs for Drown occupancy of a non-Submerge minion and targetless
//! controller healing (RULE-CATALOG-0045–0046, 0693–0694, 1104, 2433–2438).
//!
//! 0591–0592 already cover a Submerge minion surviving underwater and the
//! earth-only paid no-op. Drown also submerges a minion without Submerge, and
//! that minion dies. Targetless `healController` restores only the caster
//! Avatar through the printed-life cap and cannot leave Death's Door, unlike
//! 0651–0652 which offer a chosen Avatar. While Deathrites wait for ordering,
//! healController Magic stays withheld until the chain drains.
//!
//! Supplemental 2433–2438 bind cemetery persistence, empty-repeat, enemy-arrival,
//! multi-minion, far-minion, and a newly summoned lander. Distinct from 0693,
//! which kills the first C1 lander on the same turn, and from 1933–1938, which
//! keep a Submerge minion in play.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
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
        "elements": ["earth", "water"],
    })
}

fn lander() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn drown() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "submergeTargetMinion": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn loss() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisLoseControllerLife": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn heal() -> Value {
    json!({
        "cardType": "magic",
        "healController": 7,
        "manaCost": 1,
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

fn drown_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drown-heal-legacy-drown" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drown-heal-legacy-drown-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-drown": drown(),
            "north-site": earth_site(),
            "south-avatar": avatar(20),
            "south-minion": lander(),
            "south-site": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-drown"; 6],
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

fn heal_manifest(seed: u32, life: u8) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drown-heal-legacy-heal" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drown-heal-legacy-heal-v1",
        },
        "cards": {
            "north-avatar": avatar(life),
            "north-heal": heal(),
            "north-loss": loss(),
            "north-site": earth_site(),
            "south-avatar": avatar(20),
            "south-minion": lander(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-heal",
                    "north-heal",
                    "north-heal",
                    "north-loss",
                    "north-loss",
                    "north-loss",
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
    let mut session = Session::new(encoded).expect("valid drown-heal session");
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
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn opening_has_all(encoded: &str, card_ids: &[&str]) -> bool {
    Session::new(encoded).ok().is_some_and(|preview| {
        state(&preview)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| {
                card_ids
                    .iter()
                    .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
            })
    })
}

fn seed_drown(start: u32) -> String {
    (start..start + 256)
        .map(drown_manifest)
        .find(|candidate| opening_has_all(candidate, &["north-drown"]))
        .expect("bounded seed with Drown in the opening hand")
}

fn seed_heal(life: u8, start: u32) -> String {
    (start..start + 256)
        .map(|seed| heal_manifest(seed, life))
        .find(|candidate| opening_has_all(candidate, &["north-heal", "north-loss"]))
        .expect("bounded seed with healController and life-loss Genesis in the opening hand")
}

fn offers_heal(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-heal"
        })
}

fn deathrite_heal_controller_manifest(seed: u32) -> String {
    let fixture = "drown-heal-legacy-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-heal": heal(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(20),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-heal",
                    "north-rain",
                    "north-rain",
                    "north-heal",
                    "north-rain",
                    "north-heal",
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

fn north_has_heal_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-heal", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteHealControllerSetup {
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_heal_controller_magic(
    encoded: &str,
) -> Option<PendingDeathriteHealControllerSetup> {
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
    if !north_has_heal_and_rain(&state(&session)) {
        return None;
    }
    if !offers_heal(&session) {
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
    Some(PendingDeathriteHealControllerSetup {
        deathrite_ids,
        session,
    })
}

fn deathrite_heal_controller_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_heal_controller_manifest)
        .find(|candidate| try_pending_deathrite_with_heal_controller_magic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with healController Magic in hand")
}

fn south_plays_c1_and_summons(session: &mut Session) -> String {
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
        .expect("Drown target identity")
        .to_owned()
}

fn healing_after_genesis(life: u8, start: u32) -> (Session, String) {
    let encoded = seed_heal(life, start);
    let mut session = opening_main(&encoded);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-loss"
            && descriptor["region"].is_null()
    });
    let heal = session
        .legal_actions()
        .expect("healing action")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-heal"
        })
        .expect("targetless healing Magic");
    assert!(heal.descriptor.get("target").is_none());
    assert!(heal.descriptor.get("cemeteryMinionInstanceId").is_none());
    let spell_id = heal.descriptor["cardInstanceId"]
        .as_str()
        .expect("healing Magic identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor == &heal.descriptor);
    (session, spell_id)
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
fn rule_catalog_0693_drown_kills_a_non_submerge_minion_at_a_water_site() {
    let encoded = seed_drown(693);
    let mut session = opening_main(&encoded);
    let target_id = south_plays_c1_and_summons(&mut session);
    assert_eq!(
        realm_unit(&state(&session), &target_id).expect("surface lander")["region"],
        "surface"
    );

    let (cast, drowned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-drown"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&drowned),
        [
            "magic-cast",
            "minion-submerged",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(
        drowned.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": target_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    let after = state(&session);
    assert!(realm_unit(&after, &target_id).is_none());
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0694_heal_controller_caps_and_cannot_leave_deaths_door() {
    let (capped, spell_id) = healing_after_genesis(20, 694);
    let capped_state = state(&capped);
    assert_eq!(capped_state["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(capped_state["players"]["north"]["mana"], 0);
    assert!(
        capped_state["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    let receipt = capped.transcript().last().expect("healing receipt");
    assert_eq!(
        event_types(receipt),
        ["magic-cast", "avatar-healed", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["amount"], 2);
    assert_eq!(receipt.events[1].payload["attemptedAmount"], 7);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], spell_id);

    let (death_door, _) = healing_after_genesis(2, 1694);
    let death_door_state = state(&death_door);
    assert_eq!(death_door_state["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(death_door_state["terminal"]["status"], "active");
    assert_eq!(
        event_types(
            death_door
                .transcript()
                .last()
                .expect("Death's Door receipt")
        ),
        ["magic-cast", "magic-resolved"]
    );
    assert_exact_replay(&capped);
    assert_exact_replay(&death_door);
}

#[test]
fn rule_catalog_1104_heal_controller_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_heal_controller_seed_with(1104);
    let mut setup = try_pending_deathrite_with_heal_controller_magic(&encoded)
        .expect("complete healController Deathrite withheld setup");
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["players"]["north"]["avatar"]["life"], 20);
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!offers_heal(session));

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
    assert_eq!(resumed["players"]["north"]["avatar"]["life"], 20);
    assert!(offers_heal(session));

    let (descriptor, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-heal"
    });
    assert!(descriptor.get("target").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert_eq!(state(session)["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(session);
}

fn drown_legacy_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "drown-heal-legacy-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-drown-heal-legacy-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-drown": drown(),
            "north-site": earth_site(),
            "south-avatar": avatar(20),
            "south-minion": lander(),
            "south-site": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-drown"; 8],
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

fn seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(693..693 + 2048)
        .map(drown_legacy_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-drown")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Drown and required South landers")
}

fn drown_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-drown")
                .count()
        })
        .unwrap_or_default()
}

fn drown_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("drown actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-drown"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .ok()
        .is_some_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn in_cemetery(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .is_some_and(|cemetery| {
            cemetery
                .iter()
                .any(|card| card["instanceId"] == instance_id)
        })
}

fn drown_kill_events() -> [&'static str; 4] {
    [
        "magic-cast",
        "minion-submerged",
        "minion-died",
        "magic-resolved",
    ]
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

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
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
        .expect("Drown target identity")
        .to_owned()
}

fn setup_c1_with_landers(session: &mut Session, count: usize) -> Vec<String> {
    south_plays_c1(session);
    (0..count).map(|_| summon_south_at(session, "C1")).collect()
}

fn cast_drown_target(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-drown"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = opening_main(encoded);
    let c1_ids = setup_c1_with_landers(&mut session, 2);
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
    (!drown_targets(&session).is_empty()).then_some((session, c1_ids, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(693..693 + 2048)
        .find_map(|seed| {
            let encoded = drown_legacy_supplemental_manifest(seed);
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
        .expect("bounded seed reaching Drown far-minion setup")
}

fn try_second_drown_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_landers(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_drown_target(&mut session, &first_id);
    if event_types(&first) != drown_kill_events() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if drown_spells_in_hand(&state(&session)) < 1 {
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
    drown_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_drown_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(693..693 + 8192)
        .find_map(|seed| {
            let encoded = drown_legacy_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_drown_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Drown enemy-arrival setup")
}

fn try_second_drown_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_landers(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_drown_target(&mut session, &first_id);
    if event_types(&first) != drown_kill_events() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if drown_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C1");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    drown_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_drown_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(693..693 + 8192)
        .find_map(|seed| {
            let encoded = drown_legacy_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_drown_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Drown new-summon setup")
}

#[test]
fn rule_catalog_2433_drowned_non_submerge_minion_stays_dead_after_turns_pass() {
    let encoded = seed_with_start(2433, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_landers(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let receipt = cast_drown_target(&mut session, &target_id);
    assert_eq!(event_types(&receipt), drown_kill_events());
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert!(in_cemetery(&state(&session), "south", &target_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert!(in_cemetery(&state(&session), "south", &target_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2434_second_drown_without_a_surface_target_stays_unoffered() {
    let encoded = seed_with_start(2434, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_landers(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let receipt = cast_drown_target(&mut session, &target_id);
    assert_eq!(event_types(&receipt), drown_kill_events());
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert!(drown_spells_in_hand(&state(&session)) >= 1);
    assert!(drown_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-drown"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2435_second_drown_kills_a_newly_arrived_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_drown_enemy_arrival(2435);
    let (mut session, minion_id) =
        try_second_drown_enemy_arrival_prefix(&encoded).expect("second Drown enemy-arrival prefix");
    let receipt = cast_drown_target(&mut session, &minion_id);
    assert_eq!(event_types(&receipt), drown_kill_events());
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(in_cemetery(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2436_drown_offers_every_surface_non_submerge_minion_at_the_target_water_site() {
    let encoded = seed_with_start(2436, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c1_with_landers(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = drown_targets(&session);
    assert_eq!(offered.len(), 2);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2437_drown_leaves_a_far_minion_on_the_surface() {
    let encoded = seed_for_far_minion(2437);
    let (mut session, c1_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("Drown far-minion prefix");
    let drowned_id = &c1_ids[0];
    let receipt = cast_drown_target(&mut session, drowned_id);
    assert_eq!(event_types(&receipt), drown_kill_events());
    assert!(realm_unit(&state(&session), drowned_id).is_none());
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("far minion")["region"],
        "surface"
    );
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("far minion")["location"],
        "C4"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2438_second_drown_kills_a_newly_summoned_minion() {
    let encoded = seed_for_second_drown_new_summon(2438);
    let (mut session, minion_id) =
        try_second_drown_new_summon_prefix(&encoded).expect("second Drown new-summon prefix");
    let receipt = cast_drown_target(&mut session, &minion_id);
    assert_eq!(event_types(&receipt), drown_kill_events());
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(in_cemetery(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}
