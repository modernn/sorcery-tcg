//! Shared execution of admitted composed abilities, suspended by the existing death driver.

use super::ability::{CompiledAbility, Effect, UnitSet};
use super::{
    CardFacts, CardId, CardInstance, Cell, DeathriteContinuation, Game, GameError, IdentityHash,
    Location, OutcomeLog, Region, Seat, UnitDamageSource, UnitKind, UnitQuery, UnitTarget, Value,
    json, seat_index,
};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AbilityEntry {
    Magic,
    Genesis,
    Activated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RealmReference {
    instance_id: IdentityHash,
    realm_entry: u64,
}

impl RealmReference {
    pub(super) fn from_card(card: &CardInstance) -> Self {
        Self {
            instance_id: card.instance_id.clone(),
            realm_entry: card.realm_entry,
        }
    }

    fn matches(&self, card: &CardInstance) -> bool {
        self.instance_id == card.instance_id && self.realm_entry == card.realm_entry
    }

    fn value(&self) -> Value {
        json!({ "instanceId": self.instance_id, "realmEntry": self.realm_entry })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EffectSource {
    pub(super) instance_id: IdentityHash,
    pub(super) owner: Seat,
    pub(super) controller: Seat,
    pub(super) realm: Option<RealmReference>,
    pub(super) actor: Option<RealmReference>,
    pub(super) region: Region,
    pub(super) cells: Vec<Cell>,
    pub(super) damage: UnitDamageSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BindingState {
    Active,
    Invalid,
    Protected,
}

impl BindingState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Invalid => "invalid",
            Self::Protected => "protected",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UnitBinding {
    reference: RealmReference,
    state: BindingState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocationBinding {
    location: Location,
    site: Option<RealmReference>,
    state: BindingState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EffectFrame {
    card_id: CardId,
    entry: AbilityEntry,
    cursor: usize,
    started: bool,
    source: EffectSource,
    target: Option<UnitBinding>,
    location: Option<LocationBinding>,
    magic: Option<CardInstance>,
}

impl CardInstance {
    pub(super) fn enter_realm(&mut self) -> Result<(), GameError> {
        // A physical card may return, but references to its previous realm object expire.
        self.realm_entry = self
            .realm_entry
            .checked_add(1)
            .ok_or(GameError::IllegalAction)?;
        Ok(())
    }
}

impl Game {
    fn compiled_ability(&self, card_id: CardId, entry: AbilityEntry) -> Option<&CompiledAbility> {
        let abilities = &self.rules.cards[usize::from(card_id.0)].abilities;
        match entry {
            AbilityEntry::Magic => &abilities.magic,
            AbilityEntry::Genesis => &abilities.genesis,
            AbilityEntry::Activated => &abilities.activated,
        }
        .as_ref()
    }

    fn referenced_unit(&self, reference: &RealmReference) -> Option<(UnitKind, Seat)> {
        for seat in [Seat::North, Seat::South] {
            if reference.matches(&self.position.players[seat_index(seat)].avatar.card) {
                return Some((UnitKind::Avatar, seat));
            }
        }
        self.position
            .units
            .iter()
            .find(|unit| reference.matches(&unit.card))
            .map(|unit| (UnitKind::Minion, unit.controller))
    }

    fn realm_reference_exists(&self, reference: &RealmReference) -> bool {
        self.referenced_unit(reference).is_some()
            || self
                .position
                .artifacts
                .iter()
                .any(|artifact| reference.matches(&artifact.card))
            || self
                .position
                .sites
                .iter()
                .flatten()
                .any(|site| reference.matches(&site.card))
    }

    pub(super) fn unit_reference(&self, target: &UnitTarget) -> Result<RealmReference, GameError> {
        let card = match target {
            UnitTarget::Avatar { seat, instance_id } => {
                let card = &self.position.players[seat_index(*seat)].avatar.card;
                if card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                card
            }
            UnitTarget::Minion { instance_id, seat } => {
                &self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == *instance_id && unit.controller == *seat)
                    .ok_or(GameError::IllegalAction)?
                    .card
            }
        };
        Ok(RealmReference::from_card(card))
    }

    pub(super) fn unit_effect_source(
        &self,
        target: &UnitTarget,
    ) -> Result<EffectSource, GameError> {
        let reference = self.unit_reference(target)?;
        let (kind, controller) = self
            .referenced_unit(&reference)
            .ok_or(GameError::IllegalAction)?;
        let (card, region, cells) = match kind {
            UnitKind::Avatar => {
                let avatar = &self.position.players[seat_index(controller)].avatar;
                (&avatar.card, Region::Surface, vec![avatar.location])
            }
            UnitKind::Minion => {
                let unit = self
                    .position
                    .units
                    .iter()
                    .find(|unit| reference.matches(&unit.card))
                    .ok_or(GameError::IllegalAction)?;
                (
                    &unit.card,
                    unit.region,
                    Self::unit_occupied_cells(unit).to_vec(),
                )
            }
        };
        let (_, damage) = self.combatant_damage_stats(kind, controller, &reference.instance_id)?;
        Ok(EffectSource {
            instance_id: card.instance_id.clone(),
            owner: card.owner,
            controller,
            realm: Some(reference.clone()),
            actor: Some(reference),
            region,
            cells,
            damage,
        })
    }

    pub(super) fn effect_frame(
        &self,
        card_id: CardId,
        entry: AbilityEntry,
        source: EffectSource,
        target: Option<&UnitTarget>,
        location: Option<Location>,
        magic: Option<CardInstance>,
    ) -> Result<EffectFrame, GameError> {
        if self.compiled_ability(card_id, entry).is_none() {
            return Err(GameError::IllegalAction);
        }
        Ok(EffectFrame {
            card_id,
            entry,
            cursor: 0,
            started: false,
            source,
            target: target
                .map(|target| {
                    self.unit_reference(target).map(|reference| UnitBinding {
                        reference,
                        state: BindingState::Active,
                    })
                })
                .transpose()?,
            location: location.map(|location| LocationBinding {
                location,
                site: self.position.sites[location.cell.index()]
                    .as_ref()
                    .map(|site| RealmReference::from_card(&site.card)),
                state: BindingState::Active,
            }),
            magic,
        })
    }

    /// Target protection is decided once for the whole ability, separately from damage prevention.
    fn start_effect_frame(&mut self, frame: &mut EffectFrame, outcomes: &mut OutcomeLog<'_>) {
        frame.started = true;
        if let Some(binding) = &mut frame.target {
            match self.referenced_unit(&binding.reference) {
                None => binding.state = BindingState::Invalid,
                Some((UnitKind::Minion, seat)) if seat != frame.source.controller => {
                    let unit = self
                        .position
                        .units
                        .iter_mut()
                        .find(|unit| binding.reference.matches(&unit.card))
                        .expect("resolved target");
                    if unit.warded {
                        unit.warded = false;
                        binding.state = BindingState::Protected;
                        outcomes.push(
                            "ward-broken",
                            || json!({ "instanceId": unit.card.instance_id, "seat": seat }),
                        );
                    }
                }
                Some(_) => {}
            }
        }
        if let Some(binding) = &mut frame.location {
            let site = self.position.sites[binding.location.cell.index()].as_mut();
            match (&binding.site, site) {
                (Some(reference), Some(site)) if reference.matches(&site.card) => {
                    if site.controller != frame.source.controller && site.warded {
                        site.warded = false;
                        binding.state = BindingState::Protected;
                        outcomes.push("ward-broken", || json!({ "cell": binding.location.cell, "instanceId": site.card.instance_id, "seat": site.controller }));
                    }
                }
                (None, None) => {}
                _ => binding.state = BindingState::Invalid,
            }
        }
    }

    fn effect_recipients(
        &self,
        frame: &EffectFrame,
        set: UnitSet,
    ) -> Result<Vec<(IdentityHash, UnitKind, Seat)>, GameError> {
        match set {
            UnitSet::Target => {
                let binding = frame.target.as_ref().ok_or(GameError::IllegalAction)?;
                Ok(if binding.state == BindingState::Active {
                    self.referenced_unit(&binding.reference)
                        .map(|(kind, seat)| (binding.reference.instance_id.clone(), kind, seat))
                        .into_iter()
                        .collect()
                } else {
                    Vec::new()
                })
            }
            UnitSet::Location => {
                let binding = frame.location.as_ref().ok_or(GameError::IllegalAction)?;
                Ok(if binding.state == BindingState::Active {
                    self.units_at_location(binding.location)
                } else {
                    Vec::new()
                })
            }
            UnitSet::OtherUnitsHere => Ok(self.query_units(UnitQuery {
                region: frame.source.region,
                cells: Some(&frame.source.cells),
                kind: None,
                controller: None,
                exclude: frame
                    .source
                    .realm
                    .as_ref()
                    .filter(|reference| self.realm_reference_exists(reference))
                    .map(|reference| &reference.instance_id),
            })),
            UnitSet::SurfaceMinions => Ok(self.query_units(UnitQuery {
                region: Region::Surface,
                cells: None,
                kind: Some(UnitKind::Minion),
                controller: None,
                exclude: None,
            })),
        }
    }

    pub(super) fn run_effect_frame(
        &mut self,
        mut frame: EffectFrame,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if let Some(pending) = &mut self.position.pending_deathrites {
            if pending.continuation.is_some() {
                return Err(GameError::UnsupportedMechanic(
                    "overlapping ability continuations before effect start".to_owned(),
                ));
            }
            pending.continuation = Some(DeathriteContinuation::Effect(Box::new(frame)));
            return Ok(());
        }
        if !frame.started {
            if frame
                .source
                .realm
                .as_ref()
                .is_some_and(|reference| !self.realm_reference_exists(reference))
            {
                self.finish_effect_frame(frame, outcomes);
                return Ok(());
            }
            self.start_effect_frame(&mut frame, outcomes);
        }
        while self.position.terminal.is_none() {
            let effect = self
                .compiled_ability(frame.card_id, frame.entry)
                .ok_or(GameError::IllegalAction)?
                .effects
                .get(frame.cursor)
                .copied();
            let Some(effect) = effect else {
                break;
            };
            frame.cursor += 1;
            match effect {
                Effect::Damage { recipients, amount } => {
                    if let Some(reference) = &frame.source.realm
                        && let Some((kind, seat)) = self.referenced_unit(reference)
                    {
                        frame.source.damage = self
                            .combatant_damage_stats(kind, seat, &reference.instance_id)?
                            .1;
                    }
                    let targets = self.effect_recipients(&frame, recipients)?;
                    let allocated = match frame.entry {
                        AbilityEntry::Magic => "magic-damage-allocated",
                        AbilityEntry::Genesis => "genesis-damage-allocated",
                        AbilityEntry::Activated
                            if matches!(
                                self.rules.cards[usize::from(frame.card_id.0)].facts,
                                CardFacts::Artifact(_)
                            ) =>
                        {
                            "artifact-damage-allocated"
                        }
                        AbilityEntry::Activated => "area-damage-allocated",
                    };
                    let casualties = self.apply_area_damage(
                        targets,
                        amount,
                        frame.source.damage,
                        (allocated, &frame.source.instance_id),
                        outcomes,
                    )?;
                    if !casualties.minions.is_empty() || !casualties.avatars.is_empty() {
                        return self.begin_minion_deaths_with_continuation(
                            &casualties.minions,
                            &casualties.avatars,
                            self.position.phase,
                            self.position.active_seat,
                            Some(DeathriteContinuation::Effect(Box::new(frame))),
                            outcomes,
                        );
                    }
                }
                Effect::Untap { recipients } => {
                    for (id, kind, seat) in self.effect_recipients(&frame, recipients)? {
                        match kind {
                            UnitKind::Avatar => {
                                self.apply_untap_avatar(seat, &frame.source.instance_id, outcomes);
                            }
                            UnitKind::Minion => self.apply_untap_minion(
                                &id,
                                seat,
                                &frame.source.instance_id,
                                outcomes,
                            )?,
                        }
                    }
                }
                Effect::Draw { zone, count } => self.apply_genesis_draws(
                    frame.source.controller,
                    &frame.source.instance_id,
                    zone,
                    count,
                    outcomes,
                ),
            }
        }
        self.finish_effect_frame(frame, outcomes);
        Ok(())
    }

    pub(super) fn finish_effect_frame(
        &mut self,
        frame: EffectFrame,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        if let Some(card) = frame.magic {
            let owner = card.owner;
            let instance_id = card.instance_id.clone();
            let card_id = self.rules.cards[usize::from(card.card_id.0)].id.clone();
            self.position.players[seat_index(owner)].cemetery.push(card);
            Self::emit_continuation_magic_resolved(&card_id, &instance_id, owner, outcomes);
        }
    }

    pub(super) fn effect_frame_value(&self, frame: &EffectFrame) -> Value {
        json!({
            "kind": "effect", "cardId": self.rules.cards[usize::from(frame.card_id.0)].id,
            "entry": match frame.entry { AbilityEntry::Magic => "magic", AbilityEntry::Genesis => "genesis", AbilityEntry::Activated => "activated" },
            "cursor": frame.cursor, "started": frame.started,
            "source": { "instanceId": frame.source.instance_id, "owner": frame.source.owner, "controller": frame.source.controller,
                "realm": frame.source.realm.as_ref().map(RealmReference::value), "actor": frame.source.actor.as_ref().map(RealmReference::value),
                "region": frame.source.region, "cells": frame.source.cells, "power": frame.source.damage.current_power, "lethal": frame.source.damage.lethal },
            "target": frame.target.as_ref().map(|binding| json!({ "object": binding.reference.value(), "state": binding.state.as_str() })),
            "location": frame.location.as_ref().map(|binding| json!({ "location": binding.location, "site": binding.site.as_ref().map(RealmReference::value), "state": binding.state.as_str() })),
            "magic": frame.magic.as_ref().map(|card| self.card_value(card)),
        })
    }
}
