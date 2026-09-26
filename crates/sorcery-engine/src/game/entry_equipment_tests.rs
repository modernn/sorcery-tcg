//! Focused runtime proofs for generic minion entry equipment.

use serde_json::json;

use super::{
    ArtifactPlacement, CardId, CardInstance, CardSource, Cell, Game, OutcomeLog, Phase, Region,
    Seat, SitePosition, SummonPlacement, TemporaryModifierKind, UnitPosition,
};
use crate::canonical::{IdentityHash, identity_hash};
use crate::synthetic::selfplay_manifest_with;

fn id(label: &str) -> IdentityHash {
    identity_hash(&json!({"entry-equipment-test": label})).expect("test identity")
}

fn fixture() -> Game {
    let manifest = selfplay_manifest_with(9217, |manifest| {
        manifest["cards"]["north-spell-1"] = json!({
            "attack": 1,
            "cardType": "minion",
            "defense": 2,
            "entersCarrying": ["north-spell-50"],
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-48"] = json!({
            "attack": 1,
            "cardType": "minion",
            "defense": 1,
            "entersCarrying": ["north-spell-50"],
            "genesisProgram": {"effects": [{
                "op": "draw", "zone": "spellbook", "count": 1
            }]},
            "manaCost": null,
            "token": true,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-49"] = json!({
            "cardType": "magic",
            "effectProgram": {"effects": [{
                "op": "summon-token", "token": "north-spell-48", "count": 1,
                "destination": "source"
            }]},
            "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-spell-50"] = json!({
            "cardType": "artifact",
            "grantsBearerPower": 2,
            "manaCost": null,
            "token": true,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["decks"]["north"]["spellbook"]
            .as_array_mut()
            .expect("north spellbook")
            .retain(|card| card != "north-spell-48" && card != "north-spell-50");
    });
    let mut game = Game::from_manifest_json(&manifest).expect("entry fixture");
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    for (label, cell) in [
        ("site-a", Cell::parse("C3").expect("site cell")),
        ("site-b", Cell::parse("D3").expect("site cell")),
    ] {
        let mut card = CardInstance {
            card_id: card_id(&game, "north-site-1"),
            instance_id: id(label),
            owner: Seat::North,
            realm_entry: 0,
            source: CardSource::Atlas,
        };
        card.enter_realm().expect("site realm entry");
        game.position.sites[cell.index()] = Some(SitePosition {
            card,
            controller: Seat::North,
            last_flight_turn: None,
            warded: false,
        });
    }
    game
}

fn card_id(game: &Game, name: &str) -> CardId {
    CardId(
        u16::try_from(
            game.rules
                .cards
                .iter()
                .position(|card| card.id == name)
                .expect("fixture card"),
        )
        .expect("card index"),
    )
}

fn regular_minion(game: &Game, label: &str, owner: Seat, controller: Seat) -> UnitPosition {
    let mut unit = SummonPlacement {
        card: CardInstance {
            card_id: card_id(game, "north-spell-1"),
            instance_id: id(label),
            owner,
            realm_entry: 0,
            source: CardSource::Spellbook,
        },
        controller,
        lance_count: 0,
        location: Cell::parse("C3").expect("fixture cell"),
        occupied_cells: None,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit();
    unit.card.enter_realm().expect("realm entry");
    unit.summoning_sickness = false;
    unit
}

fn site_cells(game: &Game) -> [Cell; 2] {
    let mut cells = Cell::ALL
        .into_iter()
        .filter(|cell| game.position.sites[cell.index()].is_some());
    [
        cells.next().expect("first fixture site"),
        cells.next().expect("second fixture site"),
    ]
}

#[test]
fn token_entry_equipment_survives_immediate_bearer_silence_and_disable() {
    let mut game = fixture();
    let mut token = game
        .create_token_unit(
            Seat::North,
            "north-spell-48",
            &id("token-source"),
            super::Location {
                cell: site_cells(&game)[0],
                region: Region::Surface,
            },
            0,
            game.position.state_version,
        )
        .expect("token unit");
    token
        .temporary_modifiers
        .grant(TemporaryModifierKind::Silence, 1, id("silence"));
    token.disabled_until_damaged = true;
    let bearer_id = token.card.instance_id.clone();
    let mut events = Vec::new();
    game.finish_token_entries(
        vec![super::TokenEntryContinuation {
            seat: Seat::North,
            token,
            source_instance_id: id("token-source"),
            mana_paid: 0,
        }],
        &mut OutcomeLog::Record(&mut events),
    )
    .expect("token entry");
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == bearer_id)
    );
    assert_eq!(game.position.artifacts.len(), 1);
    assert!(matches!(
        &game.position.artifacts[0].placement,
        ArtifactPlacement::Carried { bearer, .. }
            if bearer.instance_id() == &bearer_id && bearer.seat() == Seat::North
    ));
    assert!(events.iter().all(|(kind, _)| kind != "spell-drawn"));
    assert!(
        game.position.units[0]
            .temporary_modifiers
            .has(TemporaryModifierKind::Silence)
    );
    assert!(game.position.units[0].disabled_until_damaged);
}

#[test]
fn entry_equipment_owner_follows_entry_controller_for_a_reowned_minion() {
    let mut game = fixture();
    let unit = regular_minion(&game, "reowned", Seat::South, Seat::North);
    let unit_id = unit.card.instance_id.clone();
    game.position.units.push(unit);
    game.begin_entry_equipment(
        vec![(
            Seat::North,
            unit_id.clone(),
            card_id(&game, "north-spell-1"),
        )],
        false,
        &mut OutcomeLog::Ignore,
    )
    .expect("entry equipment");
    assert_eq!(game.position.artifacts.len(), 1);
    assert_eq!(game.position.artifacts[0].card.owner, Seat::North);
    assert!(game.position.artifacts[0].bearer().is_some_and(|bearer| {
        bearer.instance_id() == &unit_id && bearer.seat() == Seat::North
    }));
}

#[test]
fn simultaneous_token_entries_receive_equipment_before_genesis_settlement() {
    for region in [Region::Surface, Region::Underground] {
        let mut game = fixture();
        let source = id("batch-source");
        let entries = site_cells(&game)
            .into_iter()
            .enumerate()
            .map(|(ordinal, cell)| {
                let token = game
                    .create_token_unit(
                        Seat::North,
                        "north-spell-48",
                        &source,
                        super::Location { cell, region },
                        ordinal,
                        game.position.state_version,
                    )
                    .unwrap();
                super::TokenEntryContinuation {
                    seat: Seat::North,
                    token,
                    source_instance_id: source.clone(),
                    mana_paid: 0,
                }
            })
            .collect();
        let mut events = Vec::new();
        game.finish_token_entries(entries, &mut OutcomeLog::Record(&mut events))
            .unwrap();
        assert_eq!(
            game.position.units.len(),
            if region == Region::Surface { 2 } else { 0 }
        );
        assert_eq!(game.position.artifacts.len(), 2);
        let second_entry = events
            .iter()
            .rposition(|(kind, _)| kind == "minion-summoned")
            .expect("second entry");
        let first_artifact = events
            .iter()
            .position(|(kind, _)| kind == "artifact-conjured")
            .expect("entry artifact");
        assert!(first_artifact > second_entry);
        if region == Region::Underground {
            let departure = events
                .iter()
                .position(|(kind, _)| kind == "minion-banished")
                .unwrap();
            let last_equipment = events
                .iter()
                .rposition(|(kind, _)| kind == "artifact-conjured")
                .unwrap();
            assert!(last_equipment < departure);
            assert!(game.position.artifacts.iter().all(|a| a.bearer().is_none()));
        } else {
            assert!(game.position.pending_trigger_order.is_some());
            for _ in 0..4 {
                if game.position.pending_trigger_order.is_none() {
                    break;
                }
                let action = game.legal_actions().unwrap().into_iter().next().unwrap();
                events.extend(game.apply_action_recorded(&action).unwrap().0);
            }
            assert!(game.position.pending_trigger_order.is_none());
            assert_eq!(
                events
                    .iter()
                    .filter(|(kind, _)| kind == "spell-drawn")
                    .count(),
                2
            );
            let first_genesis = events
                .iter()
                .position(|(kind, _)| kind == "spell-drawn")
                .unwrap();
            assert_eq!(
                events[..first_genesis]
                    .iter()
                    .filter(|(kind, _)| kind == "artifact-conjured")
                    .count(),
                2
            );
        }
    }
}
