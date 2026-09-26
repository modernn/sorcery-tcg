//! Engine-issued unit choices during an effect's resolution.

use super::ability::{SpatialRelation, UnitChoiceSpec};
use super::effect::EffectSource;
use super::{
    ActionDescriptor, EffectFrame, Game, GameError, IdentityHash, IssuedAction, OutcomeLog, Phase,
    ResolutionContinuation, Seat, UnitQuery, UnitTarget, Value, json,
};
use crate::action::DeckZone;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingAbilityChoice {
    pub(super) frame: Box<EffectFrame>,
    pub(super) spec: AbilityChoiceSpec,
    pub(super) continuation: Option<ResolutionContinuation>,
    pub(super) return_phase: Phase,
    pub(super) return_decision_seat: Seat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AbilityChoiceSpec {
    Unit(UnitChoiceSpec),
    DrawCard,
}

impl Game {
    pub(super) fn ability_unit_choices(
        &self,
        source: &EffectSource,
        spec: UnitChoiceSpec,
    ) -> Vec<UnitTarget> {
        self.selected_units(
            UnitQuery {
                region: (spec.relation != SpatialRelation::Anywhere).then_some(source.region),
                cells: Some(&source.cells),
                kind: spec.kind,
                controller: spec.allied_only.then_some(source.controller),
                exclude: source
                    .realm
                    .as_ref()
                    .filter(|reference| {
                        spec.exclude_source && self.realm_reference_exists(reference)
                    })
                    .map(super::effect::RealmReference::instance_id),
            },
            spec.relation,
            None,
        )
    }

    pub(super) fn begin_ability_unit_choice(
        &mut self,
        mut frame: EffectFrame,
        spec: UnitChoiceSpec,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if spec.relation != SpatialRelation::Anywhere {
            (frame.source.region, frame.source.cells) = self.effect_anchor(&frame.source)?;
        }
        self.select_effect_unit(&mut frame, None)?;
        if self.ability_unit_choices(&frame.source, spec).is_empty() {
            return self.run_effect_frame(frame, outcomes);
        }
        let controller = frame.source.controller;
        self.position.pending_ability_choice = Some(Box::new(PendingAbilityChoice {
            frame: Box::new(frame),
            spec: AbilityChoiceSpec::Unit(spec),
            continuation: None,
            return_phase: self.position.phase,
            return_decision_seat: self.position.decision_seat,
        }));
        self.position.phase = Phase::AbilityChoice;
        self.position.decision_seat = controller;
        Ok(())
    }

    pub(super) fn begin_ability_draw_choice(&mut self, frame: EffectFrame) {
        let controller = frame.source.controller;
        self.position.pending_ability_choice = Some(Box::new(PendingAbilityChoice {
            frame: Box::new(frame),
            spec: AbilityChoiceSpec::DrawCard,
            continuation: None,
            return_phase: self.position.phase,
            return_decision_seat: self.position.decision_seat,
        }));
        self.position.phase = Phase::AbilityChoice;
        self.position.decision_seat = controller;
    }

    pub(super) fn append_ability_choice_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
    ) -> Result<(), GameError> {
        let Some(pending) = &self.position.pending_ability_choice else {
            return self.append_trigger_order_actions(actions);
        };
        match pending.spec {
            AbilityChoiceSpec::Unit(spec) => {
                for target in self.ability_unit_choices(&pending.frame.source, spec) {
                    self.push_ability_choice(
                        actions,
                        &pending.frame.source.instance_id,
                        Some(target),
                    );
                }
                if spec.optional {
                    self.push_ability_choice(actions, &pending.frame.source.instance_id, None);
                }
            }
            AbilityChoiceSpec::DrawCard => {
                for zone in [DeckZone::Atlas, DeckZone::Spellbook] {
                    self.push_ability_draw_choice(actions, &pending.frame.source.instance_id, zone);
                }
            }
        }
        Ok(())
    }

    pub(super) fn push_ability_choice(
        &self,
        actions: &mut Vec<IssuedAction>,
        source: &IdentityHash,
        target: Option<UnitTarget>,
    ) {
        let descriptor = ActionDescriptor::ChooseAbility {
            source_instance_id: source.clone(),
            target,
        };
        let label = descriptor
            .state_independent_label()
            .expect("ability choice label");
        self.push_action(actions, descriptor, label);
    }

    pub(super) fn apply_ability_choice_action(
        &mut self,
        seat: Seat,
        source: &IdentityHash,
        target: Option<&UnitTarget>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let Some(pending) = &self.position.pending_ability_choice else {
            return self.apply_genesis_target_choice(seat, source, target, outcomes);
        };
        let AbilityChoiceSpec::Unit(spec) = pending.spec else {
            return Err(GameError::IllegalAction);
        };
        if self.position.phase != Phase::AbilityChoice
            || seat != pending.frame.source.controller
            || source != &pending.frame.source.instance_id
            || match target {
                None => !spec.optional,
                Some(target) => !self
                    .ability_unit_choices(&pending.frame.source, spec)
                    .contains(target),
            }
        {
            return Err(GameError::IllegalAction);
        }
        let mut pending = self
            .position
            .pending_ability_choice
            .take()
            .expect("pending choice");
        self.select_effect_unit(&mut pending.frame, target)?;
        Self::emit_ability_choice(seat, source, target, outcomes);
        self.resume_ability_choice(*pending, outcomes)?;
        self.position.state_version += 1;
        Ok(())
    }

    pub(super) fn push_ability_draw_choice(
        &self,
        actions: &mut Vec<IssuedAction>,
        source: &IdentityHash,
        zone: DeckZone,
    ) {
        let descriptor = ActionDescriptor::ChooseAbilityDraw {
            source_instance_id: source.clone(),
            zone,
        };
        let label = descriptor
            .state_independent_label()
            .expect("ability draw choice label");
        self.push_action(actions, descriptor, label);
    }

    pub(super) fn apply_ability_draw_choice_action(
        &mut self,
        seat: Seat,
        source: &IdentityHash,
        zone: DeckZone,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let Some(pending) = &self.position.pending_ability_choice else {
            return Err(GameError::IllegalAction);
        };
        if self.position.phase != Phase::AbilityChoice
            || seat != pending.frame.source.controller
            || source != &pending.frame.source.instance_id
            || pending.spec != AbilityChoiceSpec::DrawCard
        {
            return Err(GameError::IllegalAction);
        }
        let pending = self
            .position
            .pending_ability_choice
            .take()
            .expect("pending draw choice");
        outcomes.push(
            "ability-draw-choice-committed",
            || json!({"seat": seat, "sourceInstanceId": source, "zone": zone}),
        );
        self.apply_genesis_draws(seat, source, zone, 1, outcomes);
        self.resume_ability_choice(*pending, outcomes)?;
        self.position.state_version += 1;
        Ok(())
    }

    fn resume_ability_choice(
        &mut self,
        pending: PendingAbilityChoice,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.terminal.is_none() {
            self.position.phase = pending.return_phase;
            self.position.decision_seat = pending.return_decision_seat;
        }
        let mut steps = vec![ResolutionContinuation::Effect(pending.frame)];
        steps.extend(pending.continuation);
        self.continue_resolution(ResolutionContinuation::Sequence(steps), outcomes)
    }

    /// Settlement may remove every eligible unit while an interrupted choice is pending.
    pub(super) fn resume_empty_ability_choice(
        &mut self,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.terminal.is_none()
            && self.position.pending_deathrites.is_none()
            && self.position.pending_trigger_order.is_none()
            && self
                .position
                .pending_ability_choice
                .as_ref()
                .is_some_and(|pending| {
                    matches!(pending.spec, AbilityChoiceSpec::Unit(spec)
                        if self.ability_unit_choices(&pending.frame.source, spec).is_empty())
                })
        {
            let mut pending = self
                .position
                .pending_ability_choice
                .take()
                .expect("empty choice");
            self.select_effect_unit(&mut pending.frame, None)?;
            self.resume_ability_choice(*pending, outcomes)?;
        }
        Ok(())
    }

    pub(super) fn emit_ability_choice(
        seat: Seat,
        source: &IdentityHash,
        target: Option<&UnitTarget>,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        outcomes.push(
            "ability-choice-committed",
            || json!({"seat": seat, "sourceInstanceId": source, "target": target}),
        );
    }

    pub(super) fn ability_choice_value(&self, pending: &PendingAbilityChoice) -> Value {
        // The immutable program and cursor identify the selector; do not duplicate its facts.
        json!({"frame": self.effect_frame_value(&pending.frame),
            "choice": match pending.spec {
                AbilityChoiceSpec::Unit(_) => "unit",
                AbilityChoiceSpec::DrawCard => "draw-card",
            },
            "returnPhase": pending.return_phase.as_str(),
            "returnDecisionSeat": pending.return_decision_seat,
            "continuation": pending.continuation.as_ref().map(|tail| self.resolution_continuation_value(tail))})
    }
}
