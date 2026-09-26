//! Focused proofs for engine-issued damage replacement choices.

use serde_json::json;

use super::{
    ArtifactPlacement, ArtifactPosition, CardId, CardInstance, CardSource, Cell, Game,
    IssuedAction, Phase, Region, Seat, SitePosition, SummonPlacement, UnitPosition, UnitTarget,
};
use crate::canonical::{IdentityHash, identity_hash};
use crate::synthetic::selfplay_manifest_with;

fn id(label: &str) -> IdentityHash {
    identity_hash(&json!({"damage-order-test": label})).expect("test identity")
}

fn fixture() -> Game {
    let manifest = selfplay_manifest_with(9917, |manifest| {
        let thresholds = json!({"air": 0, "earth": 0, "fire": 0, "water": 0});
        manifest["cards"]["north-spell-1"] = json!({
            "attack": 2, "cardType": "minion", "defense": 3, "manaCost": 0,
            "ranged": true, "thresholds": thresholds,
        });
        manifest["cards"]["south-spell-1"] = json!({
            "attack": 1, "cardType": "minion", "defense": 10, "manaCost": 0,
            "thresholds": thresholds,
        });
        manifest["cards"]["north-spell-49"] = json!({
            "bearerUnitStrike": {"damageBonus": 1, "destroyAfterStrike": true}, "cardType": "artifact", "manaCost": 0,
            "thresholds": thresholds,
        });
        manifest["cards"]["north-spell-50"] = json!({
            "cardType": "artifact", "manaCost": 0,
            "nearbyStrikesAgainstUnitsDealDoubleDamage": true, "thresholds": thresholds,
        });
    });
    let mut game = Game::from_manifest_json(&manifest).expect("damage-order fixture");
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

fn unit(game: &Game, name: &str, label: &str, seat: Seat, cell: &str) -> UnitPosition {
    let mut unit = SummonPlacement {
        card: CardInstance {
            card_id: card_id(game, name),
            instance_id: id(label),
            owner: seat,
            realm_entry: 1,
            source: CardSource::Spellbook,
        },
        controller: seat,
        lance_count: 0,
        location: Cell::parse(cell).expect("cell"),
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

fn setup() -> (Game, IdentityHash, IdentityHash) {
    let mut game = fixture();
    let shooter = unit(&game, "north-spell-1", "shooter", Seat::North, "C3");
    let target = unit(&game, "south-spell-1", "target", Seat::South, "C4");
    let shooter_id = shooter.card.instance_id.clone();
    let target_id = target.card.instance_id.clone();
    for (seat, cell) in [
        (Seat::North, Cell::parse("C3").unwrap()),
        (Seat::South, Cell::parse("C4").unwrap()),
    ] {
        let card = game.position.players[super::seat_index(seat)]
            .hand_atlas
            .remove(0);
        game.position.sites[cell.index()] = Some(SitePosition {
            card,
            controller: seat,
            last_flight_turn: None,
            warded: false,
        });
    }
    game.position.units = vec![shooter, target];
    game.position.artifacts = vec![
        ArtifactPosition {
            card: CardInstance {
                card_id: card_id(&game, "north-spell-49"),
                instance_id: id("add"),
                owner: Seat::North,
                realm_entry: 1,
                source: CardSource::Spellbook,
            },
            placement: ArtifactPlacement::Carried {
                bearer: UnitTarget::Minion {
                    instance_id: shooter_id.clone(),
                    seat: Seat::North,
                },
                cell: None,
            },
        },
        ArtifactPosition {
            card: CardInstance {
                card_id: card_id(&game, "north-spell-50"),
                instance_id: id("double"),
                owner: Seat::North,
                realm_entry: 1,
                source: CardSource::Spellbook,
            },
            placement: ArtifactPlacement::Loose {
                location: Cell::parse("C4").unwrap(),
                region: Region::Surface,
            },
        },
    ];
    (game, shooter_id, target_id)
}

fn ranged_action(game: &Game, shooter: &IdentityHash, target: &IdentityHash) -> IssuedAction {
    game.legal_actions().expect("legal actions").into_iter().find(|action| {
        matches!(&action.descriptor, super::ActionDescriptor::ShootProjectile { shooter_instance_id, hit: Some(UnitTarget::Minion { instance_id, .. }), .. } if shooter_instance_id == shooter && instance_id == target)
    }).expect("issued ranged hit")
}

#[test]
fn same_controller_order_can_choose_add_then_double_or_double_then_add() {
    for choose_double_first in [false, true] {
        let (mut game, shooter, target) = setup();
        let action = ranged_action(&game, &shooter, &target);
        let (events, _) = game.apply_action_recorded(&action).expect("ranged strike");
        assert_eq!(game.position.phase, Phase::DamageOrder);
        let choices = game.legal_actions().expect("damage choices");
        assert_eq!(choices.len(), 2);
        let selected = choices
            .into_iter()
            .find(|choice| {
                let index = choice.descriptor.clone();
                if let super::ActionDescriptor::ChooseDamageModifier { modifier_index } = index {
                    (choose_double_first && modifier_index == 1)
                        || (!choose_double_first && modifier_index == 0)
                } else {
                    false
                }
            })
            .expect("selected replacement");
        let (first_events, _) = game.apply_action_recorded(&selected).expect("replacement");
        let expected = if choose_double_first { 5 } else { 6 };
        assert_ne!(game.position.phase, Phase::DamageOrder);
        assert_eq!(game.position.units[1].damage, expected);
        assert!(first_events.iter().any(|(kind, data)| kind == "damage-modifier-applied" && data["amount"] == expected));
        assert!(events.iter().any(|(kind, _)| kind == "projectile-shot"));
        assert_eq!(game.position.artifacts.len(), 1);
        assert_eq!(
            first_events
                .iter()
                .filter(|(kind, _)| kind == "artifact-consumed-after-strike")
                .count(),
            1
        );
    }
}

#[test]
fn forged_or_stale_damage_choice_is_rejected() {
    let (mut game, shooter, target) = setup();
    let action = ranged_action(&game, &shooter, &target);
    game.apply_action_recorded(&action).expect("ranged strike");
    let stale = game
        .legal_actions()
        .expect("damage choices")
        .into_iter()
        .next()
        .expect("choice");
    let mut forged = stale.clone();
    forged.descriptor = super::ActionDescriptor::ChooseDamageModifier {
        modifier_index: 999,
    };
    let phase = game.position.phase;
    let state_version = game.position.state_version;
    assert!(game.apply_action_recorded(&forged).is_err());
    assert_eq!(game.position.phase, phase);
    assert_eq!(game.position.state_version, state_version);
    let valid = game
        .legal_actions()
        .expect("damage choices")
        .into_iter()
        .next()
        .expect("valid choice");
    game.apply_action_recorded(&valid)
        .expect("valid replacement");
    assert!(game.apply_action_recorded(&stale).is_err());
}

#[test]
fn active_then_nonactive_controller_order_is_forced_without_choice() {
    let (mut game, shooter, target) = setup();
    game.position.artifacts[1].placement = ArtifactPlacement::Carried {
        bearer: UnitTarget::Minion {
            instance_id: target.clone(),
            seat: Seat::South,
        },
        cell: None,
    };
    let action = ranged_action(&game, &shooter, &target);
    let (events, _) = game.apply_action_recorded(&action).expect("ranged strike");
    assert_ne!(game.position.phase, Phase::DamageOrder);
    assert!(
        events
            .iter()
            .any(|(kind, data)| { kind == "strike-damage-allocated" && data["amount"] == 6 })
    );
    assert_eq!(game.position.artifacts.len(), 1);
}

#[test]
fn prevention_happens_after_replacement_and_consumption_still_occurs() {
    let (mut game, shooter, target) = setup();
    game.position.units[1].warded = true;
    let action = ranged_action(&game, &shooter, &target);
    game.apply_action_recorded(&action).expect("ranged strike");
    let choice = game
        .legal_actions()
        .expect("damage choices")
        .into_iter()
        .next()
        .expect("choice");
    let (events, _) = game
        .apply_action_recorded(&choice)
        .expect("replacement and prevention");
    assert_eq!(game.position.units[1].damage, 0);
    let last_modifier = events
        .iter()
        .rposition(|(kind, _)| kind == "damage-modifier-applied")
        .unwrap();
    let prevention = events
        .iter()
        .position(|(kind, _)| kind == "ward-broken")
        .unwrap();
    let consumption = events
        .iter()
        .position(|(kind, _)| kind == "artifact-consumed-after-strike")
        .unwrap();
    assert!(last_modifier < prevention && prevention < consumption);
    assert_eq!(game.position.artifacts.len(), 1);
    assert!(
        events
            .iter()
            .any(|(kind, data)| { kind == "ward-broken" && data["instanceId"] == target.as_str() })
    );
    assert!(
        events
            .iter()
            .any(|(kind, _)| kind == "artifact-consumed-after-strike")
    );
}

#[test]
fn legacy_counter_bonuses_remain_independently_orderable() {
    let (mut game, shooter, target) = setup();
    game.position.units[0].carried_lance_count = 2;
    game.position.artifacts.remove(0);
    let action = ranged_action(&game, &shooter, &target);
    game.apply_action_recorded(&action).expect("ranged strike");
    for selected_index in [0, 1] {
        let action = game.legal_actions().unwrap().into_iter().find(|action| {
            matches!(action.descriptor, super::ActionDescriptor::ChooseDamageModifier { modifier_index } if modifier_index == selected_index)
        }).unwrap();
        game.apply_action_recorded(&action)
            .expect("add then double, final add automatic");
    }
    assert_eq!(game.position.units[1].damage, 7);
    assert_eq!(game.position.units[0].carried_lance_count, 0);
    assert!(game.position.pending_damage_order.is_none());
}
