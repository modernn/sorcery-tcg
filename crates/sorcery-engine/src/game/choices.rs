//! Engine-issued unit choices during an effect's resolution.

use super::ability::UnitChoiceSpec;
use super::effect::EffectSource;
use super::{
    ActionDescriptor, EffectFrame, Game, GameError, IdentityHash, IssuedAction, OutcomeLog, Phase,
    ResolutionContinuation, Seat, UnitQuery, UnitTarget, Value, json,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingAbilityChoice {
    pub(super) frame: Box<EffectFrame>,
    pub(super) spec: UnitChoiceSpec,
    pub(super) continuation: Option<ResolutionContinuation>,
    pub(super) return_phase: Phase,
    pub(super) return_decision_seat: Seat,
}

impl Game {
    pub(super) fn ability_unit_choices(
        &self,
        source: &EffectSource,
        spec: UnitChoiceSpec,
    ) -> Vec<UnitTarget> {
        self.selected_units(
            UnitQuery {
                region: source.region,
                cells: Some(&source.cells),
                kind: spec.kind,
                controller: spec.allied_only.then_some(source.controller),
                exclude: None,
            },
            spec.relation,
            None,
        )
    }

    pub(super) fn begin_ability_unit_choice(
        &mut self,
        frame: EffectFrame,
        spec: UnitChoiceSpec,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.ability_unit_choices(&frame.source, spec).is_empty() {
            return self.run_effect_frame(frame, outcomes);
        }
        let controller = frame.source.controller;
        self.position.pending_ability_choice = Some(Box::new(PendingAbilityChoice {
            frame: Box::new(frame),
            spec,
            continuation: None,
            return_phase: self.position.phase,
            return_decision_seat: self.position.decision_seat,
        }));
        self.position.phase = Phase::AbilityChoice;
        self.position.decision_seat = controller;
        Ok(())
    }

    pub(super) fn append_ability_choice_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
    ) -> Result<(), GameError> {
        let Some(pending) = &self.position.pending_ability_choice else {
            return self.append_trigger_order_actions(actions);
        };
        for target in self.ability_unit_choices(&pending.frame.source, pending.spec) {
            self.push_ability_choice(actions, &pending.frame.source.instance_id, Some(target));
        }
        if pending.spec.optional {
            self.push_ability_choice(actions, &pending.frame.source.instance_id, None);
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
        if self.position.phase != Phase::AbilityChoice
            || seat != pending.frame.source.controller
            || source != &pending.frame.source.instance_id
            || match target {
                None => !pending.spec.optional,
                Some(target) => !self
                    .ability_unit_choices(&pending.frame.source, pending.spec)
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

    fn resume_ability_choice(
        &mut self,
        pending: PendingAbilityChoice,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        self.position.phase = pending.return_phase;
        self.position.decision_seat = pending.return_decision_seat;
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
                    self.ability_unit_choices(&pending.frame.source, pending.spec)
                        .is_empty()
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
            "returnPhase": pending.return_phase.as_str(),
            "returnDecisionSeat": pending.return_decision_seat,
            "continuation": pending.continuation.as_ref().map(|tail| self.resolution_continuation_value(tail))})
    }
}
