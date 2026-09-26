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
                Effect::Grant {
                    recipients,
                    modifier,
                    amount,
                    duration: _,
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
                Effect::GiveStealth { recipients } => {
                    self.validate_recipients(*recipients, previous_choice, &path)?;
                    if !minion_only(*recipients, self.selection, previous_choice) {
                        return Err(format!(
                            "{path}.recipients requires a minion-only recipient"
                        ));
                    }
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
            UnitSet::Chosen if previous_choice.is_none() => {
                return Err(format!(
                    "{path}.recipients chosen requires a preceding choose-unit"
                ));
            }
            UnitSet::Chosen => {}
            UnitSet::Query(cohort) => {
                if matches!(cohort.area, UnitArea::Location)
                    && !matches!(self.selection, Some(SelectionSpec::Location { .. }))
                {
                    return Err(format!(
                        "{path}.recipients query location requires location selection"
                    ));
                }
            }
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
        UnitSet::Query(cohort) => cohort.kind == Some(UnitKind::Minion),
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

/// The area from which a query derives its units.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum UnitArea {
    /// The source unit's location, footprint, and region.
    Source,
    /// The active declared location.
    Location,
    /// The whole realm, optionally restricted to one region.
    Realm {
        /// The region to include, or every region when absent.
        #[serde(default)]
        region: Option<crate::board::Region>,
    },
}

/// A controller relation used by a unit query.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum ControllerRelation {
    /// Either controller.
    #[default]
    Any,
    /// The source controller's units.
    Allied,
    /// Units controlled by the opposing seat.
    Enemy,
}

/// A bounded query describing a recipient cohort.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnitCohort {
    /// The area from which units are selected.
    pub area: UnitArea,
    /// Restricts the query to a unit kind, when present.
    #[serde(default)]
    pub kind: Option<UnitKind>,
    /// Restricts the query by controller.
    #[serde(default)]
    pub controller: ControllerRelation,
    /// Excludes the source unit from the result.
    #[serde(default)]
    pub exclude_source: bool,
}

/// A recipient cohort for an effect.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum UnitSet {
    /// The declared unit target.
    Target,
    /// The unit selected by a preceding ordinary choice.
    Chosen,
    /// Units matching a bounded query.
    Query(UnitCohort),
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

/// How long an authored temporary modifier remains active.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectDuration {
    /// Expires at the end of the current turn.
    ThisTurn,
    /// Expires at the source controller's next turn, before untapping.
    UntilYourNextTurn,
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
    Grant {
        /// The cohort receiving the modifier.
        recipients: UnitSet,
        /// The modifier to grant.
        modifier: TemporaryModifierKind,
        /// The modifier amount.
        amount: u16,
        /// The modifier duration.
        duration: EffectDuration,
    },
    /// Draws one card from either deck according to runtime rules.
    DrawCard,
    /// Grants stealth to a minion recipient cohort.
    GiveStealth {
        /// The cohort receiving stealth.
        recipients: UnitSet,
    },
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
    fn grant_requires_an_explicit_supported_duration() {
        let grant = serde_json::json!({
            "op": "grant",
            "recipients": {"query": {
                "area": {"realm": {"region": null}},
                "kind": "minion",
                "controller": "any",
                "excludeSource": false,
            }},
            "modifier": "power",
            "amount": 1,
        });
        assert!(serde_json::from_value::<Effect>(grant.clone()).is_err());
        for duration in ["this-turn", "until-your-next-turn"] {
            let mut valid = grant.clone();
            valid["duration"] = serde_json::json!(duration);
            let effect: Effect = serde_json::from_value(valid.clone()).unwrap();
            assert_eq!(serde_json::to_value(effect).unwrap(), valid);
        }
        let mut invalid = grant.clone();
        invalid["duration"] = serde_json::json!("permanent");
        assert!(serde_json::from_value::<Effect>(invalid).is_err());
        let mut obsolete = grant;
        obsolete["op"] = serde_json::json!("grant-this-turn");
        assert!(serde_json::from_value::<Effect>(obsolete).is_err());
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
            effects: vec![Effect::Grant {
                recipients: UnitSet::Target,
                modifier: TemporaryModifierKind::Airborne,
                amount: 1,
                duration: EffectDuration::ThisTurn,
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
            effects: vec![Effect::Grant {
                recipients: UnitSet::Target,
                modifier: TemporaryModifierKind::Power,
                amount: 1,
                duration: EffectDuration::ThisTurn,
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
            effects: vec![Effect::Grant {
                recipients: UnitSet::Target,
                modifier: TemporaryModifierKind::Charge,
                amount: 2,
                duration: EffectDuration::ThisTurn,
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
            effects: vec![Effect::Grant {
                recipients: UnitSet::Target,
                modifier: TemporaryModifierKind::Power,
                amount: 256,
                duration: EffectDuration::ThisTurn,
            }]
            .into_boxed_slice(),
        };
        assert!(power_overflow.validate().unwrap_err().contains("1-255"));

        let silence = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: vec![Effect::Grant {
                recipients: UnitSet::Query(UnitCohort {
                    area: UnitArea::Realm { region: None },
                    kind: Some(UnitKind::Minion),
                    controller: ControllerRelation::Any,
                    exclude_source: false,
                }),
                modifier: TemporaryModifierKind::Silence,
                amount: 1,
                duration: EffectDuration::ThisTurn,
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

    #[test]
    fn cohorts_round_trip_and_reject_unknown_or_invalid_scopes() {
        let value = serde_json::json!({
            "query": {
                "area": {"realm": {"region": "underwater"}},
                "kind": "minion",
                "controller": "enemy",
                "excludeSource": true,
            }
        });
        let set: UnitSet = serde_json::from_value(value.clone()).expect("valid cohort");
        assert_eq!(serde_json::to_value(set).expect("serialize cohort"), value);

        let unknown = serde_json::json!({
            "query": {"area": "source", "unexpected": true}
        });
        assert!(serde_json::from_value::<UnitSet>(unknown).is_err());

        let bad_scope = serde_json::json!({
            "query": {"area": "bogus"}
        });
        assert!(serde_json::from_value::<UnitSet>(bad_scope).is_err());
    }

    #[test]
    fn location_cohorts_require_location_selection() {
        let location = UnitSet::Query(UnitCohort {
            area: UnitArea::Location,
            kind: None,
            controller: ControllerRelation::Any,
            exclude_source: false,
        });
        let program = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: vec![Effect::Untap {
                recipients: location,
            }]
            .into_boxed_slice(),
        };
        assert!(
            program
                .validate()
                .unwrap_err()
                .contains("location selection")
        );

        let realm = UnitSet::Query(UnitCohort {
            area: UnitArea::Realm { region: None },
            kind: None,
            controller: ControllerRelation::Any,
            exclude_source: false,
        });
        let valid = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: vec![Effect::Untap { recipients: realm }].into_boxed_slice(),
        };
        valid
            .validate()
            .expect("realm query needs no declared location");
    }

    #[test]
    fn stealth_requires_minion_only_recipients() {
        let avatar = AbilityProgram {
            selection: Some(SelectionSpec::Unit {
                kind: Some(UnitKind::Avatar),
                relation: SpatialRelation::Anywhere,
            }),
            optional_selection: false,
            effects: vec![Effect::GiveStealth {
                recipients: UnitSet::Target,
            }]
            .into_boxed_slice(),
        };
        assert!(avatar.validate().unwrap_err().contains("minion-only"));

        let minion = AbilityProgram {
            selection: None,
            optional_selection: false,
            effects: vec![Effect::GiveStealth {
                recipients: UnitSet::Query(UnitCohort {
                    area: UnitArea::Realm { region: None },
                    kind: Some(UnitKind::Minion),
                    controller: ControllerRelation::Allied,
                    exclude_source: true,
                }),
            }]
            .into_boxed_slice(),
        };
        minion
            .validate()
            .expect("minion cohort can receive stealth");
    }
}
