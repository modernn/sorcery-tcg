//! Statically lowered ability effects used by the effect-frame runtime.

use std::sync::Arc;

pub(super) use crate::ability::{
    AbilityProgram, ControllerRelation, Effect, SelectionSpec, SpatialRelation, UnitArea,
    UnitChoiceSpec, UnitCohort, UnitSet,
};
use crate::ability::{EffectDuration, TemporaryModifierKind};
use crate::action::DeckZone;
use crate::facts::{ArtifactEffect, CardFacts, MagicEffect, MinionFacts};

/// The immutable, supported entries for one card definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CompiledAbilities {
    pub(super) magic: Option<Arc<AbilityProgram>>,
    pub(super) genesis: Option<Arc<AbilityProgram>>,
    pub(super) activated: Option<Arc<AbilityProgram>>,
}

impl CompiledAbilities {
    /// Lowers the supported effects in validated card facts.
    #[must_use]
    pub(super) fn from_facts(facts: &CardFacts) -> Self {
        match facts {
            CardFacts::Magic(facts) => Self {
                magic: compile_magic(facts),
                genesis: None,
                activated: None,
            },
            CardFacts::Minion(facts) => Self {
                magic: None,
                genesis: compile_genesis(facts),
                activated: compile_minion_activation(facts),
            },
            CardFacts::Artifact(facts) => Self {
                magic: None,
                genesis: None,
                activated: match facts.effect {
                    ArtifactEffect::TapBearerAndAnotherAllyHereToDamageTargetWithinTwoStepsThree => {
                        Some(ability_with_selection(
                            Some(SelectionSpec::Unit {
                                kind: None,
                                relation: SpatialRelation::Measured(2),
                            }),
                            [Effect::Damage {
                                recipients: UnitSet::Target,
                                amount: 3,
                            }],
                        ))
                    }
                    _ => None,
                },
            },
            CardFacts::Avatar(_) | CardFacts::Aura(_) | CardFacts::Site(_) => Self {
                magic: None,
                genesis: None,
                activated: None,
            },
        }
    }
}

fn ability<const N: usize>(effects: [Effect; N]) -> Arc<AbilityProgram> {
    ability_with_selection(None, effects)
}

fn ability_with_selection<const N: usize>(
    selection: Option<SelectionSpec>,
    effects: [Effect; N],
) -> Arc<AbilityProgram> {
    ability_with_options(selection, false, effects)
}

fn ability_with_options<const N: usize>(
    selection: Option<SelectionSpec>,
    optional_selection: bool,
    effects: [Effect; N],
) -> Arc<AbilityProgram> {
    Arc::new(AbilityProgram {
        selection,
        optional_selection,
        effects: Box::new(effects),
    })
}

fn ally_grant(
    modifier: TemporaryModifierKind,
    amount: u16,
    kind: Option<super::UnitKind>,
    draw_spell: bool,
) -> Arc<AbilityProgram> {
    let mut effects = vec![
        Effect::ChooseUnit(UnitChoiceSpec {
            kind,
            relation: SpatialRelation::Anywhere,
            exclude_source: false,
            allied_only: true,
            optional: false,
        }),
        Effect::Grant {
            duration: EffectDuration::ThisTurn,
            recipients: UnitSet::Chosen,
            modifier,
            amount,
        },
    ];
    if draw_spell {
        effects.push(Effect::Draw {
            zone: DeckZone::Spellbook,
            count: 1,
        });
    }
    Arc::new(AbilityProgram {
        selection: None,
        optional_selection: false,
        effects: effects.into_boxed_slice(),
    })
}

// Keep the finite legacy-to-program lowering together; new bindings author programs directly.
#[allow(clippy::too_many_lines)]
fn compile_magic(facts: &crate::facts::MagicFacts) -> Option<Arc<AbilityProgram>> {
    let effect = match &facts.effect {
        MagicEffect::Program(program) => return Some(Arc::clone(program)),
        MagicEffect::GrantChargeToAllyThisTurn => {
            return Some(ally_grant(TemporaryModifierKind::Charge, 1, None, false));
        }
        MagicEffect::GrantFirstStrikeToAllyThisTurn => {
            return Some(ally_grant(
                TemporaryModifierKind::FirstStrike,
                1,
                None,
                false,
            ));
        }
        MagicEffect::GrantMovementOneToAllyThisTurnThenDrawSpell => {
            return Some(ally_grant(TemporaryModifierKind::Movement, 1, None, true));
        }
        MagicEffect::GrantPowerTwoToAllyThisTurn => {
            return Some(ally_grant(TemporaryModifierKind::Power, 2, None, false));
        }
        MagicEffect::GrantPowerTwoToAllyThisTurnThenDrawSpell => {
            return Some(ally_grant(
                TemporaryModifierKind::Power,
                2,
                Some(super::UnitKind::Minion),
                true,
            ));
        }
        MagicEffect::DamageTargetUnit {
            amount,
            target_nearby,
            untap_target_minion_after_damage,
        } => {
            let damage = Effect::Damage {
                recipients: UnitSet::Target,
                amount: u16::from(*amount),
            };
            let selection = Some(SelectionSpec::Unit {
                kind: (*untap_target_minion_after_damage).then_some(super::UnitKind::Minion),
                relation: if *target_nearby {
                    SpatialRelation::Nearby
                } else {
                    SpatialRelation::Anywhere
                },
            });
            return Some(if *untap_target_minion_after_damage {
                ability_with_selection(
                    selection,
                    [
                        damage,
                        Effect::Untap {
                            recipients: UnitSet::Target,
                        },
                    ],
                )
            } else {
                ability_with_selection(selection, [damage])
            });
        }
        MagicEffect::GrantStealthToTargetMinion => {
            return Some(ability_with_selection(
                Some(SelectionSpec::Unit {
                    kind: Some(super::UnitKind::Minion),
                    relation: SpatialRelation::Anywhere,
                }),
                [Effect::GiveStealth {
                    recipients: UnitSet::Target,
                }],
            ));
        }
        MagicEffect::GrantStealthToAlliedMinionsThenDrawSpell => {
            return Some(ability([
                Effect::GiveStealth {
                    recipients: UnitSet::Query(UnitCohort {
                        area: UnitArea::Realm { region: None },
                        kind: Some(super::UnitKind::Minion),
                        controller: ControllerRelation::Allied,
                        relation: None,
                        exclude_source: false,
                    }),
                },
                Effect::Draw {
                    zone: DeckZone::Spellbook,
                    count: 1,
                },
            ]));
        }
        MagicEffect::UntapTargetMinion => {
            return Some(ability_with_selection(
                Some(SelectionSpec::Unit {
                    kind: Some(super::UnitKind::Minion),
                    relation: SpatialRelation::Anywhere,
                }),
                [Effect::Untap {
                    recipients: UnitSet::Target,
                }],
            ));
        }
        MagicEffect::DrawSites(count) => Effect::Draw {
            zone: DeckZone::Atlas,
            count: *count,
        },
        MagicEffect::DrawSpells(count) => Effect::Draw {
            zone: DeckZone::Spellbook,
            count: *count,
        },
        MagicEffect::DamageEachAbovegroundMinionOne => Effect::Damage {
            recipients: UnitSet::Query(UnitCohort {
                area: UnitArea::Realm {
                    region: Some(super::Region::Surface),
                },
                kind: Some(super::UnitKind::Minion),
                controller: ControllerRelation::Any,
                relation: None,
                exclude_source: false,
            }),
            amount: 1,
        },
        MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(amount) => Effect::Damage {
            recipients: UnitSet::Query(UnitCohort {
                area: UnitArea::Location,
                kind: None,
                controller: ControllerRelation::Any,
                relation: None,
                exclude_source: false,
            }),
            amount: u16::from(*amount),
        },
        _ => return None,
    };
    let selection = match &facts.effect {
        MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(_) => Some(SelectionSpec::Location {
            relation: SpatialRelation::Measured(2),
        }),
        _ => None,
    };
    Some(ability_with_selection(selection, [effect]))
}

fn compile_minion_activation(facts: &MinionFacts) -> Option<Arc<AbilityProgram>> {
    facts.tap_to_damage_each_unit_at_adjacent_location.then(|| {
        ability_with_selection(
            Some(SelectionSpec::Location {
                relation: SpatialRelation::Adjacent,
            }),
            [Effect::Damage {
                recipients: UnitSet::Query(UnitCohort {
                    area: UnitArea::Location,
                    kind: None,
                    controller: ControllerRelation::Any,
                    relation: None,
                    exclude_source: false,
                }),
                amount: 2,
            }],
        )
    })
}

pub(super) fn genesis_clause_count(facts: &MinionFacts) -> usize {
    usize::from(facts.genesis_program.is_some())
        + usize::from(facts.genesis_damage_each_other_unit_here)
        + usize::from(facts.genesis_disable_self_until_damaged)
        + usize::from(facts.genesis_draw_site)
        + usize::from(facts.genesis_draw_spells.is_some())
        + usize::from(facts.genesis_each_player_controlled_by_previous_player_next_turn)
        + usize::from(facts.genesis_gain_control_of_tapped_minions_here_until_this_leaves)
        + usize::from(facts.genesis_heal_controller)
        + usize::from(facts.genesis_lose_controller_life)
        + usize::from(facts.genesis_may_damage_target_adjacent_unit)
        + usize::from(facts.genesis_strike_each_enemy_here)
        + usize::from(facts.genesis_untap_adjacent_allies)
}

fn compile_genesis(facts: &MinionFacts) -> Option<Arc<AbilityProgram>> {
    if let Some(program) = &facts.genesis_program {
        return Some(Arc::clone(program));
    }
    if genesis_clause_count(facts) != 1 {
        return None;
    }
    if facts.genesis_draw_site {
        return Some(ability([Effect::Draw {
            zone: DeckZone::Atlas,
            count: 1,
        }]));
    }
    if let Some(count) = facts.genesis_draw_spells {
        return Some(ability([Effect::Draw {
            zone: DeckZone::Spellbook,
            count,
        }]));
    }
    if facts.genesis_damage_each_other_unit_here {
        return Some(ability([Effect::Damage {
            recipients: UnitSet::Query(UnitCohort {
                area: UnitArea::Source,
                kind: None,
                controller: ControllerRelation::Any,
                relation: None,
                exclude_source: true,
            }),
            amount: 1,
        }]));
    }
    if facts.genesis_may_damage_target_adjacent_unit {
        return Some(ability_with_options(
            Some(SelectionSpec::Unit {
                kind: None,
                relation: SpatialRelation::Adjacent,
            }),
            true,
            [Effect::Damage {
                recipients: UnitSet::Target,
                amount: 2,
            }],
        ));
    }
    if facts.genesis_untap_adjacent_allies {
        return Some(ability([
            Effect::ChooseUnit(UnitChoiceSpec {
                kind: None,
                relation: SpatialRelation::Adjacent,
                exclude_source: false,
                allied_only: true,
                optional: false,
            }),
            Effect::Untap {
                recipients: UnitSet::Chosen,
            },
        ]));
    }
    None
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        CompiledAbilities, Effect, SelectionSpec, SpatialRelation, UnitChoiceSpec, UnitSet,
    };
    use crate::facts::parse_card_definition;
    use crate::game::UnitKind;

    #[test]
    fn authored_program_reuses_the_validated_allocation() {
        let facts = magic(&json!({"effectProgram": {"effects": [
            {"op": "draw-card"},
            {"op": "choose-unit", "relation": "anywhere", "alliedOnly": true},
            {"op": "grant", "duration": "this-turn", "recipients": "chosen", "modifier": "movement", "amount": 1}
        ]}}));
        let compiled = CompiledAbilities::from_facts(&facts);
        let crate::facts::CardFacts::Magic(facts) = facts else {
            panic!("Magic")
        };
        let crate::facts::MagicEffect::Program(program) = facts.effect else {
            panic!("program")
        };
        assert!(std::sync::Arc::ptr_eq(
            &program,
            compiled.magic.as_ref().unwrap()
        ));
    }

    fn minion(extra: &serde_json::Value) -> crate::facts::CardFacts {
        let mut value = json!({
            "attack": 1,
            "cardType": "minion",
            "defense": 1,
            "manaCost": 1,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0}
        });
        value
            .as_object_mut()
            .expect("minion object")
            .extend(extra.as_object().expect("extra object").clone());
        parse_card_definition("test-minion", &value).expect("valid synthetic minion")
    }

    fn magic(extra: &serde_json::Value) -> crate::facts::CardFacts {
        let mut value = json!({
            "cardType": "magic",
            "manaCost": 1,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0}
        });
        value
            .as_object_mut()
            .expect("magic object")
            .extend(extra.as_object().expect("extra object").clone());
        parse_card_definition("test-magic", &value).expect("valid synthetic magic")
    }

    #[test]
    fn mixed_genesis_stays_legacy_as_a_whole() {
        let facts = minion(&json!({
            "genesisDamageEachOtherUnitHere": 1,
            "genesisDrawSite": true
        }));
        let compiled = CompiledAbilities::from_facts(&facts);
        assert_eq!(compiled.genesis, None);
    }

    #[test]
    fn targeted_magic_preserves_damage_then_untap_order() {
        let facts = parse_card_definition(
            "test-magic",
            &json!({
                "cardType": "magic",
                "damageTargetUnit": 2,
                "manaCost": 1,
                "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
                "untapTargetMinionAfterDamage": true
            }),
        )
        .expect("valid synthetic magic");
        let compiled = CompiledAbilities::from_facts(&facts);
        let magic = compiled.magic.expect("compiled magic");
        assert_eq!(
            magic.effects.as_ref(),
            &[
                Effect::Damage {
                    recipients: UnitSet::Target,
                    amount: 2
                },
                Effect::Untap {
                    recipients: UnitSet::Target
                }
            ]
        );
        assert_eq!(
            magic.selection,
            Some(SelectionSpec::Unit {
                kind: Some(UnitKind::Minion),
                relation: SpatialRelation::Anywhere,
            })
        );
    }

    #[test]
    fn selection_lowering_tracks_source_ranges_and_kinds() {
        let nearby_target = CompiledAbilities::from_facts(&magic(&json!({
            "damageTargetUnit": 2,
            "targetNearby": true,
            "untapTargetMinionAfterDamage": true
        })))
        .magic
        .expect("compiled nearby target");
        assert_eq!(
            nearby_target.selection,
            Some(SelectionSpec::Unit {
                kind: Some(UnitKind::Minion),
                relation: SpatialRelation::Nearby,
            })
        );

        let location = CompiledAbilities::from_facts(&magic(&json!({
            "damageEachUnitAtLocationWithinTwoSteps": 2
        })))
        .magic
        .expect("compiled location target");
        assert_eq!(
            location.selection,
            Some(SelectionSpec::Location {
                relation: SpatialRelation::Measured(2),
            })
        );

        let artifact = {
            let facts = parse_card_definition(
                "test-artifact",
                &json!({
                    "cardType": "artifact",
                    "manaCost": 1,
                    "tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps": 3,
                    "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0}
                }),
            )
            .expect("valid synthetic artifact");
            CompiledAbilities::from_facts(&facts)
                .activated
                .expect("compiled artifact")
        };
        assert_eq!(
            artifact.selection,
            Some(SelectionSpec::Unit {
                kind: None,
                relation: SpatialRelation::Measured(2),
            })
        );

        let minion = {
            let facts = minion(&json!({
                "tapToDamageEachUnitAtAdjacentLocation": 2
            }));
            CompiledAbilities::from_facts(&facts)
                .activated
                .expect("compiled minion activation")
        };
        assert_eq!(
            minion.selection,
            Some(SelectionSpec::Location {
                relation: SpatialRelation::Adjacent,
            })
        );
    }

    #[test]
    fn area_draw_and_genesis_entries_have_no_selection_spec() {
        let area = CompiledAbilities::from_facts(&magic(&json!({
            "damageEachAbovegroundMinion": 1
        })))
        .magic
        .expect("compiled area damage");
        assert_eq!(area.selection, None);

        let draw = CompiledAbilities::from_facts(&magic(&json!({
            "drawSites": 1
        })))
        .magic
        .expect("compiled draw");
        assert_eq!(draw.selection, None);

        let genesis = CompiledAbilities::from_facts(&minion(&json!({
            "genesisDamageEachOtherUnitHere": 1
        })))
        .genesis
        .expect("compiled genesis");
        assert_eq!(genesis.selection, None);
    }

    #[test]
    fn optional_genesis_damage_lowers_to_adjacent_target_selection() {
        let genesis = CompiledAbilities::from_facts(&minion(&json!({
            "genesisMayDamageTargetAdjacentUnit": 2
        })))
        .genesis
        .expect("compiled genesis");
        assert!(genesis.optional_selection);
        assert_eq!(
            genesis.selection,
            Some(SelectionSpec::Unit {
                kind: None,
                relation: SpatialRelation::Adjacent,
            })
        );
        assert_eq!(
            genesis.effects.as_ref(),
            &[Effect::Damage {
                recipients: UnitSet::Target,
                amount: 2,
            }]
        );
    }

    #[test]
    fn genesis_untap_lowers_to_choice_then_untap() {
        let genesis = CompiledAbilities::from_facts(&minion(&json!({
            "genesisUntapAdjacentAllies": true
        })))
        .genesis
        .expect("compiled genesis");
        assert!(!genesis.optional_selection);
        assert_eq!(genesis.selection, None);
        assert_eq!(
            genesis.effects.as_ref(),
            &[
                Effect::ChooseUnit(UnitChoiceSpec {
                    kind: None,
                    relation: SpatialRelation::Adjacent,
                    exclude_source: false,
                    allied_only: true,
                    optional: false,
                }),
                Effect::Untap {
                    recipients: UnitSet::Chosen,
                },
            ]
        );
    }
}
