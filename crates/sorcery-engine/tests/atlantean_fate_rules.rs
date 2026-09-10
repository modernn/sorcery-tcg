//! Direct proofs for official Atlantean Fate (RULE-CATALOG-0310–0311).
//!
//! Fate is not Flood. Affected non-Ordinary sites become Water sites, provide
//! only Water threshold, and lose printed abilities. Ordinary sites in the same
//! 2×2 stay printed. Genesis submerges minions atop affected sites.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
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

fn earth_site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"] })
}

fn ordinary_earth_site() -> Value {
    json!({ "cardType": "site", "elements": ["earth"], "ordinary": true })
}

fn fate() -> Value {
    json!({
        "affectedNonOrdinarySitesAreFloodedProvideOnlyWaterAndLoseOtherAbilities": true,
        "cardType": "aura",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(north_spell: &str, south_spell: &str, north_site: &str) -> String {
    let north_card = match north_spell {
        "north-fate" => fate(),
        "north-minion" => minion(),
        _ => panic!("unsupported north spell {north_spell}"),
    };
    let south_card = match south_spell {
        "south-fate" => fate(),
        "south-minion" => minion(),
        _ => panic!("unsupported south spell {south_spell}"),
    };
    let north_site_card = match north_site {
        "north-site" => earth_site(),
        "north-ordinary-site" => ordinary_earth_site(),
        _ => panic!("unsupported north site {north_site}"),
    };
    let mut cards = json!({
        "north-avatar": avatar(),
        "south-avatar": avatar(),
        "south-site": earth_site(),
    });
    cards[north_spell] = north_card;
    cards[south_spell] = south_card;
    cards[north_site] = north_site_card;
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "atlantean-fate" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-atlantean-fate-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec![north_site; 6],
                "avatar": "north-avatar",
                "spellbook": vec![north_spell; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec![south_spell; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .expect("expected engine-issued action");
    let descriptor = action.descriptor.clone();
    let result = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative step");
    let StepResult::Accepted(receipt) = result else {
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn north_affinity(session: &Session) -> (u64, u64) {
    let view = session.public_view(Seat::North).expect("North public view");
    (
        view["players"]["north"]["affinity"]["earth"]
            .as_u64()
            .expect("earth affinity"),
        view["players"]["north"]["affinity"]["water"]
            .as_u64()
            .expect("water affinity"),
    )
}

fn cells_include(descriptor: &Value, cell: &str) -> bool {
    descriptor["cells"]
        .as_array()
        .is_some_and(|cells| cells.len() == 4 && cells.iter().any(|value| value == cell))
}

fn cast_covering(session: &mut Session, card_id: &str, cell: &str) -> (Value, Receipt) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == card_id
            && cells_include(descriptor, cell)
    })
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<_> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert_eq!(replayed.transcript(), session.transcript());
    assert!(session.verify_replay().expect("verified replay"));
}

fn after_mulligans(north_spell: &str, south_spell: &str, north_site: &str) -> Session {
    let mut session =
        Session::new(&manifest(north_spell, south_spell, north_site)).expect("Fate Aura");
    keep(&mut session);
    keep(&mut session);
    session
}

#[test]
fn rule_catalog_0310_fate_makes_non_ordinary_sites_water_only_and_spares_ordinary() {
    let mut session = after_mulligans("north-fate", "south-minion", "north-site");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(north_affinity(&session), (1, 0));
    let fate_casts = session
        .legal_actions()
        .expect("Fate casts")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-aura" && action.descriptor["cardId"] == "north-fate"
        })
        .collect::<Vec<_>>();
    assert!(
        fate_casts.iter().all(|action| action.descriptor["cells"]
            .as_array()
            .is_some_and(|cells| cells.len() == 4)),
        "Fate uses the default 2×2 footprint"
    );
    let (cast, _) = cast_covering(&mut session, "north-fate", "C4");
    assert!(cells_include(&cast, "C4"));
    assert_eq!(
        north_affinity(&session),
        (0, 1),
        "Fate strips other affinities; Flood would have kept Earth"
    );
    let after = state(&session);
    assert!(after["realm"].get("immobileAreas").is_none());
    assert_eq!(after["realm"]["auras"][0]["cardId"], "north-fate");
    assert_eq!(after["realm"]["auras"][0]["turnCounters"], 0);

    let mut ordinary = after_mulligans("north-fate", "south-minion", "north-ordinary-site");
    accept_where(&mut ordinary, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert_eq!(north_affinity(&ordinary), (1, 0));
    cast_covering(&mut ordinary, "north-fate", "C4");
    assert_eq!(
        north_affinity(&ordinary),
        (1, 0),
        "Fate does not flood Ordinary sites"
    );
    assert_exact_replay(&session);
    assert_exact_replay(&ordinary);
}

#[test]
fn rule_catalog_0311_fate_genesis_submerges_and_kills_a_minion_without_submerge() {
    let mut session = after_mulligans("north-minion", "south-fate", "north-site");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-minion"
            && descriptor["cell"] == "C4"
    });
    let before = state(&session);
    assert_eq!(before["realm"]["units"].as_array().expect("units").len(), 1);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    let (_, receipt) = cast_covering(&mut session, "south-fate", "C4");
    let types: Vec<_> = receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    assert!(
        types.contains(&"aura-conjured"),
        "Fate still announces the Aura: {types:?}"
    );
    assert!(
        types.contains(&"minion-submerged"),
        "Genesis submerges occupants of affected sites: {types:?}"
    );
    let after = state(&session);
    assert!(
        after["realm"]["units"].as_array().is_none_or(Vec::is_empty),
        "a minion without Submerge dies once Fate puts it underwater"
    );
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .map_or(0, Vec::len),
        1,
        "the drowned minion enters its owner's cemetery"
    );
    assert_exact_replay(&session);
}
