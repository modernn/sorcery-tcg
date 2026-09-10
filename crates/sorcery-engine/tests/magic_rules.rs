use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode, Seat};
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

fn site(discard_top_spells: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
    });
    if discard_top_spells {
        value["genesisDiscardTopSpells"] = json!(2);
    }
    value
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn magic(effect: (&str, Value), mana_cost: u8) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": mana_cost,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("Magic facts")
        .insert(effect.0.to_owned(), effect.1);
    value
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(
    seed: u32,
    cards: &Value,
    north_spellbook: &[&str],
    south_spellbook: &[&str],
) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "magic-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-magic-rules-v1",
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
                "spellbook": south_spellbook,
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

fn opening_main(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid Magic scenario");
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
    let action_ids: Vec<IdentityHash> = session
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
#[expect(
    clippy::too_many_lines,
    reason = "the direct proof retains targeting, Deathrite, Death's Door, terminal, and replay"
)]
fn rule_catalog_0019_targeted_magic_is_a_non_unit_source_and_resolves_deathrites() {
    let cards = json!({
        "north-avatar": avatar(1),
        "north-magic": magic(("damageTargetUnit", json!(1)), 1),
        "north-site": site(false),
        "south-avatar": avatar(1),
        "south-minion": minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
            "manaCost": 1,
            "preventsDamageFromUnitsWithPowerAtLeast": 4,
        })),
        "south-site": site(false),
    });
    let manifest = manifest(148, &cards, &["north-magic"; 6], &["south-minion"; 6]);
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let before = state(&session);
    let spell_id = before["players"]["north"]["hand"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("targeted Magic identity")
        .to_owned();
    let target_descriptors: Vec<_> = session
        .legal_actions()
        .expect("targeted Magic actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == spell_id
        })
        .map(|action| action.descriptor)
        .collect();
    let canonical_targets: Vec<_> = target_descriptors
        .iter()
        .map(|descriptor| canonical_json(descriptor).expect("canonical target descriptor"))
        .collect();
    let mut sorted_targets = canonical_targets.clone();
    sorted_targets.sort_unstable();
    assert_eq!(canonical_targets, sorted_targets);
    let mut target_keys: Vec<_> = target_descriptors
        .iter()
        .map(|descriptor| {
            format!(
                "{}:{}:{}",
                descriptor["target"]["kind"].as_str().expect("target kind"),
                descriptor["target"]["seat"].as_str().expect("target seat"),
                descriptor["target"]["instanceId"]
                    .as_str()
                    .expect("target identity")
            )
        })
        .collect();
    target_keys.sort_unstable();
    let mut expected_targets = vec![
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
    expected_targets.sort_unstable();
    assert_eq!(target_keys, expected_targets);

    let north_mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("North mana");
    let north_hand_before = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .len();
    let south_atlas_before = before["players"]["south"]["atlas"]
        .as_array()
        .expect("South Atlas")
        .len();
    let south_atlas_hand_before = before["players"]["south"]["hand"]["atlas"]
        .as_array()
        .expect("South Atlas hand")
        .len();
    let (_, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(killed.events[1].payload["amount"], 1);
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], target_id);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["mana"], north_mana_before - 1);
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North hand")
            .len(),
        north_hand_before - 1
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    assert!(
        !after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == target_id)
    );
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == target_id)
    );
    assert_eq!(
        after["players"]["south"]["atlas"]
            .as_array()
            .expect("South Atlas")
            .len(),
        south_atlas_before - 1
    );
    assert_eq!(
        after["players"]["south"]["hand"]["atlas"]
            .as_array()
            .expect("South Atlas hand")
            .len(),
        south_atlas_hand_before + 1
    );
    assert_eq!(after["phase"], "main");
    assert_exact_replay(&session);

    for _ in 0..2 {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
    });
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 0);
    assert_eq!(state(&session)["terminal"]["status"], "active");

    for _ in 0..2 {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
    let (_, death_blow) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
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
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof retains location order, simultaneous damage, ordered Deathrites, and replay"
)]
fn rule_catalog_0029_minor_explosion_damages_every_unit_at_one_nearby_location() {
    let cards = json!({
        "north-ally": minion(json!({
            "summonToAnySite": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "north-avatar": avatar(20),
        "north-explosion": {
            "cardType": "magic",
            "damageEachUnitAtLocationWithinTwoSteps": 3,
            "manaCost": 1,
            "thresholds": { "air": 0, "earth": 0, "fire": 1, "water": 0 },
        },
        "north-site": {
            "cardType": "site",
            "elements": ["fire"],
        },
        "south-avatar": avatar(20),
        "south-deathrite": minion(json!({
            "deathriteHeal": 3,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "south-site": {
            "cardType": "site",
            "elements": ["fire"],
        },
        "south-stealth": minion(json!({
            "deathriteDrawSite": true,
            "stealth": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "south-warded": minion(json!({
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            "ward": true,
        })),
    });
    let north_spellbook = [
        "north-ally",
        "north-ally",
        "north-ally",
        "north-explosion",
        "north-explosion",
        "north-explosion",
    ];
    let south_spellbook = [
        "south-deathrite",
        "south-deathrite",
        "south-warded",
        "south-warded",
        "south-stealth",
        "south-stealth",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &south_spellbook))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate Minor Explosion session");
            let preview_state = state(&preview);
            let north_hand = preview_state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening Spellbook hand");
            let south_hand = preview_state["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("South opening Spellbook hand");
            ["north-ally", "north-explosion"]
                .into_iter()
                .all(|card_id| north_hand.iter().any(|card| card["cardId"] == card_id))
                && ["south-deathrite", "south-warded", "south-stealth"]
                    .into_iter()
                    .all(|card_id| south_hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("seed with every Minor Explosion scenario card in the opening hands");
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (deathrite_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C2"
    });
    let (warded_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-warded"
            && descriptor["cell"] == "C2"
    });
    let (stealth_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-stealth"
            && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let south_avatar_id = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == south_avatar_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (ally_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C2"
    });

    let before = state(&session);
    let explosion_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-explosion")
        .expect("Minor Explosion in hand")["instanceId"]
        .as_str()
        .expect("Minor Explosion identity")
        .to_owned();
    let north_avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let ally_id = ally_summon["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned();
    let deathrite_id = deathrite_summon["cardInstanceId"]
        .as_str()
        .expect("healing Deathrite identity")
        .to_owned();
    let warded_id = warded_summon["cardInstanceId"]
        .as_str()
        .expect("Ward identity")
        .to_owned();
    let stealth_id = stealth_summon["cardInstanceId"]
        .as_str()
        .expect("Stealth Deathrite identity")
        .to_owned();
    let casts: Vec<_> = session
        .legal_actions()
        .expect("Minor Explosion actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == explosion_id
        })
        .collect();
    assert_eq!(
        casts
            .iter()
            .map(|action| action.descriptor["targetLocation"]["cell"]
                .as_str()
                .expect("target cell"))
            .collect::<Vec<_>>(),
        ["C2", "C3", "C4"]
    );
    let cast = casts
        .into_iter()
        .find(|action| action.descriptor["targetLocation"]["cell"] == "C2")
        .expect("engine-issued C2 Minor Explosion");
    assert_eq!(
        cast.descriptor,
        json!({
            "cardId": "north-explosion",
            "cardInstanceId": explosion_id,
            "casterInstanceId": north_avatar_id,
            "kind": "cast-magic",
            "targetLocation": { "cell": "C2", "region": "surface" },
        })
    );
    assert_eq!(cast.label, "Cast north-explosion at C2 surface");
    let before_mana = before["players"]["north"]["mana"]
        .as_u64()
        .expect("North mana");
    let before_state_version = before["stateVersion"].as_u64().expect("state version");
    let StepResult::Accepted(damaged) = session
        .step(ActionRequest {
            action_id: cast.action_id.to_string(),
            seat: cast.seat,
            state_version: cast.state_version,
        })
        .expect("cast Minor Explosion")
    else {
        panic!("engine-issued Minor Explosion must be accepted");
    };
    assert!(damaged.random_draws.is_empty());
    let mut affected_ids = vec![
        south_avatar_id.clone(),
        ally_id.clone(),
        deathrite_id.clone(),
        warded_id.clone(),
        stealth_id.clone(),
    ];
    affected_ids.sort_unstable();
    assert_eq!(
        damaged
            .events
            .iter()
            .filter(|event| event.event_type == "magic-damage-allocated")
            .map(|event| event.payload.clone())
            .collect::<Vec<_>>(),
        affected_ids
            .into_iter()
            .map(|target_instance_id| json!({
                "amount": 3,
                "sourceInstanceId": explosion_id,
                "targetInstanceId": target_instance_id,
            }))
            .collect::<Vec<_>>()
    );
    let last_allocation = damaged
        .events
        .iter()
        .rposition(|event| event.event_type == "magic-damage-allocated")
        .expect("damage allocations");
    let first_damage = damaged
        .events
        .iter()
        .position(|event| event.event_type == "damage-dealt")
        .expect("resolved damage");
    assert!(last_allocation < first_damage);
    let paused = state(&session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert_eq!(paused["players"]["north"]["mana"], before_mana - 1);
    assert_eq!(paused["players"]["south"]["avatar"]["life"], 17);
    assert!(!damaged.events.iter().any(|event| matches!(
        event.event_type.as_str(),
        "avatar-healed" | "site-drawn" | "minion-died" | "magic-resolved"
    )));
    assert!(
        !damaged
            .events
            .iter()
            .any(|event| event.event_type == "stealth-lost")
    );
    assert!(realm_unit(&paused, &ally_id).is_none());
    assert!(realm_unit(&paused, &deathrite_id).is_none());
    assert!(realm_unit(&paused, &stealth_id).is_none());
    let warded = realm_unit(&paused, &warded_id).expect("Ward survivor");
    assert_eq!(warded["damage"], 0);
    assert_eq!(warded["warded"], false);
    assert!(
        paused["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == explosion_id)
    );
    assert!(
        !paused["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == ally_id)
    );
    let order_actions: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .collect();
    let mut expected_order_ids = vec![deathrite_id.clone(), stealth_id.clone()];
    expected_order_ids.sort_unstable();
    assert_eq!(
        order_actions
            .iter()
            .map(|action| action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source identity")
                .to_owned())
            .collect::<Vec<_>>(),
        expected_order_ids
    );
    let order_heal = order_actions
        .into_iter()
        .find(|action| action.descriptor["sourceInstanceId"] == deathrite_id)
        .expect("engine-issued healing Deathrite first");
    let StepResult::Accepted(completed) = session
        .step(ActionRequest {
            action_id: order_heal.action_id.to_string(),
            seat: order_heal.seat,
            state_version: order_heal.state_version,
        })
        .expect("order Deathrites")
    else {
        panic!("engine-issued Deathrite order must be accepted");
    };
    assert!(completed.random_draws.is_empty());
    let first_death = completed
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .expect("deferred deaths");
    let heal = completed
        .events
        .iter()
        .position(|event| {
            event.event_type == "avatar-healed" && event.payload["sourceInstanceId"] == deathrite_id
        })
        .expect("healing Deathrite");
    let draw = completed
        .events
        .iter()
        .position(|event| {
            event.event_type == "site-drawn" && event.payload["sourceInstanceId"] == stealth_id
        })
        .expect("drawing Deathrite");
    assert!(heal < first_death);
    assert!(draw < first_death);
    assert_eq!(
        completed
            .events
            .last()
            .map(|event| event.event_type.as_str()),
        Some("magic-resolved")
    );
    let after = state(&session);
    assert_eq!(after["stateVersion"], before_state_version + 2);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["players"]["south"]["avatar"]["life"], 20);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == ally_id)
    );
    assert!([deathrite_id, stealth_id].into_iter().all(|instance_id| {
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id)
    }));
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof retains staged choices, Stealth, Ward, Deathrite, and replay"
)]
fn rule_catalog_0030_chain_magic_stages_distinct_nearby_hops_and_resolves_simultaneously() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-chain": {
            "cardType": "magic",
            "damageChainNearbyUnits": true,
            "manaCost": 2,
            "thresholds": { "air": 2, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-deathrite": minion(json!({
            "deathriteDrawSite": true,
            "defense": 2,
            "spellcaster": true,
            "stealth": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "north-site": {
            "cardType": "site",
            "elements": ["air"],
        },
        "north-warded": minion(json!({
            "defense": 2,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            "ward": true,
        })),
        "south-avatar": avatar(20),
        "south-site": {
            "cardType": "site",
            "elements": ["air"],
        },
        "south-stealthed": minion(json!({
            "defense": 2,
            "stealth": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
    });
    let north_spellbook = [
        "north-chain",
        "north-deathrite",
        "north-warded",
        "north-chain",
        "north-deathrite",
        "north-warded",
        "north-chain",
        "north-deathrite",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-stealthed"; 8]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate Chain Magic session");
            let preview_state = state(&preview);
            let hand = preview_state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening Spellbook hand");
            ["north-chain", "north-deathrite", "north-warded"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("seed with Chain Magic and both friendly targets in the opening hand");
    let mut session = opening_main(&manifest);

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let (deathrite_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-deathrite"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (warded_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-warded"
            && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B2"
    });
    let (stealthed_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-stealthed"
            && descriptor["cell"] == "B2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B4"
    });

    let before = state(&session);
    let chain_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-chain")
        .expect("Chain Magic in hand")["instanceId"]
        .as_str()
        .expect("Chain Magic identity")
        .to_owned();
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let deathrite_id = deathrite_summon["cardInstanceId"]
        .as_str()
        .expect("Deathrite identity")
        .to_owned();
    let warded_id = warded_summon["cardInstanceId"]
        .as_str()
        .expect("warded target identity")
        .to_owned();
    let enemy_stealth_id = stealthed_summon["cardInstanceId"]
        .as_str()
        .expect("enemy Stealth identity")
        .to_owned();
    let all_starts: Vec<_> = session
        .legal_actions()
        .expect("Chain Magic starts")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardInstanceId"] == chain_id
        })
        .collect();
    assert!(all_starts.iter().any(|action| {
        action.descriptor["casterInstanceId"] == deathrite_id
            && action.label.starts_with("Choose ")
            && !action.label.contains("with minion")
    }));
    let starts: Vec<_> = all_starts
        .into_iter()
        .filter(|action| action.descriptor["casterInstanceId"] == avatar_id)
        .collect();
    let mut expected_start_ids = vec![avatar_id.clone(), deathrite_id.clone()];
    expected_start_ids.sort_unstable();
    assert_eq!(
        starts
            .iter()
            .map(|action| action.descriptor["target"]["instanceId"]
                .as_str()
                .expect("start target identity")
                .to_owned())
            .collect::<Vec<_>>(),
        expected_start_ids
    );
    assert!(!starts.iter().any(|action| {
        matches!(
            action.descriptor["target"]["instanceId"].as_str(),
            Some(id) if id == warded_id || id == enemy_stealth_id
        )
    }));
    let begin = starts
        .into_iter()
        .find(|action| action.descriptor["target"]["instanceId"] == deathrite_id)
        .expect("engine-issued first hop");
    assert_eq!(
        begin.descriptor,
        json!({
            "cardId": "north-chain",
            "cardInstanceId": chain_id,
            "casterInstanceId": avatar_id,
            "kind": "begin-chain-magic",
            "target": {
                "instanceId": deathrite_id,
                "kind": "minion",
                "seat": "north",
            },
        })
    );
    assert_eq!(
        begin.label,
        format!(
            "Choose minion {}… as the first target for north-chain",
            &deathrite_id[..15]
        )
    );
    let before_mana = before["players"]["north"]["mana"]
        .as_u64()
        .expect("North mana");
    let transcript_before = session.transcript().len();
    let StepResult::Accepted(begin_receipt) = session
        .step(ActionRequest {
            action_id: begin.action_id.to_string(),
            seat: begin.seat,
            state_version: begin.state_version,
        })
        .expect("begin Chain Magic")
    else {
        panic!("engine-issued first hop must be accepted");
    };
    assert!(begin_receipt.events.is_empty());
    assert!(begin_receipt.random_draws.is_empty());
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(staged["players"]["north"]["mana"], before_mana);
    assert_eq!(
        staged["pendingChainMagic"],
        json!({
            "cardId": "north-chain",
            "cardInstanceId": chain_id,
            "casterInstanceId": avatar_id,
            "seat": "north",
            "targets": [{
                "instanceId": deathrite_id,
                "kind": "minion",
                "seat": "north",
            }],
        })
    );

    let staged_actions = session.legal_actions().expect("staged Chain Magic actions");
    let extend_target_ids: Vec<_> = staged_actions
        .iter()
        .filter(|action| action.descriptor["kind"] == "extend-chain-magic")
        .map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .expect("extension target identity")
                .to_owned()
        })
        .collect();
    let mut expected_extension_ids = vec![avatar_id.clone(), warded_id.clone()];
    expected_extension_ids.sort_unstable();
    assert_eq!(extend_target_ids, expected_extension_ids);
    assert!(!extend_target_ids.contains(&deathrite_id));
    assert!(!extend_target_ids.contains(&enemy_stealth_id));
    assert_eq!(
        staged_actions
            .last()
            .expect("resolve action after canonical extensions")
            .descriptor,
        json!({ "kind": "resolve-chain-magic" })
    );
    let extend = staged_actions
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "extend-chain-magic"
                && action.descriptor["target"]["instanceId"] == warded_id
        })
        .expect("engine-issued second hop");
    assert_eq!(
        extend.descriptor,
        json!({
            "kind": "extend-chain-magic",
            "target": {
                "instanceId": warded_id,
                "kind": "minion",
                "seat": "north",
            },
        })
    );
    assert_eq!(
        extend.label,
        format!(
            "Add minion {}… as a chained target (+2 mana)",
            &warded_id[..15]
        )
    );
    let StepResult::Accepted(extend_receipt) = session
        .step(ActionRequest {
            action_id: extend.action_id.to_string(),
            seat: extend.seat,
            state_version: extend.state_version,
        })
        .expect("extend Chain Magic")
    else {
        panic!("engine-issued second hop must be accepted");
    };
    assert!(extend_receipt.events.is_empty());
    assert!(extend_receipt.random_draws.is_empty());
    assert_eq!(state(&session)["players"]["north"]["mana"], before_mana);

    let final_actions = session.legal_actions().expect("final Chain Magic actions");
    assert_eq!(final_actions.len(), 1);
    let finish = &final_actions[0];
    assert_eq!(finish.descriptor, json!({ "kind": "resolve-chain-magic" }));
    assert_eq!(
        finish.label,
        "Cast north-chain through 2 chosen units (4 mana)"
    );
    let StepResult::Accepted(resolved) = session
        .step(ActionRequest {
            action_id: finish.action_id.to_string(),
            seat: finish.seat,
            state_version: finish.state_version,
        })
        .expect("resolve Chain Magic")
    else {
        panic!("engine-issued Chain Magic resolution must be accepted");
    };
    assert!(resolved.random_draws.is_empty());
    assert_eq!(
        resolved
            .events
            .iter()
            .filter(|event| event.event_type == "magic-damage-allocated")
            .map(|event| event.payload.clone())
            .collect::<Vec<_>>(),
        [deathrite_id.clone(), warded_id.clone()]
            .into_iter()
            .map(|target_instance_id| json!({
                "amount": 2,
                "sourceInstanceId": chain_id,
                "targetInstanceId": target_instance_id,
            }))
            .collect::<Vec<_>>()
    );
    let first_death = resolved
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .expect("Deathrite target death");
    assert!(
        resolved
            .events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.event_type == "damage-dealt")
            .all(|(index, _)| index < first_death)
    );
    assert!(
        resolved
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken"
                && event.payload["instanceId"] == warded_id)
    );
    assert_eq!(
        resolved
            .events
            .iter()
            .find(|event| {
                event.event_type == "damage-dealt" && event.payload["instanceId"] == warded_id
            })
            .expect("Ward prevention damage event")
            .payload,
        json!({
            "amount": 0,
            "attemptedAmount": 2,
            "direct": true,
            "instanceId": warded_id,
            "prevented": true,
            "seat": "north",
        })
    );
    let site_drawn = resolved
        .events
        .iter()
        .position(|event| {
            event.event_type == "site-drawn" && event.payload["sourceInstanceId"] == deathrite_id
        })
        .expect("Deathrite site draw");
    assert!(site_drawn < first_death);
    assert_eq!(
        resolved
            .events
            .last()
            .map(|event| event.event_type.as_str()),
        Some("magic-resolved")
    );
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingChainMagic"].is_null());
    assert_eq!(after["players"]["north"]["mana"], before_mana - 4);
    assert!(realm_unit(&after, &deathrite_id).is_none());
    assert_eq!(
        realm_unit(&after, &warded_id).expect("Ward survivor")["warded"],
        false
    );
    assert!(realm_unit(&after, &enemy_stealth_id).is_some());
    assert_eq!(session.transcript().len(), transcript_before + 3);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one Chain Magic filter proof keeps low-mana and underground hop exclusion together"
)]
fn chain_magic_should_require_mana_and_same_region_hops() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-burrower": minion(json!({
            "burrowing": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "north-chain": {
            "cardType": "magic",
            "damageChainNearbyUnits": true,
            "manaCost": 2,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-site": site(false),
        "north-target": minion(json!({
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-chain",
        "north-target",
        "north-burrower",
        "north-chain",
        "north-target",
        "north-burrower",
    ];
    let chosen = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-minion"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Chain Magic filter candidate");
            let preview_state = state(&preview);
            let hand = preview_state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand");
            ["north-chain", "north-target", "north-burrower"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("seed with Chain Magic and both minions");
    let mut session = opening_main(&chosen);
    let chain_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-chain")
        .expect("Chain Magic in hand")["instanceId"]
        .as_str()
        .expect("Chain Magic identity")
        .to_owned();
    assert_eq!(state(&session)["players"]["north"]["mana"], 1);
    assert!(
        session
            .legal_actions()
            .expect("low-mana actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "begin-chain-magic"
                    || action.descriptor["cardInstanceId"] != chain_id
            })
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-target"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-burrower"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    });

    let target_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-target")
        .expect("surface target")["instanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    let burrower_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-burrower")
        .expect("burrower")["instanceId"]
        .as_str()
        .expect("burrower identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
    });
    assert!(
        session
            .legal_actions()
            .expect("staged Chain Magic")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "extend-chain-magic"
                    || action.descriptor["target"]["instanceId"] != burrower_id
            })
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof retains region filtering, Stealth, Ward, Deathrite, and replay"
)]
fn rule_catalog_0031_rain_of_arrows_simultaneously_damages_every_surface_minion() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-burrower": minion(json!({
            "burrowing": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "north-bury": {
            "burrowTargetMinionOrArtifact": true,
            "cardType": "magic",
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-deathrite": minion(json!({
            "deathriteDrawSite": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "north-rain": {
            "cardType": "magic",
            "damageEachAbovegroundMinion": 1,
            "manaCost": 1,
            "thresholds": { "air": 1, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-site": {
            "cardType": "site",
            "elements": ["air"],
        },
        "south-avatar": avatar(20),
        "south-site": {
            "cardType": "site",
            "elements": ["air"],
        },
        "south-stealth": minion(json!({
            "stealth": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "south-warded": minion(json!({
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            "ward": true,
        })),
    });
    let north_spellbook = [
        "north-bury",
        "north-deathrite",
        "north-burrower",
        "north-rain",
        "north-rain",
        "north-rain",
        "north-rain",
        "north-rain",
    ];
    let south_spellbook = [
        "south-stealth",
        "south-warded",
        "south-stealth",
        "south-warded",
        "south-stealth",
        "south-warded",
        "south-stealth",
        "south-warded",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &south_spellbook))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate Rain of Arrows session");
            let preview_state = state(&preview);
            let north_hand = preview_state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening Spellbook hand");
            let south_hand = preview_state["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("South opening Spellbook hand");
            ["north-bury", "north-deathrite", "north-burrower"]
                .into_iter()
                .all(|card_id| north_hand.iter().any(|card| card["cardId"] == card_id))
                && ["south-stealth", "south-warded"]
                    .into_iter()
                    .all(|card_id| south_hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("seed with every Rain fixture in the opening hands");
    let mut session = opening_main(&manifest);
    let (deathrite_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-deathrite"
            && descriptor["cell"] == "C4"
    });
    let (burrower_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-burrower"
            && descriptor["cell"] == "C4"
    });
    let deathrite_id = deathrite_summon["cardInstanceId"]
        .as_str()
        .expect("Deathrite identity")
        .to_owned();
    let burrower_id = burrower_summon["cardInstanceId"]
        .as_str()
        .expect("Burrower identity")
        .to_owned();
    let bury_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();
    cast_bury(&mut session, &burrower_id, &bury_id);
    assert_eq!(
        realm_unit(&state(&session), &burrower_id).expect("underground Burrower")["region"],
        "underground"
    );

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (stealth_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-stealth"
            && descriptor["cell"] == "C1"
    });
    let stealth_id = stealth_summon["cardInstanceId"]
        .as_str()
        .expect("Stealth identity")
        .to_owned();
    let (warded_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-warded"
            && descriptor["cell"] == "C1"
    });
    let warded_id = warded_summon["cardInstanceId"]
        .as_str()
        .expect("Ward identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let before = state(&session);
    let rain_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-rain")
        .expect("Rain of Arrows in hand")["instanceId"]
        .as_str()
        .expect("Rain of Arrows identity")
        .to_owned();
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let casts: Vec<_> = session
        .legal_actions()
        .expect("Rain of Arrows actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == rain_id
        })
        .collect();
    assert_eq!(casts.len(), 1);
    assert_eq!(
        casts[0].descriptor,
        json!({
            "cardId": "north-rain",
            "cardInstanceId": rain_id,
            "casterInstanceId": avatar_id,
            "kind": "cast-magic",
        })
    );
    assert_eq!(casts[0].label, "Cast north-rain");
    let before_mana = before["players"]["north"]["mana"]
        .as_u64()
        .expect("North mana");
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let StepResult::Accepted(resolved) = session
        .step(ActionRequest {
            action_id: casts[0].action_id.to_string(),
            seat: casts[0].seat,
            state_version: casts[0].state_version,
        })
        .expect("cast Rain of Arrows")
    else {
        panic!("engine-issued Rain of Arrows must be accepted");
    };
    assert!(resolved.random_draws.is_empty());
    assert_eq!(
        resolved
            .events
            .first()
            .map(|event| event.event_type.as_str()),
        Some("magic-cast")
    );
    let mut affected = vec![deathrite_id.clone(), stealth_id.clone(), warded_id.clone()];
    affected.sort_unstable();
    assert_eq!(
        resolved
            .events
            .iter()
            .filter(|event| event.event_type == "magic-damage-allocated")
            .map(|event| event.payload.clone())
            .collect::<Vec<_>>(),
        affected
            .into_iter()
            .map(|target_instance_id| json!({
                "amount": 1,
                "sourceInstanceId": rain_id,
                "targetInstanceId": target_instance_id,
            }))
            .collect::<Vec<_>>()
    );
    let first_death = resolved
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .expect("surface Deathrite death");
    assert!(
        resolved
            .events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.event_type == "damage-dealt")
            .all(|(index, _)| index < first_death)
    );
    let site_drawn = resolved
        .events
        .iter()
        .position(|event| {
            event.event_type == "site-drawn" && event.payload["sourceInstanceId"] == deathrite_id
        })
        .expect("Deathrite site draw");
    assert!(site_drawn < first_death);
    assert!(
        resolved
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken"
                && event.payload["instanceId"] == warded_id)
    );
    assert_eq!(
        resolved
            .events
            .iter()
            .find(|event| {
                event.event_type == "damage-dealt" && event.payload["instanceId"] == warded_id
            })
            .expect("Ward damage event")
            .payload,
        json!({
            "amount": 0,
            "attemptedAmount": 1,
            "direct": true,
            "instanceId": warded_id,
            "prevented": true,
            "seat": "south",
        })
    );
    assert!(
        !resolved
            .events
            .iter()
            .any(|event| event.event_type == "stealth-lost")
    );
    assert_eq!(
        resolved
            .events
            .last()
            .map(|event| event.event_type.as_str()),
        Some("magic-resolved")
    );

    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert_eq!(after["stateVersion"], before_version + 1);
    assert_eq!(after["players"]["north"]["mana"], before_mana - 1);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 20);
    assert!(realm_unit(&after, &deathrite_id).is_none());
    assert!(realm_unit(&after, &stealth_id).is_none());
    let warded = realm_unit(&after, &warded_id).expect("Ward survivor");
    assert_eq!(warded["damage"], 0);
    assert_eq!(warded["warded"], false);
    let burrower = realm_unit(&after, &burrower_id).expect("excluded Burrower");
    assert_eq!(burrower["damage"], 0);
    assert_eq!(burrower["region"], "underground");
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == rain_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == deathrite_id)
    );
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == stealth_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0045_drown_should_submerge_a_target_minion_only_when_able() {
    let (mut swimmer, swimmer_id, swimmer_spell) =
        drown_checkpoint(450, json!({ "submerge": true }), true);
    let drown_actions: Vec<Value> = swimmer
        .legal_actions()
        .expect("Drown actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == swimmer_spell
        })
        .map(|action| action.descriptor)
        .collect();
    assert_eq!(drown_actions.len(), 1);
    assert_eq!(drown_actions[0]["target"]["kind"], "minion");
    assert_eq!(drown_actions[0]["target"]["instanceId"], swimmer_id);

    let submerged = cast_drown(&mut swimmer, &swimmer_id, &swimmer_spell);
    assert_eq!(
        event_types(&submerged),
        ["magic-cast", "minion-submerged", "magic-resolved"]
    );
    assert!(submerged.random_draws.is_empty());
    assert_eq!(
        submerged.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": swimmer_id,
            "seat": "south",
            "sourceInstanceId": swimmer_spell,
        })
    );
    assert_eq!(
        realm_unit(&state(&swimmer), &swimmer_id).expect("submerged swimmer")["region"],
        "underwater"
    );
    assert!(
        !swimmer
            .legal_actions()
            .expect("post-Drown actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["target"]["instanceId"] == swimmer_id
            })
    );
    assert_exact_replay(&swimmer);

    let (mut lander, lander_id, lander_spell) = drown_checkpoint(451, json!({}), true);
    let drowned = cast_drown(&mut lander, &lander_id, &lander_spell);
    assert_eq!(
        event_types(&drowned),
        [
            "magic-cast",
            "minion-submerged",
            "minion-died",
            "magic-resolved",
        ]
    );
    let lander_state = state(&lander);
    assert!(realm_unit(&lander_state, &lander_id).is_none());
    assert!(
        lander_state["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == lander_id)
    );
    assert_exact_replay(&lander);

    let (mut warded, warded_id, warded_spell) =
        drown_checkpoint(452, json!({ "submerge": true, "ward": true }), true);
    let ward = cast_drown(&mut warded, &warded_id, &warded_spell);
    assert_eq!(
        event_types(&ward),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let warded_state = state(&warded);
    let warded_unit = realm_unit(&warded_state, &warded_id).expect("Ward survivor");
    assert_eq!(warded_unit["region"], "surface");
    assert_eq!(warded_unit["warded"], false);
    assert_exact_replay(&warded);

    let (mut dry, dry_id, dry_spell) = drown_checkpoint(453, json!({ "submerge": true }), false);
    let dry_before = realm_unit(&state(&dry), &dry_id)
        .expect("dry-land target")
        .clone();
    let blocked = cast_drown(&mut dry, &dry_id, &dry_spell);
    assert_eq!(event_types(&blocked), ["magic-cast", "magic-resolved"]);
    assert_eq!(
        realm_unit(&state(&dry), &dry_id).expect("unmoved target"),
        &dry_before
    );
    assert_exact_replay(&dry);
}

fn bury_checkpoint(seed: u32, target_extra: Value, water: bool) -> (Session, String, String) {
    let mut cards = json!({
        "north-avatar": avatar(20),
        "north-bury": magic(("burrowTargetMinionOrArtifact", json!(true)), 1),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(target_extra),
        "south-site": site(false),
    });
    if water {
        cards["south-site"]["elements"] = json!(["earth", "water"]);
    }
    let manifest = manifest(seed, &cards, &["north-bury"; 6], &["south-minion"; 6]);
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Bury target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let spell_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();
    (session, target_id, spell_id)
}

fn drown_checkpoint(seed: u32, target_extra: Value, water: bool) -> (Session, String, String) {
    let mut cards = json!({
        "north-avatar": avatar(20),
        "north-drown": magic(("submergeTargetMinion", json!(true)), 1),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(target_extra),
        "south-site": site(false),
    });
    if water {
        cards["south-site"]["elements"] = json!(["earth", "water"]);
    }
    let manifest = manifest(seed, &cards, &["north-drown"; 6], &["south-minion"; 6]);
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Drown target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let spell_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-drown")
        .expect("Drown in hand")["instanceId"]
        .as_str()
        .expect("Drown identity")
        .to_owned();
    (session, target_id, spell_id)
}

fn cast_drown(session: &mut Session, target_id: &str, spell_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn cast_bury(session: &mut Session, target_id: &str, spell_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn realm_unit<'a>(value: &'a Value, instance_id: &str) -> Option<&'a Value> {
    value["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof retains Bury survival, death, Ward, terrain, checkpoint, and replay"
)]
fn rule_catalog_0042_bury_moves_and_immediately_settles_a_minion() {
    let (mut survivor, survivor_id, survivor_spell) =
        bury_checkpoint(420, json!({ "burrowing": true }), false);
    let actions = survivor.legal_actions().expect("Bury actions");
    let bury_actions: Vec<_> = actions
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == survivor_spell
        })
        .collect();
    assert_eq!(bury_actions.len(), 1);
    assert_eq!(bury_actions[0].descriptor["target"]["kind"], "minion");
    assert_eq!(
        bury_actions[0].descriptor["target"]["instanceId"],
        survivor_id
    );

    let checkpoint = create_game_checkpoint(&survivor).expect("captured Bury checkpoint");
    let restored = resume_game_checkpoint(
        &parse_game_checkpoint(
            &serialize_game_checkpoint(&checkpoint).expect("serialized Bury checkpoint"),
        )
        .expect("parsed Bury checkpoint"),
    )
    .expect("restored Bury checkpoint");
    assert_eq!(
        restored.replay_value().expect("restored Bury state"),
        survivor.replay_value().expect("source Bury state")
    );
    assert_eq!(
        restored.legal_actions().expect("restored Bury actions"),
        actions
    );

    let survived = cast_bury(&mut survivor, &survivor_id, &survivor_spell);
    assert_eq!(
        event_types(&survived),
        ["magic-cast", "minion-burrowed", "magic-resolved"]
    );
    assert!(survived.random_draws.is_empty());
    assert_eq!(
        survived.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": survivor_id,
            "seat": "south",
            "sourceInstanceId": survivor_spell,
        })
    );
    assert_eq!(
        realm_unit(&state(&survivor), &survivor_id).expect("burrowed survivor")["region"],
        "underground"
    );
    assert!(
        !survivor
            .legal_actions()
            .expect("post-Bury actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["target"]["instanceId"] == survivor_id
            })
    );
    assert_exact_replay(&survivor);

    let (mut ordinary, ordinary_id, ordinary_spell) = bury_checkpoint(421, json!({}), false);
    let died = cast_bury(&mut ordinary, &ordinary_id, &ordinary_spell);
    assert_eq!(
        event_types(&died),
        [
            "magic-cast",
            "minion-burrowed",
            "minion-died",
            "magic-resolved",
        ]
    );
    let ordinary_state = state(&ordinary);
    assert!(realm_unit(&ordinary_state, &ordinary_id).is_none());
    assert!(
        ordinary_state["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == ordinary_id)
    );
    assert_exact_replay(&ordinary);

    let (mut warded, warded_id, warded_spell) =
        bury_checkpoint(422, json!({ "ward": true }), false);
    let ward = cast_bury(&mut warded, &warded_id, &warded_spell);
    assert_eq!(
        event_types(&ward),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let warded_state = state(&warded);
    let warded_unit = realm_unit(&warded_state, &warded_id).expect("Ward survivor");
    assert_eq!(warded_unit["region"], "surface");
    assert_eq!(warded_unit["warded"], false);
    assert_exact_replay(&warded);

    let (mut water, water_id, water_spell) = bury_checkpoint(423, json!({}), true);
    let water_before = realm_unit(&state(&water), &water_id)
        .expect("Water target")
        .clone();
    let blocked = cast_bury(&mut water, &water_id, &water_spell);
    assert_eq!(event_types(&blocked), ["magic-cast", "magic-resolved"]);
    assert_eq!(
        realm_unit(&state(&water), &water_id).expect("blocked target"),
        &water_before
    );
    assert_exact_replay(&water);

    let (mut deathrite, deathrite_id, deathrite_spell) =
        bury_checkpoint(424, json!({ "deathriteDrawSite": true }), false);
    let deathrite_result = cast_bury(&mut deathrite, &deathrite_id, &deathrite_spell);
    assert_eq!(
        event_types(&deathrite_result),
        [
            "magic-cast",
            "minion-burrowed",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_exact_replay(&deathrite);
}

fn bury_artifact_site(water: bool) -> Value {
    let mut site = json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisGainMana": 6,
    });
    if water {
        site["elements"] = json!(["water"]);
    }
    site
}

fn bury_artifact_manifest(seed: u32, water: bool) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "bury-artifact-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-bury-artifact-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-bury": {
                "burrowTargetMinionOrArtifact": true,
                "cardType": "magic",
                "manaCost": 1,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-site": bury_artifact_site(false),
            "south-artifact": {
                "cardType": "artifact",
                "grantsBearerPower": 2,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-avatar": avatar(20),
            "south-site": bury_artifact_site(water),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 4],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bury"; 4],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": vec!["south-artifact"; 4],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn realm_artifact<'a>(value: &'a Value, instance_id: &str) -> Option<&'a Value> {
    value["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
}

fn setup_bury_artifact(carried: bool, water: bool) -> (Session, String, String, Value) {
    let mut session = opening_main(&bury_artifact_manifest(156, water));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-artifact"
            && if carried {
                descriptor["bearer"]["kind"] == "avatar"
            } else {
                descriptor["bearer"].is_null() && descriptor["cell"] == "C1"
            }
    });
    let artifact_id = state(&session)["realm"]["artifacts"][0]["instanceId"]
        .as_str()
        .expect("artifact identity")
        .to_owned();
    let before_cast = state(&session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let spell_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();
    let bury_actions: Vec<_> = session
        .legal_actions()
        .expect("Bury choices")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == spell_id
                && action.descriptor["targetArtifactInstanceId"] == artifact_id
        })
        .collect();
    assert_eq!(bury_actions.len(), 1);
    assert!(bury_actions[0].label.contains("artifact"));
    (session, artifact_id, spell_id, before_cast)
}

fn cast_bury_artifact(session: &mut Session, artifact_id: &str, spell_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["targetArtifactInstanceId"] == artifact_id
    });
    receipt
}

#[test]
fn rule_catalog_0043_bury_detaches_and_burrows_artifacts() {
    let (mut uncarried, uncarried_id, uncarried_spell, uncarried_before) =
        setup_bury_artifact(false, false);
    let uncarried_receipt = cast_bury_artifact(&mut uncarried, &uncarried_id, &uncarried_spell);
    assert_eq!(
        event_types(&uncarried_receipt),
        ["magic-cast", "artifact-burrowed", "magic-resolved"]
    );
    assert_eq!(
        realm_artifact(&state(&uncarried), &uncarried_id).expect("burrowed artifact")["region"],
        "underground"
    );
    assert_eq!(
        realm_artifact(&state(&uncarried), &uncarried_id).expect("burrowed artifact")["location"],
        realm_artifact(&uncarried_before, &uncarried_id).expect("surface artifact")["location"]
    );
    assert_eq!(
        uncarried_receipt.events[0].payload["targetArtifactInstanceId"],
        uncarried_id
    );
    assert_exact_replay(&uncarried);

    let (mut carried, carried_id, carried_spell, _) = setup_bury_artifact(true, false);
    let carried_receipt = cast_bury_artifact(&mut carried, &carried_id, &carried_spell);
    assert_eq!(
        event_types(&carried_receipt),
        ["magic-cast", "artifact-burrowed", "magic-resolved"]
    );
    let carried_state = state(&carried);
    let carried_artifact = realm_artifact(&carried_state, &carried_id).expect("detached artifact");
    assert_eq!(carried_artifact["location"], "C1");
    assert_eq!(carried_artifact["owner"], "south");
    assert_eq!(carried_artifact["region"], "underground");
    assert!(carried_artifact.get("bearer").is_none());
    assert_exact_replay(&carried);

    let (mut water, water_id, water_spell, water_before) = setup_bury_artifact(false, true);
    let water_receipt = cast_bury_artifact(&mut water, &water_id, &water_spell);
    assert_eq!(
        event_types(&water_receipt),
        ["magic-cast", "magic-resolved"]
    );
    let water_state = state(&water);
    assert_eq!(
        realm_artifact(&water_state, &water_id),
        realm_artifact(&water_before, &water_id)
    );
    assert_exact_replay(&water);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one Cave-In proof retains canonical terrain choices, simultaneous burrowing, ordered Deathrites, and replay"
)]
fn rule_catalog_0044_cave_in_minion_slice_should_burrow_in_canonical_order() {
    let mut north_water = site(false);
    north_water["elements"] = json!(["earth", "water"]);
    let mut south_geomancer = avatar(20);
    south_geomancer["earthSitePlayCreatesAdjacentRubble"] = json!(true);
    let cards = json!({
        "north-avatar": avatar(20),
        "north-cave-in": magic(("burrowAllMinionsAndArtifactsAtTargetLandSite", json!(true)), 0),
        "north-site": north_water,
        "south-avatar": south_geomancer,
        "south-cave-in": magic(("burrowAllMinionsAndArtifactsAtTargetLandSite", json!(true)), 0),
        "south-site": site(false),
        "south-survivor": minion(json!({
            "burrowing": true,
            "spellcaster": true,
            "stealth": true,
            "ward": true,
        })),
        "south-victim": minion(json!({ "deathriteDrawSite": true })),
    });
    let south_spellbook = [
        "south-survivor",
        "south-victim",
        "south-victim",
        "south-cave-in",
        "south-cave-in",
        "south-cave-in",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &["north-cave-in"; 6], &south_spellbook))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Cave-In candidate");
            let preview_state = state(&preview);
            let hand = preview_state["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("South opening hand");
            hand.iter()
                .filter(|card| card["cardId"] == "south-survivor")
                .count()
                >= 1
                && hand
                    .iter()
                    .filter(|card| card["cardId"] == "south-victim")
                    .count()
                    >= 2
        })
        .expect("bounded seed with one survivor and two victims");

    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cell"] == "C1"
            && descriptor["createRubbleAt"] == "B1"
    });
    let south_avatar_id = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    let (survivor, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-survivor"
            && descriptor["cell"] == "C1"
            && descriptor["casterInstanceId"] == south_avatar_id
    });
    let survivor_id = survivor["cardInstanceId"]
        .as_str()
        .expect("Cave-In survivor identity")
        .to_owned();
    let mut victim_ids = Vec::new();
    for _ in 0..2 {
        let (victim, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["region"].is_null()
                && descriptor["cardId"] == "south-victim"
                && descriptor["cell"] == "C1"
                && descriptor["casterInstanceId"] == south_avatar_id
        });
        victim_ids.push(
            victim["cardInstanceId"]
                .as_str()
                .expect("Cave-In victim identity")
                .to_owned(),
        );
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let before = state(&session);
    let spell_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-cave-in")
        .expect("Cave-In in hand")["instanceId"]
        .as_str()
        .expect("Cave-In identity")
        .to_owned();
    let caster_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("surface Avatar identity");
    let rubble_id = before["realm"]["sites"]["B1"]["instanceId"]
        .as_str()
        .expect("Rubble identity");
    let land_site_id = before["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("Land Site identity");
    let cave_in_actions: Vec<_> = session
        .legal_actions()
        .expect("Cave-In actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == spell_id
        })
        .collect();
    assert_eq!(cave_in_actions.len(), 2);
    assert_eq!(
        cave_in_actions
            .iter()
            .map(|action| {
                (
                    action.descriptor["targetLocation"]["cell"]
                        .as_str()
                        .expect("Cave-In target cell"),
                    action.descriptor["targetSiteInstanceId"]
                        .as_str()
                        .expect("Cave-In target Site identity"),
                )
            })
            .collect::<Vec<_>>(),
        [("B1", rubble_id), ("C1", land_site_id)]
    );
    assert!(cave_in_actions.iter().all(|action| {
        action.descriptor["casterInstanceId"] == caster_id
            && action.descriptor["targetLocation"]["region"] == "surface"
            && action.descriptor["targetLocation"]["cell"] != "C4"
    }));

    let (_, cast) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == land_site_id
    });
    let mut burrowed_ids = vec![survivor_id.clone()];
    burrowed_ids.extend(victim_ids.iter().cloned());
    burrowed_ids.sort();
    assert_eq!(
        event_types(&cast),
        [
            "magic-cast",
            "minion-burrowed",
            "minion-burrowed",
            "minion-burrowed",
        ]
    );
    assert_eq!(
        cast.events[0].payload["targetLocation"],
        json!({
            "cell": "C1",
            "region": "surface",
        })
    );
    assert_eq!(cast.events[0].payload["targetSiteInstanceId"], land_site_id);
    assert_eq!(
        cast.events
            .iter()
            .filter(|event| event.event_type == "minion-burrowed")
            .map(|event| {
                assert_eq!(event.payload["cell"], "C1");
                assert_eq!(event.payload["seat"], "south");
                assert_eq!(event.payload["sourceInstanceId"], spell_id);
                event.payload["instanceId"]
                    .as_str()
                    .expect("burrowed identity")
                    .to_owned()
            })
            .collect::<Vec<_>>(),
        burrowed_ids
    );
    assert!(cast.random_draws.is_empty());
    let pending = state(&session);
    let survivor = realm_unit(&pending, &survivor_id).expect("Burrowing survivor");
    assert_eq!(survivor["region"], "underground");
    assert_eq!(survivor["stealthed"], true);
    assert_eq!(survivor["warded"], true);
    assert!(
        victim_ids
            .iter()
            .all(|instance_id| realm_unit(&pending, instance_id).is_none())
    );
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(
        pending["pendingDeathrites"]["deferredOutcomes"],
        json!([{
            "payload": {
                "cardId": "north-cave-in",
                "instanceId": spell_id,
                "owner": "north",
            },
            "type": "magic-resolved",
        }])
    );

    victim_ids.sort();
    let (_, ordered) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == victim_ids[0]
    });
    assert_eq!(
        event_types(&ordered),
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert!(ordered.random_draws.is_empty());
    let completed = state(&session);
    assert_eq!(completed["phase"], "main");
    assert!(victim_ids.iter().all(|instance_id| {
        completed["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id.as_str())
    }));

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let south_turn = state(&session);
    let south_spell_id = south_turn["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South hand")
        .iter()
        .find(|card| card["cardId"] == "south-cave-in")
        .expect("South Cave-In in hand")["instanceId"]
        .as_str()
        .expect("South Cave-In identity");
    let south_avatar_id = south_turn["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity");
    let south_cave_actions: Vec<_> = session
        .legal_actions()
        .expect("South Cave-In actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == south_spell_id
        })
        .collect();
    assert!(!south_cave_actions.is_empty());
    assert!(south_cave_actions.iter().all(|action| {
        action.descriptor["casterInstanceId"] == south_avatar_id
            && action.descriptor["casterInstanceId"] != survivor_id
            && action.descriptor["targetLocation"]["region"] == "surface"
    }));
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one Cave-In Artifact proof retains bearer variants, canonical burrow order, and replay"
)]
fn rule_catalog_0161_cave_in_burrows_artifacts_with_minions_in_canonical_order() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-cave-in": {
            "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
            "cardType": "magic",
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-water-site": {
            "cardType": "site",
            "elements": ["water"],
        },
        "south-artifact": {
            "cardType": "artifact",
            "grantsBearerPower": 2,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "south-avatar": avatar(20),
        "south-burrower": minion(json!({
            "burrowing": true,
            "stealth": true,
            "ward": true,
        })),
        "south-site": site(false),
        "south-victim": minion(json!({})),
    });
    let manifest = finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cave-in-artifact-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cave-in-artifact-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-water-site"; 4],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 4],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": vec!["south-burrower", "south-victim", "south-artifact"],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 157,
    }));

    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let south_avatar_id = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    let (burrower, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-burrower"
            && descriptor["cell"] == "C1"
            && descriptor["casterInstanceId"] == south_avatar_id
    });
    let burrower_id = burrower["cardInstanceId"]
        .as_str()
        .expect("burrower identity")
        .to_owned();
    let (victim, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-victim"
            && descriptor["cell"] == "C1"
            && descriptor["casterInstanceId"] == south_avatar_id
    });
    let victim_id = victim["cardInstanceId"]
        .as_str()
        .expect("victim identity")
        .to_owned();
    let artifact_id = state(&session)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South hand")
        .iter()
        .find(|card| card["cardId"] == "south-artifact")
        .expect("Artifact in hand")["instanceId"]
        .as_str()
        .expect("Artifact identity")
        .to_owned();
    let checkpoint = create_game_checkpoint(&session).expect("Cave-In Artifact checkpoint");

    let cast_cave_in = |bearer: &str| {
        let mut session = resume_game_checkpoint(&checkpoint).expect("restored Cave-In checkpoint");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-artifact"
                && descriptor["cardInstanceId"] == artifact_id
                && if bearer == "avatar" {
                    descriptor["bearer"]["kind"] == "avatar"
                } else {
                    descriptor["bearer"]["kind"] == "minion"
                        && descriptor["bearer"]["instanceId"] == burrower_id
                }
        });
        let artifact_before = realm_artifact(&state(&session), &artifact_id)
            .expect("cast artifact")
            .clone();
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        let turn_state = state(&session);
        let spell_id = turn_state["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North hand")
            .iter()
            .find(|card| card["cardId"] == "north-cave-in")
            .expect("Cave-In in hand")["instanceId"]
            .as_str()
            .expect("Cave-In identity")
            .to_owned();
        let land_site_id = turn_state["realm"]["sites"]["C1"]["instanceId"]
            .as_str()
            .expect("Land Site identity")
            .to_owned();
        let (_, cast) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardInstanceId"] == spell_id
                && descriptor["targetLocation"]["cell"] == "C1"
                && descriptor["targetSiteInstanceId"] == land_site_id
        });
        let mut burrow_ids = vec![artifact_id.clone(), burrower_id.clone(), victim_id.clone()];
        burrow_ids.sort();
        let burrow_events: Vec<_> = cast
            .events
            .iter()
            .filter(|event| {
                event.event_type == "minion-burrowed" || event.event_type == "artifact-burrowed"
            })
            .map(|event| {
                event.payload["instanceId"]
                    .as_str()
                    .expect("burrowed identity")
                    .to_owned()
            })
            .collect();
        assert_eq!(burrow_events, burrow_ids);
        assert!(
            cast.events
                .iter()
                .any(|event| event.event_type == "minion-died")
        );
        let after = state(&session);
        assert_eq!(after["phase"], "main");
        let surviving_burrower = realm_unit(&after, &burrower_id).expect("burrowing survivor");
        assert_eq!(surviving_burrower["region"], "underground");
        assert_eq!(surviving_burrower["stealthed"], true);
        assert_eq!(surviving_burrower["warded"], true);
        assert!(realm_unit(&after, &victim_id).is_none());
        assert_eq!(after["players"]["south"]["avatar"]["region"], "surface");
        let moved_artifact = realm_artifact(&after, &artifact_id).expect("moved artifact");
        if bearer == "minion" {
            assert_eq!(moved_artifact, &artifact_before);
        } else {
            assert_eq!(moved_artifact["location"], "C1");
            assert_eq!(moved_artifact["owner"], "south");
            assert_eq!(moved_artifact["region"], "underground");
            assert!(moved_artifact.get("bearer").is_none());
        }
        assert!(cast.random_draws.is_empty());
        assert_exact_replay(&session);
    };

    cast_cave_in("minion");
    cast_cave_in("avatar");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one ordered Bury proof retains setup, deferred state, checkpoint, completion, and replay"
)]
fn bury_should_defer_completion_until_ordered_static_deathrites_finish() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-bury": magic(("burrowTargetMinionOrArtifact", json!(true)), 0),
        "north-pinger": minion(json!({
            "defense": 10,
            "genesisDamageEachOtherUnitHere": 1,
            "summonToAnySite": true,
        })),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-buff": minion(json!({
            "burrowing": true,
            "deathriteDrawSite": true,
            "otherNearbyAlliesPowerBonus": 1,
        })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-bury",
        "north-bury",
        "north-pinger",
        "north-bury",
        "north-bury",
        "north-pinger",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-buff"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("ordered Bury candidate");
            let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand")
                .clone();
            hand.iter().any(|card| card["cardId"] == "north-bury")
                && hand.iter().any(|card| card["cardId"] == "north-pinger")
        })
        .expect("bounded seed with Bury and a pinger");
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut buff_ids = Vec::new();
    for _ in 0..2 {
        let (summoned, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["region"].is_null()
                && descriptor["cardId"] == "south-buff"
                && descriptor["cell"] == "C1"
        });
        buff_ids.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("buff identity")
                .to_owned(),
        );
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-pinger"
            && descriptor["cell"] == "C1"
    });
    assert!(buff_ids.iter().all(|instance_id| {
        realm_unit(&state(&session), instance_id).is_some_and(|unit| unit["damage"] == 1)
    }));
    let spell_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();

    let cast = cast_bury(&mut session, &buff_ids[0], &spell_id);
    assert_eq!(event_types(&cast), ["magic-cast", "minion-burrowed"]);
    let pending = state(&session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(pending["decisionSeat"], "south");
    assert_eq!(
        pending["pendingDeathrites"]["deferredOutcomes"],
        json!([{
            "payload": {
                "cardId": "north-bury",
                "instanceId": spell_id,
                "owner": "north",
            },
            "type": "magic-resolved",
        }])
    );

    let checkpoint = create_game_checkpoint(&session).expect("ordered Bury checkpoint");
    let restored = resume_game_checkpoint(&checkpoint).expect("restored ordered Bury checkpoint");
    assert_eq!(
        restored.replay_value().expect("restored pending state"),
        session.replay_value().expect("source pending state")
    );
    let (_, ordered) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == buff_ids[0]
    });
    assert_eq!(
        event_types(&ordered),
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(state(&session)["phase"], "main");
    assert_exact_replay(&session);
}

#[test]
fn disable_magic_should_kill_its_underground_burrowing_target() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-bury": magic(("burrowTargetMinionOrArtifact", json!(true)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-freeze": magic(("disableTargetNearbyMinionUntilNextTurn", json!(true)), 0),
        "south-minion": minion(json!({
            "burrowing": true,
            "spellcaster": true,
        })),
        "south-site": site(false),
    });
    let south_spellbook = [
        "south-freeze",
        "south-minion",
        "south-freeze",
        "south-minion",
        "south-freeze",
        "south-minion",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &["north-bury"; 6], &south_spellbook))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Disable candidate");
            let hand = state(&preview)["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("South opening hand")
                .clone();
            hand.iter().any(|card| card["cardId"] == "south-freeze")
                && hand.iter().any(|card| card["cardId"] == "south-minion")
        })
        .expect("bounded seed with Freeze and a Burrowing spellcaster");
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Burrowing spellcaster identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let bury_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();
    cast_bury(&mut session, &target_id, &bury_id);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let freeze_id = state(&session)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South hand")
        .iter()
        .find(|card| card["cardId"] == "south-freeze")
        .expect("Freeze in hand")["instanceId"]
        .as_str()
        .expect("Freeze identity")
        .to_owned();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == freeze_id
            && descriptor["casterInstanceId"] == target_id
            && descriptor["target"]["instanceId"] == target_id
    });

    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-disabled",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert_exact_replay(&session);
}

fn magic_target_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("targeted Magic actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-magic"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof keeps region confinement and Stealth exclusion in one scenario"
)]
fn rule_catalog_0023_magic_targets_should_stay_in_the_caster_region_and_exclude_enemy_stealth() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-bury": magic(("burrowTargetMinionOrArtifact", json!(true)), 0),
        "north-magic": magic(("damageTargetUnit", json!(1)), 0),
        "north-minion": minion(json!({ "burrowing": true, "stealth": true })),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-plain": minion(json!({})),
        "south-site": site(false),
        "south-stealth": minion(json!({ "stealth": true })),
    });
    let north_spellbook = [
        "north-bury",
        "north-bury",
        "north-magic",
        "north-magic",
        "north-minion",
        "north-minion",
    ];
    let manifest = (1..=512)
        .map(|seed| {
            manifest(
                seed,
                &cards,
                &north_spellbook,
                &[
                    "south-plain",
                    "south-plain",
                    "south-plain",
                    "south-stealth",
                    "south-stealth",
                    "south-stealth",
                ],
            )
        })
        .find(|candidate| {
            let preview = Session::new(candidate).expect("target visibility candidate");
            let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand")
                .clone();
            ["north-bury", "north-magic", "north-minion"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("bounded seed with Bury, targeted Magic, and friendly Stealth");
    let mut session = opening_main(&manifest);
    let (friendly, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["region"].is_null()
    });
    let friendly_id = friendly["cardInstanceId"]
        .as_str()
        .expect("friendly Stealth identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (stealth, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-stealth"
            && descriptor["region"].is_null()
    });
    let stealth_id = stealth["cardInstanceId"]
        .as_str()
        .expect("enemy Stealth identity")
        .to_owned();
    let (plain, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-plain"
            && descriptor["region"].is_null()
    });
    let plain_id = plain["cardInstanceId"]
        .as_str()
        .expect("plain enemy identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let surface = magic_target_ids(&session);
    assert!(
        surface.contains(&friendly_id),
        "own Stealth stays targetable"
    );
    assert!(
        surface.contains(&plain_id),
        "an exposed enemy stays targetable"
    );
    assert!(
        !surface.contains(&stealth_id),
        "enemy active Stealth must never be offered"
    );

    let bury = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();
    let burrowed = cast_bury(&mut session, &friendly_id, &bury);
    assert_eq!(
        event_types(&burrowed),
        ["magic-cast", "minion-burrowed", "magic-resolved"]
    );
    assert_eq!(
        realm_unit(&state(&session), &friendly_id).expect("burrowed ally")["region"],
        "underground"
    );

    let confined = magic_target_ids(&session);
    assert!(
        !confined.contains(&friendly_id),
        "a surface caster may not reach its own burrowed ally"
    );
    assert!(
        confined.contains(&plain_id),
        "the surface enemy stays reachable from the surface"
    );
    assert_exact_replay(&session);
}

#[test]
fn targeted_magic_breaks_ward_instead_of_damaging_the_minion() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-magic": magic(("damageTargetUnit", json!(1)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({ "defense": 1, "ward": true })),
        "south-site": site(false),
    });
    let manifest = manifest(149, &cards, &["north-magic"; 6], &["south-minion"; 6]);
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let target_id = summoned["cardInstanceId"]
        .as_str()
        .expect("warded target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "ward-broken",
            "magic-resolved",
        ]
    );
    let target = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == target_id)
        .expect("Ward survivor")
        .clone();
    assert_eq!(target["damage"], 0);
    assert_eq!(target["warded"], false);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the direct Lash proof retains nearby legality and both survivor and death branches"
)]
fn rule_catalog_0024_lash_damages_then_untaps_only_a_surviving_nearby_minion() {
    let mut lash = magic(("damageTargetUnit", json!(1)), 0);
    lash["targetNearby"] = json!(true);
    lash["untapTargetMinionAfterDamage"] = json!(true);
    let cards = json!({
        "north-avatar": avatar(20),
        "north-lash": lash,
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({
            "defense": 2,
            "summonToAnySite": true,
            "tapForMana": 1,
        })),
        "south-site": site(false),
    });
    let manifest = manifest(230, &cards, &["north-lash"; 6], &["south-minion"; 8]);
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (nearby, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let nearby_id = nearby["cardInstanceId"]
        .as_str()
        .expect("nearby target identity")
        .to_owned();
    let (distant, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let distant_id = distant["cardInstanceId"]
        .as_str()
        .expect("distant target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == nearby_id
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let lash_actions: Vec<_> = session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-lash"
        })
        .collect();
    assert!(!lash_actions.is_empty());
    assert!(lash_actions.iter().all(|action| {
        action.descriptor["target"]["kind"] == "minion"
            && action.descriptor["target"]["instanceId"] == nearby_id
            && action.descriptor["target"]["instanceId"] != distant_id
    }));

    let (_, survived) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&survived),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-untapped",
            "magic-resolved",
        ]
    );
    let survivor = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == nearby_id)
        .expect("surviving target")
        .clone();
    assert_eq!(survivor["damage"], 1);
    assert_eq!(survivor["tapped"], false);

    let (_, died) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["target"]["instanceId"] == nearby_id
    });
    assert_eq!(
        event_types(&died),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert!(
        !state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == nearby_id)
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the direct proof retains token creation, combat, banishment, and replay"
)]
fn token_magic_should_summon_in_cell_order_and_banish_a_dead_token() {
    let token_id = "foot-soldier-token";
    let cards = json!({
        "north-avatar": avatar(20),
        "north-magic": magic(("summonTokenToEachControlledSiteBorderingEnemySite", json!(token_id)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({})),
        "south-site": site(false),
        token_id: minion(json!({ "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 }, "token": true })),
    });
    let manifest = manifest(218, &cards, &["north-magic"; 6], &["south-minion"; 6]);
    let mut session = opening_main(&manifest);
    let (_, empty) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor.get("cemeteryMinionInstanceId").is_none()
    });
    assert_eq!(event_types(&empty), ["magic-cast", "magic-resolved"]);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );

    for (draw_zone, site_cell) in [
        (Some("atlas"), Some("C1")),
        (Some("atlas"), Some("C3")),
        (Some("atlas"), Some("C2")),
        (Some("atlas"), Some("B3")),
        (Some("atlas"), Some("B2")),
    ] {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        if let Some(zone) = draw_zone {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == zone
            });
        }
        if let Some(cell) = site_cell {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == cell
            });
        }
    }
    let (summoned_attacker, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "B2"
    });
    let attacker_id = summoned_attacker["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let pre_cast_version = state(&session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, summoned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
    });
    assert_eq!(
        event_types(&summoned),
        [
            "magic-cast",
            "minion-summoned",
            "minion-summoned",
            "magic-resolved",
        ]
    );
    assert_eq!(
        summoned.events[1..3]
            .iter()
            .map(|event| event.payload["cell"].as_str().expect("token cell"))
            .collect::<Vec<_>>(),
        ["B3", "C3"]
    );
    assert!(summoned.random_draws.is_empty());
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    assert!(
        summoned.events[1..3]
            .iter()
            .all(|event| event.payload["sourceInstanceId"] == source_id)
    );
    assert_eq!(
        summoned.events[1..3]
            .iter()
            .map(|event| event.payload["instanceId"]
                .as_str()
                .expect("token identity"))
            .collect::<Vec<_>>(),
        ["B3", "C3"]
            .into_iter()
            .enumerate()
            .map(|(ordinal, cell)| {
                identity_hash(&json!({
                    "cardId": token_id,
                    "cell": cell,
                    "ordinal": ordinal,
                    "owner": "north",
                    "source": "token",
                    "sourceInstanceId": source_id,
                    "stateVersion": pre_cast_version,
                }))
                .expect("expected token identity")
                .to_string()
            })
            .collect::<Vec<_>>()
    );
    let killed_id = summoned.events[1].payload["instanceId"]
        .as_str()
        .expect("token identity")
        .to_owned();

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "B3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == killed_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let token_exits: Vec<_> = fight
        .events
        .iter()
        .filter(|event| event.payload["instanceId"] == killed_id)
        .map(|event| event.event_type.as_str())
        .filter(|event_type| matches!(*event_type, "minion-died" | "minion-banished"))
        .collect();
    assert_eq!(token_exits, ["minion-died", "minion-banished"]);
    assert!(
        state(&session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != killed_id)
    );
    assert_exact_replay(&session);
}

fn rescue_cards(discard: bool) -> Value {
    json!({
        "north-avatar": avatar(20),
        "north-minion": minion(json!({})),
        "north-rescue": magic(("returnMinionFromOwnCemetery", json!(true)), 0),
        "north-site": site(discard),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({})),
        "south-rescue": magic(("returnMinionFromOwnCemetery", json!(true)), 0),
        "south-site": site(discard),
    })
}

fn rescue_checkpoint() -> Session {
    for seed in 1..=200 {
        let manifest = manifest(
            seed,
            &rescue_cards(true),
            &[
                "north-minion",
                "north-minion",
                "north-minion",
                "north-rescue",
                "north-rescue",
                "north-rescue",
            ],
            &[
                "south-minion",
                "south-minion",
                "south-minion",
                "south-rescue",
                "south-rescue",
                "south-rescue",
            ],
        );
        let mut session = Session::new(&manifest).expect("valid Rescue candidate");
        keep(&mut session);
        keep(&mut session);
        let value = state(&session);
        let exact_top_pair = |seat: &str| {
            let mut types = value["players"][seat]["spellbook"]
                .as_array()
                .expect("spellbook")
                .iter()
                .take(2)
                .map(|card| {
                    card["cardId"]
                        .as_str()
                        .expect("card identity")
                        .contains("minion")
                })
                .collect::<Vec<_>>();
            types.sort_unstable();
            types == [false, true]
        };
        let north_has_rescue = value["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand")
            .iter()
            .any(|card| card["cardId"] == "north-rescue");
        if exact_top_pair("north") && exact_top_pair("south") && north_has_rescue {
            return session;
        }
    }
    panic!("bounded seeds should contain a deterministic Rescue fixture");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the direct proof retains empty, own, opposing, hidden-hand, and replay branches"
)]
fn rescue_should_issue_only_own_minion_choices_and_keep_the_return_hidden() {
    let no_choice_manifest = manifest(
        11,
        &rescue_cards(false),
        &[
            "north-minion",
            "north-minion",
            "north-minion",
            "north-rescue",
            "north-rescue",
            "north-rescue",
        ],
        &[
            "south-minion",
            "south-minion",
            "south-minion",
            "south-rescue",
            "south-rescue",
            "south-rescue",
        ],
    );
    let mut no_choice = opening_main(&no_choice_manifest);
    let actions = no_choice.legal_actions().expect("targetless Rescue action");
    let cast = actions
        .iter()
        .find(|action| action.descriptor["kind"] == "cast-magic")
        .expect("targetless Rescue");
    assert!(cast.descriptor.get("cemeteryMinionInstanceId").is_none());
    let (_, resolved) = accept_where(&mut no_choice, |descriptor| descriptor == &cast.descriptor);
    assert_eq!(event_types(&resolved), ["magic-cast", "magic-resolved"]);

    let mut session = rescue_checkpoint();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let before = state(&session);
    let own_minion = before["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .iter()
        .find(|card| card["cardId"] == "north-minion")
        .expect("own dead minion")["instanceId"]
        .as_str()
        .expect("own minion identity")
        .to_owned();
    let opposing_minion = before["players"]["south"]["cemetery"]
        .as_array()
        .expect("south cemetery")
        .iter()
        .find(|card| card["cardId"] == "south-minion")
        .expect("opposing dead minion")["instanceId"]
        .as_str()
        .expect("opposing minion identity")
        .to_owned();
    let rescue_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .find(|card| card["cardId"] == "north-rescue")
        .expect("Rescue in hand")["instanceId"]
        .as_str()
        .expect("Rescue identity")
        .to_owned();
    let choices: Vec<_> = session
        .legal_actions()
        .expect("Rescue choices")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == rescue_id
        })
        .collect();
    assert_eq!(choices.len(), 1);
    assert_eq!(
        choices[0].descriptor["cemeteryMinionInstanceId"],
        own_minion
    );
    assert_ne!(
        choices[0].descriptor["cemeteryMinionInstanceId"],
        opposing_minion
    );
    let before_hand_count = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .len();
    let south_observation = session.observe(Seat::South);
    let (_, returned) = accept_where(&mut session, |descriptor| {
        descriptor == &choices[0].descriptor
    });
    assert_eq!(
        event_types(&returned),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(returned.events[1].payload["instanceId"], own_minion);
    assert_eq!(returned.events[1].payload["sourceInstanceId"], rescue_id);
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
            .any(|card| card["instanceId"] == own_minion)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    assert_exact_replay(&no_choice);
    assert_exact_replay(&session);
}

fn healing_session(life: u8, seed: u32) -> (Session, String) {
    let cards = json!({
        "north-avatar": avatar(life),
        "north-heal": magic(("healController", json!(7)), 1),
        "north-loss": minion(json!({ "genesisLoseControllerLife": 2 })),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({})),
        "south-site": site(false),
    });
    let manifest = manifest(
        seed,
        &cards,
        &[
            "north-heal",
            "north-heal",
            "north-heal",
            "north-loss",
            "north-loss",
            "north-loss",
        ],
        &["south-minion"; 6],
    );
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-loss"
            && descriptor["region"].is_null()
    });
    let heal = session
        .legal_actions()
        .expect("healing action")
        .into_iter()
        .find(|action| action.descriptor["kind"] == "cast-magic")
        .expect("targetless healing Magic");
    assert!(heal.descriptor.get("cemeteryMinionInstanceId").is_none());
    let spell_id = heal.descriptor["cardInstanceId"]
        .as_str()
        .expect("healing Magic identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor == &heal.descriptor);
    (session, spell_id)
}

#[test]
fn healing_magic_should_cap_and_not_leave_deaths_door() {
    let (capped, spell_id) = healing_session(20, 151);
    let capped_state = state(&capped);
    assert_eq!(capped_state["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(capped_state["players"]["north"]["mana"], 0);
    assert!(
        capped_state["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    let receipt = capped.transcript().last().expect("healing receipt");
    assert_eq!(
        event_types(receipt),
        ["magic-cast", "avatar-healed", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["amount"], 2);
    assert_eq!(receipt.events[1].payload["attemptedAmount"], 7);

    let (death_door, _) = healing_session(2, 152);
    let death_door_state = state(&death_door);
    assert_eq!(death_door_state["players"]["north"]["avatar"]["life"], 0);
    assert_eq!(death_door_state["terminal"]["status"], "active");
    assert_eq!(
        event_types(
            death_door
                .transcript()
                .last()
                .expect("Death's Door receipt")
        ),
        ["magic-cast", "magic-resolved"]
    );
    assert_exact_replay(&capped);
    assert_exact_replay(&death_door);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the direct proof retains sick, tapped, disabled, Magic, summon, and replay branches"
)]
fn printed_spellcaster_should_cast_and_summon_while_sick_or_tapped() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-caster": minion(json!({
            "spellcaster": true,
            "stealth": true,
            "tapForMana": 1,
        })),
        "north-freeze": magic(("disableTargetNearbyMinionUntilNextTurn", json!(true)), 0),
        "north-site": site(false),
        "north-disabled-caster": minion(json!({
            "spellcaster": true,
            "waterbound": true,
        })),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({ "tapForMana": 1 })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-caster",
        "north-caster",
        "north-freeze",
        "north-freeze",
        "north-disabled-caster",
        "north-disabled-caster",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-minion"; 8]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate session");
            let state = state(&preview);
            let hand = state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("north opening Spellbook hand");
            ["north-caster", "north-disabled-caster", "north-freeze"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("seed with caster, Magic, and minion in the opening hand");
    let mut session = opening_main(&manifest);
    let (caster_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    });
    let caster_instance_id = caster_summon["cardInstanceId"]
        .as_str()
        .expect("caster instance identity")
        .to_owned();
    let checkpoint = session.clone();

    let sick_magic = checkpoint
        .legal_actions()
        .expect("sick caster actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-freeze"
                && action.descriptor["casterInstanceId"] == caster_instance_id
                && action.descriptor["target"]["instanceId"] == caster_instance_id
        })
        .expect("summoning-sick printed Spellcaster Magic action");
    assert_eq!(
        sick_magic.label,
        format!(
            "Cast north-freeze on minion {}… with minion {}…",
            &caster_instance_id[..15],
            &caster_instance_id[..15]
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

    let mut sick_summon = checkpoint.clone();
    let (summon_descriptor, summon_receipt) = accept_where(&mut sick_summon, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-disabled-caster"
            && descriptor["casterInstanceId"] == caster_instance_id
            && descriptor["cell"] == "C4"
    });
    assert_eq!(summon_descriptor["casterInstanceId"], caster_instance_id);
    assert_eq!(
        event_types(&summon_receipt),
        ["stealth-lost", "minion-summoned"]
    );
    let disabled_caster_id = summon_descriptor["cardInstanceId"]
        .as_str()
        .expect("disabled caster instance identity");
    assert!(
        !sick_summon
            .legal_actions()
            .expect("actions after disabled caster summon")
            .iter()
            .any(|action| {
                action.descriptor["casterInstanceId"] == disabled_caster_id
                    && matches!(
                        action.descriptor["kind"].as_str(),
                        Some("cast-magic" | "summon-minion")
                    )
            })
    );
    assert_exact_replay(&sick_summon);

    let mut tapped_cast = checkpoint;
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == caster_instance_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B2"
    });
    let (target_summon, _) = accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "B2"
    });
    let target_instance_id = target_summon["cardInstanceId"]
        .as_str()
        .expect("nearby target identity")
        .to_owned();
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == caster_instance_id
    });
    assert_eq!(
        state(&tapped_cast)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == caster_instance_id)
            .expect("tapped caster")["tapped"],
        true
    );
    let actions = tapped_cast.legal_actions().expect("tapped caster actions");
    let avatar_instance_id =
        state(&tapped_cast)["players"]["north"]["avatar"]["card"]["instanceId"]
            .as_str()
            .expect("Avatar identity")
            .to_owned();
    assert!(actions.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic"
            && action.descriptor["casterInstanceId"] == caster_instance_id
            && action.descriptor["target"]["instanceId"] == target_instance_id
    }));
    assert!(!actions.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic"
            && action.descriptor["casterInstanceId"] == avatar_instance_id
            && action.descriptor["target"]["instanceId"] == target_instance_id
    }));
    let (_, tapped_receipt) = accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["casterInstanceId"] == caster_instance_id
            && descriptor["target"]["instanceId"] == target_instance_id
    });
    assert_eq!(
        event_types(&tapped_receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert_eq!(
        state(&tapped_cast)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == target_instance_id)
            .expect("disabled target")["disableEffects"][0]["sourceInstanceId"],
        tapped_receipt.events[0].payload["instanceId"]
    );
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    assert!(
        !tapped_cast
            .legal_actions()
            .expect("disabled target actions")
            .iter()
            .any(|action| action.descriptor["unitInstanceId"] == target_instance_id)
    );
    let (_, expiration) = accept_where(&mut tapped_cast, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    assert_eq!(
        event_types(&expiration),
        ["turn-ended", "minion-disable-expired", "turn-started"]
    );
    assert!(
        state(&tapped_cast)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == target_instance_id)
            .expect("expired target")["disableEffects"]
            .is_null()
    );
    assert_exact_replay(&tapped_cast);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct catalog proof retains Freeze legality, suppression, expiry, checkpoint, and replay"
)]
fn freeze_should_disable_nearby_minion_until_caster_next_start_phase() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-freeze": magic(("disableTargetNearbyMinionUntilNextTurn", json!(true)), 0),
        "north-site": site(false),
        "south-air-magic": {
            "cardType": "magic",
            "healController": 1,
            "manaCost": 0,
            "thresholds": { "air": 1, "earth": 0, "fire": 0, "water": 0 },
        },
        "south-avatar": avatar(20),
        "south-far": minion(json!({})),
        "south-site": site(false),
        "south-target": minion(json!({
            "movementBonus": 1,
            "provides": "air",
            "tapForMana": 1,
        })),
    });
    let south_spellbook = [
        "south-far",
        "south-target",
        "south-far",
        "south-air-magic",
        "south-target",
        "south-far",
        "south-air-magic",
        "south-target",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &["north-freeze"; 8], &south_spellbook))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate Freeze session");
            let hand = state(&preview)["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("south opening Spellbook hand")
                .clone();
            ["south-air-magic", "south-far", "south-target"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("seed with both South minions in the opening hand");
    let mut session = opening_main(&manifest);

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (target_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let avatar_instance_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == avatar_instance_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });

    let far_instance_id = far_summon["cardInstanceId"]
        .as_str()
        .expect("far minion identity");
    let target_instance_id = target_summon["cardInstanceId"]
        .as_str()
        .expect("Freeze target identity");
    let actions = session.legal_actions().expect("North Freeze actions");
    let freeze = actions
        .iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-freeze"
                && action.descriptor["target"]["instanceId"] == target_instance_id
        })
        .expect("nearby target Freeze action")
        .clone();
    assert!(!actions.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic"
            && action.descriptor["target"]["instanceId"] == far_instance_id
    }));
    assert_eq!(freeze.descriptor["casterInstanceId"], avatar_instance_id);
    assert_eq!(
        freeze.label,
        format!("Cast north-freeze on minion {}…", &target_instance_id[..15])
    );

    let mut enabled_branch = session.clone();
    accept_where(&mut enabled_branch, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut enabled_branch, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let enabled_actions = enabled_branch
        .legal_actions()
        .expect("enabled target actions");
    assert!(enabled_actions.iter().any(|action| {
        action.descriptor["kind"] == "move-and-attack"
            && action.descriptor["unitInstanceId"] == target_instance_id
    }));
    assert!(enabled_actions.iter().any(|action| {
        action.descriptor["kind"] == "activate-mana"
            && action.descriptor["unitInstanceId"] == target_instance_id
    }));
    assert!(enabled_actions.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic"
            && action.descriptor["cardId"] == "south-air-magic"
    }));

    let cast_state_version = freeze.state_version;
    let freeze_source_id = freeze.descriptor["cardInstanceId"]
        .as_str()
        .expect("Freeze source identity")
        .to_owned();
    let StepResult::Accepted(cast_receipt) = session
        .step(ActionRequest {
            action_id: freeze.action_id.to_string(),
            seat: freeze.seat,
            state_version: freeze.state_version,
        })
        .expect("authoritative Freeze step")
    else {
        panic!("engine-issued Freeze action must be accepted");
    };
    assert_eq!(cast_receipt.state_version, cast_state_version);
    assert_eq!(cast_receipt.next_state_version, cast_state_version + 1);
    assert_eq!(
        state(&session)["stateVersion"],
        cast_receipt.next_state_version
    );
    assert_eq!(
        event_types(&cast_receipt),
        ["magic-cast", "minion-disabled", "magic-resolved"]
    );
    assert_eq!(
        cast_receipt.events[1].payload,
        json!({
            "expiresAtSeat": "north",
            "instanceId": target_instance_id,
            "seat": "south",
            "sourceInstanceId": freeze_source_id,
            "stealthRemoved": false,
            "wardRemoved": false,
        })
    );

    let (_, south_started) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(event_types(&south_started), ["turn-ended", "turn-started"]);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let disabled_actions = session.legal_actions().expect("disabled target actions");
    assert!(
        !disabled_actions
            .iter()
            .any(|action| { action.descriptor["unitInstanceId"] == target_instance_id })
    );
    assert!(!disabled_actions.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic"
            && action.descriptor["cardId"] == "south-air-magic"
    }));
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == target_instance_id)
            .expect("disabled target")["disableEffects"],
        json!([{
            "expiresAtSeat": "north",
            "sourceInstanceId": freeze_source_id,
        }])
    );

    let checkpoint = create_game_checkpoint(&session).expect("captured Freeze checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized Freeze checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed Freeze checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored Freeze checkpoint");
    assert_eq!(
        restored.replay_value().expect("restored Freeze state"),
        session.replay_value().expect("source Freeze state")
    );
    assert_eq!(
        restored.legal_actions().expect("restored Freeze actions"),
        disabled_actions
    );

    let (_, expiration) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_types(&expiration),
        ["turn-ended", "minion-disable-expired", "turn-started"]
    );
    assert_eq!(
        expiration.events[1].payload,
        json!({
            "instanceId": target_instance_id,
            "seat": "south",
            "sourceInstanceId": freeze_source_id,
        })
    );
    let expired = state(&session);
    assert!(
        expired["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == target_instance_id)
            .expect("expired target")["disableEffects"]
            .is_null()
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct Charge proof retains ally legality, stacking, expiry, and replay"
)]
fn rule_catalog_0032_charge_magic_grants_an_ally_charge_only_for_the_current_turn() {
    let cards = json!({
        "north-ally": minion(json!({})),
        "north-avatar": avatar(20),
        "north-charge": magic(("grantChargeToAllyThisTurn", json!(true)), 1),
        "north-filler": minion(json!({})),
        "north-printed": minion(json!({
            "charge": true,
            "stealth": true,
            "ward": true,
        })),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-enemy": minion(json!({
            "stealth": true,
            "ward": true,
        })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-printed",
        "north-charge",
        "north-charge",
        "north-ally",
        "north-filler",
        "north-filler",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-enemy"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate Charge session");
            let preview = state(&preview);
            let hand = preview["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening Spellbook hand");
            hand.iter()
                .filter(|card| card["cardId"] == "north-charge")
                .count()
                == 2
                && hand.iter().any(|card| card["cardId"] == "north-printed")
                && preview["players"]["north"]["spellbook"][0]["cardId"] == "north-ally"
        })
        .expect("seed with printed Charge, two Charge Magics, and the ally next");
    let mut session = opening_main(&manifest);

    let (printed_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-printed"
            && descriptor["cell"] == "C4"
    });
    let printed_id = printed_summon["cardInstanceId"]
        .as_str()
        .expect("printed Charge identity")
        .to_owned();
    let after_printed = state(&session);
    let printed = after_printed["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == printed_id)
        .expect("printed Charge minion");
    assert_eq!(
        (
            printed["region"].as_str(),
            printed["stealthed"].as_bool(),
            printed["warded"].as_bool(),
            printed["summoningSickness"].as_bool(),
        ),
        (Some("surface"), Some(true), Some(true), Some(true))
    );
    assert!(
        session
            .legal_actions()
            .expect("printed Charge actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "move-and-attack"
                    && action.descriptor["unitInstanceId"] == printed_id
            })
    );

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (enemy_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C1"
    });
    let enemy_id = enemy_summon["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let (ally_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    });
    let ally_id = ally_summon["cardInstanceId"]
        .as_str()
        .expect("Charge ally identity")
        .to_owned();

    let checkpoint = session.clone();
    let checkpoint_state = state(&checkpoint);
    let avatar_id = checkpoint_state["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let charge_ids: Vec<_> = checkpoint_state["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook hand")
        .iter()
        .filter(|card| card["cardId"] == "north-charge")
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("Charge Magic identity")
                .to_owned()
        })
        .collect();
    assert_eq!(charge_ids.len(), 2);

    let checkpoint_actions = checkpoint.legal_actions().expect("Charge Magic actions");
    let canonical_actions: Vec<_> = checkpoint_actions
        .iter()
        .map(|action| canonical_json(&action.descriptor).expect("canonical action descriptor"))
        .collect();
    let mut sorted_actions = canonical_actions.clone();
    sorted_actions.sort_unstable();
    assert_eq!(canonical_actions, sorted_actions);
    let casts: Vec<_> = checkpoint_actions
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == charge_ids[0]
        })
        .collect();
    let canonical_casts: Vec<_> = casts
        .iter()
        .map(|action| canonical_json(&action.descriptor).expect("canonical Charge descriptor"))
        .collect();
    let mut sorted_casts = canonical_casts.clone();
    sorted_casts.sort_unstable();
    assert_eq!(canonical_casts, sorted_casts);
    let mut ally_ids: Vec<_> = casts
        .iter()
        .map(|action| {
            assert!(action.descriptor["target"].is_null());
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .expect("engine-issued Charge ally")
                .to_owned()
        })
        .collect();
    ally_ids.sort_unstable();
    let mut expected_ally_ids = vec![avatar_id.clone(), printed_id.clone(), ally_id.clone()];
    expected_ally_ids.sort_unstable();
    assert_eq!(ally_ids, expected_ally_ids);
    assert!(!ally_ids.contains(&enemy_id));
    assert!(casts.iter().any(|action| {
        action.descriptor["ally"]["instanceId"] == ally_id && action.label.contains("grant Charge")
    }));
    assert!(
        !checkpoint
            .legal_actions()
            .expect("pre-Charge actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "move-and-attack"
                    && action.descriptor["unitInstanceId"] == ally_id
            })
    );

    let mut avatar_branch = checkpoint.clone();
    let (_, avatar_grant) = accept_where(&mut avatar_branch, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == charge_ids[0]
            && descriptor["ally"]["kind"] == "avatar"
    });
    assert_eq!(
        event_types(&avatar_grant),
        ["magic-cast", "charge-granted", "magic-resolved"]
    );
    assert_eq!(
        avatar_grant.events[1].payload,
        json!({
            "instanceId": avatar_id,
            "seat": "north",
            "sourceInstanceId": charge_ids[0],
        })
    );
    assert!(
        state(&avatar_branch)["realm"]["units"]
            .as_array()
            .expect("avatar branch units")
            .iter()
            .all(|unit| unit["temporaryChargeSources"].is_null())
    );
    assert!(avatar_grant.random_draws.is_empty());
    assert_exact_replay(&avatar_branch);

    let checkpoint_version = checkpoint_state["stateVersion"]
        .as_u64()
        .expect("checkpoint state version");
    let checkpoint_mana = checkpoint_state["players"]["north"]["mana"]
        .as_u64()
        .expect("checkpoint mana");
    session = checkpoint;
    let (_, first) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == charge_ids[0]
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&first),
        ["magic-cast", "charge-granted", "magic-resolved"]
    );
    assert_eq!(
        first.events[0].payload,
        json!({
            "allyInstanceId": ally_id,
            "allySeat": "north",
            "cardId": "north-charge",
            "casterInstanceId": avatar_id,
            "instanceId": charge_ids[0],
            "manaPaid": 1,
            "seat": "north",
        })
    );
    assert_eq!(
        first.events[1].payload,
        json!({
            "instanceId": ally_id,
            "seat": "north",
            "sourceInstanceId": charge_ids[0],
        })
    );
    assert!(first.random_draws.is_empty());
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == ally_id)
            .expect("charged ally")["temporaryChargeSources"],
        json!([charge_ids[0]])
    );
    assert!(
        session
            .legal_actions()
            .expect("temporarily charged actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "move-and-attack"
                    && action.descriptor["unitInstanceId"] == ally_id
            })
    );
    let charged_state = state(&session);
    let charged_checkpoint = create_game_checkpoint(&session).expect("captured Charge checkpoint");
    let charged_bytes =
        serialize_game_checkpoint(&charged_checkpoint).expect("serialized Charge checkpoint");
    session = resume_game_checkpoint(
        &parse_game_checkpoint(&charged_bytes).expect("parsed Charge checkpoint"),
    )
    .expect("restored Charge checkpoint");
    assert_eq!(state(&session), charged_state);

    let (_, second) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == charge_ids[1]
            && descriptor["ally"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&second),
        ["magic-cast", "charge-granted", "magic-resolved"]
    );
    assert!(second.random_draws.is_empty());
    let stacked = state(&session);
    assert_eq!(
        stacked["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == ally_id)
            .expect("stacked Charge ally")["temporaryChargeSources"],
        json!(charge_ids)
    );
    assert_eq!(stacked["players"]["north"]["mana"], checkpoint_mana - 2);
    assert_eq!(stacked["stateVersion"], checkpoint_version + 2);
    assert_eq!(
        stacked["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .filter(|card| charge_ids.iter().any(|id| card["instanceId"] == *id))
            .count(),
        2
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == ally_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        event_types(&ended),
        [
            "charge-expired",
            "charge-expired",
            "turn-ended",
            "turn-started",
        ]
    );
    assert_eq!(
        ended.events[..2]
            .iter()
            .map(|event| event.payload.clone())
            .collect::<Vec<_>>(),
        charge_ids
            .iter()
            .map(|source_id| json!({
                "instanceId": ally_id,
                "seat": "north",
                "sourceInstanceId": source_id,
            }))
            .collect::<Vec<_>>()
    );
    let expired = state(&session);
    let units = expired["realm"]["units"]
        .as_array()
        .expect("expired realm units");
    let expired_ally = units
        .iter()
        .find(|unit| unit["instanceId"] == ally_id)
        .expect("expired Charge ally");
    assert!(expired_ally["temporaryChargeSources"].is_null());
    assert_eq!(expired_ally["tapped"], true);
    assert!(
        units
            .iter()
            .find(|unit| unit["instanceId"] == printed_id)
            .expect("printed Charge minion")["temporaryChargeSources"]
            .is_null()
    );
    assert_eq!(cards["north-printed"]["charge"], true);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct Overpower proof retains ally legality, combat prevention, stacking, expiry, and replay"
)]
fn rule_catalog_0033_overpower_changes_current_power_until_the_current_end_phase() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-fighter": minion(json!({
            "attack": 2,
            "defense": 2,
        })),
        "north-filler": minion(json!({})),
        "north-overpower": magic(("grantPowerToAllyThisTurn", json!(2)), 1),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-enemy": minion(json!({
            "attack": 2,
            "defense": 2,
            "preventsDamageFromUnitsWithPowerAtLeast": 4,
            "summonToAnySite": true,
        })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-overpower",
        "north-overpower",
        "north-fighter",
        "north-filler",
        "north-filler",
        "north-filler",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-enemy"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("candidate Overpower session");
            let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening Spellbook hand")
                .clone();
            hand.iter()
                .filter(|card| card["cardId"] == "north-overpower")
                .count()
                == 2
                && hand.iter().any(|card| card["cardId"] == "north-fighter")
        })
        .expect("seed with the fighter and two Overpower Magics");
    let mut session = opening_main(&manifest);

    let (fighter_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-fighter"
            && descriptor["cell"] == "C4"
    });
    let fighter_id = fighter_summon["cardInstanceId"]
        .as_str()
        .expect("fighter identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (enemy_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C4"
    });
    let enemy_id = enemy_summon["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });

    let checkpoint = session.clone();
    let checkpoint_state = state(&checkpoint);
    let avatar_id = checkpoint_state["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let overpower_ids: Vec<_> = checkpoint_state["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook hand")
        .iter()
        .filter(|card| card["cardId"] == "north-overpower")
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("Overpower identity")
                .to_owned()
        })
        .collect();
    assert_eq!(overpower_ids.len(), 2);
    let checkpoint_version = checkpoint_state["stateVersion"]
        .as_u64()
        .expect("checkpoint state version");
    let checkpoint_mana = checkpoint_state["players"]["north"]["mana"]
        .as_u64()
        .expect("checkpoint mana");

    let checkpoint_actions = checkpoint.legal_actions().expect("Overpower actions");
    let canonical_actions: Vec<_> = checkpoint_actions
        .iter()
        .map(|action| canonical_json(&action.descriptor).expect("canonical action descriptor"))
        .collect();
    let mut sorted_actions = canonical_actions.clone();
    sorted_actions.sort_unstable();
    assert_eq!(canonical_actions, sorted_actions);
    let casts: Vec<_> = checkpoint_actions
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == overpower_ids[0]
        })
        .collect();
    let mut ally_ids: Vec<_> = casts
        .iter()
        .map(|action| {
            assert!(action.descriptor["target"].is_null());
            assert!(action.descriptor["targetLocation"].is_null());
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .expect("engine-issued Overpower ally")
                .to_owned()
        })
        .collect();
    ally_ids.sort_unstable();
    let mut expected_ally_ids = vec![avatar_id.clone(), fighter_id.clone()];
    expected_ally_ids.sort_unstable();
    assert_eq!(ally_ids, expected_ally_ids);
    assert!(!ally_ids.contains(&enemy_id));
    assert_eq!(
        casts
            .iter()
            .find(|action| action.descriptor["ally"]["instanceId"] == fighter_id)
            .expect("fighter Overpower action")
            .label,
        format!(
            "Cast north-overpower to grant +2 power to minion {}…",
            &fighter_id[..15]
        )
    );

    let mut avatar_branch = checkpoint.clone();
    let (_, avatar_grant) = accept_where(&mut avatar_branch, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == overpower_ids[0]
            && descriptor["ally"]["kind"] == "avatar"
    });
    assert_eq!(
        event_types(&avatar_grant),
        ["magic-cast", "power-granted", "magic-resolved"]
    );
    assert_eq!(
        avatar_grant.events[1].payload,
        json!({
            "amount": 2,
            "instanceId": avatar_id,
            "seat": "north",
            "sourceInstanceId": overpower_ids[0],
        })
    );
    assert_eq!(
        state(&avatar_branch)["players"]["north"]["avatar"]["temporaryPowerSources"],
        json!([overpower_ids[0]])
    );
    assert!(avatar_grant.random_draws.is_empty());
    let (_, avatar_ended) = accept_where(&mut avatar_branch, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    let avatar_expiry = avatar_ended
        .events
        .iter()
        .position(|event| event.event_type == "power-expired")
        .expect("Avatar power expiry");
    let avatar_turn_ended = avatar_ended
        .events
        .iter()
        .position(|event| event.event_type == "turn-ended")
        .expect("Avatar turn ended");
    assert!(avatar_expiry < avatar_turn_ended);
    assert_eq!(
        avatar_ended.events[avatar_expiry].payload,
        json!({
            "amount": 2,
            "instanceId": avatar_id,
            "seat": "north",
            "sourceInstanceId": overpower_ids[0],
        })
    );
    assert!(state(&avatar_branch)["players"]["north"]["avatar"]["temporaryPowerSources"].is_null());
    assert_exact_replay(&avatar_branch);

    session = checkpoint;
    let (_, first) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == overpower_ids[0]
            && descriptor["ally"]["instanceId"] == fighter_id
    });
    assert_eq!(
        event_types(&first),
        ["magic-cast", "power-granted", "magic-resolved"]
    );
    assert_eq!(
        first.events[0].payload,
        json!({
            "allyInstanceId": fighter_id,
            "allySeat": "north",
            "cardId": "north-overpower",
            "casterInstanceId": avatar_id,
            "instanceId": overpower_ids[0],
            "manaPaid": 1,
            "seat": "north",
        })
    );
    assert_eq!(
        first.events[1].payload,
        json!({
            "amount": 2,
            "instanceId": fighter_id,
            "seat": "north",
            "sourceInstanceId": overpower_ids[0],
        })
    );
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == fighter_id)
            .expect("powered fighter")["temporaryPowerSources"],
        json!([overpower_ids[0]])
    );
    assert!(first.random_draws.is_empty());

    let powered_state = state(&session);
    let powered_actions = session.legal_actions().expect("powered legal actions");
    let powered_checkpoint = create_game_checkpoint(&session).expect("captured power checkpoint");
    let powered_bytes =
        serialize_game_checkpoint(&powered_checkpoint).expect("serialized power checkpoint");
    session = resume_game_checkpoint(
        &parse_game_checkpoint(&powered_bytes).expect("parsed power checkpoint"),
    )
    .expect("restored power checkpoint");
    assert_eq!(state(&session), powered_state);
    assert_eq!(
        session.legal_actions().expect("restored powered actions"),
        powered_actions
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == fighter_id
            && descriptor["path"]
                .as_array()
                .is_some_and(|path| path.len() == 1)
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == enemy_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    let prevented = fight
        .events
        .iter()
        .find(|event| event.event_type == "damage-dealt" && event.payload["instanceId"] == enemy_id)
        .expect("source-aware prevention event");
    assert_eq!(
        prevented.payload,
        json!({
            "accumulated": 0,
            "amount": 0,
            "attemptedAmount": 4,
            "direct": true,
            "instanceId": enemy_id,
            "prevented": true,
            "seat": "south",
        })
    );
    assert_eq!(
        fight
            .events
            .iter()
            .find(|event| event.event_type == "strike-damage-allocated")
            .expect("powered strike allocation")
            .payload["amount"],
        4
    );
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .expect("post-fight units")
            .iter()
            .find(|unit| unit["instanceId"] == fighter_id)
            .expect("powered fighter survived")["damage"],
        2
    );
    assert!(fight.random_draws.is_empty());

    let (_, second) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == overpower_ids[1]
            && descriptor["ally"]["instanceId"] == fighter_id
    });
    assert_eq!(
        event_types(&second),
        ["magic-cast", "power-granted", "magic-resolved"]
    );
    assert!(second.random_draws.is_empty());
    let stacked = state(&session);
    assert_eq!(
        stacked["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == fighter_id)
            .expect("stacked fighter")["temporaryPowerSources"],
        json!(overpower_ids)
    );
    assert_eq!(stacked["players"]["north"]["mana"], checkpoint_mana - 2);
    assert_eq!(stacked["stateVersion"], checkpoint_version + 5);
    assert_eq!(
        stacked["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .filter(|card| overpower_ids.iter().any(|id| card["instanceId"] == *id))
            .count(),
        2
    );

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let expiry: Vec<_> = ended
        .events
        .iter()
        .filter(|event| event.event_type == "power-expired")
        .map(|event| event.payload.clone())
        .collect();
    assert_eq!(
        expiry,
        overpower_ids
            .iter()
            .map(|source_id| json!({
                "amount": 2,
                "instanceId": fighter_id,
                "seat": "north",
                "sourceInstanceId": source_id,
            }))
            .collect::<Vec<_>>()
    );
    let expiry_end = ended
        .events
        .iter()
        .rposition(|event| event.event_type == "power-expired")
        .expect("power expiry");
    let turn_ended = ended
        .events
        .iter()
        .position(|event| event.event_type == "turn-ended")
        .expect("turn ended");
    assert!(expiry_end < turn_ended);
    assert!(ended.random_draws.is_empty());
    let expired = state(&session);
    let expired_fighter = expired["realm"]["units"]
        .as_array()
        .expect("expired units")
        .iter()
        .find(|unit| unit["instanceId"] == fighter_id)
        .expect("fighter survived expiry");
    assert!(expired_fighter["temporaryPowerSources"].is_null());
    assert_eq!(expired_fighter["damage"], 0);
    assert!(
        expired["realm"]["units"]
            .as_array()
            .expect("expired units")
            .iter()
            .any(|unit| unit["instanceId"] == enemy_id)
    );
    assert_exact_replay(&session);
}

struct DuelSetup {
    ally_id: String,
    caster_id: String,
    diagonal_id: String,
    disabled_id: String,
    normal_id: String,
    session: Session,
    stealthed_id: String,
    warded_id: String,
    wrong_region_id: String,
}

fn summon_duel_minion(
    session: &mut Session,
    card_id: &str,
    cell: &str,
    region: Option<&str>,
) -> String {
    let (descriptor, _) = accept_where(session, |candidate| {
        candidate["kind"] == "summon-minion"
            && candidate["cardId"] == card_id
            && candidate["cell"] == cell
            && region.map_or_else(
                || candidate["region"].is_null(),
                |expected| candidate["region"] == expected,
            )
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned Duel minion identity")
        .to_owned()
}

#[expect(
    clippy::too_many_lines,
    reason = "one staged checkpoint keeps Duel geometry and branch comparisons on the same position"
)]
fn duel_checkpoint() -> DuelSetup {
    let cards = json!({
        "north-ally": minion(json!({ "attack": 3, "defense": 4 })),
        "north-avatar": avatar(20),
        "north-bury": magic(("burrowTargetMinionOrArtifact", json!(true)), 0),
        "north-caster": minion(json!({ "burrowing": true, "spellcaster": true })),
        "north-duel": magic(("fightAllyWithAdjacentEnemy", json!(true)), 1),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-diagonal": minion(json!({ "attack": 2, "defense": 3, "summonToAnySite": true })),
        "south-disabled": minion(json!({
            "attack": 2,
            "defense": 3,
            "summonToAnySite": true,
            "waterbound": true,
        })),
        "south-normal": minion(json!({ "attack": 2, "defense": 3, "summonToAnySite": true })),
        "south-site": site(false),
        "south-stealthed": minion(json!({
            "attack": 2,
            "defense": 3,
            "stealth": true,
            "summonToAnySite": true,
        })),
        "south-warded": minion(json!({
            "attack": 2,
            "defense": 3,
            "summonToAnySite": true,
            "ward": true,
        })),
        "south-wrong-region": minion(json!({
            "attack": 2,
            "burrowing": true,
            "defense": 3,
            "summonToAnySite": true,
        })),
    });
    let north_spellbook = [
        "north-duel",
        "north-ally",
        "north-caster",
        "north-bury",
        "north-bury",
        "north-bury",
    ];
    let south_spellbook = [
        "south-normal",
        "south-warded",
        "south-stealthed",
        "south-disabled",
        "south-wrong-region",
        "south-diagonal",
    ];

    for seed in 1..=128 {
        let manifest = manifest(seed, &cards, &north_spellbook, &south_spellbook);
        let mut session = Session::new(&manifest).expect("valid Duel scenario");
        keep(&mut session);
        keep(&mut session);
        let opening = state(&session);
        let north_hand = opening["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North opening spellbook hand");
        if !["north-duel", "north-ally", "north-caster"]
            .into_iter()
            .all(|card_id| north_hand.iter().any(|card| card["cardId"] == card_id))
        {
            continue;
        }

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
        });
        let ally_id = summon_duel_minion(&mut session, "north-ally", "C4", None);
        let caster_id = summon_duel_minion(&mut session, "north-caster", "C4", None);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        for south_site in ["C2", "C3"] {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw"
                    && descriptor["zone"]
                        == if south_site == "C2" {
                            "spellbook"
                        } else {
                            "atlas"
                        }
            });
            if south_site == "C2" {
                accept_where(&mut session, |descriptor| {
                    descriptor["kind"] == "cast-magic"
                        && descriptor["cardId"] == "north-bury"
                        && descriptor["target"]["instanceId"] == caster_id
                });
            }
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == south_site
            });
            if south_site == "C2" {
                accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            }
        }

        let normal_id = summon_duel_minion(&mut session, "south-normal", "C3", None);
        let warded_id = summon_duel_minion(&mut session, "south-warded", "C4", None);
        let stealthed_id = summon_duel_minion(&mut session, "south-stealthed", "C4", None);
        let disabled_id = summon_duel_minion(&mut session, "south-disabled", "C4", None);
        let wrong_region_id = summon_duel_minion(&mut session, "south-wrong-region", "C4", None);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == "north-bury"
                && descriptor["target"]["instanceId"] == wrong_region_id
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "B3"
        });
        let diagonal_id = summon_duel_minion(&mut session, "south-diagonal", "B3", None);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });

        return DuelSetup {
            ally_id,
            caster_id,
            diagonal_id,
            disabled_id,
            normal_id,
            session,
            stealthed_id,
            warded_id,
            wrong_region_id,
        };
    }
    panic!("no deterministic Duel opening found");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct Duel proof keeps legality, all three outcome branches, checkpoint, stale request, and replay together"
)]
fn rule_catalog_0020_duel_uses_an_allys_region_and_the_shared_fight_pipeline() {
    let setup = duel_checkpoint();
    let checkpoint_state = state(&setup.session);
    let duel_spell_id = checkpoint_state["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North spellbook hand")
        .iter()
        .find(|card| card["cardId"] == "north-duel")
        .expect("Duel in hand")["instanceId"]
        .as_str()
        .expect("Duel identity")
        .to_owned();
    let checkpoint_mana = checkpoint_state["players"]["north"]["mana"]
        .as_u64()
        .expect("North mana");
    let checkpoint_actions = setup.session.legal_actions().expect("Duel actions");
    let duel_actions: Vec<_> = checkpoint_actions
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == duel_spell_id
                && action.descriptor["casterInstanceId"] == setup.caster_id
                && action.descriptor["ally"]["instanceId"] == setup.ally_id
        })
        .cloned()
        .collect();
    let target_ids: Vec<_> = duel_actions
        .iter()
        .map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .expect("Duel target identity")
                .to_owned()
        })
        .collect();
    let mut expected_target_ids = vec![
        setup.disabled_id.clone(),
        setup.normal_id.clone(),
        setup.warded_id.clone(),
    ];
    expected_target_ids.sort_unstable();
    assert_eq!(target_ids, expected_target_ids);
    assert!(!target_ids.contains(&setup.stealthed_id));
    assert!(!target_ids.contains(&setup.wrong_region_id));
    assert!(!target_ids.contains(&setup.diagonal_id));
    assert_eq!(
        checkpoint_state["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == setup.caster_id)
            .expect("Duel caster")["region"],
        "underground"
    );
    for action in &duel_actions {
        let target_id = action.descriptor["target"]["instanceId"]
            .as_str()
            .expect("Duel target identity");
        assert_eq!(
            action.descriptor,
            json!({
                "ally": { "instanceId": setup.ally_id, "kind": "minion", "seat": "north" },
                "cardId": "north-duel",
                "cardInstanceId": duel_spell_id,
                "casterInstanceId": setup.caster_id,
                "kind": "cast-magic",
                "target": { "instanceId": target_id, "kind": "minion", "seat": "south" },
            })
        );
        assert_eq!(
            action.label,
            format!(
                "Cast north-duel: minion {}… fights minion {}… with minion {}…",
                &setup.ally_id[..15],
                &target_id[..15],
                &setup.caster_id[..15]
            )
        );
    }

    let checkpoint = create_game_checkpoint(&setup.session).expect("captured Duel checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized Duel checkpoint");
    let mut restored = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed Duel checkpoint"),
    )
    .expect("restored Duel checkpoint");
    assert_eq!(state(&restored), checkpoint_state);
    assert_eq!(
        restored.legal_actions().expect("restored Duel actions"),
        checkpoint_actions
    );

    let normal_action = duel_actions
        .iter()
        .find(|action| action.descriptor["target"]["instanceId"] == setup.normal_id)
        .expect("normal Duel action")
        .clone();
    let pre_stale_hash = restored.state_hash().expect("pre-stale state hash");
    let pre_stale_transcript = restored.transcript().to_vec();
    let StepResult::Rejected(stale) = restored
        .step(ActionRequest {
            action_id: normal_action.action_id.to_string(),
            seat: normal_action.seat,
            state_version: normal_action.state_version + 1,
        })
        .expect("stable stale Duel rejection")
    else {
        panic!("stale Duel request must be rejected");
    };
    assert_eq!(stale.code, RejectionCode::StaleVersion);
    assert_eq!(
        restored.state_hash().expect("unchanged Duel state"),
        pre_stale_hash
    );
    assert_eq!(restored.transcript(), pre_stale_transcript);

    let StepResult::Accepted(normal) = restored
        .step(ActionRequest {
            action_id: normal_action.action_id.to_string(),
            seat: normal_action.seat,
            state_version: normal_action.state_version,
        })
        .expect("normal Duel step")
    else {
        panic!("engine-issued Duel action must be accepted");
    };
    assert_eq!(
        event_types(&normal),
        [
            "magic-cast",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "damage-dealt",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(
        normal.events[0].payload,
        json!({
            "allyInstanceId": setup.ally_id,
            "allySeat": "north",
            "cardId": "north-duel",
            "casterInstanceId": setup.caster_id,
            "instanceId": duel_spell_id,
            "manaPaid": 1,
            "seat": "north",
            "targetInstanceId": setup.normal_id,
            "targetSeat": "south",
        })
    );
    assert_eq!(
        normal.events[1].payload,
        json!({
            "attackerInstanceId": setup.ally_id,
            "combatantInstanceIds": [setup.normal_id],
        })
    );
    assert_eq!(
        normal.events[2].payload,
        json!({
            "amount": 3,
            "strikerInstanceId": setup.ally_id,
            "targetInstanceId": setup.normal_id,
        })
    );
    assert_eq!(normal.events[3].payload["instanceId"], setup.ally_id);
    assert_eq!(normal.events[3].payload["amount"], 2);
    assert_eq!(normal.events[4].payload["instanceId"], setup.normal_id);
    assert_eq!(normal.events[4].payload["amount"], 3);
    assert_eq!(
        normal.events[5].payload,
        json!({
            "cardId": "south-normal",
            "instanceId": setup.normal_id,
            "owner": "south",
        })
    );
    assert_eq!(
        normal.events[6].payload,
        json!({
            "cardId": "north-duel",
            "instanceId": duel_spell_id,
            "owner": "north",
        })
    );
    assert!(normal.random_draws.is_empty());
    let fought = state(&restored);
    let ally = fought["realm"]["units"]
        .as_array()
        .expect("post-Duel units")
        .iter()
        .find(|unit| unit["instanceId"] == setup.ally_id)
        .expect("surviving Duel ally");
    assert_eq!(ally["damage"], 2);
    assert_eq!(ally["location"], "C4");
    assert_eq!(ally["tapped"], false);
    assert!(
        fought["realm"]["units"]
            .as_array()
            .expect("post-Duel units")
            .iter()
            .all(|unit| unit["instanceId"] != setup.normal_id)
    );
    assert!(
        fought["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == setup.normal_id)
    );
    assert_eq!(fought["players"]["north"]["mana"], checkpoint_mana - 1);
    assert!(
        fought["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == duel_spell_id)
    );
    assert_exact_replay(&restored);

    let mut warded = setup.session.clone();
    let warded_action = duel_actions
        .iter()
        .find(|action| action.descriptor["target"]["instanceId"] == setup.warded_id)
        .expect("warded Duel action");
    let StepResult::Accepted(ward) = warded
        .step(ActionRequest {
            action_id: warded_action.action_id.to_string(),
            seat: warded_action.seat,
            state_version: warded_action.state_version,
        })
        .expect("warded Duel step")
    else {
        panic!("engine-issued warded Duel must be accepted");
    };
    assert_eq!(
        event_types(&ward),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let warded_state = state(&warded);
    assert_eq!(
        warded_state["realm"]["units"]
            .as_array()
            .expect("ward branch units")
            .iter()
            .find(|unit| unit["instanceId"] == setup.ally_id)
            .expect("unharmed Duel ally")["damage"],
        0
    );
    assert_eq!(
        warded_state["realm"]["units"]
            .as_array()
            .expect("ward branch units")
            .iter()
            .find(|unit| unit["instanceId"] == setup.warded_id)
            .expect("surviving Ward target")["warded"],
        false
    );
    assert_exact_replay(&warded);

    let mut disabled = setup.session;
    let disabled_action = duel_actions
        .iter()
        .find(|action| action.descriptor["target"]["instanceId"] == setup.disabled_id)
        .expect("disabled Duel action");
    let StepResult::Accepted(disabled_fight) = disabled
        .step(ActionRequest {
            action_id: disabled_action.action_id.to_string(),
            seat: disabled_action.seat,
            state_version: disabled_action.state_version,
        })
        .expect("disabled Duel step")
    else {
        panic!("engine-issued disabled Duel must be accepted");
    };
    assert_eq!(
        event_types(&disabled_fight),
        [
            "magic-cast",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(
        disabled_fight.events[3].payload["instanceId"],
        setup.disabled_id
    );
    let disabled_state = state(&disabled);
    assert_eq!(
        disabled_state["realm"]["units"]
            .as_array()
            .expect("disabled branch units")
            .iter()
            .find(|unit| unit["instanceId"] == setup.ally_id)
            .expect("unharmed Duel ally")["damage"],
        0
    );
    assert!(
        disabled_state["realm"]["units"]
            .as_array()
            .expect("disabled branch units")
            .iter()
            .all(|unit| unit["instanceId"] != setup.disabled_id)
    );
    assert!(disabled_fight.random_draws.is_empty());
    assert_exact_replay(&disabled);
}

#[test]
fn duel_should_apply_an_avatar_ally_from_the_existing_checkpoint() {
    let setup = duel_checkpoint();
    let north_avatar_id = state(&setup.session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let mut session = setup.session;
    let (descriptor, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["ally"]["instanceId"] == north_avatar_id
            && descriptor["target"]["instanceId"] == setup.normal_id
    });
    assert_eq!(descriptor["ally"]["kind"], "avatar");
    assert_eq!(descriptor["target"]["kind"], "minion");
    assert_eq!(
        event_types(&fight),
        [
            "magic-cast",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
            "damage-dealt",
            "magic-resolved",
        ]
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one pending Duel proof keeps lower-region combat, first strike, ordered Deathrites, terminal completion, checkpoint, and replay together"
)]
fn duel_should_checkpoint_an_underground_first_strike_and_finish_before_terminal() {
    let cards = json!({
        "north-ally": minion(json!({
            "attack": 2,
            "burrowing": true,
            "defense": 10,
            "strikesFirstWhileAttacking": true,
        })),
        "north-avatar": avatar(20),
        "north-burrow": magic(("burrowAllMinionsAndArtifactsAtTargetLandSite", json!(true)), 0),
        "north-duel": magic(("fightAllyWithAdjacentEnemy", json!(true)), 0),
        "north-filler": minion(json!({})),
        "north-pinger": minion(json!({
            "defense": 10,
            "genesisDamageEachOtherUnitHere": 1,
        })),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-buff": minion(json!({
            "burrowing": true,
            "deathriteDrawSite": true,
            "otherNearbyAlliesPowerBonus": 1,
            "summonToAnySite": true,
        })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-ally",
        "north-pinger",
        "north-burrow",
        "north-duel",
        "north-filler",
        "north-filler",
    ];
    let mut prepared = None;
    for seed in 1..=512 {
        let manifest = manifest(seed, &cards, &north_spellbook, &["south-buff"; 6]);
        let mut session = Session::new(&manifest).expect("valid pending Duel candidate");
        keep(&mut session);
        keep(&mut session);
        let opening = state(&session);
        let hand = opening["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North opening hand");
        if !hand.iter().any(|card| card["cardId"] == "north-ally") {
            continue;
        }
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
        });
        let ally_id = summon_duel_minion(&mut session, "north-ally", "C4", None);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
        let mut target_ids = Vec::new();
        for _ in 0..2 {
            target_ids.push(summon_duel_minion(&mut session, "south-buff", "C4", None));
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        let drawn = state(&session);
        let north_hand = drawn["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North hand after one draw")
            .iter()
            .filter_map(|card| card["cardId"].as_str())
            .collect::<Vec<_>>();
        if !["north-pinger", "north-burrow", "north-duel"]
            .into_iter()
            .all(|card_id| north_hand.contains(&card_id))
        {
            continue;
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        summon_duel_minion(&mut session, "north-pinger", "C4", None);
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == "north-burrow"
                && descriptor["targetLocation"]["cell"] == "C4"
        });
        let pre_duel = state(&session);
        assert!(target_ids.iter().all(|instance_id| {
            realm_unit(&pre_duel, instance_id).is_some_and(|unit| unit["damage"] == 1)
        }));
        prepared = Some((session, ally_id, target_ids));
        break;
    }
    let (mut session, ally_id, mut target_ids) =
        prepared.expect("bounded seed with the complete pending Duel setup");
    target_ids.sort_unstable();
    let target_id = target_ids[0].clone();
    let duel_action = session
        .legal_actions()
        .expect("underground Duel actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-duel"
                && action.descriptor["ally"]["instanceId"] == ally_id
                && action.descriptor["target"]["instanceId"] == target_id
        })
        .expect("engine-issued underground Duel");
    let StepResult::Accepted(cast) = session
        .step(ActionRequest {
            action_id: duel_action.action_id.to_string(),
            seat: duel_action.seat,
            state_version: duel_action.state_version,
        })
        .expect("underground Duel cast")
    else {
        panic!("engine-issued underground Duel must be accepted");
    };
    assert_eq!(
        event_types(&cast),
        [
            "magic-cast",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
        ]
    );
    let pending = state(&session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(pending["decisionSeat"], "south");
    assert_eq!(
        pending["pendingDeathrites"]["continuation"]["kind"],
        "first-strike"
    );
    assert_eq!(
        pending["pendingDeathrites"]["continuation"]["pending"]["region"],
        "underground"
    );
    assert_eq!(
        pending["pendingDeathrites"]["deferredOutcomes"],
        json!([{
            "payload": {
                "cardId": "north-duel",
                "instanceId": cast.events[0].payload["instanceId"],
                "owner": "north",
            },
            "type": "magic-resolved",
        }])
    );

    let checkpoint = create_game_checkpoint(&session).expect("pending Duel checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized pending Duel");
    session = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed pending Duel checkpoint"),
    )
    .expect("resumed pending Duel checkpoint");
    assert_eq!(state(&session), pending);
    let (_, completed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == target_ids[0]
    });
    let completed_types = event_types(&completed);
    assert_eq!(
        &completed_types[completed_types.len() - 2..],
        ["magic-resolved", "game-ended"]
    );
    assert_eq!(state(&session)["phase"], "terminal");
    assert_exact_replay(&session);
}

fn fire_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["fire"],
    })
}

fn fire_minion(extra: Value) -> Value {
    let mut value = minion(extra);
    value["thresholds"] = json!({ "air": 0, "earth": 0, "fire": 1, "water": 0 });
    value
}

fn fire_magic(effect: (&str, Value), mana_cost: u8) -> Value {
    let mut value = magic(effect, mana_cost);
    value["thresholds"] = json!({ "air": 0, "earth": 0, "fire": 1, "water": 0 });
    value
}

#[expect(
    clippy::too_many_lines,
    reason = "the catalog proof keeps the oversized Leap destination and focused strike beside the core rule"
)]
fn assert_oversized_leap_attack_focuses_one_occupied_cell() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-filler": fire_minion(json!({})),
        "north-giant": fire_minion(json!({
            "attack": 3,
            "defense": 6,
            "occupiesSquareArea": 2,
        })),
        "north-leap": fire_magic(("leapAttackAlly", json!(true)), 1),
        "north-site": fire_site(),
        "south-avatar": avatar(20),
        "south-enemy": fire_minion(json!({
            "attack": 1,
            "defense": 3,
            "summonToAnySite": true,
        })),
        "south-site": fire_site(),
    });
    let north_spellbook = [
        "north-giant",
        "north-leap",
        "north-filler",
        "north-leap",
        "north-giant",
        "north-leap",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-enemy"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("oversized Leap candidate");
            let opening = state(&preview);
            let hand = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand");
            let future = opening["players"]["north"]["spellbook"]
                .as_array()
                .expect("North spellbook");
            ["north-giant", "north-leap"].into_iter().all(|card_id| {
                hand.iter()
                    .chain(future.iter().take(3))
                    .any(|card| card["cardId"] == card_id)
            })
        })
        .expect("bounded seed with Giant and Leap by the fourth turn");
    let mut session = Session::new(&manifest).expect("valid oversized Leap scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let c2_enemy = summon_duel_minion(&mut session, "south-enemy", "C2", None);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B2"
    });
    let b2_enemy = summon_duel_minion(&mut session, "south-enemy", "B2", None);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B3"
    });
    let giant_id = summon_duel_minion(&mut session, "north-giant", "B3", None);
    let leap_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-leap")
        .expect("oversized Leap in hand")["instanceId"]
        .as_str()
        .expect("oversized Leap identity")
        .to_owned();

    let actions = session.legal_actions().expect("oversized Leap actions");
    let b2_actions: Vec<_> = actions
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == leap_id
                && action.descriptor["ally"]["instanceId"] == giant_id
                && action.descriptor["allyDestination"]["cell"] == "B2"
        })
        .cloned()
        .collect();
    assert_eq!(
        b2_actions
            .iter()
            .map(|action| action.descriptor["allyStrikeLocation"]["cell"]
                .as_str()
                .expect("oversized strike cell"))
            .collect::<Vec<_>>(),
        ["B2", "B3", "C2", "C3"]
    );
    let focused = b2_actions
        .iter()
        .find(|action| action.descriptor["allyStrikeLocation"]["cell"] == "B2")
        .expect("focused B2 oversized Leap");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: focused.action_id.to_string(),
            seat: focused.seat,
            state_version: focused.state_version,
        })
        .expect("focused oversized Leap")
    else {
        panic!("engine-issued oversized Leap must be accepted");
    };
    let allocations: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| event.payload["targetInstanceId"].clone())
        .collect();
    assert_eq!(allocations, [json!(b2_enemy)]);
    let after = state(&session);
    assert!(realm_unit(&after, &b2_enemy).is_none());
    assert!(realm_unit(&after, &c2_enemy).is_some());
    assert_eq!(
        realm_unit(&after, &giant_id).expect("surviving Giant")["occupiedCells"],
        json!(["B2", "B3", "C2", "C3"])
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct Leap Attack proof keeps legality, status restrictions, both outcomes, oversized focus, and replay together"
)]
fn rule_catalog_0021_leap_attack_optionally_steps_an_ally_before_it_strikes_every_enemy_there() {
    let cards = json!({
        "north-ally": fire_minion(json!({
            "attack": 3,
            "defense": 4,
            "movementBonus": 2,
        })),
        "north-avatar": avatar(20),
        "north-disabled": fire_minion(json!({ "defense": 4, "waterbound": true })),
        "north-immobile": fire_minion(json!({ "defense": 4, "immobile": true })),
        "north-leap": fire_magic(("leapAttackAlly", json!(true)), 1),
        "north-site": fire_site(),
        "south-avatar": avatar(20),
        "south-normal": fire_minion(json!({
            "attack": 2,
            "defense": 3,
            "summonToAnySite": true,
        })),
        "south-origin": fire_minion(json!({
            "attack": 2,
            "defense": 3,
            "summonToAnySite": true,
        })),
        "south-site": fire_site(),
        "south-stealthed": fire_minion(json!({
            "attack": 2,
            "defense": 3,
            "stealth": true,
            "summonToAnySite": true,
        })),
        "south-warded": fire_minion(json!({
            "airborne": true,
            "attack": 2,
            "defense": 3,
            "summonToAnySite": true,
            "ward": true,
        })),
    });
    let north_spellbook = [
        "north-leap",
        "north-ally",
        "north-immobile",
        "north-disabled",
        "north-leap",
        "north-ally",
    ];
    let south_spellbook = [
        "south-origin",
        "south-normal",
        "south-warded",
        "south-stealthed",
        "south-origin",
        "south-normal",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &south_spellbook))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Leap Attack candidate");
            let opening = state(&preview);
            let north_hand = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand");
            let north_future = opening["players"]["north"]["spellbook"]
                .as_array()
                .expect("North spellbook");
            let south_hand = opening["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("South opening hand");
            let south_future = opening["players"]["south"]["spellbook"]
                .as_array()
                .expect("South spellbook");
            ["north-leap", "north-ally"]
                .into_iter()
                .all(|card_id| north_hand.iter().any(|card| card["cardId"] == card_id))
                && ["north-immobile", "north-disabled"]
                    .into_iter()
                    .all(|card_id| {
                        north_hand
                            .iter()
                            .chain(north_future.iter().take(1))
                            .any(|card| card["cardId"] == card_id)
                    })
                && [
                    "south-origin",
                    "south-normal",
                    "south-warded",
                    "south-stealthed",
                ]
                .into_iter()
                .all(|card_id| {
                    south_hand
                        .iter()
                        .chain(south_future.iter().take(1))
                        .any(|card| card["cardId"] == card_id)
                })
        })
        .expect("bounded seed with complete Leap Attack setup");
    let mut session = Session::new(&manifest).expect("valid Leap Attack scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let ally_id = summon_duel_minion(&mut session, "north-ally", "C4", None);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let origin_id = summon_duel_minion(&mut session, "south-origin", "C4", None);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let immobile_id = summon_duel_minion(&mut session, "north-immobile", "C4", None);
    let disabled_id = summon_duel_minion(&mut session, "north-disabled", "C4", None);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let normal_id = summon_duel_minion(&mut session, "south-normal", "C3", None);
    let warded_id = summon_duel_minion(&mut session, "south-warded", "C3", None);
    let stealthed_id = summon_duel_minion(&mut session, "south-stealthed", "C3", None);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });

    let before = state(&session);
    let spell_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-leap")
        .expect("Leap Attack in hand")["instanceId"]
        .as_str()
        .expect("Leap Attack identity")
        .to_owned();
    let actions = session.legal_actions().expect("Leap Attack actions");
    let actions_for = |instance_id: &str| {
        actions
            .iter()
            .filter(|action| {
                action.descriptor["kind"] == "cast-magic"
                    && action.descriptor["cardInstanceId"] == spell_id
                    && action.descriptor["ally"]["instanceId"] == instance_id
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let ally_actions = actions_for(&ally_id);
    assert_eq!(
        ally_actions
            .iter()
            .map(|action| format!(
                "{}/{}",
                action.descriptor["allyDestination"]["cell"]
                    .as_str()
                    .expect("destination cell"),
                action.descriptor["allyDestination"]["region"]
                    .as_str()
                    .expect("destination region")
            ))
            .collect::<Vec<_>>(),
        ["C3/surface", "C4/surface"]
    );
    assert_eq!(
        actions_for(&immobile_id)
            .iter()
            .map(|action| action.descriptor["allyDestination"]["cell"].clone())
            .collect::<Vec<_>>(),
        [json!("C4")]
    );
    assert_eq!(
        actions_for(&disabled_id)
            .iter()
            .map(|action| action.descriptor["allyDestination"]["cell"].clone())
            .collect::<Vec<_>>(),
        [json!("C4")]
    );
    assert!(actions.iter().any(|action| {
        action.descriptor["kind"] == "cast-magic"
            && action.descriptor["cardInstanceId"] == spell_id
            && action.descriptor["ally"]["kind"] == "avatar"
    }));
    let stay_action = ally_actions
        .iter()
        .find(|action| action.descriptor["allyDestination"]["cell"] == "C4")
        .expect("Leap stay action");
    let step_action = ally_actions
        .iter()
        .find(|action| action.descriptor["allyDestination"]["cell"] == "C3")
        .expect("Leap step action");
    assert_eq!(
        stay_action.label,
        format!(
            "Cast north-leap: minion {}… stays and strikes enemies at C4",
            &ally_id[..15]
        )
    );
    assert_eq!(
        step_action.label,
        format!(
            "Cast north-leap: minion {}… steps to C3 and strikes enemies at C3",
            &ally_id[..15]
        )
    );
    let before_mana = before["players"]["north"]["mana"]
        .as_u64()
        .expect("North mana");
    let before_version = before["stateVersion"].as_u64().expect("state version");

    let mut stayed = session.clone();
    let StepResult::Accepted(stay_receipt) = stayed
        .step(ActionRequest {
            action_id: stay_action.action_id.to_string(),
            seat: stay_action.seat,
            state_version: stay_action.state_version,
        })
        .expect("stay Leap")
    else {
        panic!("engine-issued stay Leap must be accepted");
    };
    assert!(!event_types(&stay_receipt).contains(&"unit-stepped"));
    assert_eq!(
        event_types(&stay_receipt)
            .into_iter()
            .filter(|event_type| *event_type == "strike-damage-allocated")
            .count(),
        1
    );
    let stayed_state = state(&stayed);
    assert!(realm_unit(&stayed_state, &origin_id).is_none());
    assert!(realm_unit(&stayed_state, &normal_id).is_some());
    assert!(realm_unit(&stayed_state, &warded_id).is_some());
    assert!(realm_unit(&stayed_state, &stealthed_id).is_some());
    assert_eq!(stayed_state["stateVersion"], before_version + 1);
    assert_exact_replay(&stayed);

    let mut stepped = session;
    let StepResult::Accepted(step_receipt) = stepped
        .step(ActionRequest {
            action_id: step_action.action_id.to_string(),
            seat: step_action.seat,
            state_version: step_action.state_version,
        })
        .expect("step Leap")
    else {
        panic!("engine-issued step Leap must be accepted");
    };
    let movement = step_receipt
        .events
        .iter()
        .find(|event| event.event_type == "unit-stepped")
        .expect("Leap movement");
    assert_eq!(
        movement.payload,
        json!({
            "from": { "cell": "C4", "region": "surface" },
            "instanceId": ally_id,
            "seat": "north",
            "sourceInstanceId": spell_id,
            "steps": 1,
            "to": { "cell": "C3", "region": "surface" },
        })
    );
    assert_eq!(
        step_receipt
            .events
            .iter()
            .filter(|event| event.event_type == "strike-damage-allocated")
            .count(),
        3
    );
    assert!(!event_types(&step_receipt).contains(&"fight-started"));
    assert!(step_receipt.random_draws.is_empty());
    assert_eq!(
        step_receipt
            .events
            .last()
            .expect("Leap completion")
            .event_type,
        "magic-resolved"
    );
    let after = state(&stepped);
    let ally = realm_unit(&after, &ally_id).expect("surviving Leap ally");
    assert_eq!(ally["damage"], 0);
    assert_eq!(ally["location"], "C3");
    assert_eq!(ally["tapped"], false);
    assert!(realm_unit(&after, &normal_id).is_none());
    assert!(realm_unit(&after, &stealthed_id).is_none());
    assert_eq!(
        realm_unit(&after, &warded_id).expect("Ward survivor")["warded"],
        false
    );
    assert!(realm_unit(&after, &origin_id).is_some());
    assert_eq!(after["players"]["north"]["mana"], before_mana - 1);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    assert_eq!(after["stateVersion"], before_version + 1);
    assert_exact_replay(&stepped);

    assert_oversized_leap_attack_focuses_one_occupied_cell();
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one ordered Leap proof retains setup, continuation JSON, checkpoint, both order branches, versioning, and replay"
)]
fn rule_catalog_0022_leap_attack_resumes_its_strike_after_ordered_movement_deathrites() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-fragile-a": fire_minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
        })),
        "north-fragile-b": fire_minion(json!({
            "deathriteDrawSite": true,
            "defense": 1,
        })),
        "north-leap": fire_magic(("leapAttackAlly", json!(true)), 1),
        "north-rain": fire_magic(("damageEachAbovegroundMinion", json!(1)), 1),
        "north-site": fire_site(),
        "north-source": fire_minion(json!({
            "attack": 3,
            "defense": 3,
            "otherNearbyAlliesPowerBonus": 1,
        })),
        "south-avatar": avatar(20),
        "south-enemy": fire_minion(json!({
            "attack": 1,
            "defense": 3,
            "summonToAnySite": true,
        })),
        "south-site": fire_site(),
    });
    let north_spellbook = [
        "north-leap",
        "north-source",
        "north-fragile-a",
        "north-fragile-b",
        "north-rain",
        "north-leap",
        "north-source",
        "north-rain",
    ];
    let manifest = (1..=4096)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-enemy"; 8]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("ordered Leap candidate");
            let opening = state(&preview);
            let north_hand = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand");
            let north_future = opening["players"]["north"]["spellbook"]
                .as_array()
                .expect("North spellbook");
            ["north-fragile-a", "north-fragile-b"]
                .into_iter()
                .all(|card_id| north_hand.iter().any(|card| card["cardId"] == card_id))
                && ["north-source"].into_iter().all(|card_id| {
                    north_hand
                        .iter()
                        .chain(north_future.iter().take(1))
                        .any(|card| card["cardId"] == card_id)
                })
                && ["north-leap", "north-rain"].into_iter().all(|card_id| {
                    north_hand
                        .iter()
                        .chain(north_future.iter().take(2))
                        .any(|card| card["cardId"] == card_id)
                })
        })
        .expect("bounded seed with complete ordered Leap setup");
    let mut session = Session::new(&manifest).expect("valid ordered Leap scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let fragile_ids = [
        summon_duel_minion(&mut session, "north-fragile-a", "C4", None),
        summon_duel_minion(&mut session, "north-fragile-b", "C4", None),
    ];
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let source_id = summon_duel_minion(&mut session, "north-source", "C3", None);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let enemy_id = summon_duel_minion(&mut session, "south-enemy", "C2", None);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    let before = state(&session);
    assert!(fragile_ids.iter().all(|instance_id| {
        realm_unit(&before, instance_id).is_some_and(|unit| unit["damage"] == 1)
    }));
    let atlas_before = before["players"]["north"]["atlas"]
        .as_array()
        .expect("North atlas")
        .len();
    let leap_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-leap")
        .expect("Leap Attack in hand")["instanceId"]
        .as_str()
        .expect("Leap identity")
        .to_owned();
    let before_version = before["stateVersion"].as_u64().expect("state version");
    let leap = session
        .legal_actions()
        .expect("ordered Leap actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == leap_id
                && action.descriptor["ally"]["instanceId"] == source_id
                && action.descriptor["allyDestination"]["cell"] == "C2"
        })
        .expect("engine-issued ordered Leap");
    let StepResult::Accepted(interrupted) = session
        .step(ActionRequest {
            action_id: leap.action_id.to_string(),
            seat: leap.seat,
            state_version: leap.state_version,
        })
        .expect("ordered Leap cast")
    else {
        panic!("engine-issued ordered Leap must be accepted");
    };
    assert_eq!(event_types(&interrupted), ["magic-cast", "unit-stepped"]);
    let pending = state(&session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(pending["decisionSeat"], "north");
    assert_eq!(pending["stateVersion"], before_version + 1);
    assert_eq!(
        pending["pendingDeathrites"]["continuation"],
        json!({
            "ally": { "instanceId": source_id, "kind": "minion", "seat": "north" },
            "cardId": "north-leap",
            "instanceId": leap_id,
            "kind": "leap-attack",
            "owner": "north",
            "strikeLocation": { "cell": "C2", "region": "surface" },
        })
    );
    assert_eq!(
        realm_unit(&pending, &source_id).expect("moved Leap source")["location"],
        "C2"
    );
    assert!(realm_unit(&pending, &enemy_id).is_some());
    assert!(
        fragile_ids
            .iter()
            .all(|instance_id| realm_unit(&pending, instance_id).is_none())
    );
    assert!(fragile_ids.iter().all(|instance_id| {
        !pending["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id.as_str())
    }));

    let checkpoint = create_game_checkpoint(&session).expect("ordered Leap checkpoint");
    let serialized =
        serialize_game_checkpoint(&checkpoint).expect("serialized ordered Leap checkpoint");
    let restored = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed ordered Leap checkpoint"),
    )
    .expect("resumed ordered Leap checkpoint");
    assert_eq!(state(&restored), pending);
    let order_actions = session.legal_actions().expect("source order actions");
    let restored_actions = restored.legal_actions().expect("restored order actions");
    assert_eq!(
        restored_actions
            .iter()
            .map(|action| action.action_id.clone())
            .collect::<Vec<_>>(),
        order_actions
            .iter()
            .map(|action| action.action_id.clone())
            .collect::<Vec<_>>()
    );
    let order_actions: Vec<_> = restored_actions
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .collect();
    assert_eq!(order_actions.len(), 2);

    let mut branch_hashes = Vec::new();
    for order in order_actions {
        let chosen_id = order.descriptor["sourceInstanceId"]
            .as_str()
            .expect("chosen Deathrite identity");
        let other_id = fragile_ids
            .iter()
            .find(|instance_id| instance_id.as_str() != chosen_id)
            .expect("other Deathrite identity");
        let mut branch = restored.clone();
        let StepResult::Accepted(ordered) = branch
            .step(ActionRequest {
                action_id: order.action_id.to_string(),
                seat: order.seat,
                state_version: order.state_version,
            })
            .expect("ordered Leap completion")
        else {
            panic!("engine-issued Deathrite order must be accepted");
        };
        let types = event_types(&ordered);
        assert_eq!(
            &types[..5],
            [
                "deathrite-order-committed",
                "site-drawn",
                "site-drawn",
                "minion-died",
                "minion-died",
            ]
        );
        assert_eq!(types.last(), Some(&"magic-resolved"));
        let strike_index = types
            .iter()
            .position(|event_type| *event_type == "strike-damage-allocated")
            .expect("resumed Leap strike");
        assert!(strike_index > 4);
        assert!(
            strike_index
                < types
                    .iter()
                    .rposition(|event_type| *event_type == "minion-died")
                    .expect("enemy death")
        );
        assert_eq!(
            ordered
                .events
                .iter()
                .filter(|event| event.event_type == "site-drawn")
                .map(|event| event.payload["sourceInstanceId"].clone())
                .collect::<Vec<_>>(),
            [json!(chosen_id), json!(other_id)]
        );
        let completed = state(&branch);
        assert_eq!(completed["phase"], "main");
        assert!(completed["pendingDeathrites"].is_null());
        assert_eq!(
            realm_unit(&completed, &source_id).expect("surviving Leap source")["location"],
            "C2"
        );
        assert!(realm_unit(&completed, &enemy_id).is_none());
        assert!(fragile_ids.iter().all(|instance_id| {
            completed["players"]["north"]["cemetery"]
                .as_array()
                .expect("North cemetery")
                .iter()
                .any(|card| card["instanceId"] == instance_id.as_str())
        }));
        assert_eq!(
            completed["players"]["north"]["atlas"]
                .as_array()
                .expect("North atlas")
                .len(),
            atlas_before - 2
        );
        assert_eq!(completed["stateVersion"], before_version + 2);
        assert_exact_replay(&branch);
        branch_hashes.push(branch.state_hash().expect("completed state hash"));
    }
    assert_eq!(branch_hashes[0], branch_hashes[1]);
}

fn fatality_manifest() -> String {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-fatality": magic(("killTargetWoundedMinion", json!(true)), 0),
        "north-lash": magic(("damageTargetUnit", json!(1)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({ "defense": 3 })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-lash",
        "north-lash",
        "north-lash",
        "north-fatality",
        "north-fatality",
        "north-fatality",
    ];
    (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-minion"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Fatality seed candidate");
            let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand")
                .clone();
            ["north-fatality", "north-lash"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("bounded seed with both North Magic cards in hand")
}

fn genesis_strike_manifest() -> String {
    let cards = json!({
        "north-ally": minion(json!({ "defense": 3, "summonToAnySite": true })),
        "north-avatar": avatar(20),
        "north-site": site(false),
        "north-titan": minion(json!({
            "attack": 3,
            "defense": 3,
            "genesisStrikeEachEnemyHere": true,
            "summonToAnySite": true,
        })),
        "south-avatar": avatar(20),
        "south-plain": minion(json!({ "defense": 5 })),
        "south-site": site(false),
        "south-warded": minion(json!({ "defense": 5, "ward": true })),
    });
    let north_spellbook = [
        "north-ally",
        "north-titan",
        "north-ally",
        "north-titan",
        "north-ally",
        "north-titan",
    ];
    let south_spellbook = [
        "south-plain",
        "south-warded",
        "south-plain",
        "south-warded",
        "south-plain",
        "south-warded",
    ];
    (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &south_spellbook))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Genesis strike seed candidate");
            let opening = state(&preview);
            let has = |seat: &str, wanted: [&str; 2]| {
                let hand = opening["players"][seat]["hand"]["spellbook"]
                    .as_array()
                    .expect("opening hand")
                    .clone();
                wanted
                    .into_iter()
                    .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
            };
            has("north", ["north-ally", "north-titan"])
                && has("south", ["south-plain", "south-warded"])
        })
        .expect("bounded seed with every Genesis strike participant in hand")
}

/// Walks South into two minions on its own opening site at C1.
fn enemies_at_c1(session: &mut Session) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut enemy_ids = Vec::new();
    for card_id in ["south-plain", "south-warded"] {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["region"].is_null()
                && descriptor["cardId"] == card_id
                && descriptor["cell"] == "C1"
        });
        enemy_ids.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("enemy identity")
                .to_owned(),
        );
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    enemy_ids
}

#[test]
fn rule_catalog_0057_genesis_strike_should_hit_every_enemy_sharing_the_newcomers_cell() {
    let manifest = genesis_strike_manifest();
    let mut session = opening_main(&manifest);
    let enemy_ids = enemies_at_c1(&mut session);
    let (ally, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C1"
    });
    let ally_id = ally["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned();
    let (titan, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-titan"
            && descriptor["cell"] == "C1"
    });
    let titan_id = titan["cardInstanceId"]
        .as_str()
        .expect("titan identity")
        .to_owned();

    let mut struck: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "strike-damage-allocated")
        .map(|event| {
            assert_eq!(event.payload["amount"], 3);
            assert_eq!(event.payload["strikerInstanceId"], titan_id.as_str());
            event.payload["targetInstanceId"]
                .as_str()
                .expect("struck identity")
                .to_owned()
        })
        .collect();
    let enemy_avatar_id = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    let mut expected = enemy_ids.clone();
    expected.push(enemy_avatar_id.clone());
    struck.sort_unstable();
    expected.sort_unstable();
    // Every enemy sharing the cell is struck, including the Avatar standing on its own site,
    // while the co-located ally and the striker itself are skipped.
    assert_eq!(struck, expected);
    assert!(!struck.contains(&ally_id));
    assert!(!struck.contains(&titan_id));
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken")
    );

    let resolved = state(&session);
    let plain_id = enemy_ids
        .iter()
        .find(|instance_id| {
            realm_unit(&resolved, instance_id).expect("enemy")["cardId"] == "south-plain"
        })
        .expect("plain enemy")
        .clone();
    let warded_id = enemy_ids
        .iter()
        .find(|instance_id| **instance_id != plain_id)
        .expect("warded enemy")
        .clone();
    assert_eq!(
        realm_unit(&resolved, &plain_id).expect("struck enemy")["damage"],
        3
    );
    let warded = realm_unit(&resolved, &warded_id).expect("warded enemy");
    assert_eq!(warded["damage"], 0);
    assert_eq!(warded["warded"], false);
    assert_eq!(
        realm_unit(&resolved, &ally_id).expect("spared ally")["damage"],
        0
    );
    assert_eq!(
        realm_unit(&resolved, &titan_id).expect("striker")["damage"],
        0
    );
    assert_eq!(resolved["players"]["south"]["avatar"]["life"], 17);
    assert_eq!(resolved["players"]["north"]["avatar"]["life"], 20);
    assert_exact_replay(&session);
}

/// Walks South into a second site at C2 holding two identical minions.
fn two_minions_at_c2(session: &mut Session) -> Vec<String> {
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
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let mut candidates = Vec::new();
    for _ in 0..2 {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cell"] == "C2"
                && descriptor["region"].is_null()
        });
        candidates.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("summoned candidate identity")
                .to_owned(),
        );
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    candidates.sort_unstable();
    candidates
}

#[test]
fn rule_catalog_0027_lightning_bolt_should_damage_one_random_unit_at_the_chosen_location() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-bolt": magic(("damageRandomUnitAtLocation", json!(2)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({ "defense": 3 })),
        "south-site": site(false),
    });
    let manifest = manifest(37, &cards, &["north-bolt"; 6], &["south-minion"; 6]);
    let mut session = opening_main(&manifest);
    let candidates = two_minions_at_c2(&mut session);

    // One choice per surface location that exists, and the target unit stays out of the descriptor.
    let mut offered: Vec<_> = session
        .legal_actions()
        .expect("Lightning Bolt actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-bolt"
        })
        .map(|action| {
            assert_eq!(action.descriptor["targetLocation"]["region"], "surface");
            assert!(action.descriptor.get("target").is_none());
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .expect("bolt target cell")
                .to_owned()
        })
        .collect();
    offered.sort_unstable();
    offered.dedup();
    assert_eq!(offered, ["C1", "C2", "C4"]);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bolt"
            && descriptor["targetLocation"]["cell"] == "C2"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "magic-resolved",
        ]
    );
    assert!(receipt.random_draws.iter().all(|draw| {
        draw["purpose"] == "magic_random_unit_at_location"
            && draw["domain"]["kind"] == "unit_index_candidate"
            && draw["domain"]["exclusiveMaximum"] == 2
    }));
    let accepted = receipt
        .random_draws
        .iter()
        .rev()
        .find(|draw| draw["domain"]["accepted"] == true)
        .expect("accepted random draw");
    let selected_index = usize::try_from(
        accepted["result"].as_u64().expect("random uint32") % candidates.len() as u64,
    )
    .expect("candidate index");
    let selected = candidates[selected_index].clone();
    let spared = candidates
        .iter()
        .find(|instance_id| **instance_id != selected)
        .expect("spared candidate")
        .clone();
    assert_eq!(
        receipt.events[1].payload["targetInstanceId"],
        selected.as_str()
    );

    let damaged = state(&session);
    assert_eq!(
        realm_unit(&damaged, &selected).expect("struck unit")["damage"],
        2
    );
    assert_eq!(
        realm_unit(&damaged, &spared).expect("spared unit")["damage"],
        0
    );
    assert_exact_replay(&session);
}

fn mesmerism_manifest() -> String {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-lash": magic(("damageTargetUnit", json!(1)), 0),
        "north-mesmerism": magic(("gainControlOfTargetNearbyMinion", json!(true)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-far": minion(json!({ "deathriteDrawSite": true })),
        "south-site": site(false),
        "south-target": minion(json!({ "deathriteDrawSite": true })),
    });
    let north_spellbook = [
        "north-mesmerism",
        "north-lash",
        "north-mesmerism",
        "north-lash",
        "north-mesmerism",
        "north-lash",
        "north-lash",
        "north-lash",
    ];
    let south_spellbook = [
        "south-far",
        "south-target",
        "south-far",
        "south-target",
        "south-far",
        "south-target",
        "south-far",
        "south-target",
    ];
    (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &south_spellbook))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Mesmerism seed candidate");
            let opening = state(&preview);
            let north = opening["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand")
                .clone();
            let south = opening["players"]["south"]["hand"]["spellbook"]
                .as_array()
                .expect("South opening hand")
                .clone();
            ["north-lash", "north-mesmerism"]
                .into_iter()
                .all(|card_id| north.iter().any(|card| card["cardId"] == card_id))
                && ["south-far", "south-target"]
                    .into_iter()
                    .all(|card_id| south.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("bounded seed with both Mesmerism and both South minions in hand")
}

/// Walks the shared opening into North Avatar at C3 with South minions at C1 and C2.
fn mesmerism_opening(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
    });
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
    let (near, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "south-target"
            && descriptor["cell"] == "C2"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let avatar_id = state(session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == avatar_id.as_str()
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    (
        far["cardInstanceId"]
            .as_str()
            .expect("far minion identity")
            .to_owned(),
        near["cardInstanceId"]
            .as_str()
            .expect("nearby minion identity")
            .to_owned(),
    )
}

#[test]
fn rule_catalog_0146_mesmerism_should_transfer_a_minion_and_its_deathrite_to_the_new_controller() {
    let manifest = mesmerism_manifest();
    let mut session = opening_main(&manifest);
    let (far_id, near_id) = mesmerism_opening(&mut session);

    let mesmerism_targets: Vec<_> = session
        .legal_actions()
        .expect("Mesmerism actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-mesmerism"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    assert!(mesmerism_targets.contains(&near_id));
    assert!(!mesmerism_targets.contains(&far_id));

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == near_id.as_str()
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-control-changed", "magic-resolved"]
    );
    let changed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-control-changed")
        .expect("control change event");
    assert_eq!(changed.payload["fromSeat"], "south");
    assert_eq!(changed.payload["seat"], "north");

    let transferred = state(&session);
    assert_eq!(
        realm_unit(&transferred, &near_id).expect("transferred minion")["controller"],
        "north"
    );
    assert_eq!(
        realm_unit(&transferred, &near_id).expect("transferred minion")["owner"],
        "south"
    );
    let north_atlas = transferred["players"]["north"]["atlas"]
        .as_array()
        .expect("North atlas")
        .len();

    // Killing the stolen minion now fires its Deathrite for its new controller.
    let (_, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == near_id.as_str()
    });
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");
    let finished = state(&session);
    assert!(realm_unit(&finished, &near_id).is_none());
    assert_eq!(
        finished["players"]["north"]["atlas"]
            .as_array()
            .expect("North atlas after Deathrite")
            .len(),
        north_atlas - 1
    );
    // The corpse still returns to its owner's cemetery, not the new controller's.
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == near_id.as_str())
    );
    assert_exact_replay(&session);
}

fn fatality_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("Fatality actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fatality"
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

#[test]
fn rule_catalog_0147_fatality_should_kill_only_a_wounded_minion_in_the_caster_region() {
    let manifest = fatality_manifest();
    let mut session = opening_main(&manifest);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut enemy_ids = Vec::new();
    for _ in 0..2 {
        let (summoned, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cell"] == "C1"
                && descriptor["region"].is_null()
        });
        enemy_ids.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("summoned enemy identity")
                .to_owned(),
        );
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });

    let wounded_id = enemy_ids[0].clone();
    let healthy_id = enemy_ids[1].clone();
    // Nothing is wounded yet, so the caster has no legal Fatality target at all.
    assert!(fatality_targets(&session).is_empty());

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == wounded_id.as_str()
    });
    let wounded_only = fatality_targets(&session);
    assert_eq!(wounded_only, [wounded_id.as_str()]);
    assert!(!wounded_only.contains(&healthy_id));

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fatality"
    });
    assert_eq!(descriptor["target"]["instanceId"], wounded_id.as_str());
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    let killed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-killed")
        .expect("Fatality kill event");
    assert_eq!(killed.payload["cardId"], "south-minion");
    assert_eq!(killed.payload["owner"], "south");
    assert_eq!(killed.payload["seat"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &wounded_id).is_none());
    assert_eq!(
        realm_unit(&finished, &healthy_id).expect("survivor")["damage"],
        0
    );
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == wounded_id.as_str())
    );
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("Fatality checkpoint");
    let serialized =
        serialize_game_checkpoint(&checkpoint).expect("serialized Fatality checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed Fatality checkpoint");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed Fatality session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0196_draw_spells_magic_draws_hidden_spellbook_cards() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-draw": magic(("drawSpells", json!(2)), 1),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({})),
        "south-site": site(false),
    });
    let encoded = manifest(196, &cards, &["north-draw"; 6], &["south-minion"; 6]);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let expected: Vec<_> = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .iter()
        .take(2)
        .map(|card| card["instanceId"].clone())
        .collect();
    assert_eq!(expected.len(), 2);
    let south_before = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_before["players"]["north"]["hand"]["spellbook"], 3);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    });
    let spell_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("draw Magic identity")
        .to_owned();
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "spell-drawn", "spell-drawn", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], spell_id);
    let after = state(&session);
    let hand = after["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand");
    assert_eq!(hand.len(), 4);
    for instance_id in &expected {
        assert!(
            hand.iter().any(|card| &card["instanceId"] == instance_id),
            "the drawn identities must enter the hidden Spellbook hand"
        );
    }
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    let south_after = session
        .public_view(Seat::South)
        .expect("South public view after the draws");
    assert_eq!(
        south_after["players"]["north"]["hand"]["spellbook"], 4,
        "the opponent sees only the new hand count"
    );
    assert_eq!(south_after["players"]["north"]["spellbookCount"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0197_draw_spells_magic_exhausts_then_loses_on_empty_library() {
    for remaining in 0..=1 {
        let mut cards = json!({
            "north-avatar": avatar(20),
            "north-draw": magic(("drawSpells", json!(2)), 1),
            "north-site": site(false),
            "south-avatar": avatar(20),
            "south-minion": minion(json!({})),
            "south-site": site(false),
        });
        let mut north_spells = vec!["north-draw", "north-draw", "north-draw"];
        if remaining > 0 {
            cards["north-filler"] = minion(json!({}));
            north_spells.extend(std::iter::repeat_n("north-filler", remaining));
        }
        let encoded = manifest(
            197 + u32::try_from(remaining).expect("small remaining count"),
            &cards,
            &north_spells,
            &["south-minion"; 6],
        );
        let mut session = opening_main(&encoded);
        let before = state(&session);
        let expected = before["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .clone();
        let (_, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
        });
        let kinds = event_types(&receipt);
        assert_eq!(kinds.first(), Some(&"magic-cast"));
        assert_eq!(
            kinds.iter().filter(|kind| **kind == "spell-drawn").count(),
            expected.len()
        );
        assert_eq!(kinds.last(), Some(&"game-ended"));
        assert!(kinds.contains(&"magic-resolved"));
        let after = state(&session);
        assert_eq!(after["players"]["north"]["spellbook"], json!([]));
        assert_eq!(after["terminal"]["reason"], "deck_empty");
        assert_eq!(after["terminal"]["loser"], "north");
        for card in expected {
            assert!(
                after["players"]["north"]["hand"]["spellbook"]
                    .as_array()
                    .expect("north hand")
                    .contains(&card)
            );
        }
        assert_exact_replay(&session);
    }
}

fn kill_target_minion_manifest(ward: bool) -> String {
    let mut south_minion = minion(json!({ "defense": 3 }));
    if ward {
        south_minion["ward"] = json!(true);
    }
    let cards = json!({
        "north-avatar": avatar(20),
        "north-kill": magic(("killTargetMinion", json!(true)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": south_minion,
        "south-site": site(false),
    });
    manifest(198, &cards, &["north-kill"; 6], &["south-minion"; 6])
}

fn kill_target_minion_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("kill-minion actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-kill"
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

fn stage_south_minion_at_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned enemy identity")
        .to_owned()
}

#[test]
fn rule_catalog_0198_kill_target_minion_destroys_a_healthy_minion_and_excludes_avatars() {
    let manifest = kill_target_minion_manifest(false);
    let mut session = opening_main(&manifest);
    let enemy_id = stage_south_minion_at_c1(&mut session);
    let before = state(&session);
    let north_avatar = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    let targets = kill_target_minion_targets(&session);
    assert_eq!(targets, [enemy_id.as_str()]);
    assert!(!targets.contains(&north_avatar));
    assert!(!targets.contains(&south_avatar));
    assert_eq!(
        realm_unit(&before, &enemy_id).expect("healthy enemy")["damage"],
        0
    );

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-kill"
    });
    assert_eq!(descriptor["target"]["instanceId"], enemy_id.as_str());
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-killed",
            "minion-died",
            "magic-resolved"
        ]
    );
    let killed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-killed")
        .expect("unconditional kill event");
    assert_eq!(killed.payload["cardId"], "south-minion");
    assert_eq!(killed.payload["owner"], "south");
    assert_eq!(killed.payload["seat"], "south");
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "damage-dealt")
    );

    let finished = state(&session);
    assert!(realm_unit(&finished, &enemy_id).is_none());
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == enemy_id.as_str())
    );
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("kill-minion checkpoint");
    let serialized =
        serialize_game_checkpoint(&checkpoint).expect("serialized kill-minion checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed kill-minion checkpoint");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed kill-minion session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0199_kill_target_minion_ward_absorbs_the_kill() {
    let manifest = kill_target_minion_manifest(true);
    let mut session = opening_main(&manifest);
    let enemy_id = stage_south_minion_at_c1(&mut session);
    assert_eq!(kill_target_minion_targets(&session), [enemy_id.as_str()]);
    assert_eq!(
        realm_unit(&state(&session), &enemy_id).expect("warded enemy")["warded"],
        true
    );

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-kill"
            && descriptor["target"]["instanceId"] == enemy_id.as_str()
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let broken = receipt
        .events
        .iter()
        .find(|event| event.event_type == "ward-broken")
        .expect("Ward absorption");
    assert_eq!(broken.payload["instanceId"], enemy_id.as_str());
    assert_eq!(broken.payload["seat"], "south");
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-killed" || event.event_type == "minion-died")
    );

    let after = state(&session);
    let survivor = realm_unit(&after, &enemy_id).expect("Ward survivor");
    assert_eq!(survivor["warded"], false);
    assert_eq!(survivor["damage"], 0);
    assert!(
        !after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == enemy_id.as_str())
    );
    assert_exact_replay(&session);
}

fn draw_sites_manifest(seed: u32, atlas_count: usize) -> String {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-draw": magic(("drawSites", json!(2)), 1),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({})),
        "south-site": site(false),
    });
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "magic-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-magic-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; atlas_count],
                "avatar": "north-avatar",
                "spellbook": vec!["north-draw"; 6],
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

#[test]
fn rule_catalog_0200_draw_sites_magic_draws_hidden_atlas_cards() {
    let encoded = draw_sites_manifest(200, 6);
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let expected: Vec<_> = before["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .iter()
        .take(2)
        .map(|card| card["instanceId"].clone())
        .collect();
    assert_eq!(expected.len(), 2);
    let south_before = session.public_view(Seat::South).expect("South public view");
    assert_eq!(south_before["players"]["north"]["hand"]["atlas"], 2);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
    });
    let spell_id = descriptor["cardInstanceId"]
        .as_str()
        .expect("draw Magic identity")
        .to_owned();
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-drawn", "site-drawn", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(receipt.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(receipt.events[2].payload["sourceInstanceId"], spell_id);
    let after = state(&session);
    let hand = after["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas hand");
    assert_eq!(hand.len(), 4);
    for instance_id in &expected {
        assert!(
            hand.iter().any(|card| &card["instanceId"] == instance_id),
            "the drawn identities must enter the hidden Atlas hand"
        );
    }
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == spell_id)
    );
    let south_after = session
        .public_view(Seat::South)
        .expect("South public view after the draws");
    assert_eq!(
        south_after["players"]["north"]["hand"]["atlas"], 4,
        "the opponent sees only the new Atlas hand count"
    );
    assert_eq!(south_after["players"]["north"]["atlasCount"], 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0201_draw_sites_magic_exhausts_then_loses_on_empty_library() {
    for remaining in 0..=1 {
        let encoded = draw_sites_manifest(
            201 + u32::try_from(remaining).expect("small remaining count"),
            3 + remaining,
        );
        let mut session = opening_main(&encoded);
        let before = state(&session);
        let expected = before["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .clone();
        let (_, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-draw"
        });
        let kinds = event_types(&receipt);
        assert_eq!(kinds.first(), Some(&"magic-cast"));
        assert_eq!(
            kinds.iter().filter(|kind| **kind == "site-drawn").count(),
            expected.len()
        );
        assert_eq!(kinds.last(), Some(&"game-ended"));
        assert!(kinds.contains(&"magic-resolved"));
        let after = state(&session);
        assert_eq!(after["players"]["north"]["atlas"], json!([]));
        assert_eq!(after["terminal"]["reason"], "deck_empty");
        assert_eq!(after["terminal"]["loser"], "north");
        for card in expected {
            assert!(
                after["players"]["north"]["hand"]["atlas"]
                    .as_array()
                    .expect("north Atlas hand")
                    .contains(&card)
            );
        }
        assert_exact_replay(&session);
    }
}

fn bounce_manifest(ward: bool) -> String {
    let mut south_minion = minion(json!({ "defense": 3 }));
    if ward {
        south_minion["ward"] = json!(true);
    }
    let cards = json!({
        "north-avatar": avatar(20),
        "north-bounce": magic(("returnTargetMinionToOwnerHand", json!(true)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": south_minion,
        "south-site": site(false),
    });
    manifest(202, &cards, &["north-bounce"; 6], &["south-minion"; 6])
}

fn bounce_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("bounce actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-bounce"
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

#[test]
fn rule_catalog_0202_bounce_returns_a_healthy_minion_to_its_owners_hand() {
    let encoded = bounce_manifest(false);
    let mut session = opening_main(&encoded);
    let enemy_id = stage_south_minion_at_c1(&mut session);
    let before = state(&session);
    let north_avatar = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let south_avatar = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();
    let south_hand_before = before["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South Spellbook hand")
        .len();
    assert_eq!(bounce_targets(&session), [enemy_id.as_str()]);
    assert!(!bounce_targets(&session).contains(&north_avatar));
    assert!(!bounce_targets(&session).contains(&south_avatar));

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-bounce"
    });
    assert_eq!(descriptor["target"]["instanceId"], enemy_id.as_str());
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "minion-returned-to-hand", "magic-resolved"]
    );
    let returned = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-returned-to-hand")
        .expect("bounce return event");
    assert_eq!(returned.payload["cardId"], "south-minion");
    assert_eq!(returned.payload["instanceId"], enemy_id.as_str());
    assert_eq!(returned.payload["owner"], "south");
    assert_eq!(returned.payload["seat"], "south");
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-died"
                || event.event_type == "minion-killed"
                || event.event_type == "damage-dealt")
    );

    let finished = state(&session);
    assert!(realm_unit(&finished, &enemy_id).is_none());
    assert!(
        finished["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == enemy_id.as_str())
    );
    assert_eq!(
        finished["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .len(),
        south_hand_before + 1
    );
    assert!(
        !finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == enemy_id.as_str())
    );
    let north_view = session
        .public_view(Seat::North)
        .expect("North public view after the bounce");
    assert_eq!(
        north_view["players"]["south"]["hand"]["spellbook"],
        south_hand_before + 1,
        "the opponent sees only the new Spellbook hand count"
    );
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("bounce checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized bounce checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed bounce checkpoint");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed bounce session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0203_bounce_ward_absorbs_the_return() {
    let encoded = bounce_manifest(true);
    let mut session = opening_main(&encoded);
    let enemy_id = stage_south_minion_at_c1(&mut session);
    assert_eq!(bounce_targets(&session), [enemy_id.as_str()]);
    assert_eq!(
        realm_unit(&state(&session), &enemy_id).expect("warded enemy")["warded"],
        true
    );
    let hand_before = state(&session)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South Spellbook hand")
        .len();

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bounce"
            && descriptor["target"]["instanceId"] == enemy_id.as_str()
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "ward-broken", "magic-resolved"]
    );
    let broken = receipt
        .events
        .iter()
        .find(|event| event.event_type == "ward-broken")
        .expect("Ward absorption");
    assert_eq!(broken.payload["instanceId"], enemy_id.as_str());
    assert_eq!(broken.payload["seat"], "south");
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-returned-to-hand"
                || event.event_type == "minion-banished"
                || event.event_type == "minion-died")
    );

    let after = state(&session);
    let survivor = realm_unit(&after, &enemy_id).expect("Ward survivor");
    assert_eq!(survivor["warded"], false);
    assert!(
        !after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == enemy_id.as_str())
    );
    assert_eq!(
        after["players"]["south"]["hand"]["spellbook"]
            .as_array()
            .expect("South Spellbook hand")
            .len(),
        hand_before
    );
    assert_exact_replay(&session);
}

fn destroy_site_manifest(seed: u32, protected: bool) -> String {
    let mut south_site = site(false);
    if protected {
        south_site["cannotBeMovedDestroyedOrModified"] = json!(true);
    }
    let cards = json!({
        "north-avatar": avatar(20),
        "north-destroy": magic(("destroyTargetSite", json!(true)), 0),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({})),
        "south-site": south_site,
    });
    manifest(seed, &cards, &["north-destroy"; 6], &["south-minion"; 6])
}

fn stage_south_site_at_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let south_site_id = state(session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("South site identity")
        .to_owned();
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    south_site_id
}

fn destroy_site_targets(session: &Session) -> Vec<(String, String)> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("destroy-site actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-destroy"
        })
        .filter_map(|action| {
            Some((
                action.descriptor["targetLocation"]["cell"]
                    .as_str()?
                    .to_owned(),
                action.descriptor["targetSiteInstanceId"]
                    .as_str()?
                    .to_owned(),
            ))
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

#[test]
fn rule_catalog_0207_destroy_target_site_replaces_the_site_with_rubble() {
    let encoded = destroy_site_manifest(207, false);
    let mut session = opening_main(&encoded);
    let north_site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("North site identity")
        .to_owned();
    let south_site_id = stage_south_site_at_c1(&mut session);
    let before = state(&session);
    assert_eq!(
        destroy_site_targets(&session),
        [
            ("C1".to_owned(), south_site_id.clone()),
            ("C4".to_owned(), north_site_id.clone()),
        ]
    );
    assert_eq!(before["realm"]["sites"]["C1"]["cardId"], "south-site");
    assert_eq!(before["realm"]["sites"]["C1"]["rubble"], Value::Null);

    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
    });
    assert_eq!(
        descriptor["targetLocation"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "site-destroyed",
            "rubble-created",
            "magic-resolved"
        ]
    );
    let destroyed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-destroyed")
        .expect("site destruction");
    assert_eq!(destroyed.payload["cell"], "C1");
    assert_eq!(destroyed.payload["instanceId"], south_site_id);
    assert_eq!(destroyed.payload["owner"], "south");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    let rubble = receipt
        .events
        .iter()
        .find(|event| event.event_type == "rubble-created")
        .expect("Rubble creation");
    assert_eq!(rubble.payload["cell"], "C1");
    assert_eq!(
        rubble.payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "damage-dealt"
                || event.event_type == "minion-died"
                || event.event_type == "card-discarded")
    );

    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"]["rubble"], true);
    assert_eq!(
        after["realm"]["sites"]["C1"]["instanceId"],
        rubble.payload["instanceId"]
    );
    assert_eq!(after["realm"]["sites"]["C4"]["instanceId"], north_site_id);
    assert_eq!(after["players"]["south"]["avatar"]["location"], "C1");
    assert!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == south_site_id)
    );
    assert_eq!(
        destroy_site_targets(&session),
        [("C4".to_owned(), north_site_id)]
    );
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("destroy-site checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized destroy-site");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed destroy-site");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed destroy-site session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0208_destroy_target_site_is_prevented_on_a_protected_site() {
    let encoded = destroy_site_manifest(208, true);
    let mut session = opening_main(&encoded);
    let south_site_id = stage_south_site_at_c1(&mut session);
    let before = state(&session);
    let south_site = before["realm"]["sites"]["C1"].clone();
    assert_eq!(south_site["instanceId"], south_site_id);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == south_site_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-destruction-prevented", "magic-resolved"]
    );
    let prevented = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-destruction-prevented")
        .expect("protected-site prevention");
    assert_eq!(prevented.payload["cell"], "C1");
    assert_eq!(prevented.payload["instanceId"], south_site_id);
    assert_eq!(prevented.payload["owner"], "south");
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed"
                || event.event_type == "rubble-created")
    );

    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C1"], south_site);
    assert!(
        !after["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == south_site_id)
    );
    assert_exact_replay(&session);
}
