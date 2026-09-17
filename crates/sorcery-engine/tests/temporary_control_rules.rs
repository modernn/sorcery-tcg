//! Direct proofs for temporary enemy-minion control (RULE-CATALOG-0513–0524).
//!
//! Official Magic can gain control of a target enemy minion this turn and
//! untap it, or gain control until that minion loses Stealth after tapping it
//! and granting Stealth. Neither transfer is Nearby-restricted. This-turn
//! control reverts at End Phase; stealth-bound control survives End Phase and
//! reverts when Stealth is lost. Genesis can also steal every tapped minion
//! sharing the newcomer's footprint until that source leaves play.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode, Seat};
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

fn dummy() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn far() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn betrayal() -> Value {
    json!({
        "cardType": "magic",
        "gainControlOfTargetEnemyMinionThisTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "temporary-control" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-temporary-control-v1",
        },
        "cards": {
            "north-ally": dummy(),
            "north-avatar": avatar(),
            "north-betrayal": betrayal(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": far(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-betrayal",
                    "north-betrayal",
                    "north-betrayal",
                    "north-betrayal"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-far"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn opening_spell_ids(session: &Session, seat: &str) -> Vec<String> {
    state(session)["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("spellbook hand")
        .iter()
        .filter_map(|card| card["cardId"].as_str().map(ToOwned::to_owned))
        .collect()
}

fn accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> (Value, Receipt) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .find(|action| predicate(&action.descriptor))
        .unwrap_or_else(|| {
            panic!(
                "expected engine-issued action among {:?}",
                session
                    .legal_actions()
                    .expect("legal actions")
                    .iter()
                    .map(|action| action.descriptor.clone())
                    .collect::<Vec<_>>()
            )
        });
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

fn unit<'a>(after: &'a Value, instance_id: &str) -> &'a Value {
    after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("unit")
}

fn betrayal_targets(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("Betrayal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-betrayal"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn offers_activate_mana(session: &Session, instance_id: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "activate-mana"
                && action.descriptor["unitInstanceId"] == instance_id
        })
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

fn end_then_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

/// North dummy at C4, tapped South minion at C1, North ready to cast.
fn betrayal_opening() -> (Session, String, String) {
    let mut session = (1..=4096)
        .map(manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("temporary-control candidate");
            let north = opening_spell_ids(&session, "north");
            let south = opening_spell_ids(&session, "south");
            (north.iter().any(|card| card == "north-ally")
                && north.iter().any(|card| card == "north-betrayal")
                && south.iter().any(|card| card == "south-far"))
            .then_some(session)
        })
        .expect("bounded seed opening with Betrayal, an ally, and a far enemy");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (ally, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    });
    let ally_id = ally["cardInstanceId"]
        .as_str()
        .expect("north ally identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
    });
    let far_id = far["cardInstanceId"]
        .as_str()
        .expect("south far identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "atlas");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == far_id.as_str()
    });
    end_then_draw(&mut session, "spellbook");
    (session, ally_id, far_id)
}

#[test]
fn rule_catalog_0513_temporary_control_takes_a_distant_tapped_enemy_minion_this_turn() {
    let (mut session, ally_id, far_id) = betrayal_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &far_id)["controller"], "south");
    assert_eq!(unit(&before, &far_id)["owner"], "south");
    assert_eq!(unit(&before, &far_id)["tapped"], true);
    assert_eq!(unit(&before, &ally_id)["controller"], "north");

    let targets = betrayal_targets(&session);
    assert!(targets.contains(&far_id), "{targets:?}");
    assert!(!targets.contains(&ally_id), "{targets:?}");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-betrayal"
            && descriptor["target"]["instanceId"] == far_id.as_str()
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-control-changed",
            "minion-untapped",
            "magic-resolved"
        ]
    );
    let changed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-control-changed")
        .expect("control change");
    assert_eq!(changed.payload["fromSeat"], "south");
    assert_eq!(changed.payload["seat"], "north");

    let after = state(&session);
    assert_eq!(unit(&after, &far_id)["controller"], "north");
    assert_eq!(unit(&after, &far_id)["owner"], "south");
    assert_eq!(unit(&after, &far_id)["tapped"], false);
    assert!(offers_activate_mana(&session, &far_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0514_temporary_control_reverts_at_end_of_turn() {
    let (mut session, _, far_id) = betrayal_opening();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-betrayal"
            && descriptor["target"]["instanceId"] == far_id.as_str()
    });
    assert_eq!(unit(&state(&session), &far_id)["controller"], "north");

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == far_id
    }));
    let after = state(&session);
    assert_eq!(unit(&after, &far_id)["controller"], "south");
    assert_eq!(unit(&after, &far_id)["owner"], "south");
    assert_exact_replay(&session);
}

fn infiltrate() -> Value {
    json!({
        "cardType": "magic",
        "gainControlOfTargetEnemyMinionUntilStealthLost": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn infiltrate_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "stealth-bound-control" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-stealth-bound-control-v1",
        },
        "cards": {
            "north-ally": dummy(),
            "north-avatar": avatar(),
            "north-infiltrate": infiltrate(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": far(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-infiltrate",
                    "north-infiltrate",
                    "north-infiltrate",
                    "north-infiltrate"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-far"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn infiltrate_targets(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("Infiltrate actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-infiltrate"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

/// North dummy at C4 and an untapped South minion at C1, North ready to cast.
fn infiltrate_opening() -> (Session, String, String) {
    let mut session = (1..=4096)
        .map(infiltrate_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("stealth-bound-control candidate");
            let north = opening_spell_ids(&session, "north");
            let south = opening_spell_ids(&session, "south");
            (north.iter().any(|card| card == "north-ally")
                && north.iter().any(|card| card == "north-infiltrate")
                && south.iter().any(|card| card == "south-far"))
            .then_some(session)
        })
        .expect("bounded seed opening with Infiltrate, an ally, and a far enemy");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (ally, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    });
    let ally_id = ally["cardInstanceId"]
        .as_str()
        .expect("north ally identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
    });
    let far_id = far["cardInstanceId"]
        .as_str()
        .expect("south far identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    (session, ally_id, far_id)
}

fn steal_until_stealth_lost(session: &mut Session, far_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-infiltrate"
            && descriptor["target"]["instanceId"] == far_id
    });
    receipt
}

#[test]
fn rule_catalog_0515_stealth_bound_control_steals_taps_and_hides_a_distant_enemy() {
    let (mut session, ally_id, far_id) = infiltrate_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &far_id)["controller"], "south");
    assert_eq!(unit(&before, &far_id)["tapped"], false);
    assert_eq!(unit(&before, &far_id)["stealthed"], false);

    let targets = infiltrate_targets(&session);
    assert!(targets.contains(&far_id), "{targets:?}");
    assert!(!targets.contains(&ally_id), "{targets:?}");

    let receipt = steal_until_stealth_lost(&mut session, &far_id);
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-control-changed",
            "minion-stealthed",
            "minion-tapped",
            "magic-resolved"
        ]
    );
    let after = state(&session);
    assert_eq!(unit(&after, &far_id)["controller"], "north");
    assert_eq!(unit(&after, &far_id)["owner"], "south");
    assert_eq!(unit(&after, &far_id)["tapped"], true);
    assert_eq!(unit(&after, &far_id)["stealthed"], true);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0516_stealth_bound_control_survives_end_of_turn_and_reverts_when_stealth_is_lost() {
    let (mut session, _, far_id) = infiltrate_opening();
    steal_until_stealth_lost(&mut session, &far_id);
    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !ended
            .events
            .iter()
            .any(|event| event.event_type == "minion-control-changed")
    );
    assert_eq!(unit(&state(&session), &far_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &far_id)["stealthed"], true);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    end_then_draw(&mut session, "spellbook");
    assert_eq!(unit(&state(&session), &far_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &far_id)["tapped"], false);

    let (_, interacted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == far_id.as_str()
    });
    assert!(interacted.events.iter().any(|event| {
        event.event_type == "stealth-lost" && event.payload["instanceId"] == far_id
    }));
    assert!(interacted.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == far_id
    }));
    let after = state(&session);
    assert_eq!(unit(&after, &far_id)["controller"], "south");
    assert_eq!(unit(&after, &far_id)["owner"], "south");
    assert_eq!(unit(&after, &far_id)["stealthed"], false);
    assert_exact_replay(&session);
}

fn puppet() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "genesisGainControlOfTappedMinionsHereUntilThisLeaves": true,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bounce() -> Value {
    json!({
        "cardType": "magic",
        "manaCost": 0,
        "returnTargetMinionToOwnerHand": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn near() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        "ward": true,
    })
}

fn ready() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn puppet_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "source-bound-control" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-source-bound-control-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bounce": bounce(),
            "north-puppet": puppet(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "summonToAnySite": true,
                "tapForMana": 1,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-near": near(),
            "south-ready": ready(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-puppet",
                    "north-puppet",
                    "north-puppet",
                    "north-puppet",
                    "north-bounce",
                    "north-bounce",
                    "north-bounce",
                    "north-bounce"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-near",
                    "south-near",
                    "south-near",
                    "south-ready",
                    "south-ready",
                    "south-ready",
                    "south-far",
                    "south-far"
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

/// Tapped warded South minion and untapped South minion at C1, tapped South
/// minion at C2, North ready to summon the Genesis source onto C1.
fn puppet_opening() -> (Session, String, String, String) {
    let mut session = (1..=4096)
        .map(puppet_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("source-bound-control candidate");
            let north = opening_spell_ids(&session, "north");
            let south = opening_spell_ids(&session, "south");
            (north.iter().any(|card| card == "north-puppet")
                && north.iter().any(|card| card == "north-bounce")
                && south.iter().any(|card| card == "south-near")
                && south.iter().any(|card| card == "south-ready")
                && south.iter().any(|card| card == "south-far"))
            .then_some(session)
        })
        .expect("bounded seed opening with Puppet, bounce, and three South minions");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (near_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-near"
            && descriptor["cell"] == "C1"
    });
    let near_id = near_summon["cardInstanceId"]
        .as_str()
        .expect("south near identity")
        .to_owned();
    let (ready_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-ready"
            && descriptor["cell"] == "C1"
    });
    let ready_id = ready_summon["cardInstanceId"]
        .as_str()
        .expect("south ready identity")
        .to_owned();
    let (far_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C4"
    });
    let far_id = far_summon["cardInstanceId"]
        .as_str()
        .expect("south far identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "atlas");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == near_id.as_str()
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == far_id.as_str()
    });
    end_then_draw(&mut session, "spellbook");
    (session, near_id, ready_id, far_id)
}

fn steal_tapped_here(session: &mut Session) -> (String, Receipt) {
    let (summoned, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-puppet"
            && descriptor["cell"] == "C1"
    });
    let puppet_id = summoned["cardInstanceId"]
        .as_str()
        .expect("puppet identity")
        .to_owned();
    (puppet_id, receipt)
}

#[test]
fn rule_catalog_0517_genesis_control_steals_tapped_minions_here_and_skips_the_rest() {
    let (mut session, near_id, ready_id, far_id) = puppet_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &near_id)["controller"], "south");
    assert_eq!(unit(&before, &near_id)["owner"], "south");
    assert_eq!(unit(&before, &near_id)["tapped"], true);
    assert_eq!(unit(&before, &near_id)["warded"], true);
    assert_eq!(unit(&before, &ready_id)["controller"], "south");
    assert_eq!(unit(&before, &ready_id)["tapped"], false);
    assert_eq!(unit(&before, &far_id)["controller"], "south");
    assert_eq!(unit(&before, &far_id)["tapped"], true);

    let (puppet_id, receipt) = steal_tapped_here(&mut session);
    assert!(event_types(&receipt).contains(&"minion-summoned"));
    let changed: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "minion-control-changed")
        .collect();
    assert_eq!(changed.len(), 1, "{:?}", event_types(&receipt));
    assert_eq!(changed[0].payload["fromSeat"], "south");
    assert_eq!(changed[0].payload["seat"], "north");
    assert_eq!(changed[0].payload["instanceId"], near_id);
    assert_eq!(changed[0].payload["sourceInstanceId"], puppet_id);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken")
    );

    let after = state(&session);
    assert_eq!(unit(&after, &near_id)["controller"], "north");
    assert_eq!(unit(&after, &near_id)["owner"], "south");
    assert_eq!(unit(&after, &near_id)["tapped"], true);
    assert_eq!(unit(&after, &near_id)["warded"], true);
    assert_eq!(unit(&after, &ready_id)["controller"], "south");
    assert_eq!(unit(&after, &ready_id)["tapped"], false);
    assert_eq!(unit(&after, &far_id)["controller"], "south");
    assert_eq!(unit(&after, &far_id)["tapped"], true);
    assert_eq!(unit(&after, &puppet_id)["controller"], "north");

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !ended
            .events
            .iter()
            .any(|event| event.event_type == "minion-control-changed")
    );
    assert_eq!(unit(&state(&session), &near_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &near_id)["owner"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0518_genesis_control_reverts_when_the_source_leaves() {
    let (mut session, near_id, _, _) = puppet_opening();
    let (puppet_id, _) = steal_tapped_here(&mut session);
    assert_eq!(unit(&state(&session), &near_id)["controller"], "north");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bounce"
            && descriptor["target"]["instanceId"] == puppet_id.as_str()
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-returned-to-hand",
            "minion-control-changed",
            "magic-resolved"
        ]
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == near_id
    }));
    let after = state(&session);
    assert_eq!(unit(&after, &near_id)["controller"], "south");
    assert_eq!(unit(&after, &near_id)["owner"], "south");
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != puppet_id)
    );
    assert_exact_replay(&session);
}

fn potion() -> Value {
    json!({
        "cardType": "artifact",
        "manaCost": 0,
        "sacrificeThisToGainControlOfTargetEnemyMinionHereUntilBearerLeaves": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn potion_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sacrifice-control-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sacrifice-control-artifact-v1",
        },
        "cards": {
            "north-ally": dummy(),
            "north-avatar": avatar(),
            "north-bounce": bounce(),
            "north-potion": potion(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-far": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-near": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-ally",
                    "north-potion",
                    "north-potion",
                    "north-potion",
                    "north-bounce",
                    "north-bounce",
                    "north-bounce"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-near",
                    "south-near",
                    "south-near",
                    "south-near",
                    "south-far",
                    "south-far",
                    "south-far",
                    "south-far"
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn potion_targets(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("sacrifice-control actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "activate-artifact-sacrifice-control")
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

/// North ally carrying the Artifact at C4, South enemy here at C4 and distant at C1.
fn potion_opening() -> (Session, String, String, String, String) {
    let mut session = (1..=4096)
        .map(potion_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("sacrifice-control-artifact candidate");
            let north = opening_spell_ids(&session, "north");
            let south = opening_spell_ids(&session, "south");
            (north.iter().any(|card| card == "north-ally")
                && north.iter().any(|card| card == "north-potion")
                && north.iter().any(|card| card == "north-bounce")
                && south.iter().any(|card| card == "south-near")
                && south.iter().any(|card| card == "south-far"))
            .then_some(session)
        })
        .expect("bounded seed opening with potion, bounce, ally, and two South minions");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (ally, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    });
    let ally_id = ally["cardInstanceId"]
        .as_str()
        .expect("north ally identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (far, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-far"
            && descriptor["cell"] == "C1"
    });
    let far_id = far["cardInstanceId"]
        .as_str()
        .expect("south far identity")
        .to_owned();
    let (near, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-near"
            && descriptor["cell"] == "C4"
    });
    let near_id = near["cardInstanceId"]
        .as_str()
        .expect("south near identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    let (cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-potion"
            && descriptor["bearer"]["instanceId"] == ally_id.as_str()
    });
    let artifact_id = cast["cardInstanceId"]
        .as_str()
        .expect("potion identity")
        .to_owned();
    (session, ally_id, near_id, far_id, artifact_id)
}

fn sacrifice_steal(session: &mut Session, artifact_id: &str, near_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-artifact-sacrifice-control"
            && descriptor["artifactInstanceId"] == artifact_id
            && descriptor["target"]["instanceId"] == near_id
    });
    receipt
}

#[test]
fn rule_catalog_0519_sacrifice_artifact_steals_an_enemy_minion_here_and_skips_the_rest() {
    let (mut session, ally_id, near_id, far_id, artifact_id) = potion_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &near_id)["controller"], "south");
    assert_eq!(unit(&before, &near_id)["owner"], "south");
    assert_eq!(unit(&before, &far_id)["controller"], "south");
    assert_eq!(unit(&before, &ally_id)["controller"], "north");

    let targets = potion_targets(&session);
    assert!(targets.contains(&near_id), "{targets:?}");
    assert!(!targets.contains(&far_id), "{targets:?}");
    assert!(!targets.contains(&ally_id), "{targets:?}");

    let receipt = sacrifice_steal(&mut session, &artifact_id, &near_id);
    assert_eq!(
        event_types(&receipt),
        ["artifact-sacrificed", "minion-control-changed"]
    );
    let changed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-control-changed")
        .expect("control change");
    assert_eq!(changed.payload["fromSeat"], "south");
    assert_eq!(changed.payload["seat"], "north");
    assert_eq!(changed.payload["instanceId"], near_id);
    assert_eq!(changed.payload["sourceInstanceId"], ally_id);
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "artifact-sacrificed"
            && event.payload["instanceId"] == artifact_id
            && event.payload["sourceInstanceId"] == ally_id
    }));

    let after = state(&session);
    assert_eq!(unit(&after, &near_id)["controller"], "north");
    assert_eq!(unit(&after, &near_id)["owner"], "south");
    assert_eq!(unit(&after, &far_id)["controller"], "south");
    assert_eq!(unit(&after, &ally_id)["controller"], "north");
    assert!(
        after["realm"]["artifacts"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|artifact| artifact["instanceId"] != artifact_id)
    );
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == artifact_id && card["cardId"] == "north-potion")
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0520_sacrifice_artifact_control_persists_until_the_bearer_leaves() {
    let (mut session, ally_id, near_id, _, artifact_id) = potion_opening();
    sacrifice_steal(&mut session, &artifact_id, &near_id);
    assert_eq!(unit(&state(&session), &near_id)["controller"], "north");

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !ended
            .events
            .iter()
            .any(|event| event.event_type == "minion-control-changed")
    );
    assert_eq!(unit(&state(&session), &near_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &near_id)["owner"], "south");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    end_then_draw(&mut session, "spellbook");
    assert_eq!(unit(&state(&session), &near_id)["controller"], "north");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bounce"
            && descriptor["target"]["instanceId"] == ally_id.as_str()
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "minion-returned-to-hand",
            "minion-control-changed",
            "magic-resolved"
        ]
    );
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == near_id
    }));
    let after = state(&session);
    assert_eq!(unit(&after, &near_id)["controller"], "south");
    assert_eq!(unit(&after, &near_id)["owner"], "south");
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .all(|unit| unit["instanceId"] != ally_id)
    );
    assert_exact_replay(&session);
}

fn sellsword() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "nearbyAvatarsMayDiscardCardToGainControlOfThis": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn sellsword_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "nearby-avatar-discard-control" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-nearby-avatar-discard-control-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dummy": dummy(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-sellsword": sellsword(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-dummy"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-sellsword"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

/// South Sellsword at North C3, North Avatar nearby at C4, South Avatar distant at C1.
fn sellsword_opening() -> (Session, String) {
    let mut session = (1..=4096)
        .map(sellsword_manifest)
        .find_map(|candidate| {
            let session =
                Session::new(&candidate).expect("nearby-avatar-discard-control candidate");
            let north = opening_spell_ids(&session, "north");
            let south = opening_spell_ids(&session, "south");
            (north.iter().any(|card| card == "north-dummy")
                && south.iter().any(|card| card == "south-sellsword"))
            .then_some(session)
        })
        .expect("bounded seed opening with a discard and Seasoned Sellsword");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    end_then_draw(&mut session, "spellbook");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-sellsword"
            && descriptor["cell"] == "C3"
    });
    let sellsword_id = summoned["cardInstanceId"]
        .as_str()
        .expect("sellsword identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    (session, sellsword_id)
}

fn steal_sellsword(session: &mut Session, sellsword_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "activate-discard-to-gain-control"
            && descriptor["minionInstanceId"] == sellsword_id
    });
    receipt
}

fn sellsword_steal_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("discard-to-gain-control actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "activate-discard-to-gain-control")
        .filter_map(|action| {
            action.descriptor["minionInstanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect()
}

#[test]
fn rule_catalog_0521_nearby_avatar_discards_to_steal_this_minion() {
    let (mut session, sellsword_id) = sellsword_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &sellsword_id)["controller"], "south");
    assert_eq!(unit(&before, &sellsword_id)["owner"], "south");
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar")
        .to_owned();

    let targets = sellsword_steal_ids(&session);
    assert!(targets.contains(&sellsword_id), "{targets:?}");

    let receipt = steal_sellsword(&mut session, &sellsword_id);
    assert_eq!(
        event_types(&receipt),
        ["card-discarded", "minion-control-changed"]
    );
    let changed = receipt
        .events
        .iter()
        .find(|event| event.event_type == "minion-control-changed")
        .expect("control change");
    assert_eq!(changed.payload["fromSeat"], "south");
    assert_eq!(changed.payload["seat"], "north");
    assert_eq!(changed.payload["instanceId"], sellsword_id);
    assert_eq!(changed.payload["sourceInstanceId"], avatar_id);
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.event_type == "ward-broken")
    );

    let after = state(&session);
    assert_eq!(unit(&after, &sellsword_id)["controller"], "north");
    assert_eq!(unit(&after, &sellsword_id)["owner"], "south");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0522_nearby_avatar_control_is_permanent_and_distant_avatars_cannot_steal() {
    let (mut session, sellsword_id) = sellsword_opening();
    steal_sellsword(&mut session, &sellsword_id);
    assert_eq!(unit(&state(&session), &sellsword_id)["controller"], "north");

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !ended
            .events
            .iter()
            .any(|event| event.event_type == "minion-control-changed")
    );
    assert_eq!(unit(&state(&session), &sellsword_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &sellsword_id)["owner"], "south");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    assert!(
        sellsword_steal_ids(&session).is_empty(),
        "distant South Avatar at C1 is not nearby C3"
    );

    end_then_draw(&mut session, "spellbook");
    assert_eq!(unit(&state(&session), &sellsword_id)["controller"], "north");
    assert_eq!(unit(&state(&session), &sellsword_id)["owner"], "south");
    assert_exact_replay(&session);
}

fn thais() -> Value {
    json!({
        "attack": 0,
        "cardType": "minion",
        "defense": 0,
        "genesisEachPlayerControlledByPreviousPlayerNextTurn": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn thais_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "previous-player-control" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-previous-player-control-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-thais": thais(),
            "south-avatar": avatar(),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-thais"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-dummy"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

/// North has summoned the Genesis source at C4 and still holds priority.
fn thais_opening() -> (Session, String) {
    let mut session = (1..=4096)
        .map(thais_manifest)
        .find_map(|candidate| {
            let session = Session::new(&candidate).expect("previous-player-control candidate");
            opening_spell_ids(&session, "north")
                .iter()
                .any(|card| card == "north-thais")
                .then_some(session)
        })
        .expect("bounded seed opening with previous-player-control Genesis");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-thais"
            && descriptor["cell"] == "C4"
    });
    let thais_id = summoned["cardInstanceId"]
        .as_str()
        .expect("thais identity")
        .to_owned();
    (session, thais_id)
}

fn reject_wrong_seat(session: &mut Session, seat: Seat) {
    let action = session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .next()
        .expect("issued action");
    let StepResult::Rejected(rejection) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat,
            state_version: action.state_version,
        })
        .expect("wrong-seat step")
    else {
        panic!("the controlled player must not submit the current decision");
    };
    assert_eq!(rejection.code, RejectionCode::WrongSeat);
}

fn assert_acting_seat(session: &Session, seat: Seat) {
    let actions = session.legal_actions().expect("legal actions");
    assert!(!actions.is_empty());
    assert!(
        actions.iter().all(|action| action.seat == seat),
        "expected every issued action for {seat:?}, got {:?}",
        actions.iter().map(|action| action.seat).collect::<Vec<_>>()
    );
}

#[test]
fn rule_catalog_0523_genesis_schedules_previous_player_control_of_the_next_turns() {
    let (mut session, thais_id) = thais_opening();
    let scheduled = state(&session);
    assert_eq!(
        scheduled["pendingPlayerControllers"]["south"]["controller"],
        "north"
    );
    assert_eq!(
        scheduled["pendingPlayerControllers"]["south"]["sourceInstanceId"],
        thais_id
    );
    assert_eq!(
        scheduled["pendingPlayerControllers"]["north"]["controller"],
        "south"
    );
    assert_eq!(scheduled["turnController"], Value::Null);
    assert_eq!(unit(&scheduled, &thais_id)["controller"], "north");

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let controlled = ended
        .events
        .iter()
        .find(|event| event.event_type == "player-controlled")
        .expect("player control starts on the next turn");
    assert_eq!(controlled.payload["seat"], "south");
    assert_eq!(controlled.payload["controller"], "north");
    assert_eq!(controlled.payload["sourceInstanceId"], thais_id);

    let hijacked = state(&session);
    assert_eq!(hijacked["activeSeat"], "south");
    assert_eq!(hijacked["decisionSeat"], "south");
    assert_eq!(hijacked["turnController"], "north");
    assert_eq!(hijacked["pendingPlayerControllers"]["south"], Value::Null);
    assert_eq!(
        hijacked["pendingPlayerControllers"]["north"]["controller"],
        "south"
    );

    assert_acting_seat(&session, Seat::North);
    reject_wrong_seat(&mut session, Seat::South);

    let north_view = session.public_view(Seat::North).expect("north public view");
    assert!(north_view["players"]["north"]["hand"]["atlas"].is_array());
    assert!(north_view["players"]["south"]["hand"]["atlas"].is_array());
    let south_view = session.public_view(Seat::South).expect("south public view");
    assert!(south_view["players"]["south"]["hand"]["atlas"].is_array());
    assert!(south_view["players"]["north"]["hand"]["atlas"].is_number());

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    assert_eq!(
        state(&session)["realm"]["sites"]["C1"]["controller"],
        "south"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0524_previous_player_control_covers_each_next_turn_then_expires() {
    let (mut session, thais_id) = thais_opening();
    let (_, south_started) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(south_started.events.iter().any(|event| {
        event.event_type == "player-controlled"
            && event.payload["seat"] == "south"
            && event.payload["controller"] == "north"
            && event.payload["sourceInstanceId"] == thais_id
    }));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (_, north_started) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    let north_control = north_started
        .events
        .iter()
        .find(|event| event.event_type == "player-controlled")
        .expect("North's next turn is controlled by South");
    assert_eq!(north_control.payload["seat"], "north");
    assert_eq!(north_control.payload["controller"], "south");
    assert_eq!(north_control.payload["sourceInstanceId"], thais_id);

    let north_hijacked = state(&session);
    assert_eq!(north_hijacked["activeSeat"], "north");
    assert_eq!(north_hijacked["turnController"], "south");
    assert_eq!(
        north_hijacked["pendingPlayerControllers"]["north"],
        Value::Null
    );
    assert_eq!(
        north_hijacked["pendingPlayerControllers"]["south"],
        Value::Null
    );
    assert_acting_seat(&session, Seat::South);
    reject_wrong_seat(&mut session, Seat::North);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let (_, expired) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !expired
            .events
            .iter()
            .any(|event| event.event_type == "player-controlled")
    );

    let after = state(&session);
    assert_eq!(after["activeSeat"], "south");
    assert!(after.get("turnController").is_none());
    assert!(after.get("pendingPlayerControllers").is_none());
    assert_acting_seat(&session, Seat::South);
    reject_wrong_seat(&mut session, Seat::North);
    assert_exact_replay(&session);
}

fn deathrite() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "tapForMana": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn puppet_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "source-bound-control-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-source-bound-control-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-lash": lash(),
            "north-puppet": puppet(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-puppet",
                    "north-lash",
                    "north-puppet",
                    "north-lash",
                    "north-puppet",
                    "north-lash",
                    "north-puppet",
                    "north-lash",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn north_has_puppet_and_lash(snapshot: &Value) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    ["north-puppet", "north-lash"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn puppet_deathrite_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(puppet_deathrite_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_puppet_and_lash(&state(&preview)))
        })
        .expect("bounded seed with Puppet and Lash in the opening hand")
}

/// Tapped South Deathrite at distant C1, North ready to summon the Genesis source there.
fn puppet_deathrite_opening() -> (Session, String) {
    let encoded = puppet_deathrite_seed_with(951);
    let mut session = Session::new(&encoded).expect("valid source-bound Deathrite control session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
    });
    let deathrite_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "atlas");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana"
            && descriptor["unitInstanceId"] == deathrite_id.as_str()
    });
    end_then_draw(&mut session, "spellbook");
    (session, deathrite_id)
}

#[test]
fn rule_catalog_0951_source_bound_control_transfers_distant_deathrite_to_thief_before_source_leaves()
 {
    let (mut session, deathrite_id) = puppet_deathrite_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &deathrite_id)["controller"], "south");
    assert_eq!(unit(&before, &deathrite_id)["owner"], "south");
    assert_eq!(unit(&before, &deathrite_id)["tapped"], true);

    let (puppet_id, stolen) = steal_tapped_here(&mut session);
    assert!(stolen.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "south"
            && event.payload["seat"] == "north"
            && event.payload["instanceId"] == deathrite_id
            && event.payload["sourceInstanceId"] == puppet_id
    }));
    let stolen_state = state(&session);
    let transferred = realm_unit(&stolen_state, &deathrite_id).expect("stolen minion");
    assert_eq!(transferred["controller"], "north");
    assert_eq!(transferred["owner"], "south");
    assert_eq!(transferred["tapped"], true);
    assert!(realm_unit(&stolen_state, &puppet_id).is_some());

    let before_kill = state(&session);
    let north_atlas = atlas_len(&before_kill, "north");
    let south_atlas = atlas_len(&before_kill, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == deathrite_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], deathrite_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &deathrite_id).is_none());
    assert!(realm_unit(&finished, &puppet_id).is_some());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &deathrite_id));
    assert!(!cemetery_has(&finished, "north", &deathrite_id));
    assert_exact_replay(&session);
}

fn puppet_deathrite_bounce_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "source-bound-control-deathrite-revert" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-source-bound-control-deathrite-revert-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bounce": bounce(),
            "north-lash": lash(),
            "north-puppet": puppet(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-puppet",
                    "north-bounce",
                    "north-lash",
                    "north-puppet",
                    "north-bounce",
                    "north-lash",
                    "north-puppet",
                    "north-lash",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_puppet_lash_and_bounce(snapshot: &Value) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    ["north-puppet", "north-bounce", "north-lash"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn puppet_deathrite_bounce_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(puppet_deathrite_bounce_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_puppet_lash_and_bounce(&state(&preview)))
        })
        .expect("bounded seed with Puppet, Bounce, and Lash in the opening hand")
}

/// Tapped South Deathrite at distant C1, North ready to steal then bounce the Genesis source.
fn puppet_deathrite_bounce_opening() -> (Session, String) {
    let encoded = puppet_deathrite_bounce_seed_with(971);
    let mut session = Session::new(&encoded).expect("valid source-bound Deathrite revert session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
    });
    let deathrite_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "atlas");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana"
            && descriptor["unitInstanceId"] == deathrite_id.as_str()
    });
    end_then_draw(&mut session, "spellbook");
    (session, deathrite_id)
}

#[test]
fn rule_catalog_0971_source_bound_deathrite_draws_for_original_controller_when_thief_kills_after_source_leaves()
 {
    let (mut session, deathrite_id) = puppet_deathrite_bounce_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &deathrite_id)["controller"], "south");
    assert_eq!(unit(&before, &deathrite_id)["owner"], "south");
    assert_eq!(unit(&before, &deathrite_id)["tapped"], true);

    let (puppet_id, stolen) = steal_tapped_here(&mut session);
    assert!(stolen.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "south"
            && event.payload["seat"] == "north"
            && event.payload["instanceId"] == deathrite_id
            && event.payload["sourceInstanceId"] == puppet_id
    }));
    assert_eq!(unit(&state(&session), &deathrite_id)["controller"], "north");

    let (_, reverted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bounce"
            && descriptor["target"]["instanceId"] == puppet_id.as_str()
    });
    assert_eq!(
        event_types(&reverted),
        [
            "magic-cast",
            "minion-returned-to-hand",
            "minion-control-changed",
            "magic-resolved"
        ]
    );
    assert!(reverted.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == deathrite_id
    }));
    let reverted_state = state(&session);
    assert!(realm_unit(&reverted_state, &puppet_id).is_none());
    let restored = realm_unit(&reverted_state, &deathrite_id).expect("Deathrite still on board");
    assert_eq!(restored["controller"], "south");
    assert_eq!(restored["owner"], "south");

    let before_kill = state(&session);
    let north_atlas = atlas_len(&before_kill, "north");
    let south_atlas = atlas_len(&before_kill, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == deathrite_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], deathrite_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &deathrite_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert!(cemetery_has(&finished, "south", &deathrite_id));
    assert!(!cemetery_has(&finished, "north", &deathrite_id));
    assert_exact_replay(&session);
}

fn potion_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sacrifice-control-artifact-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sacrifice-control-artifact-deathrite-v1",
        },
        "cards": {
            "north-ally": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-avatar": avatar(),
            "north-lash": lash(),
            "north-potion": potion(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-potion",
                    "north-lash",
                    "north-potion",
                    "north-lash",
                    "north-potion",
                    "north-lash",
                    "north-potion",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_ally_potion_and_lash(snapshot: &Value) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    ["north-ally", "north-potion", "north-lash"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn potion_deathrite_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(potion_deathrite_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_ally_potion_and_lash(&state(&preview)))
        })
        .expect("bounded seed with Ally, Potion, and Lash in the opening hand")
}

/// Tapped South Deathrite at distant C1, North ally there carrying the Artifact.
fn potion_deathrite_opening() -> (Session, String, String, String) {
    let encoded = potion_deathrite_seed_with(958);
    let mut session =
        Session::new(&encoded).expect("valid sacrifice-control Deathrite control session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
    });
    let deathrite_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "atlas");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana"
            && descriptor["unitInstanceId"] == deathrite_id.as_str()
    });
    end_then_draw(&mut session, "spellbook");
    let (ally, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C1"
    });
    let ally_id = ally["cardInstanceId"]
        .as_str()
        .expect("north ally identity")
        .to_owned();
    let (cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-potion"
            && descriptor["bearer"]["instanceId"] == ally_id.as_str()
    });
    let artifact_id = cast["cardInstanceId"]
        .as_str()
        .expect("potion identity")
        .to_owned();
    (session, ally_id, deathrite_id, artifact_id)
}

#[test]
fn rule_catalog_0958_sacrifice_artifact_control_transfers_distant_deathrite_to_thief_before_bearer_leaves()
 {
    let (mut session, ally_id, deathrite_id, artifact_id) = potion_deathrite_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &deathrite_id)["controller"], "south");
    assert_eq!(unit(&before, &deathrite_id)["owner"], "south");
    assert_eq!(unit(&before, &deathrite_id)["tapped"], true);
    assert_eq!(unit(&before, &ally_id)["controller"], "north");

    let targets = potion_targets(&session);
    assert!(targets.contains(&deathrite_id), "{targets:?}");
    assert!(!targets.contains(&ally_id), "{targets:?}");

    let stolen = sacrifice_steal(&mut session, &artifact_id, &deathrite_id);
    assert_eq!(
        event_types(&stolen),
        ["artifact-sacrificed", "minion-control-changed"]
    );
    assert!(stolen.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "south"
            && event.payload["seat"] == "north"
            && event.payload["instanceId"] == deathrite_id
            && event.payload["sourceInstanceId"] == ally_id
    }));
    let stolen_state = state(&session);
    let transferred = realm_unit(&stolen_state, &deathrite_id).expect("stolen minion");
    assert_eq!(transferred["controller"], "north");
    assert_eq!(transferred["owner"], "south");
    assert_eq!(transferred["tapped"], true);
    assert!(realm_unit(&stolen_state, &ally_id).is_some());
    assert!(
        stolen_state["realm"]["artifacts"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|artifact| artifact["instanceId"] != artifact_id)
    );

    let before_kill = state(&session);
    let north_atlas = atlas_len(&before_kill, "north");
    let south_atlas = atlas_len(&before_kill, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == deathrite_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], deathrite_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &deathrite_id).is_none());
    assert!(realm_unit(&finished, &ally_id).is_some());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &deathrite_id));
    assert!(!cemetery_has(&finished, "north", &deathrite_id));
    assert_exact_replay(&session);
}

fn discard_deathrite() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "nearbyAvatarsMayDiscardCardToGainControlOfThis": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn discard_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "nearby-avatar-discard-control-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-nearby-avatar-discard-control-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-dummy": dummy(),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": discard_deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-dummy",
                    "north-dummy",
                    "north-lash",
                    "north-lash",
                    "north-dummy",
                    "north-lash",
                    "north-dummy",
                    "north-lash",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_lash_and_discardable(snapshot: &Value) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    ["north-dummy", "north-lash"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn discard_deathrite_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(discard_deathrite_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_lash_and_discardable(&state(&preview)))
        })
        .expect("bounded seed with Lash and a discard card in the opening hand")
}

/// Tapped South Deathrite at North C3, North Avatar nearby at C4, South Avatar distant at C1.
fn discard_deathrite_opening() -> (Session, String) {
    let encoded = discard_deathrite_seed_with(959);
    let mut session =
        Session::new(&encoded).expect("valid nearby-avatar-discard Deathrite control session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    end_then_draw(&mut session, "spellbook");
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C3"
    });
    let deathrite_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    (session, deathrite_id)
}

#[test]
fn rule_catalog_0959_nearby_avatar_discard_control_transfers_distant_deathrite_to_thief_before_permanent_control()
 {
    let (mut session, deathrite_id) = discard_deathrite_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &deathrite_id)["controller"], "south");
    assert_eq!(unit(&before, &deathrite_id)["owner"], "south");
    assert_eq!(unit(&before, &deathrite_id)["tapped"], false);
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("north avatar")
        .to_owned();

    let targets = sellsword_steal_ids(&session);
    assert!(targets.contains(&deathrite_id), "{targets:?}");

    let stolen = steal_sellsword(&mut session, &deathrite_id);
    assert_eq!(
        event_types(&stolen),
        ["card-discarded", "minion-control-changed"]
    );
    assert!(stolen.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "south"
            && event.payload["seat"] == "north"
            && event.payload["instanceId"] == deathrite_id
            && event.payload["sourceInstanceId"] == avatar_id
    }));
    let stolen_state = state(&session);
    let transferred = realm_unit(&stolen_state, &deathrite_id).expect("stolen minion");
    assert_eq!(transferred["controller"], "north");
    assert_eq!(transferred["owner"], "south");
    assert_eq!(transferred["tapped"], false);

    let before_kill = state(&session);
    let north_atlas = atlas_len(&before_kill, "north");
    let south_atlas = atlas_len(&before_kill, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == deathrite_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], deathrite_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &deathrite_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &deathrite_id));
    assert!(!cemetery_has(&finished, "north", &deathrite_id));
    assert_exact_replay(&session);
}

fn potion_deathrite_bounce_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sacrifice-control-artifact-deathrite-revert" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sacrifice-control-artifact-deathrite-revert-v1",
        },
        "cards": {
            "north-ally": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "north-avatar": avatar(),
            "north-bounce": bounce(),
            "north-lash": lash(),
            "north-potion": potion(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-potion",
                    "north-bounce",
                    "north-lash",
                    "north-potion",
                    "north-bounce",
                    "north-lash",
                    "north-potion",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 8],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_ally_potion_and_bounce(snapshot: &Value) -> bool {
    let hand = snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook");
    ["north-ally", "north-potion", "north-bounce"]
        .into_iter()
        .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
}

fn potion_deathrite_bounce_seed_with(start: u32) -> String {
    (start..start + 256)
        .map(potion_deathrite_bounce_manifest)
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_ally_potion_and_bounce(&state(&preview)))
        })
        .expect("bounded seed with Ally, Potion, and Bounce in the opening hand")
}

/// Tapped South Deathrite at distant C1, North ally there carrying the Artifact.
fn potion_deathrite_bounce_opening() -> (Session, String, String, String) {
    let encoded = potion_deathrite_bounce_seed_with(974);
    let mut session =
        Session::new(&encoded).expect("valid sacrifice-control Deathrite revert session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_then_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
    });
    let deathrite_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned();
    end_then_draw(&mut session, "spellbook");
    end_then_draw(&mut session, "atlas");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana"
            && descriptor["unitInstanceId"] == deathrite_id.as_str()
    });
    end_then_draw(&mut session, "spellbook");
    let (ally, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C1"
    });
    let ally_id = ally["cardInstanceId"]
        .as_str()
        .expect("north ally identity")
        .to_owned();
    let (cast, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-potion"
            && descriptor["bearer"]["instanceId"] == ally_id.as_str()
    });
    let artifact_id = cast["cardInstanceId"]
        .as_str()
        .expect("potion identity")
        .to_owned();
    (session, ally_id, deathrite_id, artifact_id)
}

#[test]
fn rule_catalog_0974_sacrifice_artifact_deathrite_draws_for_original_controller_when_bearer_leaves_before_kill()
 {
    let (mut session, ally_id, deathrite_id, artifact_id) = potion_deathrite_bounce_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &deathrite_id)["controller"], "south");
    assert_eq!(unit(&before, &deathrite_id)["owner"], "south");
    assert_eq!(unit(&before, &deathrite_id)["tapped"], true);
    assert_eq!(unit(&before, &ally_id)["controller"], "north");

    let targets = potion_targets(&session);
    assert!(targets.contains(&deathrite_id), "{targets:?}");
    assert!(!targets.contains(&ally_id), "{targets:?}");

    let stolen = sacrifice_steal(&mut session, &artifact_id, &deathrite_id);
    assert_eq!(
        event_types(&stolen),
        ["artifact-sacrificed", "minion-control-changed"]
    );
    assert!(stolen.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "south"
            && event.payload["seat"] == "north"
            && event.payload["instanceId"] == deathrite_id
            && event.payload["sourceInstanceId"] == ally_id
    }));
    assert_eq!(unit(&state(&session), &deathrite_id)["controller"], "north");

    let (_, reverted) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bounce"
            && descriptor["target"]["instanceId"] == ally_id.as_str()
    });
    assert_eq!(
        event_types(&reverted),
        [
            "magic-cast",
            "minion-returned-to-hand",
            "minion-control-changed",
            "magic-resolved"
        ]
    );
    assert!(reverted.events.iter().any(|event| {
        event.event_type == "minion-control-changed"
            && event.payload["fromSeat"] == "north"
            && event.payload["seat"] == "south"
            && event.payload["instanceId"] == deathrite_id
    }));
    let reverted_state = state(&session);
    assert!(realm_unit(&reverted_state, &ally_id).is_none());
    let restored = realm_unit(&reverted_state, &deathrite_id).expect("Deathrite still on board");
    assert_eq!(restored["controller"], "south");
    assert_eq!(restored["owner"], "south");

    let before_kill = state(&session);
    let north_atlas = atlas_len(&before_kill, "north");
    let south_atlas = atlas_len(&before_kill, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == deathrite_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], deathrite_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &deathrite_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert!(cemetery_has(&finished, "south", &deathrite_id));
    assert!(!cemetery_has(&finished, "north", &deathrite_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0976_nearby_avatar_discard_deathrite_still_draws_for_permanent_thief_after_turn_boundary()
 {
    let (mut session, deathrite_id) = discard_deathrite_opening();
    let before = state(&session);
    assert_eq!(unit(&before, &deathrite_id)["controller"], "south");
    assert_eq!(unit(&before, &deathrite_id)["owner"], "south");

    steal_sellsword(&mut session, &deathrite_id);
    assert_eq!(unit(&state(&session), &deathrite_id)["controller"], "north");

    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !ended
            .events
            .iter()
            .any(|event| event.event_type == "minion-control-changed")
    );
    let persisted = state(&session);
    assert_eq!(
        realm_unit(&persisted, &deathrite_id).expect("stolen minion")["controller"],
        "north"
    );
    assert_eq!(
        realm_unit(&persisted, &deathrite_id).expect("stolen minion")["owner"],
        "south"
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    let before_kill = state(&session);
    let north_atlas = atlas_len(&before_kill, "north");
    let south_atlas = atlas_len(&before_kill, "south");
    assert_eq!(
        realm_unit(&before_kill, &deathrite_id).expect("stolen minion")["controller"],
        "north"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["instanceId"] == deathrite_id
    });
    let spell_id = lash["cardInstanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], deathrite_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "north");

    let finished = state(&session);
    assert!(realm_unit(&finished, &deathrite_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas - 1);
    assert_eq!(atlas_len(&finished, "south"), south_atlas);
    assert!(cemetery_has(&finished, "south", &deathrite_id));
    assert!(!cemetery_has(&finished, "north", &deathrite_id));
    assert_exact_replay(&session);
}

fn south_bolt() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn thais_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "previous-player-control-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-previous-player-control-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-site": site(),
            "north-thais": thais(),
            "south-avatar": avatar(),
            "south-bolt": south_bolt(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": vec!["north-thais"; 8],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-deathrite",
                    "south-deathrite",
                    "south-bolt",
                    "south-bolt",
                    "south-deathrite",
                    "south-bolt",
                    "south-deathrite",
                    "south-bolt",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn south_has_deathrite(snapshot: &Value) -> bool {
    snapshot["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South Spellbook")
        .iter()
        .any(|card| card["cardId"] == "south-deathrite")
}

fn south_has_bolt(snapshot: &Value) -> bool {
    snapshot["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "south-bolt"))
}

fn try_accept_where(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> bool {
    let Some(action) = session.legal_actions().ok().and_then(|actions| {
        actions
            .into_iter()
            .find(|action| predicate(&action.descriptor))
    }) else {
        return false;
    };
    matches!(
        session.step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        }),
        Ok(StepResult::Accepted(_))
    )
}

fn try_end_then_draw(session: &mut Session, zone: &str) -> bool {
    if !try_accept_where(session, |descriptor| descriptor["kind"] == "end-turn") {
        return false;
    }
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    }) || (zone == "spellbook"
        && try_accept_where(session, |descriptor| descriptor["kind"] == "draw-site"))
}

fn deathrite_instance_id(snapshot: &Value) -> Option<String> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["cardId"] == "south-deathrite")
        .and_then(|unit| unit["instanceId"].as_str().map(ToOwned::to_owned))
}

fn try_advance_hijacked_turn_draws(session: &mut Session) -> bool {
    while session.legal_actions().ok().is_some_and(|actions| {
        actions.iter().any(|action| {
            matches!(
                action.descriptor["kind"].as_str(),
                Some("draw-site" | "draw")
            )
        })
    }) {
        if !try_accept_where(session, |descriptor| {
            matches!(descriptor["kind"].as_str(), Some("draw-site" | "draw"))
        }) {
            return false;
        }
    }
    true
}

fn advance_hijacked_turn_draws(session: &mut Session) {
    assert!(try_advance_hijacked_turn_draws(session));
}

fn try_keep(session: &mut Session) -> bool {
    try_accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })
}

fn end_then_flexible_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        try_accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == zone
        }) || (zone == "spellbook"
            && try_accept_where(session, |descriptor| descriptor["kind"] == "draw-site")),
        "expected a start-of-turn draw after ending the turn"
    );
}

fn thais_deathrite_seed_with(start: u32) -> String {
    (start..start + 4096)
        .map(thais_deathrite_manifest)
        .find(|candidate| {
            let Ok(mut session) = Session::new(candidate) else {
                return false;
            };
            if !try_keep(&mut session) || !try_keep(&mut session) {
                return false;
            }
            if !try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
            }) {
                return false;
            }
            if !try_end_then_draw(&mut session, "spellbook")
                || !south_has_deathrite(&state(&session))
            {
                return false;
            }
            if !try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            }) {
                return false;
            }
            if !try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "south-deathrite"
                    && descriptor["cell"] == "C1"
            }) {
                return false;
            }
            if deathrite_instance_id(&state(&session)).is_none()
                || !try_end_then_draw(&mut session, "spellbook")
            {
                return false;
            }
            if !try_accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "summon-minion"
                    && descriptor["cardId"] == "north-thais"
                    && descriptor["cell"] == "C4"
            }) || !try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")
            {
                return false;
            }
            let Some(deathrite_id) = deathrite_instance_id(&state(&session)) else {
                return false;
            };
            if !try_advance_hijacked_turn_draws(&mut session) || !south_has_bolt(&state(&session)) {
                return false;
            }
            session.legal_actions().ok().is_some_and(|actions| {
                actions.iter().any(|action| {
                    action.descriptor["kind"] == "cast-magic"
                        && action.descriptor["cardId"] == "south-bolt"
                        && action.descriptor["target"]["instanceId"] == deathrite_id
                })
            })
        })
        .expect("bounded seed reaching a hijacked turn with Bolt on Deathrite")
}

/// South Deathrite at C1, North Thais at C4 after two turns; South's turn is hijacked by North.
fn thais_deathrite_opening() -> (Session, String, String) {
    let encoded = thais_deathrite_seed_with(975);
    let mut session =
        Session::new(&encoded).expect("valid previous-player-control Deathrite session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    end_then_flexible_draw(&mut session, "spellbook");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
    });
    let deathrite_id = deathrite_instance_id(&state(&session)).expect("South Deathrite on board");
    end_then_flexible_draw(&mut session, "spellbook");
    let (thais_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-thais"
            && descriptor["cell"] == "C4"
    });
    let thais_id = thais_summon["cardInstanceId"]
        .as_str()
        .expect("Thais identity")
        .to_owned();
    let (_, ended) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(ended.events.iter().any(|event| {
        event.event_type == "player-controlled"
            && event.payload["seat"] == "south"
            && event.payload["controller"] == "north"
            && event.payload["sourceInstanceId"] == thais_id
    }));
    let hijacked = state(&session);
    assert_eq!(hijacked["activeSeat"], "south");
    assert_eq!(hijacked["turnController"], "north");
    assert_eq!(unit(&hijacked, &deathrite_id)["controller"], "south");
    assert_acting_seat(&session, Seat::North);
    (session, thais_id, deathrite_id)
}

#[test]
fn rule_catalog_0975_previous_player_control_deathrite_draws_for_minion_controller_not_turn_controller()
 {
    let (mut session, _thais_id, deathrite_id) = thais_deathrite_opening();
    assert_eq!(unit(&state(&session), &deathrite_id)["controller"], "south");
    assert_eq!(unit(&state(&session), &deathrite_id)["owner"], "south");

    advance_hijacked_turn_draws(&mut session);
    let before_kill = state(&session);
    let north_atlas = atlas_len(&before_kill, "north");
    let south_atlas = atlas_len(&before_kill, "south");
    let (bolt, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "south-bolt"
            && descriptor["target"]["instanceId"] == deathrite_id.as_str()
    });
    let spell_id = bolt["cardInstanceId"]
        .as_str()
        .expect("Bolt identity")
        .to_owned();
    assert_eq!(
        event_types(&killed),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], deathrite_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(realm_unit(&finished, &deathrite_id).is_none());
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert!(cemetery_has(&finished, "south", &deathrite_id));
    assert!(!cemetery_has(&finished, "north", &deathrite_id));
    assert_exact_replay(&session);
}

fn rain() -> Value {
    json!({
        "cardType": "magic",
        "damageEachAbovegroundMinion": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_potion_withheld_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "sacrifice-control-artifact-deathrite-withheld" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sacrifice-control-artifact-deathrite-withheld-v1",
        },
        "cards": {
            "north-ally": dummy(),
            "north-avatar": avatar(),
            "north-potion": potion(),
            "north-rain": rain(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-near": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 8],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-ally",
                    "north-potion",
                    "north-rain",
                    "north-rain",
                    "north-ally",
                    "north-potion",
                    "north-rain",
                    "north-potion"
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 8],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-deathrite",
                    "south-deathrite",
                    "south-near",
                    "south-deathrite",
                    "south-near",
                    "south-deathrite",
                    "south-near",
                    "south-near"
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn north_has_rain(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-rain"))
}

fn try_take_action(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> Option<Value> {
    let action = session
        .legal_actions()
        .ok()?
        .into_iter()
        .find(|action| predicate(&action.descriptor))?;
    let descriptor = action.descriptor.clone();
    let StepResult::Accepted(_) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .ok()?
    else {
        return None;
    };
    Some(descriptor)
}

struct PendingDeathritePotionSetup {
    artifact_id: String,
    deathrite_ids: [String; 2],
    near_id: String,
    session: Session,
}

/// Potion opening plus two South Deathrites and Rain: ready sacrifice-control, then pending order.
fn try_pending_deathrite_with_ready_potion(encoded: &str) -> Option<PendingDeathritePotionSetup> {
    let mut session = Session::new(encoded).ok()?;
    try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    })?;
    try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let ally = try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let ally_id = ally["cardInstanceId"].as_str()?.to_owned();
    try_take_action(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let first = try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let second = try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C1"
            && descriptor["region"].is_null()
    })?;
    let near = try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-near"
            && descriptor["cell"] == "C4"
    })?;
    let near_id = near["cardInstanceId"].as_str()?.to_owned();
    try_take_action(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let cast = try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-potion"
            && descriptor["bearer"]["instanceId"] == ally_id.as_str()
    })?;
    let artifact_id = cast["cardInstanceId"].as_str()?.to_owned();
    if !potion_targets(&session).contains(&near_id) {
        return None;
    }
    if !north_has_rain(&state(&session)) {
        return None;
    }
    try_take_action(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-rain"
    })?;
    if state(&session)["phase"] != "deathrite-order" {
        return None;
    }
    let mut deathrite_ids = [
        first["cardInstanceId"].as_str()?.to_owned(),
        second["cardInstanceId"].as_str()?.to_owned(),
    ];
    deathrite_ids.sort_unstable();
    Some(PendingDeathritePotionSetup {
        artifact_id,
        deathrite_ids,
        near_id,
        session,
    })
}

fn deathrite_potion_withheld_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_potion_withheld_manifest)
        .find(|candidate| try_pending_deathrite_with_ready_potion(candidate).is_some())
        .expect(
            "bounded seed that reaches pending Deathrites with ready activate-artifact-sacrifice-control",
        )
}

#[test]
fn rule_catalog_1161_activate_artifact_sacrifice_control_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_potion_withheld_seed_with(1161);
    let mut setup = try_pending_deathrite_with_ready_potion(&encoded)
        .expect("complete activate-artifact-sacrifice-control Deathrite withheld setup");
    let artifact_id = setup.artifact_id.clone();
    let deathrite_ids = setup.deathrite_ids.clone();
    let near_id = setup.near_id.clone();
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
    assert_eq!(unit(&paused, &near_id)["controller"], "south");
    assert_eq!(
        paused["realm"]["artifacts"]
            .as_array()
            .expect("realm artifacts")
            .iter()
            .find(|artifact| artifact["instanceId"] == artifact_id)
            .expect("ready potion")["cardId"],
        "north-potion"
    );
    assert!(potion_targets(session).is_empty());
    assert!(session
        .legal_actions()
        .expect("paused legal actions")
        .iter()
        .all(|action| action.descriptor["kind"] != "activate-artifact-sacrifice-control"));

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
    assert!(potion_targets(session).contains(&near_id));
    assert_exact_replay(session);
}
