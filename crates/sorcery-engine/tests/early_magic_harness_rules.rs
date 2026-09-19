//! Direct proofs for targeted Magic as a non-unit source
//! (RULE-CATALOG-0019, RULE-CATALOG-0685–0686, RULE-CATALOG-2393–2398).
//!
//! 0595–0596 prove ordinary Zap lethal/Ward. 0659–0660 prove Deathrite after
//! nearby-control. These proofs keep the 0019 harness slice: Magic still kills
//! a Deathrite minion that prevents high-power unit damage, and a later-turn
//! Zap is the death blow after Death's Door. 2393–2398 add persistence,
//! no-minion-repeat, enemy-arrival, multi-minion, far-minion, and new-summon
//! branches on that same Deathrite harness.

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

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn deathrite() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "preventsDamageFromUnitsWithPowerAtLeast": 4,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "early-harness-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-early-harness-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lash"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn avatar_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "early-harness-avatar" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-early-harness-avatar-v1",
        },
        "cards": {
            "north-avatar": avatar(1),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(1),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lash"; 6],
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
    let mut session = Session::new(encoded).expect("valid early Magic harness session");
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

fn north_has_lash(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-lash"))
}

fn seed_with(start: u32, deathrite: bool) -> String {
    (start..start + 256)
        .map(|seed| {
            if deathrite {
                deathrite_manifest(seed)
            } else {
                avatar_manifest(seed)
            }
        })
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_lash(&state(&preview)))
        })
        .expect("bounded seed with Lash in the opening hand")
}

fn stage_south_turn(session: &mut Session, summon_deathrite: bool) -> Option<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let summoned = summon_deathrite.then(|| {
        let (descriptor, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-deathrite"
                && descriptor["cell"] == "C1"
                && descriptor["region"].is_null()
        });
        descriptor["cardInstanceId"]
            .as_str()
            .expect("Deathrite minion identity")
            .to_owned()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned
}

fn stage_south_deathrite(session: &mut Session) -> String {
    stage_south_turn(session, true).expect("Deathrite minion identity")
}

fn stage_south_site(session: &mut Session) {
    assert!(stage_south_turn(session, false).is_none());
}

fn pass_round_drawing_spells(session: &mut Session) {
    for _ in 0..2 {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn atlas_hand_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["hand"]["atlas"]
        .as_array()
        .expect("atlas hand")
        .len()
}

fn lash_target_keys(session: &Session, spell_id: &str) -> Vec<String> {
    let mut keys: Vec<_> = session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == spell_id
        })
        .map(|action| {
            format!(
                "{}:{}:{}",
                action.descriptor["target"]["kind"]
                    .as_str()
                    .expect("target kind"),
                action.descriptor["target"]["seat"]
                    .as_str()
                    .expect("target seat"),
                action.descriptor["target"]["instanceId"]
                    .as_str()
                    .expect("target identity")
            )
        })
        .collect();
    keys.sort_unstable();
    keys
}

fn assert_lash_targets_are_canonical(session: &Session, spell_id: &str) {
    let descriptors: Vec<_> = session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == spell_id
        })
        .map(|action| action.descriptor)
        .collect();
    let canonical: Vec<_> = descriptors
        .iter()
        .map(|descriptor| canonical_json(descriptor).expect("canonical target descriptor"))
        .collect();
    let mut sorted = canonical.clone();
    sorted.sort_unstable();
    assert_eq!(canonical, sorted);
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
fn rule_catalog_0685_targeted_magic_kills_a_deathrite_minion_as_a_non_unit_source() {
    let encoded = seed_with(685, true);
    let mut session = opening_main(&encoded);
    let target_id = stage_south_deathrite(&mut session);

    let before = state(&session);
    let spell_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook")
        .iter()
        .find(|card| card["cardId"] == "north-lash")
        .expect("Lash in hand")["instanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_lash_targets_are_canonical(&session, &spell_id);
    let mut expected = vec![
        format!(
            "avatar:north:{}",
            before["players"]["north"]["avatar"]["card"]["instanceId"]
                .as_str()
                .expect("North Avatar identity")
        ),
        format!(
            "avatar:south:{}",
            before["players"]["south"]["avatar"]["card"]["instanceId"]
                .as_str()
                .expect("South Avatar identity")
        ),
        format!("minion:south:{target_id}"),
    ];
    expected.sort_unstable();
    assert_eq!(lash_target_keys(&session, &spell_id), expected);

    let south_atlas = atlas_len(&before, "south");
    let south_atlas_hand = atlas_hand_len(&before, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(lash["cardInstanceId"], spell_id);
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["amount"], 1);
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], target_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(
        !finished["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == target_id)
    );
    assert!(cemetery_has(&finished, "south", &target_id));
    assert!(cemetery_has(&finished, "north", &spell_id));
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_eq!(atlas_hand_len(&finished, "south"), south_atlas_hand + 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0686_targeted_magic_reaches_deaths_door_then_defeats_the_avatar() {
    let encoded = seed_with(686, false);
    let mut session = opening_main(&encoded);
    stage_south_site(&mut session);
    let south_avatar = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();

    let (_, first) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    assert!(event_types(&first).contains(&"magic-damage-allocated"));
    assert!(event_types(&first).contains(&"avatar-reached-deaths-door"));
    assert!(!event_types(&first).contains(&"game-ended"));
    let after_first = state(&session);
    assert_eq!(after_first["players"]["south"]["avatar"]["life"], 0);
    assert_eq!(after_first["terminal"]["status"], "active");

    pass_round_drawing_spells(&mut session);
    let (_, death_blow) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    let terminal = state(&session)["terminal"].clone();
    assert_eq!(terminal["status"], "finished");
    assert_eq!(terminal["winner"], "north");
    assert_eq!(terminal["loser"], "south");
    assert_eq!(terminal["reason"], "avatar_defeated");
    assert_eq!(
        event_types(&death_blow)
            .into_iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>(),
        ["game-ended", "magic-resolved"]
    );
    assert!(event_types(&death_blow).contains(&"death-blow"));
    assert_exact_replay(&session);
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

fn deathrite_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "early-harness-deathrite-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-early-harness-deathrite-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lash"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 8],
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

fn supplemental_seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(685..685 + 2048)
        .map(deathrite_supplemental_manifest)
        .find(|candidate| {
            opening_hand_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-lash")
                && opening_hand_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-deathrite")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Lash and required South Deathrite minions")
}

fn lash_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-lash")
                .count()
        })
        .unwrap_or_default()
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    realm_unit(snapshot, instance_id).expect("expected realm unit")
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

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned()
}

fn setup_c1_with_south_minions(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    (0..count).map(|_| summon_south_at(session, "C1")).collect()
}

fn cast_lash_target(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn lash_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-lash"
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

fn lash_minion_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-lash"
                && action.descriptor["target"]["kind"] == "minion"
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

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = opening_main(encoded);
    let c1_ids = setup_c1_with_south_minions(&mut session, 2);
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
    (!lash_targets(&session).is_empty()).then_some((session, c1_ids, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(685..685 + 2048)
        .find_map(|seed| {
            let encoded = deathrite_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-deathrite")
                .count()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching Deathrite harness far-minion setup")
}

fn try_second_lash_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_lash_target(&mut session, &first_id);
    if !event_types(&first).contains(&"minion-died") {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if lash_spells_in_hand(&state(&session)) < 1 {
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
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    lash_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_lash_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(685..685 + 8192)
        .find_map(|seed| {
            let encoded = deathrite_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-deathrite")
                .count()
                < 2
            {
                return None;
            }
            try_second_lash_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Lash enemy-arrival setup")
}

fn try_second_lash_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_lash_target(&mut session, &first_id);
    if !event_types(&first).contains(&"minion-died") {
        return None;
    }
    if realm_unit(&state(&session), &first_id).is_some() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if lash_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C1");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    lash_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_lash_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(685..685 + 8192)
        .find_map(|seed| {
            let encoded = deathrite_supplemental_manifest(seed);
            if opening_hand_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-deathrite")
                .count()
                < 2
            {
                return None;
            }
            try_second_lash_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Lash new-summon setup")
}

#[test]
fn rule_catalog_2393_killed_deathrite_minion_stays_in_the_cemetery_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2393, 1);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c1_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let receipt = cast_lash_target(&mut session, &minion_id);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(receipt.events[1].payload["targetInstanceId"], minion_id);
    assert!(event_types(&receipt).contains(&"site-drawn"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2394_second_lash_offers_no_minion_targets_after_killing_the_only_deathrite() {
    let encoded = supplemental_seed_with_start(2394, 1);
    let mut session = opening_main(&encoded);
    let minion_id = setup_c1_with_south_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_lash_target(&mut session, &minion_id);
    assert!(event_types(&first).contains(&"minion-died"));
    assert!(event_types(&first).contains(&"site-drawn"));
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(lash_spells_in_hand(&state(&session)) >= 1);
    assert!(lash_minion_targets(&session).is_empty());
    assert!(offers(&session, |descriptor| descriptor["kind"]
        == "cast-magic"
        && descriptor["cardId"] == "north-lash"));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2395_second_lash_kills_a_newly_arrived_deathrite_after_enemy_site_placement() {
    let encoded = seed_for_second_lash_enemy_arrival(2395);
    let (mut session, minion_id) = try_second_lash_enemy_arrival_prefix(&encoded)
        .expect("second Deathrite-harness enemy-arrival prefix");
    let receipt = cast_lash_target(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"site-drawn"));
    assert_eq!(receipt.events[1].payload["targetInstanceId"], minion_id);
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2396_lash_offers_every_same_region_deathrite_minion_and_both_avatars() {
    let encoded = supplemental_seed_with_start(2396, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c1_with_south_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = lash_targets(&session);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_eq!(lash_minion_targets(&session).len(), 2);
    let snapshot = state(&session);
    let north_avatar = snapshot["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity");
    let south_avatar = snapshot["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity");
    assert!(offered.iter().any(|target| target == north_avatar));
    assert!(offered.iter().any(|target| target == south_avatar));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2397_lash_kills_a_deathrite_minion_and_leaves_a_far_minion_untouched() {
    let encoded = seed_for_far_minion(2397);
    let (mut session, c1_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("Deathrite harness far-minion prefix");
    let receipt = cast_lash_target(&mut session, &c1_ids[0]);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"site-drawn"));
    assert!(realm_unit(&state(&session), &c1_ids[0]).is_none());
    assert_eq!(unit(&state(&session), &far_id)["damage"], 0);
    assert_eq!(unit(&state(&session), &far_id)["location"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2398_second_lash_kills_a_newly_summoned_deathrite_minion() {
    let encoded = seed_for_second_lash_new_summon(2398);
    let (mut session, minion_id) = try_second_lash_new_summon_prefix(&encoded)
        .expect("second Deathrite-harness new-summon prefix");
    let receipt = cast_lash_target(&mut session, &minion_id);
    assert!(event_types(&receipt).contains(&"minion-died"));
    assert!(event_types(&receipt).contains(&"site-drawn"));
    assert_eq!(receipt.events[1].payload["targetInstanceId"], minion_id);
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}
