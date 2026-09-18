//! Direct proofs for ally-submerges-target-nearby-minion Magic
//! (RULE-CATALOG-0555–0556, RULE-CATALOG-1056, RULE-CATALOG-1753–1758).
//!
//! Ordinary Magic chooses a controlled ally, then submerges one other
//! minion nearby that ally. Nearby is measured from the ally, not the
//! caster. A far minion is not offered. An Earth site is a paid no-op.

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

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["water"],
    })
}

fn grounded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn trial() -> Value {
    json!({
        "allySubmergesTargetNearbyMinion": true,
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

fn trial_manifest(seed: u32, water: bool) -> String {
    let site = if water { water_site() } else { earth_site() };
    let fixture = if water {
        "ally-submerge-nearby-water"
    } else {
        "ally-submerge-nearby-earth"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-site": site,
            "north-trial": trial(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-site": site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-trial",
                    "north-trial",
                    "north-trial",
                    "north-trial",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-raider"; 6],
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
    let mut session = Session::new(encoded).expect("valid ally-submerge session");
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

fn trial_pairs(session: &Session) -> Vec<(String, String)> {
    session
        .legal_actions()
        .expect("trial actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-trial"
        })
        .filter_map(|action| {
            Some((
                action.descriptor["ally"]["instanceId"].as_str()?.to_owned(),
                action.descriptor["target"]["instanceId"]
                    .as_str()?
                    .to_owned(),
            ))
        })
        .collect()
}

fn seed_with(water: bool, start: u32, required: &[&str]) -> String {
    (start..start + 256)
        .map(|seed| trial_manifest(seed, water))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn advance_full_round(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn south_raids_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (nearby, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
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

fn summon_north_ally(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned()
}

fn opening_with_ally(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let ally_id = summon_north_ally(&mut session);
    (session, ally_id)
}

fn host_setup(water: bool, start: u32) -> (Session, String) {
    let encoded = seed_with(water, start, &["north-ally", "north-trial"]);
    opening_with_ally(&encoded)
}

fn host_setup_with_two_trials(water: bool, start: u32) -> (Session, String) {
    let encoded = seed_with(water, start, &["north-ally", "north-trial", "north-trial"]);
    opening_with_ally(&encoded)
}

fn cast_trial_submerge(session: &mut Session, ally_id: &str, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-trial"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn avatar_id(snapshot: &Value) -> String {
    snapshot["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar identity")
        .to_owned()
}

fn cast_trial_submerge_via_avatar(session: &mut Session, target_id: &str) -> Receipt {
    let avatar = avatar_id(&state(session));
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-trial"
            && descriptor["ally"]["kind"] == "avatar"
            && descriptor["ally"]["instanceId"] == avatar
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn south_plays_c1_and_raids_c4(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let (nearby, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (
        far["cardInstanceId"]
            .as_str()
            .expect("far enemy identity")
            .to_owned(),
        nearby["cardInstanceId"]
            .as_str()
            .expect("nearby enemy identity")
            .to_owned(),
    )
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

fn assert_offered_pairs(session: &Session, ally_id: &str, nearby_id: &str, far_id: &str) {
    let avatar_id = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar identity")
        .to_owned();
    let pairs = trial_pairs(session);
    assert!(pairs.contains(&(avatar_id.clone(), ally_id.to_owned())));
    assert!(pairs.contains(&(avatar_id.clone(), nearby_id.to_owned())));
    assert!(pairs.contains(&(ally_id.to_owned(), nearby_id.to_owned())));
    assert!(!pairs.iter().any(|(_, target)| target == far_id));
    assert!(!pairs.contains(&(ally_id.to_owned(), ally_id.to_owned())));
    assert!(!pairs.contains(&(avatar_id, far_id.to_owned())));
}

fn deathrite_submerge_manifest(seed: u32) -> String {
    let fixture = "ally-submerge-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-rain": rain_spell(),
            "north-site": water_site(),
            "north-trial": trial(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-raider": raider(),
            "south-site": water_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-trial",
                    "north-rain",
                    "north-rain",
                    "north-trial",
                    "north-rain",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-raider", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_trial_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-trial", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteSubmergeSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    nearby_id: String,
    session: Session,
}

fn try_pending_deathrite_with_nearby_target(
    encoded: &str,
) -> Option<PendingDeathriteSubmergeSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let nearby = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let nearby_id = nearby.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_trial_and_rain(&state(&session)) {
        return None;
    }
    if !trial_pairs(&session)
        .iter()
        .any(|(ally, target)| ally == &ally_id && target == &nearby_id)
    {
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
    Some(PendingDeathriteSubmergeSetup {
        ally_id,
        deathrite_ids,
        nearby_id,
        session,
    })
}

fn deathrite_submerge_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_submerge_manifest)
        .find(|candidate| try_pending_deathrite_with_nearby_target(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with ally-submerge Magic in hand")
}

#[test]
fn rule_catalog_0555_ally_submerges_a_nearby_minion_on_water() {
    let encoded = seed_with(true, 555, &["north-ally", "north-trial"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    let (far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "surface");
    assert_offered_pairs(&session, &ally_id, &nearby_id, &far_id);

    let (cast, submerged) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-trial"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&submerged),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(submerged.events[1].payload["cell"], "C4");
    assert_eq!(submerged.events[1].payload["instanceId"], nearby_id);
    assert_eq!(submerged.events[1].payload["seat"], "south");
    assert_eq!(
        submerged.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "underwater");
    assert_eq!(unit(&state(&session), &ally_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0556_ally_submerge_is_a_paid_noop_on_earth() {
    let encoded = seed_with(false, 556, &["north-ally", "north-trial"]);
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    let (far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    assert_offered_pairs(&session, &ally_id, &nearby_id, &far_id);

    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-trial"
            && descriptor["ally"]["kind"] == "minion"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &ally_id)["region"], "surface");
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1056_ally_submerge_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_submerge_seed_with(1056);
    let mut setup = try_pending_deathrite_with_nearby_target(&encoded)
        .expect("complete ally-submerge Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
    let nearby_id = setup.nearby_id.clone();
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
    assert_eq!(unit(&paused, &nearby_id)["region"], "surface");
    assert_eq!(unit(&paused, &ally_id)["region"], "surface");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(trial_pairs(session).is_empty());

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
    assert_eq!(unit(&resumed, &nearby_id)["region"], "surface");
    assert!(
        trial_pairs(session)
            .iter()
            .any(|(ally, target)| ally == &ally_id && target == &nearby_id)
    );

    let (_, submerged) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-trial"
            && descriptor["ally"]["instanceId"] == ally_id
            && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&submerged),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(unit(&state(session), &nearby_id)["region"], "underwater");
    assert_eq!(unit(&state(session), &ally_id)["region"], "surface");
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1753_submerged_minion_stays_underwater_after_turns_pass() {
    let (mut session, ally_id) = host_setup(true, 1753);
    let (_far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    cast_trial_submerge(&mut session, &ally_id, &nearby_id);
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "underwater");
    advance_full_round(&mut session);
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "underwater");
    assert_eq!(unit(&state(&session), &ally_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1754_second_trial_omits_an_already_submerged_minion() {
    let (mut session, ally_id) = host_setup_with_two_trials(true, 1754);
    let (_far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    let first = cast_trial_submerge(&mut session, &ally_id, &nearby_id);
    assert_eq!(
        event_types(&first),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert!(
        !trial_pairs(&session)
            .iter()
            .any(|(_, target)| target == &nearby_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1755_second_earth_trial_on_the_same_pair_is_still_a_paid_noop() {
    let (mut session, ally_id) = host_setup_with_two_trials(false, 1755);
    let (_far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    let first = cast_trial_submerge(&mut session, &ally_id, &nearby_id);
    assert_eq!(event_types(&first), ["magic-cast", "magic-resolved"]);
    let second = cast_trial_submerge(&mut session, &ally_id, &nearby_id);
    assert_eq!(event_types(&second), ["magic-cast", "magic-resolved"]);
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1756_trial_submerges_via_avatar_anchor_while_ally_stays_above() {
    let encoded = seed_with(true, 1756, &["north-ally", "north-trial"]);
    let mut session = opening_main(&encoded);
    let ally_id = summon_north_ally(&mut session);
    let (_far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    let submerged = cast_trial_submerge_via_avatar(&mut session, &nearby_id);
    assert_eq!(
        event_types(&submerged),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "underwater");
    assert_eq!(unit(&state(&session), &ally_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1757_trial_leaves_a_far_minion_on_the_surface() {
    let (mut session, ally_id) = host_setup(true, 1757);
    let (far_id, nearby_id) = south_plays_c1_and_raids_c4(&mut session);
    cast_trial_submerge(&mut session, &ally_id, &nearby_id);
    assert_eq!(unit(&state(&session), &nearby_id)["region"], "underwater");
    assert_eq!(unit(&state(&session), &far_id)["region"], "surface");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1758_second_trial_submerges_a_newly_arrived_nearby_minion() {
    let (mut session, ally_id) = host_setup_with_two_trials(true, 1758);
    let (_far_id, first_nearby) = south_plays_c1_and_raids_c4(&mut session);
    cast_trial_submerge(&mut session, &ally_id, &first_nearby);
    assert_eq!(
        unit(&state(&session), &first_nearby)["region"],
        "underwater"
    );
    let second_nearby = south_raids_c4(&mut session);
    assert_eq!(unit(&state(&session), &second_nearby)["region"], "surface");
    let submerged = cast_trial_submerge(&mut session, &ally_id, &second_nearby);
    assert_eq!(
        event_types(&submerged),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert_eq!(submerged.events[1].payload["instanceId"], second_nearby);
    assert_eq!(
        unit(&state(&session), &second_nearby)["region"],
        "underwater"
    );
    assert_eq!(
        unit(&state(&session), &first_nearby)["region"],
        "underwater"
    );
    assert_exact_replay(&session);
}
