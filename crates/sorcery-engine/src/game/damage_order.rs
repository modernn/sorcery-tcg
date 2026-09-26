//! Controller-ordered damage replacements, before prevention and source consumption.

use super::{
    ActionDescriptor, Game, GameError, IdentityHash, IssuedAction, OutcomeLog, PendingCombat,
    Phase, RealmReference, ResolutionContinuation, Seat, StrikeStats, UnitKind, UnitTarget, Value,
    json, other_seat,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DamageOperation {
    Add(u16),
    Double,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DamageReplacement {
    damage_index: usize,
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
        json!({"damageIndex": self.damage_index, "controller": self.controller, "owner": self.owner, "choosingSeat": self.choosing_seat(), "operation": operation,
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
struct DamagePacket {
    amount: u16,
    striker: UnitTarget,
    target: UnitTarget,
}

/// No realm mutations occur between preparing these packets and finishing the fight.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FightDamage {
    pub(super) pending: PendingCombat,
    pub(super) attacker_strikes: bool,
    pub(super) striking_combatant_ids: Vec<IdentityHash>,
    pub(super) continuation: Option<ResolutionContinuation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DamageContinuation {
    Ranged(RangedDamage),
    Fight(Box<FightDamage>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingDamageOrder {
    damage: Vec<DamagePacket>,
    active_seat: Seat,
    remaining: Vec<DamageReplacement>,
    continuation: DamageContinuation,
}

impl PendingDamageOrder {
    // Independent packets commute. Finish the active player bucket first; within
    // each packet this still strictly precedes the non-active player replacements.
    fn choosing_seat(&self) -> Option<Seat> {
        [self.active_seat, other_seat(self.active_seat)]
            .into_iter()
            .find(|seat| {
                self.remaining
                    .iter()
                    .any(|effect| effect.choosing_seat() == *seat)
            })
    }

    fn current_damage_index(&self, seat: Seat) -> Option<usize> {
        self.remaining
            .iter()
            .find(|effect| effect.choosing_seat() == seat)
            .map(|effect| effect.damage_index)
    }

    fn needs_choice(&self, seat: Seat) -> bool {
        let mut add = false;
        let mut double = false;
        for effect in self.remaining.iter().filter(|effect| {
            effect.choosing_seat() == seat
                && Some(effect.damage_index) == self.current_damage_index(seat)
        }) {
            match effect.operation {
                DamageOperation::Add(_) => add = true,
                DamageOperation::Double => double = true,
            }
        }
        add && double
    }

    fn apply(&mut self, index: usize, outcomes: &mut OutcomeLog<'_>) -> Result<(), GameError> {
        let replacement = self.remaining.get(index).ok_or(GameError::IllegalAction)?;
        if Some(replacement.choosing_seat()) != self.choosing_seat()
            || Some(replacement.damage_index)
                != self.current_damage_index(replacement.choosing_seat())
        {
            return Err(GameError::IllegalAction);
        }
        let packet = &mut self.damage[replacement.damage_index];
        let before = packet.amount;
        packet.amount = match replacement.operation {
            DamageOperation::Add(bonus) => before.checked_add(bonus),
            DamageOperation::Double => before.checked_mul(2),
        }
        .ok_or(GameError::IllegalAction)?;
        outcomes.push("damage-modifier-applied", || {
            json!({
                "before": before, "amount": packet.amount, "modifier": replacement.value(),
                "strikerInstanceId": packet.striker.instance_id(),
                "targetInstanceId": packet.target.instance_id(),
            })
        });
        self.remaining.remove(index);
        Ok(())
    }

    pub(super) fn value(&self, game: &Game) -> Value {
        let current = self
            .choosing_seat()
            .and_then(|seat| self.current_damage_index(seat))
            .unwrap_or(0);
        let continuation = match &self.continuation {
            DamageContinuation::Ranged(continuation) => {
                let strike = &continuation.strike;
                json!({
                    "kind": "ranged-strike", "shooterInstanceId": continuation.shooter,
                    "seat": continuation.seat, "target": continuation.target,
                    "returnPhase": continuation.return_phase.as_str(), "stepAfter": continuation.step_after,
                    "strike": {"amount": strike.amount, "additiveBonus": strike.additive_bonus,
                        "currentPower": strike.current_power, "lethal": strike.lethal,
                        "lanceCount": strike.lance_count,
                        "consumedArtifacts": strike.consumed_artifacts.iter().map(RealmReference::value).collect::<Vec<_>>()},
                })
            }
            DamageContinuation::Fight(fight) => json!({
                "kind": "fight-window", "pending": Game::pending_combat_value(&fight.pending),
                "attackerStrikes": fight.attacker_strikes,
                "strikingCombatantIds": fight.striking_combatant_ids,
                "continuation": fight.continuation.as_ref().map(|next| game.resolution_continuation_value(next)),
            }),
        };
        json!({
            "activeSeat": self.active_seat, "amount": self.damage[current].amount,
            "currentDamageIndex": current,
            "damage": self.damage.iter().map(|packet| json!({"amount": packet.amount, "striker": packet.striker, "target": packet.target})).collect::<Vec<_>>(),
            "remaining": self.remaining.iter().map(DamageReplacement::value).collect::<Vec<_>>(),
            "continuation": continuation,
        })
    }
}

impl Game {
    pub(super) fn begin_ranged_damage_order(
        &mut self,
        continuation: RangedDamage,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let striker = UnitTarget::Minion {
            instance_id: continuation.shooter.clone(),
            seat: continuation.seat,
        };
        let mut pending = PendingDamageOrder {
            damage: Vec::new(),
            active_seat: self.position.active_seat,
            remaining: Vec::new(),
            continuation: DamageContinuation::Ranged(continuation.clone()),
        };
        self.push_ordered_damage(
            &mut pending,
            striker,
            continuation.target,
            &continuation.strike,
        )?;
        self.position.pending_damage_order = Some(Box::new(pending));
        self.advance_damage_order(false, outcomes)
    }

    fn push_ordered_damage(
        &self,
        pending: &mut PendingDamageOrder,
        striker: UnitTarget,
        target: UnitTarget,
        strike: &StrikeStats,
    ) -> Result<(), GameError> {
        let damage_index = pending.damage.len();
        let kind = match striker {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        for _ in 0..strike.lance_count {
            let owner = self
                .position
                .units
                .iter()
                .find(|unit| &unit.card.instance_id == striker.instance_id())
                .map_or(striker.seat(), |unit| unit.card.owner);
            pending.remaining.push(DamageReplacement {
                damage_index,
                controller: Some(striker.seat()),
                owner,
                operation: DamageOperation::Add(1),
                source: self.unit_reference(&striker)?,
            });
        }
        for artifact in &self.position.artifacts {
            if artifact.carried_by(kind, striker.seat(), striker.instance_id())
                && let Some(modifier) = self.artifact_facts(artifact)?.bearer_unit_strike
                && modifier.damage_bonus > 0
            {
                pending.remaining.push(DamageReplacement {
                    damage_index,
                    controller: Some(striker.seat()),
                    owner: artifact.card.owner,
                    operation: DamageOperation::Add(u16::from(modifier.damage_bonus)),
                    source: RealmReference::from_card(&artifact.card),
                });
            }
        }
        let target_kind = match target {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        for artifact in
            self.nearby_unit_strike_sources(target_kind, target.seat(), target.instance_id())?
        {
            pending.remaining.push(DamageReplacement {
                damage_index,
                controller: artifact.bearer().map(UnitTarget::seat),
                owner: artifact.card.owner,
                operation: DamageOperation::Double,
                source: RealmReference::from_card(&artifact.card),
            });
        }
        pending.damage.push(DamagePacket {
            amount: strike
                .amount
                .checked_sub(strike.additive_bonus)
                .ok_or(GameError::IllegalAction)?,
            striker,
            target,
        });
        Ok(())
    }

    pub(super) fn begin_fight_damage_order(
        &mut self,
        fight: FightDamage,
        attacker: &StrikeStats,
        attacker_owes_allocation: bool,
        return_sources: &[(UnitKind, UnitTarget, StrikeStats)],
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let attacker_target = match fight.pending.attacker_kind {
            UnitKind::Avatar => UnitTarget::Avatar {
                instance_id: fight.pending.attacker_instance_id.clone(),
                seat: fight.pending.attacking_seat,
            },
            UnitKind::Minion => UnitTarget::Minion {
                instance_id: fight.pending.attacker_instance_id.clone(),
                seat: fight.pending.attacking_seat,
            },
        };
        let mut pending = PendingDamageOrder {
            damage: Vec::new(),
            active_seat: self.position.active_seat,
            remaining: Vec::new(),
            continuation: DamageContinuation::Fight(Box::new(fight)),
        };
        for (_, striker, strike) in return_sources {
            self.push_ordered_damage(
                &mut pending,
                striker.clone(),
                attacker_target.clone(),
                strike,
            )?;
        }
        // The allocations of a split strike already include its shared bonus pool. Keep
        // mixed split modifiers guarded until their allocation timing is authoritative.
        let DamageContinuation::Fight(fight) = &pending.continuation else {
            unreachable!()
        };
        let combat = fight.pending.clone();
        if attacker_owes_allocation {
            for target in &combat.combatants {
                let amount = combat
                    .allocations
                    .iter()
                    .find(|allocation| &allocation.target_instance_id == target.instance_id())
                    .ok_or(GameError::IllegalAction)?
                    .amount;
                if combat.combatants.len() == 1 {
                    let mut strike = attacker.clone();
                    strike.amount = amount;
                    self.push_ordered_damage(
                        &mut pending,
                        attacker_target.clone(),
                        target.clone(),
                        &strike,
                    )?;
                } else {
                    let kind = match target {
                        UnitTarget::Avatar { .. } => UnitKind::Avatar,
                        UnitTarget::Minion { .. } => UnitKind::Minion,
                    };
                    let amount = self.nearby_unit_strike_amount(
                        amount,
                        attacker.additive_bonus,
                        kind,
                        target.seat(),
                        target.instance_id(),
                    )?;
                    pending.damage.push(DamagePacket {
                        amount,
                        striker: attacker_target.clone(),
                        target: target.clone(),
                    });
                }
            }
        }
        self.position.pending_damage_order = Some(Box::new(pending));
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
        match pending.continuation {
            DamageContinuation::Ranged(continuation) => {
                self.position.phase = continuation.return_phase;
                self.position.decision_seat = continuation.seat;
                self.finish_ranged_damage(
                    &continuation.strike,
                    &continuation.target,
                    (continuation.seat, &continuation.shooter),
                    continuation.return_phase,
                    pending.damage[0].amount,
                    outcomes,
                )?;
                if resumed && continuation.step_after {
                    self.queue_ranged_step(continuation.seat, &continuation.shooter)?;
                }
            }
            DamageContinuation::Fight(fight) => {
                let amounts = pending
                    .damage
                    .iter()
                    .map(|packet| packet.amount)
                    .collect::<Vec<_>>();
                let interrupted = self.resolve_fight_window_with_damage(
                    &fight.pending,
                    fight.attacker_strikes,
                    &fight.striking_combatant_ids,
                    fight.continuation.clone(),
                    Some(&amounts),
                    outcomes,
                )?;
                if !interrupted && let Some(next) = fight.continuation {
                    self.continue_resolution(next, outcomes)?;
                }
            }
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
            if replacement.choosing_seat() == seat
                && Some(replacement.damage_index) == pending.current_damage_index(seat)
            {
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
