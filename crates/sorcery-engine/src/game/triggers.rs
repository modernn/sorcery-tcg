//! Trigger ordering shares a kernel; an interrupted death chain retains its own cleanup barrier.

use super::effect::{EffectFrame, EffectSource};
use super::{
    AbilityEntry, ActionDescriptor, CardFacts, CardId, EngineRandomDraw, Game, GameError,
    IdentityHash, IssuedAction, OutcomeLog, Phase, ResolutionContinuation, Seat, TriggerBatch,
    TriggerOrderStage, TriggerSource, UnitTarget, Value, json,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum GenesisTrigger {
    Compiled(EffectFrame),
    Legacy {
        card_id: CardId,
        source: EffectSource,
    },
}

impl GenesisTrigger {
    fn card_id(&self) -> CardId {
        match self {
            Self::Compiled(frame) => frame.card_id,
            Self::Legacy { card_id, .. } => *card_id,
        }
    }
    fn source(&self) -> &EffectSource {
        match self {
            Self::Compiled(frame) => &frame.source,
            Self::Legacy { source, .. } => source,
        }
    }
}

impl TriggerSource for GenesisTrigger {
    fn controller(&self) -> Seat {
        self.source().controller
    }
    fn instance_id(&self) -> &IdentityHash {
        &self.source().instance_id
    }
    fn needs_declaration(&self) -> bool {
        matches!(self, Self::Compiled(frame) if frame.declaration_pending)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingTriggerOrder {
    pub(super) batch: TriggerBatch<GenesisTrigger>,
    pub(super) continuation: Option<ResolutionContinuation>,
    pub(super) return_phase: Phase,
    pub(super) return_decision_seat: Seat,
}

impl Game {
    pub(super) fn genesis_trigger(
        &self,
        seat: Seat,
        instance_id: &IdentityHash,
        card_id: CardId,
    ) -> Result<Option<GenesisTrigger>, GameError> {
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(card_id.0)].facts else {
            return Err(GameError::IllegalAction);
        };
        if super::ability::genesis_clause_count(facts) == 0 {
            return Ok(None);
        }
        let unit = self
            .position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == *instance_id)
            .ok_or(GameError::IllegalAction)?;
        if self.minion_abilities_lost(unit) {
            return Ok(None);
        }
        let mut source = self.unit_effect_source(&UnitTarget::Minion {
            seat: unit.controller,
            instance_id: instance_id.clone(),
        })?;
        source.controller = seat;
        if self.rules.cards[usize::from(card_id.0)]
            .abilities
            .genesis
            .is_some()
        {
            Ok(Some(GenesisTrigger::Compiled(self.effect_frame(
                card_id,
                AbilityEntry::Genesis,
                source,
                None,
                None,
                None,
            )?)))
        } else if facts.genesis_may_damage_target_adjacent_unit
            || facts.genesis_untap_adjacent_allies
        {
            Err(GameError::UnsupportedMechanic(
                "mixed Genesis choices require an explicitly ordered compiled program".to_owned(),
            ))
        } else {
            Ok(Some(GenesisTrigger::Legacy { card_id, source }))
        }
    }

    pub(super) fn begin_genesis_triggers(
        &mut self,
        mut triggers: Vec<GenesisTrigger>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if triggers.len() == 1 && !triggers[0].needs_declaration() {
            return self.continue_resolution(
                ResolutionContinuation::Genesis(triggers.pop().expect("one trigger")),
                outcomes,
            );
        }
        if triggers.iter().any(|trigger| {
            self.rules.cards[usize::from(trigger.card_id().0)]
                .abilities
                .genesis
                .is_none()
        }) {
            return Err(GameError::UnsupportedMechanic(
                "simultaneous Genesis requires compiled effects and post-entry choices".to_owned(),
            ));
        }
        let Some(batch) = TriggerBatch::new(triggers, self.position.active_seat) else {
            return Ok(());
        };
        self.continue_resolution(
            ResolutionContinuation::TriggerBatch(Box::new(PendingTriggerOrder {
                batch,
                continuation: None,
                return_phase: self.position.phase,
                return_decision_seat: self.position.decision_seat,
            })),
            outcomes,
        )
    }

    pub(super) fn drive_trigger_batch(
        &mut self,
        pending: Box<PendingTriggerOrder>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if let Some(sources) = pending.batch.pending_order() {
            self.position.phase = if sources.len() == 1 {
                Phase::AbilityChoice
            } else {
                Phase::TriggerOrder
            };
            self.position.decision_seat = sources[0].controller();
            self.position.pending_trigger_order = Some(pending);
            return Ok(());
        }
        let PendingTriggerOrder {
            batch,
            continuation,
            return_phase,
            return_decision_seat,
        } = *pending;
        self.position.phase = return_phase;
        self.position.decision_seat = return_decision_seat;
        let mut steps = batch
            .resolving
            .into_iter()
            .map(ResolutionContinuation::Genesis)
            .collect::<Vec<_>>();
        steps.extend(continuation);
        // Each nested death chain finalizes its corpses before the next queued Genesis.
        self.continue_resolution(ResolutionContinuation::Sequence(steps), outcomes)
    }

    pub(super) fn resolve_genesis_trigger(
        &mut self,
        trigger: GenesisTrigger,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        match trigger {
            GenesisTrigger::Compiled(frame) => self.run_effect_frame(frame, outcomes),
            GenesisTrigger::Legacy { card_id, source } => {
                let reference = source.realm.as_ref().ok_or(GameError::IllegalAction)?;
                if !self.realm_reference_exists(reference) {
                    return Ok(());
                }
                let unit = self
                    .position
                    .units
                    .iter()
                    .find(|unit| reference.matches(&unit.card))
                    .ok_or(GameError::IllegalAction)?;
                if unit.controller != source.controller {
                    return Err(GameError::UnsupportedMechanic(
                        "control changed before an uncompiled Genesis resolved".to_owned(),
                    ));
                }
                self.apply_legacy_minion_genesis(
                    source.controller,
                    &source.instance_id,
                    card_id,
                    outcomes,
                )
            }
        }
    }

    pub(super) fn append_trigger_order_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
    ) -> Result<(), GameError> {
        if let Some(pending) = &self.position.pending_deathrites {
            let sources = pending
                .batches
                .first()
                .and_then(TriggerBatch::pending_order)
                .ok_or(GameError::IllegalAction)?;
            for source in sources {
                self.push_trigger_order_action(
                    actions,
                    &source.instance_id,
                    source.unit.card.card_id,
                    source.controller,
                )?;
            }
        } else {
            let sources = self
                .position
                .pending_trigger_order
                .as_ref()
                .and_then(|pending| pending.batch.pending_order())
                .ok_or(GameError::IllegalAction)?;
            for source in sources {
                if source.needs_declaration() {
                    let ability = self
                        .compiled_ability(source.card_id(), AbilityEntry::Genesis)
                        .ok_or(GameError::IllegalAction)?;
                    for choice in self.selection_choices(
                        source.controller(),
                        source.source().region,
                        &source.source().cells,
                        ability.selection,
                    ) {
                        if let Some(target) = choice.target {
                            self.push_ability_choice(actions, source.instance_id(), Some(target));
                        }
                    }
                    if ability.optional_selection {
                        self.push_ability_choice(actions, source.instance_id(), None);
                    }
                } else {
                    self.push_trigger_order_action(
                        actions,
                        source.instance_id(),
                        source.card_id(),
                        source.controller(),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn push_trigger_order_action(
        &self,
        actions: &mut Vec<IssuedAction>,
        instance_id: &IdentityHash,
        card_id: CardId,
        controller: Seat,
    ) -> Result<(), GameError> {
        if controller != self.position.decision_seat {
            return Err(GameError::IllegalAction);
        }
        self.push_action(
            actions,
            ActionDescriptor::OrderTriggers {
                source_instance_id: instance_id.clone(),
            },
            format!(
                "Order {} first within your triggers",
                self.rules.cards[usize::from(card_id.0)].id
            ),
        );
        Ok(())
    }

    pub(super) fn apply_genesis_target_choice(
        &mut self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        target: Option<&UnitTarget>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_trigger_order
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        if !matches!(
            self.position.phase,
            Phase::TriggerOrder | Phase::AbilityChoice
        ) || seat != self.position.decision_seat
        {
            return Err(GameError::IllegalAction);
        }
        let source = pending
            .batch
            .pending_order()
            .ok_or(GameError::IllegalAction)?
            .iter()
            .find(|source| source.instance_id() == source_instance_id)
            .ok_or(GameError::IllegalAction)?;
        if !source.needs_declaration() || source.controller() != seat {
            return Err(GameError::IllegalAction);
        }
        let ability = self.rules.cards[usize::from(source.card_id().0)]
            .abilities
            .genesis
            .as_ref()
            .ok_or(GameError::UnsupportedMechanic(
                "Genesis declaration requires a compiled ability".to_owned(),
            ))?;
        let choices = self.selection_choices(
            source.controller(),
            source.source().region,
            &source.source().cells,
            ability.selection,
        );
        let valid = target.is_some_and(|target| {
            choices
                .iter()
                .any(|choice| choice.target.as_ref() == Some(target))
        }) || target.is_none() && ability.optional_selection;
        if !valid {
            return Err(GameError::IllegalAction);
        }
        let mut pending = self
            .position
            .pending_trigger_order
            .take()
            .ok_or(GameError::IllegalAction)?;
        let source = pending
            .batch
            .pending_order()
            .ok_or(GameError::IllegalAction)?
            .iter()
            .position(|source| source.instance_id() == source_instance_id)
            .ok_or(GameError::IllegalAction)?;
        let trigger = match pending.batch.stage {
            TriggerOrderStage::ActiveOrder => pending.batch.active_remaining.get_mut(source),
            TriggerOrderStage::NonActiveOrder => pending.batch.non_active_remaining.get_mut(source),
            TriggerOrderStage::Resolve => None,
        }
        .ok_or(GameError::IllegalAction)?;
        let GenesisTrigger::Compiled(frame) = trigger else {
            return Err(GameError::IllegalAction);
        };
        self.declare_effect_target(frame, target)?;
        pending.batch.commit(source_instance_id)?;
        Self::emit_ability_choice(seat, source_instance_id, target, outcomes);
        self.drive_trigger_batch(pending, outcomes)?;
        self.position.state_version += 1;
        Ok(())
    }

    pub(super) fn apply_trigger_order_action(
        &mut self,
        seat: Seat,
        instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
        random_draws: Option<&mut Vec<EngineRandomDraw>>,
    ) -> Result<(), GameError> {
        if self.position.pending_deathrites.is_some() {
            return self.apply_deathrite_order_action(seat, instance_id, outcomes, random_draws);
        }
        if self.position.phase != Phase::TriggerOrder
            || seat != self.position.decision_seat
            || !self
                .position
                .pending_trigger_order
                .as_ref()
                .and_then(|pending| pending.batch.pending_order())
                .is_some_and(|sources| {
                    sources.iter().any(|source| {
                        source.instance_id() == instance_id && !source.needs_declaration()
                    })
                })
        {
            return Err(GameError::IllegalAction);
        }
        let mut pending = self
            .position
            .pending_trigger_order
            .take()
            .ok_or(GameError::IllegalAction)?;
        pending.batch.commit(instance_id)?;
        outcomes.push(
            "trigger-order-committed",
            || json!({"seat":seat,"sourceInstanceId":instance_id}),
        );
        self.drive_trigger_batch(pending, outcomes)?;
        self.position.state_version += 1;
        Ok(())
    }

    pub(super) fn genesis_trigger_value(&self, trigger: &GenesisTrigger) -> Value {
        match trigger {
            GenesisTrigger::Compiled(frame) => {
                json!({"kind":"genesis", "frame":self.effect_frame_value(frame)})
            }
            GenesisTrigger::Legacy { card_id, source } => json!({"kind":"genesis",
                "cardId":self.rules.cards[usize::from(card_id.0)].id, "source":source.value()}),
        }
    }

    pub(super) fn trigger_order_value(&self, pending: &PendingTriggerOrder) -> Value {
        json!({"kind":"trigger-batch", "batch":Self::trigger_batch_value(&pending.batch, |source| self.genesis_trigger_value(source)),
            "returnPhase":pending.return_phase.as_str(), "returnDecisionSeat":pending.return_decision_seat,
            "continuation":pending.continuation.as_ref().map(|tail| self.resolution_continuation_value(tail))})
    }

    pub(super) fn trigger_batch_value<T: TriggerSource>(
        batch: &TriggerBatch<T>,
        value: impl Fn(&T) -> Value,
    ) -> Value {
        let sources = |items: &[T]| Value::Array(items.iter().map(&value).collect());
        json!({"activeOrder":sources(&batch.active_order), "activeRemaining":sources(&batch.active_remaining),
            "nonActiveOrder":sources(&batch.non_active_order), "nonActiveRemaining":sources(&batch.non_active_remaining),
            "resolving":sources(&batch.resolving), "stage":match batch.stage {
                TriggerOrderStage::ActiveOrder=>"active-order", TriggerOrderStage::NonActiveOrder=>"non-active-order",
                TriggerOrderStage::Resolve=>"resolve"}})
    }
}
