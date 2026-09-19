//! Direct proofs for disable-target-nearby-minion-until-next-turn Magic
//! (RULE-CATALOG-0581–0582, RULE-CATALOG-0661–0662, RULE-CATALOG-1013,
//! RULE-CATALOG-2273–2278).
//!
//! Ordinary Freeze Magic disables a nearby minion until the caster's next Start
//! Phase. Unlike measured disable-until-damaged, the flag expires on that turn
//! boundary rather than on damage. While Deathrites wait for ordering, Freeze
//! Magic stays withheld until the chain drains. Supplemental 2273–2278 bind
//! persistence, empty-repeat, enemy-arrival, multi-target, far-minion, and a
//! newly summoned nearby minion.

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

fn nearby() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn far() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn freeze() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
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

fn kill_spell() -> Value {
    json!({
        "cardType": "magic",
        "killTargetMinion": true,
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

fn freeze_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "freeze-nearby" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-freeze-nearby-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-freeze": freeze(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": far(),
            "south-nearby": nearby(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-freeze",
                    "north-freeze",
                    "north-freeze",
                    "north-freeze",
                    "north-freeze",
                    "north-freeze",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-far",
                    "south-nearby",
                    "south-far",
                    "south-nearby",
                    "south-far",
                    "south-nearby",
                ],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid freeze session");
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

fn has_activate_mana(session: &Session, instance_id: &str) -> bool {
    session
        .legal_actions()
        .expect("mana actions")
        .into_iter()
        .any(|action| {
            action.descriptor["kind"] == "activate-mana"
                && action.descriptor["unitInstanceId"] == instance_id
        })
}

fn freeze_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Freeze actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-freeze"
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

fn deathrite_freeze_manifest(seed: u32) -> String {
    let fixture = "freeze-nearby-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-freeze": freeze(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-freeze",
                    "north-rain",
                    "north-rain",
                    "north-freeze",
                    "north-rain",
                    "north-freeze",
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

fn north_has_freeze_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-freeze", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteFreezeSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_pending_deathrite_with_nearby_visitor(encoded: &str) -> Option<PendingDeathriteFreezeSetup> {
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
    if !north_has_freeze_and_rain(&state(&session)) {
        return None;
    }
    if !freeze_targets(&session).contains(&visitor_id) {
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
    Some(PendingDeathriteFreezeSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_freeze_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_freeze_manifest)
        .find(|candidate| try_pending_deathrite_with_nearby_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Freeze Magic in hand")
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

fn seed_with(required: &[&str]) -> String {
    (581..581 + 512)
        .map(freeze_manifest)
        .find(|candidate| {
            let north = opening_spell_ids(candidate, "north");
            let south = opening_spell_ids(candidate, "south");
            required
                .iter()
                .all(|id| north.iter().any(|card| card == id))
                && ["south-far", "south-nearby"]
                    .into_iter()
                    .all(|id| south.iter().any(|card| card == id))
        })
        .expect("bounded seed with Freeze and both South minions")
}

fn south_plays_c1_and_summons(session: &mut Session, card_id: &str) -> String {
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
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned instance identity")
        .to_owned()
}

fn setup_nearby_and_far(session: &mut Session) -> (String, String) {
    let far_id = south_plays_c1_and_summons(session, "south-far");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let nearby_id = {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-nearby"
                && descriptor["cell"] == "C2"
                && descriptor["region"].is_null()
        });
        summoned["cardInstanceId"]
            .as_str()
            .expect("nearby instance identity")
            .to_owned()
    };
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let north_avatar = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_avatar
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    (nearby_id, far_id)
}

#[test]
fn rule_catalog_0581_freeze_disables_nearby_minion_until_caster_next_start_phase() {
    let encoded = seed_with(&["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, far_id) = setup_nearby_and_far(&mut session);

    let targets = freeze_targets(&session);
    assert!(targets.contains(&nearby_id));
    assert!(!targets.contains(&far_id));

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    let source_id = cast["cardInstanceId"].as_str().expect("Freeze source");
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "expiresAtSeat": "north",
            "instanceId": nearby_id,
            "seat": "south",
            "sourceInstanceId": source_id,
            "stealthRemoved": false,
            "wardRemoved": false,
        })
    );
    assert!(!has_activate_mana(&session, &nearby_id));

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(!has_activate_mana(&session, &nearby_id));

    let (_, expired) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_types(&expired),
        ["turn-ended", "minion-disable-expired", "turn-started"]
    );
    assert_eq!(expired.events[1].payload["instanceId"], nearby_id);
    assert!(unit(&state(&session), &nearby_id)["disableEffects"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0582_freeze_does_not_offer_a_minion_three_steps_away() {
    let encoded = seed_with(&["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, far_id) = setup_nearby_and_far(&mut session);

    let targets = freeze_targets(&session);
    assert!(targets.contains(&nearby_id));
    assert!(!targets.contains(&far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1013_freeze_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_freeze_seed_with(1013);
    let mut setup = try_pending_deathrite_with_nearby_visitor(&encoded)
        .expect("complete Freeze Deathrite withheld setup");
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
    assert!(unit(&paused, &visitor_id)["disableEffects"].is_null());
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(freeze_targets(session).is_empty());

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
    assert!(unit(&resumed, &visitor_id)["disableEffects"].is_null());
    assert!(freeze_targets(session).contains(&visitor_id));

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == visitor_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    let source_id = cast["cardInstanceId"].as_str().expect("Freeze source");
    assert_eq!(
        receipt.events[1].payload,
        json!({
            "expiresAtSeat": "north",
            "instanceId": visitor_id,
            "seat": "south",
            "sourceInstanceId": source_id,
            "stealthRemoved": false,
            "wardRemoved": false,
        })
    );
    assert!(!has_activate_mana(session, &visitor_id));
    assert_exact_replay(session);
}

fn freeze_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "freeze-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-freeze-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-freeze": freeze(),
            "north-kill": kill_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": far(),
            "south-nearby": nearby(),
            "south-site": site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-freeze",
                    "north-freeze",
                    "north-kill",
                    "north-freeze",
                    "north-freeze",
                    "north-kill",
                    "north-freeze",
                    "north-freeze",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-nearby"; 3]
                    .into_iter()
                    .chain(vec!["south-far"; 2])
                    .chain(vec!["south-visitor"; 1])
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn seed_has_opening(encoded: &str, required_north: &[&str]) -> bool {
    let north = opening_spell_ids(encoded, "north");
    let south = opening_spell_ids(encoded, "south");
    required_north
        .iter()
        .all(|id| north.iter().any(|card| card == id))
        && ["south-far", "south-nearby"]
            .into_iter()
            .all(|id| south.iter().any(|card| card == id))
}

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(661..661 + 2048)
        .map(freeze_supplemental_manifest)
        .find(|candidate| seed_has_opening(candidate, required))
        .expect("bounded seed with required opening cards")
}

fn freeze_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-freeze")
                .count()
        })
        .unwrap_or_default()
}

fn is_disabled(snapshot: &Value, instance_id: &str) -> bool {
    !unit(snapshot, instance_id)["disableEffects"].is_null()
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

fn try_pass_turn_to_north_spellbook(session: &mut Session) -> Option<()> {
    end_turn_if_offered(session);
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    Some(())
}

fn try_cast_freeze(session: &mut Session, target_id: &str) -> Option<Receipt> {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == target_id
    })
    .map(|(_, receipt)| receipt)
}

fn cast_freeze(session: &mut Session, target_id: &str) -> Receipt {
    try_cast_freeze(session, target_id).expect("Freeze target")
}

fn cast_kill(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn setup_two_nearby_minions(session: &mut Session) -> (String, String) {
    let (first_id, _far_id) = setup_nearby_and_far(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D2"
    });
    let (second, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-nearby"
            && descriptor["cell"] == "D2"
            && descriptor["region"].is_null()
    });
    let second_id = second["cardInstanceId"]
        .as_str()
        .expect("second nearby identity")
        .to_owned();
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (first_id, second_id)
}

fn try_second_freeze_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    if !seed_has_opening(encoded, &["north-freeze"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    let (nearby_id, _) = setup_nearby_and_far(&mut session);
    try_cast_freeze(&mut session, &nearby_id)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    if freeze_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D2"
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "D2"
            && descriptor["region"].is_null()
    })?;
    let visitor_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    freeze_targets(&session)
        .contains(&visitor_id)
        .then_some((session, visitor_id))
}

fn seed_for_second_freeze_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(661..661 + 8192)
        .find_map(|seed| {
            let encoded = freeze_supplemental_manifest(seed);
            try_second_freeze_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Freeze enemy-arrival setup")
}

fn try_second_freeze_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    if !seed_has_opening(encoded, &["north-freeze"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    let (nearby_id, _) = setup_nearby_and_far(&mut session);
    try_cast_freeze(&mut session, &nearby_id)?;
    try_pass_turn_to_north_spellbook(&mut session)?;
    if freeze_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D2"
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-nearby"
            && descriptor["cell"] == "D2"
            && descriptor["region"].is_null()
    })?;
    let new_id = summoned["cardInstanceId"].as_str()?.to_owned();
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    freeze_targets(&session)
        .contains(&new_id)
        .then_some((session, new_id))
}

fn seed_for_second_freeze_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(661..661 + 8192)
        .find_map(|seed| {
            let encoded = freeze_supplemental_manifest(seed);
            try_second_freeze_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Freeze new-summon setup")
}

#[test]
fn rule_catalog_1883_disabled_minion_stays_disabled_after_turns_pass() {
    let encoded = seed_with_start(1883, &["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, _) = setup_nearby_and_far(&mut session);
    cast_freeze(&mut session, &nearby_id);
    assert!(is_disabled(&state(&session), &nearby_id));
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(is_disabled(&state(&session), &nearby_id));
    assert!(!has_activate_mana(&session, &nearby_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1884_second_freeze_without_a_nearby_target_stays_unoffered() {
    let encoded = seed_with_start(1884, &["north-freeze", "north-kill"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, _) = setup_nearby_and_far(&mut session);
    cast_freeze(&mut session, &nearby_id);
    assert!(is_disabled(&state(&session), &nearby_id));
    cast_kill(&mut session, &nearby_id);
    assert!(freeze_spells_in_hand(&state(&session)) >= 1);
    assert!(freeze_targets(&session).is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1885_second_freeze_disables_a_newly_arrived_nearby_minion_after_enemy_arrival() {
    let encoded = seed_for_second_freeze_enemy_arrival(1885);
    let (mut session, visitor_id) = try_second_freeze_enemy_arrival_prefix(&encoded)
        .expect("second Freeze enemy-arrival prefix");
    let receipt = cast_freeze(&mut session, &visitor_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert!(is_disabled(&state(&session), &visitor_id));
    assert!(!has_activate_mana(&session, &visitor_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1886_freeze_offers_every_nearby_minion_as_a_separate_target() {
    let encoded = seed_with_start(1886, &["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (first_id, second_id) = setup_two_nearby_minions(&mut session);
    let targets = freeze_targets(&session);
    assert!(targets.contains(&first_id));
    assert!(targets.contains(&second_id));
    assert_eq!(targets.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1887_freeze_leaves_a_far_minion_untouched() {
    let encoded = seed_with_start(1887, &["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, far_id) = setup_nearby_and_far(&mut session);
    cast_freeze(&mut session, &nearby_id);
    assert!(is_disabled(&state(&session), &nearby_id));
    assert!(!is_disabled(&state(&session), &far_id));
    assert!(!has_activate_mana(&session, &nearby_id));
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == far_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1888_second_freeze_disables_a_newly_summoned_nearby_minion() {
    let encoded = seed_for_second_freeze_new_summon(1888);
    let (mut session, new_id) =
        try_second_freeze_new_summon_prefix(&encoded).expect("second Freeze new-summon prefix");
    let receipt = cast_freeze(&mut session, &new_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert!(is_disabled(&state(&session), &new_id));
    assert!(!has_activate_mana(&session, &new_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2273_disabled_minion_stays_disabled_after_turns_pass() {
    let encoded = seed_with_start(2273, &["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, _) = setup_nearby_and_far(&mut session);
    cast_freeze(&mut session, &nearby_id);
    assert!(is_disabled(&state(&session), &nearby_id));
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(is_disabled(&state(&session), &nearby_id));
    assert!(!has_activate_mana(&session, &nearby_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2274_second_freeze_without_a_nearby_target_stays_unoffered() {
    let encoded = seed_with_start(2274, &["north-freeze", "north-kill"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, _) = setup_nearby_and_far(&mut session);
    cast_freeze(&mut session, &nearby_id);
    assert!(is_disabled(&state(&session), &nearby_id));
    cast_kill(&mut session, &nearby_id);
    assert!(freeze_spells_in_hand(&state(&session)) >= 1);
    assert!(freeze_targets(&session).is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2275_second_freeze_disables_a_newly_arrived_nearby_minion_after_enemy_arrival() {
    let encoded = seed_for_second_freeze_enemy_arrival(2275);
    let (mut session, visitor_id) = try_second_freeze_enemy_arrival_prefix(&encoded)
        .expect("second Freeze enemy-arrival prefix");
    let receipt = cast_freeze(&mut session, &visitor_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert!(is_disabled(&state(&session), &visitor_id));
    assert!(!has_activate_mana(&session, &visitor_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2276_freeze_offers_every_nearby_minion_as_a_separate_target() {
    let encoded = seed_with_start(2276, &["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (first_id, second_id) = setup_two_nearby_minions(&mut session);
    let targets = freeze_targets(&session);
    assert!(targets.contains(&first_id));
    assert!(targets.contains(&second_id));
    assert_eq!(targets.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2277_freeze_leaves_a_far_minion_untouched() {
    let encoded = seed_with_start(2277, &["north-freeze"]);
    let mut session = opening_main(&encoded);
    let (nearby_id, far_id) = setup_nearby_and_far(&mut session);
    cast_freeze(&mut session, &nearby_id);
    assert!(is_disabled(&state(&session), &nearby_id));
    assert!(!is_disabled(&state(&session), &far_id));
    assert!(!has_activate_mana(&session, &nearby_id));
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == far_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2278_second_freeze_disables_a_newly_summoned_nearby_minion() {
    let encoded = seed_for_second_freeze_new_summon(2278);
    let (mut session, new_id) =
        try_second_freeze_new_summon_prefix(&encoded).expect("second Freeze new-summon prefix");
    let receipt = cast_freeze(&mut session, &new_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert!(is_disabled(&state(&session), &new_id));
    assert!(!has_activate_mana(&session, &new_id));
    assert_exact_replay(&session);
}
