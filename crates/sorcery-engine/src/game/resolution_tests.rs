use serde_json::{Value, json};

use super::{
    ActionDescriptor, CardId, CardInstance, CardSource, Cell, Game, IssuedAction, Phase, Region,
    Seat, SitePosition, SummonPlacement, UnitPosition, seat_index,
};
use crate::canonical::identity_hash;
use crate::synthetic::selfplay_manifest_with;

fn card_id(game: &Game, id: &str) -> CardId {
    CardId(
        u16::try_from(
            game.rules
                .cards
                .iter()
                .position(|card| card.id == id)
                .expect("fixture card"),
        )
        .expect("card index"),
    )
}

fn card(game: &Game, id: &str, owner: Seat, label: &str, source: CardSource) -> CardInstance {
    CardInstance {
        card_id: card_id(game, id),
        instance_id: identity_hash(&json!({ "resolution-test": label })).expect("card identity"),
        owner,
        realm_entry: 0,
        source,
    }
}

fn site(game: &Game, id: &str, owner: Seat, label: &str) -> SitePosition {
    let mut card = card(game, id, owner, label, CardSource::Atlas);
    card.enter_realm().expect("site realm entry");
    SitePosition {
        card,
        controller: owner,
        last_flight_turn: None,
        warded: false,
    }
}

fn deathrite(game: &Game, id: &str, label: &str, cell: &str) -> UnitPosition {
    let mut unit = SummonPlacement {
        card: card(game, id, Seat::North, label, CardSource::Spellbook),
        controller: Seat::North,
        lance_count: 0,
        location: Cell::parse(cell).expect("unit cell"),
        occupied_cells: None,
        region: Region::Surface,
        stealthed: false,
        warded: false,
    }
    .into_unit();
    unit.card.enter_realm().expect("unit realm entry");
    unit
}

fn manifest(seed: u32) -> String {
    selfplay_manifest_with(seed, |manifest| {
        manifest["cards"]["north-spell-1"] = json!({
            "cardType": "magic",
            "manaCost": 0,
            "summonTokenToAlliedMinionThenDrawSpell": "resolution-token",
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["resolution-token"] = json!({
            "attack": 0,
            "cardType": "minion",
            "defense": 1,
            "genesisDamageEachOtherUnitHere": 1,
            "manaCost": 0,
            "token": true,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["quiet-token"] = json!({
            "attack": 0, "cardType": "minion", "defense": 1, "manaCost": 0,
            "token": true, "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
        });
        manifest["cards"]["north-site-2"]["genesisPayOneManaToSummonToken"] = json!("quiet-token");
        for ordinal in 1..=2 {
            let card = &mut manifest["cards"][format!("south-spell-{ordinal}")];
            *card = json!({
                "attack": 0,
                "cardType": "minion",
                "defense": 1,
                "deathriteDrawSpells": true,
                "manaCost": 0,
                "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
            });
        }
    })
}

fn fixture(terminal: bool) -> (Game, String) {
    let game = Game::from_manifest_json(&manifest(if terminal { 902 } else { 901 }))
        .expect("resolution fixture");
    setup_fixture(game, terminal)
}

fn setup_fixture(mut game: Game, terminal: bool) -> (Game, String) {
    game.position.phase = Phase::Main;
    game.position.active_seat = Seat::North;
    game.position.decision_seat = Seat::North;
    game.position.players[seat_index(Seat::North)].domain_established = true;
    game.position.players[seat_index(Seat::North)].mana = 0;
    game.position.players[seat_index(Seat::North)].hand_spellbook = vec![card(
        &game,
        "north-spell-1",
        Seat::North,
        "draw-token-magic",
        CardSource::Spellbook,
    )];
    game.position.players[seat_index(Seat::North)].spellbook = (0..if terminal { 0 } else { 8 })
        .map(|ordinal| {
            card(
                &game,
                "north-spell-2",
                Seat::North,
                &format!("draw-card-{ordinal}"),
                CardSource::Spellbook,
            )
        })
        .collect();
    game.position.sites[Cell::parse("C3").unwrap().index()] =
        Some(site(&game, "north-site-1", Seat::North, "north-site-c3"));
    game.position.sites[Cell::parse("C4").unwrap().index()] =
        Some(site(&game, "north-site-2", Seat::North, "north-site-c4"));
    game.position.sites[Cell::parse("B3").unwrap().index()] =
        Some(site(&game, "south-site-1", Seat::South, "south-site-b3"));
    game.position.sites[Cell::parse("B4").unwrap().index()] =
        Some(site(&game, "south-site-2", Seat::South, "south-site-b4"));
    game.position.units = vec![
        deathrite(&game, "south-spell-1", "deathrite-a", "C3"),
        deathrite(&game, "south-spell-2", "deathrite-b", "C3"),
    ];
    let magic_id = game.position.players[seat_index(Seat::North)].hand_spellbook[0]
        .instance_id
        .to_string();
    (game, magic_id)
}

fn cast_action(game: &Game, magic_id: &str) -> IssuedAction {
    game.legal_actions()
        .expect("cast actions")
        .into_iter()
        .find(|action| {
            matches!(
                &action.descriptor,
                ActionDescriptor::CastMagic { card_instance_id, .. }
                    if card_instance_id.as_str() == magic_id
            )
        })
        .expect("token Magic action")
}

fn cast_and_choose_host(game: &mut Game, magic_id: &str) -> Vec<(String, Value)> {
    let action = cast_action(game, magic_id);
    let (mut events, _) = game
        .apply_action_recorded(&action)
        .expect("cast token Magic");
    assert_eq!(game.position.phase, Phase::AbilityChoice);
    assert_eq!(token_count(game), 0);
    let choice = game
        .legal_actions()
        .expect("host choices")
        .into_iter()
        .find(|action| matches!(action.descriptor, ActionDescriptor::ChooseAbility { .. }))
        .expect("ordinary allied minion choice");
    let (chosen_events, _) = game
        .apply_action_recorded(&choice)
        .expect("summon at chosen host");
    events.extend(chosen_events);
    events
}

fn order_action(game: &Game) -> IssuedAction {
    let actions = game.legal_actions().expect("Deathrite actions");
    assert_eq!(
        actions
            .iter()
            .filter(|action| matches!(action.descriptor, ActionDescriptor::OrderTriggers { .. }))
            .count(),
        2,
        "two Deathrites require player ordering"
    );
    actions
        .into_iter()
        .find(|action| matches!(action.descriptor, ActionDescriptor::OrderTriggers { .. }))
        .expect("Deathrite ordering action")
}

fn event_types(events: &[(String, Value)]) -> Vec<&str> {
    events.iter().map(|(kind, _)| kind.as_str()).collect()
}

fn token_count(game: &Game) -> usize {
    game.position
        .units
        .iter()
        .filter(|unit| unit.card.source == CardSource::Token)
        .count()
}

fn cemetery_count(game: &Game, magic_id: &str) -> usize {
    game.position.players[seat_index(Seat::North)]
        .cemetery
        .iter()
        .filter(|card| card.instance_id.as_str() == magic_id)
        .count()
}

#[test]
fn token_magic_holds_draw_and_completion_behind_ordered_genesis_deathrites() {
    let (mut game, magic_id) = fixture(false);
    let initial_events = cast_and_choose_host(&mut game, &magic_id);
    assert_eq!(game.position.phase, Phase::TriggerOrder);
    assert_eq!(token_count(&game), 1);
    assert_eq!(cemetery_count(&game, &magic_id), 0);
    assert!(
        !initial_events
            .iter()
            .any(|(kind, _)| kind == "magic-resolved")
    );
    let continuation = game.authoritative_state()["pendingDeathrites"]["continuation"].clone();
    assert_eq!(continuation["kind"], "sequence");
    assert_eq!(continuation["steps"][0]["kind"], "effect");
    assert_eq!(continuation["steps"][1]["kind"], "effect");
    assert_eq!(continuation["steps"].as_array().unwrap().len(), 2);

    let order = order_action(&game);
    let mut replay = game.clone();
    let (replayed_events, _) = replay
        .apply_action_recorded(&order)
        .expect("replay Deathrite continuation");
    let (events, _) = game
        .apply_action_recorded(&order)
        .expect("resume remaining token entries");
    assert_eq!(replayed_events, events);
    assert_eq!(replay.position, game.position);
    assert_eq!(replay.state_hash().unwrap(), game.state_hash().unwrap());
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(token_count(&game), 1);
    assert_eq!(cemetery_count(&game, &magic_id), 1);
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "magic-resolved")
            .count(),
        1
    );
    assert!(event_types(&events).contains(&"trigger-order-committed"));
    let parent_draws: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(_, (kind, event))| kind == "spell-drawn" && event["sourceInstanceId"] == magic_id)
        .collect();
    assert_eq!(
        parent_draws.len(),
        1,
        "the parent Magic still owes its own draw"
    );
    let completion = events
        .iter()
        .position(|(kind, _)| kind == "magic-resolved")
        .unwrap();
    assert!(parent_draws[0].0 < completion);
    let last_corpse = events
        .iter()
        .rposition(|(kind, _)| kind == "minion-died")
        .unwrap();
    assert!(last_corpse < parent_draws[0].0);
}

#[test]
fn terminal_token_genesis_skips_draw_and_later_resolution_but_retires_magic_once() {
    let (mut game, magic_id) = fixture(true);
    let initial_events = cast_and_choose_host(&mut game, &magic_id);
    assert_eq!(game.position.phase, Phase::TriggerOrder);
    assert_eq!(token_count(&game), 1);
    let continuation = game.authoritative_state()["pendingDeathrites"]["continuation"].clone();
    assert_eq!(continuation["kind"], "sequence");
    assert_eq!(continuation["steps"][0]["kind"], "effect");
    assert_eq!(continuation["steps"][1]["kind"], "effect");
    assert_eq!(continuation["steps"].as_array().unwrap().len(), 2);
    assert!(!initial_events.iter().any(|(kind, _)| kind == "spell-drawn"));

    let order = order_action(&game);
    let (events, _) = game
        .apply_action_recorded(&order)
        .expect("terminal Deathrite continuation");
    assert_eq!(game.position.phase, Phase::Terminal);
    assert!(!events.iter().any(|(kind, _)| kind == "spell-drawn"));
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "magic-resolved")
            .count(),
        1
    );
    assert_eq!(cemetery_count(&game, &magic_id), 1);
}

fn token_entry(game: &Game, token_id: &str, ordinal: usize) -> super::TokenEntryContinuation {
    let source = game.position.players[0].hand_spellbook[0]
        .instance_id
        .clone();
    super::TokenEntryContinuation {
        seat: Seat::North,
        token: game
            .create_token_unit(
                Seat::North,
                token_id,
                &source,
                super::Location {
                    cell: Cell::parse("C3").unwrap(),
                    region: Region::Surface,
                },
                ordinal,
                game.position.state_version,
            )
            .unwrap(),
        source_instance_id: source,
        mana_paid: 0,
    }
}

#[test]
fn simultaneous_entry_places_all_tokens_before_genesis_damage() {
    let (mut game, _) = fixture(false);
    game.position.units.clear();
    let entries = vec![
        token_entry(&game, "resolution-token", 0),
        token_entry(&game, "quiet-token", 1),
    ];
    let quiet_id = entries[1].token.card.instance_id.clone();
    let mut events = Vec::new();
    game.finish_token_entries(entries, &mut super::OutcomeLog::Record(&mut events))
        .unwrap();
    assert_eq!(
        token_count(&game),
        1,
        "arrival damage sees the other simultaneous entrant"
    );
    assert!(
        !game
            .position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == quiet_id)
    );
    assert_eq!(
        &event_types(&events)[..2],
        &["minion-summoned", "minion-summoned"]
    );
    assert!(
        events
            .iter()
            .any(|(kind, event)| kind == "minion-banished"
                && event["instanceId"] == quiet_id.as_str())
    );
}

#[test]
fn simultaneous_genesis_resumes_after_nested_deathrite_ordering() {
    let (mut game, _) = fixture(false);
    let entries = vec![
        token_entry(&game, "resolution-token", 0),
        token_entry(&game, "resolution-token", 1),
    ];
    let survivor = entries[0].token.card.instance_id.clone();
    let mut events = Vec::new();
    game.finish_token_entries(entries, &mut super::OutcomeLog::Record(&mut events))
        .expect("simultaneous entry");
    assert_eq!(game.position.phase, Phase::TriggerOrder);
    assert!(game.position.pending_trigger_order.is_some());
    assert_eq!(event_types(&events), ["minion-summoned", "minion-summoned"]);
    let first = game
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(&action.descriptor, ActionDescriptor::OrderTriggers { source_instance_id }
            if *source_instance_id == survivor)
        })
        .unwrap();
    game.apply_action_recorded(&first)
        .expect("Genesis interrupts with Deathrites");
    assert_eq!(game.position.phase, Phase::TriggerOrder);
    assert!(game.position.pending_deathrites.is_some());
    let deathrite_order = game.legal_actions().unwrap().into_iter().next().unwrap();
    game.apply_action_recorded(&deathrite_order)
        .expect("finish nested death chain");
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(token_count(&game), 1);
    assert!(
        game.position
            .units
            .iter()
            .any(|unit| unit.card.instance_id == survivor)
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one entry sequence across the four realm regions"
)]
fn ordinary_token_choice_enters_regions_then_settles_before_genesis_and_parent_draw() {
    for (region, survives) in [
        (Region::Surface, true),
        (Region::Underwater, true),
        (Region::Underground, false),
        (Region::Void, false),
    ] {
        let encoded = selfplay_manifest_with(908, |manifest| {
            manifest["cards"]["north-spell-1"] = json!({
                "cardType": "magic", "manaCost": 0,
                "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
                "effectProgram": {"effects": [
                    {"op": "choose-unit", "kind": "minion", "relation": "anywhere", "alliedOnly": true},
                    {"op": "summon-token", "token": "region-token", "count": 2, "destination": "chosen"},
                    {"op": "draw", "zone": "spellbook", "count": 1}
                ]}
            });
            manifest["cards"]["region-token"] = json!({
                "cardType": "minion", "attack": 0, "defense": 0, "manaCost": 0,
                "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
                "token": true, "submerge": true, "deathriteDrawSite": true,
                "genesisProgram": {"effects": [{"op": "draw", "zone": "spellbook", "count": 1}]}
            });
            for id in ["south-spell-1", "south-spell-2"] {
                manifest["cards"][id]["submerge"] = json!(true);
                manifest["cards"][id]["burrowing"] = json!(true);
                manifest["cards"][id]["voidwalk"] = json!(true);
            }
            if region == Region::Underwater {
                manifest["cards"]["north-site-1"]["elements"] = json!(["water"]);
            }
        });
        let (mut game, magic_id) =
            setup_fixture(Game::from_manifest_json(&encoded).unwrap(), false);
        game.position.units.truncate(1);
        game.position.units[0].region = region;
        game.position.units[0].warded = true;
        game.position.units[0].stealthed = true;
        if region == Region::Void {
            game.position.sites[Cell::parse("C3").unwrap().index()] = None;
        }
        let before_draw = game.position.players[0].hand_spellbook.len();
        let mut events = cast_and_choose_host(&mut game, &magic_id);
        assert!(
            game.position.units[0].warded,
            "ordinary choice consumes no Ward"
        );
        assert!(
            game.position.units[0].stealthed,
            "ordinary choice preserves Stealth"
        );
        let mut branch = game.clone();
        while game.position.phase == Phase::TriggerOrder {
            let action = game
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| matches!(action.descriptor, ActionDescriptor::OrderTriggers { .. }))
                .unwrap();
            let (actual, _) = game.apply_action_recorded(&action).unwrap();
            let (replayed, _) = branch.apply_action_recorded(&action).unwrap();
            assert_eq!(actual, replayed);
            assert_eq!(game.position, branch.position);
            events.extend(actual);
        }
        assert_eq!(game.position.phase, Phase::Main);
        assert_eq!(
            token_count(&game),
            if survives { 2 } else { 0 },
            "{region:?}"
        );
        // Casting removes the Magic; its draw restores it. Only surviving token Genesis adds two.
        assert_eq!(
            game.position.players[0].hand_spellbook.len(),
            before_draw + if survives { 2 } else { 0 }
        );
        let summons: Vec<_> = events
            .iter()
            .enumerate()
            .filter(|(_, (kind, value))| kind == "minion-summoned" && value["token"] == true)
            .collect();
        assert_eq!(summons.len(), 2);
        let parent_draw = events
            .iter()
            .position(|(kind, value)| {
                kind == "spell-drawn" && value["sourceInstanceId"] == magic_id
            })
            .unwrap();
        assert!(summons[1].0 < parent_draw);
        if !survives {
            let removal = if region == Region::Void {
                "minion-banished"
            } else {
                "minion-died"
            };
            assert_eq!(events.iter().filter(|(kind, _)| kind == removal).count(), 2);
            assert!(
                events
                    .iter()
                    .rposition(|(kind, _)| kind == removal)
                    .unwrap()
                    < parent_draw
            );
        }
        assert_eq!(cemetery_count(&game, &magic_id), 1);
    }
}

#[test]
fn authored_genesis_summons_nested_groups_with_distinct_identities() {
    let encoded = selfplay_manifest_with(909, |manifest| {
        manifest["cards"]["north-spell-1"] = json!({
            "cardType": "minion", "attack": 1, "defense": 1, "manaCost": 0,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
            "genesisProgram": {"effects": [
                {"op": "summon-token", "token": "branch-token", "count": 2, "destination": "source"},
                {"op": "summon-token", "token": "branch-token", "count": 1, "destination": "source"}
            ]}
        });
        manifest["cards"]["leaf-token"] = json!({
            "cardType": "minion", "attack": 1, "defense": 1, "manaCost": 0, "token": true,
            "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0}
        });
        manifest["cards"]["branch-token"] = manifest["cards"]["leaf-token"].clone();
        manifest["cards"]["branch-token"]["genesisProgram"] = json!({"effects": [
            {"op": "summon-token", "token": "leaf-token", "count": 2, "destination": "source"}
        ]});
    });
    let (mut game, _) = setup_fixture(Game::from_manifest_json(&encoded).unwrap(), false);
    game.position.units.clear();
    let action = game
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| matches!(action.descriptor, ActionDescriptor::SummonMinion { .. }))
        .unwrap();
    let (mut events, _) = game.apply_action_recorded(&action).unwrap();
    let mut branch = game.clone();
    while game.position.phase == Phase::TriggerOrder {
        let action = game.legal_actions().unwrap().into_iter().next().unwrap();
        let (actual, _) = game.apply_action_recorded(&action).unwrap();
        let (replayed, _) = branch.apply_action_recorded(&action).unwrap();
        assert_eq!(actual, replayed);
        events.extend(actual);
    }
    assert_eq!(game.position.phase, Phase::Main);
    assert_eq!(game.position, branch.position);
    assert_eq!(token_count(&game), 9);
    let ids: std::collections::BTreeSet<_> = game
        .position
        .units
        .iter()
        .map(|unit| &unit.card.instance_id)
        .collect();
    assert_eq!(
        ids.len(),
        10,
        "repeated groups and nested token sources have distinct identities"
    );
    let first_leaf = events
        .iter()
        .position(|(kind, value)| kind == "minion-summoned" && value["cardId"] == "leaf-token")
        .unwrap();
    assert_eq!(
        events[..first_leaf]
            .iter()
            .filter(|(kind, value)| kind == "minion-summoned" && value["cardId"] == "branch-token")
            .count(),
        2,
        "the first group enters completely before either Genesis resolves"
    );
}

#[test]
fn token_destinations_respect_declared_protection_and_entry_barriers_then_draw() {
    for destination in ["target", "location"] {
        for (warded, blocked) in [(false, false), (true, false), (false, true)] {
            let encoded = selfplay_manifest_with(910, |manifest| {
                manifest["cards"]["north-spell-1"] = json!({
                    "cardType": "magic", "manaCost": 0,
                    "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0},
                    "effectProgram": {
                        "selection": {"kind": if destination == "target" { "unit" } else { "location" }, "relation": "anywhere"},
                        "effects": [
                            {"op": "summon-token", "token": "bound-token", "count": 1, "destination": destination},
                            {"op": "draw", "zone": "spellbook", "count": 1}
                        ]
                    }
                });
                manifest["cards"]["bound-token"] = json!({
                    "cardType": "minion", "attack": 3, "defense": 3, "manaCost": 0, "token": true,
                    "thresholds": {"air": 0, "earth": 0, "fire": 0, "water": 0}
                });
                if blocked {
                    manifest["cards"]["north-site-1"]["preventsUnitsWithPowerAtLeastFromEntering"] =
                        json!(3);
                }
            });
            let (mut game, magic_id) =
                setup_fixture(Game::from_manifest_json(&encoded).unwrap(), false);
            game.position.units.truncate(1);
            game.position.units[0].controller = Seat::South;
            game.position.units[0].warded = warded;
            let host_id = game.position.units[0].card.instance_id.clone();
            let cell = Cell::parse("C3").unwrap();
            let site = game.position.sites[cell.index()].as_mut().unwrap();
            site.controller = Seat::South;
            site.warded = warded;
            let hand_before = game.position.players[0].hand_spellbook.len();
            let action = game
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| match &action.descriptor {
                    ActionDescriptor::CastMagic {
                        target,
                        target_location,
                        ..
                    } => {
                        if destination == "target" {
                            target
                                .as_ref()
                                .is_some_and(|target| target.instance_id() == &host_id)
                        } else {
                            target_location.is_some_and(|location| location.cell == cell)
                        }
                    }
                    _ => false,
                })
                .expect("declared destination remains legal even when entry will fail");
            let mut branch = game.clone();
            let (events, _) = game.apply_action_recorded(&action).unwrap();
            let (replayed, _) = branch.apply_action_recorded(&action).unwrap();
            assert_eq!(events, replayed);
            assert_eq!(game.position, branch.position);
            assert_eq!(token_count(&game), usize::from(!warded && !blocked));
            assert_eq!(
                events
                    .iter()
                    .filter(|(kind, _)| kind == "ward-broken")
                    .count(),
                usize::from(warded)
            );
            assert_eq!(
                game.position.players[0].hand_spellbook.len(),
                hand_before,
                "independent draw continues"
            );
            assert_eq!(cemetery_count(&game, &magic_id), 1);
        }
    }
}
