//! Direct proofs for teleport-target-one-diagonal Magic (RULE-CATALOG-0563–0564,
//! RULE-CATALOG-1032, RULE-CATALOG-1793–1798).
//!
//! Ordinary Magic targets a minion, Artifact, or Aura and teleports it one
//! diagonal step onto an existing location. Cardinal cells and stay are not
//! offered. No diagonal site means the spell has no legal cast. While Deathrites
//! wait for ordering, teleport Magic stays withheld until the chain drains.

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

fn fighter() -> Value {
    json!({
        "attack": 2,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn displace() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "teleportTargetMinionArtifactOrAuraOneDiagonal": true,
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

fn visitor() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 3,
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

fn displace_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "teleport-one-diagonal" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-teleport-one-diagonal-v1",
        },
        "cards": {
            "north-ally": fighter(),
            "north-avatar": avatar(),
            "north-displace": displace(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-displace",
                    "north-displace",
                    "north-displace",
                    "north-displace",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-minion",
                    "south-minion",
                    "south-visitor",
                    "south-visitor",
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
    let mut session = Session::new(encoded).expect("valid teleport-one-diagonal session");
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

fn displace_destinations(session: &Session, instance_id: &str) -> Vec<String> {
    let mut cells: Vec<String> = session
        .legal_actions()
        .expect("displace actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-displace"
                && action.descriptor["target"]["instanceId"] == instance_id
        })
        .filter_map(|action| {
            action.descriptor["targetLocation"]["cell"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

fn displace_casts(session: &Session) -> usize {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-displace"
        })
        .count()
}

fn seed_with(start: u32, required: &[&str]) -> String {
    (start..start + 2048)
        .map(displace_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn displace_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-displace")
                .count()
        })
        .unwrap_or_default()
}

fn seed_with_two_displace_spells_in_hand_after_cardinal_setup(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = displace_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if hand.iter().filter(|card| *card == "north-ally").count() < 1
                || !hand.iter().any(|card| card == "north-displace")
            {
                return None;
            }
            let (mut session, _) = opening_with_ally(&encoded);
            lay_cardinal_site(&mut session);
            (displace_spells_in_hand(&state(&session)) >= 2).then_some(encoded)
        })
        .expect("bounded seed with two Displace spells in hand after cardinal setup")
}

fn allies_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-ally")
                .count()
        })
        .unwrap_or_default()
}

fn seed_for_second_ally_displace(start: u32) -> String {
    (start..start + 2048)
        .find_map(|seed| {
            let encoded = displace_manifest(seed);
            let hand = opening_spell_ids(&encoded);
            if hand.iter().filter(|card| *card == "north-ally").count() < 2
                || !hand.iter().any(|card| card == "north-displace")
            {
                return None;
            }
            let (session, _) = opening_with_ally(&encoded);
            let snap = state(&session);
            (displace_spells_in_hand(&snap) >= 2 && allies_in_hand(&snap) >= 1).then_some(encoded)
        })
        .expect("bounded seed with spare ally and two Displace spells after path setup")
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

fn cast_displace(session: &mut Session, target_id: &str, cell: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-displace"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["targetLocation"]["cell"] == cell
    });
    receipt
}

fn summon_north_ally_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("ally instance identity")
        .to_owned()
}

fn south_summons_visitor_at_c4(session: &mut Session) -> String {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("visitor instance identity")
        .to_owned()
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

fn south_plays_c1_and_summons_minion(session: &mut Session) -> String {
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
        .expect("minion instance identity")
        .to_owned()
}

fn opening_with_diagonal_board(encoded: &str) -> Session {
    let mut session = opening_main(encoded);
    south_plays_c1(&mut session);
    lay_diagonal_site(&mut session);
    session
}

fn opening_with_ally_and_far_minion(encoded: &str) -> (Session, String, String) {
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
    let far_id = south_plays_c1_and_summons_minion(&mut session);
    lay_diagonal_site(&mut session);
    (session, ally_id, far_id)
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
    south_plays_c1(&mut session);
    (session, ally_id)
}

fn lay_diagonal_site(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D4"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
    });
}

fn lay_cardinal_site(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
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

fn deathrite_displace_manifest(seed: u32) -> String {
    let fixture = "teleport-one-diagonal-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-displace": displace(),
            "north-rain": rain_spell(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
            "south-visitor": visitor(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-displace",
                    "north-rain",
                    "north-rain",
                    "north-displace",
                    "north-rain",
                    "north-displace",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4]
                    .into_iter()
                    .chain(std::iter::repeat_n("south-visitor", 2))
                    .collect::<Vec<_>>(),
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_displace_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-displace", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteDisplaceSetup {
    deathrite_ids: [String; 2],
    session: Session,
    visitor_id: String,
}

fn try_lay_diagonal_site(session: &mut Session) -> bool {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D4"
    })
    .is_some()
        && try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn").is_some()
        && try_accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
        .is_some()
        && try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn").is_some()
        && try_accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
        .is_some()
        && try_accept_where(session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "D3"
        })
        .is_some()
}

fn try_pending_deathrite_with_ready_visitor(
    encoded: &str,
) -> Option<PendingDeathriteDisplaceSetup> {
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
    let visitor = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visitor"
            && descriptor["cell"] == "C4"
    })?;
    let visitor_id = visitor.0["cardInstanceId"].as_str()?.to_owned();
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
    if !try_lay_diagonal_site(&mut session) {
        return None;
    }
    if !north_has_displace_and_rain(&state(&session)) {
        return None;
    }
    if displace_destinations(&session, &visitor_id).is_empty() {
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
    Some(PendingDeathriteDisplaceSetup {
        deathrite_ids,
        session,
        visitor_id,
    })
}

fn deathrite_displace_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_displace_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_visitor(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with teleport Magic in hand")
}

#[test]
fn rule_catalog_0563_teleport_targets_a_minion_one_diagonal() {
    let encoded = seed_with(563, &["north-ally", "north-displace"]);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    lay_diagonal_site(&mut session);
    assert_eq!(displace_destinations(&session, &ally_id), ["D3"]);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C4");

    let (cast, teleported) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-displace"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == ally_id
            && descriptor["targetLocation"]["cell"] == "D3"
    });
    let types = event_types(&teleported);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(types.contains(&"unit-teleported"));
    assert!(!types.contains(&"unit-stepped"));
    let hop = teleported
        .events
        .iter()
        .find(|event| event.event_type == "unit-teleported")
        .expect("unit-teleported");
    assert_eq!(hop.payload["targetInstanceId"], ally_id);
    assert_eq!(hop.payload["from"]["cell"], "C4");
    assert_eq!(hop.payload["to"]["cell"], "D3");
    assert_eq!(
        teleported.events[0].payload["instanceId"],
        cast["cardInstanceId"]
    );
    assert_eq!(unit(&state(&session), &ally_id)["location"], "D3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0564_teleport_one_diagonal_is_unoffered_without_a_diagonal_site() {
    let encoded = seed_with(564, &["north-ally", "north-displace"]);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    lay_cardinal_site(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C4");
    assert!(displace_destinations(&session, &ally_id).is_empty());
    assert_eq!(displace_casts(&session), 0);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1032_teleport_one_diagonal_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_displace_seed_with(1032);
    let mut setup = try_pending_deathrite_with_ready_visitor(&encoded)
        .expect("complete teleport Deathrite withheld setup");
    let visitor_id = setup.visitor_id.clone();
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
    assert_eq!(unit(&paused, &visitor_id)["location"], "C4");
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(displace_destinations(session, &visitor_id).is_empty());
    assert_eq!(displace_casts(session), 0);

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
    assert_eq!(unit(&resumed, &visitor_id)["location"], "C4");
    assert_eq!(displace_destinations(session, &visitor_id), ["D3"]);

    let (_, teleported) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-displace"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == visitor_id
            && descriptor["targetLocation"]["cell"] == "D3"
    });
    let types = event_types(&teleported);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(types.contains(&"unit-teleported"));
    assert_eq!(unit(&state(session), &visitor_id)["location"], "D3");
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1793_teleported_minion_stays_at_its_destination_after_turns_pass() {
    let encoded = seed_with(1793, &["north-ally", "north-displace"]);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    lay_diagonal_site(&mut session);
    cast_displace(&mut session, &ally_id, "D3");
    assert_eq!(unit(&state(&session), &ally_id)["location"], "D3");
    advance_full_round(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "D3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1794_second_displace_still_has_zero_casts_with_only_a_cardinal_site() {
    let encoded = seed_with_two_displace_spells_in_hand_after_cardinal_setup(1794);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    lay_cardinal_site(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["location"], "C4");
    assert!(displace_destinations(&session, &ally_id).is_empty());
    assert_eq!(displace_casts(&session), 0);
    assert!(displace_spells_in_hand(&state(&session)) >= 2);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1795_displace_teleports_an_enemy_minion_one_diagonal() {
    let encoded = seed_with(1795, &["north-displace"]);
    let mut session = opening_with_diagonal_board(&encoded);
    let visitor_id = south_summons_visitor_at_c4(&mut session);
    assert_eq!(unit(&state(&session), &visitor_id)["location"], "C4");
    assert_eq!(displace_destinations(&session, &visitor_id), ["D3"]);
    let teleported = cast_displace(&mut session, &visitor_id, "D3");
    assert!(event_types(&teleported).contains(&"unit-teleported"));
    assert_eq!(unit(&state(&session), &visitor_id)["location"], "D3");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1796_displace_leaves_a_far_minion_at_its_cell() {
    let encoded = seed_with(1796, &["north-ally", "north-displace"]);
    let (mut session, ally_id, far_id) = opening_with_ally_and_far_minion(&encoded);
    cast_displace(&mut session, &ally_id, "D3");
    assert_eq!(unit(&state(&session), &ally_id)["location"], "D3");
    assert_eq!(unit(&state(&session), &far_id)["location"], "C1");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1797_displace_omits_a_far_cell_beyond_one_diagonal() {
    let encoded = seed_with(1797, &["north-ally", "north-displace"]);
    let (mut session, ally_id) = opening_with_ally(&encoded);
    lay_diagonal_site(&mut session);
    let offered = displace_destinations(&session, &ally_id);
    assert!(offered.contains(&"D3".to_owned()));
    assert!(!offered.contains(&"C1".to_owned()));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1798_second_displace_teleports_a_newly_summoned_ally() {
    let encoded = seed_for_second_ally_displace(1798);
    let (mut session, first_ally) = opening_with_ally(&encoded);
    lay_diagonal_site(&mut session);
    cast_displace(&mut session, &first_ally, "D3");
    assert_eq!(unit(&state(&session), &first_ally)["location"], "D3");
    let second_ally = summon_north_ally_at(&mut session, "C4");
    assert_eq!(unit(&state(&session), &second_ally)["location"], "C4");
    assert!(displace_destinations(&session, &second_ally).contains(&"D3".to_owned()));
    let teleported = cast_displace(&mut session, &second_ally, "D3");
    assert!(event_types(&teleported).contains(&"unit-teleported"));
    assert_eq!(
        teleported
            .events
            .iter()
            .find(|event| event.event_type == "unit-teleported")
            .expect("unit-teleported")
            .payload["targetInstanceId"],
        second_ally
    );
    assert_eq!(unit(&state(&session), &second_ally)["location"], "D3");
    assert_eq!(unit(&state(&session), &first_ally)["location"], "D3");
    assert_exact_replay(&session);
}
