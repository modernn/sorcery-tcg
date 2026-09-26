//! Typed, ordered ability programs supplied by card data.

use serde::{Deserialize, Serialize};

use crate::action::DeckZone;

/// An ordered ability program.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityProgram {
    /// The declared target, if this program has one.
    #[serde(default)]
    pub selection: Option<SelectionSpec>,
    /// Whether the declared target may be omitted.
    #[serde(default)]
    pub optional_selection: bool,
    /// Effects are executed in this exact order.
    pub effects: Box<[Effect]>,
}

impl AbilityProgram {
    /// Validates structural constraints shared by all ability entries.
    ///
    /// # Errors
    /// Returns the invalid operation or unsupported selector/grant constraint.
    pub fn validate(&self) -> Result<(), String> {
        if self.effects.is_empty() || self.effects.len() > 64 {
            return Err("effects must contain 1-64 operations".to_owned());
        }
        if self.optional_selection && self.selection.is_none() {
            return Err("optionalSelection requires selection".to_owned());
        }
        if let Some(selection) = self.selection {
            validate_relation(selection.relation())?;
        }

        let mut previous_choice = None;
        for (index, effect) in self.effects.iter().enumerate() {
            let path = format!("effects[{index}]");
            match effect {
                Effect::Damage { recipients, amount } => {
                    validate_amount(*amount, &path, "damage")?;
                    self.validate_recipients(*recipients, previous_choice, &path)?;
                }
                Effect::Untap { recipients } => {
                    self.validate_recipients(*recipients, previous_choice, &path)?;
                }
                Effect::Draw { count, .. } => {
                    if *count == 0 || *count > 32 {
                        return Err(format!("{path}.count must be 1-32"));
                    }
                }
                Effect::DrawCard => {}
                Effect::ChooseUnit(spec) => {
                    validate_relation(spec.relation)?;
                    previous_choice = Some(*spec);
                }
                Effect::GrantThisTurn {
                    recipients,
                    modifier,
                    amount,
                } => {
                    self.validate_recipients(*recipients, previous_choice, &path)?;
                    validate_grant(
                        *recipients,
                        *modifier,
                        *amount,
                        self.selection,
                        previous_choice,
                        &path,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn validate_recipients(
        &self,
        recipients: UnitSet,
        previous_choice: Option<UnitChoiceSpec>,
        path: &str,
    ) -> Result<(), String> {
        match recipients {
            UnitSet::Target => {
                if !matches!(self.selection, Some(SelectionSpec::Unit { .. })) {
                    return Err(format!("{path}.recipients target requires unit selection"));
                }
            }
            UnitSet::Location => {
                if !matches!(self.selection, Some(SelectionSpec::Location { .. })) {
                    return Err(format!(
                        "{path}.recipients location requires location selection"
                    ));
                }
            }
            UnitSet::Chosen if previous_choice.is_none() => {
                return Err(format!(
                    "{path}.recipients chosen requires a preceding choose-unit"
                ));
            }
            UnitSet::Chosen | UnitSet::OtherUnitsHere | UnitSet::SurfaceMinions => {}
        }
        Ok(())
    }
}

fn validate_grant(
    recipients: UnitSet,
    modifier: TemporaryModifierKind,
    amount: u16,
    selection: Option<SelectionSpec>,
    previous_choice: Option<UnitChoiceSpec>,
    path: &str,
) -> Result<(), String> {
    if modifier == TemporaryModifierKind::NextStrikeDouble {
        return Err(format!(
            "{path}.modifier next-strike-double is unsupported in authored programs"
        ));
    }
    if matches!(
        modifier,
        TemporaryModifierKind::Power | TemporaryModifierKind::Movement
    ) {
        if !(1..=255).contains(&amount) {
            return Err(format!("{path}.amount must be 1-255 for this modifier"));
        }
    } else if amount != 1 {
        return Err(format!("{path}.amount must be exactly 1 for this modifier"));
    }
    if modifier == TemporaryModifierKind::Silence {
        return Err(format!(
            "{path}.modifier silence is unsupported in authored programs"
        ));
    }
    if matches!(
        modifier,
        TemporaryModifierKind::Airborne
            | TemporaryModifierKind::Ranged
            | TemporaryModifierKind::Lethal
    ) && !minion_only(recipients, selection, previous_choice)
    {
        return Err(format!("{path}.modifier requires a minion-only recipient"));
    }
    Ok(())
}

fn minion_only(
    recipients: UnitSet,
    selection: Option<SelectionSpec>,
    previous_choice: Option<UnitChoiceSpec>,
) -> bool {
    match recipients {
        UnitSet::SurfaceMinions => true,
        UnitSet::Chosen => {
            previous_choice.is_some_and(|choice| choice.kind == Some(UnitKind::Minion))
        }
        UnitSet::Target => matches!(
            selection,
            Some(SelectionSpec::Unit {
                kind: Some(UnitKind::Minion),
                ..
            })
        ),
        UnitSet::Location | UnitSet::OtherUnitsHere => false,
    }
}

fn validate_amount(amount: u16, path: &str, name: &str) -> Result<(), String> {
    if amount == 0 || amount > 255 {
        return Err(format!("{path}.{name} must be 1-255"));
    }
    Ok(())
}

fn validate_relation(relation: SpatialRelation) -> Result<(), String> {
    if matches!(relation, SpatialRelation::Measured(0)) {
        return Err("measured relation must be positive".to_owned());
    }
    Ok(())
}

/// A declared target for an ability program.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SelectionSpec {
    /// A unit target.
    Unit {
        /// Restricts the target to a unit kind, when present.
        #[serde(rename = "unitKind")]
        kind: Option<UnitKind>,
        /// Restricts the target's distance.
        relation: SpatialRelation,
    },
    /// A location target.
    Location {
        /// Restricts the target location's distance.
        relation: SpatialRelation,
    },
}

impl SelectionSpec {
    const fn relation(self) -> SpatialRelation {
        match self {
            Self::Unit { relation, .. } | Self::Location { relation } => relation,
        }
    }
}

/// A unit choice performed during an ability program.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnitChoiceSpec {
    /// Restricts the chosen unit kind, when present.
    #[serde(default)]
    pub kind: Option<UnitKind>,
    /// Restricts the chosen unit's distance.
    pub relation: SpatialRelation,
    /// Restricts the choice to allied units.
    #[serde(default)]
    pub allied_only: bool,
    /// Allows declining the choice.
    #[serde(default)]
    pub optional: bool,
}

/// Unit kinds understood by authored selectors.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnitKind {
    /// An avatar.
    Avatar,
    /// A minion.
    Minion,
}

impl UnitKind {
    /// Returns the stable event label for this unit kind.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Avatar => "avatar",
            Self::Minion => "minion",
        }
    }
}

/// A spatial relationship used by selectors.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpatialRelation {
    /// Any legal distance.
    Anywhere,
    /// Within the nearby range.
    Nearby,
    /// An adjacent square.
    Adjacent,
    /// A bounded measured range.
    Measured(u8),
}

/// A recipient cohort for an effect.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnitSet {
    /// The declared unit target.
    Target,
    /// The unit selected by a preceding ordinary choice.
    Chosen,
    /// Every unit at the declared location.
    Location,
    /// Every other unit at the source location.
    OtherUnitsHere,
    /// Every surface minion in the realm.
    SurfaceMinions,
}

/// Temporary keyword modifiers that an authored program may grant.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemporaryModifierKind {
    /// Grants the Airborne keyword.
    Airborne,
    /// Grants the Charge keyword.
    Charge,
    /// Grants the First Strike keyword.
    FirstStrike,
    /// Grants the Lethal keyword.
    Lethal,
    /// Doubles damage on the next strike.
    NextStrikeDouble,
    /// Grants movement.
    Movement,
    /// Grants power.
    Power,
    /// Grants the Ranged keyword.
    Ranged,
    /// Grants the Silence keyword.
    Silence,
}

/// One executable operation in an ordered ability program.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Effect {
    /// Deals damage to a recipient cohort.
    Damage {
        /// The cohort receiving damage.
        recipients: UnitSet,
        /// The damage amount.
        amount: u16,
    },
    /// Untaps a recipient cohort.
    Untap {
        /// The cohort receiving untap.
        recipients: UnitSet,
    },
    /// Draws cards from a specific deck.
    Draw {
        /// The deck to draw from.
        zone: DeckZone,
        /// The number of cards to draw.
        count: u8,
    },
    /// Performs an ordinary unit choice.
    ChooseUnit(UnitChoiceSpec),
    /// Grants a temporary modifier to a recipient cohort.
    GrantThisTurn {
        /// The cohort receiving the modifier.
        recipients: UnitSet,
        /// The modifier to grant.
        modifier: TemporaryModifierKind,
        /// The modifier amount.
        amount: u16,
    },
    /// Draws one card from either deck according to runtime rules.
    DrawCard,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_rejects_unknown_effect_fields() {
        let error = serde_json::from_str::<AbilityProgram>(
            r#"{"effects":[{"op":"draw","zone":"atlas","count":1,"extra":true}]}"#,
        )
        .expect_err("unknown effect field must be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn serde_uses_camel_case_choice_fields() {
        let program = serde_json::from_str::<AbilityProgram>(
            r#"{"effects":[{"op":"choose-unit","relation":"adjacent","alliedOnly":true}]}"#,
        )
        .expect("choice fields use the public camelCase contract");
        assert_eq!(
            serde_json::to_value(program).expect("serialize program")["effects"][0]["alliedOnly"],
            true
        );
    }

    #[test]
    fn validation_rejects_bounds_and_unbound_recipients() {
        let too_many = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: (0..65)
                .map(|_| Effect::Draw {
                    zone: DeckZone::Atlas,
                    count: 1,
                })
                .collect(),
        };
        assert!(too_many.validate().is_err());

        let unbound = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: vec![Effect::Damage {
                recipients: UnitSet::Target,
                amount: 1,
            }]
            .into_boxed_slice(),
        };
        assert!(unbound.validate().unwrap_err().contains("unit selection"));
    }

    #[test]
    fn validation_requires_choice_before_chosen_and_restricts_grants() {
        let chosen = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: vec![Effect::Untap {
                recipients: UnitSet::Chosen,
            }]
            .into_boxed_slice(),
        };
        assert!(
            chosen
                .validate()
                .unwrap_err()
                .contains("preceding choose-unit")
        );

        let avatar_grant = AbilityProgram {
            selection: Some(SelectionSpec::Unit {
                kind: None,
                relation: SpatialRelation::Anywhere,
            }),
            optional_selection: false,
            effects: vec![Effect::GrantThisTurn {
                recipients: UnitSet::Target,
                modifier: TemporaryModifierKind::Airborne,
                amount: 1,
            }]
            .into_boxed_slice(),
        };
        assert!(avatar_grant.validate().unwrap_err().contains("minion-only"));
    }

    #[test]
    fn grants_require_bound_recipients_and_supported_amounts() {
        let unbound = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: vec![Effect::GrantThisTurn {
                recipients: UnitSet::Target,
                modifier: TemporaryModifierKind::Power,
                amount: 1,
            }]
            .into_boxed_slice(),
        };
        assert!(unbound.validate().unwrap_err().contains("unit selection"));

        let boolean_amount = AbilityProgram {
            selection: Some(SelectionSpec::Unit {
                kind: Some(UnitKind::Minion),
                relation: SpatialRelation::Anywhere,
            }),
            optional_selection: false,
            effects: vec![Effect::GrantThisTurn {
                recipients: UnitSet::Target,
                modifier: TemporaryModifierKind::Charge,
                amount: 2,
            }]
            .into_boxed_slice(),
        };
        assert!(boolean_amount.validate().unwrap_err().contains("exactly 1"));

        let power_overflow = AbilityProgram {
            selection: Some(SelectionSpec::Unit {
                kind: Some(UnitKind::Minion),
                relation: SpatialRelation::Anywhere,
            }),
            optional_selection: false,
            effects: vec![Effect::GrantThisTurn {
                recipients: UnitSet::Target,
                modifier: TemporaryModifierKind::Power,
                amount: 256,
            }]
            .into_boxed_slice(),
        };
        assert!(power_overflow.validate().unwrap_err().contains("1-255"));

        let silence = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: vec![Effect::GrantThisTurn {
                recipients: UnitSet::SurfaceMinions,
                modifier: TemporaryModifierKind::Silence,
                amount: 1,
            }]
            .into_boxed_slice(),
        };
        assert!(
            silence
                .validate()
                .unwrap_err()
                .contains("silence is unsupported")
        );
    }
}
