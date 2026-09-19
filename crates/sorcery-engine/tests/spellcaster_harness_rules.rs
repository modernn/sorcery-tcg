//! Direct proofs for printed Spellcaster while sick or tapped
//! (RULE-CATALOG-0018, RULE-CATALOG-0689–0690, RULE-CATALOG-2413–2418).
//!
//! A printed Spellcaster may cast Magic and summon on the turn it enters, and
//! may still cast after tapping. Magic originates at the chosen caster, so a
//! tapped minion at C3 can Freeze nearby C2 while the Avatar at C4 cannot.
//! Disabled (Waterbound-on-land) casters are excluded. Distinct from 0151
//! (Tower-granted Spellcaster) and from 0661 (Freeze itself). Supplemental
//! 2413–2418 keep persistence, empty-repeat, enemy-arrival, multi-minion,
//! far-minion, and new-summon proofs on those sick and tapped boards.

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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn freeze() -> Value {
    json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn sick_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "spellcaster-harness-sick" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-spellcaster-harness-sick-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-caster": minion(json!({
                "spellcaster": true,
                "stealth": true,
            })),
            "north-disabled-caster": minion(json!({
                "spellcaster": true,
                "waterbound": true,
            })),
            "north-freeze": freeze(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-caster",
                    "north-disabled-caster",
                    "north-freeze",
                    "north-caster",
                    "north-disabled-caster",
                    "north-freeze",
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

fn tapped_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "spellcaster-harness-tapped" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-spellcaster-harness-tapped-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-caster": minion(json!({
                "spellcaster": true,
                "tapForMana": 1,
            })),
            "north-freeze": freeze(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-caster",
                    "north-freeze",
                    "north-caster",
                    "north-freeze",
                    "north-caster",
                    "north-freeze",
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
    let mut session = Session::new(encoded).expect("valid printed Spellcaster session");
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

fn north_hand_has(snapshot: &Value, card_ids: &[&str]) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    card_ids
        .iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
}

fn seed_sick() -> String {
    (689..689 + 256)
        .map(sick_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                north_hand_has(
                    &state(&preview),
                    &["north-caster", "north-disabled-caster", "north-freeze"],
                )
            })
        })
        .expect("bounded seed with caster, Waterbound caster, and Freeze in hand")
}

fn seed_tapped() -> String {
    (690..690 + 256)
        .map(tapped_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                north_hand_has(&state(&preview), &["north-caster", "north-freeze"])
            })
        })
        .expect("bounded seed with caster and Freeze in hand")
}

fn summon_printed_caster(session: &mut Session) -> String {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("printed Spellcaster identity")
        .to_owned()
}

fn end_then_draw_spell(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected unit")
}

fn stage_tapped_caster_near_c2(session: &mut Session, caster_id: &str) -> String {
    end_then_draw_spell(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_then_draw_spell(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == caster_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    end_then_draw_spell(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C2"
    });
    end_then_draw_spell(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == caster_id
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("nearby target identity")
        .to_owned()
}

fn freeze_count(snapshot: &Value) -> usize {
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

fn freeze_targets_from_caster(session: &Session, caster_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-freeze"
                && action.descriptor["casterInstanceId"] == caster_id
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect()
}

fn caster_can_freeze(session: &Session, caster_id: &str) -> bool {
    !freeze_targets_from_caster(session, caster_id).is_empty()
}

fn freeze_from_caster(session: &mut Session, caster_id: &str, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
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
    let Ok(StepResult::Accepted(receipt)) = session.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    }) else {
        return None;
    };
    Some((descriptor, receipt))
}

fn supplemental_seed_sick(start: u32, required: &[&str], min_freeze: usize) -> String {
    (start..start + 2048)
        .chain(689..689 + 2048)
        .map(sick_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                let snapshot = state(&preview);
                north_hand_has(&snapshot, required) && freeze_count(&snapshot) >= min_freeze
            })
        })
        .expect("bounded seed with required sick-harness opening cards")
}

fn supplemental_seed_tapped(start: u32) -> String {
    (start..start + 2048)
        .chain(689..689 + 2048)
        .map(tapped_manifest)
        .find(|candidate| {
            Session::new(candidate).ok().is_some_and(|preview| {
                north_hand_has(&state(&preview), &["north-caster", "north-freeze"])
            })
        })
        .expect("bounded seed with tapped caster and Freeze in hand")
}

fn stage_tapped_caster_c2_and_c1(session: &mut Session, caster_id: &str) -> (String, String) {
    end_then_draw_spell(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    let far_id = far["cardInstanceId"]
        .as_str()
        .expect("far minion identity")
        .to_owned();
    end_then_draw_spell(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == caster_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    end_then_draw_spell(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (near, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C2"
    });
    end_then_draw_spell(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == caster_id
    });
    (
        near["cardInstanceId"]
            .as_str()
            .expect("nearby minion identity")
            .to_owned(),
        far_id,
    )
}

fn try_retap_caster(session: &mut Session, caster_id: &str) {
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == caster_id
    });
}

fn try_second_freeze_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let caster_id = {
        let (summoned, _) = try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["region"].is_null()
                && descriptor["cardId"] == "north-caster"
                && descriptor["cell"] == "C4"
        })?;
        summoned["cardInstanceId"].as_str()?.to_owned()
    };
    let nearby_id = stage_tapped_caster_near_c2(&mut session, &caster_id);
    freeze_from_caster(&mut session, &caster_id, &nearby_id);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D2"
    })?;
    let (visitor, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "D2"
    })?;
    let visitor_id = visitor["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_retap_caster(&mut session, &caster_id);
    (freeze_count(&state(&session)) >= 1
        && freeze_targets_from_caster(&session, &caster_id).contains(&visitor_id))
    .then_some((session, caster_id, visitor_id))
}

fn seed_for_second_freeze_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(689..689 + 8192)
        .find_map(|seed| {
            let encoded = tapped_manifest(seed);
            try_second_freeze_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching tapped Spellcaster enemy-arrival setup")
}

fn try_second_freeze_new_summon_prefix(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let caster_id = {
        let (summoned, _) = try_accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["region"].is_null()
                && descriptor["cardId"] == "north-caster"
                && descriptor["cell"] == "C4"
        })?;
        summoned["cardInstanceId"].as_str()?.to_owned()
    };
    let nearby_id = stage_tapped_caster_near_c2(&mut session, &caster_id);
    freeze_from_caster(&mut session, &caster_id, &nearby_id);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C2"
    })?;
    let new_id = summoned["cardInstanceId"].as_str()?.to_owned();
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_retap_caster(&mut session, &caster_id);
    (freeze_count(&state(&session)) >= 1
        && freeze_targets_from_caster(&session, &caster_id).contains(&new_id))
    .then_some((session, caster_id, new_id))
}

fn seed_for_second_freeze_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(689..689 + 8192)
        .find_map(|seed| {
            let encoded = tapped_manifest(seed);
            try_second_freeze_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching tapped Spellcaster new-summon setup")
}

#[test]
fn rule_catalog_0689_printed_spellcaster_casts_and_summons_while_summoning_sick() {
    let mut session = opening_main(&seed_sick());
    let caster_id = summon_printed_caster(&mut session);
    assert_eq!(
        unit(&state(&session), &caster_id)["summoningSickness"],
        true
    );
    let checkpoint = session.clone();

    let sick_magic = checkpoint
        .legal_actions()
        .expect("sick caster actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-freeze"
                && action.descriptor["casterInstanceId"] == caster_id
                && action.descriptor["target"]["instanceId"] == caster_id
        })
        .expect("summoning-sick printed Spellcaster Magic action");
    assert_eq!(
        sick_magic.label,
        format!(
            "Cast north-freeze on minion {}… with minion {}…",
            &caster_id[..15],
            &caster_id[..15]
        )
    );
    let mut sick_cast = checkpoint.clone();
    let StepResult::Accepted(sick_receipt) = sick_cast
        .step(ActionRequest {
            action_id: sick_magic.action_id.to_string(),
            seat: sick_magic.seat,
            state_version: sick_magic.state_version,
        })
        .expect("sick caster Magic step")
    else {
        panic!("engine-issued sick caster action must be accepted");
    };
    assert_eq!(
        event_types(&sick_receipt),
        [
            "magic-cast",
            "stealth-lost",
            "minion-disabled",
            "magic-resolved"
        ]
    );
    assert_exact_replay(&sick_cast);

    let mut sick_summon = checkpoint;
    let (summoned, summon_receipt) = accept_where(&mut sick_summon, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-disabled-caster"
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["cell"] == "C4"
    });
    assert_eq!(
        event_types(&summon_receipt),
        ["stealth-lost", "minion-summoned"]
    );
    let disabled_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Disabled caster identity");
    assert!(
        !sick_summon
            .legal_actions()
            .expect("actions after Disabled caster summon")
            .iter()
            .any(|action| {
                action.descriptor["casterInstanceId"] == disabled_id
                    && matches!(
                        action.descriptor["kind"].as_str(),
                        Some("cast-magic" | "summon-minion")
                    )
            })
    );
    assert_exact_replay(&sick_summon);
}

#[test]
fn rule_catalog_0690_printed_spellcaster_casts_from_its_site_while_tapped() {
    let mut session = opening_main(&seed_tapped());
    let caster_id = summon_printed_caster(&mut session);
    let target_id = stage_tapped_caster_near_c2(&mut session, &caster_id);
    let snapshot = state(&session);
    assert_eq!(unit(&snapshot, &caster_id)["tapped"], true);
    assert_eq!(unit(&snapshot, &caster_id)["summoningSickness"], false);
    let avatar_id = snapshot["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let actions = session.legal_actions().expect("tapped caster actions");
    assert!(actions.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic"
            && action.descriptor["casterInstanceId"] == caster_id
            && action.descriptor["target"]["instanceId"] == target_id
    }));
    assert!(!actions.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic"
            && action.descriptor["casterInstanceId"] == avatar_id
            && action.descriptor["target"]["instanceId"] == target_id
    }));
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert_eq!(
        unit(&state(&session), &target_id)["disableEffects"][0]["sourceInstanceId"],
        receipt.events[0].payload["instanceId"]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2413_sick_spellcaster_freeze_stays_disabled_after_turns_pass() {
    let encoded = supplemental_seed_sick(2413, &["north-caster", "north-freeze"], 1);
    let mut session = opening_main(&encoded);
    let caster_id = summon_printed_caster(&mut session);
    assert_eq!(
        unit(&state(&session), &caster_id)["summoningSickness"],
        true
    );
    freeze_from_caster(&mut session, &caster_id, &caster_id);
    assert!(is_disabled(&state(&session), &caster_id));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    if try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })
    .is_none()
    {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    assert!(is_disabled(&state(&session), &caster_id));
    assert!(!caster_can_freeze(&session, &caster_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2414_disabled_sick_spellcaster_cannot_recast_with_a_remaining_freeze() {
    let encoded = supplemental_seed_sick(2414, &["north-caster"], 2);
    let mut session = opening_main(&encoded);
    let caster_id = summon_printed_caster(&mut session);
    assert_eq!(
        unit(&state(&session), &caster_id)["summoningSickness"],
        true
    );
    freeze_from_caster(&mut session, &caster_id, &caster_id);
    assert!(is_disabled(&state(&session), &caster_id));
    assert!(freeze_count(&state(&session)) >= 1);
    assert!(!caster_can_freeze(&session, &caster_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2415_tapped_spellcaster_freezes_a_newly_arrived_nearby_minion_after_enemy_site_placement()
 {
    let encoded = seed_for_second_freeze_enemy_arrival(2415);
    let (mut session, caster_id, visitor_id) = try_second_freeze_enemy_arrival_prefix(&encoded)
        .expect("tapped Spellcaster enemy-arrival prefix");
    let receipt = freeze_from_caster(&mut session, &caster_id, &visitor_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert!(is_disabled(&state(&session), &visitor_id));
    assert_eq!(unit(&state(&session), &visitor_id)["location"], "D2");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2416_sick_spellcaster_freeze_offers_every_nearby_minion() {
    let encoded = supplemental_seed_sick(
        2416,
        &["north-caster", "north-disabled-caster", "north-freeze"],
        1,
    );
    let mut session = opening_main(&encoded);
    let caster_id = summon_printed_caster(&mut session);
    assert_eq!(
        unit(&state(&session), &caster_id)["summoningSickness"],
        true
    );
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-disabled-caster"
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["cell"] == "C4"
    });
    let disabled_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Disabled caster identity")
        .to_owned();
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity")
        .to_owned();
    let targets = freeze_targets_from_caster(&session, &caster_id);
    assert!(targets.contains(&caster_id));
    assert!(targets.contains(&disabled_id));
    assert!(!targets.contains(&avatar_id));
    assert_eq!(targets.len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2417_tapped_spellcaster_leaves_a_far_minion_untouched() {
    let encoded = supplemental_seed_tapped(2417);
    let mut session = opening_main(&encoded);
    let caster_id = summon_printed_caster(&mut session);
    let (nearby_id, far_id) = stage_tapped_caster_c2_and_c1(&mut session, &caster_id);
    let snapshot = state(&session);
    assert_eq!(unit(&snapshot, &caster_id)["tapped"], true);
    let targets = freeze_targets_from_caster(&session, &caster_id);
    assert!(targets.contains(&nearby_id));
    assert!(!targets.contains(&far_id));
    freeze_from_caster(&mut session, &caster_id, &nearby_id);
    assert!(is_disabled(&state(&session), &nearby_id));
    assert!(!is_disabled(&state(&session), &far_id));
    assert_eq!(unit(&state(&session), &far_id)["location"], "C1");
    assert_eq!(unit(&state(&session), &far_id)["controller"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2418_tapped_spellcaster_freezes_a_newly_summoned_nearby_minion() {
    let encoded = seed_for_second_freeze_new_summon(2418);
    let (mut session, caster_id, new_id) = try_second_freeze_new_summon_prefix(&encoded)
        .expect("tapped Spellcaster new-summon prefix");
    let receipt = freeze_from_caster(&mut session, &caster_id, &new_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert!(is_disabled(&state(&session), &new_id));
    assert_eq!(unit(&state(&session), &new_id)["location"], "C2");
    assert_exact_replay(&session);
}
