//! Granary Rats suppression branches the TypeScript setup suite used to prove with forged state.
//!
//! `tests/engine/game-setup-05.test.ts` hand-built rat units to show that a rat in the void
//! suppresses nothing and that one disabled rat does not lift the suppression an enabled twin
//! still applies. `rule_catalog_0094_granary_rats_suppress_site_threshold_while_enabled` in
//! `readiness_affinity_rules.rs` already covers the single-rat, all-disabled, and protected-site
//! branches; these two reach the remaining ones through legal play.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

fn granary_rats(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "siteProvidesNoThreshold": true,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra Granary Rats facts must be an object");
    };
    value
        .as_object_mut()
        .expect("Granary Rats facts")
        .extend(extra);
    value
}

fn manifest(seed: u32, extra_cards: &Value, south_spellbook: &[&str]) -> String {
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
        "north-gated": {
            "attack": 1,
            "cardType": "minion",
            "defense": 1,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
        },
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
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "granary-rats-cutover" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-granary-rats-cutover-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 20],
                "avatar": "north-avatar",
                "spellbook": vec!["north-gated"; 4],
            },
            "south": {
                "atlas": vec!["south-site"; 20],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    });
    value["manifestId"] = json!(identity_hash(&value).expect("manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
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

fn gated_summon_is_legal(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("threshold actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-gated"
        })
}

fn end_turn_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| descriptor["kind"] == "draw");
}

fn descriptors_of_kind(session: &Session, kind: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .map(|action| action.descriptor)
        .filter(|descriptor| descriptor["kind"] == kind)
        .collect()
}

/// North opens on C4, confirms the site still pays its Earth threshold, and hands over.
fn opening(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid Granary Rats scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    assert!(
        gated_summon_is_legal(&session),
        "the site should pay its threshold before any Granary Rats arrives"
    );
    session
}

#[test]
fn void_granary_rats_leaves_every_site_threshold_alone() {
    let manifest = manifest(
        94,
        &json!({ "south-rats": granary_rats(json!({ "voidwalk": true })) }),
        &["south-rats"; 4],
    );
    let mut session = opening(&manifest);

    end_turn_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-rats"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let rat_id = summon["cardInstanceId"]
        .as_str()
        .expect("Granary Rats identity")
        .to_owned();
    end_turn_and_draw(&mut session);
    assert!(
        !gated_summon_is_legal(&session),
        "a surface Granary Rats on the site should suppress its threshold"
    );

    // The void covers only cells no site occupies, so a rat that walks into it stands over no
    // site at all and the Earth threshold comes straight back.
    end_turn_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == rat_id
            && descriptor["to"]["region"] == "void"
    });
    if !descriptors_of_kind(&session, "decline-attack").is_empty() {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "decline-attack"
        });
    }
    let walked = state(&session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == rat_id)
        .expect("void walker")
        .clone();
    assert_eq!(walked["region"], json!("void"));
    end_turn_and_draw(&mut session);
    assert!(
        gated_summon_is_legal(&session),
        "a Granary Rats in the void should suppress no site"
    );
}

#[test]
fn one_disabled_granary_rats_still_suppresses_beside_its_enabled_twin() {
    let manifest = manifest(
        94,
        &json!({
            "south-rats": granary_rats(json!({})),
            "south-rats-disabled": granary_rats(json!({ "genesisDisableSelfUntilDamaged": true })),
        }),
        &[
            "south-rats-disabled",
            "south-rats",
            "south-rats",
            "south-rats-disabled",
        ],
    );
    let mut session = opening(&manifest);

    end_turn_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (disabled_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-rats-disabled"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let disabled_id = disabled_summon["cardInstanceId"]
        .as_str()
        .expect("disabled Granary Rats identity")
        .to_owned();
    end_turn_and_draw(&mut session);
    assert!(
        gated_summon_is_legal(&session),
        "a lone disabled Granary Rats should not suppress the site"
    );

    end_turn_and_draw(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (enabled_summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-rats"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    });
    let enabled_id = enabled_summon["cardInstanceId"]
        .as_str()
        .expect("enabled Granary Rats identity")
        .to_owned();
    end_turn_and_draw(&mut session);

    let units = state(&session)["realm"]["units"].clone();
    let flag = |instance_id: &str| {
        units
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["instanceId"] == instance_id)
            .expect("summoned Granary Rats")["disabledUntilDamaged"]
            .as_bool()
            .unwrap_or(false)
    };
    assert!(flag(&disabled_id));
    assert!(!flag(&enabled_id));
    assert!(
        !gated_summon_is_legal(&session),
        "one enabled Granary Rats keeps the site suppressed beside a disabled twin"
    );
}
