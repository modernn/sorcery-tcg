//! Direct proofs for derived static power bonuses (RULE-CATALOG-0034 / 0035 / 0036 / 0037).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn magic(effect: (&str, Value)) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("Magic facts")
        .insert(effect.0.to_owned(), effect.1);
    value
}

fn manifest(
    seed: u32,
    cards: &Value,
    north: &[&str],
    south: &[&str],
    north_atlas: usize,
) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "static-power-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-static-power-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; north_atlas],
                "avatar": "north-avatar",
                "spellbook": north,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": south,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
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

fn end_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn summon(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned()
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_unit<'a>(current: &'a Value, instance_id: &str) -> Option<&'a Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn cemetery_order(current: &Value, seat: &str) -> Vec<String> {
    current["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("cemetery identity")
                .to_owned()
        })
        .collect()
}

fn died_order(receipt: &Receipt) -> Vec<String> {
    receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-died")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("death identity")
                .to_owned()
        })
        .collect()
}

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("captured static power checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn seeded_where(
    cards: &Value,
    north: &[&str],
    south: &[&str],
    north_atlas: usize,
    accept: impl Fn(&Value) -> bool,
) -> String {
    (1..=4096)
        .map(|seed| manifest(seed, cards, north, south, north_atlas))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("static power candidate");
            accept(&state(&preview))
        })
        .expect("bounded seed with the required static power opening hands")
}

fn seeded(
    cards: &Value,
    north: &[&str],
    south: &[&str],
    wanted: &[&str],
    north_atlas: usize,
) -> String {
    seeded_where(cards, north, south, north_atlas, |current| {
        wanted.iter().all(|entry| {
            let (seat, card_id) = entry.split_once(':').expect("seat-qualified card");
            current["players"][seat]["hand"]["spellbook"]
                .as_array()
                .expect("opening hand")
                .iter()
                .any(|card| card["cardId"] == card_id)
        })
    })
}

#[test]
fn rule_catalog_0034_nearby_allies_power_is_derived_and_settles_deaths_when_its_source_dies() {
    let cards = json!({
        "north-ally": minion(json!({})),
        "north-avatar": avatar(),
        "north-far": minion(json!({})),
        "north-site": site(),
        "north-source": minion(json!({ "otherNearbyAlliesPowerBonus": 1 })),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-rain": magic(("damageEachAbovegroundMinion", json!(1))),
        "south-site": site(),
    });
    let north = [
        "north-source",
        "north-ally",
        "north-far",
        "north-far",
        "north-far",
        "north-far",
    ];
    let south = [
        "south-rain",
        "south-filler",
        "south-rain",
        "south-filler",
        "south-rain",
        "south-filler",
    ];
    let manifest = seeded(
        &cards,
        &north,
        &south,
        &[
            "north:north-source",
            "north:north-ally",
            "north:north-far",
            "south:south-rain",
        ],
        6,
    );
    let mut session = Session::new(&manifest).expect("valid static power scenario");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "C4");
    let source_id = summon(&mut session, "north-source", "C4");
    let ally_id = summon(&mut session, "north-ally", "C4");
    end_and_draw(&mut session);
    play_site(&mut session, "C1");
    end_and_draw(&mut session);
    play_site(&mut session, "B4");
    end_and_draw(&mut session);
    play_site(&mut session, "C2");
    end_and_draw(&mut session);
    play_site(&mut session, "A4");
    let far_id = summon(&mut session, "north-far", "A4");
    end_and_draw(&mut session);

    // One point of Rain is lethal to every printed 1/1 that the source does not reach.
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "south-rain"
    });
    // The far ally shares the direct batch with the source; the nearby ally only falls after it.
    let mut direct = died_order(&receipt);
    let settled = direct.pop().expect("settled death");
    direct.sort_unstable();
    let mut expected = vec![far_id.clone(), source_id.clone()];
    expected.sort_unstable();
    assert_eq!(direct, expected);
    assert_eq!(settled, ally_id);

    let finished = state(&session);
    assert!(realm_unit(&finished, &source_id).is_none());
    assert!(realm_unit(&finished, &ally_id).is_none());
    assert!(realm_unit(&finished, &far_id).is_none());
    let cemetery = cemetery_order(&finished, "north");
    assert_eq!(cemetery.len(), 3);
    assert_eq!(cemetery.last().expect("settled corpse"), &ally_id);
    assert_exact_replay(&session);
}

/// Walks both seats into a North Mortal at C4, South units at C1, and the South source at C2.
fn controlled_mortal_position(session: &mut Session) -> (String, String, String, String) {
    play_site(session, "C4");
    let north_mortal = summon(session, "north-mortal", "C4");
    end_and_draw(session);
    play_site(session, "C1");
    let south_mortal = summon(session, "south-mortal", "C1");
    let south_plain = summon(session, "south-plain", "C1");
    end_and_draw(session);
    play_site(session, "C3");
    end_and_draw(session);
    play_site(session, "C2");
    let king = summon(session, "south-king", "C2");
    end_and_draw(session);
    // Two quiet rounds stock North's hand and return the decision to North untouched.
    for _ in 0..2 {
        end_and_draw(session);
        end_and_draw(session);
    }
    (north_mortal, south_mortal, south_plain, king)
}

fn rain(session: &mut Session) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    receipt
}

fn damage_of(session: &Session, instance_id: &str) -> Value {
    realm_unit(&state(session), instance_id).expect("living unit")["damage"].clone()
}

#[test]
fn rule_catalog_0035_controlled_mortal_power_should_follow_current_control_and_settle_deaths() {
    let cards = json!({
        "north-avatar": avatar(),
        "north-mesmerism": magic(("gainControlOfTargetNearbyMinion", json!(true))),
        "north-mortal": minion(json!({ "defense": 2, "mortal": true })),
        "north-rain": magic(("damageEachAbovegroundMinion", json!(1))),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-king": minion(json!({
            "defense": 3,
            "mortal": true,
            "otherControlledMortalsPowerBonus": 1,
        })),
        "south-mortal": minion(json!({ "mortal": true })),
        "south-plain": minion(json!({})),
        "south-site": site(),
    });
    let north = [
        "north-mortal",
        "north-mesmerism",
        "north-rain",
        "north-rain",
        "north-rain",
        "north-rain",
        "north-rain",
        "north-rain",
        "north-rain",
        "north-rain",
    ];
    let south = [
        "south-mortal",
        "south-plain",
        "south-king",
        "south-plain",
        "south-plain",
        "south-plain",
        "south-plain",
        "south-plain",
        "south-plain",
        "south-plain",
    ];
    let manifest = seeded(
        &cards,
        &north,
        &south,
        &[
            "north:north-mortal",
            "north:north-mesmerism",
            "north:north-rain",
            "south:south-mortal",
            "south:south-plain",
            "south:south-king",
        ],
        6,
    );
    let mut session = Session::new(&manifest).expect("valid controlled Mortal scenario");
    keep(&mut session);
    keep(&mut session);
    let (north_mortal, south_mortal, south_plain, king) = controlled_mortal_position(&mut session);

    // One point of Rain separates the buffed South Mortal from its unbuffed non-Mortal ally.
    let first = rain(&mut session);
    assert_eq!(died_order(&first), [south_plain.as_str()]);
    assert_eq!(damage_of(&session, &south_mortal), 1);
    assert_eq!(damage_of(&session, &north_mortal), 1);
    assert_eq!(damage_of(&session, &king), 1);

    // Stealing the source flips whose Mortals derive the bonus and settles the loser immediately.
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == avatar_id.as_str()
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    let (_, stolen) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-mesmerism"
            && descriptor["target"]["instanceId"] == king.as_str()
    });
    assert_eq!(died_order(&stolen), [south_mortal.as_str()]);
    let transferred = state(&session);
    assert_eq!(
        realm_unit(&transferred, &king).expect("stolen source")["controller"],
        "north"
    );
    assert!(realm_unit(&transferred, &south_mortal).is_none());

    // North's Mortal now derives the stolen bonus and outlives its printed defense of two.
    let second = rain(&mut session);
    assert!(died_order(&second).is_empty());
    assert_eq!(damage_of(&session, &north_mortal), 2);
    assert_exact_replay(&session);
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn defender_ids(current: &Value) -> Vec<&str> {
    current["pendingCombat"]["defenders"]
        .as_array()
        .expect("pending combat defenders")
        .iter()
        .map(|defender| defender["instanceId"].as_str().expect("defender identity"))
        .collect()
}

/// Walks both seats into a declared attack on a wounded North target at C4, with two fragile
/// allies that only survive because the aura source stands next door at B4.
fn stale_combat_defend_position(session: &mut Session) -> (String, String, Vec<String>) {
    play_site(session, "C4");
    let target_id = summon(session, "north-target", "C4");
    let mut fragile_ids = vec![summon(session, "north-fragile", "C4")];
    let source_id = summon(session, "north-source", "C4");
    end_and_draw(session);
    play_site(session, "C1");
    end_and_draw(session);

    play_site(session, "B4");
    fragile_ids.push(summon(session, "north-fragile", "C4"));
    // Stepping the aura source next door keeps both fragile allies on two effective defense.
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == source_id.as_str()
            && descriptor["path"]
                == json!([
                    { "cell": "C4", "region": "surface" },
                    { "cell": "B4", "region": "surface" },
                ])
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    end_and_draw(session);

    play_site(session, "B1");
    let attacker_id = summon(session, "south-attacker", "C4");
    end_and_draw(session);
    play_site(session, "A4");
    end_and_draw(session);
    play_site(session, "A1");

    // One point of Rain leaves every fragile ally exactly one aura point from death.
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "south-rain"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id.as_str()
            && descriptor["path"] == json!([{ "cell": "C4", "region": "surface" }])
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id.as_str()
    });
    (source_id, target_id, fragile_ids)
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct proof keeps the doomed defender, the paused path, ordered Deathrites, and replay together"
)]
fn rule_catalog_0037_aura_loss_deaths_cannot_restore_stale_combat_during_a_defender_path() {
    let cards = json!({
        "north-avatar": avatar(),
        "north-fragile": minion(json!({ "deathriteHeal": 3 })),
        "north-site": site(),
        "north-source": minion(json!({
            "defense": 2,
            "movementBonus": 2,
            "otherNearbyAlliesPowerBonus": 1,
        })),
        "north-target": minion(json!({ "defense": 3 })),
        "south-attacker": minion(json!({
            "charge": true,
            "defense": 10,
            "summonToAnySite": true,
        })),
        "south-avatar": avatar(),
        "south-rain": magic(("damageEachAbovegroundMinion", json!(1))),
        "south-site": site(),
    });
    let north = [
        "north-target",
        "north-fragile",
        "north-source",
        "north-fragile",
        "north-target",
        "north-source",
    ];
    let south = [
        "south-attacker",
        "south-rain",
        "south-attacker",
        "south-rain",
        "south-attacker",
        "south-rain",
    ];
    // North opens on all three roles and draws the second fragile ally next.
    let encoded = seeded_where(&cards, &north, &south, 3, |current| {
        let opening = |seat: &str| -> Vec<&str> {
            current["players"][seat]["hand"]["spellbook"]
                .as_array()
                .expect("opening hand")
                .iter()
                .map(|card| card["cardId"].as_str().expect("opening card"))
                .collect()
        };
        let north_opening = opening("north");
        let south_opening = opening("south");
        ["north-target", "north-fragile", "north-source"]
            .iter()
            .all(|card_id| north_opening.contains(card_id))
            && current["players"]["north"]["spellbook"][0]["cardId"] == "north-fragile"
            && south_opening.contains(&"south-attacker")
            && south_opening.contains(&"south-rain")
    });
    let mut session = Session::new(&encoded).expect("valid stale combat defend scenario");
    keep(&mut session);
    keep(&mut session);
    let (source_id, _target_id, fragile_ids) = stale_combat_defend_position(&mut session);

    let wounded = state(&session);
    assert_eq!(fragile_ids.len(), 2);
    for fragile_id in &fragile_ids {
        assert_eq!(
            realm_unit(&wounded, fragile_id).expect("wounded fragile ally")["damage"],
            1
        );
    }

    // One fragile ally joins the defence while the aura still holds it up, so the combat it
    // leaves behind when it dies is the stale combat this rule must not restore.
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == fragile_ids[0].as_str()
            && descriptor["path"].as_array().expect("defend path").len() == 1
    });
    assert_eq!(
        defender_ids(&state(&session)),
        [fragile_ids[0].as_str()],
        "the doomed ally must hold the combat that its own death later abandons"
    );

    // The aura source defends along a path that abandons its allies and returns to them.
    let (_, defended) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "defend"
            && descriptor["unitInstanceId"] == source_id.as_str()
            && descriptor["path"]
                == json!([
                    { "cell": "B4", "region": "surface" },
                    { "cell": "A4", "region": "surface" },
                    { "cell": "B4", "region": "surface" },
                    { "cell": "C4", "region": "surface" },
                ])
    });
    let paused = state(&session);
    assert_eq!(
        (
            event_types(&defended),
            paused["phase"].clone(),
            paused["decisionSeat"].clone()
        ),
        (
            vec!["basic-movement-started"],
            json!("deathrite-order"),
            json!("north")
        ),
        "leaving the aura mid-path must kill both allies and owe an ordered Deathrite"
    );

    // Both allies are off the board but stay out of the cemetery until North orders them.
    let paused_cemetery = cemetery_order(&paused, "north");
    for fragile_id in &fragile_ids {
        assert!(realm_unit(&paused, fragile_id).is_none());
        assert!(!paused_cemetery.contains(fragile_id));
    }
    let mut owed: Vec<String> = session
        .legal_actions()
        .expect("Deathrite ordering actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("owed Deathrite source")
                .to_owned()
        })
        .collect();
    owed.sort_unstable();
    let mut expected_owed = fragile_ids.clone();
    expected_owed.sort_unstable();
    assert_eq!(owed, expected_owed);
    assert_checkpoint_round_trip(&session);

    // Ordering the owed Deathrites settles both corpses and hands the path back to North.
    let (_, ordered) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == owed[0].as_str()
    });
    let settled = state(&session);
    let settled_cemetery = cemetery_order(&settled, "north");
    assert_eq!(settled["phase"], "movement");
    assert_eq!(died_order(&ordered).len(), 2);
    for fragile_id in &fragile_ids {
        assert!(settled_cemetery.contains(fragile_id));
    }

    // The abandoned path still owes every remaining step before anyone joins combat.
    let mut receipts = vec![defended, ordered];
    for _ in 0..8 {
        if state(&session)["phase"] != "movement" {
            break;
        }
        let (_, continued) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "continue-basic-movement"
        });
        receipts.push(continued);
    }
    let joined = state(&session);
    let events: Vec<&str> = receipts.iter().flat_map(event_types).collect();
    let count = |wanted: &str| events.iter().filter(|event| **event == wanted).count();
    assert_eq!(joined["phase"], "defend");
    assert_eq!(
        defender_ids(&joined),
        [source_id.as_str()],
        "the dead ally's stale defence must not return alongside the surviving defender"
    );
    assert_eq!(
        (
            count("basic-movement-started"),
            count("basic-movement-continued"),
            count("defender-joined"),
        ),
        (1, 2, 1)
    );
    assert!(
        events.iter().position(|event| *event == "defender-joined")
            > events.iter().rposition(|event| *event == "minion-died"),
        "the surviving defender may only join combat after every aura-loss death has settled"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0036_aura_loss_deathrite_should_end_the_game_after_its_triggering_magic_resolves() {
    let cards = json!({
        "north-ally": minion(json!({ "deathriteDrawSite": true })),
        "north-avatar": avatar(),
        "north-rain": magic(("damageEachAbovegroundMinion", json!(1))),
        "north-site": site(),
        "north-source": minion(json!({ "defense": 2, "otherNearbyAlliesPowerBonus": 1 })),
        "north-teleport": magic(("teleportAllyToTargetSite", json!(true))),
        "south-avatar": avatar(),
        "south-filler": minion(json!({})),
        "south-site": site(),
    });
    let north = [
        "north-source",
        "north-ally",
        "north-rain",
        "north-teleport",
        "north-source",
        "north-ally",
        "north-rain",
        "north-teleport",
    ];
    let south = ["south-filler"; 8];
    // Three Atlas cards are the whole opening hand, so the Deathrite draw has nothing behind it.
    let manifest = seeded(
        &cards,
        &north,
        &south,
        &["north:north-source", "north:north-ally"],
        3,
    );
    let mut session = Session::new(&manifest).expect("valid aura-loss Deathrite scenario");
    keep(&mut session);
    keep(&mut session);

    play_site(&mut session, "C4");
    let source_id = summon(&mut session, "north-source", "C4");
    let ally_id = summon(&mut session, "north-ally", "C4");
    end_and_draw(&mut session);
    play_site(&mut session, "C1");
    // Four quiet North draws stock the rest of the hand whichever way the Spellbook shuffled.
    for _ in 0..7 {
        end_and_draw(&mut session);
    }
    assert_eq!(state(&session)["players"]["north"]["atlas"], json!([]));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    });
    assert_eq!(damage_of(&session, &ally_id), 1);
    assert_eq!(damage_of(&session, &source_id), 1);

    // Teleporting the aura away drops the ally to its printed defense of one.
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-teleport"
            && descriptor["ally"]["instanceId"] == source_id.as_str()
            && descriptor["targetLocation"]["cell"] == "C1"
    });

    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "magic-cast",
            "unit-teleported",
            "minion-died",
            "magic-resolved",
            "game-ended",
        ],
        "the triggering Magic must resolve before the Deathrite's empty draw ends the game"
    );
    let finished = state(&session);
    assert!(realm_unit(&finished, &ally_id).is_none());
    assert_eq!(
        finished["terminal"],
        json!({
            "loser": "north",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "south",
        })
    );
    assert_exact_replay(&session);
}
