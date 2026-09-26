//! Ordered work that resumes after an interrupting effect finishes.

use super::{
    CardId, CardInstance, Cell, DeferredMagicResolved, Game, GameError, IdentityHash, OutcomeLog,
    ResolutionContinuation, Seat, UnitPosition, seat_index,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TokenEntryContinuation {
    pub(super) seat: Seat,
    pub(super) token: UnitPosition,
    pub(super) source_instance_id: IdentityHash,
    pub(super) mana_paid: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SiteGenesisTail {
    pub(super) card_id: CardId,
    pub(super) card_instance_id: IdentityHash,
    pub(super) cell: Cell,
    pub(super) create_rubble_at: Option<Cell>,
    pub(super) defer_token: bool,
    pub(super) genesis_spell_draw_count: usize,
    pub(super) origin_state_version: u64,
    pub(super) seat: Seat,
    pub(super) abilities_lost: bool,
}

impl ResolutionContinuation {
    fn followed_by(self, next: Self) -> Self {
        let mut sequence = match self {
            Self::Sequence(sequence) => sequence,
            first => vec![first],
        };
        match next {
            Self::Sequence(next) => sequence.extend(next),
            next => sequence.push(next),
        }
        Self::Sequence(sequence)
    }

    pub(super) fn owns_magic_completion(&self) -> bool {
        match self {
            Self::Blink(_) | Self::LeapAttack(_) | Self::MagicResolved { .. } => true,
            Self::Effect(frame) => frame.magic.is_some(),
            Self::Sequence(steps) => steps.iter().any(Self::owns_magic_completion),
            Self::TriggerBatch(pending) => pending
                .continuation
                .as_ref()
                .is_some_and(Self::owns_magic_completion),
            _ => false,
        }
    }
}

impl Game {
    /// A simultaneous entry group finishes placing its members before any Genesis resolves.
    pub(super) fn finish_token_entries(
        &mut self,
        entries: Vec<TokenEntryContinuation>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.units.len().saturating_add(entries.len()) > 4096 {
            return Err(GameError::UnsupportedMechanic(
                "token entry exceeds the supported realm unit capacity".to_owned(),
            ));
        }
        let sources = entries
            .iter()
            .map(|entry| {
                (
                    entry.seat,
                    entry.token.card.instance_id.clone(),
                    entry.token.card.card_id,
                )
            })
            .collect::<Vec<_>>();
        for entry in entries {
            self.enter_token_unit(entry, outcomes)?;
        }
        self.begin_entry_equipment(sources, true, outcomes)
    }

    /// Entry equipment is created for the whole group before collecting suppressible Genesis.
    pub(super) fn begin_entry_equipment(
        &mut self,
        sources: Vec<(Seat, IdentityHash, CardId)>,
        settle_regions: bool,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let sources = sources
            .into_iter()
            .map(|(seat, instance_id, card_id)| {
                self.unit_reference(&super::UnitTarget::Minion { seat, instance_id })
                    .map(|reference| (seat, reference, card_id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (seat, reference, card_id) in &sources {
            if self
                .compiled_ability(*card_id, super::AbilityEntry::Entry)
                .is_some()
            {
                let source = self.unit_effect_source(&super::UnitTarget::Minion {
                    seat: *seat,
                    instance_id: reference.instance_id().clone(),
                })?;
                let frame = self.effect_frame(
                    *card_id,
                    super::AbilityEntry::Entry,
                    source,
                    None,
                    None,
                    None,
                )?;
                self.run_effect_frame(frame, outcomes)?;
            }
        }
        self.continue_resolution(
            ResolutionContinuation::EntryGenesis {
                sources,
                settle_regions,
            },
            outcomes,
        )
    }

    pub(super) fn finish_entry_genesis(
        &mut self,
        sources: Vec<(Seat, super::effect::RealmReference, CardId)>,
        settle_regions: bool,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let mut triggers = Vec::new();
        for (seat, reference, card_id) in sources {
            if self
                .position
                .units
                .iter()
                .any(|unit| reference.matches(&unit.card))
                && let Some(trigger) =
                    self.genesis_trigger(seat, reference.instance_id(), card_id)?
            {
                triggers.push(trigger);
            }
        }
        if settle_regions {
            self.settle_region_occupancy(outcomes)?;
        }
        self.begin_genesis_triggers(triggers, outcomes)
    }

    pub(super) fn finish_magic_resolution(
        &mut self,
        resolution: &DeferredMagicResolved,
        held_card: Option<CardInstance>,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        if let Some(card) = held_card {
            self.position.players[seat_index(card.owner)]
                .cemetery
                .push(card);
        }
        Self::emit_continuation_magic_resolved(
            &self.rules.cards[usize::from(resolution.card_id.0)].id,
            &resolution.instance_id,
            resolution.owner,
            outcomes,
        );
    }

    /// Keep the remaining work behind the current interruption, or run it immediately.
    pub(super) fn continue_resolution(
        &mut self,
        continuation: ResolutionContinuation,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.terminal.is_some() {
            self.emit_interrupted_magic_resolved(Some(&continuation), outcomes);
        } else if let Some(pending) = &mut self.position.pending_deathrites {
            pending.continuation = Some(match pending.continuation.take() {
                Some(first) => first.followed_by(continuation),
                None => continuation,
            });
        } else if let Some(pending) = &mut self.position.pending_trigger_order {
            pending.continuation = Some(match pending.continuation.take() {
                Some(first) => first.followed_by(continuation),
                None => continuation,
            });
        } else if let Some(pending) = &mut self.position.pending_ability_choice {
            pending.continuation = Some(match pending.continuation.take() {
                Some(first) => first.followed_by(continuation),
                None => continuation,
            });
        } else {
            self.resume_resolution_continuation(
                continuation,
                self.position.phase,
                self.position.decision_seat,
                outcomes,
                None,
            )?;
        }
        Ok(())
    }
}
