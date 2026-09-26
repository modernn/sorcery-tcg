//! Direct proofs for cemetery Aura return Magic (RULE-CATALOG-0641–0642,
//! RULE-CATALOG-1073, RULE-CATALOG-2173–2178).
//!
//! Cemetery Aura return offers only Auras in the caster's own cemetery and
//! restores the chosen instance to the hidden Spellbook hand. A cemetery that
//! holds only Magic is still a paid no-choice resolution. While Deathrites
//! wait for ordering, cemetery Aura return Magic stays withheld until the
//! chain drains. Supplemental 2173–2178 bind persistence, empty-cemetery
//! repeat, enemy-arrival, multi-aura offer, unselected remainder, and a
//! newly destroyed Aura.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn flood() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn destroy_spell() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetAura": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn return_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetAuraFromOwnCemetery": true,
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

fn cemetery_aura_manifest(seed: u32, include_setup: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-return": return_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-minion": minion(),
        "south-site": site(),
    });
    let north_spellbook = if include_setup {
        cards["north-destroy"] = destroy_spell();
        cards["north-flood"] = flood();
        json!([
            "north-flood",
            "north-destroy",
            "north-return",
            "north-flood",
            "north-destroy",
            "north-return"
        ])
    } else {
        json!(vec!["north-return"; 6])
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-aura-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-aura-return-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
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
    let mut session = Session::new(encoded).expect("valid cemetery-aura session");
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

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

fn cemetery_aura_cast_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("cemetery Aura actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return"
        })
        .filter_map(|action| {
            action.descriptor["cemeteryMinionInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn north_hand_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn seed_with(required: &[&str], include_setup: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| cemetery_aura_manifest(seed, include_setup))
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                let snapshot = state(&preview);
                let hand = north_hand_ids(&snapshot);
                required.iter().all(|id| hand.iter().any(|card| card == id))
            })
        })
        .expect("bounded seed with required opening cards")
}

fn setup_own_cemetery_aura(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C4")
    });
    let aura_id = state(session)["realm"]["auras"][0]["instanceId"]
        .as_str()
        .expect("Flood identity")
        .to_owned();
    let (_, destroyed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    });
    assert!(
        destroyed
            .events
            .iter()
            .any(|event| event.event_type == "aura-destroyed")
    );
    let destroy_id = state(session)["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("destroy Magic in cemetery")["instanceId"]
        .as_str()
        .expect("destroy identity")
        .to_owned();
    (aura_id, destroy_id)
}

fn deathrite_cemetery_aura_manifest(seed: u32) -> String {
    let fixture = "cemetery-aura-return-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_spell(),
            "north-flood": flood(),
            "north-rain": rain_spell(),
            "north-return": return_spell(),
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
                    "north-flood",
                    "north-destroy",
                    "north-return",
                    "north-rain",
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

fn cemetery_has_card(snapshot: &Value, owner: &str, instance_id: &str) -> bool {
    snapshot["players"][owner]["cemetery"]
        .as_array()
        .is_some_and(|cemetery| {
            cemetery
                .iter()
                .any(|card| card["instanceId"] == instance_id)
        })
}

struct PendingDeathriteCemeteryAuraSetup {
    aura_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_cemetery_aura_target(
    encoded: &str,
) -> Option<PendingDeathriteCemeteryAuraSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let opening_hand = north_hand_ids(&state(&session));
    if !opening_hand.iter().any(|card| card == "north-flood")
        || !opening_hand.iter().any(|card| card == "north-destroy")
    {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C4")
    })?;
    let aura_id = state(&session)["realm"]["auras"][0]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
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
    if !north_has_return_and_rain(&state(&session)) {
        return None;
    }
    if cemetery_aura_cast_ids(&session).is_empty() {
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
    Some(PendingDeathriteCemeteryAuraSetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_cemetery_aura_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cemetery_aura_manifest)
        .find(|candidate| try_pending_deathrite_with_cemetery_aura_target(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with cemetery-aura return Magic in hand",
        )
}

#[test]
fn rule_catalog_0641_cemetery_aura_return_restores_own_cemetery_aura_to_hidden_hand() {
    let encoded = seed_with(&["north-flood", "north-destroy", "north-return"], true, 641);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_aura_cast_ids(&session), Vec::<String>::new());
    let (aura_id, destroy_id) = setup_own_cemetery_aura(&mut session);
    assert_ne!(destroy_id, aura_id);
    let cemetery_targets = cemetery_aura_cast_ids(&session);
    assert!(!cemetery_targets.is_empty());
    assert!(cemetery_targets.iter().all(|id| id == &aura_id));
    assert!(!cemetery_targets.iter().any(|id| id == &destroy_id));
    let before = state(&session);
    let before_hand_count = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let south_observation = session.observe(Seat::South);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["cardId"], "north-flood");
    assert_eq!(receipt.events[1].payload["instanceId"], aura_id);
    assert_eq!(receipt.events[1].payload["owner"], "north");
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "magic-returned-to-hand"
                || event.event_type == "minion-returned-to-hand"
                || event.event_type == "aura-destroyed"
                || event.event_type == "aura-dispelled")
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        before_hand_count
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == aura_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != aura_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == destroy_id)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_view["players"]["north"]["hand"]["spellbook"], 1);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("cemetery-aura checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized cemetery-aura");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed cemetery-aura");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed cemetery-aura session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0642_cemetery_aura_return_is_a_paid_noop_without_cemetery_aura() {
    let encoded = seed_with(&["north-return"], false, 642);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_aura_cast_ids(&session), Vec::<String>::new());
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-returned-to-hand"
                || event.event_type == "magic-returned-to-hand")
    );
    assert_eq!(cemetery_aura_cast_ids(&session), Vec::<String>::new());
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        1
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1073_cemetery_aura_return_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cemetery_aura_seed_with(1073);
    let mut setup = try_pending_deathrite_with_cemetery_aura_target(&encoded)
        .expect("complete cemetery-aura return Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
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
    assert!(cemetery_has_card(&paused, "north", &aura_id));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(cemetery_aura_cast_ids(session).is_empty());

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
    assert!(cemetery_has_card(&resumed, "north", &aura_id));
    assert_eq!(
        cemetery_aura_cast_ids(session).as_slice(),
        std::slice::from_ref(&aura_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == aura_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    assert!(!cemetery_has_card(&state(session), "north", &aura_id));
    assert_exact_replay(session);
}

fn cemetery_aura_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-aura-return-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-aura-return-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_spell(),
            "north-flood": flood(),
            "north-return": return_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-flood",
                    "north-destroy",
                    "north-return",
                    "north-return",
                    "north-flood",
                    "north-destroy",
                    "north-flood",
                    "north-return",
                ],
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

fn opening_hand_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    north_hand_ids(&state(&preview))
}

fn supplemental_seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(641..641 + 2048)
        .map(cemetery_aura_supplemental_manifest)
        .find(|candidate| {
            required.iter().all(|id| {
                opening_hand_spell_ids(candidate)
                    .iter()
                    .any(|card| card == *id)
            })
        })
        .expect("bounded seed with cemetery Aura return supplemental opening cards")
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

fn cemetery_aura_instance_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .map(|cemetery| {
            cemetery
                .iter()
                .filter(|card| card["cardId"] == "north-flood")
                .filter_map(|card| card["instanceId"].as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn hand_has_instance(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["instanceId"] == instance_id))
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

fn try_opening_main(encoded: &str) -> Option<Session> {
    let mut session = Session::new(encoded).ok()?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    Some(session)
}

fn try_end_turn_if_offered(session: &mut Session) -> Option<()> {
    decline_attack_if_needed(session);
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    Some(())
}

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    try_end_turn_if_offered(session)?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    try_end_turn_if_offered(session)?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    try_pass_turn_to_north_spellbook(session).expect("north Spellbook draw");
}

fn try_advance_north_spellbook_draws(session: &mut Session, draws: usize) -> Option<()> {
    for _ in 0..draws {
        try_pass_turn_to_north_spellbook(session)?;
    }
    Some(())
}

fn try_destroy_own_aura(session: &mut Session, avoid: &[String]) -> Option<String> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-flood"
            && cells_include(descriptor, "C4")
            && !avoid.iter().any(|id| descriptor["cardInstanceId"] == *id)
    })?;
    let aura_id = state(session)["realm"]["auras"]
        .as_array()?
        .iter()
        .rev()
        .find(|aura| aura["cardId"] == "north-flood")?["instanceId"]
        .as_str()?
        .to_owned();
    let (_, destroyed) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetAuraInstanceId"] == aura_id
    })?;
    event_types(&destroyed)
        .contains(&"aura-destroyed")
        .then_some(aura_id)
}

fn destroy_own_aura(session: &mut Session) -> String {
    try_destroy_own_aura(session, &[]).expect("own cemetery Aura")
}

fn try_cast_return_target(session: &mut Session, aura_id: &str) -> Option<Receipt> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == aura_id
    })
    .map(|(_, receipt)| receipt)
}

fn cast_return_target(session: &mut Session, aura_id: &str) -> Receipt {
    try_cast_return_target(session, aura_id).expect("cemetery Aura return")
}

fn try_two_cemetery_auras_on_session(session: &mut Session) -> Option<[String; 2]> {
    let first = try_destroy_own_aura(session, &[])?;
    let mut second = None;
    for _ in 0..4 {
        if let Some(id) = try_destroy_own_aura(session, std::slice::from_ref(&first)) {
            second = Some(id);
            break;
        }
        try_pass_turn_to_north_spellbook(session)?;
    }
    let second = second?;
    (cemetery_aura_instance_ids(&state(session), "north").len() == 2).then_some([first, second])
}

fn seed_with_two_cemetery_auras(start: u32) -> String {
    (start..start + 8192)
        .chain(641..641 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_aura_supplemental_manifest(seed);
            if !["north-flood", "north-destroy", "north-return"]
                .iter()
                .all(|id| {
                    opening_hand_spell_ids(&encoded)
                        .iter()
                        .any(|card| card == *id)
                })
            {
                return None;
            }
            let mut session = try_opening_main(&encoded)?;
            try_two_cemetery_auras_on_session(&mut session).map(|_| encoded)
        })
        .expect("bounded seed with two own cemetery Auras")
}

fn prepare_two_cemetery_auras(encoded: &str) -> (Session, [String; 2]) {
    let mut session = try_opening_main(encoded).expect("opening main");
    let auras = try_two_cemetery_auras_on_session(&mut session).expect("two own cemetery Auras");
    (session, auras)
}

fn try_second_return_empty_prefix(encoded: &str) -> Option<Session> {
    let mut session = try_opening_main(encoded)?;
    let aura_id = try_destroy_own_aura(&mut session, &[])?;
    try_cast_return_target(&mut session, &aura_id)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    (return_spells_in_hand(&state(&session)) >= 1
        && cemetery_aura_instance_ids(&state(&session), "north").is_empty())
    .then_some(session)
}

fn seed_for_second_return_empty(start: u32) -> String {
    (start..start + 8192)
        .chain(641..641 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_aura_supplemental_manifest(seed);
            try_second_return_empty_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Aura return empty-cemetery setup")
}

fn try_second_return_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = try_opening_main(encoded)?;
    let first_id = try_destroy_own_aura(&mut session, &[])?;
    try_cast_return_target(&mut session, &first_id)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    if return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_end_turn_if_offered(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    try_end_turn_if_offered(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_advance_north_spellbook_draws(&mut session, 2)?;
    let second_id = try_destroy_own_aura(&mut session, &[first_id])?;
    (return_spells_in_hand(&state(&session)) >= 1
        && cemetery_aura_cast_ids(&session).contains(&second_id))
    .then_some((session, second_id))
}

fn seed_for_second_return_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(641..641 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_aura_supplemental_manifest(seed);
            try_second_return_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Aura return enemy-arrival setup")
}

fn try_second_return_new_destroy_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = try_opening_main(encoded)?;
    let first_id = try_destroy_own_aura(&mut session, &[])?;
    try_cast_return_target(&mut session, &first_id)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    if return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_advance_north_spellbook_draws(&mut session, 2)?;
    let second_id = try_destroy_own_aura(&mut session, &[first_id])?;
    (return_spells_in_hand(&state(&session)) >= 1
        && cemetery_aura_cast_ids(&session).contains(&second_id))
    .then_some((session, second_id))
}

fn seed_for_second_return_new_destroy(start: u32) -> String {
    (start..start + 8192)
        .chain(641..641 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_aura_supplemental_manifest(seed);
            try_second_return_new_destroy_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Aura return new-destroy setup")
}

#[test]
fn rule_catalog_2173_returned_aura_stays_in_hand_after_turns_pass() {
    let encoded =
        supplemental_seed_with_start(2173, &["north-flood", "north-destroy", "north-return"]);
    let mut session = opening_main(&encoded);
    let aura_id = destroy_own_aura(&mut session);
    cast_return_target(&mut session, &aura_id);
    assert!(hand_has_instance(&state(&session), "north", &aura_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(hand_has_instance(&state(&session), "north", &aura_id));
    assert!(cemetery_aura_instance_ids(&state(&session), "north").is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2174_second_return_without_a_cemetery_aura_is_a_paid_noop() {
    let encoded = seed_for_second_return_empty(2174);
    let mut session =
        try_second_return_empty_prefix(&encoded).expect("second cemetery Aura return empty prefix");
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-returned-to-hand")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2175_second_return_restores_a_newly_destroyed_aura_after_enemy_site_placement() {
    let encoded = seed_for_second_return_enemy_arrival(2175);
    let (mut session, aura_id) = try_second_return_enemy_arrival_prefix(&encoded)
        .expect("second cemetery Aura return enemy-arrival prefix");
    let receipt = cast_return_target(&mut session, &aura_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), "north", &aura_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2176_cemetery_aura_return_offers_every_own_cemetery_aura() {
    let encoded = seed_with_two_cemetery_auras(2176);
    let (session, aura_ids) = prepare_two_cemetery_auras(&encoded);
    let mut offered = cemetery_aura_cast_ids(&session);
    offered.sort();
    offered.dedup();
    assert_eq!(offered.len(), 2);
    for aura_id in &aura_ids {
        assert!(offered.contains(aura_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2177_cemetery_aura_return_leaves_an_unselected_cemetery_aura_in_place() {
    let encoded = seed_with_two_cemetery_auras(2177);
    let (mut session, aura_ids) = prepare_two_cemetery_auras(&encoded);
    let returned_id = &aura_ids[0];
    cast_return_target(&mut session, returned_id);
    assert!(hand_has_instance(&state(&session), "north", returned_id));
    let remaining = cemetery_aura_instance_ids(&state(&session), "north");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0], aura_ids[1]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2178_second_return_restores_a_newly_destroyed_aura() {
    let encoded = seed_for_second_return_new_destroy(2178);
    let (mut session, aura_id) = try_second_return_new_destroy_prefix(&encoded)
        .expect("second cemetery Aura return new-destroy prefix");
    let receipt = cast_return_target(&mut session, &aura_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "aura-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), "north", &aura_id));
    assert_exact_replay(&session);
}
