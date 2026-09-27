//! Full-power strike groups caused by effects, sharing replacement and death handling.
use super::{
    Game, GameError, OutcomeLog, Phase, RealmReference, Seat, StrikeStats, UnitDamageSource,
    UnitKind, UnitTarget, Value, json,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EffectStrikes {
    pub(super) source: UnitTarget,
    pub(super) targets: Vec<UnitTarget>,
    pub(super) strike: StrikeStats,
    pub(super) reveal_source: bool,
    pub(super) return_phase: Phase,
    pub(super) return_seat: Seat,
}

impl EffectStrikes {
    pub(super) fn value(&self) -> Value {
        json!({"kind": "effect-strikes", "source": self.source, "targets": self.targets,
            "revealSource": self.reveal_source, "returnPhase": self.return_phase.as_str(), "returnSeat": self.return_seat,
            "strike": {"amount": self.strike.amount, "additiveBonus": self.strike.additive_bonus,
                "currentPower": self.strike.current_power, "lethal": self.strike.lethal, "lanceCount": self.strike.lance_count,
                "consumedArtifacts": self.strike.consumed_artifacts.iter().map(RealmReference::value).collect::<Vec<_>>()}})
    }
}

impl Game {
    pub(super) fn resolve_effect_strikes(
        &mut self,
        group: &EffectStrikes,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if group.targets.is_empty() {
            return Ok(());
        }
        if group.strike.additive_bonus > 0
            && group.targets.iter().try_fold(false, |mixed, target| {
                let kind = match target {
                    UnitTarget::Avatar { .. } => UnitKind::Avatar,
                    UnitTarget::Minion { .. } => UnitKind::Minion,
                };
                self.nearby_unit_strike_multiplier(kind, target.seat(), target.instance_id())
                    .map(|multiplier| mixed || multiplier > 1)
            })?
        {
            return self.begin_effect_damage_order(group, outcomes);
        }
        self.finish_effect_strikes(group, None, outcomes)
    }

    pub(super) fn finish_effect_strikes(
        &mut self,
        group: &EffectStrikes,
        amounts: Option<&[u16]>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if amounts.is_some_and(|values| values.len() != group.targets.len()) {
            return Err(GameError::IllegalAction);
        }
        self.position.phase = group.return_phase;
        self.position.decision_seat = group.return_seat;
        // Snapshot prevention and modified damage together before any target is hit.
        let targets = group
            .targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let (kind, status) = match target {
                    UnitTarget::Avatar { .. } => (UnitKind::Avatar, None),
                    UnitTarget::Minion { instance_id, .. } => (
                        UnitKind::Minion,
                        Some(self.minion_damage_status(instance_id)?),
                    ),
                };
                let amount = if let Some(values) = amounts {
                    values[index]
                } else {
                    self.nearby_unit_strike_amount(
                        group.strike.amount,
                        group.strike.additive_bonus,
                        kind,
                        target.seat(),
                        target.instance_id(),
                    )?
                };
                Ok((status, amount))
            })
            .collect::<Result<Vec<_>, GameError>>()?;
        let kind = match group.source {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        let lost_stealth = group.reveal_source
            && self.mark_unit_interaction(kind, group.source.seat(), group.source.instance_id())?;
        for (target, (_, amount)) in group.targets.iter().zip(&targets) {
            outcomes.push("strike-damage-allocated", || json!({"amount": amount,
                "strikerInstanceId": group.source.instance_id(), "targetInstanceId": target.instance_id()}));
        }
        let mut dead_minions = Vec::new();
        let mut defeated_avatars = Vec::new();
        for (target, (status, amount)) in group.targets.iter().zip(targets) {
            let target_kind = match target {
                UnitTarget::Avatar { .. } => UnitKind::Avatar,
                UnitTarget::Minion { .. } => UnitKind::Minion,
            };
            let result = self.apply_simple_damage_with_status(
                target_kind,
                target.seat(),
                target.instance_id(),
                amount,
                UnitDamageSource {
                    origin: crate::game::DamageOrigin::Other,
                    current_power: group.strike.current_power,
                    lethal: group.strike.lethal,
                },
                status,
                outcomes,
            )?;
            if result.minion_died {
                dead_minions.push(target.instance_id().clone());
            }
            if result.avatar_defeated && !defeated_avatars.contains(&target.seat()) {
                defeated_avatars.push(target.seat());
            }
        }
        if lost_stealth {
            outcomes.push(
                "stealth-lost",
                || json!({"instanceId": group.source.instance_id(), "seat": group.source.seat()}),
            );
            self.revert_stealth_bound_controls(outcomes)?;
        }
        self.consume_strike_artifacts(&group.strike, group.source.instance_id(), outcomes)?;
        if group.strike.lance_count > 0 {
            self.break_lance(group.source.seat(), group.source.instance_id(), outcomes)?;
        }
        self.consume_next_strike_double(
            kind,
            group.source.seat(),
            group.source.instance_id(),
            outcomes,
        )?;
        if !group.strike.consumed_artifacts.is_empty() {
            dead_minions.extend(self.static_power_death_ids()?);
        }
        if !dead_minions.is_empty() || !defeated_avatars.is_empty() {
            self.begin_minion_deaths(
                &dead_minions,
                &defeated_avatars,
                group.return_phase,
                group.return_seat,
                outcomes,
            )?;
        }
        Ok(())
    }
}
