//! Synthetic direct proofs of the bounded layer slice; official receipts remain private.
use super::*;
use crate::canonical::{IdentityHash, identity_hash};
use crate::facts::{Element, parse_card_definition};
use crate::game::{
    ArtifactPlacement, AuraPosition, CardId, CardInstance, CardSource, Phase, Seat, SitePosition,
    SummonPlacement, UnitTarget,
};
use crate::synthetic::selfplay_manifest_with;
use serde_json::{Value, json};

fn id(label: &str) -> IdentityHash {
    identity_hash(&json!({"continuous-proof":label})).unwrap()
}
fn thresholds() -> Value {
    json!({"air":0,"earth":0,"fire":0,"water":0})
}
fn fixture() -> Game {
    let manifest = selfplay_manifest_with(9414, |m| {
        m["cards"]["north-spell-1"] = json!({"cardType":"minion", "attack":3,"defense":3,
            "manaCost":0,"thresholds":thresholds()});
        m["cards"]["captured-magic"] = json!({"cardType":"magic", "damageTargetUnit":1,
            "manaCost":0,"thresholds":thresholds()});
        m["cards"]["spirit"] = json!({"cardType":"minion", "attack":4,"defense":4,
            "manaCost":0,"thresholds":thresholds(),"disabledUnlessAdjacentAlliedSpellcaster":true});
        m["cards"]["conditional"] = json!({"cardType":"minion", "attack":1,"defense":1,
            "manaCost":0,"thresholds":thresholds(),"gainsPowerRangedAndSpellcasterAtopTower":2});
        m["cards"]["relic"] = json!({"cardType":"artifact","bearerPowerBonus":1,
            "bearerSpellcasterElements":["water"],"manaCost":1,"thresholds":thresholds()});
        m["cards"]["fire-relic"] = json!({"cardType":"artifact","bearerPowerBonus":1,
            "bearerSpellcasterElements":["fire"],"manaCost":1,"thresholds":thresholds()});
        m["cards"]["source-silence"] = json!({"cardType":"aura","silenceAffectedSitesAndArtifactsThere":true,
            "manaCost":0,"thresholds":thresholds()});
        m["cards"]["recipient-silence"] = json!({"cardType":"aura","silenceMinionsAtAffectedSites":true,
            "manaCost":0,"thresholds":thresholds()});
        m["cards"]["tower"] = json!({"cardType":"site","elements":["air"],"subtypes":["Tower"],
            "genesisGainManaIfOnlyControlledCopy":1});
        for (name, facts) in [
            ("lethal", json!({"grantsBearerLethal":true})),
            (
                "strike",
                json!({"bearerUnitStrike":{"damageBonus":1,"firstStrike":true,"destroyAfterStrike":true}}),
            ),
            (
                "luck",
                json!({"bearerControllerChoosesExtraRandomOutcome":true}),
            ),
            ("forced", json!({"nearbyMinionsMustAttackIfAble":true})),
            (
                "double",
                json!({"nearbyStrikesAgainstUnitsDealDoubleDamage":true}),
            ),
            (
                "ballista",
                json!({"tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps":3}),
            ),
            (
                "trebuchet",
                json!({"tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps":true}),
            ),
            (
                "roll",
                json!({"tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath":4}),
            ),
            (
                "control",
                json!({"sacrificeThisToGainControlOfTargetEnemyMinionHereUntilBearerLeaves":true}),
            ),
            (
                "return",
                json!({"returnToOwnerHandAfterEachTurn":true,"bearerPowerBonus":1}),
            ),
        ] {
            let mut card = json!({"cardType":"artifact","manaCost":0,"thresholds":thresholds()});
            card.as_object_mut()
                .unwrap()
                .extend(facts.as_object().unwrap().clone());
            m["cards"][name] = card;
        }
        let names: Vec<_> = m["cards"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|name| !name.starts_with("north-") && !name.starts_with("south-"))
            .cloned()
            .collect();
        for name in names {
            let zone = if name == "tower" {
                "atlas"
            } else {
                "spellbook"
            };
            m["decks"]["north"][zone]
                .as_array_mut()
                .unwrap()
                .push(json!(name));
        }
    });
    let mut game = Game::from_manifest_json(&manifest).unwrap();
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    game.position.players[0].domain_established = true;
    game.position.players[1].domain_established = true;
    for cell in ["C3", "B3", "D3", "B2", "C2"] {
        let label = cell;
        let cell = Cell::parse(cell).unwrap();
        game.position.sites[cell.index()] = Some(SitePosition {
            card: card(&game, "tower", label),
            controller: Seat::North,
            warded: false,
            last_flight_turn: None,
        });
    }
    game
}
fn card(game: &Game, name: &str, label: &str) -> CardInstance {
    CardInstance {
        card_id: CardId(
            u16::try_from(game.rules.cards.iter().position(|c| c.id == name).unwrap()).unwrap(),
        ),
        instance_id: id(label),
        owner: Seat::North,
        realm_entry: 1,
        source: CardSource::Spellbook,
    }
}
fn unit(game: &Game, name: &str, label: &str, seat: Seat) -> UnitPosition {
    let mut unit = SummonPlacement {
        card: card(game, name, label),
        controller: seat,
        lance_count: 0,
        location: Cell::parse("C3").unwrap(),
        occupied_cells: None,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit();
    unit.summoning_sickness = false;
    unit
}
fn artifact(game: &Game, name: &str, label: &str, bearer: &UnitPosition) -> ArtifactPosition {
    ArtifactPosition {
        card: card(game, name, label),
        placement: ArtifactPlacement::Carried {
            bearer: UnitTarget::Minion {
                instance_id: bearer.card.instance_id.clone(),
                seat: bearer.controller,
            },
            cell: None,
        },
    }
}
fn silence(game: &mut Game, name: &str) {
    game.position.auras.push(AuraPosition {
        card: card(game, name, "aura"),
        cells: vec![Cell::parse("C3").unwrap()],
        controller: Seat::South,
        turn_counters: 0,
        visited_cells: std::collections::BTreeSet::default(),
    });
}
#[test]
fn parser_rejects_empty_null_duplicate_and_unknown_grants() {
    let game = fixture();
    let card = game
        .rules
        .cards
        .iter()
        .find(|c| c.id == "relic")
        .unwrap()
        .value
        .clone();
    for value in [
        json!([]),
        json!(null),
        json!(true),
        json!(1),
        json!(["water", "water"]),
        json!(["unknown"]),
    ] {
        let mut bad = card.clone();
        bad["bearerSpellcasterElements"] = value;
        assert!(parse_card_definition("bad", &bad).is_err());
    }
    let mut grant_only = card;
    grant_only
        .as_object_mut()
        .unwrap()
        .remove("bearerPowerBonus");
    assert!(parse_card_definition("grant-only", &grant_only).is_ok());
}
#[test]
fn restricted_union_duplicate_power_and_avatar_characteristic() {
    for seat in [Seat::North, Seat::South] {
        let mut game = fixture();
        let bearer = unit(&game, "north-spell-1", "bearer", seat);
        game.position.units.push(bearer.clone());
        for (name, label) in [("relic", "a"), ("relic", "b"), ("fire-relic", "c")] {
            game.position
                .artifacts
                .push(artifact(&game, name, label, &bearer));
        }
        let permission = EffectiveView::new(&game).caster(&bearer).unwrap().unwrap();
        assert!(permission.contains(Element::Water) && permission.contains(Element::Fire));
        assert!(!permission.contains(Element::Air));
        assert_eq!(game.spellcasters(seat).len(), 2);
        assert_eq!(
            game.carried_power_bonus(UnitKind::Minion, seat, &bearer.card.instance_id)
                .unwrap(),
            3
        );
        game.position.artifacts.remove(0);
        assert!(
            EffectiveView::new(&game)
                .caster(&bearer)
                .unwrap()
                .unwrap()
                .contains(Element::Water)
        );
        let avatar = &game.position.players[crate::game::seat_index(seat)].avatar;
        assert!(game.spellcaster_can_cast(
            seat,
            &avatar.card.instance_id,
            crate::facts::Thresholds::default()
        ));
    }
}
#[test]
fn source_silence_removes_both_but_recipient_silence_preserves_power_and_basic_actions() {
    for seat in [Seat::North, Seat::South] {
        let mut game = fixture();
        game.position.active_seat = seat;
        game.position.decision_seat = seat;
        let bearer = unit(&game, "conditional", "bearer", seat);
        game.position.units.push(bearer.clone());
        game.position
            .artifacts
            .push(artifact(&game, "relic", "a", &bearer));
        silence(&mut game, "recipient-silence");
        assert!(!game.minion_is_disabled(&bearer));
        assert!(game.minion_is_silenced(&bearer));
        assert!(EffectiveView::new(&game).caster(&bearer).is_none());
        assert_eq!(
            game.carried_power_bonus(UnitKind::Minion, seat, &bearer.card.instance_id)
                .unwrap(),
            1
        );
        assert!(!game.minion_is_ranged(
            &bearer,
            match &game.rules.cards[usize::from(bearer.card.card_id.0)].facts {
                CardFacts::Minion(f) => f,
                _ => unreachable!(),
            }
        ));
        let actions = game.legal_actions().unwrap();
        assert!(actions.iter().any(|a| matches!(&a.descriptor,crate::game::ActionDescriptor::MoveAndAttack{unit_instance_id,..} if unit_instance_id==&bearer.card.instance_id)));
        assert!(actions.iter().any(|a| matches!(
            &a.descriptor,
            crate::game::ActionDescriptor::DropArtifacts { .. }
        )));
        game.position.auras.clear();
        silence(&mut game, "source-silence");
        assert_eq!(
            game.carried_power_bonus(UnitKind::Minion, seat, &bearer.card.instance_id)
                .unwrap(),
            0
        );
        assert!(game.site_abilities_lost(bearer.location));
        assert!(game.minion_atop_tower(&bearer)); // Tower is a characteristic, its caster clause belongs to the minion.
        assert_eq!(EffectiveView::new(&game).caster(&bearer), Some(None));
        game.position.auras.clear();
        assert_eq!(
            game.carried_power_bonus(UnitKind::Minion, seat, &bearer.card.instance_id)
                .unwrap(),
            1
        );
        game.position.units[0].disabled_until_damaged = true;
        assert!(game.minion_is_disabled(&game.position.units[0]));
        assert!(
            EffectiveView::new(&game)
                .caster(&game.position.units[0])
                .is_none()
        );
    }
}
#[test]
fn conditional_disability_has_self_witness_but_never_bootstraps_unsupported_spirits() {
    let mut game = fixture();
    game.position.players[0].avatar.location = Cell::parse("A1").unwrap();
    let a = unit(&game, "spirit", "a", Seat::North);
    let mut b = unit(&game, "spirit", "b", Seat::North);
    b.location = Cell::parse("B3").unwrap();
    game.position.units = vec![a.clone(), b.clone()];
    assert!(game.minion_is_disabled(&a) && game.minion_is_disabled(&b));
    game.position
        .artifacts
        .push(artifact(&game, "relic", "coral", &a));
    assert!(!game.minion_is_disabled(&a) && !game.minion_is_disabled(&b));
    game.position.units[0].disabled_until_damaged = true;
    assert!(game.minion_is_disabled(&game.position.units[0]) && game.minion_is_disabled(&b));
    game.position.units[0].disabled_until_damaged = false;
    silence(&mut game, "source-silence");
    assert!(game.minion_is_disabled(&a) && game.minion_is_disabled(&b));
    game.position.auras.clear();
    silence(&mut game, "recipient-silence");
    assert!(!game.minion_is_disabled(&a)); // Conditional clause lost under Silence; basic actions persist.
    assert!(game.minion_is_disabled(&b)); // Silenced source is not a Spellcaster witness.
}
#[test]
fn source_gate_covers_ongoing_and_strike_capture_without_gating_characteristics() {
    let mut game = fixture();
    let bearer = unit(&game, "north-spell-1", "bearer", Seat::North);
    game.position.units.push(bearer.clone());
    for name in ["lethal", "strike", "luck", "forced", "double"] {
        game.position
            .artifacts
            .push(artifact(&game, name, name, &bearer));
    }
    assert!(
        game.carried_lethal(UnitKind::Minion, Seat::North, &bearer.card.instance_id)
            .unwrap()
    );
    assert_eq!(game.lucky_charm_count(Seat::North), 1);
    assert!(game.minion_is_nearby_must_attack_artifact(&bearer));
    let captured = game
        .combatant_strike_stats(UnitKind::Minion, Seat::North, &bearer.card.instance_id)
        .unwrap();
    assert_eq!(captured.consumed_artifacts.len(), 1);
    silence(&mut game, "source-silence");
    assert!(
        !game
            .carried_lethal(UnitKind::Minion, Seat::North, &bearer.card.instance_id)
            .unwrap()
    );
    assert_eq!(game.lucky_charm_count(Seat::North), 0);
    assert!(!game.minion_is_nearby_must_attack_artifact(&bearer));
    assert!(
        game.position
            .artifacts
            .iter()
            .all(|a| !game.artifact_doubles_nearby_unit_strikes(a))
    );
    assert!(
        !game
            .combatant_strikes_first(
                UnitKind::Minion,
                Seat::North,
                &bearer.card.instance_id,
                true
            )
            .unwrap()
    );
    assert!(
        game.combatant_strike_stats(UnitKind::Minion, Seat::North, &bearer.card.instance_id)
            .unwrap()
            .consumed_artifacts
            .is_empty()
    );
    assert!(game.artifact_facts(&game.position.artifacts[0]).is_ok());
    game.consume_strike_artifacts(
        &captured,
        &bearer.card.instance_id,
        &mut crate::game::OutcomeLog::Ignore,
    )
    .unwrap();
    assert!(
        !game
            .position
            .artifacts
            .iter()
            .any(|a| a.card.instance_id == id("strike"))
    );
}
#[test]
fn activation_gate_precedes_costs_and_does_not_remove_physical_pickup_drop() {
    let mut game = fixture();
    let bearer = unit(&game, "north-spell-1", "bearer", Seat::North);
    let helper = unit(&game, "north-spell-1", "helper", Seat::North);
    let enemy = unit(&game, "north-spell-1", "enemy", Seat::South);
    game.position.units = vec![bearer.clone(), helper, enemy];
    for name in ["ballista", "trebuchet", "roll", "control"] {
        game.position
            .artifacts
            .push(artifact(&game, name, name, &bearer));
    }
    let stale = game
        .artifact_damage_descriptors(Seat::North)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert!(
        !game
            .artifact_discard_area_damage_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    assert!(
        !game
            .artifact_roll_damage_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    assert!(
        !game
            .artifact_sacrifice_control_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    silence(&mut game, "source-silence");
    assert!(
        game.artifact_damage_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    assert!(
        game.artifact_discard_area_damage_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    assert!(
        game.artifact_roll_damage_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    assert!(
        game.artifact_sacrifice_control_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    let mut action = game.legal_actions().unwrap().pop().unwrap();
    action.descriptor = stale;
    assert!(game.apply_action(&action).is_err());
    assert!(game.position.units.iter().all(|u| !u.tapped));
    assert!(
        !game
            .drop_artifact_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    game.position.artifacts[0].placement = ArtifactPlacement::Loose {
        location: bearer.location,
        region: Region::Surface,
    };
    assert!(
        !game
            .pick_up_artifact_descriptors(Seat::North)
            .unwrap()
            .is_empty()
    );
    // Pure synthetic captured-work test: a source collected before suppression remains an incarnation reference.
    let reference = game
        .artifact_return_trigger(&game.position.artifacts[0])
        .unwrap();
    assert!(matches!(
        reference,
        crate::game::GenesisTrigger::ReturnArtifact { .. }
    ));
}
#[test]
fn exact_legacy_pairs_fail_closed_in_both_definition_orders_but_legacy_alone_remains_supported() {
    for (name, value) in [
        ("atEndOfControllerTurnUntapNearbyAllies", json!(true)),
        ("atEndOfEachTurnSiteControllerLosesLife", json!(1)),
        (
            "atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn",
            json!(1),
        ),
    ] {
        for (a, b) in [("a-source", "z-held"), ("z-source", "a-held")] {
            let m = selfplay_manifest_with(4, |m| {
                m["cards"][a] = json!({"cardType":"aura","silenceAffectedSitesAndArtifactsThere":true,"manaCost":0,"thresholds":thresholds()});
                m["cards"][b] = json!({"cardType":"artifact","manaCost":0,"thresholds":thresholds(),name:value});
                m["decks"]["north"]["spellbook"]
                    .as_array_mut()
                    .unwrap()
                    .extend([json!(a), json!(b)]);
            });
            assert!(
                matches!(Game::from_manifest_json(&m),Err(GameError::UnsupportedMechanic(reason)) if reason.contains(a)&&reason.contains(b))
            );
            let mut v: Value = serde_json::from_str(&m).unwrap();
            v["cards"].as_object_mut().unwrap().remove(a);
            v["decks"]["north"]["spellbook"]
                .as_array_mut()
                .unwrap()
                .retain(|c| c != a);
            v.as_object_mut().unwrap().remove("manifestId");
            v["manifestId"] = json!(identity_hash(&v).unwrap());
            assert!(
                Game::from_manifest_json(&crate::canonical::canonical_json(&v).unwrap()).is_ok()
            );
        }
    }
}

#[test]
fn after_turn_source_gate_does_not_cancel_captured_return_work() {
    let mut game = fixture();
    let bearer = unit(&game, "north-spell-1", "bearer", Seat::North);
    game.position.units.push(bearer.clone());
    game.position
        .artifacts
        .push(artifact(&game, "return", "trinket", &bearer));
    let captured = game
        .artifact_return_trigger(&game.position.artifacts[0])
        .unwrap();
    silence(&mut game, "source-silence");
    let end = game
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| matches!(a.descriptor, crate::game::ActionDescriptor::EndTurn))
        .unwrap();
    game.apply_action(&end).unwrap();
    assert!(
        game.position
            .artifacts
            .iter()
            .any(|a| a.card.instance_id == id("trinket"))
    );
    game.resolve_genesis_trigger(captured, &mut crate::game::OutcomeLog::Ignore)
        .unwrap();
    assert!(
        !game
            .position
            .artifacts
            .iter()
            .any(|a| a.card.instance_id == id("trinket"))
    );
}

#[test]
fn incoming_ordinary_tower_genesis_is_suppressed_but_its_characteristics_remain() {
    let mut game = fixture();
    silence(&mut game, "source-silence");
    let cell = Cell::parse("C3").unwrap();
    let tower = card(&game, "tower", "incoming").card_id;
    let CardFacts::Site(mut facts) = game.rules.cards[usize::from(tower.0)].facts.clone() else {
        unreachable!()
    };
    facts.ordinary = true;
    game.position.sites[cell.index()] = None;
    assert_eq!(
        game.site_genesis_gain_and_draws(Seat::North, cell, tower, &facts),
        (None, 0)
    );
    assert!(Game::site_qualifies_as_tower(&facts, true));
    game.position.auras.clear();
    // There are other controlled copies in this fixture: the complete only-copy condition is false.
    assert_eq!(
        game.site_genesis_gain_and_draws(Seat::North, cell, tower, &facts),
        (None, 0)
    );
    for site in &mut game.position.sites {
        *site = None;
    }
    assert_eq!(
        game.site_genesis_gain_and_draws(Seat::North, cell, tower, &facts),
        (Some(1), 0)
    );
}

#[test]
#[ignore = "requires ignored source-guarded native checkpoints via SORCERY_ELEMENTAL_PROOF_CHECKPOINTS"]
fn private_source_guarded_checkpoints_have_equal_record_silent_and_owned_native_results() {
    let path = std::env::var("SORCERY_ELEMENTAL_PROOF_CHECKPOINTS")
        .expect("private checkpoint proof path");
    let data: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for checkpoint in data["checkpoints"].as_array().unwrap() {
        let manifest = crate::canonical::canonical_json(&checkpoint["manifest"]).unwrap();
        let mut record = Game::from_manifest_json(&manifest).unwrap();
        let mut silent = Game::from_manifest_json(&manifest).unwrap();
        let mut owned = Game::from_manifest_json(&manifest).unwrap();
        let mut replay = Game::from_manifest_json(&manifest).unwrap();
        let mut accepted = 0;
        for request in checkpoint["requests"].as_array().unwrap() {
            let actions = record.legal_actions().unwrap();
            assert_eq!(actions, silent.legal_actions().unwrap());
            assert_eq!(actions, owned.legal_actions().unwrap());
            let Some(action) = actions.into_iter().find(|action| {
                action.to_legal_action().unwrap().action_id.as_str()
                    == request["actionId"].as_str().unwrap()
            }) else {
                continue;
            };
            accepted += 1;
            let (events, random) = record.apply_action_recorded(&action).unwrap();
            let (replay_events, replay_random) = replay.apply_action_recorded(&action).unwrap();
            assert_eq!(events, replay_events);
            assert_eq!(random, replay_random);
            silent.apply_action(&action).unwrap();
            owned = owned.apply_action_owned(&action).unwrap();
            assert_eq!(record.position, silent.position);
            assert_eq!(record.position, owned.position);
            assert_eq!(record.state_hash().unwrap(), silent.state_hash().unwrap());
        }
        assert!(accepted > 0);
        assert_eq!(record.authoritative_state(), replay.authoritative_state());
    }
}

#[test]
fn captured_magic_and_paid_activation_continue_after_current_source_suppression() {
    use crate::game::effect::{AbilityEntry, RealmReference};
    use crate::game::{DamageOrigin, OutcomeLog, UnitDamageSource};
    for aura in ["recipient-silence", "source-silence"] {
        let mut game = fixture();
        let bearer = unit(&game, "north-spell-1", "bearer", Seat::North);
        let victim = unit(&game, "north-spell-1", "victim", Seat::South);
        let target = UnitTarget::Minion {
            instance_id: victim.card.instance_id.clone(),
            seat: Seat::South,
        };
        let actor = UnitTarget::Minion {
            instance_id: bearer.card.instance_id.clone(),
            seat: Seat::North,
        };
        game.position.units = vec![bearer.clone(), victim];
        game.position
            .artifacts
            .push(artifact(&game, "relic", "relic", &bearer));
        let mut source = game.unit_effect_source(&actor).unwrap();
        source.realm = None;
        source.instance_id = id("magic");
        let frame = game
            .effect_frame(
                card(&game, "captured-magic", "magic").card_id,
                AbilityEntry::Magic,
                source,
                Some(&target),
                None,
                None,
            )
            .unwrap();
        silence(&mut game, aura);
        assert!(EffectiveView::new(&game).caster(&bearer).is_none());
        game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
            .unwrap();
        assert_eq!(game.position.units[1].damage, 1);
    }
    let mut game = fixture();
    let bearer = unit(&game, "north-spell-1", "bearer", Seat::North);
    let victim = unit(&game, "north-spell-1", "victim", Seat::South);
    let target = UnitTarget::Minion {
        instance_id: victim.card.instance_id.clone(),
        seat: Seat::South,
    };
    let actor = UnitTarget::Minion {
        instance_id: bearer.card.instance_id.clone(),
        seat: Seat::North,
    };
    game.position.units = vec![bearer.clone(), victim];
    let source_artifact = artifact(&game, "ballista", "ballista", &bearer);
    let mut source = game.unit_effect_source(&actor).unwrap();
    source.realm = Some(RealmReference::from_card(&source_artifact.card));
    source.instance_id = source_artifact.card.instance_id.clone();
    source.damage = UnitDamageSource {
        origin: DamageOrigin::Other,
        current_power: 0,
        lethal: false,
    };
    let card_id = source_artifact.card.card_id;
    game.position.artifacts.push(source_artifact);
    // Synthetic owner-boundary proof, not an invented real-card response window.
    game.tap_unit_target(&actor).unwrap();
    let frame = game
        .effect_frame(
            card_id,
            AbilityEntry::Activated,
            source,
            Some(&target),
            None,
            None,
        )
        .unwrap();
    silence(&mut game, "source-silence");
    game.run_effect_frame(frame, &mut OutcomeLog::Ignore)
        .unwrap();
    assert!(
        game.position
            .units
            .iter()
            .all(|u| u.card.instance_id != id("victim"))
    );
    assert!(game.position.units[0].tapped);
}

#[test]
fn conditional_spellcaster_witness_uses_adjacent_not_diagonal_nearby_and_same_region() {
    let mut game = fixture();
    game.position.players[0].avatar.location = Cell::parse("A1").unwrap();
    let spirit = unit(&game, "spirit", "spirit", Seat::North);
    let mut caster = unit(&game, "north-spell-1", "caster", Seat::North);
    caster.location = Cell::parse("B2").unwrap();
    game.position.units = vec![spirit.clone(), caster.clone()];
    game.position
        .artifacts
        .push(artifact(&game, "relic", "relic", &caster));
    assert!(game.minion_is_disabled(&spirit));
    game.position.units[1].location = Cell::parse("B3").unwrap();
    assert!(!game.minion_is_disabled(&spirit));
    game.position.units[1].region = Region::Underground;
    assert!(game.minion_is_disabled(&spirit));
    game.position.units[1].region = Region::Surface;
    game.position.units[1].controller = Seat::South;
    assert!(game.minion_is_disabled(&spirit));
}

#[test]
fn prospective_entry_power_applies_recipient_silence_before_own_tower_clause() {
    let mut game = fixture();
    let cell = Cell::parse("C3").unwrap();
    let conditional = card(&game, "conditional", "incoming").card_id;
    let CardFacts::Minion(facts) = game.rules.cards[usize::from(conditional.0)].facts.clone()
    else {
        unreachable!()
    };
    let tower = card(&game, "tower", "incoming-site").card_id;
    let CardFacts::Site(site) = game.rules.cards[usize::from(tower.0)].facts.clone() else {
        unreachable!()
    };
    assert_eq!(
        game.prospective_minion_entry_power(Seat::North, &facts, &[cell], None),
        3
    );
    silence(&mut game, "recipient-silence");
    assert_eq!(
        game.prospective_minion_entry_power(Seat::North, &facts, &[cell], None),
        1
    );
    game.position.sites[cell.index()] = None;
    assert_eq!(
        game.prospective_minion_entry_power(Seat::North, &facts, &[cell], Some((cell, &site))),
        1
    );
    game.position.auras.clear();
    assert_eq!(
        game.prospective_minion_entry_power(Seat::North, &facts, &[cell], Some((cell, &site))),
        3
    );
}

#[test]
fn token_entry_reuses_region_and_projected_suppression_guards() {
    let mut game = fixture();
    let cell = Cell::parse("C3").unwrap();
    let conditional = card(&game, "conditional", "incoming").card_id;
    let CardFacts::Minion(facts) = game.rules.cards[usize::from(conditional.0)].facts.clone()
    else {
        unreachable!()
    };
    let tower = card(&game, "tower", "incoming-site").card_id;
    let CardFacts::Site(mut site) = game.rules.cards[usize::from(tower.0)].facts.clone() else {
        unreachable!()
    };
    site.prevents_units_with_power_at_least_from_entering = Some(2);
    assert!(
        game.token_entry_power_for_location(
            Seat::North,
            &facts,
            &[cell],
            Region::Surface,
            Some((cell, &site))
        )
        .is_err()
    );
    assert_eq!(
        game.prospective_minion_entry_bonus(
            Seat::North,
            &facts,
            &[cell],
            Region::Underground,
            Some((cell, &site))
        ),
        0
    );
    assert_eq!(
        game.token_entry_power_for_location(
            Seat::North,
            &facts,
            &[cell],
            Region::Underground,
            Some((cell, &site))
        )
        .unwrap(),
        1
    );
    silence(&mut game, "recipient-silence");
    assert_eq!(
        game.token_entry_power_for_location(
            Seat::North,
            &facts,
            &[cell],
            Region::Surface,
            Some((cell, &site))
        )
        .unwrap(),
        1
    );
    game.position.auras.clear();
    silence(&mut game, "source-silence");
    assert_eq!(
        game.token_entry_power_for_location(
            Seat::North,
            &facts,
            &[cell],
            Region::Surface,
            Some((cell, &site))
        )
        .unwrap(),
        1
    );
}
