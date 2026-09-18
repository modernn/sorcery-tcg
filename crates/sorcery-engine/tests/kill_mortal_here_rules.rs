//! Direct proofs for kill-mortal-minions-at-location-within-two-steps Magic
//! (RULE-CATALOG-0567–0568, RULE-CATALOG-1017, RULE-CATALOG-1096,
//! RULE-CATALOG-1813–1818).
//!
//! 1017 covers kill mortal here killing a Deathrite minion: the controller
//! draws a site and magic-resolved only appears after deathrite settlement.
//! 1096 covers kill mortal here withheld while Deathrites wait for ordering,
//! until the chain drains.
//!
//! Ordinary Magic offers existing locations within two measured cardinal
//! steps of the caster footprint and kills every Mortal minion there.
//! Avatars and non-Mortal minions are left unwounded. An empty qualifying
//! set is a paid no-op.

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

fn deathrite_mortal() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 3,
        "manaCost": 0,
        "mortal": true,
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

fn mortality() -> Value {
    json!({
        "cardType": "magic",
        "killMortalMinionsAtLocationWithinTwoSteps": true,
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

fn mortality_deathrite_manifest(seed: u32) -> String {
    mortality_manifest_with_mortal(seed, &deathrite_mortal())
}

fn mortality_manifest(seed: u32) -> String {
    mortality_manifest_with_mortal(seed, &mortal())
}

fn mortality_manifest_with_mortal(seed: u32, north_mortal: &Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "kill-mortal-here" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-kill-mortal-here-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-beast": beast(),
            "north-mortal": north_mortal.clone(),
            "north-mortality": mortality(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-mortal": {
                "attack": 1,
                "cardType": "minion",
                "defense": 3,
                "manaCost": 0,
                "mortal": true,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-mortal",
                    "north-beast",
                    "north-mortality",
                    "north-mortality",
                    "north-mortal",
                    "north-beast",
                    "north-mortality",
                    "north-mortality",
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
    let mut session = Session::new(encoded).expect("valid kill-mortal-here session");
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

fn mortality_locations(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("mortality actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-mortality"
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
    seed_with_start(567, required)
}

fn seed_with_deathrite(required: &[&str]) -> String {
    seed_with_manifest(1017, required, mortality_deathrite_manifest)
}

fn seed_with_start(start: u32, required: &[&str]) -> String {
    seed_with_manifest(start, required, mortality_manifest)
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

fn mortality_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-mortality")
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

fn cast_mortality(session: &mut Session, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mortality"
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
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
            && descriptor["cardId"] == "south-mortal"
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

fn seed_with_two_mortality_spells_in_hand_after_setup(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = mortality_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if hand.iter().filter(|card| *card == "north-mortal").count() < 1
                || !hand.iter().any(|card| card == "north-mortality")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = summon_at(&mut session, "north-mortal", "C4");
            (mortality_spells_in_hand(&state(&session)) >= 2).then_some(encoded)
        })
        .expect("bounded seed with two Mortality spells in hand after setup")
}

fn allies_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-mortal")
                .count()
        })
        .unwrap_or_default()
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

struct SecondMortalityKillSetup {
    second_mortal: String,
    session: Session,
}

fn try_second_mortality_kill_prefix(encoded: &str) -> Option<SecondMortalityKillSetup> {
    let mut session = opening_main(encoded);
    let first_mortal = summon_at(&mut session, "north-mortal", "C4");
    cast_mortality(&mut session, "C4");
    if !cemetery_has(&state(&session), "north", &first_mortal) {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    let snap = state(&session);
    if mortality_spells_in_hand(&snap) < 1 || allies_in_hand(&snap) < 1 {
        return None;
    }
    let second_mortal = summon_at(&mut session, "north-mortal", "C4");
    mortality_locations(&session)
        .contains(&"C4".to_owned())
        .then_some(SecondMortalityKillSetup {
            second_mortal,
            session,
        })
}

fn seed_for_second_mortality_kill(start: u32) -> String {
    (start..start + 8192)
        .find_map(|seed| {
            let encoded = mortality_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if !hand.iter().any(|card| card == "north-mortal")
                || !hand.iter().any(|card| card == "north-mortality")
            {
                return None;
            }
            try_second_mortality_kill_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Mortality kill setup")
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

fn deathrite_mortality_manifest(seed: u32) -> String {
    let fixture = "kill-mortal-here-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-mortal": mortal(),
            "north-mortality": mortality(),
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
                    "north-mortal",
                    "north-mortality",
                    "north-rain",
                    "north-rain",
                    "north-mortality",
                    "north-mortality",
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

fn north_has_mortality_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-mortality", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteMortalitySetup {
    deathrite_ids: [String; 2],
    occupant_id: String,
    session: Session,
}

fn try_pending_deathrite_with_ready_occupant(
    encoded: &str,
) -> Option<PendingDeathriteMortalitySetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
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
    if !north_has_mortality_and_rain(&state(&session)) {
        return None;
    }
    if mortality_locations(&session) != ["C4"] {
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
    Some(PendingDeathriteMortalitySetup {
        deathrite_ids,
        occupant_id,
        session,
    })
}

fn deathrite_mortality_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_mortality_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_occupant(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with kill-mortal Magic in hand")
}

#[test]
fn rule_catalog_0567_kills_mortal_at_location_and_spares_non_mortal_and_far_mortal() {
    let encoded = seed_with(&["north-mortal", "north-beast", "north-mortality"]);
    let mut session = opening_main(&encoded);
    let mortal_id = summon_at(&mut session, "north-mortal", "C4");
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    let far_id = south_plays_c1_and_summons(&mut session);

    let before = state(&session);
    assert_eq!(unit(&before, &mortal_id)["location"], "C4");
    assert_eq!(unit(&before, &beast_id)["location"], "C4");
    assert_eq!(unit(&before, &beast_id)["damage"], 0);
    assert_eq!(unit(&before, &far_id)["location"], "C1");
    assert_eq!(mortality_locations(&session), ["C4"]);
    assert!(!mortality_locations(&session).contains(&"C1".to_owned()));

    let (cast, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mortality"
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
    let kill = killed
        .events
        .iter()
        .find(|event| event.event_type == "minion-killed")
        .expect("minion-killed");
    assert_eq!(kill.payload["cardId"], "north-mortal");
    assert_eq!(kill.payload["instanceId"], mortal_id);
    assert_eq!(kill.payload["owner"], "north");
    assert_eq!(kill.payload["seat"], "north");
    assert_eq!(kill.payload["sourceInstanceId"], cast["cardInstanceId"]);
    assert!(
        !killed
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
            .all(|unit| unit["instanceId"] != mortal_id)
    );
    assert_eq!(unit(&after, &beast_id)["location"], "C4");
    assert_eq!(unit(&after, &beast_id)["damage"], 0);
    assert_eq!(unit(&after, &far_id)["location"], "C1");
    assert_eq!(unit(&after, &far_id)["damage"], 0);
    assert!(cemetery_has(&after, "north", &mortal_id));
    assert!(!cemetery_has(&after, "south", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0568_location_with_only_a_non_mortal_is_a_paid_noop() {
    let encoded = seed_with(&["north-beast", "north-mortality"]);
    let mut session = opening_main(&encoded);
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    assert_eq!(mortality_locations(&session), ["C4"]);
    assert_eq!(unit(&state(&session), &beast_id)["damage"], 0);

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mortality"
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert!(
        !resolved
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed" || event.event_type == "minion-died")
    );

    let after = state(&session);
    assert_eq!(unit(&after, &beast_id)["location"], "C4");
    assert_eq!(unit(&after, &beast_id)["damage"], 0);
    assert!(!cemetery_has(&after, "north", &beast_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1813_killed_mortal_stays_in_cemetery_after_turns_pass() {
    let encoded = seed_with_start(1813, &["north-mortal", "north-mortality"]);
    let mut session = opening_main(&encoded);
    let mortal_id = summon_at(&mut session, "north-mortal", "C4");
    cast_mortality(&mut session, "C4");
    assert!(cemetery_has(&state(&session), "north", &mortal_id));
    advance_full_round(&mut session);
    assert!(cemetery_has(&state(&session), "north", &mortal_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1814_second_mortality_without_a_mortal_is_still_a_paid_noop() {
    let encoded = seed_with_two_mortality_spells_in_hand_after_setup(1814);
    let mut session = opening_main(&encoded);
    let mortal_id = summon_at(&mut session, "north-mortal", "C4");
    let first = cast_mortality(&mut session, "C4");
    assert!(event_types(&first).contains(&"minion-killed"));
    assert!(cemetery_has(&state(&session), "north", &mortal_id));
    let second = cast_mortality(&mut session, "C4");
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert!(
        !second
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed" || event.event_type == "minion-died")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1815_second_mortality_kills_a_newly_arrived_mortal_at_the_same_cell() {
    let encoded = seed_with_two_mortality_spells_in_hand_after_setup(1815);
    let mut session = opening_main(&encoded);
    let mortal_id = summon_at(&mut session, "north-mortal", "C4");
    cast_mortality(&mut session, "C4");
    assert!(cemetery_has(&state(&session), "north", &mortal_id));
    let nearby_id = south_raids_c4(&mut session);
    let killed = cast_mortality(&mut session, "C4");
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
    assert!(cemetery_has(&state(&session), "south", &nearby_id));
    assert_exact_replay(&session);
}

fn seed_with_two_mortals_at_c4(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = mortality_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if hand.iter().filter(|card| *card == "north-mortal").count() < 2
                || !hand.iter().any(|card| card == "north-mortality")
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let _ = summon_at(&mut session, "north-mortal", "C4");
            try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "north-mortal"
                    && descriptor["cell"] == "C4"
                    && descriptor["region"].is_null()
            })?;
            Some(encoded)
        })
        .expect("bounded seed reaching two Mortals at C4")
}

#[test]
fn rule_catalog_1816_mortality_kills_every_mortal_sharing_the_target_cell() {
    let encoded = seed_with_two_mortals_at_c4(1816);
    let mut session = opening_main(&encoded);
    let first_mortal = summon_at(&mut session, "north-mortal", "C4");
    let second_mortal = summon_at(&mut session, "north-mortal", "C4");
    let killed = cast_mortality(&mut session, "C4");
    let kills: Vec<_> = killed
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
    assert_eq!(kills.len(), 2);
    assert!(kills.contains(&first_mortal));
    assert!(kills.contains(&second_mortal));
    assert!(cemetery_has(&state(&session), "north", &first_mortal));
    assert!(cemetery_has(&state(&session), "north", &second_mortal));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1817_mortality_leaves_a_far_mortal_untouched() {
    let encoded = seed_with_start(1817, &["north-mortal", "north-beast", "north-mortality"]);
    let mut session = opening_main(&encoded);
    let mortal_id = summon_at(&mut session, "north-mortal", "C4");
    let beast_id = summon_at(&mut session, "north-beast", "C4");
    let far_id = south_plays_c1_and_summons(&mut session);
    cast_mortality(&mut session, "C4");
    assert!(cemetery_has(&state(&session), "north", &mortal_id));
    assert_eq!(unit(&state(&session), &beast_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert!(!cemetery_has(&state(&session), "south", &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1818_second_mortality_kills_a_newly_summoned_mortal() {
    let encoded = seed_for_second_mortality_kill(1818);
    let SecondMortalityKillSetup {
        mut session,
        second_mortal,
    } = try_second_mortality_kill_prefix(&encoded).expect("second Mortality kill prefix");
    let killed = cast_mortality(&mut session, "C4");
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["instanceId"], second_mortal);
    assert!(cemetery_has(&state(&session), "north", &second_mortal));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1017_kill_mortal_here_deathrite_draws_for_controller_on_kill() {
    let encoded = (1017..1017 + 256)
        .map(mortality_deathrite_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|card| card == "north-mortal")
                && hand.iter().any(|card| card == "north-mortality")
        })
        .unwrap_or_else(|| seed_with_deathrite(&["north-mortal", "north-mortality"]));
    let mut session = opening_main(&encoded);
    let mortal_id = summon_at(&mut session, "north-mortal", "C4");
    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (_, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mortality"
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
    assert_eq!(kill.payload["instanceId"], mortal_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");
    assert_eq!(drawn.payload["sourceInstanceId"], mortal_id);
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
    assert!(cemetery_has(&after, "north", &mortal_id));
    assert_eq!(atlas_len(&after, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&after, "south"), south_atlas);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1096_kill_mortal_here_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_mortality_seed_with(1096);
    let mut setup = try_pending_deathrite_with_ready_occupant(&encoded)
        .expect("complete kill-mortal-here Deathrite withheld setup");
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
    assert!(mortality_locations(session).is_empty());

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
    assert_eq!(mortality_locations(session), ["C4"]);

    let (cast, killed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mortality"
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
