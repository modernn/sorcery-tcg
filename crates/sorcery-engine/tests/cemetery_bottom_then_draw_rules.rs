//! Direct proofs for return-up-to-three-cemetery-cards-to-deck-bottom
//! then draw-spell Magic (RULE-CATALOG-0549–0550, RULE-CATALOG-1005).
//!
//! Ordinary Magic can return up to three cards from the caster's cemetery
//! to the bottoms of their owners' matching decks and then draw one spell.
//! An empty selection is a paid no-op that still draws.

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

fn nature() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnUpToThreeCemeteryCardsToDeckBottomThenDrawSpell": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn destroy_site() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn nature_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-bottom-then-draw" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-bottom-then-draw-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-nature": nature(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-nature"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["north-nature"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn empty_library_nature_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-bottom-then-draw-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-bottom-then-draw-empty-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_site(),
            "north-nature": nature(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": ["north-nature", "north-nature", "north-destroy"],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["north-nature"; 6],
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
    let mut session = Session::new(encoded).expect("valid cemetery-bottom session");
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

fn cemetery_ids(snapshot: &Value, seat: &str) -> Vec<String> {
    snapshot["players"][seat]["cemetery"]
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

fn nature_offers(session: &Session) -> Vec<Vec<String>> {
    session
        .legal_actions()
        .expect("nature actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-nature"
        })
        .map(|action| {
            action.descriptor["cemeteryCardInstanceIds"]
                .as_array()
                .map(|cards| {
                    cards
                        .iter()
                        .map(|card| card.as_str().expect("cemetery card identity").to_owned())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}

fn seed_with(required_count: usize) -> String {
    (549..549 + 256)
        .map(nature_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .filter(|card| *card == "north-nature")
                .count()
                >= required_count
        })
        .expect("bounded seed with required Return to Nature opening cards")
}

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
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
fn rule_catalog_0549_nature_returns_a_cemetery_card_to_the_deck_bottom_then_draws() {
    let encoded = seed_with(2);
    let mut session = opening_main(&encoded);
    let (first_cast, first_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-nature"
            && descriptor["cemeteryCardInstanceIds"]
                .as_array()
                .is_none_or(Vec::is_empty)
    });
    assert_eq!(
        event_types(&first_receipt),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    let first_id = first_cast["cardInstanceId"]
        .as_str()
        .expect("first nature identity")
        .to_owned();
    south_plays_c1(&mut session);
    let before = state(&session);
    assert!(cemetery_ids(&before, "north").contains(&first_id));
    let library_top = before["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let offered = nature_offers(&session);
    assert!(offered.iter().any(Vec::is_empty));
    assert!(offered.iter().any(|cards| cards == &vec![first_id.clone()]));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-nature"
            && descriptor["cemeteryCardInstanceIds"] == json!([first_id])
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "card-returned-to-deck-bottom",
            "spell-drawn",
            "magic-resolved"
        ]
    );
    assert_eq!(granted.events[1].payload["instanceId"], first_id);
    assert_eq!(granted.events[1].payload["zone"], "spellbook");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert!(!cemetery_ids(&after, "north").contains(&first_id));
    let spellbook = after["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook after return");
    assert_eq!(
        spellbook.last().expect("bottom card")["instanceId"],
        first_id
    );
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
fn rule_catalog_0550_nature_still_draws_when_no_cemetery_card_is_returned() {
    let encoded = seed_with(1);
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
    assert!(nature_offers(&session).iter().all(Vec::is_empty));

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-nature"
            && descriptor["cemeteryCardInstanceIds"]
                .as_array()
                .is_none_or(Vec::is_empty)
    });
    assert_eq!(
        event_types(&granted),
        ["magic-cast", "spell-drawn", "magic-resolved"]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "card-returned-to-deck-bottom")
    );
    let after = state(&session);
    assert!(
        cemetery_ids(&after, "north").contains(
            &cast["cardInstanceId"]
                .as_str()
                .expect("cast identity")
                .to_owned()
        )
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1005_cemetery_bottom_grant_then_empty_spellbook_is_a_deck_out() {
    let encoded = empty_library_nature_manifest(1005);
    let mut session = opening_main(&encoded);
    assert_eq!(
        state(&session)["players"]["north"]["spellbook"]
            .as_array()
            .expect("empty library")
            .len(),
        0
    );
    let site_id = state(&session)["realm"]["sites"]["C4"]["instanceId"]
        .as_str()
        .expect("C4 site identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "C4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert!(cemetery_ids(&state(&session), "north").contains(&site_id));
    assert!(
        nature_offers(&session)
            .iter()
            .any(|cards| cards == &vec![site_id.clone()])
    );

    let (cast, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-nature"
            && descriptor["cemeteryCardInstanceIds"] == json!([site_id])
    });
    assert_eq!(
        event_types(&granted),
        [
            "magic-cast",
            "card-returned-to-deck-bottom",
            "magic-resolved",
            "game-ended"
        ]
    );
    assert!(
        !granted
            .events
            .iter()
            .any(|event| event.event_type == "spell-drawn")
    );
    let ended = granted
        .events
        .iter()
        .find(|event| event.event_type == "game-ended")
        .expect("deck-out");
    assert_eq!(ended.payload["reason"], "deck_empty");
    assert_eq!(ended.payload["loser"], "north");
    assert_eq!(ended.payload["winner"], "south");
    assert_eq!(granted.events[1].payload["instanceId"], site_id);
    assert_eq!(granted.events[1].payload["zone"], "atlas");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(&session);
    assert!(!cemetery_ids(&after, "north").contains(&site_id));
    assert_eq!(
        after["players"]["north"]["atlas"]
            .as_array()
            .expect("north Atlas after return")
            .last()
            .expect("bottom card")["instanceId"],
        site_id
    );
    assert_eq!(after["terminal"]["status"], "finished");
    assert_eq!(after["terminal"]["reason"], "deck_empty");
    assert_exact_replay(&session);
}
