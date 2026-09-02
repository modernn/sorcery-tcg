//! Direct proofs for discard-funded random damage (RULE-CATALOG-0152 / 0153 / 0154).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
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
        "defense": 9,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn manifest(seed: u32, cards: &Value, north: &[&str], south: &[&str]) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "discard-random-damage-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-discard-random-damage-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 12],
                "avatar": "north-avatar",
                "spellbook": north,
            },
            "south": {
                "atlas": vec!["south-site"; 12],
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

fn activate(session: &mut Session, source_instance_id: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-discard-random-damage"
            && descriptor["sourceInstanceId"] == source_instance_id
    })
}

fn offered_activations(session: &Session, source_instance_id: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "activate-discard-random-damage"
                && action.descriptor["sourceInstanceId"] == source_instance_id
        })
        .map(|action| action.descriptor)
        .collect()
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn hand_identities(current: &Value, seat: &str) -> Vec<String> {
    current["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("opening hand")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("hand identity")
                .to_owned()
        })
        .collect()
}

fn cemetery_identities(current: &Value, seat: &str) -> Vec<String> {
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

fn realm_damage(current: &Value, instance_id: &str) -> u64 {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("realm minion")["damage"]
        .as_u64()
        .unwrap_or(0)
}

fn candidate_count(receipt: &Receipt) -> usize {
    let draws: Vec<_> = receipt
        .random_draws
        .iter()
        .filter(|draw| draw["purpose"] == "discard_spell_random_other_unit_here")
        .collect();
    assert_eq!(draws.len(), 1, "one hidden random draw per activation");
    assert_eq!(draws[0]["domain"]["kind"], "unit_index_candidate");
    usize::try_from(
        draws[0]["domain"]["exclusiveMaximum"]
            .as_u64()
            .expect("candidate count"),
    )
    .expect("candidate count fits")
}

fn event_payload<'a>(receipt: &'a Receipt, event_type: &str) -> &'a Value {
    &receipt
        .events
        .iter()
        .find(|event| event.event_type == event_type)
        .unwrap_or_else(|| panic!("{event_type} event"))
        .payload
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
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

fn seeded(cards: &Value, north: &[&str], south: &[&str], wanted: &[&str]) -> String {
    (1..=4096)
        .map(|seed| manifest(seed, cards, north, south))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("discard damage candidate");
            let current = state(&preview);
            wanted.iter().all(|entry| {
                let (seat, card_id) = entry.split_once(':').expect("seat-qualified card");
                current["players"][seat]["hand"]["spellbook"]
                    .as_array()
                    .expect("opening hand")
                    .iter()
                    .any(|card| card["cardId"] == card_id)
            })
        })
        .expect("bounded seed with the required discard damage opening hands")
}

#[test]
fn rule_catalog_0152_discarding_a_chosen_spell_should_damage_a_random_other_unit_here() {
    let cards = json!({
        "north-avatar": avatar(),
        "north-fodder": minion(json!({})),
        "north-jinn": minion(json!({ "discardSpellToDamageRandomOtherUnitHere": 3 })),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-fodder": minion(json!({})),
        "south-site": site(),
    });
    let north = ["north-jinn", "north-fodder"].repeat(6);
    let south = ["south-fodder"; 12];
    let manifest = seeded(&cards, &north, &south, &["north:north-jinn"]);
    let mut session = Session::new(&manifest).expect("valid discard damage scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let jinn = summon(&mut session, "north-jinn", "C4");

    let before = state(&session);
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let hand = hand_identities(&before, "north");
    let offered = offered_activations(&session, &jinn);
    assert_eq!(
        offered.len(),
        hand.len(),
        "one activation per discardable Spellbook card"
    );
    for descriptor in &offered {
        let canonical = canonical_json(descriptor).expect("canonical activation");
        assert!(
            !canonical.contains("target"),
            "the random target stays hidden until resolution"
        );
    }
    let (descriptor, receipt) = activate(&mut session, &jinn);
    let discarded = descriptor["discardCardInstanceId"]
        .as_str()
        .expect("chosen discard identity")
        .to_owned();

    assert_eq!(
        event_types(&receipt),
        [
            "card-discarded",
            "discard-random-damage-activated",
            "discard-random-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
        ]
    );
    assert_eq!(candidate_count(&receipt), 1);
    let activated = event_payload(&receipt, "discard-random-damage-activated");
    assert_eq!(activated["amount"], 3);
    assert_eq!(activated["discardCardInstanceId"], discarded.as_str());
    assert_eq!(activated["sourceLocation"]["cell"], "C4");
    assert_eq!(activated["targetInstanceId"], avatar_id.as_str());
    assert_eq!(activated["targetKind"], "avatar");
    let dealt = event_payload(&receipt, "damage-dealt");
    assert_eq!(dealt["amount"], 3);
    assert_eq!(dealt["instanceId"], avatar_id.as_str());

    let after = state(&session);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 17);
    assert_eq!(cemetery_identities(&after, "north"), [discarded.as_str()]);
    let remaining = hand_identities(&after, "north");
    assert!(!remaining.contains(&discarded));
    assert_eq!(remaining.len(), hand.len() - 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0153_random_other_unit_candidates_should_include_allies_avatars_and_stealth() {
    let cards = json!({
        "north-ally": minion(json!({})),
        "north-avatar": avatar(),
        "north-jinn": minion(json!({ "discardSpellToDamageRandomOtherUnitHere": 1 })),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-lurker": minion(json!({ "stealth": true, "summonToAnySite": true })),
        "south-raider": minion(json!({ "summonToAnySite": true })),
        "south-site": site(),
    });
    let north = ["north-jinn", "north-ally"].repeat(6);
    let south = ["south-raider", "south-lurker"].repeat(6);
    let manifest = seeded(
        &cards,
        &north,
        &south,
        &[
            "north:north-jinn",
            "north:north-ally",
            "south:south-raider",
            "south:south-lurker",
        ],
    );
    let mut session = Session::new(&manifest).expect("valid candidate scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    let jinn = summon(&mut session, "north-jinn", "C4");

    let (_, only_avatar) = activate(&mut session, &jinn);
    assert_eq!(
        candidate_count(&only_avatar),
        1,
        "the Avatar is a candidate"
    );
    end_and_draw(&mut session);

    play_site(&mut session, "C1");
    end_and_draw(&mut session);

    summon(&mut session, "north-ally", "C4");
    let (_, with_ally) = activate(&mut session, &jinn);
    assert_eq!(
        candidate_count(&with_ally),
        2,
        "an allied minion joins the candidates"
    );
    end_and_draw(&mut session);

    summon(&mut session, "south-raider", "C4");
    end_and_draw(&mut session);

    let (_, with_enemy) = activate(&mut session, &jinn);
    assert_eq!(
        candidate_count(&with_enemy),
        3,
        "an enemy minion joins the candidates"
    );
    end_and_draw(&mut session);

    summon(&mut session, "south-lurker", "C4");
    end_and_draw(&mut session);

    let (_, with_stealth) = activate(&mut session, &jinn);
    assert_eq!(
        candidate_count(&with_stealth),
        4,
        "a Stealth minion stays a candidate for hidden random damage"
    );
    assert_exact_replay(&session);
}

/// Walks North's Jinn onto its own C3 site so exactly one South answer can join it there.
fn jinn_versus(
    guard_card: &str,
    guard_facts: Value,
    extra_north: Option<(&str, Value)>,
) -> Session {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-jinn": minion(json!({
            "attack": 3,
            "discardSpellToDamageRandomOtherUnitHere": 3,
        })),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    let card_map = cards.as_object_mut().expect("scenario cards");
    card_map.insert(guard_card.to_owned(), guard_facts);
    let mut north = vec!["north-jinn"; 12];
    let mut wanted = vec!["north:north-jinn".to_owned()];
    if let Some((card_id, facts)) = extra_north {
        card_map.insert(card_id.to_owned(), facts);
        north = [vec!["north-jinn"; 6], vec![card_id; 6]].concat();
        wanted.push(format!("north:{card_id}"));
    }
    wanted.push(format!("south:{guard_card}"));
    let south = vec![guard_card; 12];
    let wanted: Vec<&str> = wanted.iter().map(String::as_str).collect();
    let manifest = seeded(&cards, &north, &south, &wanted);
    let mut session = Session::new(&manifest).expect("valid discard damage answer scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "C4");
    end_and_draw(&mut session);
    play_site(&mut session, "C1");
    end_and_draw(&mut session);
    play_site(&mut session, "C3");
    session
}

#[test]
fn rule_catalog_0154_discard_damage_should_snapshot_derived_power_and_use_ward_and_prevention() {
    let mut warded = jinn_versus(
        "south-ward",
        minion(json!({ "summonToAnySite": true, "ward": true })),
        None,
    );
    let jinn = summon(&mut warded, "north-jinn", "C3");
    end_and_draw(&mut warded);
    let ward = summon(&mut warded, "south-ward", "C3");
    end_and_draw(&mut warded);
    let (_, broken) = activate(&mut warded, &jinn);
    assert_eq!(candidate_count(&broken), 1);
    assert!(event_types(&broken).contains(&"ward-broken"));
    assert_eq!(event_payload(&broken, "damage-dealt")["amount"], 0);
    assert_eq!(realm_damage(&state(&warded), &ward), 0);
    let (_, after_ward) = activate(&mut warded, &jinn);
    assert_eq!(event_payload(&after_ward, "damage-dealt")["amount"], 3);
    assert_eq!(realm_damage(&state(&warded), &ward), 3);
    assert_exact_replay(&warded);

    let mut prevented = jinn_versus(
        "south-guard",
        minion(json!({
            "preventsDamageFromUnitsWithPowerAtLeast": 4,
            "summonToAnySite": true,
        })),
        Some((
            "north-buffer",
            minion(json!({
                "diesAtEndOfControllerTurn": true,
                "otherNearbyAlliesPowerBonus": 1,
            })),
        )),
    );
    let jinn = summon(&mut prevented, "north-jinn", "C3");
    end_and_draw(&mut prevented);
    let guard = summon(&mut prevented, "south-guard", "C3");
    end_and_draw(&mut prevented);

    summon(&mut prevented, "north-buffer", "C4");
    let (_, receipt) = activate(&mut prevented, &jinn);
    let blocked = event_payload(&receipt, "damage-dealt");
    assert_eq!(blocked["amount"], 0);
    assert_eq!(blocked["attemptedAmount"], 3);
    assert_eq!(blocked["prevented"], true);
    assert_eq!(realm_damage(&state(&prevented), &guard), 0);
    end_and_draw(&mut prevented);
    end_and_draw(&mut prevented);

    let (_, unbuffed) = activate(&mut prevented, &jinn);
    let landed = event_payload(&unbuffed, "damage-dealt");
    assert_eq!(landed["amount"], 3);
    assert_eq!(landed["accumulated"], 3);
    assert_eq!(realm_damage(&state(&prevented), &guard), 3);
    assert_exact_replay(&prevented);
}
