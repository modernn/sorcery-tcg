//! Focused proofs for intrinsic and tower-granted Spellcaster restrictions.

use serde_json::json;

use super::{
    CardId, CardInstance, CardSource, Cell, Game, Phase, Region, Seat, SitePosition,
    SummonPlacement, TemporaryModifierKind, UnitPosition,
};
use crate::canonical::{IdentityHash, identity_hash};
use crate::facts::{CardFacts, Thresholds};
use crate::synthetic::selfplay_manifest_with;

fn id(label: &str) -> IdentityHash {
    identity_hash(&json!({"elemental-spellcaster-test": label})).expect("test identity")
}

fn fixture() -> Game {
    let manifest = selfplay_manifest_with(6204, |manifest| {
        manifest["cards"]["north-spell-1"] = json!({
            "attack": 1,
            "cardType": "minion",
            "defense": 3,
            "elements": ["earth"],
            "gainsPowerRangedAndSpellcasterAtopTower": 2,
            "manaCost": 0,
            "spellcaster": true,
            "spellcasterElements": ["fire"],
            "thresholds": {"air": 0, "earth": 0, "fire": 1, "water": 0}
        });
        manifest["cards"]["north-spell-2"] = json!({
            "cardType": "magic", "manaCost": 0, "damageTargetUnit":1,
            "thresholds": {"air": 0, "earth": 0, "fire": 1, "water": 0}
        });
        manifest["cards"]["north-spell-3"] = json!({
            "cardType": "magic", "manaCost": 0, "damageTargetUnit":1,
            "thresholds": {"air": 1, "earth": 0, "fire": 0, "water": 0}
        });
        manifest["cards"]["tower"] = json!({
            "cardType": "site", "isTower": true, "elements": ["air"]
        });
        manifest["decks"]["north"]["atlas"]
            .as_array_mut()
            .unwrap()
            .push(json!("tower"));
    });
    let mut game = Game::from_manifest_json(&manifest).expect("elemental Spellcaster fixture");
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    game.position.players[0].domain_established = true;
    game.position.players[1].domain_established = true;
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

fn caster(game: &Game, label: &str) -> UnitPosition {
    let mut unit = SummonPlacement {
        card: CardInstance {
            card_id: card_id(game, "north-spell-1"),
            instance_id: id(label),
            owner: Seat::North,
            realm_entry: 1,
            source: CardSource::Spellbook,
        },
        controller: Seat::North,
        lance_count: 0,
        location: Cell::parse("C3").expect("cell"),
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

fn threshold(game: &Game, card: &str) -> Thresholds {
    let index = game
        .rules
        .cards
        .iter()
        .position(|definition| definition.id == card)
        .expect("threshold fixture card");
    match &game.rules.cards[index].facts {
        CardFacts::Magic(facts) => facts.thresholds,
        _ => panic!("threshold fixture card must be magic"),
    }
}

#[test]
fn intrinsic_fire_restriction_is_removed_by_tower_and_restored_after_departure() {
    let mut game = fixture();
    let mut unit = caster(&game, "caster");
    let unit_id = unit.card.instance_id.clone();
    game.position.units = vec![unit.clone()];
    assert!(game.spellcaster_can_cast(Seat::North, &unit_id, threshold(&game, "north-spell-2")));
    assert!(!game.spellcaster_can_cast(Seat::North, &unit_id, threshold(&game, "north-spell-3")));
    let tower_card = CardInstance {
        card_id: card_id(&game, "tower"),
        instance_id: id("tower"),
        owner: Seat::North,
        realm_entry: 1,
        source: CardSource::Atlas,
    };
    game.position.sites[unit.location.index()] = Some(SitePosition {
        card: tower_card,
        controller: Seat::North,
        last_flight_turn: None,
        warded: false,
    });
    assert!(game.minion_atop_tower(&unit));
    assert!(game.spellcaster_can_cast(Seat::North, &unit_id, threshold(&game, "north-spell-3")));

    unit.location = Cell::parse("C4").expect("departure cell");
    game.position.units[0] = unit.clone();
    assert!(!game.minion_atop_tower(&unit));
    assert!(game.spellcaster_can_cast(Seat::North, &unit_id, threshold(&game, "north-spell-2")));
    assert!(!game.spellcaster_can_cast(Seat::North, &unit_id, threshold(&game, "north-spell-3")));
}

#[test]
fn silence_removes_intrinsic_and_tower_spellcasting() {
    let mut game = fixture();
    let mut unit = caster(&game, "silenced-caster");
    let unit_id = unit.card.instance_id.clone();
    let tower_card = CardInstance {
        card_id: card_id(&game, "tower"),
        instance_id: id("tower"),
        owner: Seat::North,
        realm_entry: 1,
        source: CardSource::Atlas,
    };
    game.position.sites[unit.location.index()] = Some(SitePosition {
        card: tower_card,
        controller: Seat::North,
        last_flight_turn: None,
        warded: false,
    });
    unit.temporary_modifiers
        .grant(TemporaryModifierKind::Silence, 1, id("silence"));
    game.position.units = vec![unit.clone()];

    assert!(!game.minion_atop_tower(&unit));
    assert!(!game.spellcaster_can_cast(Seat::North, &unit_id, threshold(&game, "north-spell-2")));
    assert!(!game.spellcaster_can_cast(Seat::North, &unit_id, threshold(&game, "north-spell-3")));
}
