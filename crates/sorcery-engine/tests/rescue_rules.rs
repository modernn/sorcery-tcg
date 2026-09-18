//! Direct proofs for return-minion-from-own-cemetery Magic
//! (RULE-CATALOG-0593–0594, 0681–0682, 1051, RULE-CATALOG-1943–1948).
//!
//! Ordinary Rescue Magic offers only minions in the caster's own cemetery and
//! returns the chosen instance to the hidden Spellbook hand. An empty own
//! cemetery still resolves the spell as a paid no-op. 0593–0594 never place an
//! opposing cemetery minion; 0681–0682 mill one onto each side so the own-only
//! filter is the thing under test. While Deathrites wait for ordering, Rescue
//! Magic stays withheld until the chain drains.

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

fn mill_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisDiscardTopSpells": 2,
    })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn kill() -> Value {
    json!({
        "cardType": "magic",
        "killTargetMinion": true,
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

fn rescue() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnMinionFromOwnCemetery": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn empty_rescue_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rescue-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rescue-empty-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rescue": rescue(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-rescue"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opposing_cemetery_manifest(seed: u32, north_has_minion: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-rescue": rescue(),
        "north-site": mill_site(),
        "south-avatar": avatar(),
        "south-minion": minion(),
        "south-site": mill_site(),
    });
    let north_spellbook = if north_has_minion {
        cards["north-minion"] = minion();
        json!([
            "north-minion",
            "north-rescue",
            "north-minion",
            "north-rescue",
            "north-minion",
            "north-rescue",
        ])
    } else {
        json!(vec!["north-rescue"; 6])
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rescue-opposing-cemetery" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rescue-opposing-cemetery-v1",
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

fn rescue_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rescue-minion" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rescue-minion-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-kill": kill(),
            "north-minion": minion(),
            "north-rescue": rescue(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-minion",
                    "north-kill",
                    "north-rescue",
                    "north-minion",
                    "north-kill",
                    "north-rescue",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 6],
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
    let mut session = Session::new(encoded).expect("valid rescue session");
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

fn seed_with(required: &[&str], start: u32) -> String {
    (start..start + 256)
        .map(rescue_manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            let snapshot = state(&preview);
            let hand = snapshot["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("opening hand")
                .iter()
                .map(|card| card["cardId"].as_str().expect("card id"))
                .collect::<Vec<_>>();
            required.iter().all(|id| hand.contains(id))
        })
        .expect("bounded seed with required opening cards")
}

fn cemetery_minions<'a>(snapshot: &'a Value, seat: &str) -> Vec<&'a Value> {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|card| {
            card["cardId"]
                .as_str()
                .is_some_and(|id| id.contains("minion"))
        })
        .collect()
}

fn top_spell_ids<'a>(snapshot: &'a Value, seat: &str) -> Vec<&'a str> {
    snapshot["players"][seat]["spellbook"]
        .as_array()
        .expect("spellbook")
        .iter()
        .take(2)
        .map(|card| card["cardId"].as_str().expect("card id"))
        .collect()
}

fn hand_has(snapshot: &Value, seat: &str, card_id: &str) -> bool {
    snapshot["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("opening hand")
        .iter()
        .any(|card| card["cardId"] == card_id)
}

fn seed_opposing(start: u32, north_needs_minion: bool) -> String {
    (start..start + 256)
        .map(|seed| opposing_cemetery_manifest(seed, north_needs_minion))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            let snapshot = state(&preview);
            let north_top = top_spell_ids(&snapshot, "north");
            let south_top = top_spell_ids(&snapshot, "south");
            let north_milled = north_top.iter().filter(|id| **id == "north-minion").count();
            hand_has(&snapshot, "north", "north-rescue")
                && south_top.contains(&"south-minion")
                && north_milled == usize::from(north_needs_minion)
        })
        .expect("bounded seed with opposing cemetery mill")
}

fn mill_south_opening_site(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn rescue_cemetery_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("Rescue actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-rescue"
        })
        .filter_map(|action| {
            action.descriptor["cemeteryMinionInstanceId"]
                .as_str()
                .map(str::to_owned)
        })
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

fn deathrite_rescue_manifest(seed: u32) -> String {
    let fixture = "rescue-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-kill": kill(),
            "north-minion": minion(),
            "north-rain": rain_spell(),
            "north-rescue": rescue(),
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
                    "north-minion",
                    "north-kill",
                    "north-rescue",
                    "north-rain",
                    "north-rescue",
                    "north-rain",
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

fn north_has_rescue_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-rescue", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteRescueSetup {
    cemetery_minion_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_cemetery_minion(
    encoded: &str,
) -> Option<PendingDeathriteRescueSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let summoned = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let cemetery_minion_id = summoned.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == cemetery_minion_id
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
    if !north_has_rescue_and_rain(&state(&session)) {
        return None;
    }
    if rescue_cemetery_ids(&session).is_empty() {
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
    Some(PendingDeathriteRescueSetup {
        cemetery_minion_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_rescue_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_rescue_manifest)
        .find(|candidate| try_pending_deathrite_with_cemetery_minion(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Rescue Magic in hand")
}

fn setup_own_cemetery_minion(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned minion identity")
        .to_owned();
    let (_, killed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == minion_id
    });
    assert!(event_types(&killed).contains(&"minion-killed"));
    assert!(
        state(session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == minion_id)
    );
    minion_id
}

#[test]
fn rule_catalog_0593_rescue_returns_an_own_cemetery_minion_to_hidden_hand() {
    let encoded = seed_with(&["north-minion", "north-kill", "north-rescue"], 593);
    let mut session = opening_main(&encoded);
    let minion_id = setup_own_cemetery_minion(&mut session);
    let hand_before = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();

    let (cast, returned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-rescue"
            && descriptor["cemeteryMinionInstanceId"] == minion_id
    });
    assert_eq!(
        event_types(&returned),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(returned.events[1].payload["instanceId"], minion_id);
    assert_eq!(
        returned.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == minion_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .is_none_or(|cemetery| !cemetery.iter().any(|card| card["instanceId"] == minion_id))
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0594_rescue_with_empty_cemetery_is_a_paid_noop() {
    let encoded = (594..594 + 256)
        .map(empty_rescue_manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            let snapshot = state(&preview);
            snapshot["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rescue"))
                && cemetery_minions(&snapshot, "north").is_empty()
        })
        .expect("bounded seed with Rescue and no cemetery minions");
    let mut session = opening_main(&encoded);
    assert!(cemetery_minions(&state(&session), "north").is_empty());

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rescue"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(cemetery_minions(&state(&session), "north").is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0681_rescue_offers_only_an_own_cemetery_minion_not_an_opposing_one() {
    let encoded = seed_opposing(681, true);
    let mut session = opening_main(&encoded);
    mill_south_opening_site(&mut session);
    let before = state(&session);
    let own = cemetery_minions(&before, "north");
    let opposing = cemetery_minions(&before, "south");
    assert_eq!(own.len(), 1);
    assert!(!opposing.is_empty());
    let own_id = own[0]["instanceId"]
        .as_str()
        .expect("own minion identity")
        .to_owned();
    let opposing_ids: Vec<_> = opposing
        .iter()
        .map(|card| card["instanceId"].as_str().expect("opposing identity"))
        .collect();
    let mut choices = rescue_cemetery_ids(&session);
    choices.sort();
    choices.dedup();
    assert_eq!(choices, [own_id.as_str()]);
    assert!(!opposing_ids.contains(&own_id.as_str()));

    let hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let south_observation = session.observe(Seat::South);
    let (cast, returned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-rescue"
            && descriptor["cemeteryMinionInstanceId"] == own_id
    });
    assert_eq!(
        event_types(&returned),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(returned.events[1].payload["instanceId"], own_id);
    assert_eq!(
        returned.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .len(),
        hand_before
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == own_id)
    );
    assert!(cemetery_minions(&after, "north").is_empty());
    assert_eq!(cemetery_minions(&after, "south").len(), opposing.len());
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_view["players"]["north"]["hand"]["spellbook"],
        json!(hand_before)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0682_rescue_ignores_an_opposing_cemetery_minion_when_own_cemetery_is_empty() {
    let encoded = seed_opposing(682, false);
    let mut session = opening_main(&encoded);
    mill_south_opening_site(&mut session);
    let before = state(&session);
    assert!(cemetery_minions(&before, "north").is_empty());
    assert!(!cemetery_minions(&before, "south").is_empty());
    assert!(rescue_cemetery_ids(&session).is_empty());

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rescue"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    let after = state(&session);
    assert!(cemetery_minions(&after, "north").is_empty());
    assert_eq!(
        cemetery_minions(&after, "south").len(),
        cemetery_minions(&before, "south").len()
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1051_rescue_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_rescue_seed_with(1051);
    let mut setup = try_pending_deathrite_with_cemetery_minion(&encoded)
        .expect("complete Rescue Deathrite withheld setup");
    let cemetery_minion_id = setup.cemetery_minion_id.clone();
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
    assert!(
        paused["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == cemetery_minion_id)
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(rescue_cemetery_ids(session).is_empty());

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
    assert_eq!(rescue_cemetery_ids(session), [cemetery_minion_id.as_str()]);

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-rescue"
            && descriptor["cemeteryMinionInstanceId"] == cemetery_minion_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    assert!(
        state(session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == cemetery_minion_id)
    );
    assert_exact_replay(session);
}

fn rescue_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "rescue-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-rescue-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-kill": kill(),
            "north-minion": minion(),
            "north-rescue": rescue(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec![
                    "north-minion",
                    "north-kill",
                    "north-rescue",
                    "north-rescue",
                    "north-minion",
                    "north-kill",
                    "north-minion",
                    "north-rescue",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 8],
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

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(593..593 + 2048)
        .map(rescue_supplemental_manifest)
        .find(|candidate| {
            required.iter().all(|id| {
                opening_hand_spell_ids(candidate)
                    .iter()
                    .any(|card| card == *id)
            })
        })
        .expect("bounded seed with Rescue supplemental opening cards")
}

fn rescue_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-rescue")
                .count()
        })
        .unwrap_or_default()
}

fn cemetery_instance_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    cemetery_minions(snapshot, seat)
        .into_iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("cemetery minion identity")
                .to_owned()
        })
        .collect()
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

fn kill_own_minion(session: &mut Session, card_id: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let minion_id = summoned["cardInstanceId"]
        .as_str()
        .expect("summoned minion identity")
        .to_owned();
    let (_, killed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == minion_id
    });
    assert!(event_types(&killed).contains(&"minion-killed"));
    assert!(cemetery_instance_ids(&state(session), "north").contains(&minion_id));
    minion_id
}

fn cast_rescue_target(session: &mut Session, minion_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-rescue"
            && descriptor["cemeteryMinionInstanceId"] == minion_id
    });
    receipt
}

fn try_kill_own_minion(session: &mut Session, card_id: &str) -> Option<String> {
    let (summoned, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let minion_id = summoned["cardInstanceId"].as_str()?.to_owned();
    let (_, killed) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == minion_id
    })?;
    event_types(&killed)
        .contains(&"minion-killed")
        .then_some(minion_id)
}

fn advance_north_spellbook_draws(session: &mut Session, draws: usize) {
    for _ in 0..draws {
        pass_turn_to_north_spellbook(session);
    }
}

fn try_two_corpses_on_session(session: &mut Session) -> Option<[String; 2]> {
    let first = try_kill_own_minion(session, "north-minion")?;
    advance_north_spellbook_draws(session, 3);
    let second = try_kill_own_minion(session, "north-minion")?;
    (cemetery_instance_ids(&state(session), "north").len() == 2).then_some([first, second])
}

fn try_two_corpses_prefix(encoded: &str) -> Option<[String; 2]> {
    let mut session = opening_main(encoded);
    try_two_corpses_on_session(&mut session)
}

fn prepare_two_corpses(encoded: &str) -> (Session, [String; 2]) {
    let mut session = opening_main(encoded);
    let corpses = try_two_corpses_on_session(&mut session).expect("two own cemetery minions");
    (session, corpses)
}

fn seed_with_two_corpses(start: u32) -> String {
    (start..start + 512)
        .chain(593..593 + 512)
        .find_map(|seed| {
            let encoded = rescue_supplemental_manifest(seed);
            if !["north-minion", "north-kill", "north-rescue"]
                .iter()
                .all(|id| {
                    opening_hand_spell_ids(&encoded)
                        .iter()
                        .any(|card| card == *id)
                })
            {
                return None;
            }
            try_two_corpses_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed with two own cemetery minions")
}

fn try_second_rescue_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = try_kill_own_minion(&mut session, "north-minion")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-rescue"
            && descriptor["cemeteryMinionInstanceId"] == first_id
    })?;
    pass_turn_to_north_spellbook(&mut session);
    if rescue_spells_in_hand(&state(&session)) < 1 {
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
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    advance_north_spellbook_draws(&mut session, 2);
    let second_id = try_kill_own_minion(&mut session, "north-minion")?;
    (rescue_spells_in_hand(&state(&session)) >= 1
        && rescue_cemetery_ids(&session).contains(&second_id))
    .then_some((session, second_id))
}

fn seed_for_second_rescue_enemy_arrival(start: u32) -> String {
    (start..start + 512)
        .chain(593..593 + 512)
        .find_map(|seed| {
            let encoded = rescue_supplemental_manifest(seed);
            if !opening_hand_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-rescue")
            {
                return None;
            }
            try_second_rescue_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Rescue enemy-arrival setup")
}

fn try_second_rescue_new_kill_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = try_kill_own_minion(&mut session, "north-minion")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-rescue"
            && descriptor["cemeteryMinionInstanceId"] == first_id
    })?;
    pass_turn_to_north_spellbook(&mut session);
    if rescue_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    advance_north_spellbook_draws(&mut session, 2);
    let second_id = try_kill_own_minion(&mut session, "north-minion")?;
    (rescue_spells_in_hand(&state(&session)) >= 1
        && rescue_cemetery_ids(&session).contains(&second_id))
    .then_some((session, second_id))
}

fn seed_for_second_rescue_new_kill(start: u32) -> String {
    (start..start + 512)
        .chain(593..593 + 512)
        .find_map(|seed| {
            let encoded = rescue_supplemental_manifest(seed);
            if !opening_hand_spell_ids(&encoded)
                .iter()
                .any(|card| card == "north-rescue")
            {
                return None;
            }
            try_second_rescue_new_kill_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Rescue new-kill setup")
}

#[test]
fn rule_catalog_1943_rescued_minion_stays_in_hand_after_turns_pass() {
    let encoded = seed_with_start(1943, &["north-minion", "north-kill", "north-rescue"]);
    let mut session = opening_main(&encoded);
    let minion_id = kill_own_minion(&mut session, "north-minion");
    cast_rescue_target(&mut session, &minion_id);
    assert!(hand_has_instance(&state(&session), "north", &minion_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(hand_has_instance(&state(&session), "north", &minion_id));
    assert!(cemetery_instance_ids(&state(&session), "north").is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1944_second_rescue_without_a_cemetery_minion_is_a_paid_noop() {
    let encoded = seed_with_start(1944, &["north-minion", "north-kill", "north-rescue"]);
    let mut session = opening_main(&encoded);
    let minion_id = kill_own_minion(&mut session, "north-minion");
    cast_rescue_target(&mut session, &minion_id);
    pass_turn_to_north_spellbook(&mut session);
    assert!(rescue_spells_in_hand(&state(&session)) >= 1);
    assert!(rescue_cemetery_ids(&session).is_empty());
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rescue"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-returned-to-hand")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1945_second_rescue_returns_a_newly_killed_minion_after_enemy_site_placement() {
    let encoded = seed_for_second_rescue_enemy_arrival(1945);
    let (mut session, minion_id) = try_second_rescue_enemy_arrival_prefix(&encoded)
        .expect("second Rescue enemy-arrival prefix");
    let receipt = cast_rescue_target(&mut session, &minion_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), "north", &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1946_rescue_offers_every_own_cemetery_minion() {
    let encoded = seed_with_two_corpses(1946);
    let (session, minion_ids) = prepare_two_corpses(&encoded);
    let mut offered = rescue_cemetery_ids(&session);
    offered.sort();
    offered.dedup();
    assert_eq!(offered.len(), 2);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1947_rescue_leaves_an_unselected_cemetery_minion_in_place() {
    let encoded = seed_with_two_corpses(1947);
    let (mut session, minion_ids) = prepare_two_corpses(&encoded);
    let rescued_id = &minion_ids[0];
    cast_rescue_target(&mut session, rescued_id);
    assert!(hand_has_instance(&state(&session), "north", rescued_id));
    let remaining = cemetery_instance_ids(&state(&session), "north");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0], minion_ids[1]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1948_second_rescue_returns_a_newly_killed_minion() {
    let encoded = seed_for_second_rescue_new_kill(1948);
    let (mut session, minion_id) =
        try_second_rescue_new_kill_prefix(&encoded).expect("second Rescue new-kill prefix");
    let receipt = cast_rescue_target(&mut session, &minion_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), "north", &minion_id));
    assert_exact_replay(&session);
}
