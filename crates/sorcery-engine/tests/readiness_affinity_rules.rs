use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn minion(attack: u8, defense: u8) -> Value {
    json!({
        "attack": attack,
        "cardType": "minion",
        "defense": defense,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn scenario_manifest(
    seed: u32,
    extra_cards: &Value,
    north_spellbook: &[&str],
    south_spellbook: &[&str],
) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let site = json!({ "cardType": "site", "elements": ["earth"] });
    let mut cards = json!({
        "north-avatar": avatar,
        "north-site": site,
        "south-avatar": avatar,
        "south-site": site,
    });
    cards.as_object_mut().expect("card definitions").extend(
        extra_cards
            .as_object()
            .expect("extra card definitions")
            .clone(),
    );
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "readiness-affinity-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-readiness-affinity-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 4],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
    canonical_json(&manifest).expect("canonical synthetic manifest")
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

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
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
fn charge_should_allow_immediate_move_and_attack_without_clearing_sickness() {
    let mut charge = minion(1, 2);
    charge["charge"] = json!(true);
    let manifest = scenario_manifest(
        52,
        &json!({
            "north-charge": charge,
            "south-minion": minion(1, 2),
        }),
        &["north-charge"; 4],
        &["south-minion"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid Charge scenario");
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
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-charge"
            && descriptor["cell"] == "C4"
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("Charge instance identity")
        .to_owned();
    let (_, movement) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == instance_id
            && descriptor["from"]["cell"] == "C4"
            && descriptor["to"]["cell"] == "C3"
    });

    let after = state(&session);
    let unit = after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("summoned Charge minion");
    assert_eq!(unit["summoningSickness"], true);
    assert_eq!(unit["location"], "C3");
    assert_eq!(movement.events[0].event_type, "move-and-attack-activated");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    assert_exact_replay(&session);
}

#[test]
fn restricted_attacker_should_preserve_unit_targets_and_filter_site_target() {
    let mut restricted = minion(1, 3);
    restricted["cannotAttackSites"] = json!(true);
    restricted["charge"] = json!(true);
    restricted["summonToAnySite"] = json!(true);
    let manifest = scenario_manifest(
        83,
        &json!({
            "north-restricted": restricted,
            "south-target": minion(1, 3),
        }),
        &["north-restricted"; 4],
        &["south-target"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid restricted attacker scenario");
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
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (target_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "south-target"
    });
    let target_instance_id = target_summon["cardInstanceId"]
        .as_str()
        .expect("target instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    let (attacker_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-restricted"
            && descriptor["cell"] == "C1"
    });
    let attacker_instance_id = attacker_summon["cardInstanceId"]
        .as_str()
        .expect("attacker instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_instance_id
            && descriptor["to"]["cell"] == "C1"
    });
    let targets: Vec<_> = session
        .legal_actions()
        .expect("attack actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "declare-attack")
        .map(|action| action.descriptor["target"]["kind"].clone())
        .collect();
    assert!(targets.contains(&json!("avatar")));
    assert!(targets.contains(&json!("minion")));
    assert!(!targets.contains(&json!("site")));

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_instance_id
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend"
    });
    assert_exact_replay(&session);
}

#[test]
fn provider_affinity_should_stop_immediately_when_provider_dies() {
    let mut provider = minion(0, 1);
    provider["provides"] = json!("earth");
    let mut threshold_minion = minion(1, 2);
    threshold_minion["thresholds"]["earth"] = json!(2);
    let mut killer = minion(1, 2);
    killer["charge"] = json!(true);
    killer["summonToAnySite"] = json!(true);
    let manifest = scenario_manifest(
        95,
        &json!({
            "north-provider": provider,
            "north-threshold": threshold_minion,
            "south-killer": killer,
        }),
        &[
            "north-provider",
            "north-provider",
            "north-threshold",
            "north-threshold",
        ],
        &["south-killer"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid provider scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (provider_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-provider"
    });
    let provider_instance_id = provider_summon["cardInstanceId"]
        .as_str()
        .expect("provider instance identity")
        .to_owned();
    assert!(
        session
            .legal_actions()
            .expect("provider affinity actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-threshold")
    );
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (killer_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-killer"
            && descriptor["cell"] == "C4"
    });
    let killer_instance_id = killer_summon["cardInstanceId"]
        .as_str()
        .expect("killer instance identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == killer_instance_id
            && descriptor["to"]["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == provider_instance_id
    });
    let (_, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "close-defend"
    });
    assert!(fight.events.iter().any(|event| {
        event.event_type == "minion-died" && event.payload["instanceId"] == provider_instance_id
    }));
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });

    assert!(
        !session
            .legal_actions()
            .expect("post-death actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-threshold")
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "direct scenario proof keeps the readiness and expiration sequence visible"
)]
fn mana_activation_should_require_readiness_tap_add_printed_mana_reveal_and_expire() {
    let mut mana_source = minion(1, 2);
    mana_source["charge"] = json!(true);
    mana_source["stealth"] = json!(true);
    mana_source["tapForMana"] = json!(2);
    let manifest = scenario_manifest(
        39,
        &json!({
            "north-mana-source": mana_source,
            "south-minion": minion(1, 2),
        }),
        &["north-mana-source"; 4],
        &["south-minion"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid mana activation scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-mana-source"
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("mana source identity")
        .to_owned();

    assert_eq!(state(&session)["realm"]["units"][0]["stealthed"], true);
    assert!(
        !session
            .legal_actions()
            .expect("summoning-sick actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "activate-mana")
    );

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });

    let before_mana = state(&session)["players"]["north"]["mana"]
        .as_u64()
        .expect("north mana");
    let action = session
        .legal_actions()
        .expect("ready mana actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "activate-mana"
                && action.descriptor["amount"] == 2
                && action.descriptor["unitInstanceId"] == instance_id
        })
        .expect("printed mana activation");
    assert!(action.label.ends_with(" for 2 mana"));
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("authoritative mana activation")
    else {
        panic!("engine-issued mana activation must be accepted");
    };
    let after = state(&session);
    let source = after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("mana source");

    assert_eq!(after["players"]["north"]["mana"], before_mana + 2);
    assert_eq!(source["tapped"], true);
    assert_eq!(source["stealthed"], false);
    assert_eq!(source["lastInteractedTurn"], 3);
    assert_eq!(
        receipt
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["mana-activated", "stealth-lost"]
    );
    assert_eq!(
        receipt.events[0].payload,
        json!({ "amount": 2, "seat": "north", "unitInstanceId": instance_id })
    );
    assert!(
        !session
            .legal_actions()
            .expect("post-activation actions")
            .iter()
            .any(|action| action.descriptor["kind"] == "activate-mana")
    );

    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let refreshed = state(&session);
    assert_eq!(refreshed["players"]["north"]["mana"], 1);
    assert_eq!(refreshed["realm"]["units"][0]["tapped"], false);
    assert_exact_replay(&session);
}

#[test]
fn unconditional_end_turn_stealth_should_gain_before_turn_events_without_duplicates() {
    let mut fox = minion(1, 2);
    fox["gainsStealthAtEndOfTurn"] = json!(true);
    fox["stealth"] = json!(true);
    fox["tapForMana"] = json!(1);
    let manifest = scenario_manifest(
        126,
        &json!({
            "north-fox": fox,
            "south-minion": minion(1, 2),
        }),
        &["north-fox"; 4],
        &["south-minion"; 4],
    );
    let mut session = Session::new(&manifest).expect("valid end-turn Stealth scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cardId"] == "north-fox"
    });
    let instance_id = summon["cardInstanceId"]
        .as_str()
        .expect("Stealth source identity")
        .to_owned();
    assert_eq!(state(&session)["realm"]["units"][0]["stealthed"], true);

    let (_, first_end) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !first_end
            .events
            .iter()
            .any(|event| event.event_type == "stealth-gained")
    );
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "draw");

    let (_, activation) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-mana" && descriptor["unitInstanceId"] == instance_id
    });
    assert_eq!(
        activation
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["mana-activated", "stealth-lost"]
    );
    let (_, regained) = accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert_eq!(
        regained
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["stealth-gained", "turn-ended", "turn-started"]
    );
    assert_eq!(
        regained.events[0].payload,
        json!({ "instanceId": instance_id, "seat": "north" })
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, no_duplicate) =
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    assert!(
        !no_duplicate
            .events
            .iter()
            .any(|event| event.event_type == "stealth-gained")
    );
    assert_eq!(state(&session)["realm"]["units"][0]["stealthed"], true);
    assert_exact_replay(&session);
}
