//! Borrowed current-state layer slice: source activity, caster grants, then conditional disability.
//! No derived ability is stored in a position or printed card definition.
use super::{
    ArtifactPosition, AuraEffect, CardDefinition, CardFacts, Cell, ElementSet, Game, GameError,
    Region, TemporaryModifierKind, UnitKind, UnitPosition,
};
use crate::facts::ArtifactEffect;
use std::cell::OnceCell;

type CasterCapability = Option<Option<ElementSet>>;
type SupportedCaster<'a> = (&'a UnitPosition, CasterCapability);

pub(super) struct EffectiveView<'a> {
    game: &'a Game,
    capabilities: OnceCell<Vec<SupportedCaster<'a>>>,
}

impl<'a> EffectiveView<'a> {
    pub(super) const fn new(game: &'a Game) -> Self {
        Self {
            game,
            capabilities: OnceCell::new(),
        }
    }

    pub(super) fn aura_covers_cell(&self, cell: Cell, effect: AuraEffect) -> bool {
        self.game.position.auras.iter().any(|aura| {
            aura.cells.contains(&cell)
                && matches!(&self.game.rules.cards[usize::from(aura.card.card_id.0)].facts,
                    CardFacts::Aura(facts) if facts.effect == effect)
        })
    }

    pub(super) fn artifact_active(&self, artifact: &ArtifactPosition) -> bool {
        // Location resolves this physical instance's current bearer, including footprint cell.
        self.game.artifact_location(artifact).is_ok_and(|location| {
            !(self.game.position.sites[location.cell.index()].is_some()
                && self.aura_covers_cell(
                    location.cell,
                    AuraEffect::SilenceAffectedSitesAndArtifactsThere,
                ))
        })
    }

    pub(super) fn silenced(&self, unit: &UnitPosition) -> bool {
        unit.temporary_modifiers.has(TemporaryModifierKind::Silence)
            || self.silenced_cells(Game::unit_occupied_cells(unit), None)
    }

    /// Entry queries may project the incoming site before its placement exists.
    pub(super) fn silenced_cells(&self, cells: &[Cell], incoming_site: Option<Cell>) -> bool {
        cells.iter().any(|cell| {
            (self.game.position.sites[cell.index()].is_some() || incoming_site == Some(*cell))
                && self.aura_covers_cell(*cell, AuraEffect::SilenceMinionsAtAffectedSites)
        })
    }

    fn independently_disabled(&self, unit: &UnitPosition) -> bool {
        if unit.disabled_until_damaged || !unit.disable_effects.is_empty() {
            return true;
        }
        let CardFacts::Minion(facts) =
            &self.game.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return true;
        };
        let occupied = Game::unit_occupied_cells(unit);
        (facts.waterbound && !occupied.iter().any(|cell| self.game.is_water_site(*cell)))
            || (facts.landbound && !occupied.iter().any(|cell| self.game.is_land_site(*cell)))
    }

    /// Stage-three capability: deliberately never queries conditional disability.
    fn supported_caster(&self, unit: &UnitPosition) -> CasterCapability {
        if self.independently_disabled(unit) || self.silenced(unit) {
            return None;
        }
        let CardFacts::Minion(facts) =
            &self.game.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return None;
        };
        let tower = self.game.minion_atop_tower_geometry(unit);
        if tower || (facts.spellcaster && facts.spellcaster_elements.is_none()) {
            return Some(None);
        }
        let mut elements = facts
            .spellcaster
            .then_some(facts.spellcaster_elements)
            .flatten();
        for artifact in &self.game.position.artifacts {
            if artifact.carried_by(UnitKind::Minion, unit.controller, &unit.card.instance_id)
                && self.artifact_active(artifact)
                && let Ok(facts) = self.game.artifact_facts(artifact)
                && let Some(grant) = facts.bearer_spellcaster_elements
            {
                elements = Some(
                    grant
                        .iter()
                        .fold(elements.unwrap_or_default(), ElementSet::with),
                );
            }
        }
        elements.map(Some)
    }

    pub(super) fn disabled(&self, unit: &UnitPosition) -> bool {
        if self.independently_disabled(unit) {
            return true;
        }
        if self.silenced(unit) {
            return false;
        } // Silence removes this conditional ability, not basic actions.
        let CardFacts::Minion(facts) =
            &self.game.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return true;
        };
        if !facts.disabled_unless_adjacent_allied_spellcaster {
            return false;
        }
        // Supported witnesses can witness themselves. Unsupported Spirits never bootstrap a cycle.
        let occupied = Game::unit_occupied_cells(unit);
        let avatar = &self.game.position.players[super::seat_index(unit.controller)].avatar;
        if unit.region == Region::Surface
            && Game::footprints_here_or_bordering(occupied, &[avatar.location])
        {
            return false;
        }
        !self.stage_three().iter().any(|(ally, capability)| {
            ally.controller == unit.controller
                && ally.region == unit.region
                && Game::footprints_here_or_bordering(occupied, Game::unit_occupied_cells(ally))
                && capability.is_some()
        })
    }

    fn stage_three(&self) -> &[SupportedCaster<'a>] {
        self.capabilities.get_or_init(|| {
            self.game
                .position
                .units
                .iter()
                .map(|unit| (unit, self.supported_caster(unit)))
                .collect()
        })
    }

    pub(super) fn caster(&self, unit: &UnitPosition) -> CasterCapability {
        // Build the finite unconditional witness set once per borrowed view. Each supported
        // caster supplies its own witness; unsupported Spirits never bootstrap one another.
        self.stage_three()
            .iter()
            .find(|(candidate, _)| {
                candidate.card.instance_id == unit.card.instance_id
                    && candidate.card.realm_entry == unit.card.realm_entry
            })
            .map_or_else(
                || self.supported_caster(unit),
                |(_, capability)| *capability,
            )
    }
}

impl Game {
    pub(super) fn artifact_source_active(&self, artifact: &ArtifactPosition) -> bool {
        EffectiveView::new(self).artifact_active(artifact)
    }
}

pub(super) fn ensure_suppression_context_supported(
    cards: &[CardDefinition],
) -> Result<(), GameError> {
    for aura in cards.iter().filter(|card| {
        matches!(&card.facts, CardFacts::Aura(facts)
        if facts.effect == AuraEffect::SilenceAffectedSitesAndArtifactsThere)
    }) {
        if let Some(artifact) = cards.iter().find(|card| {
            matches!(&card.facts, CardFacts::Artifact(facts)
            if matches!(facts.effect, ArtifactEffect::AtEndOfControllerTurnUntapNearbyAllies
                | ArtifactEffect::AtEndOfEachTurnSiteControllerLosesLife(_)
                | ArtifactEffect::AtStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn(_)))
        }) {
            return Err(GameError::UnsupportedMechanic(format!(
                "source suppression context {} + {} requires captured legacy artifact triggers",
                aura.id, artifact.id
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
