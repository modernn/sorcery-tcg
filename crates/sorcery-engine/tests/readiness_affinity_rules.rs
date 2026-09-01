use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::session::{Session, StepResult};

fn minion(attack: u8, defense: u8) -> Value {
    json!({
        "attack": attack,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn scenario_manifest(
    seed: u32,
    extra_cards: &Value,
    north_spellbook: &[&str],
    south_spellbook: &[&str],
) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut cards = json!({
        "north-avatar": avatar,
        "north-site": site,
        "south-avatar": avatar,
        "south-site": site,
    });
    cards.as_object_mut().expect("card definitions").extend(
        extra_cards
            .as_object()
            .expect("extra card definitions")
            .clone(),
    );
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "readiness-affinity-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-readiness-affinity-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 4],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical synthetic manifest")
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
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

fn ready_movement_session(
    seed: u32,
    mut mover: Value,
    seat: Seat,
    summon_cell: &str,
    destination_defender: bool,
) -> (Session, String) {
    mover["summonToAnySite"] = json!(true);
    let manifest = scenario_manifest(
        seed,
        &json!({
            "north-mover": mover.clone(),
            "south-mover": mover,
        }),
        &["north-mover"; 8],
        &["south-mover"; 8],
    );
    let mut session = Session::new(&manifest).expect("valid movement scenario");
    keep(&mut session);
    keep(&mut session);
    for (site_cell, draw_zone) in [
        ("C4", None),
        ("C1", Some("spellbook")),
        ("C3", Some("spellbook")),
        ("C2", Some("spellbook")),
    ] {
        if let Some(zone) = draw_zone {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == zone
            });
        }
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == site_cell
        });
        if seat == Seat::South && site_cell == "C2" {
            let (summon, _) = accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "south-mover"
                    && descriptor["cell"] == summon_cell
            });
            let instance_id = summon["cardInstanceId"]
                .as_str()
                .expect("mover identity")
                .to_owned();
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "B3"
            });
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            return (session, instance_id);
        }
        if destination_defender && seat == Seat::North && site_cell == "C2" {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "south-mover"
                    && descriptor["cell"] == "C1"
            });
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    }
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B3"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-mover"
            && descriptor["cell"] == summon_cell
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("mover identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    (session, instance_id)
}

fn movement_paths(session: &Session, instance_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("movement actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .map(|action| {
            action.descriptor["path"]
                .as_array()
                .expect("movement path")
                .iter()
                .map(|location| location["cell"].as_str().expect("path cell"))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect()
}

#[test]
fn movement_bonus_one_should_issue_exact_returning_surface_paths() {
    let mut mover = minion(2, 3);
    mover["movementBonus"] = json!(1);
    let (mut session, instance_id) = ready_movement_session(53, mover, Seat::North, "C2", false);
    assert_eq!(
        movement_paths(&session, &instance_id),
        [
            "C2,C1,C2", "C2,C1", "C2,C3,B3", "C2,C3,C2", "C2,C3,C4", "C2,C3", "C2",
        ]
    );
    let action = session
        .legal_actions()
        .expect("movement actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == instance_id
                && action.descriptor["path"].as_array().is_some_and(|path| {
                    path.iter()
                        .map(|location| location["cell"].as_str().expect("path cell"))
                        .eq(["C2", "C3", "C4"])
                })
        })
        .expect("exact two-step action");
    assert!(action.label.ends_with(" C2 → C3 → C4"));
    let (_, receipt) = accept_where(&mut session, |descriptor| descriptor == &action.descriptor);
    assert_eq!(receipt.events[0].event_type, "move-and-attack-activated");
    assert_eq!(receipt.events[0].payload["steps"], 2);
    assert_eq!(state(&session)["realm"]["units"][0]["location"], "C4");
    assert_eq!(state(&session)["pendingCombat"]["cell"], "C4");
    assert_exact_replay(&session);
}

#[test]
fn movement_bonus_two_should_reject_reused_directed_edges_and_attack_after_three_steps() {
    let mut mover = minion(2, 3);
    mover["movementBonus"] = json!(2);
    let (mut session, instance_id) = ready_movement_session(125, mover, Seat::North, "C2", true);
    let paths = movement_paths(&session, &instance_id);
    assert!(paths.contains(&"C2,C3,C2".to_owned()));
    assert!(paths.contains(&"C2,C3,C4,C3".to_owned()));
    assert!(!paths.contains(&"C2,C3,C2,C3".to_owned()));
    assert!(paths.iter().all(|path| path.split(',').count() <= 4));

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["path"].as_array().is_some_and(|path| {
                path.iter()
                    .map(|location| location["cell"].as_str().expect("path cell"))
                    .eq(["C2", "C3", "C2", "C1"])
            })
    });
    assert_eq!(receipt.events[0].payload["steps"], 3);
    let defender_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["controller"] == "south" && unit["location"] == "C1")
        .and_then(|unit| unit["instanceId"].as_str())
        .expect("destination defender")
        .to_owned();
    assert!(
        session
            .legal_actions()
            .expect("final-cell attacks")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "declare-attack"
                    && action.descriptor["target"]["kind"] == "minion"
                    && action.descriptor["target"]["instanceId"] == defender_id
            })
    );
    assert_exact_replay(&session);
}

#[test]
fn sideways_restriction_should_filter_every_non_sideways_step() {
    let mut mover = minion(3, 3);
    mover["movementBonus"] = json!(1);
    mover["movesOnlySideways"] = json!(true);
    let (mut session, instance_id) = ready_movement_session(127, mover, Seat::North, "C3", false);
    assert_eq!(
        movement_paths(&session, &instance_id),
        ["C3,B3,C3", "C3,B3", "C3"]
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["path"].as_array().is_some_and(|path| {
                path.iter()
                    .map(|location| location["cell"].as_str().expect("path cell"))
                    .eq(["C3", "B3"])
            })
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(state(&session)["realm"]["units"][0]["location"], "B3");
    assert_exact_replay(&session);
}

#[test]
fn forward_restriction_should_use_seat_direction_and_top_bottom_wrap() {
    let mut mover = minion(5, 5);
    mover["connectsTopBottom"] = json!(true);
    mover["movementBonus"] = json!(1);
    mover["movesOnlyForward"] = json!(true);
    let (mut north, north_id) =
        ready_movement_session(142, mover.clone(), Seat::North, "C3", false);
    assert_eq!(
        movement_paths(&north, &north_id),
        ["C3,C2,C1", "C3,C2", "C3"]
    );
    accept_where(&mut north, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == north_id
            && descriptor["path"].as_array().is_some_and(|path| {
                path.iter()
                    .map(|location| location["cell"].as_str().expect("path cell"))
                    .eq(["C3", "C2", "C1"])
            })
    });
    accept_where(&mut north, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut north, |descriptor| {
        descriptor["kind"] == "close-intercept"
    });
    accept_where(&mut north, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut north, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut north, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut north, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let north_edge_paths = movement_paths(&north, &north_id);
    assert!(north_edge_paths.contains(&"C1,C4".to_owned()));
    assert!(
        !north_edge_paths
            .iter()
            .any(|path| path.starts_with("C1,C2"))
    );

    let (mut south, south_id) = ready_movement_session(143, mover, Seat::South, "C4", false);
    let south_edge_paths = movement_paths(&south, &south_id);
    assert!(south_edge_paths.contains(&"C4,C1".to_owned()));
    assert!(
        !south_edge_paths
            .iter()
            .any(|path| path.starts_with("C4,C3"))
    );
    accept_where(&mut south, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == south_id
            && descriptor["path"].as_array().is_some_and(|path| {
                path.iter()
                    .map(|location| location["cell"].as_str().expect("path cell"))
                    .eq(["C4", "C1"])
            })
    });
    accept_where(&mut south, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_eq!(state(&south)["realm"]["units"][0]["location"], "C1");
    assert_exact_replay(&north);
    assert_exact_replay(&south);
}

#[test]
fn charge_should_allow_immediate_move_and_attack_without_clearing_sickness() {
    let mut charge = minion(1, 2);
    charge["charge"] = json!(true);
    let manifest = scenario_manifest(
        52,
        &json!({
            "north-charge": charge,
            "south-minion": minion(1, 2),
        }),
        &["north-charge"; 4],
        &["south-minion"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid Charge scenario");
    keep(&mut session);
    keep(&mut session);
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-charge"
            && descriptor["cell"] == "C4"
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("Charge instance identity")
        .to_owned();
    let (_, movement) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });

    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("summoned Charge minion");
    assert_eq!(unit["summoningSickness"], true);
    assert_eq!(unit["location"], "C3");
    assert_eq!(movement.events[0].event_type, "move-and-attack-activated");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_exact_replay(&session);
}

#[test]
fn restricted_attacker_should_preserve_unit_targets_and_filter_site_target() {
    let mut restricted = minion(1, 3);
    restricted["cannotAttackSites"] = json!(true);
    restricted["charge"] = json!(true);
    restricted["summonToAnySite"] = json!(true);
    let manifest = scenario_manifest(
        83,
        &json!({
            "north-restricted": restricted,
            "south-target": minion(1, 3),
        }),
        &["north-restricted"; 4],
        &["south-target"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid restricted attacker scenario");
    keep(&mut session);
    keep(&mut session);
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
    let (target_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "south-target"
    });
    let target_instance_id = target_summon["cardInstanceId"]
        .as_str()
        .expect("target instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let (attacker_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-restricted"
            && descriptor["cell"] == "C1"
    });
    let attacker_instance_id = attacker_summon["cardInstanceId"]
        .as_str()
        .expect("attacker instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_instance_id
            && descriptor["to"]["cell"] == "C1"
    });
    let targets: Vec<_> = session
        .legal_actions()
        .expect("attack actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "declare-attack")
        .map(|action| action.descriptor["target"]["kind"].clone())
        .collect();
    assert!(targets.contains(&json!("avatar")));
    assert!(targets.contains(&json!("minion")));
    assert!(!targets.contains(&json!("site")));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_instance_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend"
    });
    assert_exact_replay(&session);
}

#[test]
fn provider_affinity_should_stop_immediately_when_provider_dies() {
    let mut provider = minion(0, 1);
    provider["provides"] = json!("earth");
    let mut threshold_minion = minion(1, 2);
    threshold_minion["thresholds"]["earth"] = json!(2);
    let mut killer = minion(1, 2);
    killer["charge"] = json!(true);
    killer["summonToAnySite"] = json!(true);
    let manifest = scenario_manifest(
        95,
        &json!({
            "north-provider": provider,
            "north-threshold": threshold_minion,
            "south-killer": killer,
        }),
        &[
            "north-provider",
            "north-provider",
            "north-threshold",
            "north-threshold",
        ],
        &["south-killer"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid provider scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (provider_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-provider"
    });
    let provider_instance_id = provider_summon["cardInstanceId"]
        .as_str()
        .expect("provider instance identity")
        .to_owned();
    assert!(
        session
            .legal_actions()
            .expect("provider affinity actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-threshold")
    );
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (killer_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-killer"
            && descriptor["cell"] == "C4"
    });
    let killer_instance_id = killer_summon["cardInstanceId"]
        .as_str()
        .expect("killer instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == killer_instance_id
            && descriptor["to"]["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == provider_instance_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend"
    });
    assert!(fight.events.iter().any(|event| {
        event.event_type == "minion-died" && event.payload["instanceId"] == provider_instance_id
    }));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });

    assert!(
        !session
            .legal_actions()
            .expect("post-death actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-threshold")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0094_granary_rats_suppress_site_threshold_while_enabled() {
    let threshold_summon_is_legal = |session: &Session| {
        session
            .legal_actions()
            .expect("threshold actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "summon-minion"
                    && action.descriptor["cardId"] == "north-threshold"
            })
    };
    let prepare = |seed, rat_disabled, site_protected| {
        let mut threshold_minion = minion(1, 2);
        threshold_minion["thresholds"]["earth"] = json!(1);
        let mut granary_rats = minion(0, 1);
        granary_rats["siteProvidesNoThreshold"] = json!(true);
        granary_rats["summonToAnySite"] = json!(true);
        if rat_disabled {
            granary_rats["genesisDisableSelfUntilDamaged"] = json!(true);
        }
        let mut extra_cards = json!({
            "north-threshold": threshold_minion,
            "south-granary-rats": granary_rats,
        });
        if site_protected {
            extra_cards["north-site"] = json!({
                "cardType": "site",
                "cannotBeMovedDestroyedOrModified": true,
                "elements": ["earth"],
            });
        }
        let manifest = scenario_manifest(
            seed,
            &extra_cards,
            &["north-threshold"; 4],
            &["south-granary-rats"; 4],
        );
        let mut session = Session::new(&manifest).expect("valid Granary Rats scenario");
        keep(&mut session);
        keep(&mut session);
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
        });
        assert!(
            threshold_summon_is_legal(&session),
            "the site should provide its threshold before Granary Rats arrives"
        );
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-granary-rats"
                && descriptor["cell"] == "C4"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        session
    };

    let active = prepare(94, false, false);
    let active_rat = &state(&active)["realm"]["units"][0];
    assert_eq!(active_rat["controller"], "south");
    assert_eq!(active_rat["location"], "C4");
    assert!(
        !threshold_summon_is_legal(&active),
        "an enabled opposing Granary Rats should suppress the occupied site"
    );
    assert_exact_replay(&active);

    let disabled = prepare(940, true, false);
    assert_eq!(
        state(&disabled)["realm"]["units"][0]["disabledUntilDamaged"],
        true
    );
    assert!(
        threshold_summon_is_legal(&disabled),
        "a disabled Granary Rats should not suppress the occupied site"
    );
    assert_exact_replay(&disabled);

    let protected = prepare(941, false, true);
    assert!(
        threshold_summon_is_legal(&protected),
        "a protected site should retain its threshold"
    );
    assert_exact_replay(&protected);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "direct scenario proof keeps the readiness and expiration sequence visible"
)]
fn mana_activation_should_require_readiness_tap_add_printed_mana_reveal_and_expire() {
    let mut mana_source = minion(1, 2);
    mana_source["charge"] = json!(true);
    mana_source["stealth"] = json!(true);
    mana_source["tapForMana"] = json!(2);
    let manifest = scenario_manifest(
        39,
        &json!({
            "north-mana-source": mana_source,
            "south-minion": minion(1, 2),
        }),
        &["north-mana-source"; 4],
        &["south-minion"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid mana activation scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-mana-source"
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("mana source identity")
        .to_owned();

    assert_eq!(state(&session)["realm"]["units"][0]["stealthed"], true);
    assert!(
        !session
            .legal_actions()
            .expect("summoning-sick actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "activate-mana")
    );

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

    let before_mana = state(&session)["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let action = session
        .legal_actions()
        .expect("ready mana actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "activate-mana"
                && action.descriptor["amount"] == 2
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .expect("printed mana activation");
    assert!(action.label.ends_with(" for 2 mana"));
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative mana activation")
    else {
        panic!("engine-issued mana activation must be accepted");
    };
    let after = state(&session);
    let source = after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("mana source");

    assert_eq!(after["players"]["north"]["mana"], before_mana + 2);
    assert_eq!(source["tapped"], true);
    assert_eq!(source["stealthed"], false);
    assert_eq!(source["lastInteractedTurn"], 3);
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["mana-activated", "stealth-lost"]
    );
    assert_eq!(
        receipt.events[0].payload,
        json!({ "amount": 2, "seat": "north", "unitInstanceId": instance_id })
    );
    assert!(
        !session
            .legal_actions()
            .expect("post-activation actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "activate-mana")
    );

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let refreshed = state(&session);
    assert_eq!(refreshed["players"]["north"]["mana"], 1);
    assert_eq!(refreshed["realm"]["units"][0]["tapped"], false);
    assert_exact_replay(&session);
}

#[test]
fn unconditional_end_turn_stealth_should_gain_before_turn_events_without_duplicates() {
    let mut fox = minion(1, 2);
    fox["gainsStealthAtEndOfTurn"] = json!(true);
    fox["stealth"] = json!(true);
    fox["tapForMana"] = json!(1);
    let manifest = scenario_manifest(
        126,
        &json!({
            "north-fox": fox,
            "south-minion": minion(1, 2),
        }),
        &["north-fox"; 4],
        &["south-minion"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid end-turn Stealth scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-fox"
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("Stealth source identity")
        .to_owned();
    assert_eq!(state(&session)["realm"]["units"][0]["stealthed"], true);

    let (_, first_end) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !first_end
            .events
            .iter()
            .any(|event| event.event_type == "stealth-gained")
    );
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, activation) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == instance_id
    });
    assert_eq!(
        activation
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["mana-activated", "stealth-lost"]
    );
    let (_, regained) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        regained
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["stealth-gained", "turn-ended", "turn-started"]
    );
    assert_eq!(
        regained.events[0].payload,
        json!({ "instanceId": instance_id, "seat": "north" })
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, no_duplicate) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !no_duplicate
            .events
            .iter()
            .any(|event| event.event_type == "stealth-gained")
    );
    assert_eq!(state(&session)["realm"]["units"][0]["stealthed"], true);
    assert_exact_replay(&session);
}

#[test]
fn connected_top_bottom_should_wrap_only_the_minion() {
    let mut connector = minion(1, 2);
    connector["connectsTopBottom"] = json!(true);
    let manifest = scenario_manifest(
        112,
        &json!({
            "north-connector": connector,
            "south-minion": minion(1, 2),
        }),
        &["north-connector"; 4],
        &["south-minion"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid connected-edge scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-connector"
    });
    let minion_id = summon["cardInstanceId"]
        .as_str()
        .expect("connector identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let actions = session.legal_actions().expect("connected-edge actions");
    let snapshot = state(&session);
    let avatar_id = snapshot["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar identity");
    assert!(actions.iter().any(|action| {
        action.descriptor["kind"] == "move-and-attack"
            && action.descriptor["unitInstanceId"] == minion_id
            && action.descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "C1", "region": "surface" },
                ])
    }));
    assert!(!actions.iter().any(|action| {
        action.descriptor["kind"] == "move-and-attack"
            && action.descriptor["unitInstanceId"] == avatar_id
            && action.descriptor["to"]["cell"] == "C1"
    }));

    let (movement, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == minion_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C1"
    });
    assert_eq!(
        receipt.events[0].payload,
        json!({
            "from": movement["from"].clone(),
            "path": movement["path"].clone(),
            "seat": "north",
            "steps": 1,
            "to": movement["to"].clone(),
            "unitInstanceId": minion_id,
        })
    );
    assert_eq!(state(&session)["phase"], "attack");
    assert_eq!(state(&session)["realm"]["units"][0]["location"], "C1");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_exact_replay(&session);
}
