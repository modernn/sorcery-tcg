//! Direct proofs for destroy-artifacts-and-auras-at-location-within-two-steps
//! Magic (RULE-CATALOG-0569–0570, RULE-CATALOG-1088, RULE-CATALOG-1823–1828).
//!
//! Ordinary Magic chooses a location within two measured steps of the caster
//! and destroys every Artifact whose cell and region match, plus every Aura
//! occupying that cell. Tokens banish. An empty offered location is a paid
//! no-op. This is one composed fact, not destroy-target-artifact plus
//! destroy-target-aura. While Deathrites wait for ordering, this Magic stays
//! withheld until the chain drains.

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

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
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

fn unmake() -> Value {
    json!({
        "cardType": "magic",
        "destroyArtifactsAndAurasAtLocationWithinTwoSteps": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn veil() -> Value {
    json!({
        "affectedSitesAreFlooded": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn destroy_relics_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "destroy-relics-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-destroy-relics-here-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-relic": relic(),
            "north-unmake": unmake(),
            "north-site": earth_site(),
            "north-veil": veil(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-relic": relic(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-relic",
                    "north-relic",
                    "north-relic",
                    "north-unmake",
                    "north-unmake",
                    "north-veil",
                    "north-unmake",
                    "north-unmake",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-relic",
                    "south-relic",
                    "south-relic",
                    "south-dummy",
                    "south-dummy",
                    "south-dummy",
                ],
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
    let mut session = Session::new(encoded).expect("valid destroy-relics-here session");
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
    seed_with_start(569, required)
}

fn seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 256)
        .map(destroy_relics_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn unmake_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-unmake")
                .count()
        })
        .unwrap_or_default()
}

fn relics_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-relic")
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

fn cast_unmake(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unmake"
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

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

fn cast_veil_at(session: &mut Session, cell: &str) -> String {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-veil"
            && cells_include(descriptor, cell)
    });
    aura_at(session, "north-veil", cell)
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn aura_at(session: &Session, card_id: &str, cell: &str) -> String {
    state(session)["realm"]["auras"]
        .as_array()
        .expect("realm auras")
        .iter()
        .find(|aura| {
            aura["cardId"] == card_id
                && aura["cells"]
                    .as_array()
                    .is_some_and(|cells| cells.iter().any(|value| value == cell))
        })
        .expect("expected aura covering cell")["instanceId"]
        .as_str()
        .expect("aura instance identity")
        .to_owned()
}

fn realm_aura<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["auras"]
        .as_array()?
        .iter()
        .find(|aura| aura["instanceId"] == instance_id)
}

fn seed_with_two_unmake_spells_in_hand_after_setup(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = destroy_relics_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-relic")
                || !hand.iter().any(|card| card == "north-unmake")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = cast_relic_at(&mut session, "C4");
            (unmake_spells_in_hand(&state(&session)) >= 2).then_some(encoded)
        })
        .expect("bounded seed with two Unmake spells in hand after setup")
}

fn south_places_relic_at_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-dummy"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let bearer_id = state(session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "south-dummy")
        .expect("south courier")["instanceId"]
        .clone();
    let (cast, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-relic"
            && descriptor["bearer"]["instanceId"] == bearer_id
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    cast["cardInstanceId"]
        .as_str()
        .expect("south relic instance")
        .to_owned()
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

struct SecondUnmakeDestroySetup {
    second_relic: String,
    session: Session,
}

fn try_second_unmake_destroy_prefix(encoded: &str) -> Option<SecondUnmakeDestroySetup> {
    let mut session = opening_main(encoded);
    let first_relic = cast_relic_at(&mut session, "C4");
    cast_unmake(&mut session, "C4");
    if !cemetery_has(&state(&session), "north", &first_relic) {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    let snap = state(&session);
    if unmake_spells_in_hand(&snap) < 1 || relics_in_hand(&snap) < 1 {
        return None;
    }
    let second_relic = cast_relic_at(&mut session, "C4");
    unmake_locations(&session)
        .contains(&"C4".to_owned())
        .then_some(SecondUnmakeDestroySetup {
            second_relic,
            session,
        })
}

fn seed_for_second_unmake_destroy(start: u32) -> String {
    (start..start + 8192)
        .find_map(|seed| {
            let encoded = destroy_relics_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-relic")
                || !hand.iter().any(|card| card == "north-unmake")
            {
                return None;
            }
            try_second_unmake_destroy_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Unmake destroy setup")
}

fn seed_with_relic_and_veil_at_c4(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = destroy_relics_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-relic")
                || !hand.iter().any(|card| card == "north-veil")
                || !hand.iter().any(|card| card == "north-unmake")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = cast_relic_at(&mut session, "C4");
            try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "cast-aura"
                    && descriptor["cardId"] == "north-veil"
                    && cells_include(descriptor, "C4")
            })?;
            Some(encoded)
        })
        .expect("bounded seed reaching relic and Aura at C4")
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

fn south_casts_relic_at_c1(session: &mut Session) -> String {
    south_plays_c1(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C1"
    });
    let relic_id = artifact_at(session, "south-relic", "C1");
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    relic_id
}

fn south_ends_without_relic(session: &mut Session) {
    south_plays_c1(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn unmake_locations(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("unmake actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-unmake"
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

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
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

fn deathrite_destroy_relics_manifest(seed: u32) -> String {
    let fixture = "destroy-relics-here-deathrite-withheld";
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
            "north-unmake": unmake(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-unmake",
                    "north-rain",
                    "north-relic",
                    "north-unmake",
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

fn north_has_unmake_rain_and_relic(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-unmake", "north-rain", "north-relic"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteDestroyRelicsSetup {
    deathrite_ids: [String; 2],
    relic_id: String,
    session: Session,
}

fn try_pending_deathrite_with_nearby_relic(
    encoded: &str,
) -> Option<PendingDeathriteDestroyRelicsSetup> {
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
    if !north_has_unmake_rain_and_relic(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C4"
    })?;
    let relic_id = artifact_at(&session, "north-relic", "C4");
    if unmake_locations(&session) != ["C4"] {
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
    Some(PendingDeathriteDestroyRelicsSetup {
        deathrite_ids,
        relic_id,
        session,
    })
}

fn deathrite_destroy_relics_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_destroy_relics_manifest)
        .find(|candidate| try_pending_deathrite_with_nearby_relic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with destroy-relics Magic in hand")
}

#[test]
fn rule_catalog_0569_destroy_relics_here_destroys_a_nearby_artifact() {
    let encoded = seed_with(&["north-unmake", "north-relic"]);
    let mut session = opening_main(&encoded);
    let far_id = south_casts_relic_at_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let near_id = artifact_at(&session, "north-relic", "C3");
    let before = state(&session);
    assert_eq!(
        realm_artifact(&before, &near_id).expect("near relic")["location"],
        "C3"
    );
    assert_eq!(
        realm_artifact(&before, &far_id).expect("far relic")["location"],
        "C1"
    );
    assert_eq!(unmake_locations(&session), ["C3", "C4"]);

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unmake"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    let types = event_types(&receipt);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(types.contains(&"artifact-destroyed"));
    assert!(!types.contains(&"artifact-banished"));
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "artifact-destroyed")
        .expect("artifact destruction");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], near_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert!(realm_artifact(&after, &near_id).is_none());
    assert_eq!(
        realm_artifact(&after, &far_id).expect("far relic remains")["location"],
        "C1"
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == near_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0570_destroy_relics_here_empty_location_is_a_paid_noop() {
    let encoded = seed_with(&["north-unmake"]);
    let mut session = opening_main(&encoded);
    south_ends_without_relic(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(unmake_locations(&session), ["C3", "C4"]);
    assert!(
        state(&session)["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unmake"
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    let types = event_types(&receipt);
    assert_eq!(types, ["magic-cast", "magic-resolved"]);
    assert!(
        state(&session)["realm"]
            .get("artifacts")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1088_destroy_relics_here_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_destroy_relics_seed_with(1088);
    let mut setup = try_pending_deathrite_with_nearby_relic(&encoded)
        .expect("complete destroy-relics Deathrite withheld setup");
    let relic_id = setup.relic_id.clone();
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
    assert!(unmake_locations(session).is_empty());

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
    assert_eq!(
        realm_artifact(&resumed, &relic_id).expect("nearby relic remains")["location"],
        "C4"
    );
    assert_eq!(unmake_locations(session), ["C4"]);

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-unmake"
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
fn rule_catalog_1823_destroyed_relic_stays_in_cemetery_after_turns_pass() {
    let encoded = seed_with_start(1823, &["north-unmake", "north-relic"]);
    let mut session = opening_main(&encoded);
    let relic_id = cast_relic_at(&mut session, "C4");
    cast_unmake(&mut session, "C4");
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    advance_full_round(&mut session);
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1824_second_unmake_without_a_relic_is_still_a_paid_noop() {
    let encoded = seed_with_two_unmake_spells_in_hand_after_setup(1824);
    let mut session = opening_main(&encoded);
    let relic_id = cast_relic_at(&mut session, "C4");
    let first = cast_unmake(&mut session, "C4");
    assert!(event_types(&first).contains(&"artifact-destroyed"));
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    let second = cast_unmake(&mut session, "C4");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(!second.events.iter().any(|event| {
        event.event_type == "artifact-destroyed" || event.event_type == "aura-destroyed"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1825_second_unmake_destroys_a_newly_arrived_relic_at_the_same_cell() {
    let encoded = seed_with_two_unmake_spells_in_hand_after_setup(1825);
    let mut session = opening_main(&encoded);
    let relic_id = cast_relic_at(&mut session, "C4");
    cast_unmake(&mut session, "C4");
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    let nearby_id = south_places_relic_at_c4(&mut session);
    let destroyed = cast_unmake(&mut session, "C4");
    assert_eq!(
        event_types(&destroyed),
        ["magic-cast", "artifact-destroyed", "magic-resolved"]
    );
    assert_eq!(destroyed.events[1].payload["instanceId"], nearby_id);
    assert!(cemetery_has(&state(&session), "south", &nearby_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1826_unmake_destroys_every_artifact_and_aura_sharing_the_target_cell() {
    let encoded = seed_with_relic_and_veil_at_c4(1826);
    let mut session = opening_main(&encoded);
    let relic_id = cast_relic_at(&mut session, "C4");
    let veil_id = cast_veil_at(&mut session, "C4");
    let destroyed = cast_unmake(&mut session, "C4");
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
    let aura_destroys: Vec<_> = destroyed
        .events
        .iter()
        .filter(|event| event.event_type == "aura-destroyed")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("destroyed aura")
                .to_owned()
        })
        .collect();
    assert_eq!(relic_destroys, vec![relic_id.clone()]);
    assert_eq!(aura_destroys, vec![veil_id.clone()]);
    assert!(realm_artifact(&state(&session), &relic_id).is_none());
    assert!(realm_aura(&state(&session), &veil_id).is_none());
    assert!(cemetery_has(&state(&session), "north", &relic_id));
    assert!(cemetery_has(&state(&session), "north", &veil_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1827_unmake_leaves_a_far_relic_untouched() {
    let encoded = seed_with_start(1827, &["north-unmake", "north-relic"]);
    let mut session = opening_main(&encoded);
    let far_id = south_casts_relic_at_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let near_id = cast_relic_at(&mut session, "C3");
    cast_unmake(&mut session, "C3");
    assert!(cemetery_has(&state(&session), "north", &near_id));
    assert_eq!(
        realm_artifact(&state(&session), &far_id).expect("far relic")["location"],
        "C1"
    );
    assert!(!cemetery_has(&state(&session), "south", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1828_second_unmake_destroys_a_newly_placed_relic() {
    let encoded = seed_for_second_unmake_destroy(1828);
    let SecondUnmakeDestroySetup {
        mut session,
        second_relic,
    } = try_second_unmake_destroy_prefix(&encoded).expect("second Unmake destroy prefix");
    let destroyed = cast_unmake(&mut session, "C4");
    assert_eq!(
        event_types(&destroyed),
        ["magic-cast", "artifact-destroyed", "magic-resolved"]
    );
    assert_eq!(destroyed.events[1].payload["instanceId"], second_relic);
    assert!(cemetery_has(&state(&session), "north", &second_relic));
    assert_exact_replay(&session);
}
