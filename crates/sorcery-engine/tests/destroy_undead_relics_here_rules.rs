//! Direct proofs for destroy-undead-minions-and-artifacts-at-location-within-two-steps
//! Magic (RULE-CATALOG-0575–0576, 1052, 1098, RULE-CATALOG-1853–1858).
//!
//! Ordinary Unravel chooses a location within two measured steps of the caster
//! and destroys every Undead minion and Artifact whose cell and region match.
//! A non-Undead minion at another offered location is left unwounded. An empty
//! offered location is a paid no-op. This is one composed fact, exclusive of
//! destroy-artifacts-and-auras-at-location and kill-mortal-minions-at-location.
//!
//! 1052 covers Unravel killing a Deathrite undead minion: the controller draws
//! a site and magic-resolved only appears after deathrite settlement. 1098
//! covers Unravel withheld while Deathrites wait for ordering, until the chain
//! drains.

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

fn relic() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
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

fn unravel() -> Value {
    json!({
        "cardType": "magic",
        "destroyUndeadMinionsAndArtifactsAtLocationWithinTwoSteps": true,
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

fn unravel_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-undead-relics-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-undead-relics-here-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-undead": undead(),
            "north-unravel": unravel(),
            "south-avatar": avatar(),
            "south-dummy": beast(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-undead",
                    "north-relic",
                    "north-unravel",
                    "north-beast",
                    "north-beast",
                    "north-beast",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn unravel_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-undead-relics-here-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-undead-relics-here-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-undead": deathrite_undead(),
            "north-unravel": unravel(),
            "south-avatar": avatar(),
            "south-dummy": beast(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-undead",
                    "north-relic",
                    "north-unravel",
                    "north-beast",
                    "north-beast",
                    "north-beast",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn unravel_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-undead-relics-here-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-undead-relics-here-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-undead": undead(),
            "north-unravel": unravel(),
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
                    "north-unravel",
                    "north-unravel",
                    "north-unravel",
                    "north-unravel",
                    "north-undead",
                    "north-undead",
                    "north-relic",
                    "north-unravel",
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

fn unravel_multi_target_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-undead-relics-here-multi-target" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-undead-relics-here-multi-target-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-undead": undead(),
            "north-unravel": unravel(),
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
                    "north-undead",
                    "north-relic",
                    "north-unravel",
                    "north-undead",
                    "north-relic",
                    "north-unravel",
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
    let mut session = Session::new(encoded).expect("valid destroy-undead-relics-here session");
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

fn seed_with(required: &[&str]) -> String {
    seed_with_manifest(required, unravel_manifest)
}

fn seed_with_deathrite(required: &[&str]) -> String {
    seed_with_manifest(required, unravel_deathrite_manifest)
}

fn seed_with_manifest(required: &[&str], manifest: impl Fn(u32) -> String) -> String {
    (575..575 + 256)
        .map(manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn south_plays_c1_and_ends(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
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

fn artifact_at(session: &Session, card_id: &str, cell: &str) -> String {
    state(session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["cardId"] == card_id && artifact["location"] == cell)
        .expect("expected artifact at cell")["instanceId"]
        .as_str()
        .expect("artifact instance identity")
        .to_owned()
}

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn unravel_locations(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("unravel actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-unravel"
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

fn deathrite_unravel_manifest(seed: u32) -> String {
    let fixture = "destroy-undead-relics-here-deathrite-withheld";
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
            "north-relic": relic(),
            "north-site": earth_site(),
            "north-unravel": unravel(),
            "south-avatar": avatar(),
            "south-minion": rain_deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-unravel",
                    "north-rain",
                    "north-relic",
                    "north-unravel",
                    "north-rain",
                    "north-relic",
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

fn north_has_unravel_rain_and_relic(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-unravel", "north-rain", "north-relic"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteUnravelSetup {
    deathrite_ids: [String; 2],
    relic_id: String,
    session: Session,
}

fn try_pending_deathrite_with_nearby_relic(encoded: &str) -> Option<PendingDeathriteUnravelSetup> {
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
    if !north_has_unravel_rain_and_relic(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C4"
    })?;
    let relic_id = artifact_at(&session, "north-relic", "C4");
    if unravel_locations(&session) != ["C4"] {
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
    Some(PendingDeathriteUnravelSetup {
        deathrite_ids,
        relic_id,
        session,
    })
}

fn deathrite_unravel_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_unravel_manifest)
        .find(|candidate| try_pending_deathrite_with_nearby_relic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Unravel Magic in hand")
}

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(575..575 + 2048)
        .map(unravel_supplemental_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn unravel_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-unravel")
                .count()
        })
        .unwrap_or_default()
}

fn undead_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-undead")
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

fn cast_unravel(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unravel"
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
}

fn cast_relic_at(session: &mut Session, cell: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == cell
    });
    artifact_at(session, "north-relic", cell)
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

fn unit_absent(snapshot: &Value, instance_id: &str) -> bool {
    snapshot["realm"]["units"]
        .as_array()
        .is_none_or(|units| units.iter().all(|unit| unit["instanceId"] != instance_id))
}

fn seed_with_two_unravel_spells_in_hand_after_setup(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = unravel_supplemental_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if hand.iter().filter(|card| *card == "north-undead").count() < 1
                || !hand.iter().any(|card| card == "north-unravel")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = summon_at(&mut session, "north-undead", "C4");
            (unravel_spells_in_hand(&state(&session)) >= 2).then_some(encoded)
        })
        .expect("bounded seed with two Unravel spells in hand after setup")
}

struct SecondUnravelKillSetup {
    second_undead: String,
    session: Session,
}

fn try_second_unravel_kill_prefix(encoded: &str) -> Option<SecondUnravelKillSetup> {
    let mut session = opening_main(encoded);
    let first_undead = summon_at(&mut session, "north-undead", "C4");
    cast_unravel(&mut session, "C4");
    if !cemetery_has(&state(&session), "north", &first_undead) {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    let snap = state(&session);
    if unravel_spells_in_hand(&snap) < 1 || undead_in_hand(&snap) < 1 {
        return None;
    }
    let second_undead = summon_at(&mut session, "north-undead", "C4");
    if !unravel_locations(&session).contains(&"C4".to_owned()) {
        return None;
    }
    Some(SecondUnravelKillSetup {
        second_undead,
        session,
    })
}

fn seed_for_second_unravel_kill(start: u32) -> String {
    (start..start + 8192)
        .chain(575..575 + 8192)
        .find_map(|seed| {
            let encoded = unravel_supplemental_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-undead")
                || !hand.iter().any(|card| card == "north-unravel")
            {
                return None;
            }
            try_second_unravel_kill_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Unravel kill setup")
}

fn seed_with_undead_and_relic_at_c4(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = unravel_multi_target_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-undead")
                || !hand.iter().any(|card| card == "north-relic")
                || !hand.iter().any(|card| card == "north-unravel")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = summon_at(&mut session, "north-undead", "C4");
            try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-artifact"
                    && descriptor["cardId"] == "north-relic"
                    && descriptor["bearer"].is_null()
                    && descriptor["cell"] == "C4"
            })?;
            Some(encoded)
        })
        .expect("bounded seed reaching Undead and Artifact at C4")
}

#[test]
fn rule_catalog_0575_destroy_undead_relics_here_destroys_undead_and_artifact_and_spares_beast() {
    let encoded = seed_with(&["north-unravel", "north-undead", "north-relic"]);
    let mut session = opening_main(&encoded);
    south_plays_c1_and_ends(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    let undead_id = summon_at(&mut session, "north-undead", "C3");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let relic_id = artifact_at(&session, "north-relic", "C3");
    let before = state(&session);
    assert_eq!(unit(&before, &undead_id)["location"], "C3");
    assert_eq!(
        realm_artifact(&before, &relic_id).expect("relic at C3")["location"],
        "C3"
    );
    assert_eq!(unit(&before, &beast_id)["location"], "C4");
    assert_eq!(unit(&before, &beast_id)["damage"], 0);
    assert_eq!(unravel_locations(&session), ["C3", "C4"]);

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unravel"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "artifact-destroyed",
            "magic-resolved"
        ]
    );
    let kill = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-killed")
        .expect("minion-killed");
    assert_eq!(kill.payload["cardId"], "north-undead");
    assert_eq!(kill.payload["instanceId"], undead_id);
    assert_eq!(kill.payload["owner"], "north");
    assert_eq!(kill.payload["seat"], "north");
    assert_eq!(kill.payload["sourceInstanceId"], cast["cardInstanceId"]);
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "artifact-destroyed")
        .expect("artifact destruction");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], relic_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "damage-dealt")
    );

    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != undead_id)
    );
    assert!(realm_artifact(&after, &relic_id).is_none());
    assert_eq!(unit(&after, &beast_id)["location"], "C4");
    assert_eq!(unit(&after, &beast_id)["damage"], 0);
    assert!(cemetery_has(&after, "north", &undead_id));
    assert!(cemetery_has(&after, "north", &relic_id));
    assert!(!cemetery_has(&after, "north", &beast_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0576_destroy_undead_relics_here_empty_location_is_a_paid_noop() {
    let encoded = seed_with(&["north-unravel"]);
    let mut session = opening_main(&encoded);
    south_plays_c1_and_ends(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(unravel_locations(&session), ["C3", "C4"]);
    assert!(
        state(&session)["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unravel"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed"
                || event.event_type == "minion-died"
                || event.event_type == "artifact-destroyed")
    );
    assert!(
        state(&session)["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1052_destroy_undead_relics_deathrite_draws_for_controller_on_kill() {
    let encoded = (1052..1052 + 256)
        .map(unravel_deathrite_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|card| card == "north-undead")
                && hand.iter().any(|card| card == "north-relic")
                && hand.iter().any(|card| card == "north-unravel")
        })
        .unwrap_or_else(|| seed_with_deathrite(&["north-undead", "north-relic", "north-unravel"]));
    let mut session = opening_main(&encoded);
    south_plays_c1_and_ends(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let undead_id = summon_at(&mut session, "north-undead", "C3");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let relic_id = artifact_at(&session, "north-relic", "C3");
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unravel"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "site-drawn",
            "minion-died",
            "artifact-destroyed",
            "magic-resolved",
        ]
    );
    let kill = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-killed")
        .expect("minion-killed");
    assert_eq!(kill.payload["cardId"], "north-undead");
    assert_eq!(kill.payload["instanceId"], undead_id);
    assert_eq!(kill.payload["owner"], "north");
    assert_eq!(kill.payload["seat"], "north");
    assert_eq!(kill.payload["sourceInstanceId"], cast["cardInstanceId"]);
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");
    assert_eq!(drawn.payload["sourceInstanceId"], undead_id);
    let types = event_types(&receipt);
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        site_drawn < magic_resolved,
        "magic-resolved must follow deathrite site-drawn"
    );
    assert_eq!(types.last(), Some(&"magic-resolved"));
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "artifact-destroyed")
        .expect("artifact destruction");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], relic_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );

    let after = state(&session);
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != undead_id)
    );
    assert!(realm_artifact(&after, &relic_id).is_none());
    assert!(cemetery_has(&after, "north", &undead_id));
    assert!(cemetery_has(&after, "north", &relic_id));
    assert_eq!(atlas_len(&after, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&after, "south"), south_atlas);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1098_destroy_undead_relics_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_unravel_seed_with(1098);
    let mut setup = try_pending_deathrite_with_nearby_relic(&encoded)
        .expect("complete destroy-undead-relics Deathrite withheld setup");
    let relic_id = setup.relic_id.clone();
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
        realm_artifact(&paused, &relic_id).expect("nearby relic")["location"],
        "C4"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(unravel_locations(session).is_empty());

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
    assert_eq!(
        realm_artifact(&resumed, &relic_id).expect("nearby relic remains")["location"],
        "C4"
    );
    assert_eq!(unravel_locations(session), ["C4"]);

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unravel"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-destroyed", "magic-resolved"]
    );
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "artifact-destroyed")
        .expect("artifact destruction");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], relic_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert!(realm_artifact(&state(session), &relic_id).is_none());
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1853_killed_undead_stays_in_cemetery_after_turns_pass() {
    let encoded = seed_with_start(1853, &["north-undead", "north-relic", "north-unravel"]);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let relic_id = cast_relic_at(&mut session, "C4");
    cast_unravel(&mut session, "C4");
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    advance_full_round(&mut session);
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1854_second_unravel_without_an_undead_or_artifact_is_still_a_paid_noop() {
    let encoded = seed_with_two_unravel_spells_in_hand_after_setup(1854);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let first = cast_unravel(&mut session, "C4");
    assert!(event_types(&first).contains(&"minion-killed"));
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    let second = cast_unravel(&mut session, "C4");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!second.events.iter().any(|event| {
        event.event_type == "minion-killed"
            || event.event_type == "minion-died"
            || event.event_type == "artifact-destroyed"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1855_second_unravel_kills_a_newly_arrived_undead_at_the_same_cell() {
    let encoded = seed_with_two_unravel_spells_in_hand_after_setup(1855);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    cast_unravel(&mut session, "C4");
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    let nearby_id = south_raids_c4(&mut session);
    let killed = cast_unravel(&mut session, "C4");
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["instanceId"], nearby_id);
    assert!(unit_absent(&state(&session), &nearby_id));
    assert!(cemetery_has(&state(&session), "south", &nearby_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1856_unravel_kills_every_undead_and_destroys_every_artifact_sharing_the_target_cell()
 {
    let encoded = seed_with_undead_and_relic_at_c4(1856);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let relic_id = cast_relic_at(&mut session, "C4");
    let destroyed = cast_unravel(&mut session, "C4");
    let kills: Vec<_> = destroyed
        .events
        .iter()
        .filter(|event| event.event_type == "minion-killed")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("killed minion")
                .to_owned()
        })
        .collect();
    let relic_destroys: Vec<_> = destroyed
        .events
        .iter()
        .filter(|event| event.event_type == "artifact-destroyed")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("destroyed artifact")
                .to_owned()
        })
        .collect();
    assert_eq!(kills, vec![undead_id.clone()]);
    assert_eq!(relic_destroys, vec![relic_id.clone()]);
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(realm_artifact(&state(&session), &relic_id).is_none());
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1857_unravel_leaves_a_far_undead_untouched() {
    let encoded = seed_with_start(1857, &["north-undead", "north-relic", "north-unravel"]);
    let mut session = opening_main(&encoded);
    let undead_id = summon_at(&mut session, "north-undead", "C4");
    let relic_id = cast_relic_at(&mut session, "C4");
    let far_id = south_plays_c1_and_summons_undead(&mut session);
    cast_unravel(&mut session, "C4");
    assert!(unit_absent(&state(&session), &undead_id));
    assert!(cemetery_has(&state(&session), "north", &undead_id));
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert!(!cemetery_has(&state(&session), "south", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1858_second_unravel_kills_a_newly_summoned_undead() {
    let encoded = seed_for_second_unravel_kill(1858);
    let SecondUnravelKillSetup {
        mut session,
        second_undead,
    } = try_second_unravel_kill_prefix(&encoded).expect("second Unravel kill prefix");
    let killed = cast_unravel(&mut session, "C4");
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["instanceId"], second_undead);
    assert!(unit_absent(&state(&session), &second_undead));
    assert!(cemetery_has(&state(&session), "north", &second_undead));
    assert_exact_replay(&session);
}
