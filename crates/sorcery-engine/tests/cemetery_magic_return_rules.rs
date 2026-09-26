//! Direct proofs for cemetery Magic return (RULE-CATALOG-0635–0636, 1059,
//! RULE-CATALOG-2143–2148).
//!
//! Cemetery Magic return is the Rescue sibling for Magic cards: it offers
//! only Magic in the caster's own cemetery and returns the unchanged instance
//! to the hidden Spellbook hand. An empty own cemetery is a paid no-choice.
//! While Deathrites wait for ordering, cemetery Magic return stays withheld
//! until the chain drains.

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
        "returnTargetMagicFromOwnCemetery": true,
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

fn cemetery_magic_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-magic-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-magic-return-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-return": return_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
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
    let mut session = Session::new(encoded).expect("valid cemetery-magic-return session");
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

fn seed_with(start: u32) -> String {
    (start..start + 256)
        .map(cemetery_magic_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|card| card == "north-return")
        })
        .expect("bounded seed with cemetery Magic return in the opening hand")
}

fn cemetery_magic_cast_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("cemetery Magic actions")
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
fn rule_catalog_0635_cemetery_magic_return_restores_own_cemetery_magic_to_hidden_hand() {
    let encoded = seed_with(635);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_magic_cast_ids(&session), Vec::<String>::new());
    let (seed_descriptor, seed_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert_eq!(event_types(&seed_receipt), ["magic-cast", "magic-resolved"]);
    let seed_id = seed_descriptor["cardInstanceId"]
        .as_str()
        .expect("seed Magic identity")
        .to_owned();
    let before = state(&session);
    assert!(
        before["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == seed_id)
    );
    let cemetery_targets = cemetery_magic_cast_ids(&session);
    assert!(!cemetery_targets.is_empty());
    assert!(cemetery_targets.iter().all(|id| id == &seed_id));
    let before_hand_count = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let south_observation = session.observe(Seat::South);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == seed_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "magic-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["cardId"], "north-return");
    assert_eq!(receipt.events[1].payload["instanceId"], seed_id);
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-returned-to-hand"
                || event.event_type == "ward-broken")
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
            .any(|card| card["instanceId"] == seed_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != seed_id)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_view["players"]["north"]["hand"]["spellbook"], 2);
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0636_cemetery_magic_return_is_a_paid_noop_without_cemetery_magic() {
    let encoded = seed_with(636);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_magic_cast_ids(&session), Vec::<String>::new());
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "magic-returned-to-hand"
                || event.event_type == "minion-returned-to-hand")
    );
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

fn deathrite_cemetery_magic_manifest(seed: u32) -> String {
    let fixture = "cemetery-magic-return-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
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
                    "north-return",
                    "north-rain",
                    "north-return",
                    "north-rain",
                    "north-return",
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

fn north_has_return_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-return", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteCemeteryMagicSetup {
    cemetery_magic_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_cemetery_magic(
    encoded: &str,
) -> Option<PendingDeathriteCemeteryMagicSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let seeded = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    })?;
    let cemetery_magic_id = seeded.0["cardInstanceId"].as_str()?.to_owned();
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
    if cemetery_magic_cast_ids(&session).is_empty() {
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
    Some(PendingDeathriteCemeteryMagicSetup {
        cemetery_magic_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_cemetery_magic_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cemetery_magic_manifest)
        .find(|candidate| try_pending_deathrite_with_cemetery_magic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with cemetery Magic return in hand")
}

#[test]
fn rule_catalog_1059_cemetery_magic_return_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cemetery_magic_seed_with(1059);
    let mut setup = try_pending_deathrite_with_cemetery_magic(&encoded)
        .expect("complete cemetery Magic return Deathrite withheld setup");
    let cemetery_magic_id = setup.cemetery_magic_id.clone();
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
    assert!(
        paused["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == cemetery_magic_id)
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(cemetery_magic_cast_ids(session).is_empty());

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
    assert!(
        cemetery_magic_cast_ids(session)
            .iter()
            .any(|id| id == &cemetery_magic_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == cemetery_magic_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "magic-returned-to-hand", "magic-resolved"]
    );
    assert!(
        state(session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["instanceId"] == cemetery_magic_id)
    );
    assert_exact_replay(session);
}

fn cemetery_magic_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-magic-return-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-magic-return-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
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
                    "north-return",
                    "north-rain",
                    "north-return",
                    "north-rain",
                    "north-return",
                    "north-rain",
                    "north-return",
                    "north-rain",
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

fn opening_card_count(encoded: &str, card_id: &str) -> usize {
    opening_spell_ids(encoded)
        .into_iter()
        .filter(|card| card == card_id)
        .count()
}

fn supplemental_seed_with_start(start: u32, min_return: usize, min_rain: usize) -> String {
    (start..start + 2048)
        .chain(635..635 + 2048)
        .map(cemetery_magic_supplemental_manifest)
        .find(|candidate| {
            opening_card_count(candidate, "north-return") >= min_return
                && opening_card_count(candidate, "north-rain") >= min_rain
        })
        .expect("bounded seed with cemetery Magic return supplemental opening cards")
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

fn hand_has_instance(snapshot: &Value, instance_id: &str) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["instanceId"] == instance_id))
}

fn cemetery_instance_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("cemetery identity")
                .to_owned()
        })
        .collect()
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

fn try_seed_untargeted_return(session: &mut Session) -> Option<String> {
    let (descriptor, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor.get("cemeteryMinionInstanceId").is_none()
    })?;
    descriptor["cardInstanceId"].as_str().map(ToOwned::to_owned)
}

fn try_cast_rain(session: &mut Session) -> Option<String> {
    let (descriptor, _) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    descriptor["cardInstanceId"].as_str().map(ToOwned::to_owned)
}

fn cast_return_target(session: &mut Session, magic_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == magic_id
    });
    receipt
}

fn try_two_cemetery_magics_on_session(session: &mut Session) -> Option<[String; 2]> {
    let first = try_seed_untargeted_return(session)?;
    let second = try_cast_rain(session)?;
    let mut offered = cemetery_magic_cast_ids(session);
    offered.sort();
    offered.dedup();
    (offered.len() == 2 && offered.contains(&first) && offered.contains(&second))
        .then_some([first, second])
}

fn try_two_cemetery_magics_prefix(encoded: &str) -> Option<[String; 2]> {
    let mut session = opening_main(encoded);
    try_two_cemetery_magics_on_session(&mut session)
}

fn prepare_two_cemetery_magics(encoded: &str) -> (Session, [String; 2]) {
    let mut session = opening_main(encoded);
    let magics =
        try_two_cemetery_magics_on_session(&mut session).expect("two own cemetery Magic cards");
    (session, magics)
}

fn seed_with_two_cemetery_magics(start: u32) -> String {
    (start..start + 2048)
        .chain(635..635 + 2048)
        .find_map(|seed| {
            let encoded = cemetery_magic_supplemental_manifest(seed);
            if opening_card_count(&encoded, "north-return") < 1
                || opening_card_count(&encoded, "north-rain") < 1
            {
                return None;
            }
            try_two_cemetery_magics_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed with two own cemetery Magic cards")
}

fn try_second_return_new_magic_prefix(
    encoded: &str,
    enemy_site: bool,
) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = try_seed_untargeted_return(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == first_id
    })?;
    pass_turn_to_north_spellbook(&mut session);
    if enemy_site {
        end_turn_if_offered(&mut session);
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw"
                && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
        })?;
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "south-site"
                && descriptor["cell"] == "C2"
        })?;
        end_turn_if_offered(&mut session);
        try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })?;
    }
    for _ in 0..4 {
        if rain_spells_in_hand(&state(&session)) >= 1
            && return_spells_in_hand(&state(&session)) >= 1
        {
            break;
        }
        pass_turn_to_north_spellbook(&mut session);
    }
    if rain_spells_in_hand(&state(&session)) < 1 || return_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    let rain_id = try_cast_rain(&mut session)?;
    (return_spells_in_hand(&state(&session)) >= 1
        && cemetery_magic_cast_ids(&session).contains(&rain_id))
    .then_some((session, rain_id))
}

fn seed_for_second_return_new_magic(start: u32, enemy_site: bool) -> String {
    (start..start + 8192)
        .chain(635..635 + 8192)
        .find_map(|seed| {
            let encoded = cemetery_magic_supplemental_manifest(seed);
            if opening_card_count(&encoded, "north-return") < 1 {
                return None;
            }
            try_second_return_new_magic_prefix(&encoded, enemy_site).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Magic return new-arrival setup")
}

#[test]
fn rule_catalog_2143_returned_magic_stays_in_hand_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2143, 2, 0);
    let mut session = opening_main(&encoded);
    let seed_id = try_seed_untargeted_return(&mut session).expect("empty-cemetery seed cast");
    let receipt = cast_return_target(&mut session, &seed_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "magic-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), &seed_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(hand_has_instance(&state(&session), &seed_id));
    assert!(!cemetery_instance_ids(&state(&session)).contains(&seed_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2144_second_cemetery_magic_return_without_cemetery_magic_is_a_paid_noop() {
    let encoded = supplemental_seed_with_start(2144, 2, 0);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_magic_cast_ids(&session), Vec::<String>::new());
    assert!(return_spells_in_hand(&state(&session)) >= 2);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor.get("cemeteryMinionInstanceId").is_none()
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "magic-returned-to-hand"
                || event.event_type == "minion-returned-to-hand")
    );
    assert!(return_spells_in_hand(&state(&session)) >= 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2145_second_cemetery_magic_return_returns_a_newly_arrived_magic_after_enemy_site_placement()
 {
    let encoded = seed_for_second_return_new_magic(2145, true);
    let (mut session, magic_id) = try_second_return_new_magic_prefix(&encoded, true)
        .expect("second cemetery Magic return enemy-arrival prefix");
    let receipt = cast_return_target(&mut session, &magic_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "magic-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), &magic_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2146_cemetery_magic_return_offers_every_own_cemetery_magic() {
    let encoded = seed_with_two_cemetery_magics(2146);
    let (session, magic_ids) = prepare_two_cemetery_magics(&encoded);
    let mut offered = cemetery_magic_cast_ids(&session);
    offered.sort();
    offered.dedup();
    assert_eq!(offered.len(), 2);
    for magic_id in &magic_ids {
        assert!(offered.contains(magic_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2147_cemetery_magic_return_leaves_an_unselected_cemetery_magic_in_place() {
    let encoded = seed_with_two_cemetery_magics(2147);
    let (mut session, magic_ids) = prepare_two_cemetery_magics(&encoded);
    let returned_id = &magic_ids[0];
    cast_return_target(&mut session, returned_id);
    assert!(hand_has_instance(&state(&session), returned_id));
    let remaining = cemetery_instance_ids(&state(&session));
    assert!(remaining.contains(&magic_ids[1]));
    assert!(!remaining.contains(returned_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2148_second_cemetery_magic_return_returns_a_newly_arrived_cemetery_magic() {
    let encoded = seed_for_second_return_new_magic(2148, false);
    let (mut session, magic_id) = try_second_return_new_magic_prefix(&encoded, false)
        .expect("second cemetery Magic return new-arrival prefix");
    let receipt = cast_return_target(&mut session, &magic_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "magic-returned-to-hand", "magic-resolved"]
    );
    assert!(hand_has_instance(&state(&session), &magic_id));
    assert_exact_replay(&session);
}
