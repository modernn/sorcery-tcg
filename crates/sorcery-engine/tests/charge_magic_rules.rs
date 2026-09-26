//! Direct proofs for grant-charge-to-ally-this-turn Magic
//! (RULE-CATALOG-0597–0598, RULE-CATALOG-1033, RULE-CATALOG-1623–1628).
//!
//! Ordinary Charge Magic offers every controlled ally and grants temporary
//! Charge through End Phase. A newly summoned minion can Move and Attack
//! immediately after the grant. With no allied minion in play the cast still
//! resolves as a paid no-op.

#[path = "common/mod.rs"]
mod common;
use common::modifier_sources;

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

fn ally() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn printed_charger() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "charge": true,
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn charge() -> Value {
    json!({
        "cardType": "magic",
        "grantChargeToAllyThisTurn": true,
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

fn charge_manifest_with_ally(seed: u32, north_ally: &Value) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "charge-ally" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-charge-ally-v1",
        },
        "cards": {
            "north-ally": north_ally,
            "north-avatar": avatar(),
            "north-charge": charge(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": ally(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-ally",
                    "north-charge",
                    "north-charge",
                    "north-charge",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn charge_with_ally_manifest(seed: u32) -> String {
    charge_manifest_with_ally(seed, &ally())
}

fn charge_empty_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "charge-empty" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-charge-empty-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-charge": charge(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-filler": ally(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-charge"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-filler"; 6],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn try_opening_main(encoded: &str) -> Option<Session> {
    let mut session = Session::new(encoded).ok()?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    Some(session)
}

fn opening_main(encoded: &str) -> Session {
    try_opening_main(encoded).expect("valid charge opening main")
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

fn has_move_and_attack(session: &Session, unit_id: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "move-and-attack"
                && action.descriptor["unitInstanceId"] == unit_id
        })
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected realm unit")
}

fn opening_spell_ids(encoded: &str) -> Vec<String> {
    state(&Session::new(encoded).expect("candidate session"))["players"]["north"]["hand"]
        ["spellbook"]
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

fn summon_north_ally(session: &mut Session) -> String {
    summon_north_ally_at(session, "C4")
}

fn summon_north_ally_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == cell
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned()
}

fn grant_charge(session: &mut Session, ally_id: &str) -> (Value, Receipt, Receipt) {
    let (cast, cast_receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-charge"
    });
    let source = cast["cardInstanceId"].clone();
    let (_, choice_receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "choose-ability"
            && descriptor["sourceInstanceId"] == source
            && descriptor["target"]["instanceId"] == ally_id
    });
    (cast, cast_receipt, choice_receipt)
}

fn complete_pending_draws(session: &mut Session) {
    for _ in 0..4 {
        if try_accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        })
        .is_none()
            && try_accept_where(session, |descriptor| descriptor["kind"] == "draw").is_none()
        {
            break;
        }
    }
}

fn try_south_passive_turn(session: &mut Session) -> Option<()> {
    complete_pending_draws(session);
    if session
        .legal_actions()
        .ok()?
        .iter()
        .all(|action| action.descriptor["kind"] != "end-turn")
    {
        try_accept_where(session, |descriptor| {
            descriptor["kind"] == "play-site"
                && descriptor["cardId"] == "south-site"
                && descriptor["cell"] == "C1"
        })?;
        complete_pending_draws(session);
    }
    try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    Some(())
}

fn try_through_south_pass_to_north_main(session: &mut Session) -> Option<()> {
    try_south_passive_turn(session)?;
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    complete_pending_draws(session);
    Some(())
}

fn seed_with_ally_and_charge(north_ally: &Value, start: u32) -> String {
    (start..start + 256)
        .map(|seed| charge_manifest_with_ally(seed, north_ally))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally") && hand.iter().any(|id| id == "north-charge")
        })
        .expect("bounded seed with ally and Charge in the opening hand")
}

fn can_summon_north_ally_at(session: &Session, cell: &str) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-ally"
                && action.descriptor["cell"] == cell
        })
    })
}

fn hand_has_north_ally(session: &Session) -> bool {
    state(session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-ally"))
}

fn prepare_fresh_summon_on_later_turn(session: &mut Session) -> bool {
    try_through_south_pass_to_north_main(session).is_some()
        && hand_has_north_ally(session)
        && (can_summon_north_ally_at(session, "C3")
            || (try_accept_where(session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
            })
            .is_some()
                && can_summon_north_ally_at(session, "C3")))
}

fn try_expire_grant_charge(session: &mut Session, ally_id: &str) -> Option<String> {
    let (_, ended) = try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn")?;
    if !ended
        .events
        .iter()
        .any(|event| event.event_type == "charge-expired" && event.payload["instanceId"] == ally_id)
    {
        return None;
    }
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    })?;
    Some(ally_id.to_owned())
}

fn try_setup_later_fresh_summon(encoded: &str) -> Option<(Session, String)> {
    let mut session = try_opening_main(encoded)?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let first_id = summoned["cardInstanceId"].as_str()?.to_owned();
    let (cast, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-charge"
    })?;
    let source = cast["cardInstanceId"].clone();
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "choose-ability"
            && descriptor["sourceInstanceId"] == source
            && descriptor["target"]["instanceId"] == first_id
    })?;
    try_expire_grant_charge(&mut session, &first_id)?;
    if !prepare_fresh_summon_on_later_turn(&mut session) {
        return None;
    }
    let (second, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C3"
    })?;
    let second_id = second["cardInstanceId"].as_str()?.to_owned();
    Some((session, second_id))
}

fn setup_later_fresh_summon(north_ally: &Value, start: u32) -> (Session, String) {
    let encoded = (start..start + 2048)
        .map(|seed| charge_manifest_with_ally(seed, north_ally))
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|id| id == "north-ally")
                && hand.iter().any(|id| id == "north-charge")
                && try_setup_later_fresh_summon(candidate).is_some()
        })
        .expect("bounded seed that can summon a fresh ally on a later north Main phase");
    try_setup_later_fresh_summon(&encoded).expect("replay later fresh summon setup")
}

fn charge_available(session: &Session) -> bool {
    session.legal_actions().is_ok_and(|actions| {
        actions.iter().any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-charge"
        })
    })
}

fn deathrite_charge_manifest(seed: u32) -> String {
    let fixture = "charge-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-ally": ally(),
            "north-avatar": avatar(),
            "north-charge": charge(),
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
                    "north-ally",
                    "north-charge",
                    "north-rain",
                    "north-charge",
                    "north-rain",
                    "north-charge",
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

fn north_has_charge_and_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-charge", "north-rain"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteChargeSetup {
    ally_id: String,
    deathrite_ids: [String; 2],
    session: Session,
}

fn try_pending_deathrite(encoded: &str) -> Option<PendingDeathriteChargeSetup> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = ally.0["cardInstanceId"].as_str()?.to_owned();
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
    if !north_has_charge_and_rain(&state(&session)) {
        return None;
    }
    if !charge_available(&session) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "trigger-order" {
        return None;
    }
    let mut deathrite_ids = [
        first.0["cardInstanceId"].as_str()?.to_owned(),
        second.0["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathriteChargeSetup {
        ally_id,
        deathrite_ids,
        session,
    })
}

fn deathrite_charge_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_charge_manifest)
        .find(|candidate| try_pending_deathrite(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Charge Magic in hand")
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
fn rule_catalog_0597_charge_magic_lets_a_summoning_sick_ally_move_and_attack() {
    let encoded = (597..597 + 512)
        .map(charge_with_ally_manifest)
        .find(|candidate| {
            let preview = opening_main(candidate);
            preview
                .legal_actions()
                .expect("ally setup actions")
                .iter()
                .any(|action| {
                    action.descriptor["kind"] == "summon-minion"
                        && action.descriptor["cardId"] == "north-ally"
                })
                && preview
                    .legal_actions()
                    .expect("charge setup actions")
                    .iter()
                    .any(|action| {
                        action.descriptor["kind"] == "cast-magic"
                            && action.descriptor["cardId"] == "north-charge"
                    })
        })
        .expect("bounded seed with summonable ally and Charge after opening");
    let mut session = opening_main(&encoded);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-ally"
    });
    let ally_id = summoned["cardInstanceId"]
        .as_str()
        .expect("ally identity")
        .to_owned();
    assert!(!has_move_and_attack(&session, &ally_id));

    let (cast, cast_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-charge"
    });
    assert_eq!(event_types(&cast_receipt), ["magic-cast"]);
    let (_, granted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "choose-ability"
            && descriptor["sourceInstanceId"] == cast["cardInstanceId"]
            && descriptor["target"]["instanceId"] == ally_id
    });
    assert_eq!(
        event_types(&granted),
        [
            "ability-choice-committed",
            "charge-granted",
            "magic-resolved"
        ]
    );
    assert!(has_move_and_attack(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0598_charge_magic_grants_charge_to_the_avatar_when_no_minion_is_in_play() {
    let encoded = (598..598 + 256)
        .map(charge_empty_manifest)
        .find(|candidate| {
            Session::new(candidate).is_ok()
                && state(&Session::new(candidate).expect("candidate session"))["players"]["north"]
                    ["hand"]["spellbook"]
                    .as_array()
                    .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-charge"))
        })
        .expect("bounded seed with Charge in opening hand");
    let mut session = opening_main(&encoded);
    assert!(
        state(&session)["realm"]["units"]
            .as_array()
            .is_none_or(|units| units.iter().all(|unit| unit["kind"] != "minion"))
    );
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar identity")
        .to_owned();

    let (cast, cast_receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-charge"
    });
    assert_eq!(event_types(&cast_receipt), ["magic-cast"]);
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "choose-ability"
            && descriptor["sourceInstanceId"] == cast["cardInstanceId"]
            && descriptor["target"]["instanceId"] == avatar_id
    });
    assert_eq!(
        event_types(&receipt),
        [
            "ability-choice-committed",
            "charge-granted",
            "magic-resolved"
        ]
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1033_charge_magic_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_charge_seed_with(1033);
    let mut setup =
        try_pending_deathrite(&encoded).expect("complete Charge Deathrite withheld setup");
    let ally_id = setup.ally_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let session = &mut setup.session;
    let paused = state(session);
    assert_eq!(paused["phase"], "trigger-order");
    assert_eq!(paused["decisionSeat"], "south");
    assert!(deathrite_ids.iter().all(|instance_id| {
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != *instance_id)
    }));
    assert!(
        paused["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == ally_id)
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(!charge_available(session));
    assert!(!has_move_and_attack(session, &ally_id));

    let order_sources: Vec<_> = session
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .map(|action| {
            action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned()
        })
        .collect();
    assert_eq!(order_sources, deathrite_ids);

    accept_where(session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == deathrite_ids[0]
    });

    let resumed = state(session);
    assert_eq!(resumed["phase"], "main");
    assert_eq!(resumed["decisionSeat"], "north");
    assert!(resumed["pendingDeathrites"].is_null());
    assert!(charge_available(session));

    let (_cast, cast_receipt, receipt) = grant_charge(session, &ally_id);
    assert_eq!(event_types(&cast_receipt), ["magic-cast"]);
    assert_eq!(
        event_types(&receipt),
        [
            "ability-choice-committed",
            "charge-granted",
            "magic-resolved"
        ]
    );
    assert!(has_move_and_attack(session, &ally_id));
    assert_exact_replay(session);
}

#[test]
fn rule_catalog_1623_printed_charge_moves_and_attacks_without_grant() {
    let encoded = seed_with_ally_and_charge(&printed_charger(), 1623);
    let mut session = opening_main(&encoded);
    let ally_id = summon_north_ally(&mut session);
    assert_eq!(unit(&state(&session), &ally_id)["summoningSickness"], true);
    assert!(has_move_and_attack(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1624_granted_charge_moves_and_attacks_before_end_of_turn() {
    let encoded = seed_with_ally_and_charge(&ally(), 1624);
    let mut session = opening_main(&encoded);
    let ally_id = summon_north_ally(&mut session);
    assert!(!has_move_and_attack(&session, &ally_id));
    let (descriptor, cast_receipt, receipt) = grant_charge(&mut session, &ally_id);
    assert_eq!(event_types(&cast_receipt), ["magic-cast"]);
    assert_eq!(
        event_types(&receipt),
        [
            "ability-choice-committed",
            "charge-granted",
            "magic-resolved"
        ]
    );
    assert_eq!(
        modifier_sources(unit(&state(&session), &ally_id), "charge"),
        json!([descriptor["cardInstanceId"]])
    );
    assert!(has_move_and_attack(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1625_granted_charge_expires_before_fresh_ally_moves_on_later_turn() {
    let (session, second_id) = setup_later_fresh_summon(&ally(), 1625);
    assert_eq!(
        unit(&state(&session), &second_id)["summoningSickness"],
        true
    );
    assert!(!has_move_and_attack(&session, &second_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1626_printed_charge_still_moves_after_grant_expires_on_later_turn() {
    let (session, second_id) = setup_later_fresh_summon(&printed_charger(), 1626);
    assert!(has_move_and_attack(&session, &second_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1627_printed_and_granted_charge_compose_while_grant_is_active() {
    let encoded = seed_with_ally_and_charge(&printed_charger(), 1627);
    let mut session = opening_main(&encoded);
    let ally_id = summon_north_ally(&mut session);
    let (descriptor, cast_receipt, receipt) = grant_charge(&mut session, &ally_id);
    assert_eq!(event_types(&cast_receipt), ["magic-cast"]);
    assert_eq!(
        event_types(&receipt),
        [
            "ability-choice-committed",
            "charge-granted",
            "magic-resolved"
        ]
    );
    assert_eq!(
        modifier_sources(unit(&state(&session), &ally_id), "charge"),
        json!([descriptor["cardInstanceId"]])
    );
    assert!(has_move_and_attack(&session, &ally_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1628_printed_charge_outlasts_expired_grant_while_plain_ally_cannot_move() {
    let (plain, plain_second) = setup_later_fresh_summon(&ally(), 1628);
    assert!(!has_move_and_attack(&plain, &plain_second));

    let (session, second_id) = setup_later_fresh_summon(&printed_charger(), 1628);
    assert!(has_move_and_attack(&session, &second_id));
    assert_exact_replay(&session);
}
