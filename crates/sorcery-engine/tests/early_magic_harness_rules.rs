//! Direct proofs for targeted Magic as a non-unit source
//! (RULE-CATALOG-0019, RULE-CATALOG-0685–0686).
//!
//! 0595–0596 prove ordinary Zap lethal/Ward. 0659–0660 prove Deathrite after
//! nearby-control. These proofs keep the 0019 harness slice: Magic still kills
//! a Deathrite minion that prevents high-power unit damage, and a later-turn
//! Zap is the death blow after Death's Door.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn avatar(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    })
}

fn site() -> Value {
    json!({
        "cardType": "site",
        "elements": ["earth"],
    })
}

fn deathrite() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "preventsDamageFromUnitsWithPowerAtLeast": 4,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
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

fn lash() -> Value {
    json!({
        "cardType": "magic",
        "damageTargetUnit": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "early-harness-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-early-harness-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(20),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(20),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lash"; 6],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-deathrite"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn avatar_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "early-harness-avatar" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-early-harness-avatar-v1",
        },
        "cards": {
            "north-avatar": avatar(1),
            "north-lash": lash(),
            "north-site": site(),
            "south-avatar": avatar(1),
            "south-dummy": dummy(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-lash"; 6],
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

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid early Magic harness session");
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

fn north_has_lash(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| hand.iter().any(|card| card["cardId"] == "north-lash"))
}

fn seed_with(start: u32, deathrite: bool) -> String {
    (start..start + 256)
        .map(|seed| {
            if deathrite {
                deathrite_manifest(seed)
            } else {
                avatar_manifest(seed)
            }
        })
        .find(|candidate| {
            Session::new(candidate)
                .ok()
                .is_some_and(|preview| north_has_lash(&state(&preview)))
        })
        .expect("bounded seed with Lash in the opening hand")
}

fn stage_south_turn(session: &mut Session, summon_deathrite: bool) -> Option<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let summoned = summon_deathrite.then(|| {
        let (descriptor, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-deathrite"
                && descriptor["cell"] == "C1"
                && descriptor["region"].is_null()
        });
        descriptor["cardInstanceId"]
            .as_str()
            .expect("Deathrite minion identity")
            .to_owned()
    });
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    summoned
}

fn stage_south_deathrite(session: &mut Session) -> String {
    stage_south_turn(session, true).expect("Deathrite minion identity")
}

fn stage_south_site(session: &mut Session) {
    assert!(stage_south_turn(session, false).is_none());
}

fn pass_round_drawing_spells(session: &mut Session) {
    for _ in 0..2 {
        accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
    }
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

fn atlas_hand_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["hand"]["atlas"]
        .as_array()
        .expect("atlas hand")
        .len()
}

fn lash_target_keys(session: &Session, spell_id: &str) -> Vec<String> {
    let mut keys: Vec<_> = session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == spell_id
        })
        .map(|action| {
            format!(
                "{}:{}:{}",
                action.descriptor["target"]["kind"]
                    .as_str()
                    .expect("target kind"),
                action.descriptor["target"]["seat"]
                    .as_str()
                    .expect("target seat"),
                action.descriptor["target"]["instanceId"]
                    .as_str()
                    .expect("target identity")
            )
        })
        .collect();
    keys.sort_unstable();
    keys
}

fn assert_lash_targets_are_canonical(session: &Session, spell_id: &str) {
    let descriptors: Vec<_> = session
        .legal_actions()
        .expect("Lash actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardInstanceId"] == spell_id
        })
        .map(|action| action.descriptor)
        .collect();
    let canonical: Vec<_> = descriptors
        .iter()
        .map(|descriptor| canonical_json(descriptor).expect("canonical target descriptor"))
        .collect();
    let mut sorted = canonical.clone();
    sorted.sort_unstable();
    assert_eq!(canonical, sorted);
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
fn rule_catalog_0685_targeted_magic_kills_a_deathrite_minion_as_a_non_unit_source() {
    let encoded = seed_with(685, true);
    let mut session = opening_main(&encoded);
    let target_id = stage_south_deathrite(&mut session);

    let before = state(&session);
    let spell_id = before["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North Spellbook")
        .iter()
        .find(|card| card["cardId"] == "north-lash")
        .expect("Lash in hand")["instanceId"]
        .as_str()
        .expect("Lash identity")
        .to_owned();
    assert_lash_targets_are_canonical(&session, &spell_id);
    let mut expected = vec![
        format!(
            "avatar:north:{}",
            before["players"]["north"]["avatar"]["card"]["instanceId"]
                .as_str()
                .expect("North Avatar identity")
        ),
        format!(
            "avatar:south:{}",
            before["players"]["south"]["avatar"]["card"]["instanceId"]
                .as_str()
                .expect("South Avatar identity")
        ),
        format!("minion:south:{target_id}"),
    ];
    expected.sort_unstable();
    assert_eq!(lash_target_keys(&session, &spell_id), expected);

    let south_atlas = atlas_len(&before, "south");
    let south_atlas_hand = atlas_hand_len(&before, "south");
    let (lash, killed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(lash["cardInstanceId"], spell_id);
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
    assert_eq!(killed.events[1].payload["amount"], 1);
    assert_eq!(killed.events[1].payload["sourceInstanceId"], spell_id);
    assert_eq!(killed.events[1].payload["targetInstanceId"], target_id);
    let drawn = killed
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");

    let finished = state(&session);
    assert!(
        !finished["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == target_id)
    );
    assert!(cemetery_has(&finished, "south", &target_id));
    assert!(cemetery_has(&finished, "north", &spell_id));
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_eq!(atlas_hand_len(&finished, "south"), south_atlas_hand + 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0686_targeted_magic_reaches_deaths_door_then_defeats_the_avatar() {
    let encoded = seed_with(686, false);
    let mut session = opening_main(&encoded);
    stage_south_site(&mut session);
    let south_avatar = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("South Avatar identity")
        .to_owned();

    let (_, first) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    assert!(event_types(&first).contains(&"magic-damage-allocated"));
    assert!(event_types(&first).contains(&"avatar-reached-deaths-door"));
    assert!(!event_types(&first).contains(&"game-ended"));
    let after_first = state(&session);
    assert_eq!(after_first["players"]["south"]["avatar"]["life"], 0);
    assert_eq!(after_first["terminal"]["status"], "active");

    pass_round_drawing_spells(&mut session);
    let (_, death_blow) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-lash"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar
    });
    let terminal = state(&session)["terminal"].clone();
    assert_eq!(terminal["status"], "finished");
    assert_eq!(terminal["winner"], "north");
    assert_eq!(terminal["loser"], "south");
    assert_eq!(terminal["reason"], "avatar_defeated");
    assert_eq!(
        event_types(&death_blow)
            .into_iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>(),
        ["game-ended", "magic-resolved"]
    );
    assert!(event_types(&death_blow).contains(&"death-blow"));
    assert_exact_replay(&session);
}
