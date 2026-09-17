//! Direct proofs for burrow-all-minions-and-artifacts-at-target-land-site
//! Magic (RULE-CATALOG-0587–0588).
//!
//! Ordinary Magic offers each surface land site (or rubble) and burrows
//! every surface minion there in canonical instance-id order. A water-only
//! site has no Underground layer, so it is not offered.

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

fn earth_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn water_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["water"],
    })
}

fn burrower() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn cave_in() -> Value {
    json!({
        "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn cave_in_manifest(seed: u32, water_south: bool) -> String {
    let south_site = if water_south {
        water_site()
    } else {
        earth_site()
    };
    let fixture = if water_south {
        "cave-in-water-site"
    } else {
        "cave-in-land-site"
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-cave-in": cave_in(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": burrower(),
            "south-site": south_site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-cave-in"; 6],
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
    let mut session = Session::new(encoded).expect("valid Cave-In session");
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

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
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

fn seed_with(water_south: bool, start: u32, required_south: usize) -> String {
    (start..start + 256)
        .map(|seed| cave_in_manifest(seed, water_south))
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-cave-in")
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Cave-In and required South minions")
}

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn summon_south_at_c1(session: &mut Session) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Cave-In occupant identity")
        .to_owned()
}

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn cave_in_casts(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("Cave-In actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-cave-in"
        })
        .map(|action| action.descriptor)
        .collect()
}

fn cave_in_cells(session: &Session) -> Vec<String> {
    let mut cells: Vec<String> = cave_in_casts(session)
        .into_iter()
        .filter_map(|cast| {
            cast["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    cells.sort();
    cells.dedup();
    cells
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
fn rule_catalog_0587_cave_in_burrows_minions_at_land_site_in_canonical_order() {
    let encoded = seed_with(false, 587, 2);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    let first_id = summon_south_at_c1(&mut session);
    let second_id = summon_south_at_c1(&mut session);
    north_draws_spellbook(&mut session);

    let land_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("Land Site identity")
        .to_owned();
    let offered = cave_in_casts(&session);
    assert!(offered.iter().any(|cast| {
        cast["targetLocation"]["cell"] == "C1" && cast["targetSiteInstanceId"] == land_site_id
    }));
    assert!(
        offered
            .iter()
            .all(|cast| { cast["targetLocation"]["region"] == "surface" })
    );

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == land_site_id
    });
    let mut burrowed_ids = vec![first_id.clone(), second_id.clone()];
    burrowed_ids.sort();
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-burrowed",
            "minion-burrowed",
            "magic-resolved",
        ]
    );
    assert_eq!(
        receipt.events[0].payload["targetLocation"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert_eq!(
        receipt.events[0].payload["targetSiteInstanceId"],
        land_site_id
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "minion-burrowed")
            .map(|event| {
                assert_eq!(event.payload["cell"], "C1");
                assert_eq!(event.payload["seat"], "south");
                assert_eq!(event.payload["sourceInstanceId"], cast["cardInstanceId"]);
                event.payload["instanceId"]
                    .as_str()
                    .expect("burrowed identity")
                    .to_owned()
            })
            .collect::<Vec<_>>(),
        burrowed_ids
    );
    let after = state(&session);
    assert_eq!(unit(&after, &first_id)["region"], "underground");
    assert_eq!(unit(&after, &second_id)["region"], "underground");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0588_cave_in_does_not_offer_a_water_only_site() {
    let encoded = seed_with(true, 588, 0);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    north_draws_spellbook(&mut session);

    let cells = cave_in_cells(&session);
    assert!(cells.contains(&"C4".to_owned()));
    assert!(!cells.contains(&"C1".to_owned()));
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["cardId"],
        "south-site"
    );
    assert_exact_replay(&session);
}
