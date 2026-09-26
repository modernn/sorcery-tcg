//! Shared execution of admitted composed abilities, suspended by the existing death driver.

use super::ability::{CompiledAbility, Effect, SelectionSpec, UnitSet};
use super::{
    CardFacts, CardId, CardInstance, Cell, DeferredMagicResolved, Game, GameError, IdentityHash,
    Location, OutcomeLog, Region, ResolutionContinuation, Seat, UnitDamageSource, UnitKind,
    UnitQuery, UnitTarget, Value, json, seat_index,
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

    pub(super) fn matches(&self, card: &CardInstance) -> bool {
        self.instance_id == card.instance_id && self.realm_entry == card.realm_entry
    }

    pub(super) fn value(&self) -> Value {
        json!({ "instanceId": self.instance_id, "realmEntry": self.realm_entry })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EffectSource {
    pub(super) instance_id: IdentityHash,
    pub(super) owner: Seat,
    pub(super) controller: Seat,
    /// Realm object owning the ability and its damage characteristics; Magic has none.
    pub(super) realm: Option<RealmReference>,
    /// Casting or activating unit, retained separately from the effect's damage source.
    pub(super) actor: Option<RealmReference>,
    pub(super) region: Region,
    pub(super) cells: Vec<Cell>,
    pub(super) damage: UnitDamageSource,
}

impl EffectSource {
    pub(super) fn value(&self) -> Value {
        json!({ "instanceId": self.instance_id, "owner": self.owner, "controller": self.controller,
            "realm": self.realm.as_ref().map(RealmReference::value), "actor": self.actor.as_ref().map(RealmReference::value),
            "region": self.region, "cells": self.cells, "power": self.damage.current_power, "lethal": self.damage.lethal })
    }
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
    pub(super) card_id: CardId,
    entry: AbilityEntry,
    cursor: usize,
    started: bool,
    pub(super) source: EffectSource,
    pub(super) declaration_pending: bool,
    target: Option<UnitBinding>,
    chosen: Option<RealmReference>,
    location: Option<LocationBinding>,
    pub(super) magic: Option<CardInstance>,
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
    pub(super) fn compiled_ability(
        &self,
        card_id: CardId,
        entry: AbilityEntry,
    ) -> Option<&CompiledAbility> {
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

    pub(super) fn realm_reference_exists(&self, reference: &RealmReference) -> bool {
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

    fn referenced_geometry(
        &self,
        reference: &RealmReference,
    ) -> Result<(Region, Vec<Cell>), GameError> {
        for player in &self.position.players {
            if reference.matches(&player.avatar.card) {
                return Ok((Region::Surface, vec![player.avatar.location]));
            }
        }
        if let Some(unit) = self
            .position
            .units
            .iter()
            .find(|unit| reference.matches(&unit.card))
        {
            return Ok((unit.region, Self::unit_occupied_cells(unit).to_vec()));
        }
        if let Some(artifact) = self
            .position
            .artifacts
            .iter()
            .find(|artifact| reference.matches(&artifact.card))
        {
            let location = self.artifact_location(artifact)?;
            let cells = match artifact.bearer() {
                Some(bearer) => self.unit_target_occupied_cells(bearer)?.to_vec(),
                None => vec![location.cell],
            };
            return Ok((location.region, cells));
        }
        Err(GameError::IllegalAction)
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
            declaration_pending: self
                .compiled_ability(card_id, entry)
                .is_some_and(|ability| ability.selection.is_some())
                && target.is_none()
                && location.is_none(),
            source,
            chosen: None,
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
        let selection = self
            .compiled_ability(frame.card_id, frame.entry)
            .and_then(|ability| ability.selection);
        if let Some(binding) = &mut frame.target {
            let selected = match selection {
                Some(SelectionSpec::Unit { kind, relation }) => self
                    .selected_units(
                        UnitQuery {
                            region: frame.source.region,
                            cells: Some(&frame.source.cells),
                            kind,
                            controller: None,
                            exclude: None,
                        },
                        relation,
                        Some(frame.source.controller),
                    )
                    .iter()
                    .any(|target| target.instance_id() == &binding.reference.instance_id),
                _ => false,
            };
            if !selected {
                binding.state = BindingState::Invalid;
            }
            match self.referenced_unit(&binding.reference) {
                _ if binding.state == BindingState::Invalid => {}
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
            let selected = match selection {
                Some(SelectionSpec::Location { relation }) => self
                    .selected_locations(frame.source.region, &frame.source.cells, relation)
                    .contains(&binding.location),
                _ => false,
            };
            if !selected {
                binding.state = BindingState::Invalid;
            }
            let site = self.position.sites[binding.location.cell.index()].as_mut();
            match (&binding.site, site) {
                _ if binding.state == BindingState::Invalid => {}
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
            UnitSet::Chosen => Ok(frame
                .chosen
                .as_ref()
                .and_then(|reference| {
                    self.referenced_unit(reference)
                        .map(|(kind, seat)| (reference.instance_id.clone(), kind, seat))
                })
                .into_iter()
                .collect()),
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

    fn prepare_effect_source(&self, frame: &mut EffectFrame) -> Result<bool, GameError> {
        if frame
            .source
            .realm
            .as_ref()
            .is_some_and(|reference| !self.realm_reference_exists(reference))
            || (frame.entry == AbilityEntry::Magic
                && frame
                    .source
                    .actor
                    .as_ref()
                    .is_some_and(|reference| self.referenced_unit(reference).is_none()))
        {
            return Ok(false);
        }
        if frame.entry == AbilityEntry::Magic {
            // Caster identity is separate from Magic's damage source. Its chosen casting
            // location versus later movement needs a ruling before spatial revalidation.
            if self
                .compiled_ability(frame.card_id, frame.entry)
                .is_some_and(|ability| ability.selection.is_some())
                && let Some(caster) = &frame.source.actor
            {
                let (region, cells) = self.referenced_geometry(caster)?;
                if region != frame.source.region || cells != frame.source.cells {
                    return Err(GameError::UnsupportedMechanic(
                        "spellcaster moved before target resolution; casting-origin ruling unresolved"
                            .to_owned(),
                    ));
                }
            }
        } else if let Some(reference) = &frame.source.realm {
            (frame.source.region, frame.source.cells) = self.referenced_geometry(reference)?;
        }
        Ok(true)
    }

    pub(super) fn run_effect_frame(
        &mut self,
        mut frame: EffectFrame,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.pending_deathrites.is_some()
            || self.position.pending_trigger_order.is_some()
            || self.position.pending_ability_choice.is_some()
        {
            return self
                .continue_resolution(ResolutionContinuation::Effect(Box::new(frame)), outcomes);
        }
        if !frame.started {
            if frame.declaration_pending {
                return Err(GameError::IllegalAction);
            }
            if !self.prepare_effect_source(&mut frame)? {
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
                Effect::ChooseUnit(spec) => {
                    return self.begin_ability_unit_choice(frame, spec, outcomes);
                }
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
                            Some(ResolutionContinuation::Effect(Box::new(frame))),
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
            self.finish_magic_resolution(
                &DeferredMagicResolved {
                    card_id: card.card_id,
                    instance_id: card.instance_id.clone(),
                    owner: card.owner,
                },
                Some(card),
                outcomes,
            );
        }
    }

    pub(super) fn declare_effect_target(
        &self,
        frame: &mut EffectFrame,
        target: Option<&UnitTarget>,
    ) -> Result<(), GameError> {
        let ability = self
            .compiled_ability(frame.card_id, frame.entry)
            .ok_or(GameError::IllegalAction)?;
        if !frame.declaration_pending || (target.is_none() && !ability.optional_selection) {
            return Err(GameError::IllegalAction);
        }
        frame.target = target
            .map(|target| {
                self.unit_reference(target).map(|reference| UnitBinding {
                    reference,
                    state: BindingState::Active,
                })
            })
            .transpose()?;
        if target.is_none() {
            frame.cursor = ability.effects.len();
        }
        frame.declaration_pending = false;
        Ok(())
    }

    pub(super) fn select_effect_unit(
        &self,
        frame: &mut EffectFrame,
        unit: Option<&UnitTarget>,
    ) -> Result<(), GameError> {
        frame.chosen = unit.map(|unit| self.unit_reference(unit)).transpose()?;
        Ok(())
    }

    pub(super) fn effect_frame_value(&self, frame: &EffectFrame) -> Value {
        json!({
            "kind": "effect", "cardId": self.rules.cards[usize::from(frame.card_id.0)].id,
            "entry": match frame.entry { AbilityEntry::Magic => "magic", AbilityEntry::Genesis => "genesis", AbilityEntry::Activated => "activated" },
            "cursor": frame.cursor, "started": frame.started,
            "declarationPending": frame.declaration_pending,
            "source": frame.source.value(),
            "chosen": frame.chosen.as_ref().map(RealmReference::value),
            "target": frame.target.as_ref().map(|binding| json!({ "object": binding.reference.value(), "state": binding.state.as_str() })),
            "location": frame.location.as_ref().map(|binding| json!({ "location": binding.location, "site": binding.site.as_ref().map(RealmReference::value), "state": binding.state.as_str() })),
            "magic": frame.magic.as_ref().map(|card| self.card_value(card)),
        })
    }
}
