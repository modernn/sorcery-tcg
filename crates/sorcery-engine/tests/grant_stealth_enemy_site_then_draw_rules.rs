//! Direct proofs for grant-Stealth-to-an-allied-minion occupying an enemy
//! site then draw-spell Magic (RULE-CATALOG-0541–0542).
//!
//! Ordinary Magic can give Stealth to one allied minion that occupies an
//! enemy-controlled site and then draw one spell. Allies on friendly sites,
//! Avatars, and enemy minions are not offered. When no allied minion occupies
//! an enemy site, the grant is a paid no-op that still draws.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
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

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn grounded() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn fade() -> Value {
    json!({
        "cardType": "magic",
        "grantStealthToAlliedMinionOccupyingEnemySiteThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn fade_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-stealth-enemy-site-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-stealth-enemy-site-then-draw-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-fade": fade(),
            "north-raider": raider(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": grounded(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-fade",
                    "north-fade",
                    "north-raider",
                    "north-raider",
                    "north-raider",
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
        "seed": seed,
    }))
}

fn try_accept_where(
    session: &mut Session,
    predicate: impl Fn(&Value) -> bool,
) -> Option<(Value, Receipt)> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some((descriptor, receipt))
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    try_accept_where(session, predicate).expect("expected engine-issued action")
}

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid grant-stealth-enemy-site session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

fn opening_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("opening Spellbook hand")
        .iter()
        .map(|card| {
            card["cardId"]
                .as_str()
                .expect("hand card identity")
                .to_owned()
        })
        .collect()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn avatar_instance_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar instance identity")
        .to_owned()
}

fn fade_ally_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("fade actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-fade"
        })
        .filter_map(|action| {
            action.descriptor["ally"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn seed_with(required: &[&str]) -> String {
    (541..541 + 256)
        .map(fade_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required Fade opening cards")
}

fn south_plays_c1(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("enemy identity")
        .to_owned()
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

#[test]
fn rule_catalog_0541_fade_offers_only_an_ally_occupying_an_enemy_site() {
    let encoded = seed_with(&["north-ally", "north-fade", "north-raider"]);
    let mut session = opening_main(&encoded);
    let (home, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let home_id = home["cardInstanceId"]
        .as_str()
        .expect("home ally identity")
        .to_owned();
    let enemy_id = south_plays_c1(&mut session);
    let (raid, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-raider"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    let raid_id = raid["cardInstanceId"]
        .as_str()
        .expect("raid ally identity")
        .to_owned();
    let before = state(&session);
    let north_avatar = avatar_instance_id(&before, "north");
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let offered = fade_ally_ids(&session);
    assert!(offered.contains(&raid_id));
    assert!(!offered.contains(&home_id));
    assert!(!offered.contains(&enemy_id));
    assert!(!offered.contains(&north_avatar));
    assert_eq!(unit(&before, &raid_id)["stealthed"], false);

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fade"
            && descriptor["ally"]["instanceId"] == raid_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "minion-stealthed",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], raid_id);
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert_eq!(unit(&after, &raid_id)["stealthed"], true);
    assert_eq!(unit(&after, &home_id)["stealthed"], false);
    assert_eq!(unit(&after, &enemy_id)["stealthed"], false);
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert!(
        !serde_json::to_string(&south_view)
            .expect("view JSON")
            .contains(&library_top)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0542_fade_still_draws_when_no_ally_occupies_an_enemy_site() {
    let encoded = seed_with(&["north-ally", "north-fade"]);
    let mut session = opening_main(&encoded);
    let (home, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let home_id = home["cardInstanceId"]
        .as_str()
        .expect("home ally identity")
        .to_owned();
    south_plays_c1(&mut session);
    let before = state(&session);
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    assert_eq!(fade_ally_ids(&session), [] as [String; 0]);
    assert_eq!(unit(&before, &home_id)["stealthed"], false);

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-fade"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    assert_eq!(unit(&state(&session), &home_id)["stealthed"], false);
    assert!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(&session);
}
