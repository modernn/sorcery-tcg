//! Controller-ordered damage replacements, before prevention and source consumption.

use super::{
    ActionDescriptor, Game, GameError, IdentityHash, IssuedAction, OutcomeLog, Phase,
    RealmReference, Seat, StrikeStats, UnitKind, UnitTarget, Value, json, other_seat,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DamageOperation {
    Add(u16),
    Double,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DamageReplacement {
    controller: Option<Seat>,
    owner: Seat,
    operation: DamageOperation,
    source: RealmReference,
}

impl DamageReplacement {
    fn choosing_seat(&self) -> Seat {
        self.controller.unwrap_or(self.owner)
    }

    fn value(&self) -> Value {
        let (operation, amount) = match self.operation {
            DamageOperation::Add(amount) => ("add", amount),
            DamageOperation::Double => ("double", 2),
        };
        json!({"controller": self.controller, "owner": self.owner, "choosingSeat": self.choosing_seat(), "operation": operation,
            "amount": amount, "source": self.source.value()})
    }
}

/// The remainder of a ranged strike, held while its replacement order is chosen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RangedDamage {
    pub(super) strike: StrikeStats,
    pub(super) target: UnitTarget,
    pub(super) shooter: IdentityHash,
    pub(super) seat: Seat,
    pub(super) return_phase: Phase,
    pub(super) step_after: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingDamageOrder {
    amount: u16,
    active_seat: Seat,
    remaining: Vec<DamageReplacement>,
    continuation: RangedDamage,
}

impl PendingDamageOrder {
    fn choosing_seat(&self) -> Option<Seat> {
        [self.active_seat, other_seat(self.active_seat)]
            .into_iter()
            .find(|seat| {
                self.remaining
                    .iter()
                    .any(|effect| effect.choosing_seat() == *seat)
            })
    }

    fn needs_choice(&self, seat: Seat) -> bool {
        let mut add = false;
        let mut double = false;
        for effect in self
            .remaining
            .iter()
            .filter(|effect| effect.choosing_seat() == seat)
        {
            match effect.operation {
                DamageOperation::Add(_) => add = true,
                DamageOperation::Double => double = true,
            }
        }
        add && double
    }

    fn apply(&mut self, index: usize, outcomes: &mut OutcomeLog<'_>) -> Result<(), GameError> {
        let replacement = self.remaining.get(index).ok_or(GameError::IllegalAction)?;
        if Some(replacement.choosing_seat()) != self.choosing_seat() {
            return Err(GameError::IllegalAction);
        }
        let before = self.amount;
        self.amount = match replacement.operation {
            DamageOperation::Add(bonus) => before.checked_add(bonus),
            DamageOperation::Double => before.checked_mul(2),
        }
        .ok_or(GameError::IllegalAction)?;
        outcomes.push("damage-modifier-applied", || {
            json!({
                "before": before, "amount": self.amount, "modifier": replacement.value(),
                "strikerInstanceId": self.continuation.shooter,
                "targetInstanceId": self.continuation.target.instance_id(),
            })
        });
        self.remaining.remove(index);
        Ok(())
    }

    pub(super) fn value(&self) -> Value {
        let continuation = &self.continuation;
        let strike = &continuation.strike;
        json!({
            "activeSeat": self.active_seat,
            "amount": self.amount,
            "remaining": self.remaining.iter().map(DamageReplacement::value).collect::<Vec<_>>(),
            "continuation": {
                "kind": "ranged-strike", "shooterInstanceId": continuation.shooter,
                "seat": continuation.seat, "target": continuation.target,
                "returnPhase": continuation.return_phase.as_str(), "stepAfter": continuation.step_after,
                "strike": {"amount": strike.amount, "additiveBonus": strike.additive_bonus,
                    "currentPower": strike.current_power, "lethal": strike.lethal,
                    "lanceCount": strike.lance_count,
                    "consumedArtifacts": strike.consumed_artifacts.iter().map(RealmReference::value).collect::<Vec<_>>()},
            },
        })
    }
}

impl Game {
    pub(super) fn begin_ranged_damage_order(
        &mut self,
        continuation: RangedDamage,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let mut remaining = Vec::new();
        let strike = &continuation.strike;
        for _ in 0..strike.lance_count {
            remaining.push(DamageReplacement {
                controller: Some(continuation.seat),
                owner: self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == continuation.shooter)
                    .ok_or(GameError::IllegalAction)?
                    .card
                    .owner,
                operation: DamageOperation::Add(1),
                source: self.unit_reference(&UnitTarget::Minion {
                    instance_id: continuation.shooter.clone(),
                    seat: continuation.seat,
                })?,
            });
        }
        for artifact in &self.position.artifacts {
            if artifact.carried_by(UnitKind::Minion, continuation.seat, &continuation.shooter)
                && let Some(modifier) = self.artifact_facts(artifact)?.bearer_unit_strike
                && modifier.damage_bonus > 0
            {
                remaining.push(DamageReplacement {
                    controller: Some(continuation.seat),
                    owner: artifact.card.owner,
                    operation: DamageOperation::Add(u16::from(modifier.damage_bonus)),
                    source: RealmReference::from_card(&artifact.card),
                });
            }
        }
        let target_kind = match continuation.target {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        for artifact in self.nearby_unit_strike_sources(
            target_kind,
            continuation.target.seat(),
            continuation.target.instance_id(),
        )? {
            // A loose carriable artifact is uncontrolled; its owner orders its effects.
            remaining.push(DamageReplacement {
                controller: artifact.bearer().map(UnitTarget::seat),
                owner: artifact.card.owner,
                operation: DamageOperation::Double,
                source: RealmReference::from_card(&artifact.card),
            });
        }
        self.position.pending_damage_order = Some(Box::new(PendingDamageOrder {
            amount: strike
                .amount
                .checked_sub(strike.additive_bonus)
                .ok_or(GameError::IllegalAction)?,
            active_seat: self.position.active_seat,
            remaining,
            continuation,
        }));
        self.advance_damage_order(false, outcomes)
    }

    fn advance_damage_order(
        &mut self,
        resumed: bool,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let mut pending = self
            .position
            .pending_damage_order
            .take()
            .ok_or(GameError::IllegalAction)?;
        while let Some(seat) = pending.choosing_seat() {
            if pending.needs_choice(seat) {
                self.position.phase = Phase::DamageOrder;
                self.position.decision_seat = seat;
                self.position.pending_damage_order = Some(pending);
                return Ok(());
            }
            // Same-kind arithmetic commutes: no factorial enumeration or meaningless choices.
            let index = pending
                .remaining
                .iter()
                .position(|effect| effect.choosing_seat() == seat)
                .ok_or(GameError::IllegalAction)?;
            pending.apply(index, outcomes)?;
        }
        let continuation = &pending.continuation;
        self.position.phase = continuation.return_phase;
        self.position.decision_seat = continuation.seat;
        self.finish_ranged_damage(
            &continuation.strike,
            &continuation.target,
            (continuation.seat, &continuation.shooter),
            continuation.return_phase,
            pending.amount,
            outcomes,
        )?;
        if resumed && continuation.step_after {
            self.queue_ranged_step(continuation.seat, &continuation.shooter)?;
        }
        Ok(())
    }

    pub(super) fn append_damage_order_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_damage_order
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let seat = pending.choosing_seat().ok_or(GameError::IllegalAction)?;
        if seat != self.position.decision_seat {
            return Err(GameError::IllegalAction);
        }
        for (index, replacement) in pending.remaining.iter().enumerate() {
            if replacement.choosing_seat() == seat {
                let descriptor = ActionDescriptor::ChooseDamageModifier {
                    modifier_index: u64::try_from(index).map_err(|_| GameError::IllegalAction)?,
                };
                let label = match replacement.operation {
                    DamageOperation::Add(amount) => format!("Apply +{amount} damage"),
                    DamageOperation::Double => "Double damage".to_owned(),
                };
                self.push_action(actions, descriptor, label);
            }
        }
        Ok(())
    }

    pub(super) fn apply_damage_order_action(
        &mut self,
        index: u64,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.phase != Phase::DamageOrder {
            return Err(GameError::IllegalAction);
        }
        let pending = self
            .position
            .pending_damage_order
            .as_mut()
            .ok_or(GameError::IllegalAction)?;
        pending.apply(
            usize::try_from(index).map_err(|_| GameError::IllegalAction)?,
            outcomes,
        )?;
        self.advance_damage_order(true, outcomes)?;
        self.position.state_version += 1;
        Ok(())
    }
}
