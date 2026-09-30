//! Site-entry effects share site identity across every minion placement and relocation path.

use super::*;
use crate::canonical::identity_hash;
use crate::synthetic::selfplay_manifest_with;

fn id(label: &str) -> IdentityHash {
    identity_hash(&json!({"site-entry-test": label})).expect("test identity")
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

fn fixture_with_site_entry_usage(usage: Option<&str>) -> Game {
    let manifest = selfplay_manifest_with(9321, |manifest| {
        manifest["cards"]["north-site-1"]["siteEntryEffect"] =
            json!("grantStealthToEnteringMinion");
        if let Some(usage) = usage {
            manifest["cards"]["north-site-1"]["siteEntryUsage"] = json!(usage);
        }
        manifest["cards"]["north-site-1"]["elements"] = json!(["air"]);
        manifest["cards"]["north-site-1"]["flyToNearbyVoidOncePerTurnAtAirThreshold"] = json!(3);
        manifest["cards"]["north-site-2"]["siteEntryEffect"] =
            json!("grantStealthToEnteringMinion");
        manifest["cards"]["north-spell-49"] = json!({
            "attack": 1,
            "cardType": "minion",
            "defense": 1,
            "manaCost": null,
            "thresholds": {"air":0,"earth":0,"fire":0,"water":0},
            "token": true,
        });
        manifest["cards"]["north-spell-50"] = json!({
            "cardType":"magic",
            "effectProgram":{"effects":[{
                "op":"summon-token", "token":"north-spell-49", "count":1,
                "destination":"source"
            }]},
            "manaCost":0,
            "thresholds":{"air":0,"earth":0,"fire":0,"water":0}
        });
        manifest["decks"]["north"]["spellbook"]
            .as_array_mut()
            .expect("spellbook")
            .retain(|card| card != "north-spell-49");
    });
    let mut game = Game::from_manifest_json(&manifest).expect("entry fixture");
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    game
}

fn fixture() -> Game {
    fixture_with_site_entry_usage(None)
}

fn site(game: &Game, label: &str) -> SitePosition {
    site_from_card(game, "north-site-1", label)
}

fn site_from_card(game: &Game, name: &str, label: &str) -> SitePosition {
    let mut card = CardInstance {
        card_id: card_id(game, name),
        instance_id: id(label),
        owner: Seat::North,
        realm_entry: 0,
        source: CardSource::Atlas,
    };
    card.enter_realm().expect("site realm entry");
    SitePosition {
        card,
        controller: Seat::North,
        last_flight_turn: None,
        warded: false,
    }
}

fn minion(
    game: &Game,
    card_name: &str,
    label: &str,
    cell: &str,
    occupied_cells: Option<SquareArea>,
) -> UnitPosition {
    let mut card = CardInstance {
        card_id: card_id(game, card_name),
        instance_id: id(label),
        owner: Seat::North,
        realm_entry: 0,
        source: if card_name == "north-spell-49" {
            CardSource::Token
        } else {
            CardSource::Spellbook
        },
    };
    card.enter_realm().expect("minion realm entry");
    SummonPlacement {
        card,
        controller: Seat::North,
        lance_count: 0,
        location: Cell::parse(cell).expect("minion cell"),
        occupied_cells,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit()
}

fn event_types(events: &[(String, Value)]) -> Vec<&str> {
    events.iter().map(|(kind, _)| kind.as_str()).collect()
}

#[test]
fn site_entry_effect_is_closed_and_fail_closed() {
    let valid = json!({
        "cardType":"site", "elements":["earth"],
        "siteEntryEffect":"grantStealthToEnteringMinion"
    });
    let CardFacts::Site(facts) = crate::facts::parse_card_definition("site", &valid).unwrap()
    else {
        panic!("site facts")
    };
    assert_eq!(
        facts.site_entry_effect,
        Some(SiteEntryEffect::GrantStealthToEnteringMinion)
    );
    assert_eq!(facts.site_entry_usage, SiteEntryUsage::EveryEntry);
    let first = json!({
        "cardType":"site", "elements":["air"],
        "siteEntryEffect":"grantStealthToEnteringMinion", "siteEntryUsage":"firstEntry"
    });
    let CardFacts::Site(facts) = crate::facts::parse_card_definition("site", &first).unwrap()
    else {
        panic!("first-entry site facts")
    };
    assert_eq!(facts.site_entry_usage, SiteEntryUsage::FirstEntry);
    for value in [json!(true), json!("killEnteringMinion"), json!({})] {
        let invalid = json!({
            "cardType":"site", "elements":["earth"], "siteEntryEffect":value
        });
        assert!(crate::facts::parse_card_definition("site", &invalid).is_err());
    }
    for invalid in [
        json!({"cardType":"site", "elements":["air"], "siteEntryUsage":"firstEntry"}),
        json!({"cardType":"site", "elements":["air"], "siteEntryEffect":"grantStealthToEnteringMinion", "siteEntryUsage":"perTurn"}),
    ] {
        assert!(crate::facts::parse_card_definition("site", &invalid).is_err());
    }
}

#[test]
fn engine_issued_basic_movement_triggers_entry_and_replays_from_clone() {
    let mut game = fixture();
    let destination = Cell::parse("C3").unwrap();
    game.position.sites[destination.index()] = Some(site(&game, "destination"));
    let entrant = minion(&game, "north-spell-1", "entrant", "C2", None);
    let entrant_id = entrant.card.instance_id.clone();
    game.position.units.push(entrant);
    let path = [
        Location {
            cell: Cell::parse("C2").unwrap(),
            region: Region::Surface,
        },
        Location {
            cell: destination,
            region: Region::Surface,
        },
    ];
    game.begin_basic_movement(
        Seat::North,
        &path,
        &entrant_id,
        BasicMovementPurpose::MoveAndAttack,
        &mut OutcomeLog::Ignore,
    )
    .unwrap();
    let action = game
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(
                action.descriptor,
                ActionDescriptor::ContinueBasicMovement { .. }
            )
        })
        .expect("engine-issued movement continuation");
    let mut replay = game.clone();
    let result = game.apply_action_recorded(&action).unwrap();
    assert_eq!(result, replay.apply_action_recorded(&action).unwrap());
    assert_eq!(game.authoritative_state(), replay.authoritative_state());
    assert_eq!(
        event_types(&result.0),
        [
            "site-entry-triggered",
            "minion-stealthed",
            "basic-movement-continued"
        ]
    );
}

#[test]
fn first_entry_usage_is_per_site_incarnation_and_survives_turns_and_replay() {
    let mut game = fixture_with_site_entry_usage(Some("firstEntry"));
    let first_site = Cell::parse("C3").unwrap();
    game.position.sites[first_site.index()] = Some(site(&game, "first-site"));
    let entrant = minion(&game, "north-spell-1", "first-entrant", "C2", None);
    let entrant_id = entrant.card.instance_id.clone();
    game.position.units.push(entrant);

    let mut first_events = Vec::new();
    let mut replay = game.clone();
    game.move_minion_to(
        &entrant_id,
        Location {
            cell: first_site,
            region: Region::Surface,
        },
        None,
        &mut OutcomeLog::Record(&mut first_events),
    )
    .unwrap();
    let mut replay_events = Vec::new();
    replay
        .move_minion_to(
            &entrant_id,
            Location {
                cell: first_site,
                region: Region::Surface,
            },
            None,
            &mut OutcomeLog::Record(&mut replay_events),
        )
        .unwrap();
    assert_eq!(first_events, replay_events);
    assert_eq!(game.authoritative_state(), replay.authoritative_state());
    assert!(game.position.units[0].stealthed);
    assert_eq!(game.position.site_entry_uses.len(), 1);
    assert_eq!(
        game.authoritative_state()["siteEntryUses"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    game.position.units[0].stealthed = false;
    let outside = Cell::parse("C2").unwrap();
    game.move_minion_to(
        &entrant_id,
        Location {
            cell: outside,
            region: Region::Surface,
        },
        None,
        &mut OutcomeLog::Ignore,
    )
    .unwrap();
    game.position.turn_number += 1;
    let mut repeat_events = Vec::new();
    game.move_minion_to(
        &entrant_id,
        Location {
            cell: first_site,
            region: Region::Surface,
        },
        None,
        &mut OutcomeLog::Record(&mut repeat_events),
    )
    .unwrap();
    assert!(repeat_events.is_empty());
    assert!(!game.position.units[0].stealthed);

    let second_site = Cell::parse("D3").unwrap();
    game.position.sites[second_site.index()] = Some(site(&game, "second-site"));
    let mut second_events = Vec::new();
    game.move_minion_to(
        &entrant_id,
        Location {
            cell: second_site,
            region: Region::Surface,
        },
        None,
        &mut OutcomeLog::Record(&mut second_events),
    )
    .unwrap();
    assert!(game.position.units[0].stealthed);
    assert_eq!(
        second_events
            .iter()
            .filter(|(kind, _)| kind == "site-entry-triggered")
            .count(),
        1
    );
    assert_eq!(game.position.site_entry_uses.len(), 2);
}

#[test]
fn site_identity_ignores_layer_changes_and_oversized_overlap() {
    let mut game = fixture();
    for (cell, label) in [("C2", "old"), ("C3", "overlap"), ("C4", "new")] {
        let cell = Cell::parse(cell).unwrap();
        game.position.sites[cell.index()] = Some(site(&game, label));
    }
    let old = ["B2", "C2", "B3", "C3"].map(|cell| Cell::parse(cell).unwrap());
    let new = ["B3", "C3", "B4", "C4"].map(|cell| Cell::parse(cell).unwrap());
    let entrant = minion(&game, "north-spell-1", "large", "B2", Some(old));
    let entrant_id = entrant.card.instance_id.clone();
    game.position.units.push(entrant);
    let mut events = Vec::new();
    game.move_minion_to(
        &entrant_id,
        Location {
            cell: new[0],
            region: Region::Surface,
        },
        Some(new),
        &mut OutcomeLog::Record(&mut events),
    )
    .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "site-entry-triggered")
            .count(),
        1
    );
    game.position.units[0].stealthed = false;
    events.clear();
    game.move_minion_to(
        &entrant_id,
        Location {
            cell: new[0],
            region: Region::Underground,
        },
        Some(new),
        &mut OutcomeLog::Record(&mut events),
    )
    .unwrap();
    assert!(events.is_empty());
    assert!(!game.position.units[0].stealthed);
}

#[test]
fn paid_and_simultaneous_token_summons_apply_after_each_successful_placement() {
    let mut game = fixture();
    let cell = Cell::parse("C3").unwrap();
    game.position.sites[cell.index()] = Some(site(&game, "summon-site"));
    let paid = minion(&game, "north-spell-1", "paid", "C3", None);
    let paid_id = paid.card.instance_id.clone();
    let caster = UnitTarget::Avatar {
        instance_id: game.position.players[0].avatar.card.instance_id.clone(),
        seat: Seat::North,
    };
    let mut paid_events = Vec::new();
    game.finish_summon(
        PaidSummonContinuation {
            caster,
            mana_paid: 0,
            sacrificed_minion_instance_ids: Vec::new(),
            unit: paid,
        },
        None,
        &mut OutcomeLog::Record(&mut paid_events),
    )
    .unwrap();
    assert!(
        game.position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == paid_id)
            .unwrap()
            .stealthed
    );

    let tokens = ["token-a", "token-b"]
        .into_iter()
        .map(|label| {
            let token = minion(&game, "north-spell-49", label, "C3", None);
            let source_instance_id = token.card.instance_id.clone();
            TokenEntryContinuation {
                seat: Seat::North,
                token,
                source_instance_id,
                mana_paid: 0,
            }
        })
        .collect();
    let mut token_events = Vec::new();
    game.finish_token_entries(tokens, &mut OutcomeLog::Record(&mut token_events))
        .unwrap();
    assert_eq!(
        token_events
            .iter()
            .filter(|(kind, _)| kind == "site-entry-triggered")
            .count(),
        2
    );
}

#[test]
fn first_entry_provider_captures_all_members_of_one_simultaneous_token_group() {
    let mut game = fixture_with_site_entry_usage(Some("firstEntry"));
    let cell = Cell::parse("C3").unwrap();
    game.position.sites[cell.index()] = Some(site(&game, "first-entry-group-site"));
    let provider =
        RealmReference::from_card(&game.position.sites[cell.index()].as_ref().unwrap().card);
    let tokens = ["first-group-a", "first-group-b"]
        .into_iter()
        .map(|label| {
            let token = minion(&game, "north-spell-49", label, "C3", None);
            let source_instance_id = token.card.instance_id.clone();
            TokenEntryContinuation {
                seat: Seat::North,
                token,
                source_instance_id,
                mana_paid: 0,
            }
        })
        .collect::<Vec<_>>();
    let entrants = tokens
        .iter()
        .map(|entry| entry.token.card.instance_id.clone())
        .collect::<Vec<_>>();

    let mut events = Vec::new();
    game.finish_token_entries(tokens, &mut OutcomeLog::Record(&mut events))
        .unwrap();

    assert!(entrants.iter().all(|instance_id| {
        game.position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == *instance_id)
            .is_some_and(|unit| unit.stealthed)
    }));
    assert_eq!(game.position.site_entry_uses.len(), 1);
    assert_eq!(game.position.site_entry_uses[0], provider);
    let occurrences = events
        .iter()
        .filter(|(kind, _)| kind == "site-entry-triggered")
        .map(|(_, payload)| {
            (
                payload["sourceInstanceId"].as_str().unwrap().to_owned(),
                payload["targetInstanceId"].as_str().unwrap().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(occurrences.len(), 2);
    assert!(occurrences.iter().all(|(source, entrant)| {
        source == provider.instance_id().as_str()
            && entrants
                .iter()
                .any(|instance_id| instance_id.as_str() == entrant)
    }));
}

#[test]
fn simultaneous_entry_group_has_recorded_and_ignored_state_parity() {
    let mut ignored = fixture_with_site_entry_usage(Some("firstEntry"));
    let cell = Cell::parse("C3").unwrap();
    ignored.position.sites[cell.index()] = Some(site(&ignored, "parity-site"));
    let mut recorded = ignored.clone();
    let entries = |game: &Game| {
        ["parity-a", "parity-b"]
            .into_iter()
            .map(|label| {
                let token = minion(game, "north-spell-49", label, "C3", None);
                let source_instance_id = token.card.instance_id.clone();
                TokenEntryContinuation {
                    seat: Seat::North,
                    token,
                    source_instance_id,
                    mana_paid: 0,
                }
            })
            .collect::<Vec<_>>()
    };

    let mut events = Vec::new();
    ignored
        .finish_token_entries(entries(&ignored), &mut OutcomeLog::Ignore)
        .unwrap();
    recorded
        .finish_token_entries(entries(&recorded), &mut OutcomeLog::Record(&mut events))
        .unwrap();

    assert_eq!(ignored.position, recorded.position);
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "site-entry-triggered")
            .count(),
        2
    );
}

#[test]
fn first_entry_group_is_seat_and_enumeration_independent_and_skips_used_providers() {
    for seat in [Seat::North, Seat::South] {
        for preconsumed in [false, true] {
            for reversed in [false, true] {
                let mut game = fixture_with_site_entry_usage(Some("firstEntry"));
                let cell = Cell::parse("C3").unwrap();
                game.position.sites[cell.index()] = Some(site(&game, "group-seat-site"));
                let provider = RealmReference::from_card(
                    &game.position.sites[cell.index()].as_ref().unwrap().card,
                );
                if preconsumed {
                    game.position.site_entry_uses.push(provider.clone());
                }
                let labels = if reversed {
                    ["token-a", "token-b"]
                } else {
                    ["token-b", "token-a"]
                };
                let tokens = labels
                    .into_iter()
                    .map(|label| {
                        let mut token = minion(&game, "north-spell-49", label, "C3", None);
                        token.controller = seat;
                        token.card.owner = seat;
                        let source_instance_id = token.card.instance_id.clone();
                        TokenEntryContinuation {
                            seat,
                            token,
                            source_instance_id,
                            mana_paid: 0,
                        }
                    })
                    .collect::<Vec<_>>();
                let entrant_ids = tokens
                    .iter()
                    .map(|entry| entry.token.card.instance_id.clone())
                    .collect::<Vec<_>>();

                let mut events = Vec::new();
                game.finish_token_entries(tokens, &mut OutcomeLog::Record(&mut events))
                    .unwrap();

                let stealthed = entrant_ids
                    .iter()
                    .filter(|id| {
                        game.position
                            .units
                            .iter()
                            .find(|unit| unit.card.instance_id == **id)
                            .is_some_and(|unit| unit.stealthed)
                    })
                    .count();
                let expected = if preconsumed { 0 } else { 2 };
                assert_eq!(stealthed, expected, "seat={seat:?} reversed={reversed}");
                assert_eq!(
                    events
                        .iter()
                        .filter(|(kind, _)| kind == "site-entry-triggered")
                        .count(),
                    expected
                );
                assert_eq!(game.position.site_entry_uses, [provider]);
            }
        }
    }
}

#[test]
fn overlapping_footprint_collects_fresh_used_and_repeatable_providers_once() {
    let mut game = fixture_with_site_entry_usage(Some("firstEntry"));
    let footprint = ["B2", "C2", "B3", "C3"].map(|cell| Cell::parse(cell).unwrap());
    let fresh_cell = Cell::parse("B2").unwrap();
    let used_cell = Cell::parse("C2").unwrap();
    let repeatable_cell = Cell::parse("B3").unwrap();
    for (cell, card, label) in [
        (fresh_cell, "north-site-1", "fresh-provider"),
        (used_cell, "north-site-1", "used-provider"),
        (repeatable_cell, "north-site-2", "repeatable-provider"),
    ] {
        game.position.sites[cell.index()] = Some(site_from_card(&game, card, label));
    }
    let used_provider = RealmReference::from_card(
        &game.position.sites[used_cell.index()]
            .as_ref()
            .unwrap()
            .card,
    );
    game.position.site_entry_uses.push(used_provider.clone());
    let entrant = minion(
        &game,
        "north-spell-49",
        "footprint-entrant",
        "B2",
        Some(footprint),
    );
    let entrant_id = entrant.card.instance_id.clone();
    let source_instance_id = entrant_id.clone();

    let mut events = Vec::new();
    game.finish_token_entries(
        vec![TokenEntryContinuation {
            seat: Seat::North,
            token: entrant,
            source_instance_id,
            mana_paid: 0,
        }],
        &mut OutcomeLog::Record(&mut events),
    )
    .unwrap();

    let triggers = events
        .iter()
        .filter(|(kind, _)| kind == "site-entry-triggered")
        .map(|(_, payload)| payload["sourceInstanceId"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let fresh_provider = RealmReference::from_card(
        &game.position.sites[fresh_cell.index()]
            .as_ref()
            .unwrap()
            .card,
    );
    let repeatable_provider = RealmReference::from_card(
        &game.position.sites[repeatable_cell.index()]
            .as_ref()
            .unwrap()
            .card,
    );
    assert_eq!(triggers.len(), 2);
    assert!(triggers.contains(&fresh_provider.instance_id().to_string()));
    assert!(triggers.contains(&repeatable_provider.instance_id().to_string()));
    assert!(!triggers.contains(&used_provider.instance_id().to_string()));
    assert_eq!(
        events
            .iter()
            .filter(|(kind, payload)| {
                kind == "minion-stealthed" && payload["instanceId"] == entrant_id.as_str()
            })
            .count(),
        1
    );
}

#[test]
fn placing_or_flying_a_site_under_occupants_is_not_minion_entry() {
    let mut game = fixture();
    let cell = Cell::parse("C3").unwrap();
    let occupant = minion(&game, "north-spell-1", "occupant", "C3", None);
    game.position.units.push(occupant);
    game.position.sites[cell.index()] = Some(site(&game, "placed-under"));
    assert!(!game.position.units[0].stealthed);

    let destination = Cell::parse("D3").unwrap();
    for (cell, label) in [("A1", "air-two"), ("A2", "air-three")] {
        let support = Cell::parse(cell).unwrap();
        game.position.sites[support.index()] = Some(site(&game, label));
    }
    let source_id = game.position.sites[cell.index()]
        .as_ref()
        .unwrap()
        .card
        .instance_id
        .clone();
    let mut events = Vec::new();
    game.apply_site_flight_action(
        Seat::North,
        &source_id,
        destination,
        &mut OutcomeLog::Record(&mut events),
    )
    .unwrap();
    assert!(!game.position.units[0].stealthed);
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "site-entry-triggered")
    );
}
