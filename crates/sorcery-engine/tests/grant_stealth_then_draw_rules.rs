//! Direct proofs for grant-Stealth-to-allied-minions then draw-spell Magic
//! (RULE-CATALOG-0539–0540).
//!
//! Ordinary Magic can give every allied minion Stealth and then draw one
//! spell. The Avatar and enemy minions are not stealthed. Already-stealthed
//! allies stay stealthed without a second event. An empty allied-minion set
//! is a paid no-op grant that still draws.

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

fn vanish() -> Value {
    json!({
        "cardType": "magic",
        "grantStealthToAlliedMinionsThenDrawSpell": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn vanish_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "grant-stealth-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-grant-stealth-then-draw-v1",
        },
        "cards": {
            "north-ally": grounded(),
            "north-avatar": avatar(),
            "north-site": site(),
            "north-vanish": vanish(),
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
                    "north-ally",
                    "north-vanish",
                    "north-vanish",
                    "north-vanish",
                    "north-vanish",
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
    let mut session = Session::new(encoded).expect("valid grant-stealth-then-draw session");
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

fn vanish_casts(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("vanish actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-vanish"
        })
        .map(|action| action.descriptor.clone())
        .collect()
}

fn assert_targetless_vanish(session: &Session) {
    let casts = vanish_casts(session);
    assert!(!casts.is_empty());
    assert!(
        casts
            .iter()
            .all(|descriptor| { descriptor["ally"].is_null() && descriptor["target"].is_null() }),
        "grant-Stealth-then-draw is targetless"
    );
}

fn seed_with_ally_and_vanish() -> String {
    (539..539 + 256)
        .map(vanish_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-vanish")
        })
        .expect("bounded seed with ally and Stealth-then-draw Magic in the opening hand")
}

fn seed_with_vanish() -> String {
    (540..540 + 256)
        .map(vanish_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|id| id == "north-vanish")
        })
        .expect("bounded seed with Stealth-then-draw Magic in the opening hand")
}

fn opening_with_ally(encoded: &str) -> (Session, String) {
    let mut session = opening_main(encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned();
    (session, ally_id)
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
fn rule_catalog_0539_stealth_then_draw_stealths_allied_minions_not_enemies() {
    let encoded = seed_with_ally_and_vanish();
    let (mut session, ally_id) = opening_with_ally(&encoded);
    let enemy_id = south_plays_c1(&mut session);
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
    assert_targetless_vanish(&session);
    assert_eq!(unit(&before, &ally_id)["stealthed"], false);
    assert_eq!(unit(&before, &enemy_id)["stealthed"], false);

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-vanish"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
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
    assert_eq!(granted.events[1].payload["instanceId"], ally_id);
    assert_eq!(granted.events[1].payload["seat"], "north");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "minion-stealthed"
                && (event.payload["instanceId"] == enemy_id
                    || event.payload["instanceId"] == north_avatar))
    );
    let after = state(&session);
    assert_eq!(unit(&after, &ally_id)["stealthed"], true);
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
    let north_view = session.public_view(Seat::North).expect("North public view");
    assert_eq!(north_view["players"]["south"]["hand"]["spellbook"], 3);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0540_stealth_then_draw_still_draws_without_allied_minions() {
    let encoded = seed_with_vanish();
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    assert_targetless_vanish(&session);
    assert!(
        before["realm"]["units"]
            .as_array()
            .expect("realm units")
            .is_empty()
    );

    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-vanish"
            && descriptor["ally"].is_null()
            && descriptor["target"].is_null()
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "minion-stealthed")
    );
    let after = state(&session);
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(&session);
}
