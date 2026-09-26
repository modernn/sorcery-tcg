//! Statically lowered ability effects used by the effect-frame runtime.

use crate::action::DeckZone;
use crate::facts::{ArtifactEffect, CardFacts, MagicEffect, MinionFacts};

/// The immutable, supported entries for one card definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CompiledAbilities {
    pub(super) magic: Option<CompiledAbility>,
    pub(super) genesis: Option<CompiledAbility>,
    pub(super) activated: Option<CompiledAbility>,
}

/// One statically ordered effect program.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CompiledAbility {
    pub(super) effects: Box<[Effect]>,
}

/// An operation understood by the first effect-frame slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Effect {
    Damage { recipients: UnitSet, amount: u16 },
    Untap { recipients: UnitSet },
    Draw { zone: DeckZone, count: u8 },
}

/// A fixed recipient query for a compiled operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UnitSet {
    Target,
    Location,
    OtherUnitsHere,
    SurfaceMinions,
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
                        Some(ability([Effect::Damage {
                            recipients: UnitSet::Target,
                            amount: 3,
                        }]))
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

fn ability<const N: usize>(effects: [Effect; N]) -> CompiledAbility {
    CompiledAbility {
        effects: Box::new(effects),
    }
}

fn compile_magic(facts: &crate::facts::MagicFacts) -> Option<CompiledAbility> {
    let effect = match &facts.effect {
        MagicEffect::DamageTargetUnit {
            amount,
            untap_target_minion_after_damage,
            ..
        } => {
            let damage = Effect::Damage {
                recipients: UnitSet::Target,
                amount: u16::from(*amount),
            };
            return Some(if *untap_target_minion_after_damage {
                ability([
                    damage,
                    Effect::Untap {
                        recipients: UnitSet::Target,
                    },
                ])
            } else {
                ability([damage])
            });
        }
        MagicEffect::UntapTargetMinion => Effect::Untap {
            recipients: UnitSet::Target,
        },
        MagicEffect::DrawSites(count) => Effect::Draw {
            zone: DeckZone::Atlas,
            count: *count,
        },
        MagicEffect::DrawSpells(count) => Effect::Draw {
            zone: DeckZone::Spellbook,
            count: *count,
        },
        MagicEffect::DamageEachAbovegroundMinionOne => Effect::Damage {
            recipients: UnitSet::SurfaceMinions,
            amount: 1,
        },
        MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(amount) => Effect::Damage {
            recipients: UnitSet::Location,
            amount: u16::from(*amount),
        },
        _ => return None,
    };
    Some(ability([effect]))
}

fn compile_minion_activation(facts: &MinionFacts) -> Option<CompiledAbility> {
    facts.tap_to_damage_each_unit_at_adjacent_location.then(|| {
        ability([Effect::Damage {
            recipients: UnitSet::Location,
            amount: 2,
        }])
    })
}

fn compile_genesis(facts: &MinionFacts) -> Option<CompiledAbility> {
    let active_count = usize::from(facts.genesis_damage_each_other_unit_here)
        + usize::from(facts.genesis_disable_self_until_damaged)
        + usize::from(facts.genesis_draw_site)
        + usize::from(facts.genesis_draw_spells.is_some())
        + usize::from(facts.genesis_each_player_controlled_by_previous_player_next_turn)
        + usize::from(facts.genesis_gain_control_of_tapped_minions_here_until_this_leaves)
        + usize::from(facts.genesis_heal_controller)
        + usize::from(facts.genesis_lose_controller_life)
        + usize::from(facts.genesis_may_damage_target_adjacent_unit)
        + usize::from(facts.genesis_strike_each_enemy_here)
        + usize::from(facts.genesis_untap_adjacent_allies);

    if active_count != 1 {
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
            recipients: UnitSet::OtherUnitsHere,
            amount: 1,
        }]));
    }

    // Adjacent-allies Genesis currently needs an entry-time engine-issued choice.
    // Keep it on the legacy path until that declaration is part of the frame.
    None
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{CompiledAbilities, Effect, UnitSet};
    use crate::facts::parse_card_definition;

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
        assert_eq!(
            compiled.magic.expect("compiled magic").effects.as_ref(),
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
    }
}
