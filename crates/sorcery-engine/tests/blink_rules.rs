use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar() -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    })
}

fn site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn manifest(deathrite: bool) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "blink-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-blink-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-blink": {
                "cardType": "magic",
                "manaCost": 0,
                "teleportNearbyAllyThenDrawCard": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-fragile": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-sparkmage": {
                "attack": 1,
                "cardType": "minion",
                "defense": 4,
                "manaCost": 0,
                "otherNearbyAlliesPowerBonus": 1,
                "tapToShootProjectileDamage": 2,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                // Two copies of each part guarantee a complete hand after two Spellbook draws.
                "spellbook": [
                    "north-blink",
                    "north-blink",
                    "north-fragile",
                    "north-fragile",
                    "north-sparkmage",
                    "north-sparkmage",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 410,
    });
    if deathrite {
        value["cards"]["north-fragile"]["deathriteDamageEachUnitHere"] = json!(1);
    }
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

/// A second wounded ally so one departing aura owes two simultaneous Deathrites.
fn ordered_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "blink-ordered-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-blink-ordered-rules-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-blink": {
                "cardType": "magic",
                "manaCost": 0,
                "teleportNearbyAllyThenDrawCard": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-fragile": {
                "attack": 1,
                "cardType": "minion",
                "deathriteDamageEachUnitHere": 1,
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-sparkmage": {
                "attack": 1,
                "cardType": "minion",
                "defense": 4,
                "manaCost": 0,
                "otherNearbyAlliesPowerBonus": 1,
                "tapToShootProjectileDamage": 2,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-site": site(),
            "north-storm": {
                "cardType": "magic",
                "damageEachAbovegroundMinion": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-avatar": avatar(),
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                // Six draws empty the Spellbook, so every part reaches hand whatever the shuffle.
                "spellbook": [
                    "north-blink",
                    "north-blink",
                    "north-fragile",
                    "north-fragile",
                    "north-sparkmage",
                    "north-storm",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 411,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step")
    else {
        panic!("engine-issued action must be accepted");
    };
    (descriptor, receipt)
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn summon(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("minion identity")
        .to_owned()
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn hand_in(session: &Session, zone: &str, card_id: &str) -> String {
    state(session)["players"]["north"]["hand"][zone]
        .as_array()
        .expect("North hand zone")
        .iter()
        .find(|card| card["cardId"] == card_id)
        .unwrap_or_else(|| panic!("{card_id} should be in hand"))["instanceId"]
        .as_str()
        .expect("hand identity")
        .to_owned()
}

fn hand_count(session: &Session, zone: &str) -> usize {
    state(session)["players"]["north"]["hand"][zone]
        .as_array()
        .expect("North hand zone")
        .len()
}

fn realm_unit(session: &Session, instance_id: &str) -> Option<Value> {
    state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .cloned()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

/// Wounds a fragile ally that only its neighbour's aura keeps standing.
struct Blink {
    fragile: String,
    session: Session,
    sparkmage: String,
    spell: String,
}

fn blink_checkpoint(drain_spellbook: bool) -> Blink {
    blink_scenario(drain_spellbook, false)
}

fn blink_scenario(drain_spellbook: bool, deathrite: bool) -> Blink {
    let mut session = Session::new(&manifest(deathrite)).expect("valid Blink scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    end_turn(&mut session);

    draw(&mut session, "atlas");
    play_site(&mut session, "C1");
    end_turn(&mut session);

    draw(&mut session, "spellbook");
    play_site(&mut session, "D4");
    end_turn(&mut session);

    draw(&mut session, "atlas");
    end_turn(&mut session);

    draw(&mut session, "spellbook");
    play_site(&mut session, "E4");
    let fragile = summon(&mut session, "north-fragile", "C4");
    let sparkmage = summon(&mut session, "north-sparkmage", "D4");
    end_turn(&mut session);

    draw(&mut session, "atlas");
    end_turn(&mut session);

    draw(
        &mut session,
        if drain_spellbook {
            "spellbook"
        } else {
            "atlas"
        },
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "shoot-damage-projectile"
            && descriptor["shooterInstanceId"] == sparkmage.as_str()
            && descriptor["hit"]["instanceId"] == fragile.as_str()
    });
    let spell = hand_in(&session, "spellbook", "north-blink");
    Blink {
        fragile,
        session,
        sparkmage,
        spell,
    }
}

fn cast_blink(session: &mut Session, spell: &str, ally: &str, cell: &str, zone: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell
            && descriptor["ally"]["instanceId"] == ally
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["drawZone"] == zone
    });
    receipt
}

fn blink_casts(checkpoint: &Blink) -> Vec<Value> {
    checkpoint
        .session
        .legal_actions()
        .expect("Blink actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == checkpoint.spell.as_str()
        })
        .map(|action| action.descriptor)
        .collect()
}

fn destinations(casts: &[Value], ally: &str) -> Vec<String> {
    let mut cells: Vec<String> = casts
        .iter()
        .filter(|descriptor| descriptor["ally"]["instanceId"] == ally)
        .map(|descriptor| {
            descriptor["targetLocation"]["cell"]
                .as_str()
                .expect("Blink destination")
                .to_owned()
        })
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

#[test]
fn rule_catalog_0040_blink_should_offer_every_ally_a_nearby_location_and_either_deck() {
    let checkpoint = blink_checkpoint(false);
    assert_eq!(
        realm_unit(&checkpoint.session, &checkpoint.fragile).expect("wounded ally")["damage"],
        2,
        "the aura must be all that keeps the wounded ally standing"
    );

    let casts = blink_casts(&checkpoint);
    let mut zones: Vec<&str> = casts
        .iter()
        .map(|descriptor| descriptor["drawZone"].as_str().expect("Blink draw zone"))
        .collect();
    zones.sort_unstable();
    zones.dedup();
    assert_eq!(zones, ["atlas", "spellbook"]);
    assert!(
        casts
            .iter()
            .all(|descriptor| descriptor["ally"]["seat"] == "north"
                && descriptor["target"].is_null()),
        "Blink relocates an ally instead of targeting an enemy"
    );
    assert_eq!(
        destinations(&casts, &checkpoint.sparkmage),
        ["C4", "D4", "E4"],
        "only nearby realm locations may receive the ally"
    );
    assert_eq!(destinations(&casts, &checkpoint.fragile), ["C4", "D4"]);
}

#[test]
fn rule_catalog_0040_blink_should_resolve_deaths_before_a_private_spellbook_draw() {
    let mut checkpoint = blink_checkpoint(false);
    let drawn = state(&checkpoint.session)["players"]["north"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("top Spellbook identity")
        .to_owned();
    let before = hand_count(&checkpoint.session, "spellbook");

    let spell = checkpoint.spell.clone();
    let sparkmage = checkpoint.sparkmage.clone();
    let receipt = cast_blink(
        &mut checkpoint.session,
        &spell,
        &sparkmage,
        "E4",
        "spellbook",
    );

    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "unit-teleported",
            "minion-died",
            "spell-drawn",
            "magic-resolved",
        ]
    );
    assert_eq!(
        realm_unit(&checkpoint.session, &sparkmage).expect("blinked ally")["location"],
        "E4"
    );
    assert!(
        realm_unit(&checkpoint.session, &checkpoint.fragile).is_none(),
        "leaving the aura behind must kill the wounded ally"
    );
    assert!(
        state(&checkpoint.session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == checkpoint.fragile.as_str())
    );
    assert_eq!(
        hand_count(&checkpoint.session, "spellbook"),
        before,
        "Blink leaves hand and its draw replaces it"
    );
    assert!(
        state(&checkpoint.session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == drawn.as_str())
    );
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(&drawn),
        "the drawn spell identity must stay private"
    );
    assert!(
        checkpoint
            .session
            .verify_replay()
            .expect("verified exact replay")
    );
}

#[test]
fn rule_catalog_0040_blink_should_draw_a_site_when_the_caster_chooses_its_atlas() {
    let mut checkpoint = blink_checkpoint(false);
    let drawn = state(&checkpoint.session)["players"]["north"]["atlas"][0]["instanceId"]
        .as_str()
        .expect("top Atlas identity")
        .to_owned();
    let before = hand_count(&checkpoint.session, "atlas");

    let spell = checkpoint.spell.clone();
    let sparkmage = checkpoint.sparkmage.clone();
    let receipt = cast_blink(&mut checkpoint.session, &spell, &sparkmage, "E4", "atlas");

    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "unit-teleported",
            "minion-died",
            "site-drawn",
            "magic-resolved",
        ]
    );
    assert_eq!(hand_count(&checkpoint.session, "atlas"), before + 1);
    assert!(
        state(&checkpoint.session)["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("North Atlas hand")
            .iter()
            .any(|card| card["instanceId"] == drawn.as_str())
    );
    assert!(
        !serde_json::to_string(&receipt.events)
            .expect("event JSON")
            .contains(&drawn),
        "the drawn site identity must stay private"
    );
    assert!(
        checkpoint
            .session
            .verify_replay()
            .expect("verified exact replay")
    );
}

#[test]
fn rule_catalog_0040_blink_should_keep_an_aura_that_never_leaves_its_neighbour() {
    let mut checkpoint = blink_checkpoint(false);
    let spell = checkpoint.spell.clone();
    let sparkmage = checkpoint.sparkmage.clone();
    let receipt = cast_blink(
        &mut checkpoint.session,
        &spell,
        &sparkmage,
        "C4",
        "spellbook",
    );

    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "unit-teleported",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(
        realm_unit(&checkpoint.session, &sparkmage).expect("blinked ally")["location"],
        "C4"
    );
    assert!(
        realm_unit(&checkpoint.session, &checkpoint.fragile).is_some(),
        "staying nearby must keep the aura and the wounded ally alive"
    );
    assert!(
        checkpoint
            .session
            .verify_replay()
            .expect("verified exact replay")
    );
}

#[test]
fn rule_catalog_0040_blink_should_settle_interrupting_deathrites_before_its_draw() {
    let mut checkpoint = blink_scenario(false, true);
    let drawn = state(&checkpoint.session)["players"]["north"]["spellbook"][0]["instanceId"]
        .as_str()
        .expect("top Spellbook identity")
        .to_owned();
    let life = state(&checkpoint.session)["players"]["north"]["avatar"]["life"].clone();

    let spell = checkpoint.spell.clone();
    let sparkmage = checkpoint.sparkmage.clone();
    let receipt = cast_blink(
        &mut checkpoint.session,
        &spell,
        &sparkmage,
        "E4",
        "spellbook",
    );

    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "unit-teleported",
            "deathrite-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
            "minion-died",
            "spell-drawn",
            "magic-resolved",
        ],
        "the Deathrite the death owes must resolve before Blink pays its draw"
    );
    assert_ne!(
        state(&checkpoint.session)["players"]["north"]["avatar"]["life"],
        life,
        "the interrupting Deathrite must have struck everything at the corpse's location"
    );
    assert_eq!(
        state(&checkpoint.session)["pendingDeathrites"],
        Value::Null,
        "an undecided Deathrite must never survive the cast"
    );
    assert!(
        state(&checkpoint.session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North Spellbook hand")
            .iter()
            .any(|card| card["instanceId"] == drawn.as_str())
    );
    assert!(
        checkpoint
            .session
            .verify_replay()
            .expect("verified exact replay")
    );
}

/// Wounds two fragile allies that share the one aura Blink is about to carry away.
struct OrderedBlink {
    fragile: [String; 2],
    session: Session,
    sparkmage: String,
    spell: String,
}

fn ordered_blink_scenario() -> OrderedBlink {
    let mut session = Session::new(&ordered_manifest()).expect("valid ordered Blink scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    end_turn(&mut session);

    draw(&mut session, "atlas");
    play_site(&mut session, "C1");
    end_turn(&mut session);

    draw(&mut session, "spellbook");
    play_site(&mut session, "D4");
    end_turn(&mut session);

    draw(&mut session, "atlas");
    end_turn(&mut session);

    draw(&mut session, "spellbook");
    play_site(&mut session, "E4");
    end_turn(&mut session);

    draw(&mut session, "atlas");
    end_turn(&mut session);

    // The last Spellbook draw empties the deck, so every part is in hand whatever the shuffle.
    draw(&mut session, "spellbook");
    let fragile = [
        summon(&mut session, "north-fragile", "C4"),
        summon(&mut session, "north-fragile", "C4"),
    ];
    let sparkmage = summon(&mut session, "north-sparkmage", "D4");
    let storm = hand_in(&session, "spellbook", "north-storm");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardInstanceId"] == storm.as_str()
    });
    let spell = hand_in(&session, "spellbook", "north-blink");
    OrderedBlink {
        fragile,
        session,
        sparkmage,
        spell,
    }
}

#[test]
fn rule_catalog_0040_blink_should_owe_its_draw_until_ordered_deathrites_are_chosen() {
    let mut checkpoint = ordered_blink_scenario();
    assert!(
        checkpoint.fragile.iter().all(|instance_id| {
            realm_unit(&checkpoint.session, instance_id)
                .is_some_and(|unit| unit["damage"] == json!(1))
        }),
        "the shared aura must be all that keeps both wounded allies standing"
    );
    let atlas_before = hand_count(&checkpoint.session, "atlas");

    let spell = checkpoint.spell.clone();
    let sparkmage = checkpoint.sparkmage.clone();
    let interrupted = cast_blink(&mut checkpoint.session, &spell, &sparkmage, "E4", "atlas");

    assert_eq!(
        event_types(&interrupted),
        ["magic-cast", "unit-teleported"],
        "Blink may not resolve while the deaths it caused still owe an order"
    );
    let pending = state(&checkpoint.session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(pending["decisionSeat"], "north");
    assert_eq!(
        pending["pendingDeathrites"]["continuation"],
        json!({
            "cardId": "north-blink",
            "instanceId": checkpoint.spell,
            "kind": "blink",
            "owner": "north",
            "seat": "north",
            "zone": "atlas",
        }),
        "the undecided state must carry the draw Blink still owes"
    );
    assert_eq!(hand_count(&checkpoint.session, "atlas"), atlas_before);

    let orders: Vec<_> = checkpoint
        .session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .collect();
    assert_eq!(orders.len(), 2, "either corpse may strike first");

    for order in orders {
        let mut branch = checkpoint.session.clone();
        let StepResult::Accepted(ordered) = branch
            .step(ActionRequest {
                action_id: order.action_id.to_string(),
                seat: order.seat,
                state_version: order.state_version,
            })
            .expect("ordered Blink completion")
        else {
            panic!("engine-issued Deathrite order must be accepted");
        };
        let types = event_types(&ordered);
        assert_eq!(types.first(), Some(&"deathrite-order-committed"));
        assert_eq!(
            types.last(),
            Some(&"magic-resolved"),
            "Blink resolves only once its resumed draw is paid"
        );
        let drawn = types
            .iter()
            .position(|event_type| *event_type == "site-drawn")
            .expect("resumed Blink draw");
        assert!(
            drawn
                > types
                    .iter()
                    .rposition(|event_type| *event_type == "minion-died")
                    .expect("both Deathrite deaths"),
            "every death the Deathrites owe must land before the draw"
        );
        let completed = state(&branch);
        assert_eq!(completed["phase"], "main");
        assert_eq!(completed["pendingDeathrites"], Value::Null);
        assert_eq!(hand_count(&branch, "atlas"), atlas_before + 1);
        assert!(
            checkpoint
                .fragile
                .iter()
                .all(|instance_id| realm_unit(&branch, instance_id).is_none())
        );
        assert_eq!(
            realm_unit(&branch, &sparkmage).expect("blinked ally")["location"],
            "E4"
        );
        assert!(branch.verify_replay().expect("verified exact replay"));
    }
}

#[test]
fn rule_catalog_0040_blink_should_lose_the_game_when_its_chosen_deck_is_empty() {
    let mut checkpoint = blink_checkpoint(true);
    assert_eq!(
        state(&checkpoint.session)["players"]["north"]["spellbook"],
        json!([])
    );

    let spell = checkpoint.spell.clone();
    let sparkmage = checkpoint.sparkmage.clone();
    let receipt = cast_blink(
        &mut checkpoint.session,
        &spell,
        &sparkmage,
        "E4",
        "spellbook",
    );

    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "unit-teleported",
            "minion-died",
            "magic-resolved",
            "game-ended",
        ]
    );
    assert_eq!(
        state(&checkpoint.session)["terminal"],
        json!({
            "loser": "north",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "south",
        })
    );
    assert!(
        checkpoint
            .session
            .verify_replay()
            .expect("verified exact replay")
    );
}
