//! Direct proofs for return-up-to-three-cemetery-cards-to-deck-bottom
//! then draw-spell Magic (RULE-CATALOG-0549–0550, RULE-CATALOG-1005,
//! RULE-CATALOG-1070, RULE-CATALOG-1723–1728).
//!
//! Ordinary Magic can return up to three cards from the caster's cemetery
//! to the bottoms of their owners' matching decks and then draw one spell.
//! An empty selection is a paid no-op that still draws. While Deathrites
//! wait for ordering, cemetery-bottom Magic stays withheld until the chain
//! drains.

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

fn rain_spell() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
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

fn advance_full_round(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn nature_manifest_with_spellbook(seed: u32, spellbook: &[&str]) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-nature": nature(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-site": site(),
    });
    if spellbook.contains(&"north-destroy") {
        cards["north-destroy"] = destroy_site();
    }
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-bottom-proof" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-bottom-proof-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": spellbook,
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

fn seed_with_spellbook(required_count: usize, start: u32) -> String {
    let spellbook = vec![
        "north-nature",
        "north-nature",
        "north-nature",
        "north-nature",
        "north-nature",
        "north-nature",
    ];
    (start..start + 256)
        .map(|seed| nature_manifest_with_spellbook(seed, &spellbook))
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .filter(|card| *card == "north-nature")
                .count()
                >= required_count
        })
        .expect("bounded seed with required Return to Nature opening cards")
}

fn seed_for_bottom_persistence(start: u32) -> String {
    let spellbook = vec!["north-nature"; 12];
    (start..start + 256)
        .find_map(|seed| {
            let encoded = nature_manifest_with_spellbook(seed, &spellbook);
            if opening_spell_ids(&encoded)
                .iter()
                .filter(|card| *card == "north-nature")
                .count()
                < 2
            {
                return None;
            }
            let mut session = opening_main(&encoded);
            let first_id = cast_nature_empty(&mut session);
            south_plays_c1(&mut session);
            cast_nature_return(&mut session, std::slice::from_ref(&first_id));
            if spellbook_bottom_id(&state(&session)) != first_id {
                return None;
            }
            advance_full_round(&mut session);
            (spellbook_bottom_id(&state(&session)) == first_id).then_some(encoded)
        })
        .expect("bounded seed with Return to Nature bottom persistence")
}

fn seed_with_nature_and_destroy(start: u32) -> String {
    let spellbook = vec![
        "north-nature",
        "north-nature",
        "north-destroy",
        "north-nature",
        "north-nature",
        "north-nature",
    ];
    (start..start + 256)
        .map(|seed| nature_manifest_with_spellbook(seed, &spellbook))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().filter(|card| *card == "north-nature").count() >= 2
                && hand.iter().any(|card| card == "north-destroy")
        })
        .expect("bounded seed with Nature and destroy-site opening cards")
}

fn cast_nature_empty(session: &mut Session) -> String {
    let (cast, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-nature"
            && descriptor["cemeteryCardInstanceIds"]
                .as_array()
                .is_none_or(Vec::is_empty)
    });
    cast["cardInstanceId"]
        .as_str()
        .expect("cast identity")
        .to_owned()
}

fn cast_nature_return(session: &mut Session, ids: &[String]) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-nature"
            && descriptor["cemeteryCardInstanceIds"] == json!(ids)
    });
    receipt
}

fn spellbook_bottom_id(snapshot: &Value) -> String {
    snapshot["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .last()
        .expect("bottom card")["instanceId"]
        .as_str()
        .expect("bottom identity")
        .to_owned()
}

fn atlas_bottom_id(snapshot: &Value) -> String {
    snapshot["players"]["north"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .last()
        .expect("bottom card")["instanceId"]
        .as_str()
        .expect("bottom identity")
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

fn deathrite_cemetery_bottom_manifest(seed: u32) -> String {
    let fixture = "cemetery-bottom-then-draw-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-nature": nature(),
            "north-rain": rain_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-nature",
                    "north-rain",
                    "north-rain",
                    "north-nature",
                    "north-rain",
                    "north-nature",
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

fn north_has_nature_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-nature", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteCemeteryBottomSetup {
    cemetery_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite_with_cemetery_card(
    encoded: &str,
) -> Option<PendingDeathriteCemeteryBottomSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    if !north_has_nature_and_rain(&state(&session)) {
        return None;
    }
    if nature_offers(&session).is_empty() {
        return None;
    }
    let rain = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let cemetery_id = rain.0["cardInstanceId"].as_str()?.to_owned();
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCemeteryBottomSetup {
        cemetery_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_cemetery_bottom_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cemetery_bottom_manifest)
        .find(|candidate| try_pending_deathrite_with_cemetery_card(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with cemetery-bottom Magic in hand")
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one deathrite interruption and resumed cemetery choice"
)]
fn rule_catalog_1070_cemetery_bottom_then_draw_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cemetery_bottom_seed_with(1070);
    let mut setup = try_pending_deathrite_with_cemetery_card(&encoded)
        .expect("complete cemetery-bottom Deathrite withheld setup");
    let cemetery_id = setup.cemetery_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "deathrite-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(!cemetery_ids(&paused, "north").contains(&cemetery_id));
    assert_eq!(
        paused["pendingDeathrites"]["continuation"]["magic"]["instanceId"],
        cemetery_id
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(nature_offers(session).is_empty());

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-deathrites"
            && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(cemetery_ids(&resumed, "north").contains(&cemetery_id));
    assert!(
        nature_offers(session)
            .iter()
            .any(|cards| cards == &vec![cemetery_id.clone()])
    );

    let library_top = resumed["players"]["north"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .first()
        .expect("card to draw")["instanceId"]
        .as_str()
        .expect("drawn identity")
        .to_owned();
    let (cast, granted) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-nature"
            && descriptor["cemeteryCardInstanceIds"] == json!([cemetery_id])
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
    assert_eq!(granted.events[1].payload["instanceId"], cemetery_id);
    assert_eq!(granted.events[1].payload["zone"], "spellbook");
    assert_eq!(
        granted.events[1].payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let after = state(session);
    assert!(!cemetery_ids(&after, "north").contains(&cemetery_id));
    assert_eq!(
        after["players"]["north"]["spellbook"]
            .as_array()
            .expect("north Spellbook after return")
            .last()
            .expect("bottom card")["instanceId"],
        cemetery_id
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north hand after draw")
            .iter()
            .any(|card| card["instanceId"] == library_top)
    );
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1723_returned_spell_stays_at_deck_bottom_after_turns_pass() {
    let encoded = seed_for_bottom_persistence(1723);
    let mut session = opening_main(&encoded);
    let first_id = cast_nature_empty(&mut session);
    south_plays_c1(&mut session);
    cast_nature_return(&mut session, std::slice::from_ref(&first_id));
    assert_eq!(spellbook_bottom_id(&state(&session)), first_id);
    advance_full_round(&mut session);
    assert_eq!(spellbook_bottom_id(&state(&session)), first_id);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1724_second_nature_without_cemetery_selection_still_draws() {
    let encoded = seed_with_spellbook(2, 1724);
    let mut session = opening_main(&encoded);
    let first = cast_nature_empty(&mut session);
    let second = cast_nature_empty(&mut session);
    assert_ne!(first, second);
    assert_eq!(cemetery_ids(&state(&session), "north").len(), 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1725_nature_returns_two_cemetery_cards_in_one_cast() {
    let encoded = seed_with_spellbook(3, 1725);
    let mut session = opening_main(&encoded);
    let first_id = cast_nature_empty(&mut session);
    south_plays_c1(&mut session);
    let second_id = cast_nature_empty(&mut session);
    let mut returned = vec![first_id.clone(), second_id.clone()];
    returned.sort_unstable();
    let granted = cast_nature_return(&mut session, &returned);
    assert_eq!(
        granted
            .events
            .iter()
            .filter(|event| event.event_type == "card-returned-to-deck-bottom")
            .count(),
        2
    );
    let after = state(&session);
    assert!(!cemetery_ids(&after, "north").contains(&first_id));
    assert!(!cemetery_ids(&after, "north").contains(&second_id));
    assert_eq!(spellbook_bottom_id(&after), returned[1]);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1726_nature_routes_site_and_spell_returns_to_matching_deck_bottoms() {
    let encoded = seed_with_nature_and_destroy(1726);
    let mut session = opening_main(&encoded);
    let magic_id = cast_nature_empty(&mut session);
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
    let mut returned = vec![magic_id.clone(), site_id.clone()];
    returned.sort_unstable();
    let granted = cast_nature_return(&mut session, &returned);
    assert_eq!(
        granted
            .events
            .iter()
            .filter(|event| event.event_type == "card-returned-to-deck-bottom")
            .count(),
        2
    );
    let after = state(&session);
    assert_eq!(atlas_bottom_id(&after), site_id);
    assert_eq!(spellbook_bottom_id(&after), magic_id);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1727_nature_returns_multiple_cards_in_instance_id_order() {
    let encoded = seed_with_spellbook(3, 1727);
    let mut session = opening_main(&encoded);
    let first_id = cast_nature_empty(&mut session);
    south_plays_c1(&mut session);
    let second_id = cast_nature_empty(&mut session);
    let mut sorted = vec![first_id, second_id];
    sorted.sort_unstable();
    let granted = cast_nature_return(&mut session, &sorted);
    let returned: Vec<_> = granted
        .events
        .iter()
        .filter(|event| event.event_type == "card-returned-to-deck-bottom")
        .map(|event| {
            event.payload["instanceId"]
                .as_str()
                .expect("returned identity")
                .to_owned()
        })
        .collect();
    assert_eq!(returned, sorted);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1728_second_nature_returns_a_newly_arrived_cemetery_card() {
    let encoded = seed_with_nature_and_destroy(1728);
    let mut session = opening_main(&encoded);
    cast_nature_empty(&mut session);
    south_plays_c1(&mut session);
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
    let granted = cast_nature_return(&mut session, std::slice::from_ref(&site_id));
    assert_eq!(granted.events[1].payload["zone"], "atlas");
    assert_eq!(atlas_bottom_id(&state(&session)), site_id);
    assert_exact_replay(&session);
}
