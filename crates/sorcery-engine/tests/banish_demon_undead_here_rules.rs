//! Direct proofs for banish-demon-and-undead-minions-at-location-within-two-steps
//! Magic (RULE-CATALOG-0573–0574, 1029, 1097, RULE-CATALOG-1843–1848).
//!
//! 1029 covers exorcism banishing a Deathrite undead minion: the controller
//! draws a site and magic-resolved only appears after deathrite settlement.
//! 1097 covers Exorcism withheld while Deathrites wait for ordering, until
//! the chain drains.
//!
//! Ordinary Magic offers existing locations within two measured cardinal
//! steps of the caster footprint and banishes every Demon or Undead minion
//! there. Living non-Demon non-Undead minions are left untouched. An empty
//! qualifying set is a paid no-op. Banished minions leave the game without
//! entering the cemetery.

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

fn demon() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "demon": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn undead() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "undead": true,
    })
}

fn deathrite_undead() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 3,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "undead": true,
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

fn exorcism() -> Value {
    json!({
        "banishDemonAndUndeadMinionsAtLocationWithinTwoSteps": true,
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

fn exorcism_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "banish-demon-undead-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-banish-demon-undead-here-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-demon": demon(),
            "north-exorcism": exorcism(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
            "south-undead": undead(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-demon",
                    "north-beast",
                    "north-exorcism",
                    "north-demon",
                    "north-beast",
                    "north-exorcism",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-undead"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn exorcism_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "banish-demon-undead-here-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-banish-demon-undead-here-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-demon": demon(),
            "north-exorcism": exorcism(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
            "south-undead": {
                "attack": 1,
                "cardType": "minion",
                "defense": 3,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                "undead": true,
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-demon",
                    "north-beast",
                    "north-exorcism",
                    "north-exorcism",
                    "north-demon",
                    "north-beast",
                    "north-exorcism",
                    "north-exorcism",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-undead"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn exorcism_multi_target_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "banish-demon-undead-here-multi-target" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-banish-demon-undead-here-multi-target-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-demon": demon(),
            "north-exorcism": exorcism(),
            "north-site": earth_site(),
            "north-undead": undead(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
            "south-undead": {
                "attack": 1,
                "cardType": "minion",
                "defense": 3,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                "undead": true,
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-demon",
                    "north-undead",
                    "north-exorcism",
                    "north-demon",
                    "north-undead",
                    "north-exorcism",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-undead"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn exorcism_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "banish-demon-undead-here-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-banish-demon-undead-here-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-exorcism": exorcism(),
            "north-site": earth_site(),
            "north-undead": deathrite_undead(),
            "south-avatar": avatar(),
            "south-site": earth_site(),
            "south-undead": undead(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-undead",
                    "north-beast",
                    "north-exorcism",
                    "north-undead",
                    "north-beast",
                    "north-exorcism",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-undead"; 6],
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
    let mut session = Session::new(encoded).expect("valid banish-demon-undead-here session");
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

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn exorcism_locations(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("exorcism actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-exorcism"
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
    seed_with_manifest(573, required, exorcism_manifest)
}

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(573..573 + 2048)
        .map(exorcism_supplemental_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn seed_with_deathrite(required: &[&str]) -> String {
    seed_with_manifest(573, required, exorcism_deathrite_manifest)
}

fn seed_with_manifest(start: u32, required: &[&str], manifest: impl Fn(u32) -> String) -> String {
    (start..start + 256)
        .map(manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn exorcism_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-exorcism")
                .count()
        })
        .unwrap_or_default()
}

fn demons_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-demon")
                .count()
        })
        .unwrap_or_default()
}

fn advance_full_round(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn cast_exorcism(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-exorcism"
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn south_raids_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    let (nearby, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-undead"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    nearby["cardInstanceId"]
        .as_str()
        .expect("nearby enemy identity")
        .to_owned()
}

fn seed_with_two_exorcism_spells_in_hand_after_setup(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = exorcism_supplemental_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if hand.iter().filter(|card| *card == "north-demon").count() < 1
                || !hand.iter().any(|card| card == "north-exorcism")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = summon_at(&mut session, "north-demon", "C4");
            (exorcism_spells_in_hand(&state(&session)) >= 2).then_some(encoded)
        })
        .expect("bounded seed with two Exorcism spells in hand after setup")
}

struct SecondExorcismBanishSetup {
    second_demon: String,
    session: Session,
}

fn try_second_exorcism_banish_prefix(encoded: &str) -> Option<SecondExorcismBanishSetup> {
    let mut session = opening_main(encoded);
    let first_demon = summon_at(&mut session, "north-demon", "C4");
    cast_exorcism(&mut session, "C4");
    if state(&session)["realm"]["units"]
        .as_array()
        .is_some_and(|units| units.iter().any(|unit| unit["instanceId"] == first_demon))
    {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    let snap = state(&session);
    if exorcism_spells_in_hand(&snap) < 1 || demons_in_hand(&snap) < 1 {
        return None;
    }
    let second_demon = summon_at(&mut session, "north-demon", "C4");
    Some(SecondExorcismBanishSetup {
        second_demon,
        session,
    })
}

fn seed_for_second_exorcism_banish(start: u32) -> String {
    (start..start + 8192)
        .find_map(|seed| {
            let encoded = exorcism_supplemental_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-demon")
                || !hand.iter().any(|card| card == "north-exorcism")
            {
                return None;
            }
            try_second_exorcism_banish_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Exorcism banish setup")
}

fn seed_with_demon_and_undead_at_c4(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = exorcism_multi_target_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-demon")
                || !hand.iter().any(|card| card == "north-undead")
                || !hand.iter().any(|card| card == "north-exorcism")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = summon_at(&mut session, "north-demon", "C4");
            try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "north-undead"
                    && descriptor["cell"] == "C4"
                    && descriptor["region"].is_null()
            })?;
            Some(encoded)
        })
        .expect("bounded seed reaching Demon and Undead at C4")
}

fn unit_absent(snapshot: &Value, instance_id: &str) -> bool {
    snapshot["realm"]["units"]
        .as_array()
        .is_none_or(|units| units.iter().all(|unit| unit["instanceId"] != instance_id))
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
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

fn south_plays_c1_and_summons_undead(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let far_id = summon_at(session, "south-undead", "C1");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    far_id
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

fn deathrite_exorcism_manifest(seed: u32) -> String {
    let fixture = "banish-demon-undead-here-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-demon": demon(),
            "north-exorcism": exorcism(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": rain_deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-demon",
                    "north-exorcism",
                    "north-rain",
                    "north-rain",
                    "north-exorcism",
                    "north-exorcism",
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

fn north_has_exorcism_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-exorcism", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteExorcismSetup {
    deathrite_ids: [String; 2],
    occupant_id: String,
    session: Session,
}

fn try_pending_deathrite_with_ready_occupant(
    encoded: &str,
) -> Option<PendingDeathriteExorcismSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let occupant = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-demon"
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
    if !north_has_exorcism_and_rain(&state(&session)) {
        return None;
    }
    if exorcism_locations(&session) != ["C4"] {
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
    Some(PendingDeathriteExorcismSetup {
        deathrite_ids,
        occupant_id,
        session,
    })
}

fn deathrite_exorcism_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_exorcism_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_occupant(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Exorcism Magic in hand")
}

#[test]
fn rule_catalog_0573_banishes_demon_at_location_and_spares_beast_and_far_undead() {
    let encoded = seed_with(&["north-demon", "north-beast", "north-exorcism"]);
    let mut session = opening_main(&encoded);
    let demon_id = summon_at(&mut session, "north-demon", "C4");
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    let far_id = south_plays_c1_and_summons_undead(&mut session);

    let before = state(&session);
    assert_eq!(unit(&before, &demon_id)["location"], "C4");
    assert_eq!(unit(&before, &beast_id)["location"], "C4");
    assert_eq!(unit(&before, &beast_id)["damage"], 0);
    assert_eq!(unit(&before, &far_id)["location"], "C1");
    assert_eq!(exorcism_locations(&session), ["C4"]);
    assert!(!exorcism_locations(&session).contains(&"C1".to_owned()));

    let (cast, banished) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-exorcism"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&banished),
        ["magic-cast", "minion-banished", "magic-resolved"]
    );
    let banish = banished
        .events
        .iter()
        .find(|event| event.event_type == "minion-banished")
        .expect("minion-banished");
    assert_eq!(banish.payload["cardId"], "north-demon");
    assert_eq!(banish.payload["instanceId"], demon_id);
    assert_eq!(banish.payload["owner"], "north");
    assert_eq!(cast["cardId"], "north-exorcism");
    assert!(
        !banished
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed"
                || event.event_type == "minion-died"
                || event.event_type == "damage-dealt")
    );

    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != demon_id)
    );
    assert_eq!(unit(&after, &beast_id)["location"], "C4");
    assert_eq!(unit(&after, &beast_id)["damage"], 0);
    assert_eq!(unit(&after, &far_id)["location"], "C1");
    assert_eq!(unit(&after, &far_id)["damage"], 0);
    assert!(!cemetery_has(&after, "north", &demon_id));
    assert!(!cemetery_has(&after, "south", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0574_location_with_only_a_beast_is_a_paid_noop() {
    let encoded = seed_with(&["north-beast", "north-exorcism"]);
    let mut session = opening_main(&encoded);
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    assert_eq!(exorcism_locations(&session), ["C4"]);
    assert_eq!(unit(&state(&session), &beast_id)["damage"], 0);

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-exorcism"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert!(
        !resolved
            .events
            .iter()
            .any(|event| event.event_type == "minion-banished"
                || event.event_type == "minion-killed"
                || event.event_type == "minion-died")
    );

    let after = state(&session);
    assert_eq!(unit(&after, &beast_id)["location"], "C4");
    assert_eq!(unit(&after, &beast_id)["damage"], 0);
    assert!(!cemetery_has(&after, "north", &beast_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1843_banished_demon_stays_absent_after_turns_pass() {
    let encoded = seed_with_start(1843, &["north-demon", "north-exorcism"]);
    let mut session = opening_main(&encoded);
    let demon_id = summon_at(&mut session, "north-demon", "C4");
    cast_exorcism(&mut session, "C4");
    assert!(unit_absent(&state(&session), &demon_id));
    assert!(!cemetery_has(&state(&session), "north", &demon_id));
    advance_full_round(&mut session);
    assert!(unit_absent(&state(&session), &demon_id));
    assert!(!cemetery_has(&state(&session), "north", &demon_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1844_second_exorcism_without_a_demon_or_undead_is_still_a_paid_noop() {
    let encoded = seed_with_two_exorcism_spells_in_hand_after_setup(1844);
    let mut session = opening_main(&encoded);
    let demon_id = summon_at(&mut session, "north-demon", "C4");
    let first = cast_exorcism(&mut session, "C4");
    assert!(event_types(&first).contains(&"minion-banished"));
    assert!(unit_absent(&state(&session), &demon_id));
    assert!(!cemetery_has(&state(&session), "north", &demon_id));
    let second = cast_exorcism(&mut session, "C4");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(
        !second
            .events
            .iter()
            .any(|event| event.event_type == "minion-banished")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1845_second_exorcism_banishes_a_newly_arrived_undead_at_the_same_cell() {
    let encoded = seed_with_two_exorcism_spells_in_hand_after_setup(1845);
    let mut session = opening_main(&encoded);
    let demon_id = summon_at(&mut session, "north-demon", "C4");
    cast_exorcism(&mut session, "C4");
    assert!(unit_absent(&state(&session), &demon_id));
    let nearby_id = south_raids_c4(&mut session);
    let banished = cast_exorcism(&mut session, "C4");
    assert_eq!(
        event_types(&banished),
        ["magic-cast", "minion-banished", "magic-resolved"]
    );
    assert_eq!(banished.events[1].payload["instanceId"], nearby_id);
    assert!(unit_absent(&state(&session), &nearby_id));
    assert!(!cemetery_has(&state(&session), "south", &nearby_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1846_exorcism_banishes_every_demon_and_undead_sharing_the_target_cell() {
    let encoded = seed_with_demon_and_undead_at_c4(1846);
    let mut session = opening_main(&encoded);
    let demon_id = summon_at(&mut session, "north-demon", "C4");
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let banished = cast_exorcism(&mut session, "C4");
    let removed_ids: Vec<_> = banished
        .events
        .iter()
        .filter(|event| event.event_type == "minion-banished")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("banished minion")
                .to_owned()
        })
        .collect();
    assert_eq!(removed_ids.len(), 2);
    assert!(removed_ids.contains(&demon_id));
    assert!(removed_ids.contains(&undead_id));
    assert!(unit_absent(&state(&session), &demon_id));
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(!cemetery_has(&state(&session), "north", &demon_id));
    assert!(!cemetery_has(&state(&session), "north", &undead_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1847_exorcism_leaves_a_far_undead_untouched() {
    let encoded = seed_with_start(1847, &["north-demon", "north-beast", "north-exorcism"]);
    let mut session = opening_main(&encoded);
    let demon_id = summon_at(&mut session, "north-demon", "C4");
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    let far_id = south_plays_c1_and_summons_undead(&mut session);
    cast_exorcism(&mut session, "C4");
    assert!(unit_absent(&state(&session), &demon_id));
    assert!(!cemetery_has(&state(&session), "north", &demon_id));
    assert_eq!(unit(&state(&session), &beast_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert!(!cemetery_has(&state(&session), "south", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1848_second_exorcism_banishes_a_newly_summoned_demon() {
    let encoded = seed_for_second_exorcism_banish(1848);
    let SecondExorcismBanishSetup {
        mut session,
        second_demon,
    } = try_second_exorcism_banish_prefix(&encoded).expect("second Exorcism banish prefix");
    let banished = cast_exorcism(&mut session, "C4");
    assert_eq!(
        event_types(&banished),
        ["magic-cast", "minion-banished", "magic-resolved"]
    );
    assert_eq!(banished.events[1].payload["instanceId"], second_demon);
    assert!(unit_absent(&state(&session), &second_demon));
    assert!(!cemetery_has(&state(&session), "north", &second_demon));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1029_banish_demon_undead_deathrite_draws_for_controller_on_kill() {
    let encoded = (1029..1029 + 256)
        .map(exorcism_deathrite_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|card| card == "north-undead")
                && hand.iter().any(|card| card == "north-exorcism")
        })
        .unwrap_or_else(|| seed_with_deathrite(&["north-undead", "north-exorcism"]));
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (_, banished) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-exorcism"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&banished),
        [
            "magic-cast",
            "minion-banished",
            "site-drawn",
            "magic-resolved"
        ]
    );
    let banish = banished
        .events
        .iter()
        .find(|event| event.event_type == "minion-banished")
        .expect("minion-banished");
    assert_eq!(banish.payload["cardId"], "north-undead");
    assert_eq!(banish.payload["instanceId"], undead_id);
    assert_eq!(banish.payload["owner"], "north");
    let drawn = banished
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");
    assert_eq!(drawn.payload["sourceInstanceId"], undead_id);
    let site_drawn = event_types(&banished)
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let magic_resolved = event_types(&banished)
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        site_drawn < magic_resolved,
        "magic-resolved must follow deathrite site-drawn"
    );
    assert_eq!(event_types(&banished).last(), Some(&"magic-resolved"));
    assert!(
        !banished
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed" || event.event_type == "minion-died")
    );

    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != undead_id)
    );
    assert!(!cemetery_has(&after, "north", &undead_id));
    assert_eq!(atlas_len(&after, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&after, "south"), south_atlas);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1097_banish_demon_undead_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_exorcism_seed_with(1097);
    let mut setup = try_pending_deathrite_with_ready_occupant(&encoded)
        .expect("complete Exorcism Deathrite withheld setup");
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
    assert!(exorcism_locations(session).is_empty());

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
    assert_eq!(exorcism_locations(session), ["C4"]);

    let (_, banished) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-exorcism"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&banished),
        ["magic-cast", "minion-banished", "magic-resolved"]
    );
    let banish = banished
        .events
        .iter()
        .find(|event| event.event_type == "minion-banished")
        .expect("minion-banished");
    assert_eq!(banish.payload["cardId"], "north-demon");
    assert_eq!(banish.payload["instanceId"], occupant_id);
    assert_eq!(banish.payload["owner"], "north");
    assert!(
        !banished
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed"
                || event.event_type == "minion-died"
                || event.event_type == "damage-dealt")
    );
    let after = state(session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != occupant_id)
    );
    assert!(!cemetery_has(&after, "north", &occupant_id));
    assert_exact_replay(session);
}
