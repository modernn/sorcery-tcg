//! Direct proofs for Genesis untap-adjacent-allies (RULE-CATALOG-0537–0538).
//!
//! On entry, a minion untaps tapped allies that share its region and stand on
//! a bordering cell. Same-cell allies are not adjacent. Enemy units are not
//! untapped. The Avatar counts as an ally.

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

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn hob() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "genesisUntapAdjacentAllies": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn hob_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "genesis-untap-adjacent" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-genesis-untap-adjacent-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-hob": hob(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-hob",
                    "north-hob",
                    "north-hob",
                    "north-hob",
                    "north-hob",
                    "north-hob",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 6],
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
    let mut session = Session::new(encoded).expect("valid genesis-untap-adjacent session");
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

fn avatar_instance_id(snapshot: &Value, seat: &str) -> String {
    snapshot["players"][seat]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("avatar instance identity")
        .to_owned()
}

fn seed_with_hob() -> String {
    (537..537 + 256)
        .map(hob_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate)
                .iter()
                .any(|id| id == "north-hob")
        })
        .expect("bounded seed with Genesis hob in the opening hand")
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
            && descriptor["cardId"] == "south-dummy"
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
fn rule_catalog_0537_genesis_untaps_an_adjacent_tapped_avatar() {
    let encoded = seed_with_hob();
    let mut session = opening_main(&encoded);
    let enemy_id = south_plays_c1(&mut session);
    let before_site = state(&session);
    let north_avatar = avatar_instance_id(&before_site, "north");
    assert_eq!(before_site["players"]["north"]["avatar"]["tapped"], false);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["tapped"],
        true
    );

    let (_, summoned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-hob"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    assert_eq!(
        event_types(&summoned),
        ["minion-summoned", "avatar-untapped"]
    );
    assert_eq!(summoned.events[1].payload["instanceId"], north_avatar);
    assert!(
        !summoned
            .events
            .iter()
            .any(|event| event.event_type.ends_with("-untapped")
                && event.payload["instanceId"] == enemy_id)
    );
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["tapped"],
        false
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0538_genesis_does_not_untap_a_same_cell_avatar() {
    let encoded = seed_with_hob();
    let mut session = opening_main(&encoded);
    let before = state(&session);
    let north_avatar = avatar_instance_id(&before, "north");
    assert_eq!(before["players"]["north"]["avatar"]["tapped"], true);

    let (_, summoned) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-hob"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    assert_eq!(event_types(&summoned), ["minion-summoned"]);
    assert!(
        !summoned
            .events
            .iter()
            .any(|event| event.event_type == "avatar-untapped"
                && event.payload["instanceId"] == north_avatar)
    );
    assert_eq!(
        state(&session)["players"]["north"]["avatar"]["tapped"],
        true
    );
    assert_exact_replay(&session);
}
