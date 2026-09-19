//! Direct proofs for cemetery Site return Magic (RULE-CATALOG-0639–0640,
//! RULE-CATALOG-1071, RULE-CATALOG-2163–2168).
//!
//! Cemetery Site return offers only Sites in the caster's own cemetery and
//! restores the chosen instance to the hidden Atlas hand. An empty own
//! cemetery is a paid no-choice resolution. While Deathrites wait for
//! ordering, cemetery Site return Magic stays withheld until the chain drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn destroy_spell() -> Value {
    json!({
        "cardType": "magic",
        "destroyTargetSite": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    })
}

fn return_spell() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetSiteFromOwnCemetery": true,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
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

fn cemetery_site_manifest(seed: u32, include_destroy: bool) -> String {
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-return": return_spell(),
        "north-site": site(),
        "south-avatar": avatar(),
        "south-minion": minion(),
        "south-site": site(),
    });
    let north_spellbook = if include_destroy {
        cards["north-destroy"] = destroy_spell();
        json!([
            "north-destroy",
            "north-return",
            "north-destroy",
            "north-return",
            "north-destroy",
            "north-return"
        ])
    } else {
        json!(vec!["north-return"; 6])
    };
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-site-return" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-site-return-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid cemetery-site session");
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

fn cemetery_site_cast_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("cemetery site actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-return"
        })
        .filter_map(|action| {
            action.descriptor["cemeteryMinionInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn north_hand_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north hand")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn seed_with(required: &[&str], include_destroy: bool, start: u32) -> String {
    (start..start + 256)
        .map(|seed| cemetery_site_manifest(seed, include_destroy))
        .find(|candidate| {
            Session::new(candidate).is_ok_and(|preview| {
                let snapshot = state(&preview);
                let hand = north_hand_ids(&snapshot);
                required.iter().all(|id| hand.iter().any(|card| card == id))
            })
        })
        .expect("bounded seed with required opening cards")
}

fn setup_own_cemetery_site(session: &mut Session) -> (String, String) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B4"
    });
    let site_id = state(session)["realm"]["sites"]["B4"]["instanceId"]
        .as_str()
        .expect("B4 site identity")
        .to_owned();
    let (_, destroyed) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "B4"
            && descriptor["targetSiteInstanceId"] == site_id
    });
    assert!(
        destroyed
            .events
            .iter()
            .any(|event| event.event_type == "site-destroyed")
    );
    let destroy_id = state(session)["players"]["north"]["cemetery"]
        .as_array()
        .expect("north cemetery")
        .iter()
        .find(|card| card["cardId"] == "north-destroy")
        .expect("destroy Magic in cemetery")["instanceId"]
        .as_str()
        .expect("destroy identity")
        .to_owned();
    (site_id, destroy_id)
}

#[test]
fn rule_catalog_0639_cemetery_site_return_restores_own_cemetery_site_to_hidden_atlas() {
    let encoded = seed_with(&["north-destroy", "north-return"], true, 639);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_site_cast_ids(&session), Vec::<String>::new());
    let (site_id, destroy_id) = setup_own_cemetery_site(&mut session);
    assert_ne!(destroy_id, site_id);
    let cemetery_targets = cemetery_site_cast_ids(&session);
    assert!(!cemetery_targets.is_empty());
    assert!(cemetery_targets.iter().all(|id| id == &site_id));
    assert!(!cemetery_targets.iter().any(|id| id == &destroy_id));
    let before = state(&session);
    let before_atlas = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let south_observation = session.observe(Seat::South);
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-returned-to-hand", "magic-resolved"]
    );
    assert_eq!(receipt.events[1].payload["cardId"], "north-site");
    assert_eq!(receipt.events[1].payload["instanceId"], site_id);
    assert_eq!(receipt.events[1].payload["owner"], "north");
    assert_eq!(receipt.events[1].payload["seat"], "north");
    assert_eq!(
        receipt.events[1].payload["sourceInstanceId"],
        descriptor["cardInstanceId"]
    );
    assert!(receipt.events[1].payload.get("cell").is_none());
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "minion-returned-to-hand"
                || event.event_type == "magic-returned-to-hand"
                || event.event_type == "artifact-returned-to-hand")
    );
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        before_atlas + 1
    );
    assert!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .iter()
            .any(|card| card["instanceId"] == site_id)
    );
    assert!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north Spellbook")
            .iter()
            .all(|card| card["instanceId"] != site_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .all(|card| card["instanceId"] != site_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == destroy_id)
    );
    assert_eq!(session.observe(Seat::South), south_observation);
    let south_view = session.public_view(Seat::South).expect("South public view");
    assert_eq!(
        south_view["players"]["north"]["hand"]["atlas"],
        before_atlas + 1
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
    let checkpoint = create_game_checkpoint(&session).expect("cemetery-site checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized cemetery-site");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed cemetery-site");
    assert_eq!(
        resume_game_checkpoint(&parsed)
            .expect("resumed cemetery-site session")
            .state_hash()
            .expect("resumed state hash"),
        session.state_hash().expect("session state hash")
    );
}

#[test]
fn rule_catalog_0640_cemetery_site_return_is_a_paid_noop_without_cemetery_site() {
    let encoded = seed_with(&["north-return"], false, 640);
    let mut session = opening_main(&encoded);
    assert_eq!(cemetery_site_cast_ids(&session), Vec::<String>::new());
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-returned-to-hand"
                || event.event_type == "magic-returned-to-hand")
    );
    assert_eq!(cemetery_site_cast_ids(&session), Vec::<String>::new());
    let after = state(&session);
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .len(),
        1
    );
    assert_eq!(after["terminal"]["status"], "active");
    assert_exact_replay(&session);
}

fn deathrite_cemetery_site_manifest(seed: u32) -> String {
    let fixture = "cemetery-site-return-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_spell(),
            "north-rain": rain_spell(),
            "north-return": return_spell(),
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
                    "north-destroy",
                    "north-return",
                    "north-rain",
                    "north-destroy",
                    "north-return",
                    "north-rain",
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

fn north_has_return_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-return", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

fn cemetery_has_card(snapshot: &Value, owner: &str, instance_id: &str) -> bool {
    snapshot["players"][owner]["cemetery"]
        .as_array()
        .is_some_and(|cemetery| {
            cemetery
                .iter()
                .any(|card| card["instanceId"] == instance_id)
        })
}

struct PendingDeathriteCemeterySiteSetup {
    deathrite_ids: [String; 2],
    session: Session,
    site_id: String,
}

fn try_pending_deathrite_with_cemetery_site_target(
    encoded: &str,
) -> Option<PendingDeathriteCemeterySiteSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let opening_hand = north_hand_ids(&state(&session));
    if !opening_hand.iter().any(|card| card == "north-destroy")
        || !opening_hand.iter().any(|card| card == "north-return")
        || !opening_hand.iter().any(|card| card == "north-rain")
    {
        return None;
    }
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
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B4"
    })?;
    let site_id = state(&session)["realm"]["sites"]["B4"]["instanceId"]
        .as_str()?
        .to_owned();
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "B4"
            && descriptor["targetSiteInstanceId"] == site_id
    })?;
    if !north_has_return_and_rain(&state(&session)) {
        return None;
    }
    if cemetery_site_cast_ids(&session).is_empty() {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteCemeterySiteSetup {
        deathrite_ids,
        session,
        site_id,
    })
}

fn deathrite_cemetery_site_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_cemetery_site_manifest)
        .find(|candidate| try_pending_deathrite_with_cemetery_site_target(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with cemetery-site return Magic in hand",
        )
}

#[test]
fn rule_catalog_1071_cemetery_site_return_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_cemetery_site_seed_with(1071);
    let mut setup = try_pending_deathrite_with_cemetery_site_target(&encoded)
        .expect("complete cemetery-site return Deathrite withheld setup");
    let site_id = setup.site_id.clone();
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
    assert!(cemetery_has_card(&paused, "north", &site_id));
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(cemetery_site_cast_ids(session).is_empty());

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
    assert!(cemetery_has_card(&resumed, "north", &site_id));
    assert_eq!(
        cemetery_site_cast_ids(session).as_slice(),
        std::slice::from_ref(&site_id)
    );

    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == site_id
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-returned-to-hand", "magic-resolved"]
    );
    assert!(
        state(session)["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .iter()
            .any(|card| card["instanceId"] == site_id)
    );
    assert!(!cemetery_has_card(&state(session), "north", &site_id));
    assert_exact_replay(session);
}

fn cemetery_site_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "cemetery-site-return-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-cemetery-site-return-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-destroy": destroy_spell(),
            "north-return": return_spell(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec![
                    "north-destroy",
                    "north-return",
                    "north-destroy",
                    "north-return",
                    "north-destroy",
                    "north-return",
                    "north-destroy",
                    "north-return",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 24],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_spell_ids(encoded: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    north_hand_ids(&state(&preview))
}

fn supplemental_seed_with_start(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .chain(639..639 + 2048)
        .map(cemetery_site_supplemental_manifest)
        .find(|candidate| {
            required
                .iter()
                .all(|id| opening_spell_ids(candidate).iter().any(|card| card == *id))
        })
        .expect("bounded seed with cemetery Site return supplemental opening cards")
}

fn spells_in_hand(snapshot: &Value, card_id: &str) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| hand.iter().filter(|card| card["cardId"] == card_id).count())
        .unwrap_or_default()
}

fn return_spells_in_hand(snapshot: &Value) -> usize {
    spells_in_hand(snapshot, "north-return")
}

fn destroy_spells_in_hand(snapshot: &Value) -> usize {
    spells_in_hand(snapshot, "north-destroy")
}

fn atlas_has_instance(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["hand"]["atlas"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["instanceId"] == instance_id))
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .is_ok_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn decline_attack_if_needed(session: &mut Session) {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        accept_where(session, |descriptor| descriptor["kind"] == "decline-attack");
    }
}

fn end_turn_if_offered(session: &mut Session) {
    decline_attack_if_needed(session);
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
}

fn pass_turn_to_north_spellbook(session: &mut Session) {
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    decline_attack_if_needed(session);
    end_turn_if_offered(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn cast_return_target(session: &mut Session, site_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == site_id
    });
    receipt
}

fn try_setup_own_cemetery_site(session: &mut Session) -> Option<(String, String)> {
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "B4"
    })?;
    let site_id = state(session)["realm"]["sites"]["B4"]["instanceId"]
        .as_str()?
        .to_owned();
    let (_, destroyed) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == "B4"
            && descriptor["targetSiteInstanceId"] == site_id
    })?;
    event_types(&destroyed)
        .contains(&"site-destroyed")
        .then_some(())?;
    let destroy_id = state(session)["players"]["north"]["cemetery"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == "north-destroy")?["instanceId"]
        .as_str()?
        .to_owned();
    Some((site_id, destroy_id))
}

fn try_destroy_site_at(session: &mut Session, cell: &str) -> Option<String> {
    let site_id = state(session)["realm"]["sites"].get(cell)?["instanceId"]
        .as_str()?
        .to_owned();
    if state(session)["realm"]["sites"][cell].get("rubble") == Some(&json!(true)) {
        return None;
    }
    let (_, destroyed) = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-destroy"
            && descriptor["targetLocation"]["cell"] == cell
            && descriptor["targetSiteInstanceId"] == site_id
    })?;
    event_types(&destroyed)
        .contains(&"site-destroyed")
        .then_some(site_id)
}

fn try_end_turn(session: &mut Session) -> Option<()> {
    while offers(session, |descriptor| descriptor["kind"] == "decline-attack") {
        try_accept_where(session, |descriptor| descriptor["kind"] == "decline-attack")?;
    }
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn").map(|_| ())
}

fn try_pass_turn(session: &mut Session) -> Option<()> {
    try_end_turn(session)?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "draw")?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    try_end_turn(session)?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "draw").map(|_| ())
}

fn try_play_any_north_site(session: &mut Session) -> Option<(String, String)> {
    let cell = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "play-site" && action.descriptor["cardId"] == "north-site"
        })?
        .descriptor["cell"]
        .as_str()?
        .to_owned();
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == cell
    })?;
    let site_id = state(session)["realm"]["sites"][&cell]["instanceId"]
        .as_str()?
        .to_owned();
    Some((cell, site_id))
}

fn try_pass_north_spellbook(session: &mut Session) -> Option<()> {
    try_end_turn(session)?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "draw")?;
    let _ = try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    });
    try_end_turn(session)?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })
    .map(|_| ())
}

fn try_destroy_extra_own_site(session: &mut Session) -> Option<String> {
    for _ in 0..5 {
        if destroy_spells_in_hand(&state(session)) >= 1
            && let Some((cell, _)) = try_play_any_north_site(session)
        {
            return try_destroy_site_at(session, &cell);
        }
        try_pass_turn(session)?;
    }
    None
}

fn try_reach_second_cemetery_return(session: &mut Session) -> Option<String> {
    for _ in 0..8 {
        if return_spells_in_hand(&state(session)) >= 1
            && let Some(site_id) = cemetery_site_cast_ids(session).into_iter().next()
        {
            return Some(site_id);
        }
        if destroy_spells_in_hand(&state(session)) >= 1
            && let Some((cell, _)) = try_play_any_north_site(session)
        {
            let _ = try_destroy_site_at(session, &cell);
            continue;
        }
        try_pass_north_spellbook(session)?;
    }
    None
}

fn try_two_cemetery_sites_prefix(encoded: &str) -> Option<(Session, [String; 2])> {
    let mut session = opening_main(encoded);
    let (first, _) = try_setup_own_cemetery_site(&mut session)?;
    try_pass_turn(&mut session)?;
    let second = try_destroy_extra_own_site(&mut session)?;
    let mut offered = cemetery_site_cast_ids(&session);
    offered.sort();
    offered.dedup();
    (offered.contains(&first) && offered.contains(&second) && first != second)
        .then_some((session, [first, second]))
}

fn seed_with_two_cemetery_sites(start: u32) -> String {
    (start..start + 2048)
        .chain(639..639 + 2048)
        .find_map(|seed| {
            let encoded = cemetery_site_supplemental_manifest(seed);
            if !["north-destroy", "north-return"]
                .iter()
                .all(|id| opening_spell_ids(&encoded).iter().any(|card| card == *id))
            {
                return None;
            }
            try_two_cemetery_sites_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed with two own cemetery sites")
}

fn try_pass_turn_with_enemy_site(session: &mut Session) -> Option<()> {
    try_end_turn(session)?;
    try_accept_where(session, |descriptor| descriptor["kind"] == "draw")?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cardId"] == "south-site"
    })?;
    try_end_turn(session)?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })
    .map(|_| ())
}

fn try_second_return_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let (first_id, _) = try_setup_own_cemetery_site(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == first_id
    })?;
    try_pass_turn_with_enemy_site(&mut session)?;
    try_reach_second_cemetery_return(&mut session).map(|site_id| (session, site_id))
}

fn seed_for_second_return_enemy_arrival(start: u32) -> String {
    (start..start + 2048)
        .chain(639..639 + 2048)
        .find_map(|seed| {
            let encoded = cemetery_site_supplemental_manifest(seed);
            if !["north-destroy", "north-return"]
                .iter()
                .all(|id| opening_spell_ids(&encoded).iter().any(|card| card == *id))
            {
                return None;
            }
            try_second_return_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Site return enemy-arrival setup")
}

fn try_second_return_new_destroy_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let (first_id, _) = try_setup_own_cemetery_site(&mut session)?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-return"
            && descriptor["cemeteryMinionInstanceId"] == first_id
    })?;
    try_reach_second_cemetery_return(&mut session).map(|site_id| (session, site_id))
}

fn seed_for_second_return_new_destroy(start: u32) -> String {
    (start..start + 2048)
        .chain(639..639 + 2048)
        .find_map(|seed| {
            let encoded = cemetery_site_supplemental_manifest(seed);
            if !["north-destroy", "north-return"]
                .iter()
                .all(|id| opening_spell_ids(&encoded).iter().any(|card| card == *id))
            {
                return None;
            }
            try_second_return_new_destroy_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second cemetery Site return new-destroy setup")
}

#[test]
fn rule_catalog_2163_returned_site_stays_in_atlas_after_turns_pass() {
    let encoded = supplemental_seed_with_start(2163, &["north-destroy", "north-return"]);
    let mut session = opening_main(&encoded);
    let (site_id, _) = try_setup_own_cemetery_site(&mut session).expect("own cemetery site");
    cast_return_target(&mut session, &site_id);
    assert!(atlas_has_instance(&state(&session), "north", &site_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(atlas_has_instance(&state(&session), "north", &site_id));
    assert!(!cemetery_has_card(&state(&session), "north", &site_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2164_second_return_without_a_cemetery_site_is_a_paid_noop() {
    let encoded = supplemental_seed_with_start(2164, &["north-destroy", "north-return"]);
    let mut session = opening_main(&encoded);
    let (site_id, _) = try_setup_own_cemetery_site(&mut session).expect("own cemetery site");
    cast_return_target(&mut session, &site_id);
    if return_spells_in_hand(&state(&session)) < 1 {
        pass_turn_to_north_spellbook(&mut session);
    }
    assert!(return_spells_in_hand(&state(&session)) >= 1);
    assert!(cemetery_site_cast_ids(&session).is_empty());
    let (descriptor, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-return"
    });
    assert!(descriptor.get("cemeteryMinionInstanceId").is_none());
    assert_eq!(event_types(&receipt), ["magic-cast", "magic-resolved"]);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "site-returned-to-hand")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2165_second_return_returns_a_newly_destroyed_site_after_enemy_site_placement() {
    let encoded = seed_for_second_return_enemy_arrival(2165);
    let (mut session, site_id) = try_second_return_enemy_arrival_prefix(&encoded)
        .expect("second cemetery Site return enemy-arrival prefix");
    let receipt = cast_return_target(&mut session, &site_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-returned-to-hand", "magic-resolved"]
    );
    assert!(atlas_has_instance(&state(&session), "north", &site_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2166_cemetery_site_return_offers_every_own_cemetery_site() {
    let encoded = seed_with_two_cemetery_sites(2166);
    let (session, site_ids) =
        try_two_cemetery_sites_prefix(&encoded).expect("two own cemetery sites");
    let mut offered = cemetery_site_cast_ids(&session);
    offered.sort();
    offered.dedup();
    assert_eq!(offered.len(), 2);
    for site_id in &site_ids {
        assert!(offered.contains(site_id));
    }
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2167_cemetery_site_return_leaves_an_unselected_cemetery_site_in_place() {
    let encoded = seed_with_two_cemetery_sites(2167);
    let (mut session, site_ids) =
        try_two_cemetery_sites_prefix(&encoded).expect("two own cemetery sites");
    let returned_id = &site_ids[0];
    cast_return_target(&mut session, returned_id);
    assert!(atlas_has_instance(&state(&session), "north", returned_id));
    assert!(cemetery_has_card(&state(&session), "north", &site_ids[1]));
    assert!(!cemetery_has_card(&state(&session), "north", returned_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2168_second_return_returns_a_newly_destroyed_site() {
    let encoded = seed_for_second_return_new_destroy(2168);
    let (mut session, site_id) = try_second_return_new_destroy_prefix(&encoded)
        .expect("second cemetery Site return new-destroy prefix");
    let receipt = cast_return_target(&mut session, &site_id);
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "site-returned-to-hand", "magic-resolved"]
    );
    assert!(atlas_has_instance(&state(&session), "north", &site_id));
    assert_exact_replay(&session);
}
