//! Shared spatial selection for ability declarations and resolution checks.

use super::ability::{SelectionSpec, SpatialRelation};
use super::{
    Cell, Game, GameError, Location, MagicChoice, Region, Seat, UnitKind, UnitQuery, UnitTarget,
};

impl Game {
    fn selection_cells(
        &self,
        region: Region,
        anchor: &[Cell],
        relation: SpatialRelation,
    ) -> Option<Vec<Cell>> {
        match relation {
            SpatialRelation::Anywhere => None,
            SpatialRelation::Nearby | SpatialRelation::Adjacent => Some(
                Cell::ALL
                    .into_iter()
                    .filter(|cell| {
                        let target = std::slice::from_ref(cell);
                        match relation {
                            SpatialRelation::Nearby => Self::footprints_nearby(anchor, target),
                            SpatialRelation::Adjacent => {
                                Self::footprints_here_or_bordering(anchor, target)
                            }
                            _ => unreachable!("geometric relation"),
                        }
                    })
                    .collect(),
            ),
            SpatialRelation::Measured(steps) => Some(
                self.locations_within_measured_steps_from_cells(anchor, region, steps)
                    .into_iter()
                    .map(|location| location.cell)
                    .collect(),
            ),
        }
    }

    pub(super) fn selected_locations(
        &self,
        region: Region,
        anchor: &[Cell],
        relation: SpatialRelation,
    ) -> Vec<Location> {
        self.selection_cells(region, anchor, relation)
            .unwrap_or_else(|| Cell::ALL.to_vec())
            .into_iter()
            .filter(|cell| self.location_exists_in_region(*cell, region))
            .map(|cell| Location { cell, region })
            .collect()
    }

    pub(super) fn selected_units(
        &self,
        query: UnitQuery<'_>,
        relation: SpatialRelation,
        targeted_by: Option<Seat>,
    ) -> Vec<UnitTarget> {
        let cells = match query.region {
            Some(region) => self.selection_cells(region, query.cells.unwrap_or(&[]), relation),
            None if relation == SpatialRelation::Anywhere => None,
            None => return Vec::new(),
        };
        let query = UnitQuery {
            cells: cells.as_deref(),
            ..query
        };
        self.query_units_for(query, targeted_by)
            .into_iter()
            .map(|(instance_id, kind, seat)| match kind {
                UnitKind::Avatar => UnitTarget::Avatar { instance_id, seat },
                UnitKind::Minion => UnitTarget::Minion { instance_id, seat },
            })
            .collect()
    }

    pub(super) fn selection_choices(
        &self,
        seat: Seat,
        region: Region,
        anchor: &[Cell],
        selection: Option<SelectionSpec>,
    ) -> Vec<MagicChoice> {
        match selection {
            None => vec![MagicChoice::default()],
            Some(SelectionSpec::Unit { kind, relation }) => self
                .selected_units(
                    UnitQuery {
                        region: Some(region),
                        cells: Some(anchor),
                        kind,
                        controller: None,
                        exclude: None,
                    },
                    relation,
                    Some(seat),
                )
                .into_iter()
                .map(|target| MagicChoice {
                    target: Some(target),
                    ..MagicChoice::default()
                })
                .collect(),
            Some(SelectionSpec::Location { relation }) => self
                .selected_locations(region, anchor, relation)
                .into_iter()
                .map(|target_location| MagicChoice {
                    target_location: Some(target_location),
                    ..MagicChoice::default()
                })
                .collect(),
        }
    }

    pub(super) fn compiled_magic_choices(
        &self,
        seat: Seat,
        caster: &super::IdentityHash,
        selection: Option<SelectionSpec>,
    ) -> Result<Vec<MagicChoice>, GameError> {
        let (origin, cells) = self.spellcaster_occupied_cells(seat, caster)?;
        Ok(self.selection_choices(seat, origin.region, cells, selection))
    }
}
