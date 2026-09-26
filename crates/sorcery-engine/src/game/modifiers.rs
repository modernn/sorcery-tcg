//! Sparse temporary modifiers attached to one game object.

use serde::{Deserialize, Serialize};

use super::{Game, GameError, OutcomeLog, Seat, UnitKind, json};
use crate::canonical::IdentityHash;

pub(super) use crate::ability::TemporaryModifierKind;

impl TemporaryModifierKind {
    const fn grant_event(self) -> &'static str {
        match self {
            Self::Airborne => "airborne-granted",
            Self::Charge => "charge-granted",
            Self::FirstStrike => "first-strike-granted",
            Self::Lethal => "lethal-granted",
            Self::NextStrikeDouble => "next-strike-double-granted",
            Self::Movement => "movement-granted",
            Self::Power => "power-granted",
            Self::Ranged => "ranged-granted",
            Self::Silence => "silence-granted",
        }
    }

    pub(super) const fn expiration_event(self) -> &'static str {
        match self {
            Self::Airborne => "airborne-expired",
            Self::Charge => "charge-expired",
            Self::FirstStrike => "first-strike-expired",
            Self::Lethal => "lethal-expired",
            Self::NextStrikeDouble => "next-strike-double-expired",
            Self::Movement => "movement-expired",
            Self::Power => "power-expired",
            Self::Ranged => "ranged-expired",
            Self::Silence => "silence-expired",
        }
    }
}

impl Game {
    pub(super) fn take_expired_modifiers(
        &mut self,
        expires_at_seat: Option<Seat>,
    ) -> Vec<(IdentityHash, Seat, TemporaryModifier)> {
        let mut expired = Vec::new();
        for (index, player) in self.position.players.iter_mut().enumerate() {
            let seat = if index == 0 { Seat::North } else { Seat::South };
            for modifier in player.avatar.temporary_modifiers.expire(expires_at_seat) {
                expired.push((player.avatar.card.instance_id.clone(), seat, modifier));
            }
        }
        for unit in &mut self.position.units {
            for modifier in unit.temporary_modifiers.expire(expires_at_seat) {
                expired.push((unit.card.instance_id.clone(), unit.controller, modifier));
            }
        }
        expired
    }

    pub(super) fn emit_expired_modifiers(
        expired: Vec<(IdentityHash, Seat, TemporaryModifier)>,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        for (instance_id, controller, modifier) in expired {
            outcomes.push(modifier.kind.expiration_event(), || {
                let mut value = json!({
                    "instanceId": instance_id,
                    "seat": controller,
                    "sourceInstanceId": modifier.source_instance_id,
                });
                if matches!(
                    modifier.kind,
                    TemporaryModifierKind::Movement | TemporaryModifierKind::Power
                ) {
                    value["amount"] = json!(modifier.amount);
                }
                if let Some(seat) = modifier.expires_at_seat {
                    value["expiresAtSeat"] = json!(seat);
                }
                value
            });
        }
    }

    pub(super) fn grant_unit_modifier(
        &mut self,
        (instance_id, kind, seat): (IdentityHash, UnitKind, Seat),
        modifier: TemporaryModifierKind,
        amount: u16,
        source: &IdentityHash,
        expires_at_seat: Option<Seat>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        self.temporary_modifiers_mut(kind, seat, &instance_id)?
            .grant_until(modifier, amount, source.clone(), expires_at_seat);
        outcomes.push(modifier.grant_event(), || {
            let mut value =
                json!({"instanceId": instance_id, "seat": seat, "sourceInstanceId": source});
            if matches!(
                modifier,
                TemporaryModifierKind::Movement | TemporaryModifierKind::Power
            ) {
                value["amount"] = json!(amount);
            }
            if let Some(expiry_seat) = expires_at_seat {
                value["expiresAtSeat"] = json!(expiry_seat);
            }
            value
        });
        Ok(())
    }
}

/// One independently granted temporary effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TemporaryModifier {
    pub(super) kind: TemporaryModifierKind,
    pub(super) amount: u16,
    pub(super) source_instance_id: IdentityHash,
    /// Absent means this End Phase; otherwise expires before that seat next untaps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) expires_at_seat: Option<Seat>,
}

/// Sparse, duplicate-preserving temporary effects for one object.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(super) struct TemporaryModifiers {
    entries: Vec<TemporaryModifier>,
}

impl TemporaryModifiers {
    #[must_use]
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn grant(
        &mut self,
        kind: TemporaryModifierKind,
        amount: u16,
        source_instance_id: IdentityHash,
    ) {
        self.grant_until(kind, amount, source_instance_id, None);
    }

    pub(super) fn grant_until(
        &mut self,
        kind: TemporaryModifierKind,
        amount: u16,
        source_instance_id: IdentityHash,
        expires_at_seat: Option<Seat>,
    ) {
        self.entries.push(TemporaryModifier {
            kind,
            amount,
            source_instance_id,
            expires_at_seat,
        });
    }

    #[must_use]
    pub(super) fn has(&self, kind: TemporaryModifierKind) -> bool {
        self.entries.iter().any(|modifier| modifier.kind == kind)
    }

    /// Returns the checked u16 magnitude sum for one modifier kind.
    pub(super) fn amount(
        &self,
        kind: TemporaryModifierKind,
    ) -> Result<u16, std::num::TryFromIntError> {
        let total: u64 = self
            .entries
            .iter()
            .filter(|modifier| modifier.kind == kind)
            .map(|modifier| u64::from(modifier.amount))
            .sum();
        u16::try_from(total)
    }

    pub(super) fn sources(
        &self,
        kind: TemporaryModifierKind,
    ) -> impl Iterator<Item = &IdentityHash> {
        self.entries
            .iter()
            .filter(move |modifier| modifier.kind == kind)
            .map(|modifier| &modifier.source_instance_id)
    }

    /// Removes every grant of one kind, retaining insertion order in both results.
    pub(super) fn take(&mut self, kind: TemporaryModifierKind) -> Vec<TemporaryModifier> {
        self.entries
            .extract_if(.., |modifier| modifier.kind == kind)
            .collect()
    }

    /// Removes only grants ending at this phase boundary, preserving record order.
    pub(super) fn expire(
        &mut self,
        expires_at_seat: Option<Seat>,
    ) -> impl Iterator<Item = TemporaryModifier> + '_ {
        self.entries.extract_if(.., move |modifier| {
            modifier.expires_at_seat == expires_at_seat
        })
    }

    #[cfg(test)]
    pub(super) fn iter(&self) -> impl Iterator<Item = &TemporaryModifier> {
        self.entries.iter()
    }

    #[must_use]
    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod collection_tests {
    use super::{Seat, TemporaryModifierKind, TemporaryModifiers};
    use crate::canonical::IdentityHash;

    fn source(hex: char) -> IdentityHash {
        IdentityHash::parse(&format!("sha256:{}", hex.to_string().repeat(64))).expect("identity")
    }

    #[test]
    fn duplicate_grants_remain_independent() {
        let mut modifiers = TemporaryModifiers::new();
        modifiers.grant(TemporaryModifierKind::Power, 1, source('a'));
        modifiers.grant(TemporaryModifierKind::Power, 1, source('a'));

        assert_eq!(modifiers.iter().count(), 2);
        assert_eq!(modifiers.sources(TemporaryModifierKind::Power).count(), 2);
        assert_eq!(modifiers.amount(TemporaryModifierKind::Power), Ok(2));
    }

    #[test]
    fn taking_one_kind_does_not_remove_other_kinds() {
        let mut modifiers = TemporaryModifiers::new();
        let airborne_source = source('a');
        let charge_source = source('b');
        let second_airborne_source = source('c');
        modifiers.grant(TemporaryModifierKind::Airborne, 1, airborne_source.clone());
        modifiers.grant(TemporaryModifierKind::Charge, 1, charge_source);
        modifiers.grant(
            TemporaryModifierKind::Airborne,
            1,
            second_airborne_source.clone(),
        );

        let removed = modifiers.take(TemporaryModifierKind::Airborne);
        assert_eq!(
            removed
                .iter()
                .map(|modifier| modifier.source_instance_id.as_str())
                .collect::<Vec<_>>(),
            vec![airborne_source.as_str(), second_airborne_source.as_str()]
        );
        assert!(!modifiers.has(TemporaryModifierKind::Airborne));
        assert!(modifiers.has(TemporaryModifierKind::Charge));
        assert_eq!(modifiers.take(TemporaryModifierKind::Charge).len(), 1);
        assert!(modifiers.is_empty());
    }

    #[test]
    fn magnitude_sums_are_checked() {
        let mut modifiers = TemporaryModifiers::new();
        modifiers.grant(TemporaryModifierKind::Movement, 2, source('a'));
        modifiers.grant(TemporaryModifierKind::Movement, 3, source('b'));
        assert_eq!(modifiers.amount(TemporaryModifierKind::Movement), Ok(5));

        modifiers.grant(TemporaryModifierKind::Movement, u16::MAX, source('c'));
        assert!(modifiers.amount(TemporaryModifierKind::Movement).is_err());
    }

    #[test]
    fn expiry_preserves_other_boundaries_and_serialized_order() {
        let mut modifiers = TemporaryModifiers::new();
        for (amount, boundary) in [
            (1, Some(Seat::North)),
            (2, None),
            (4, Some(Seat::South)),
            (8, Some(Seat::North)),
        ] {
            modifiers.grant_until(TemporaryModifierKind::Power, amount, source('a'), boundary);
        }
        let encoded = serde_json::to_value(&modifiers).unwrap();
        assert!(encoded[1].get("expiresAtSeat").is_none());
        let mut restored: TemporaryModifiers = serde_json::from_value(encoded).unwrap();
        assert_eq!(restored, modifiers);
        let amounts = |entries: Vec<_>| {
            entries
                .into_iter()
                .map(|m: super::TemporaryModifier| m.amount)
                .collect::<Vec<_>>()
        };
        assert_eq!(amounts(restored.expire(None).collect()), [2]);
        assert_eq!(restored.amount(TemporaryModifierKind::Power), Ok(13));
        assert_eq!(
            amounts(restored.expire(Some(Seat::North)).collect()),
            [1, 8]
        );
        assert_eq!(restored.amount(TemporaryModifierKind::Power), Ok(4));
        assert_eq!(amounts(restored.expire(Some(Seat::South)).collect()), [4]);
        assert!(restored.is_empty());
        assert_eq!(modifiers.amount(TemporaryModifierKind::Power), Ok(15));
    }
}

#[cfg(test)]
mod tests;
