use std::mem::size_of;

use sorcery_engine::board::{Cell, Location, Region};

fn names(cells: impl Iterator<Item = Cell>) -> String {
    cells
        .map(|cell| cell.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn all_cells_are_compact_and_file_major() {
    let expected: Vec<_> = ['A', 'B', 'C', 'D', 'E']
        .into_iter()
        .flat_map(|file| (1..=4).map(move |rank| format!("{file}{rank}")))
        .collect();

    assert_eq!(
        (
            Cell::ALL.map(|cell| cell.to_string()).to_vec(),
            size_of::<Cell>(),
            size_of::<Region>(),
        ),
        (expected, 1, 1)
    );
}

#[test]
fn cell_string_and_location_json_round_trip() {
    let location = Location {
        cell: Cell::parse("C2").expect("valid cell"),
        region: Region::Underwater,
    };

    let json = serde_json::to_string(&location).expect("serialize location");
    let restored: Location = serde_json::from_str(&json).expect("deserialize location");

    assert_eq!(
        (json, restored),
        (
            r#"{"cell":"C2","region":"underwater"}"#.to_owned(),
            location,
        )
    );
}

#[test]
fn invalid_cell_names_are_rejected_by_parse_and_serde() {
    for invalid in ["", "A", "A0", "A5", "F1", "a1", "A10", " A1", "A1 "] {
        assert!(
            Cell::parse(invalid).is_err()
                && serde_json::from_str::<Cell>(&format!(r#""{invalid}""#)).is_err(),
            "accepted invalid cell {invalid:?}"
        );
    }
}

#[test]
fn bordering_cells_match_typescript_order_with_wrap_appended() {
    let middle = Cell::parse("C2").expect("middle cell");
    let bottom = Cell::parse("C1").expect("bottom cell");
    let top = Cell::parse("C4").expect("top cell");

    assert_eq!(
        (
            names(middle.bordering(false)),
            names(bottom.bordering(true)),
            names(top.bordering(true)),
        ),
        (
            "B2,C1,D2,C3".to_owned(),
            "B1,D1,C2,C4".to_owned(),
            "B4,C3,D4,C1".to_owned(),
        )
    );
}

#[test]
fn diagonal_cells_match_typescript_order_with_wrap_appended() {
    let middle = Cell::parse("C2").expect("middle cell");
    let bottom = Cell::parse("C1").expect("bottom cell");
    let top = Cell::parse("C4").expect("top cell");

    assert_eq!(
        (
            names(middle.diagonals(false)),
            names(bottom.diagonals(true)),
            names(top.diagonals(true)),
        ),
        (
            "B1,B3,D1,D3".to_owned(),
            "B2,D2,B4,D4".to_owned(),
            "B3,D3,B1,D1".to_owned(),
        )
    );
}

#[test]
fn manhattan_distance_does_not_use_top_bottom_wrap() {
    let a1 = Cell::parse("A1").expect("A1");
    let e4 = Cell::parse("E4").expect("E4");

    assert_eq!(
        (a1.manhattan_distance(e4), e4.manhattan_distance(a1)),
        (7, 7)
    );
}
