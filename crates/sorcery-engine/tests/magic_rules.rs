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
