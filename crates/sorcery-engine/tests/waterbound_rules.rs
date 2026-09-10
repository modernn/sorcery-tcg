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

fn site(water: bool) -> Value {
    json!({
        "cardType": "site",
        "elements": if water { ["water"] } else { ["earth"] },
    })
}

fn waterbound(lower: &str) -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        lower: true,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "waterbound": true,
    })
}

fn plain() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(seed: u64, lower: &str) -> String {
    let cards = json!({
        "north-avatar": avatar(),
        "north-land": site(false),
        "north-water": site(true),
        "north-waterbound": waterbound(lower),
        "south-avatar": avatar(),
        "south-plain": plain(),
        "south-site": site(false),
    });
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "waterbound-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-waterbound-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": vec!["north-waterbound"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-plain"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn waterbound_extra(extra: Value) -> Value {
    let mut value = waterbound("submerge");
    let Value::Object(extra) = extra else {
        panic!("extra Waterbound facts must be an object");
    };
    value
        .as_object_mut()
        .expect("Waterbound facts")
        .extend(extra);
    value
}

fn zap() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn teleport() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "teleportAllyToTargetSite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn composed_manifest(
    seed: u64,
    bound: Value,
    extra_cards: Value,
    north_spells: &[&str],
    south_spells: &[&str],
) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-land": site(false),
        "north-water": site(true),
        "south-avatar": avatar(),
        "south-site": site(false),
    });
    cards["north-waterbound"] = bound;
    if north_spells.contains(&"north-teleport") {
        cards["north-teleport"] = teleport();
    }
    if south_spells.contains(&"south-zap") {
        cards["south-zap"] = zap();
    }
    if south_spells.contains(&"south-plain") {
        cards["south-plain"] = plain();
    }
    let Value::Object(extra_cards) = extra_cards else {
        panic!("extra cards must be an object");
    };
    cards
        .as_object_mut()
        .expect("card definitions")
        .extend(extra_cards);
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "waterbound-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-waterbound-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": [
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                    "north-land",
                    "north-water",
                ],
                "avatar": "north-avatar",
                "spellbook": north_spells,
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": south_spells,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
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

fn draw_spell(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn play_site(session: &mut Session, card_id: &str, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
    });
}

fn play_orthogonal_land(session: &mut Session) -> String {
    let (descriptor, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-land"
            && matches!(descriptor["cell"].as_str(), Some("B4" | "C3" | "D4"))
    });
    descriptor["cell"].as_str().expect("land cell").to_owned()
}

fn move_bound_to(session: &mut Session, bound_id: &str, cell: &str) -> Receipt {
    let (_, moved) = accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bound_id
            && descriptor["to"]["cell"] == cell
            && descriptor["to"]["region"] == "surface"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    moved
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn south_turn(session: &mut Session, cell: Option<&str>) {
    draw_spell(session);
    if let Some(cell) = cell {
        play_site(session, "south-site", cell);
    }
    end_turn(session);
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn units(session: &Session) -> Vec<Value> {
    state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .clone()
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| predicate(&action.descriptor))
}

fn event_types(receipt: &Receipt) -> Vec<String> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.clone())
        .collect()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn rule_catalog_0049_waterbound_should_derive_disabled_from_terrain_and_die_without_abilities() {
    let mut session = Session::new(&manifest(136, "submerge")).expect("valid Waterbound scenario");
    keep(&mut session);
    keep(&mut session);
    play_site(&mut session, "north-water", "C4");
    assert!(offers(&session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"] == "underwater"
    }));
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"].is_null()
    });
    let bound_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Waterbound identity")
        .to_owned();
    end_turn(&mut session);
    south_turn(&mut session, Some("C1"));

    draw_spell(&mut session);
    play_site(&mut session, "north-land", "C3");
    let activates_mana = |descriptor: &Value| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == bound_id.as_str()
    };
    assert!(offers(&session, activates_mana));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bound_id.as_str()
            && descriptor["to"]["cell"] == "C3"
            && descriptor["to"]["region"] == "surface"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert!(!offers(&session, activates_mana));
    end_turn(&mut session);
    south_turn(&mut session, None);

    draw_spell(&mut session);
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "move-and-attack" && descriptor["unitInstanceId"] == bound_id.as_str()
    }));
    assert!(!offers(&session, activates_mana));
    assert_eq!(units(&session).len(), 1);
    assert_exact_replay(&session);

    let mut land = Session::new(&manifest(137, "burrowing")).expect("valid land Waterbound");
    keep(&mut land);
    keep(&mut land);
    play_site(&mut land, "north-land", "C4");
    let (_, receipt) = accept_where(&mut land, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"] == "underground"
    });
    assert_eq!(event_types(&receipt), ["minion-summoned", "minion-died"]);
    assert!(units(&land).is_empty());
    assert_exact_replay(&land);
}

fn opening(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid Waterbound composition scenario");
    keep(&mut session);
    keep(&mut session);
    session
}

fn summon_on_water(session: &mut Session) -> String {
    play_site(session, "north-water", "C4");
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Waterbound identity")
        .to_owned()
}

fn move_to_land(session: &mut Session, bound_id: &str) -> Receipt {
    play_site(session, "north-land", "C3");
    let (_, moved) = accept_where(session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == bound_id
            && descriptor["to"]["cell"] == "C3"
            && descriptor["to"]["region"] == "surface"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    moved
}

fn unit_named(session: &Session, instance_id: &str) -> Value {
    units(session)
        .into_iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("named unit")
}

#[test]
fn rule_catalog_0163_waterbound_ward_still_prevents_damage_while_disabled() {
    let manifest = composed_manifest(
        163,
        waterbound_extra(json!({ "ward": true })),
        json!({}),
        &["north-waterbound"; 8],
        &["south-zap"; 8],
    );
    let mut session = opening(&manifest);
    let bound_id = summon_on_water(&mut session);
    assert_eq!(unit_named(&session, &bound_id)["warded"], true);
    end_turn(&mut session);
    south_turn(&mut session, Some("C1"));

    draw_spell(&mut session);
    move_to_land(&mut session, &bound_id);
    assert_eq!(unit_named(&session, &bound_id)["warded"], true);
    assert_eq!(unit_named(&session, &bound_id)["damage"], 0);
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == bound_id
    }));
    end_turn(&mut session);

    draw_spell(&mut session);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["target"]["instanceId"] == bound_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "ward-broken",
            "magic-resolved",
        ]
    );
    let after = unit_named(&session, &bound_id);
    assert_eq!(after["damage"], 0);
    assert_eq!(after["warded"], false);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0164_waterbound_skips_end_turn_stealth_while_disabled_then_gains_it_on_water() {
    let manifest = composed_manifest(
        165,
        waterbound_extra(json!({ "gainsStealthAtEndOfTurn": true })),
        json!({}),
        &[
            "north-waterbound",
            "north-teleport",
            "north-teleport",
            "north-teleport",
            "north-teleport",
            "north-teleport",
            "north-teleport",
            "north-teleport",
        ],
        &["south-plain"; 8],
    );
    let mut session = opening(&manifest);
    let bound_id = summon_on_water(&mut session);
    assert_eq!(unit_named(&session, &bound_id)["stealthed"], false);
    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        event_types(&ended)
            .iter()
            .any(|kind| kind == "stealth-gained")
    );
    assert_eq!(unit_named(&session, &bound_id)["stealthed"], true);
    south_turn(&mut session, Some("C1"));

    draw_spell(&mut session);
    let land_cell = play_orthogonal_land(&mut session);
    let moved = move_bound_to(&mut session, &bound_id, &land_cell);
    assert!(
        event_types(&moved)
            .iter()
            .any(|kind| kind == "stealth-lost")
    );
    assert_eq!(unit_named(&session, &bound_id)["stealthed"], false);
    let (_, disabled_end) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !event_types(&disabled_end)
            .iter()
            .any(|kind| kind == "stealth-gained")
    );
    assert_eq!(unit_named(&session, &bound_id)["stealthed"], false);
    south_turn(&mut session, None);

    draw_spell(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-teleport"
            && descriptor["ally"]["instanceId"] == bound_id
            && descriptor["targetLocation"]["cell"] == "C4"
    });
    assert_eq!(unit_named(&session, &bound_id)["location"], "C4");
    assert_eq!(unit_named(&session, &bound_id)["stealthed"], false);
    let (_, restored) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        event_types(&restored)
            .iter()
            .any(|kind| kind == "stealth-gained")
    );
    assert_eq!(unit_named(&session, &bound_id)["stealthed"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0165_waterbound_genesis_still_draws_after_summoning() {
    let manifest = composed_manifest(
        165,
        waterbound_extra(json!({ "genesisDrawSpells": 1 })),
        json!({}),
        &["north-waterbound"; 8],
        &["south-plain"; 8],
    );
    let mut session = opening(&manifest);
    play_site(&mut session, "north-water", "C4");
    let (_, summoned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["region"].is_null()
    });
    assert_eq!(event_types(&summoned), ["minion-summoned", "spell-drawn"]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0166_waterbound_stealth_token_enters_stealthed_on_water() {
    let mut water = site(true);
    water["genesisPayOneManaToSummonToken"] = json!("bound-scout");
    let manifest = composed_manifest(
        166,
        waterbound("submerge"),
        json!({
            "north-water": water,
            "bound-scout": json!({
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "stealth": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                "token": true,
                "waterbound": true,
            }),
        }),
        &["north-waterbound"; 8],
        &["south-plain"; 8],
    );
    let mut session = opening(&manifest);
    let (_, played) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-water"
            && descriptor["cell"] == "C4"
            && descriptor["genesisTokenChoice"] == "pay-one-mana"
    });
    assert_eq!(event_types(&played), ["site-played", "minion-summoned"]);
    let token = units(&session)
        .into_iter()
        .find(|unit| unit["cardId"] == "bound-scout")
        .expect("Genesis token");
    assert_eq!(token["stealthed"], true);
    assert_eq!(token["source"], "token");
    assert_eq!(token["location"], "C4");
    assert_exact_replay(&session);
}
