//! Direct proofs for summon-token-to-each-controlled-site-bordering-enemy-site
//! Magic (RULE-CATALOG-0579–0580).
//!
//! Ordinary Magic summons one source-linked token onto each controlled site
//! that borders an enemy-controlled site, in stable cell order. When no site
//! qualifies, casting still resolves as a paid no-op.

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

fn foot_soldier_token() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "token": true,
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

fn border_militia() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "summonTokenToEachControlledSiteBorderingEnemySite": "foot-soldier-token",
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn border_militia_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "border-militia" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-border-militia-v1",
        },
        "cards": {
            "foot-soldier-token": foot_soldier_token(),
            "north-avatar": avatar(),
            "north-militia": border_militia(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": raider(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                    "north-militia",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                    "south-minion",
                ],
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
    let mut session = Session::new(encoded).expect("valid border-militia session");
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

fn seed_with(required: &[&str]) -> String {
    (579..579 + 256)
        .map(border_militia_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn end_turn(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn draw_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn summon_at(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned instance identity")
        .to_owned()
}

fn token_units_at<'a>(snapshot: &'a Value, cells: &[&str]) -> Vec<&'a Value> {
    let units = snapshot["realm"]["units"]
        .as_array()
        .expect("realm units");
    cells
        .iter()
        .map(|cell| {
            units
                .iter()
                .find(|unit| unit["location"] == *cell && unit["cardId"] == "foot-soldier-token")
                .unwrap_or_else(|| panic!("expected token at {cell}"))
        })
        .collect()
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

fn end_and_draw_atlas(session: &mut Session) {
    end_turn(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
}

fn setup_bordering_sites(session: &mut Session) {
    for cell in ["C1", "C3", "C2", "B3", "B2"] {
        end_and_draw_atlas(session);
        play_site(session, cell);
    }
    summon_at(session, "south-minion", "B2");
    end_turn(session);
    draw_spellbook(session);
}

#[test]
fn rule_catalog_0579_border_militia_summons_tokens_on_bordering_sites_in_order() {
    let encoded = seed_with(&["north-militia"]);
    let mut session = opening_main(&encoded);
    setup_bordering_sites(&mut session);
    let pre_cast_version = state(&session)["stateVersion"]
        .as_u64()
        .expect("pre-cast state version");
    let (cast, receipt) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "cast-magic");
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-summoned",
            "minion-summoned",
            "magic-resolved",
        ]
    );
    assert_eq!(
        receipt.events[1..3]
            .iter()
            .map(|event| event.payload["cell"].as_str().expect("token cell"))
            .collect::<Vec<_>>(),
        ["B3", "C3"]
    );
    let source_id = cast["cardInstanceId"].as_str().expect("Magic identity");
    assert!(
        receipt.events[1..3]
            .iter()
            .all(|event| event.payload["sourceInstanceId"] == source_id)
    );
    let after = state(&session);
    let tokens = token_units_at(&after, &["B3", "C3"]);
    assert_eq!(tokens.len(), 2);
    for ((ordinal, cell), token) in ["B3", "C3"]
        .into_iter()
        .enumerate()
        .zip(tokens)
    {
        let expected_id = identity_hash(&json!({
            "cardId": "foot-soldier-token",
            "cell": cell,
            "ordinal": ordinal,
            "owner": "north",
            "source": "token",
            "sourceInstanceId": source_id,
            "stateVersion": pre_cast_version,
        }))
        .expect("expected token identity");
        assert_eq!(token["instanceId"], json!(expected_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0580_border_militia_without_bordering_sites_is_a_paid_noop() {
    let encoded = seed_with(&["north-militia"]);
    let mut session = opening_main(&encoded);

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-militia"
    });
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert_exact_replay(&session);
}
