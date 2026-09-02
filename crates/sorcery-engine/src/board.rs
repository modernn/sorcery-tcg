//! Compact realm geometry shared by authoritative actions and transitions.

use std::error::Error;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const RANK_COUNT: u8 = 4;
const LAST_FILE: i8 = 4;
const LAST_RANK: i8 = 3;

/// One canonical two-by-two surface footprint in file-major cell order.
pub type SquareArea = [Cell; 4];

/// One of the realm's 20 cells, stored in file-major canonical order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cell(u8);

impl Cell {
    /// Every cell from A1 through E4 in file-major canonical order.
    pub const ALL: [Self; 20] = [
        Self(0),
        Self(1),
        Self(2),
        Self(3),
        Self(4),
        Self(5),
        Self(6),
        Self(7),
        Self(8),
        Self(9),
        Self(10),
        Self(11),
        Self(12),
        Self(13),
        Self(14),
        Self(15),
        Self(16),
        Self(17),
        Self(18),
        Self(19),
    ];

    /// Every in-bounds two-by-two area, ordered by its file-major anchor.
    pub const SQUARE_AREAS: [SquareArea; 12] = [
        [Self(0), Self(1), Self(4), Self(5)],
        [Self(1), Self(2), Self(5), Self(6)],
        [Self(2), Self(3), Self(6), Self(7)],
        [Self(4), Self(5), Self(8), Self(9)],
        [Self(5), Self(6), Self(9), Self(10)],
        [Self(6), Self(7), Self(10), Self(11)],
        [Self(8), Self(9), Self(12), Self(13)],
        [Self(9), Self(10), Self(13), Self(14)],
        [Self(10), Self(11), Self(14), Self(15)],
        [Self(12), Self(13), Self(16), Self(17)],
        [Self(13), Self(14), Self(17), Self(18)],
        [Self(14), Self(15), Self(18), Self(19)],
    ];

    /// Parses an exact uppercase cell name from A1 through E4.
    ///
    /// # Errors
    ///
    /// Returns [`CellParseError`] for any other string.
    pub fn parse(value: &str) -> Result<Self, CellParseError> {
        let [file @ b'A'..=b'E', rank @ b'1'..=b'4'] = value.as_bytes() else {
            return Err(CellParseError);
        };
        Ok(Self((file - b'A') * RANK_COUNT + (rank - b'1')))
    }

    /// Returns the zero-based file-major index used by compact board arrays.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn file_index(self) -> i8 {
        self.file()
    }

    pub(crate) const fn rank_index(self) -> i8 {
        self.rank()
    }

    /// Translates this cell by the displacement between two anchors.
    #[must_use]
    pub(crate) fn translated(self, from: Self, to: Self) -> Option<Self> {
        Self::from_coordinates(
            self.file() + to.file() - from.file(),
            self.rank() + to.rank() - from.rank(),
        )
    }

    /// Returns bordering cells in west, south, east, north order.
    ///
    /// A top-bottom connection appends the opposite rank after ordinary borders.
    pub fn bordering(self, connects_top_bottom: bool) -> impl Iterator<Item = Self> {
        let file = self.file();
        let rank = self.rank();
        [
            Self::from_coordinates(file - 1, rank),
            Self::from_coordinates(file, rank - 1),
            Self::from_coordinates(file + 1, rank),
            Self::from_coordinates(file, rank + 1),
            if connects_top_bottom && rank == 0 {
                Self::from_coordinates(file, LAST_RANK)
            } else if connects_top_bottom && rank == LAST_RANK {
                Self::from_coordinates(file, 0)
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
    }

    /// Returns diagonal cells in northwest, southwest, northeast, southeast order.
    ///
    /// A top-bottom connection appends wrapped west and east diagonals.
    pub fn diagonals(self, connects_top_bottom: bool) -> impl Iterator<Item = Self> {
        let file = self.file();
        let rank = self.rank();
        let wrapped_rank = if connects_top_bottom && rank == 0 {
            Some(LAST_RANK)
        } else if connects_top_bottom && rank == LAST_RANK {
            Some(0)
        } else {
            None
        };
        [
            Self::from_coordinates(file - 1, rank - 1),
            Self::from_coordinates(file - 1, rank + 1),
            Self::from_coordinates(file + 1, rank - 1),
            Self::from_coordinates(file + 1, rank + 1),
            wrapped_rank.and_then(|rank| Self::from_coordinates(file - 1, rank)),
            wrapped_rank.and_then(|rank| Self::from_coordinates(file + 1, rank)),
        ]
        .into_iter()
        .flatten()
    }

    /// Returns the non-wrapping Manhattan distance between two cells.
    #[must_use]
    pub const fn manhattan_distance(self, other: Self) -> u8 {
        self.file().abs_diff(other.file()) + self.rank().abs_diff(other.rank())
    }

    const fn file(self) -> i8 {
        (self.0 / RANK_COUNT).cast_signed()
    }

    const fn rank(self) -> i8 {
        (self.0 % RANK_COUNT).cast_signed()
    }

    fn from_coordinates(file: i8, rank: i8) -> Option<Self> {
        if (0..=LAST_FILE).contains(&file) && (0..=LAST_RANK).contains(&rank) {
            Some(Self(
                file.cast_unsigned() * RANK_COUNT + rank.cast_unsigned(),
            ))
        } else {
            None
        }
    }
}

/// Translates a complete square footprint without allocating.
#[must_use]
pub(crate) fn translated_square(area: SquareArea, from: Cell, to: Cell) -> Option<SquareArea> {
    Some([
        area[0].translated(from, to)?,
        area[1].translated(from, to)?,
        area[2].translated(from, to)?,
        area[3].translated(from, to)?,
    ])
}

impl fmt::Display for Cell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = char::from(b'A' + self.0 / RANK_COUNT);
        let rank = char::from(b'1' + self.0 % RANK_COUNT);
        write!(formatter, "{file}{rank}")
    }
}

impl FromStr for Cell {
    type Err = CellParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for Cell {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Cell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(de::Error::custom)
    }
}

/// A string was not an exact realm cell name from A1 through E4.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellParseError;

impl fmt::Display for CellParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cell must be an uppercase name from A1 through E4")
    }
}

impl Error for CellParseError {}

/// A realm occupancy layer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(u8)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    /// The normal surface layer.
    Surface,
    /// The burrowed layer.
    Underground,
    /// The submerged layer.
    Underwater,
    /// The Void layer.
    Void,
}

/// A cell and occupancy layer at the JSON boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    /// Realm cell.
    pub cell: Cell,
    /// Occupancy layer.
    pub region: Region,
}

#[cfg(test)]
mod tests {
    use super::{Cell, translated_square};

    #[test]
    fn square_areas_are_the_twelve_canonical_file_major_footprints() {
        let values = Cell::SQUARE_AREAS.map(|area| area.map(|cell| cell.to_string()));
        assert_eq!(
            values,
            [
                ["A1", "A2", "B1", "B2"],
                ["A2", "A3", "B2", "B3"],
                ["A3", "A4", "B3", "B4"],
                ["B1", "B2", "C1", "C2"],
                ["B2", "B3", "C2", "C3"],
                ["B3", "B4", "C3", "C4"],
                ["C1", "C2", "D1", "D2"],
                ["C2", "C3", "D2", "D3"],
                ["C3", "C4", "D3", "D4"],
                ["D1", "D2", "E1", "E2"],
                ["D2", "D3", "E2", "E3"],
                ["D3", "D4", "E3", "E4"],
            ]
        );
    }

    #[test]
    fn square_translation_preserves_order_and_rejects_off_board_destinations() {
        let area = Cell::SQUARE_AREAS[5];
        assert_eq!(
            translated_square(area, area[0], Cell::parse("B2").expect("cell")),
            Some(Cell::SQUARE_AREAS[4])
        );
        assert_eq!(
            translated_square(area, area[0], Cell::parse("A4").expect("cell")),
            None
        );
    }
}
