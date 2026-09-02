use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
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
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C2"
    });
    let (warded_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-warded"
            && descriptor["cell"] == "C2"
    });
    let (stealth_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
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
            && descriptor["cardId"] == "north-deathrite"
            && descriptor["cell"] == "C4"
    });
    let (burrower_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
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
            && descriptor["cardId"] == "south-stealth"
            && descriptor["cell"] == "C1"
    });
    let stealth_id = stealth_summon["cardInstanceId"]
        .as_str()
        .expect("Stealth identity")
        .to_owned();
    let (warded_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
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
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
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

#[test]
fn targeted_magic_allows_friendly_stealth_and_excludes_enemy_active_stealth() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-magic": magic(("damageTargetUnit", json!(1)), 0),
        "north-minion": minion(json!({ "stealth": true })),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({ "stealth": true })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-magic",
        "north-magic",
        "north-magic",
        "north-minion",
        "north-minion",
        "north-minion",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-minion"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("target visibility candidate");
            let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand")
                .clone();
            ["north-magic", "north-minion"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("bounded seed with Magic and friendly Stealth");
    let mut session = opening_main(&manifest);
    let (friendly, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-minion"
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
    let (enemy, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "south-minion"
    });
    let enemy_id = enemy["cardInstanceId"]
        .as_str()
        .expect("enemy Stealth identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let target_ids: Vec<_> = session
        .legal_actions()
        .expect("Stealth target actions")
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
        .collect();
    assert!(target_ids.contains(&friendly_id));
    assert!(!target_ids.contains(&enemy_id));
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
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
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
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let nearby_id = nearby["cardInstanceId"]
        .as_str()
        .expect("nearby target identity")
        .to_owned();
    let (distant, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
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
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-loss"
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
