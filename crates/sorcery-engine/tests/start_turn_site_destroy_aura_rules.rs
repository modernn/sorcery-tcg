//! Direct proofs for start-turn occupied-site Aura destruction (RULE-CATALOG-0258–0259,
//! RULE-CATALOG-1241).
//!
//! Official cards such as Hamlet's Ablaze conjure atop an Ordinary or Exceptional site.
//! At the start of the controller's next turn the Aura destroys that site, the minions
//! standing atop it, and itself. Avatars are not minions. Unique or Legendary sites
//! cannot be targeted.

use serde_json::{Value, json};
use sorcery_engine::canonical::identity_hash;
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::game::Game;
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

fn unique_site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
        "uniqueOrLegendary": true,
    })
}

fn aura() -> Value {
    json!({
        "atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf": true,
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
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn manifest(unique_south: bool) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-site-destroy-aura" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-site-destroy-aura-v1",
        },
        "cards": {
            "north-aura": aura(),
            "north-avatar": avatar(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": if unique_south { unique_site() } else { site() },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-aura"; 6],
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
        "seed": 1,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn site_ward_manifest(seed: u32) -> String {
    let mut value: Value = serde_json::from_str(&manifest(false)).unwrap();
    value.as_object_mut().unwrap().remove("manifestId");
    value["seed"] = json!(seed);
    value["cards"]["north-bless"] = json!({
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "wardNearbyMinionOrSite": true,
    });
    value["decks"]["north"]["spellbook"] = json!([
        "north-aura",
        "north-aura",
        "north-aura",
        "north-bless",
        "north-bless",
        "north-bless",
    ]);
    value["manifestId"] = json!(identity_hash(&value).expect("protected Site manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical protected Site manifest")
}

fn ordered_deathrite_manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "ordered-site-destroy-deathrites" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-ordered-site-destroy-deathrites-v1",
        },
        "cards": {
            "north-aura": aura(),
            "north-avatar": avatar(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-aura"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 258,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

fn ordered_aura_custody_manifest(seed: u32, aura_seat: &str) -> String {
    let mut value: Value = serde_json::from_str(&ordered_deathrite_manifest())
        .expect("base ordered Aura custody manifest");
    value.as_object_mut().unwrap().remove("manifestId");
    value["seed"] = json!(seed);
    value["cards"]["north-source"] = json!({
        "attack": 1,
        "atStartOfControllerTurnDrawSites": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["cards"]["south-artifact"] = json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["decks"]["north"]["spellbook"] = json!([
        "north-source",
        "north-source",
        "north-source",
        "north-aura",
        "north-aura",
        "north-aura",
    ]);
    value["decks"]["south"]["spellbook"] = json!([
        "south-deathrite",
        "south-deathrite",
        "south-deathrite",
        "south-artifact",
        "south-artifact",
        "south-artifact",
    ]);
    if aura_seat == "south" {
        value = mirror_seat_labels(value);
    }
    value["manifestId"] = json!(identity_hash(&value).expect("Aura custody manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical Aura custody manifest")
}

fn ordered_aura_source_control_manifest(seed: u32, aura_seat: &str) -> String {
    let mut value: Value = serde_json::from_str(&ordered_deathrite_manifest())
        .expect("base ordered Aura source-control manifest");
    value.as_object_mut().unwrap().remove("manifestId");
    value["seed"] = json!(seed);
    value["cards"]["north-source"] = json!({
        "attack": 1,
        "atStartOfControllerTurnDrawSites": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["cards"]["north-witness"] = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    });
    value["cards"]["south-puppet"] = json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 2,
        "genesisGainControlOfTappedMinionsHereUntilThisLeaves": true,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["cards"]["south-artifact"] = json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["decks"]["north"]["spellbook"] = json!([
        "north-source",
        "north-witness",
        "north-aura",
        "north-source",
        "north-aura",
        "north-aura",
    ]);
    value["decks"]["south"]["spellbook"] = json!([
        "south-deathrite",
        "south-puppet",
        "south-artifact",
        "south-deathrite",
        "south-puppet",
        "south-artifact",
    ]);
    if aura_seat == "south" {
        value = mirror_seat_labels(value);
    }
    value["manifestId"] =
        json!(identity_hash(&value).expect("Aura source-control manifest identity"));
    sorcery_engine::canonical::canonical_json(&value)
        .expect("canonical Aura source-control manifest")
}

fn ordered_aura_empty_atlas_manifest(seed: u32, aura_seat: &str) -> String {
    let mut value: Value = serde_json::from_str(&ordered_aura_custody_manifest(seed, aura_seat))
        .expect("base ordered Aura fixture");
    value.as_object_mut().unwrap().remove("manifestId");
    let opponent = if aura_seat == "north" {
        "south"
    } else {
        "north"
    };
    value["decks"][opponent]["atlas"] = json!(vec![format!("{opponent}-site"); 3]);
    let mut spellbook = vec![format!("{opponent}-deathrite"); 5];
    spellbook.push(format!("{opponent}-artifact"));
    value["decks"][opponent]["spellbook"] = json!(spellbook);
    value["manifestId"] = json!(identity_hash(&value).expect("empty-Atlas Aura identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical empty-Atlas Aura fixture")
}

fn ordered_aura_water_region_manifest(seed: u32, aura_seat: &str) -> String {
    let mut value: Value = serde_json::from_str(&ordered_aura_custody_manifest(seed, aura_seat))
        .expect("base ordered Aura regional fixture");
    value.as_object_mut().unwrap().remove("manifestId");
    let opponent = if aura_seat == "north" {
        "south"
    } else {
        "north"
    };
    value["cards"][format!("{aura_seat}-site")]["elements"] = json!(["earth", "water"]);
    let deathrite = format!("{opponent}-deathrite");
    value["cards"][&deathrite]["submerge"] = json!(true);
    value["cards"][&deathrite]["ward"] = json!(true);
    value["cards"][format!("{opponent}-surface")] = json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["cards"][format!("{opponent}-dual")] = json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "submerge": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "takesLessDamage": 1,
    });
    value["cards"][format!("{opponent}-artifact")] = json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["decks"][opponent]["spellbook"] = json!([
        deathrite,
        format!("{opponent}-surface"),
        format!("{opponent}-dual"),
        format!("{opponent}-artifact"),
        format!("{opponent}-artifact"),
        format!("{opponent}-deathrite"),
    ]);
    value["manifestId"] = json!(identity_hash(&value).expect("Aura regional identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical Aura regional fixture")
}

fn mirror_seat_labels(value: Value) -> Value {
    match value {
        Value::String(value) => Value::String(match value.as_str() {
            "north" => "south".to_owned(),
            "south" => "north".to_owned(),
            _ if value.starts_with("north-") => value.replacen("north-", "south-", 1),
            _ if value.starts_with("south-") => value.replacen("south-", "north-", 1),
            _ => value,
        }),
        Value::Array(values) => Value::Array(values.into_iter().map(mirror_seat_labels).collect()),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| {
                    let mirrored_key = match key.as_str() {
                        "north" => "south".to_owned(),
                        "south" => "north".to_owned(),
                        _ if key.starts_with("north-") => key.replacen("north-", "south-", 1),
                        _ if key.starts_with("south-") => key.replacen("south-", "north-", 1),
                        _ => key,
                    };
                    (mirrored_key, mirror_seat_labels(value))
                })
                .collect(),
        ),
        value => value,
    }
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

fn draw_spellbook_if_offered(session: &mut Session) -> Option<Receipt> {
    session
        .legal_actions()
        .ok()?
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "draw" && action.descriptor["zone"] == "spellbook"
        })
        .then(|| accept_where(session, |d| d["kind"] == "draw" && d["zone"] == "spellbook").1)
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("session value")["state"].clone()
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn aura_id(session: &Session) -> Value {
    state(session)["realm"]["auras"][0]["instanceId"].clone()
}

fn replay_game_for_session(session: &Session) -> Game {
    let mut game = Game::from_manifest_json(session.manifest_json()).expect("Game fixture");
    for receipt in session.transcript() {
        let action = game
            .legal_actions()
            .expect("Ignore-path legal actions")
            .into_iter()
            .find(|action| {
                action
                    .to_legal_action()
                    .expect("materialized Ignore-path action")
                    .action_id
                    == receipt.action_id
            })
            .expect("Session-issued action exists in Ignore path");
        game.apply_action(&action)
            .expect("Ignore-path setup action");
    }
    game
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

fn full_session_fingerprint(session: &Session) -> Value {
    let checkpoint = create_game_checkpoint(session).expect("complete Session checkpoint");
    json!({
        "checkpoint": serialize_game_checkpoint(&checkpoint).expect("checkpoint bytes"),
        "stateHash": session.state_hash().expect("state hash"),
        "sessionHash": session.session_hash().expect("session hash"),
        "replay": session.replay_value().expect("replay envelope"),
        "transcript": session.transcript(),
        "northView": session.public_view(sorcery_engine::contract::Seat::North).unwrap(),
        "southView": session.public_view(sorcery_engine::contract::Seat::South).unwrap(),
        "legalActions": serde_json::to_value(session.legal_actions().unwrap()).unwrap(),
    })
}

fn all_warded_manifest() -> String {
    let mut value: Value = serde_json::from_str(&manifest(false)).expect("base fixture manifest");
    let mut north_minion = minion();
    north_minion["ward"] = json!(true);
    value["cards"]["north-ward-minion"] = north_minion;
    value["cards"]["south-minion"]["ward"] = json!(true);
    value["decks"]["north"]["spellbook"] = json!([
        "north-ward-minion",
        "north-ward-minion",
        "north-ward-minion",
        "north-aura",
        "north-aura",
        "north-aura",
    ]);
    value["seed"] = json!(1);
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).expect("warded fixture manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical warded manifest")
}

fn protection_lost_manifest() -> String {
    let mut value: Value = serde_json::from_str(&all_warded_manifest()).unwrap();
    value["cards"]["north-freeze"] = json!({
        "cardType": "magic",
        "disableTargetNearbyMinionUntilNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    value["decks"]["north"]["spellbook"] = json!([
        "north-aura",
        "north-ward-minion",
        "north-aura",
        "north-aura",
        "north-aura",
        "north-freeze",
    ]);
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).expect("protection-lost manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).unwrap()
}

fn after_aura_disabling_its_warded_minion() -> Session {
    let encoded = protection_lost_manifest();
    let mut session = Session::new(&encoded).expect("valid protection-lost session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (_, summon) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ward-minion"
            && descriptor["cell"] == "C4"
    });
    let minion_id = summon
        .events
        .iter()
        .find(|event| event.event_type == "minion-summoned")
        .expect("issued protected minion summon")
        .payload["instanceId"]
        .clone();
    let (_, disabled) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-freeze"
            && descriptor["target"]["instanceId"] == minion_id
    });
    assert!(disabled.events.iter().any(|event| {
        event.event_type == "minion-disabled" && event.payload["wardRemoved"] == true
    }));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn after_aura_with_warded_minion_on_c4() -> Session {
    let encoded = all_warded_manifest();
    after_aura_with_manifest(&encoded)
}

fn after_aura_with_manifest(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid protected-recipient session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ward-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn after_aura_on_c4(unique_south: bool) -> Session {
    let mut session =
        Session::new(&manifest(unique_south)).expect("valid site-destroy aura session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    session
}

fn resolve_start_turn_destroy(session: &mut Session, source_id: &Value) -> Receipt {
    assert_eq!(state(session)["phase"], "start-turn");
    let legal = session
        .legal_actions()
        .expect("start-turn site-destroy actions");
    assert!(
        legal.iter().all(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == *source_id
                && action.descriptor.get("lureTargetInstanceId").is_none()
        }),
        "the occupied-site Aura is the only start-turn source"
    );
    accept_where(session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == *source_id
    })
    .1
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keeps the original occupied-Site trigger event and checkpoint proof together"
)]
fn rule_catalog_0258_start_turn_site_minion_destruction_completes_before_turn_draw() {
    let mut session = after_aura_on_c4(false);
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let before = state(&session);
    let site_id = before["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("occupied Ordinary Site identity")
        .to_owned();
    let victim_id = before["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["controller"] == "south")
        .expect("South minion atop the site")["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("issued occupied-site Aura trigger");
    let parent = session.clone();
    let parent_fingerprint = full_session_fingerprint(&parent);
    let parent_checkpoint = create_game_checkpoint(&session).expect("pre-Aura checkpoint");
    let parent_checkpoint = parse_game_checkpoint(
        &serialize_game_checkpoint(&parent_checkpoint).expect("serialize pre-Aura checkpoint"),
    )
    .expect("parse pre-Aura checkpoint");
    let result = session.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    });
    let StepResult::Accepted(receipt) = result.expect("mixed Site/Aura trigger completes") else {
        panic!("engine-issued start-turn trigger must be accepted");
    };
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "aura-dispelled",
            "site-destroyed",
            "rubble-created",
            "minion-died"
        ]
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "aura-dispelled"
                && event.payload["instanceId"] == source_id)
            .count(),
        1
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "site-destroyed"
                && event.payload["instanceId"] == site_id)
            .count(),
        1
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "minion-died"
                && event.payload["instanceId"] == victim_id)
            .count(),
        1
    );
    let mut restored_parent =
        resume_game_checkpoint(&parent_checkpoint).expect("restore pre-Aura parent");
    let restored = restored_parent.step(ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    });
    let StepResult::Accepted(restored_receipt) = restored.expect("replay issued Aura action")
    else {
        panic!("restored issued start-turn trigger must be accepted");
    };
    assert_eq!(restored_receipt, receipt);
    assert_eq!(
        full_session_fingerprint(&restored_parent),
        full_session_fingerprint(&session)
    );
    assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C4"]["rubble"], true);
    for seat in ["north", "south"] {
        assert!(before["players"][seat]["avatar"].is_object());
        assert!(after["players"][seat]["avatar"].is_object());
        assert_eq!(
            after["players"][seat]["avatar"],
            before["players"][seat]["avatar"]
        );
    }
    assert!(
        after["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .all(|unit| unit["instanceId"] != victim_id)
    );
    let north_cemetery = after["players"]["north"]["cemetery"].as_array().unwrap();
    assert_eq!(
        north_cemetery
            .iter()
            .filter(|card| card["instanceId"] == source_id)
            .count(),
        1
    );
    assert_eq!(
        north_cemetery
            .iter()
            .filter(|card| card["instanceId"] == site_id)
            .count(),
        1
    );
    assert_eq!(
        after["players"]["south"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|card| card["instanceId"] == victim_id)
            .count(),
        1
    );
    let rubble_id = after["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("new Rubble identity");
    assert_ne!(rubble_id, site_id);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "rubble-created"
                && event.payload["instanceId"] == rubble_id)
            .count(),
        1
    );
    assert_eq!(after["phase"], "draw");
    assert!(receipt.events.iter().all(|event| {
        !matches!(
            event.event_type.as_str(),
            "site-drawn" | "spell-drawn" | "draw"
        )
    }));
    let completion_fingerprint = full_session_fingerprint(&session);
    let completion_checkpoint =
        create_game_checkpoint(&session).expect("completed occupied-Site Aura checkpoint");
    let completion_checkpoint = parse_game_checkpoint(
        &serialize_game_checkpoint(&completion_checkpoint).expect("serialize completion"),
    )
    .expect("parse completion checkpoint");
    let restored_completion =
        resume_game_checkpoint(&completion_checkpoint).expect("restore completed Aura");
    assert_eq!(
        full_session_fingerprint(&restored_completion),
        completion_fingerprint
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keeps the start-turn continuation checkpoint and order branches together"
)]
fn start_turn_site_destruction_holds_completion_through_ordered_deathrites() {
    let encoded = ordered_deathrite_manifest();
    let mut session = Session::new(&encoded).expect("ordered deathrite fixture");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura" && descriptor["cardId"] == "north-aura"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let first = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
    });
    let second = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
    });
    let source_ids = [
        first.0["cardInstanceId"].as_str().unwrap().to_owned(),
        second.0["cardInstanceId"].as_str().unwrap().to_owned(),
    ];
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let source_id = aura_id(&session);
    let before = state(&session);
    let site_id = before["realm"]["sites"]["C4"]["instanceId"].clone();
    let before_checkpoint = create_game_checkpoint(&session).expect("before trigger checkpoint");
    let before_checkpoint = serialize_game_checkpoint(&before_checkpoint).unwrap();
    let before_checkpoint = parse_game_checkpoint(&before_checkpoint).unwrap();
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-dispelled")
    );
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed")
    );
    let pause = state(&session);
    assert_eq!(pause["phase"], "trigger-order");
    let pending = replay_game_for_session(&session).authoritative_state();
    assert_eq!(
        pending["pendingDeathrites"]["continuation"]["kind"],
        "start-turn-trigger-complete"
    );
    assert_eq!(
        pending["pendingDeathrites"]["continuation"]["sourceInstanceId"],
        source_id
    );
    assert_eq!(
        pause["realm"]["sites"]["C4"]["rubble"].as_bool(),
        Some(true)
    );
    assert!(
        pause["players"]["north"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["instanceId"] == source_id)
    );
    assert!(
        pause["players"]["north"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["instanceId"] == site_id)
    );
    for source in &source_ids {
        let unit = pause["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["instanceId"] == *source)
            .expect("Deathrite remains in realm during its trigger");
        assert_eq!(unit["deathMarked"], true);
        assert!(
            !pause["players"]["south"]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| card["instanceId"] == *source)
        );
    }
    assert_eq!(session.legal_actions().unwrap().len(), 2);
    let pause_checkpoint = create_game_checkpoint(&session).expect("pause checkpoint");
    let pause_checkpoint = serialize_game_checkpoint(&pause_checkpoint).unwrap();
    let pause_checkpoint = parse_game_checkpoint(&pause_checkpoint).unwrap();
    let parent_fingerprint = full_session_fingerprint(&session);
    for first_source in &source_ids {
        let mut branch = resume_game_checkpoint(&pause_checkpoint).expect("restored pause branch");
        accept_where(&mut branch, |descriptor| {
            descriptor["kind"] == "order-triggers"
                && descriptor["sourceInstanceId"] == *first_source
        });
        while state(&branch)["phase"] == "trigger-order" {
            accept_where(&mut branch, |descriptor| {
                descriptor["kind"] == "order-triggers"
            });
        }
        let complete = state(&branch);
        for source in &source_ids {
            assert!(
                !complete["realm"]["units"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|unit| unit["instanceId"] == *source)
            );
            assert!(
                complete["players"]["south"]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["instanceId"] == *source)
            );
        }
        assert_eq!(complete["phase"], "draw");
        assert!(branch.legal_actions().unwrap().iter().any(|action| {
            action.descriptor["kind"] == "draw" && action.descriptor["zone"] == "atlas"
        }));
        let after_checkpoint = create_game_checkpoint(&branch).expect("after checkpoint");
        let after_bytes = serialize_game_checkpoint(&after_checkpoint).unwrap();
        let restored_after = parse_game_checkpoint(&after_bytes).unwrap();
        let restored_bytes = serialize_game_checkpoint(&restored_after).unwrap();
        assert_eq!(after_bytes, restored_bytes);
        assert_exact_replay(&branch);
    }
    assert_eq!(full_session_fingerprint(&session), parent_fingerprint);
    assert!(resume_game_checkpoint(&before_checkpoint).is_ok());
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "proves an issued Aura retains only surviving start sources across held Deathrites"
)]
fn aura_deathrite_order_keeps_survivor_source_and_drops_departed_source_and_carry() {
    for aura_seat in ["north", "south"] {
        let opponent = if aura_seat == "north" {
            "south"
        } else {
            "north"
        };
        let source_card = format!("{aura_seat}-source");
        let aura_card = format!("{aura_seat}-aura");
        let deathrite_card = format!("{opponent}-deathrite");
        let artifact_card = format!("{opponent}-artifact");
        let aura_cell = if aura_seat == "north" { "C4" } else { "C1" };
        let opponent_cell = if aura_seat == "north" { "C1" } else { "C4" };
        let surviving_cell = if aura_seat == "north" { "C3" } else { "C2" };
        let seed = (258..1282)
            .find(|seed| {
                let encoded = ordered_aura_custody_manifest(*seed, aura_seat);
                let Ok(mut candidate) = Session::new(&encoded) else {
                    return false;
                };
                keep(&mut candidate);
                keep(&mut candidate);
                state(&candidate)["players"][aura_seat]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["cardId"] == source_card)
            })
            .expect("bounded opening with a start-source Minion");
        let encoded = ordered_aura_custody_manifest(seed, aura_seat);
        let mut session = Session::new(&encoded).expect("Aura custody fixture");
        keep(&mut session);
        keep(&mut session);
        accept_where(&mut session, |d| {
            d["kind"] == "play-site" && d["cell"] == aura_cell
        });
        let (first_source, _) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion" && d["cardId"] == source_card && d["cell"] == aura_cell
        });
        let first_source_id = first_source["cardInstanceId"].as_str().unwrap().to_owned();
        accept_where(&mut session, |d| d["kind"] == "end-turn");
        accept_where(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "spellbook"
        });
        accept_where(&mut session, |d| {
            d["kind"] == "play-site" && d["cell"] == opponent_cell
        });
        let (deathrite, _) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion" && d["cardId"] == deathrite_card && d["cell"] == aura_cell
        });
        let deathrite_id = deathrite["cardInstanceId"].as_str().unwrap().to_owned();
        let (second_deathrite, _) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion" && d["cardId"] == deathrite_card && d["cell"] == aura_cell
        });
        let second_deathrite_id = second_deathrite["cardInstanceId"]
            .as_str()
            .unwrap()
            .to_owned();
        let (artifact, _) = accept_where(&mut session, |d| {
            d["kind"] == "cast-artifact"
                && d["cardId"] == artifact_card
                && d["bearer"]["instanceId"] == deathrite_id
        });
        let artifact_id = artifact["cardInstanceId"].as_str().unwrap().to_owned();
        accept_where(&mut session, |d| d["kind"] == "end-turn");

        let (_, first_start) = accept_where(&mut session, |d| {
            d["kind"] == "resolve-start-turn-trigger" && d["sourceInstanceId"] == first_source_id
        });
        assert!(
            first_start
                .events
                .iter()
                .any(|event| event.event_type == "site-drawn")
        );
        accept_where(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "spellbook"
        });
        accept_where(&mut session, |d| {
            d["kind"] == "play-site" && d["cell"] == surviving_cell
        });
        let (surviving_source, _) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion"
                && d["cardId"] == source_card
                && d["cell"] == surviving_cell
        });
        let surviving_source_id = surviving_source["cardInstanceId"]
            .as_str()
            .unwrap()
            .to_owned();
        accept_where(&mut session, |d| {
            d["kind"] == "cast-aura" && d["cardId"] == aura_card && d["cells"] == json!([aura_cell])
        });
        accept_where(&mut session, |d| d["kind"] == "end-turn");
        accept_where(&mut session, |d| {
            d["kind"] == "draw" && d["zone"] == "spellbook"
        });
        accept_where(&mut session, |d| d["kind"] == "end-turn");

        let source_id = aura_id(&session);
        let parent = session.clone();
        let parent_fingerprint = full_session_fingerprint(&parent);
        let before_checkpoint = create_game_checkpoint(&session).expect("pre-trigger checkpoint");
        let before_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&before_checkpoint).unwrap()).unwrap();
        let trigger = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == source_id
            })
            .expect("Aura trigger remains an issued action")
            .descriptor;
        let (_, receipt) = accept_where(&mut session, |d| d == &trigger);
        assert!(
            receipt
                .events
                .iter()
                .any(|event| event.event_type == "aura-dispelled")
        );
        assert!(
            receipt
                .events
                .iter()
                .any(|event| event.event_type == "site-destroyed")
        );
        let pause = state(&session);
        assert_eq!(
            pause["phase"],
            "trigger-order",
            "state={pause} actions={:?}",
            session.legal_actions().unwrap()
        );
        assert_eq!(
            pause["pendingDeathrites"]["continuation"]["sourceInstanceId"],
            source_id
        );
        assert_eq!(
            realm_unit(&pause, &first_source_id).unwrap()["deathMarked"],
            true
        );
        assert_eq!(
            realm_unit(&pause, &deathrite_id).unwrap()["deathMarked"],
            true
        );
        assert_eq!(
            realm_unit(&pause, &second_deathrite_id).unwrap()["deathMarked"],
            true
        );
        assert!(realm_unit(&pause, &surviving_source_id).is_some());
        assert!(
            pause["players"][aura_seat]["cemetery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|card| { card["instanceId"] == source_id })
        );
        assert!(
            pause["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| {
                    item["instanceId"] == artifact_id
                        && item["bearer"]["instanceId"] == deathrite_id
                })
        );
        let pause_checkpoint =
            create_game_checkpoint(&session).expect("Deathrite pause checkpoint");
        let pause_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&pause_checkpoint).unwrap()).unwrap();
        let pause_fingerprint = full_session_fingerprint(&session);
        let legal = session.legal_actions().unwrap();
        assert_eq!(legal.len(), 2);
        assert!(
            legal
                .iter()
                .all(|action| action.descriptor["kind"] == "order-triggers")
        );
        let choices: Vec<_> = legal
            .into_iter()
            .filter_map(|action| {
                (action.descriptor["kind"] == "order-triggers")
                    .then(|| action.descriptor["sourceInstanceId"].clone())
            })
            .collect();
        assert_eq!(choices.len(), 2);
        for first in choices {
            let mut branch =
                resume_game_checkpoint(&pause_checkpoint).expect("restore pause branch");
            accept_where(&mut branch, |d| {
                d["kind"] == "order-triggers" && d["sourceInstanceId"] == first
            });
            while state(&branch)["phase"] == "trigger-order" {
                accept_where(&mut branch, |d| d["kind"] == "order-triggers");
            }
            let complete = state(&branch);
            assert!(realm_unit(&complete, &first_source_id).is_none());
            assert!(realm_unit(&complete, &deathrite_id).is_none());
            assert!(realm_unit(&complete, &second_deathrite_id).is_none());
            assert!(realm_unit(&complete, &surviving_source_id).is_some());
            assert!(
                complete["realm"]["artifacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| {
                        item["instanceId"] == artifact_id
                            && item["bearer"].is_null()
                            && item["location"] == aura_cell
                    })
            );
            assert_eq!(
                branch
                    .legal_actions()
                    .unwrap()
                    .iter()
                    .filter(|a| {
                        a.descriptor["kind"] == "resolve-start-turn-trigger"
                            && a.descriptor["sourceInstanceId"] == surviving_source_id
                    })
                    .count(),
                1
            );
            assert!(!branch.legal_actions().unwrap().iter().any(|a| {
                a.descriptor["kind"] == "resolve-start-turn-trigger"
                    && a.descriptor["sourceInstanceId"] == first_source_id
            }));
            let source_trigger = branch
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|a| {
                    a.descriptor["kind"] == "resolve-start-turn-trigger"
                        && a.descriptor["sourceInstanceId"] == surviving_source_id
                })
                .unwrap()
                .descriptor;
            let parent_after_deathrites = branch.clone();
            let parent_after_deathrites_fingerprint =
                full_session_fingerprint(&parent_after_deathrites);
            let checkpoint = create_game_checkpoint(&branch).expect("survivor trigger checkpoint");
            let checkpoint =
                parse_game_checkpoint(&serialize_game_checkpoint(&checkpoint).unwrap()).unwrap();
            let (_, source_receipt) = accept_where(&mut branch, |d| d == &source_trigger);
            assert_eq!(
                source_receipt
                    .events
                    .iter()
                    .filter(|event| {
                        event.event_type == "site-drawn"
                            && event.payload["sourceInstanceId"] == surviving_source_id
                    })
                    .count(),
                1
            );
            assert_eq!(
                branch
                    .legal_actions()
                    .unwrap()
                    .iter()
                    .filter(|a| {
                        a.descriptor["kind"] == "draw" && a.descriptor["zone"] == "spellbook"
                    })
                    .count(),
                1,
                "ordinary Draw follows the one surviving start-source draw"
            );
            let draw_descriptor = branch
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|a| a.descriptor["kind"] == "draw" && a.descriptor["zone"] == "spellbook")
                .unwrap()
                .descriptor;
            let (_, draw_receipt) = accept_where(&mut branch, |d| d == &draw_descriptor);
            assert_eq!(
                draw_receipt
                    .events
                    .iter()
                    .filter(|event| {
                        event.event_type == "card-drawn" && event.payload["zone"] == "spellbook"
                    })
                    .count(),
                1
            );
            let after_fingerprint = full_session_fingerprint(&branch);
            let mut restored =
                resume_game_checkpoint(&checkpoint).expect("restore surviving source trigger");
            let (_, restored_source_receipt) =
                accept_where(&mut restored, |d| d == &source_trigger);
            assert_eq!(restored_source_receipt, source_receipt);
            let restored_draw = restored
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|a| a.descriptor["kind"] == "draw" && a.descriptor["zone"] == "spellbook")
                .unwrap()
                .descriptor;
            let (_, restored_draw_receipt) = accept_where(&mut restored, |d| d == &restored_draw);
            assert_eq!(restored_draw_receipt, draw_receipt);
            assert_eq!(full_session_fingerprint(&restored), after_fingerprint);
            assert_eq!(
                full_session_fingerprint(&parent_after_deathrites),
                parent_after_deathrites_fingerprint
            );
            assert_exact_replay(&branch);
        }
        assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
        assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
        assert!(resume_game_checkpoint(&before_checkpoint).is_ok());
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "proves issued source-bound control reversion inside a held Aura Deathrite batch"
)]
fn aura_deathrite_order_reverts_genesis_control_when_marked_source_leaves() {
    for aura_seat in ["north", "south"] {
        let opponent = if aura_seat == "north" {
            "south"
        } else {
            "north"
        };
        let first_source_card = format!("{aura_seat}-source");
        let witness_card = format!("{aura_seat}-witness");
        let regular_deathrite_card = format!("{opponent}-deathrite");
        let puppet_card = format!("{opponent}-puppet");
        let artifact_card = format!("{opponent}-artifact");
        let aura_card = format!("{aura_seat}-aura");
        let aura_cell = if aura_seat == "north" { "C4" } else { "C1" };
        let opponent_cell = if aura_seat == "north" { "C1" } else { "C4" };
        let surviving_cell = if aura_seat == "north" { "C3" } else { "C2" };
        let seed = (258..8194)
            .find(|seed| {
                let encoded = ordered_aura_source_control_manifest(*seed, aura_seat);
                let Ok(mut candidate) = Session::new(&encoded) else {
                    return false;
                };
                keep(&mut candidate);
                keep(&mut candidate);
                let snapshot = state(&candidate);
                let hand = &snapshot["players"];
                [
                    (
                        &hand[aura_seat]["hand"]["spellbook"],
                        first_source_card.as_str(),
                    ),
                    (&hand[aura_seat]["hand"]["spellbook"], witness_card.as_str()),
                    (&hand[aura_seat]["hand"]["spellbook"], aura_card.as_str()),
                    (
                        &hand[opponent]["hand"]["spellbook"],
                        regular_deathrite_card.as_str(),
                    ),
                    (&hand[opponent]["hand"]["spellbook"], puppet_card.as_str()),
                    (&hand[opponent]["hand"]["spellbook"], artifact_card.as_str()),
                ]
                .iter()
                .all(|(cards, wanted)| {
                    cards
                        .as_array()
                        .is_some_and(|cards| cards.iter().any(|card| card["cardId"] == *wanted))
                }) && snapshot["players"][aura_seat]["spellbook"][0]["cardId"] == first_source_card
            })
            .expect("bounded opening with the source-control cards in both hands");
        let encoded = ordered_aura_source_control_manifest(seed, aura_seat);
        let mut session = Session::new(&encoded).expect("Aura source-control fixture");
        keep(&mut session);
        keep(&mut session);
        accept_where(&mut session, |d| {
            d["kind"] == "play-site" && d["cell"] == aura_cell
        });
        let (first_source, _) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion"
                && d["cardId"] == first_source_card
                && d["cell"] == aura_cell
        });
        let first_source_id = first_source["cardInstanceId"].as_str().unwrap().to_owned();
        let (witness, _) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion" && d["cardId"] == witness_card && d["cell"] == aura_cell
        });
        let witness_id = witness["cardInstanceId"].as_str().unwrap().to_owned();
        accept_where(&mut session, |d| d["kind"] == "end-turn");

        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |d| {
            d["kind"] == "play-site" && d["cell"] == opponent_cell
        });
        let (regular, _) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion"
                && d["cardId"] == regular_deathrite_card
                && d["cell"] == aura_cell
        });
        let regular_id = regular["cardInstanceId"].as_str().unwrap().to_owned();
        let (artifact, _) = accept_where(&mut session, |d| {
            d["kind"] == "cast-artifact"
                && d["cardId"] == artifact_card
                && d["bearer"]["instanceId"] == regular_id
        });
        let artifact_id = artifact["cardInstanceId"].as_str().unwrap().to_owned();
        accept_where(&mut session, |d| d["kind"] == "end-turn");

        let (_, first_start) = accept_where(&mut session, |d| {
            d["kind"] == "resolve-start-turn-trigger" && d["sourceInstanceId"] == first_source_id
        });
        assert!(
            first_start
                .events
                .iter()
                .any(|event| event.event_type == "site-drawn")
        );
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |d| {
            d["kind"] == "play-site" && d["cell"] == surviving_cell
        });
        let (survivor, _) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion"
                && d["cardId"] == first_source_card
                && d["cell"] == surviving_cell
        });
        let survivor_id = survivor["cardInstanceId"].as_str().unwrap().to_owned();
        let (_, tapped) = accept_where(&mut session, |d| {
            d["kind"] == "activate-mana" && d["unitInstanceId"] == witness_id
        });
        assert!(
            tapped
                .events
                .iter()
                .any(|event| event.event_type == "mana-activated")
        );
        accept_where(&mut session, |d| d["kind"] == "end-turn");

        let _ = draw_spellbook_if_offered(&mut session);
        let (puppet, control) = accept_where(&mut session, |d| {
            d["kind"] == "summon-minion" && d["cardId"] == puppet_card && d["cell"] == aura_cell
        });
        let puppet_id = puppet["cardInstanceId"].as_str().unwrap().to_owned();
        assert_eq!(
            control
                .events
                .iter()
                .filter(|event| {
                    event.event_type == "minion-control-changed"
                        && event.payload["instanceId"] == witness_id
                        && event.payload["fromSeat"] == aura_seat
                        && event.payload["seat"] == opponent
                        && event.payload["sourceInstanceId"] == puppet_id
                })
                .count(),
            1,
            "Genesis steals the exact tapped Ward witness"
        );
        let controlled = state(&session);
        assert_eq!(
            realm_unit(&controlled, &witness_id).unwrap()["owner"],
            aura_seat
        );
        assert_eq!(
            realm_unit(&controlled, &witness_id).unwrap()["controller"],
            opponent
        );
        assert_eq!(
            realm_unit(&controlled, &witness_id).unwrap()["warded"],
            true
        );
        accept_where(&mut session, |d| d["kind"] == "end-turn");

        let mut turn_three_sources = Vec::new();
        while state(&session)["phase"] == "start-turn" {
            let next_source = session
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
                .map(|action| {
                    action.descriptor["sourceInstanceId"]
                        .as_str()
                        .unwrap()
                        .to_owned()
                });
            let Some(next_source) = next_source else {
                break;
            };
            let (_, source_receipt) = accept_where(&mut session, |d| {
                d["kind"] == "resolve-start-turn-trigger" && d["sourceInstanceId"] == next_source
            });
            assert!(
                source_receipt
                    .events
                    .iter()
                    .any(|event| event.event_type == "site-drawn")
            );
            turn_three_sources.push(next_source);
        }
        assert!(turn_three_sources.contains(&first_source_id));
        assert!(turn_three_sources.contains(&survivor_id));
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |d| {
            d["kind"] == "cast-aura" && d["cardId"] == aura_card && d["cells"] == json!([aura_cell])
        });
        accept_where(&mut session, |d| d["kind"] == "end-turn");
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |d| d["kind"] == "end-turn");

        let aura_instance_id = aura_id(&session);
        let parent = session.clone();
        let parent_fingerprint = full_session_fingerprint(&parent);
        let before_checkpoint =
            create_game_checkpoint(&session).expect("Aura pre-trigger checkpoint");
        let before_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&before_checkpoint).unwrap()).unwrap();
        while !session.legal_actions().unwrap().iter().any(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == aura_instance_id
        }) {
            let next_source = session
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
                .expect("remaining start-turn source before Aura")
                .descriptor["sourceInstanceId"]
                .as_str()
                .unwrap()
                .to_owned();
            accept_where(&mut session, |d| {
                d["kind"] == "resolve-start-turn-trigger" && d["sourceInstanceId"] == next_source
            });
        }
        let aura_parent = session.clone();
        let aura_parent_fingerprint = full_session_fingerprint(&aura_parent);
        let aura_checkpoint = create_game_checkpoint(&session).expect("pre-Aura action checkpoint");
        let aura_checkpoint = parse_game_checkpoint(
            &serialize_game_checkpoint(&aura_checkpoint).expect("serialize pre-Aura action"),
        )
        .expect("parse pre-Aura action");
        let aura_descriptor = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == aura_instance_id
            })
            .expect("issued Aura action")
            .descriptor;
        let (_, aura_receipt) =
            accept_where(&mut session, |candidate| candidate == &aura_descriptor);
        let mut restored_aura_parent =
            resume_game_checkpoint(&aura_checkpoint).expect("restore exact Aura parent");
        let (_, restored_aura_receipt) = accept_where(&mut restored_aura_parent, |candidate| {
            candidate == &aura_descriptor
        });
        assert_eq!(restored_aura_receipt, aura_receipt);
        assert_eq!(
            full_session_fingerprint(&restored_aura_parent),
            full_session_fingerprint(&session)
        );
        assert_eq!(
            full_session_fingerprint(&aura_parent),
            aura_parent_fingerprint
        );
        assert!(
            aura_receipt
                .events
                .iter()
                .any(|event| event.event_type == "aura-dispelled")
        );
        assert_eq!(
            aura_receipt
                .events
                .iter()
                .filter(|event| event.event_type == "ward-broken"
                    && event.payload["instanceId"] == witness_id)
                .count(),
            1,
            "the Aura spends the witness's Ward while leaving it alive"
        );
        let pause = state(&session);
        assert_eq!(pause["phase"], "trigger-order");
        assert_eq!(
            pause["pendingDeathrites"]["continuation"]["sourceInstanceId"],
            aura_instance_id
        );
        assert_eq!(
            realm_unit(&pause, &first_source_id).unwrap()["deathMarked"],
            true
        );
        assert!(realm_unit(&pause, &survivor_id).is_some());
        assert_eq!(
            realm_unit(&pause, &witness_id).unwrap()["controller"],
            opponent
        );
        assert_eq!(realm_unit(&pause, &witness_id).unwrap()["warded"], false);
        assert!(
            realm_unit(&pause, &regular_id).unwrap()["deathMarked"]
                .as_bool()
                .unwrap()
        );
        assert!(
            realm_unit(&pause, &puppet_id).unwrap()["deathMarked"]
                .as_bool()
                .unwrap()
        );
        assert!(
            pause["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| {
                    item["instanceId"] == artifact_id && item["bearer"]["instanceId"] == regular_id
                })
        );
        let pause_checkpoint =
            create_game_checkpoint(&session).expect("source-control pause checkpoint");
        let pause_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&pause_checkpoint).unwrap()).unwrap();
        let pause_fingerprint = full_session_fingerprint(&session);
        let choices: Vec<_> = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .filter(|action| action.descriptor["kind"] == "order-triggers")
            .map(|action| {
                action.descriptor["sourceInstanceId"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(choices.len(), 2);
        assert!(choices.contains(&regular_id));
        assert!(choices.contains(&puppet_id));
        for first in choices {
            let mut branch =
                resume_game_checkpoint(&pause_checkpoint).expect("restore order branch");
            let (_, first_order_receipt) = accept_where(&mut branch, |d| {
                d["kind"] == "order-triggers" && d["sourceInstanceId"] == first
            });
            let mut resolutions = vec![(first.clone(), first_order_receipt)];
            while state(&branch)["phase"] == "trigger-order" {
                let next = branch
                    .legal_actions()
                    .unwrap()
                    .into_iter()
                    .find(|action| action.descriptor["kind"] == "order-triggers")
                    .unwrap()
                    .descriptor["sourceInstanceId"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                let (_, receipt) = accept_where(&mut branch, |d| {
                    d["kind"] == "order-triggers" && d["sourceInstanceId"] == next
                });
                resolutions.push((next, receipt));
            }
            let reversion_events: Vec<_> = resolutions
                .iter()
                .flat_map(|(_, receipt)| receipt.events.iter())
                .filter(|event| {
                    event.event_type == "minion-control-changed"
                        && event.payload["instanceId"] == witness_id
                        && event.payload["sourceInstanceId"] == puppet_id
                        && event.payload["fromSeat"] == opponent
                        && event.payload["seat"] == aura_seat
                })
                .collect();
            let after = state(&branch);
            assert_eq!(
                reversion_events.len(),
                1,
                "one source-bound control reversion; receipts={:?}; after={after}",
                resolutions
                    .iter()
                    .map(|(source, receipt)| (
                        source,
                        receipt
                            .events
                            .iter()
                            .map(|event| (event.event_type.as_str(), &event.payload))
                            .collect::<Vec<_>>()
                    ))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                realm_unit(&after, &witness_id).unwrap()["controller"],
                aura_seat
            );
            assert_eq!(realm_unit(&after, &witness_id).unwrap()["owner"], aura_seat);
            assert!(realm_unit(&after, &puppet_id).is_none());
            assert!(realm_unit(&after, &regular_id).is_none());
            assert!(realm_unit(&after, &survivor_id).is_some());
            assert!(
                after["realm"]["artifacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| {
                        item["instanceId"] == artifact_id
                            && item["bearer"].is_null()
                            && item["location"] == aura_cell
                    })
            );
            assert_eq!(
                branch
                    .transcript()
                    .iter()
                    .flat_map(|receipt| &receipt.events)
                    .filter(|event| event.event_type == "artifact-dropped"
                        && event.payload["instanceId"] == artifact_id)
                    .count(),
                1,
                "the carried Artifact leaves its exact marked bearer once"
            );
            assert!(branch.legal_actions().unwrap().iter().any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == survivor_id
            }));
            assert!(!branch.legal_actions().unwrap().iter().any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == first_source_id
            }));
            assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
            assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
            assert!(resume_game_checkpoint(&before_checkpoint).is_ok());
            let completion_fingerprint = full_session_fingerprint(&branch);
            let completion_checkpoint =
                create_game_checkpoint(&branch).expect("complete source-control checkpoint");
            let completion_checkpoint =
                parse_game_checkpoint(&serialize_game_checkpoint(&completion_checkpoint).unwrap())
                    .unwrap();
            let restored_completion =
                resume_game_checkpoint(&completion_checkpoint).expect("restore completion");
            assert_eq!(
                full_session_fingerprint(&restored_completion),
                completion_fingerprint
            );
            assert_exact_replay(&branch);
        }
        assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
        assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
        assert_exact_replay(&session);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "proves an issued Aura's held start-turn continuation survives failed Deathrite draws"
)]
fn aura_empty_atlas_draw_failure_keeps_destruction_batch_terminal_and_replayable() {
    for aura_seat in ["north", "south"] {
        let opponent = if aura_seat == "north" {
            "south"
        } else {
            "north"
        };
        let aura_cell = if aura_seat == "north" { "C4" } else { "C1" };
        let opponent_cell = if aura_seat == "north" { "C1" } else { "C4" };
        let source_card = format!("{aura_seat}-source");
        let aura_card = format!("{aura_seat}-aura");
        let deathrite_card = format!("{opponent}-deathrite");
        let artifact_card = format!("{opponent}-artifact");
        let encoded = (258..2306)
            .map(|seed| ordered_aura_empty_atlas_manifest(seed, aura_seat))
            .find(|candidate| {
                let Ok(mut preview) = Session::new(candidate) else {
                    return false;
                };
                keep(&mut preview);
                keep(&mut preview);
                let snapshot = state(&preview);
                let aura_hand = snapshot["players"][aura_seat]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["cardId"] == source_card);
                let opponent_hand = snapshot["players"][opponent]["hand"]["spellbook"]
                    .as_array()
                    .unwrap();
                aura_hand
                    && opponent_hand
                        .iter()
                        .filter(|card| card["cardId"] == deathrite_card)
                        .count()
                        >= 2
                    && opponent_hand
                        .iter()
                        .any(|card| card["cardId"] == artifact_card)
            })
            .expect("bounded opening with Aura source and two Deathrites");
        let mut session = Session::new(&encoded).expect("empty-Atlas Aura Session");
        keep(&mut session);
        keep(&mut session);
        accept_where(&mut session, |action| {
            action["kind"] == "play-site" && action["cell"] == aura_cell
        });
        let (source, _) = accept_where(&mut session, |action| {
            action["kind"] == "summon-minion"
                && action["cardId"] == source_card
                && action["cell"] == aura_cell
        });
        let source_id = source["cardInstanceId"].as_str().unwrap().to_owned();
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |action| {
            action["kind"] == "play-site" && action["cell"] == opponent_cell
        });
        let (first_deathrite, _) = accept_where(&mut session, |action| {
            action["kind"] == "summon-minion"
                && action["cardId"] == deathrite_card
                && action["cell"] == aura_cell
        });
        let (second_deathrite, _) = accept_where(&mut session, |action| {
            action["kind"] == "summon-minion"
                && action["cardId"] == deathrite_card
                && action["cell"] == aura_cell
        });
        let deathrite_ids = [
            first_deathrite["cardInstanceId"]
                .as_str()
                .unwrap()
                .to_owned(),
            second_deathrite["cardInstanceId"]
                .as_str()
                .unwrap()
                .to_owned(),
        ];
        let (artifact, _) = accept_where(&mut session, |action| {
            action["kind"] == "cast-artifact"
                && action["cardId"] == artifact_card
                && action["bearer"]["instanceId"] == deathrite_ids[0]
        });
        let artifact_id = artifact["cardInstanceId"].as_str().unwrap().to_owned();
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        let (_, source_start) = accept_where(&mut session, |action| {
            action["kind"] == "resolve-start-turn-trigger"
                && action["sourceInstanceId"] == source_id
        });
        assert!(
            source_start
                .events
                .iter()
                .any(|event| event.event_type == "site-drawn")
        );
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |action| {
            action["kind"] == "play-site"
                && action["cell"] != aura_cell
                && action["cell"] != opponent_cell
        });
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        loop {
            let next_source = session
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
                .map(|action| {
                    action.descriptor["sourceInstanceId"]
                        .as_str()
                        .unwrap()
                        .to_owned()
                });
            let Some(next_source) = next_source else {
                break;
            };
            accept_where(&mut session, |action| {
                action["kind"] == "resolve-start-turn-trigger"
                    && action["sourceInstanceId"] == next_source
            });
        }
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |action| {
            action["kind"] == "cast-aura"
                && action["cardId"] == aura_card
                && action["cells"] == json!([aura_cell])
        });
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |action| {
            action["kind"] == "play-site"
                && action["cell"] != aura_cell
                && action["cell"] != opponent_cell
        });
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        assert_eq!(
            state(&session)["players"][opponent]["atlas"]
                .as_array()
                .unwrap()
                .len(),
            0
        );

        let aura_instance_id = aura_id(&session);
        while !session.legal_actions().unwrap().iter().any(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == aura_instance_id
        }) {
            let next_source = session
                .legal_actions()
                .unwrap()
                .into_iter()
                .find(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
                .expect("remaining start source before Aura")
                .descriptor["sourceInstanceId"]
                .as_str()
                .unwrap()
                .to_owned();
            accept_where(&mut session, |action| {
                action["kind"] == "resolve-start-turn-trigger"
                    && action["sourceInstanceId"] == next_source
            });
        }
        let parent = session.clone();
        let parent_fingerprint = full_session_fingerprint(&parent);
        let checkpoint = create_game_checkpoint(&session).expect("pre-Aura checkpoint");
        let checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&checkpoint).unwrap()).unwrap();
        let aura_descriptor = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == aura_instance_id
            })
            .unwrap()
            .descriptor;
        let (_, aura_receipt) = accept_where(&mut session, |action| action == &aura_descriptor);
        assert!(
            aura_receipt
                .events
                .iter()
                .any(|event| event.event_type == "aura-dispelled")
        );
        let pause = state(&session);
        assert_eq!(pause["phase"], "trigger-order");
        assert_eq!(
            pause["players"][opponent]["atlas"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert!(
            deathrite_ids
                .iter()
                .all(|id| realm_unit(&pause, id).is_some_and(|unit| unit["deathMarked"] == true))
        );
        let pause_checkpoint = create_game_checkpoint(&session).expect("Aura pause checkpoint");
        let pause_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&pause_checkpoint).unwrap()).unwrap();
        let pause_fingerprint = full_session_fingerprint(&session);
        for first_source in &deathrite_ids {
            let mut branch =
                resume_game_checkpoint(&pause_checkpoint).expect("restore Aura branch");
            let (_, receipt) = accept_where(&mut branch, |action| {
                action["kind"] == "order-triggers" && action["sourceInstanceId"] == *first_source
            });
            let terminal = state(&branch);
            assert!(terminal["terminal"].is_object() || terminal["terminal"].is_string());
            assert_eq!(
                terminal["players"][opponent]["atlas"]
                    .as_array()
                    .unwrap()
                    .len(),
                0
            );
            assert!(deathrite_ids.iter().all(|id| {
                realm_unit(&terminal, id).is_some_and(|unit| unit["deathMarked"] == true)
            }));
            assert_eq!(
                terminal["pendingDeathrites"]["continuation"]["sourceInstanceId"],
                aura_instance_id
            );
            assert!(
                receipt
                    .events
                    .iter()
                    .all(|event| event.event_type != "deathrite-draw-site")
            );
            assert!(receipt.events.iter().all(|event| {
                !matches!(
                    event.event_type.as_str(),
                    "artifact-dropped" | "minion-died" | "site-drawn" | "spell-drawn" | "draw"
                )
            }));
            assert!(
                terminal["realm"]["artifacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|artifact| artifact["instanceId"] == artifact_id
                        && artifact["bearer"]["instanceId"] == deathrite_ids[0])
            );
            let mut restored =
                resume_game_checkpoint(&checkpoint).expect("restore pre-Aura parent");
            let (_, restored_receipt) =
                accept_where(&mut restored, |action| action == &aura_descriptor);
            assert_eq!(restored_receipt, aura_receipt);
            assert_eq!(
                full_session_fingerprint(&restored),
                full_session_fingerprint(&session)
            );
            assert!(branch.legal_actions().unwrap().is_empty());
            let terminal_fingerprint = full_session_fingerprint(&branch);
            let terminal_checkpoint =
                create_game_checkpoint(&branch).expect("terminal carried-Artifact checkpoint");
            let terminal_checkpoint =
                parse_game_checkpoint(&serialize_game_checkpoint(&terminal_checkpoint).unwrap())
                    .unwrap();
            let terminal_restored =
                resume_game_checkpoint(&terminal_checkpoint).expect("restore terminal branch");
            assert_eq!(
                full_session_fingerprint(&terminal_restored),
                terminal_fingerprint
            );
            assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
            assert_exact_replay(&branch);
        }
        assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "proves Surface and lower-region Aura deaths share one issued custody batch"
)]
fn water_site_aura_rubble_mixes_surface_and_regional_deathrite_custody() {
    for aura_seat in ["north", "south"] {
        let opponent = if aura_seat == "north" {
            "south"
        } else {
            "north"
        };
        let aura_cell = if aura_seat == "north" { "C4" } else { "C1" };
        let opponent_cell = if aura_seat == "north" { "C1" } else { "C4" };
        let aura_card = format!("{aura_seat}-aura");
        let deathrite_card = format!("{opponent}-deathrite");
        let surface_card = format!("{opponent}-surface");
        let dual_card = format!("{opponent}-dual");
        let artifact_card = format!("{opponent}-artifact");
        let encoded = (258..4354)
            .map(|seed| ordered_aura_water_region_manifest(seed, aura_seat))
            .find(|candidate| {
                let Ok(mut preview) = Session::new(candidate) else {
                    return false;
                };
                keep(&mut preview);
                keep(&mut preview);
                let opening = state(&preview);
                if !opening["players"][aura_seat]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["cardId"] == aura_card)
                {
                    return false;
                }
                accept_where(&mut preview, |action| {
                    action["kind"] == "play-site" && action["cell"] == aura_cell
                });
                accept_where(&mut preview, |action| action["kind"] == "end-turn");
                accept_where(&mut preview, |action| {
                    action["kind"] == "draw" && action["zone"] == "spellbook"
                });
                let final_state = state(&preview);
                let hand = final_state["players"][opponent]["hand"]["spellbook"]
                    .as_array()
                    .unwrap()
                    .clone();
                let first_turn_cards = [
                    deathrite_card.as_str(),
                    dual_card.as_str(),
                    artifact_card.as_str(),
                ]
                .iter()
                .all(|wanted| hand.iter().any(|card| card["cardId"] == *wanted));
                if !first_turn_cards {
                    return false;
                }
                accept_where(&mut preview, |action| {
                    action["kind"] == "play-site" && action["cell"] == opponent_cell
                });
                accept_where(&mut preview, |action| {
                    action["kind"] == "summon-minion"
                        && action["cardId"] == deathrite_card
                        && action["cell"] == aura_cell
                        && action["region"] == "underwater"
                });
                accept_where(&mut preview, |action| {
                    action["kind"] == "summon-minion"
                        && action["cardId"] == dual_card
                        && action["cell"] == aura_cell
                        && action["region"] == "underwater"
                });
                accept_where(&mut preview, |action| {
                    action["kind"] == "cast-artifact" && action["cardId"] == artifact_card
                });
                accept_where(&mut preview, |action| action["kind"] == "end-turn");
                accept_where(&mut preview, |action| {
                    action["kind"] == "draw" && action["zone"] == "atlas"
                });
                while preview
                    .legal_actions()
                    .unwrap()
                    .iter()
                    .any(|action| action.descriptor["kind"] == "draw-site")
                {
                    accept_where(&mut preview, |action| action["kind"] == "draw-site");
                }
                let _ = draw_spellbook_if_offered(&mut preview);
                if !preview.legal_actions().unwrap().iter().any(|action| {
                    action.descriptor["kind"] == "cast-aura"
                        && action.descriptor["cardId"] == aura_card
                        && action.descriptor["cells"] == json!([aura_cell])
                }) {
                    return false;
                }
                accept_where(&mut preview, |action| {
                    action["kind"] == "cast-aura"
                        && action["cardId"] == aura_card
                        && action["cells"] == json!([aura_cell])
                });
                accept_where(&mut preview, |action| action["kind"] == "end-turn");
                accept_where(&mut preview, |action| {
                    action["kind"] == "draw" && action["zone"] == "spellbook"
                });
                let final_state = state(&preview);
                let hand = final_state["players"][opponent]["hand"]["spellbook"]
                    .as_array()
                    .unwrap();
                hand.iter().any(|card| card["cardId"] == surface_card)
                    && hand
                        .iter()
                        .filter(|card| card["cardId"] == artifact_card)
                        .count()
                        >= 1
            })
            .expect("bounded Aura regional cohort opening");
        let mut session = Session::new(&encoded).expect("Aura regional Session");
        keep(&mut session);
        keep(&mut session);
        accept_where(&mut session, |action| {
            action["kind"] == "play-site" && action["cell"] == aura_cell
        });
        let water_site_id = state(&session)["realm"]["sites"][aura_cell]["instanceId"]
            .as_str()
            .unwrap()
            .to_owned();
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        accept_where(&mut session, |action| {
            action["kind"] == "draw" && action["zone"] == "spellbook"
        });
        accept_where(&mut session, |action| {
            action["kind"] == "play-site" && action["cell"] == opponent_cell
        });
        let (submerged, _) = accept_where(&mut session, |action| {
            action["kind"] == "summon-minion"
                && action["cardId"] == deathrite_card
                && action["cell"] == aura_cell
                && action["region"] == "underwater"
        });
        let (dual, _) = accept_where(&mut session, |action| {
            action["kind"] == "summon-minion"
                && action["cardId"] == dual_card
                && action["cell"] == aura_cell
                && action["region"] == "underwater"
        });
        let submerged_id = submerged["cardInstanceId"].as_str().unwrap().to_owned();
        let dual_id = dual["cardInstanceId"].as_str().unwrap().to_owned();
        let (carried, _) = accept_where(&mut session, |action| {
            action["kind"] == "cast-artifact"
                && action["cardId"] == artifact_card
                && action["bearer"]["instanceId"] == submerged_id
        });
        let carried_id = carried["cardInstanceId"].as_str().unwrap().to_owned();
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        accept_where(&mut session, |action| {
            action["kind"] == "draw" && action["zone"] == "atlas"
        });
        while session
            .legal_actions()
            .unwrap()
            .iter()
            .any(|action| action.descriptor["kind"] == "draw-site")
        {
            accept_where(&mut session, |action| action["kind"] == "draw-site");
        }
        let _ = draw_spellbook_if_offered(&mut session);
        accept_where(&mut session, |action| {
            action["kind"] == "cast-aura"
                && action["cardId"] == aura_card
                && action["cells"] == json!([aura_cell])
        });
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        accept_where(&mut session, |action| {
            action["kind"] == "draw" && action["zone"] == "spellbook"
        });
        let (surface, _) = accept_where(&mut session, |action| {
            action["kind"] == "summon-minion"
                && action["cardId"] == surface_card
                && action["cell"] == aura_cell
                && action["region"].is_null()
        });
        let surface_id = surface["cardInstanceId"].as_str().unwrap().to_owned();
        let (loose, _) = accept_where(&mut session, |action| {
            action["kind"] == "cast-artifact"
                && action["cardId"] == artifact_card
                && action["bearer"]["instanceId"] == dual_id
        });
        let loose_id = loose["cardInstanceId"].as_str().unwrap().to_owned();
        accept_where(&mut session, |action| {
            action["kind"] == "drop-artifacts"
                && action["unit"]["instanceId"] == dual_id
                && action["artifactInstanceIds"] == json!([loose_id])
        });
        let loose_before = state(&session)["realm"]["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|artifact| artifact["instanceId"] == loose_id)
            .cloned()
            .expect("actual dropped underground Artifact");
        assert_eq!(loose_before["region"], "underwater");
        accept_where(&mut session, |action| action["kind"] == "end-turn");
        let aura_instance_id = aura_id(&session);
        let parent = session.clone();
        let parent_fingerprint = full_session_fingerprint(&parent);
        let before_checkpoint = create_game_checkpoint(&session).expect("pre-Aura checkpoint");
        let before_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&before_checkpoint).unwrap()).unwrap();
        let (_, aura_receipt) = accept_where(&mut session, |action| {
            action["kind"] == "resolve-start-turn-trigger"
                && action["sourceInstanceId"] == aura_instance_id
        });
        assert!(
            aura_receipt
                .events
                .iter()
                .any(|event| event.event_type == "site-destroyed")
        );
        assert!(
            aura_receipt
                .events
                .iter()
                .any(|event| event.event_type == "rubble-created")
        );
        assert_eq!(
            aura_receipt
                .events
                .iter()
                .filter(|event| event.event_type == "ward-broken")
                .count(),
            1
        );
        assert!(aura_receipt.events.iter().any(|event| {
            event.event_type == "site-destroyed" && event.payload["instanceId"] == water_site_id
        }));
        let pause = state(&session);
        assert_eq!(pause["realm"]["sites"][aura_cell]["rubble"], true);
        assert_eq!(
            realm_unit(&pause, &submerged_id).unwrap_or_else(|| panic!(
                "submerged Deathrite absent at pause: state={pause}; receipt={:?}",
                aura_receipt.events
            ))["deathMarked"],
            true
        );
        assert_eq!(
            realm_unit(&pause, &surface_id).unwrap()["deathMarked"],
            true
        );
        assert_eq!(
            realm_unit(&pause, &dual_id).unwrap()["region"],
            "underground"
        );
        assert!(
            pause["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| {
                    artifact["instanceId"] == carried_id
                        && artifact["bearer"]["instanceId"] == submerged_id
                })
        );
        assert!(
            pause["realm"]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| {
                    artifact["instanceId"] == loose_id
                        && artifact["bearer"].is_null()
                        && artifact["location"] == aura_cell
                        && artifact["region"] == "underground"
                })
        );
        assert_eq!(
            pause["pendingDeathrites"]["continuation"]["sourceInstanceId"],
            aura_instance_id
        );
        let pause_checkpoint = create_game_checkpoint(&session).expect("Aura pause checkpoint");
        let pause_checkpoint =
            parse_game_checkpoint(&serialize_game_checkpoint(&pause_checkpoint).unwrap()).unwrap();
        let pause_fingerprint = full_session_fingerprint(&session);
        let sources: Vec<_> = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .filter(|action| action.descriptor["kind"] == "order-triggers")
            .map(|action| {
                action.descriptor["sourceInstanceId"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(sources.len(), 2);
        for first_source in &sources {
            let mut branch =
                resume_game_checkpoint(&pause_checkpoint).expect("restore Aura branch");
            accept_where(&mut branch, |action| {
                action["kind"] == "order-triggers"
                    && action["sourceInstanceId"] == first_source.as_str()
            });
            while state(&branch)["phase"] == "trigger-order" {
                accept_where(&mut branch, |action| action["kind"] == "order-triggers");
            }
            let complete = state(&branch);
            assert!(realm_unit(&complete, &submerged_id).is_none());
            assert!(realm_unit(&complete, &surface_id).is_none());
            assert!(realm_unit(&complete, &dual_id).is_some());
            assert!(
                complete["players"][opponent]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["instanceId"] == submerged_id)
            );
            assert_eq!(
                branch
                    .transcript()
                    .iter()
                    .flat_map(|receipt| &receipt.events)
                    .filter(|event| event.event_type == "aura-dispelled")
                    .count(),
                1
            );
            assert_eq!(
                branch
                    .transcript()
                    .iter()
                    .flat_map(|receipt| &receipt.events)
                    .filter(|event| event.event_type == "minion-died")
                    .count(),
                2
            );
            assert!(
                complete["players"][opponent]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|card| card["instanceId"] == surface_id)
            );
            assert!(
                complete["realm"]["artifacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|artifact| {
                        artifact["instanceId"] == carried_id
                            && artifact["bearer"].is_null()
                            && artifact["location"] == aura_cell
                            && artifact["region"] == "underground"
                    })
            );
            assert!(
                complete["realm"]["artifacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|artifact| {
                        artifact["instanceId"] == loose_id
                            && artifact["bearer"].is_null()
                            && artifact["location"] == aura_cell
                            && artifact["region"] == "underground"
                    })
            );
            assert_eq!(complete["realm"]["sites"][aura_cell]["rubble"], true);
            assert_exact_replay(&branch);
        }
        let mut restored = resume_game_checkpoint(&before_checkpoint).expect("restore pre-Aura");
        let (_, restored_receipt) = accept_where(&mut restored, |action| {
            action["kind"] == "resolve-start-turn-trigger"
                && action["sourceInstanceId"] == aura_instance_id
        });
        assert_eq!(restored_receipt, aura_receipt);
        assert_eq!(
            full_session_fingerprint(&restored),
            full_session_fingerprint(&session)
        );
        assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
        assert_eq!(full_session_fingerprint(&session), pause_fingerprint);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "proves Aura self-destruction and minion loss remain independent of Site protection"
)]
fn aura_site_ward_keeps_site_while_self_and_minion_clauses_resolve() {
    let seed = (21000..22024)
        .find(|seed| {
            let candidate = Session::new(&site_ward_manifest(*seed)).unwrap();
            let candidate_state = state(&candidate);
            let hand = candidate_state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .unwrap();
            hand.iter().any(|card| card["cardId"] == "north-aura")
                && hand.iter().any(|card| card["cardId"] == "north-bless")
        })
        .expect("bounded opening with Aura and Site-Ward Magic");
    let encoded = site_ward_manifest(seed);
    let mut session = Session::new(&encoded).expect("warded Site Aura fixture");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    let (_, ward_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bless"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert!(
        ward_receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-warded")
    );
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (victim, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    let victim_id = victim["cardInstanceId"].as_str().unwrap().to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let parent = session.clone();
    let parent_fingerprint = full_session_fingerprint(&parent);
    let source_id = aura_id(&session);
    let avatar_life_before = (
        state(&session)["players"]["north"]["avatar"]["life"].clone(),
        state(&session)["players"]["south"]["avatar"]["life"].clone(),
    );
    let before_checkpoint = create_game_checkpoint(&session).expect("pre-Aura trigger checkpoint");
    let before_checkpoint = serialize_game_checkpoint(&before_checkpoint).unwrap();
    let before_checkpoint = parse_game_checkpoint(&before_checkpoint).unwrap();
    let trigger_descriptor = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .unwrap()
        .descriptor;
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == source_id
    });
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-dispelled")
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-died" && event.payload["instanceId"] == victim_id
    }));
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "ward-broken")
            .count(),
        1
    );
    let aura_dispersed = receipt
        .events
        .iter()
        .position(|event| event.event_type == "aura-dispelled")
        .unwrap();
    let ward_broken = receipt
        .events
        .iter()
        .position(|event| event.event_type == "ward-broken")
        .unwrap();
    let victim_died = receipt
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .unwrap();
    assert!(aura_dispersed < ward_broken && ward_broken < victim_died);
    assert!(!receipt.events.iter().any(|event| {
        event.event_type == "site-destroyed" || event.event_type == "rubble-created"
    }));
    let after = state(&session);
    assert_eq!(after["realm"]["sites"]["C4"]["instanceId"], site_id);
    assert!(after["realm"]["sites"]["C4"]["warded"].is_null());
    assert!(
        after["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .all(|unit| unit["instanceId"] != victim_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["instanceId"] == source_id)
    );
    assert_eq!(
        (
            after["players"]["north"]["avatar"]["life"].clone(),
            after["players"]["south"]["avatar"]["life"].clone(),
        ),
        avatar_life_before,
        "Aura mutation and independent warded Site/minion effects leave both Avatars unchanged"
    );
    let mut restored_before =
        resume_game_checkpoint(&before_checkpoint).expect("restore pre-Aura trigger");
    let (_, restored_receipt) = accept_where(&mut restored_before, |descriptor| {
        descriptor == &trigger_descriptor
    });
    assert_eq!(restored_receipt, receipt);
    assert_eq!(
        full_session_fingerprint(&restored_before),
        full_session_fingerprint(&session)
    );
    assert_eq!(full_session_fingerprint(&parent), parent_fingerprint);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one protected start-turn destruction and ward interaction proof"
)]
fn protected_start_turn_destruction_consumes_ward_without_marking_minions() {
    let mut session = after_aura_with_warded_minion_on_c4();
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let before = state(&session);
    let before_hash = session.state_hash().expect("pre-trigger hash");
    let mut ignored = replay_game_for_session(&session);
    assert_eq!(ignored.authoritative_state(), before);
    assert_eq!(ignored.state_hash().unwrap(), before_hash);
    let checkpoint_before =
        create_game_checkpoint(&session).expect("pre-effect protected trigger checkpoint");
    let encoded_before =
        serialize_game_checkpoint(&checkpoint_before).expect("serialize pre-effect checkpoint");
    let parsed_before =
        parse_game_checkpoint(&encoded_before).expect("parse pre-effect checkpoint");
    let mut restored_before =
        resume_game_checkpoint(&parsed_before).expect("resume pre-effect checkpoint");
    assert_eq!(
        restored_before.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
    assert_eq!(
        restored_before.legal_actions().unwrap(),
        session.legal_actions().unwrap()
    );
    assert_eq!(restored_before.transcript(), session.transcript());
    let action = session
        .legal_actions()
        .expect("protected start-turn actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("issued occupied-site Aura trigger");
    let ignore_action = ignored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.to_legal_action().unwrap().action_id == action.action_id)
        .expect("same issued trigger in Ignore path");
    ignored
        .apply_action(&ignore_action)
        .expect("Ignore-path protected trigger");
    let restored_action = restored_before
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.action_id == action.action_id)
        .expect("same issued trigger after checkpoint resume");
    let StepResult::Accepted(restored_receipt) = restored_before
        .step(ActionRequest {
            action_id: restored_action.action_id.to_string(),
            seat: restored_action.seat,
            state_version: restored_action.state_version,
        })
        .expect("resumed protected trigger")
    else {
        panic!("resumed issued trigger must be accepted");
    };
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("Ward-protected destruction should resolve")
    else {
        panic!("issued start-turn trigger must be accepted");
    };

    assert_eq!(
        serde_json::to_value(&restored_receipt).unwrap(),
        serde_json::to_value(&receipt).unwrap()
    );
    assert_eq!(
        restored_before.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
    assert_eq!(restored_before.transcript(), session.transcript());
    let warded_recipients = before["realm"]["units"]
        .as_array()
        .expect("two actual Surface recipients")
        .iter()
        .map(|unit| (unit["instanceId"].clone(), unit["controller"].clone()))
        .collect::<Vec<_>>();
    assert_eq!(warded_recipients.len(), 2);
    let ward_events = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "ward-broken")
        .map(|event| event.payload.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        ward_events,
        warded_recipients
            .iter()
            .map(|(instance_id, controller)| json!({
                "instanceId": instance_id,
                "seat": controller,
            }))
            .collect::<Vec<_>>()
    );
    assert!(
        receipt.events.iter().all(|event| {
            event.event_type != "minion-died" && event.event_type != "minion-killed"
        })
    );
    let after = state(&session);
    assert_eq!(ignored.authoritative_state(), after);
    assert_eq!(ignored.state_hash().unwrap(), session.state_hash().unwrap());
    assert_eq!(
        ignored
            .legal_actions()
            .unwrap()
            .iter()
            .map(|action| action.to_legal_action().unwrap())
            .collect::<Vec<_>>(),
        session.legal_actions().unwrap()
    );
    for (id, _) in &warded_recipients {
        let unit = after["realm"]["units"]
            .as_array()
            .expect("surviving protected units")
            .iter()
            .find(|unit| unit["instanceId"] == *id)
            .expect("Ward-protected recipient remains live");
        assert_eq!(unit["warded"], false);
        assert_ne!(unit["deathMarked"], true);
    }
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["realm"]["sites"]["C4"]["rubble"], true);
    assert!(after["realm"].get("auras").is_none());
    assert_ne!(
        session.state_hash().expect("post-trigger hash"),
        before_hash
    );
    for seat in ["north", "south"] {
        assert!(
            state(&session)["players"][seat]["cemetery"]
                .as_array()
                .expect("owner cemetery")
                .iter()
                .all(|card| warded_recipients
                    .iter()
                    .all(|(id, _)| card["instanceId"] != *id))
        );
    }
    let checkpoint = create_game_checkpoint(&session).expect("protected trigger checkpoint");
    let encoded_checkpoint =
        serialize_game_checkpoint(&checkpoint).expect("serialize protected trigger checkpoint");
    let parsed = parse_game_checkpoint(&encoded_checkpoint).expect("parse protected checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("resume protected checkpoint");
    assert_eq!(
        restored.replay_value().unwrap(),
        session.replay_value().unwrap()
    );
    assert_eq!(
        serde_json::to_value(restored.legal_actions().unwrap()).unwrap(),
        serde_json::to_value(session.legal_actions().unwrap()).unwrap()
    );
    assert_eq!(restored.transcript(), session.transcript());
    assert_exact_replay(&restored);
    assert_exact_replay(&session);
}

#[test]
fn start_turn_site_destruction_mixed_ward_cohort_completes() {
    let mut value: Value = serde_json::from_str(&all_warded_manifest()).unwrap();
    value["cards"]["south-minion"]
        .as_object_mut()
        .unwrap()
        .remove("ward");
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] = json!(identity_hash(&value).expect("mixed fixture identity"));
    let encoded = sorcery_engine::canonical::canonical_json(&value).unwrap();
    let mut session = after_aura_with_manifest(&encoded);
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let before_state = state(&session);
    let mut ignored = replay_game_for_session(&session);
    assert_eq!(ignored.authoritative_state(), before_state);
    let before_draws_hash = session.initial_random_draws_hash().unwrap();
    let warded_id = before_state["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["controller"] == "north")
        .expect("North warded minion")["instanceId"]
        .clone();
    let unwarded_id = before_state["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["controller"] == "south")
        .expect("South unwarded minion")["instanceId"]
        .clone();
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("issued occupied-site Aura trigger");
    let ignored_action = ignored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.to_legal_action().unwrap().action_id == action.action_id)
        .expect("same issued mixed trigger in Ignore path");
    ignored
        .apply_action(&ignored_action)
        .expect("Ignore-path mixed trigger");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("Session mixed trigger")
    else {
        panic!("engine-issued mixed trigger must be accepted");
    };
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "ward-broken" && event.payload["instanceId"] == warded_id
    }));
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-died" && event.payload["instanceId"] == unwarded_id
    }));
    let after = state(&session);
    assert_eq!(ignored.authoritative_state(), after);
    assert!(
        after["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .any(|unit| { unit["instanceId"] == warded_id && unit["warded"] == false })
    );
    assert!(
        after["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .all(|unit| unit["instanceId"] != unwarded_id)
    );
    assert_eq!(
        session.initial_random_draws_hash().unwrap(),
        before_draws_hash
    );
    assert!(session.unsupported_mechanic().is_none());
    assert_exact_replay(&session);
}

#[test]
fn start_turn_site_destruction_disabled_ward_is_not_prevention() {
    let mut session = after_aura_disabling_its_warded_minion();
    let source_id = aura_id(&session);
    let protected_id = state(&session)["realm"]["units"][0]["instanceId"].clone();
    assert_eq!(state(&session)["realm"]["units"][0]["warded"], false);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "resolve-start-turn-trigger"
                && action.descriptor["sourceInstanceId"] == source_id
        })
        .expect("issued occupied-site Aura trigger");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("disabled-Ward Aura trigger")
    else {
        panic!("engine-issued disabled-Ward trigger must be accepted");
    };
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-died" && event.payload["instanceId"] == protected_id
    }));
    assert!(!receipt.events.iter().any(|event| {
        event.event_type == "ward-broken" && event.payload["instanceId"] == protected_id
    }));
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .all(|unit| unit["instanceId"] != protected_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn warded_minion_off_the_aura_cell_keeps_ward_and_is_not_a_recipient() {
    let mut session = after_aura_with_warded_minion_on_c4();
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let before = state(&session);
    let off_cell = before["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["controller"] == "south")
        .expect("off-cell Ward minion")["instanceId"]
        .clone();
    let receipt = resolve_start_turn_destroy(&mut session, &source_id);
    let wards = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "ward-broken")
        .map(|event| event.payload["instanceId"].clone())
        .collect::<Vec<_>>();
    assert_eq!(wards.len(), 1);
    assert_ne!(wards[0], off_cell);
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["instanceId"] == off_cell)
        .expect("unrelated minion remains live");
    assert_eq!(unit["warded"], true);
    assert_exact_replay(&session);
}

#[test]
fn unwarded_minion_off_the_aura_cell_is_not_a_recipient() {
    let mut value: Value = serde_json::from_str(&all_warded_manifest()).unwrap();
    value["cards"]["south-minion"]
        .as_object_mut()
        .unwrap()
        .remove("ward");
    value.as_object_mut().unwrap().remove("manifestId");
    value["manifestId"] =
        json!(identity_hash(&value).expect("unwarded off-cell manifest identity"));
    let encoded = sorcery_engine::canonical::canonical_json(&value).unwrap();
    let mut session = after_aura_with_manifest(&encoded);
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
    });
    let off_cell = summon["cardInstanceId"].clone();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        state(&session)["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["instanceId"] == off_cell)
            .expect("issued off-cell minion")["warded"],
        false
    );

    let receipt = resolve_start_turn_destroy(&mut session, &source_id);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "ward-broken")
            .count(),
        1,
        "only the Ward-protected in-footprint minion consumes Ward"
    );
    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["instanceId"] == off_cell)
        .expect("unwarded off-cell minion remains live");
    assert_eq!(unit["warded"], false);
    assert_eq!(unit.get("deathMarked"), None);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0259_unique_sites_are_illegal_and_empty_sites_still_burn() {
    let mut session = Session::new(&manifest(true)).expect("valid unique-site session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let legal_cells: Vec<_> = session
        .legal_actions()
        .expect("Ablaze offers beside a Unique site")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-aura" && action.descriptor["cardId"] == "north-aura"
        })
        .map(|action| action.descriptor["cells"].clone())
        .collect();
    assert!(
        legal_cells.contains(&json!(["C4"])),
        "an Ordinary site remains a legal conjure target: {legal_cells:?}"
    );
    assert!(
        !legal_cells.contains(&json!(["C1"])),
        "a Unique or Legendary site is not a legal conjure target: {legal_cells:?}"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    });
    let source_id = aura_id(&session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let receipt = resolve_start_turn_destroy(&mut session, &source_id);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed" && event.payload["cell"] == "C4")
    );
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.event_type == "aura-dispelled")
    );
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "minion-died")
    );
    let after = state(&session);
    assert_eq!(after["phase"], "draw");
    assert_eq!(after["realm"]["sites"]["C4"]["rubble"], true);
    assert_eq!(after["players"]["north"]["avatar"]["location"], "C4");
    assert!(after["realm"].get("auras").is_none());
    assert_exact_replay(&session);
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn here_pulser() -> Value {
    json!({
        "atStartOfControllerTurnDamageEachOtherUnitHere": 1,
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
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

fn deathrite_site_destroy_withheld_manifest(seed: u32) -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "start-turn-site-destroy-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-start-turn-site-destroy-deathrite-withheld-v1",
        },
        "cards": {
            "north-aura": aura(),
            "north-avatar": avatar(),
            "north-pulser": here_pulser(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_minion(),
            "south-site": site(),
            "south-visitor": minion(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-pulser", "north-aura", "north-aura", "north-aura", "north-aura", "north-aura"],
            },
            "south": {
                "atlas": vec!["south-site"; 12],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-visitor",
                    "south-deathrite",
                    "south-deathrite",
                    "south-deathrite",
                    "south-deathrite",
                    "south-deathrite",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    sorcery_engine::canonical::canonical_json(&value).expect("canonical synthetic manifest")
}

struct PendingSiteDestroySetup {
    aura_id: Value,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_during_site_destroy_start_turn(
    encoded: &str,
) -> Option<PendingSiteDestroySetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let pulser = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-pulser"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-aura"
            && descriptor["cardId"] == "north-aura"
            && descriptor["cells"] == json!(["C4"])
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    if state(&session)["phase"] != "start-turn" {
        return None;
    }
    let aura_id = aura_id(&session);
    let aura_id_str = aura_id.as_str()?.to_owned();
    let pulser_id = pulser.0["cardInstanceId"].as_str()?.to_owned();
    let offered: Vec<_> = session
        .legal_actions()
        .ok()?
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    if !offered.contains(&pulser_id) || !offered.contains(&aura_id_str) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-start-turn-trigger"
            && descriptor["sourceInstanceId"] == pulser_id
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    if session
        .legal_actions()
        .ok()?
        .iter()
        .any(|action| action.descriptor["kind"] == "resolve-start-turn-trigger")
    {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingSiteDestroySetup {
        aura_id,
        deathrite_ids,
        session,
    })
}

#[test]
fn rule_catalog_1241_start_turn_site_destroy_trigger_withheld_during_pending_deathrite_order() {
    let encoded = (1241..1241 + 256)
        .map(deathrite_site_destroy_withheld_manifest)
        .find(|candidate| try_pending_deathrite_during_site_destroy_start_turn(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites during site-destroy start-turn withhold",
        );
    let setup = try_pending_deathrite_during_site_destroy_start_turn(&encoded)
        .expect("complete site-destroy start-turn Deathrite withheld setup");
    let aura_id = setup.aura_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let mut session = setup.session;
    assert_eq!(state(&session)["phase"], "trigger-order");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "resolve-start-turn-trigger")
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });
    assert_eq!(state(&session)["phase"], "start-turn");
    assert!(
        session
            .legal_actions()
            .expect("resumed legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "resolve-start-turn-trigger"
                    && action.descriptor["sourceInstanceId"] == aura_id
            })
    );
    assert_exact_replay(&session);
}
