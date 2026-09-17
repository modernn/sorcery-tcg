//! Direct proofs for destroy-own-artifact-at-location-for-area-damage Magic
//! (RULE-CATALOG-0577–0578, 1040, 1100).
//!
//! 1040 covers detonate killing a Deathrite minion: the controller draws a site
//! and magic-resolved only appears after deathrite settlement.
//!
//! Ordinary Magic offers each Artifact the caster controls, loose or carried,
//! paired with that Artifact's location. Casting destroys the chosen Artifact
//! and deals 3 damage to each other Unit there. The destroyed Artifact's
//! bearer is excluded. No controlled Artifact leaves the spell unoffered.
//! While Deathrites wait for ordering, this Magic stays withheld until the
//! chain drains.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Event, Receipt};
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

fn relic() -> Value {
    json!({
        "cardType": "artifact",
        "grantsBearerPower": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn raider() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 4,
        "manaCost": 0,
        "summonToAnySite": true,
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

fn detonate() -> Value {
    json!({
        "cardType": "magic",
        "destroyOwnArtifactAtLocationForAreaDamage": 3,
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn detonate_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "detonate-own-artifact" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-detonate-own-artifact-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-detonate": detonate(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-raider": raider(),
            "south-relic": relic(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-relic",
                    "north-relic",
                    "north-relic",
                    "north-detonate",
                    "north-detonate",
                    "north-detonate",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-relic",
                    "south-relic",
                    "south-relic",
                    "south-raider",
                    "south-raider",
                    "south-raider",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn detonate_deathrite_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "detonate-own-artifact-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-detonate-own-artifact-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-detonate": detonate(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite_minion(),
            "south-relic": relic(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-relic",
                    "north-relic",
                    "north-relic",
                    "north-detonate",
                    "north-detonate",
                    "north-detonate",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-relic",
                    "south-relic",
                    "south-relic",
                    "south-deathrite",
                    "south-deathrite",
                    "south-deathrite",
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
    let mut session = Session::new(encoded).expect("valid detonate-own-artifact session");
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

fn seed_with(required: &[&str]) -> String {
    (577..577 + 256)
        .map(detonate_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            required.iter().all(|id| hand.iter().any(|card| card == id))
        })
        .expect("bounded seed with required opening cards")
}

fn seed_with_deathrite(start: u32) -> String {
    (start..start + 256)
        .map(detonate_deathrite_manifest)
        .find(|candidate| {
            let hand = opening_spell_ids(candidate);
            hand.iter().any(|card| card == "north-detonate")
                && hand.iter().any(|card| card == "north-relic")
        })
        .expect("bounded seed with detonate Deathrite setup")
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn end_and_draw(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn south_plays_c1(session: &mut Session) {
    end_and_draw(session);
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn south_ends_after_c1(session: &mut Session) {
    south_plays_c1(session);
    end_and_draw(session);
}

fn receipt_event<'a>(receipt: &'a Receipt, event_type: &str) -> &'a Event {
    receipt
        .events
        .iter()
        .find(|event| event.event_type == event_type)
        .unwrap_or_else(|| panic!("missing {event_type}"))
}

fn realm_artifact<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["artifacts"]
        .as_array()?
        .iter()
        .find(|artifact| artifact["instanceId"] == instance_id)
}

fn artifact_at(session: &Session, card_id: &str, cell: &str) -> String {
    state(session)["realm"]["artifacts"]
        .as_array()
        .expect("realm artifacts")
        .iter()
        .find(|artifact| artifact["cardId"] == card_id && artifact["location"] == cell)
        .expect("expected artifact at cell")["instanceId"]
        .as_str()
        .expect("artifact instance identity")
        .to_owned()
}

fn unit<'a>(snapshot: &'a Value, instance_id: &str) -> &'a Value {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .expect("expected unit")
}

fn detonate_casts(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("detonate actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-detonate"
        })
        .map(|action| action.descriptor)
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

fn deathrite_detonate_manifest(seed: u32) -> String {
    let fixture = "detonate-own-artifact-deathrite-withheld";
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": {
            "north-avatar": avatar(),
            "north-detonate": detonate(),
            "north-rain": rain_spell(),
            "north-relic": relic(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": deathrite_minion(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-detonate",
                    "north-rain",
                    "north-relic",
                    "north-detonate",
                    "north-rain",
                    "north-relic",
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

fn north_has_detonate_rain_and_relic(snapshot: &Value) -> bool {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .is_some_and(|hand| {
            ["north-detonate", "north-rain", "north-relic"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
}

struct PendingDeathriteDetonateSetup {
    deathrite_ids: [String; 2],
    relic_id: String,
    session: Session,
}

fn try_pending_deathrite_with_own_relic(encoded: &str) -> Option<PendingDeathriteDetonateSetup> {
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
    if !north_has_detonate_rain_and_relic(&state(&session)) {
        return None;
    }
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    })?;
    let relic_id = artifact_at(&session, "north-relic", "C3");
    if !detonate_casts(&session).iter().any(|cast| {
        cast["targetArtifactInstanceId"] == relic_id && cast["targetLocation"]["cell"] == "C3"
    }) {
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
    Some(PendingDeathriteDetonateSetup {
        deathrite_ids,
        relic_id,
        session,
    })
}

fn deathrite_detonate_seed_with(start: u32) -> String {
    (start..start + 2048)
        .map(deathrite_detonate_manifest)
        .find(|candidate| try_pending_deathrite_with_own_relic(candidate).is_some())
        .expect("bounded seed that reaches pending Deathrites with Detonate Magic in hand")
}

#[test]
fn rule_catalog_0577_detonate_destroys_own_relic_and_deals_three_to_an_enemy() {
    let encoded = seed_with(&["north-detonate", "north-relic"]);
    let mut session = opening_main(&encoded);
    south_ends_after_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let relic_id = artifact_at(&session, "north-relic", "C3");
    end_and_draw(&mut session);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-raider"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    let enemy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("enemy instance identity")
        .to_owned();
    end_and_draw(&mut session);

    let before = state(&session);
    assert_eq!(
        realm_artifact(&before, &relic_id).expect("own relic")["location"],
        "C3"
    );
    assert_eq!(unit(&before, &enemy_id)["location"], "C3");
    assert_eq!(unit(&before, &enemy_id)["damage"], 0);
    let offered = detonate_casts(&session);
    assert!(!offered.is_empty());
    assert!(offered.iter().all(|cast| {
        cast["targetArtifactInstanceId"] == relic_id && cast["targetLocation"]["cell"] == "C3"
    }));

    let (cast, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-detonate"
            && descriptor["targetArtifactInstanceId"] == relic_id
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    let types = event_types(&receipt);
    assert_eq!(types.first(), Some(&"magic-cast"));
    assert_eq!(types.last(), Some(&"magic-resolved"));
    assert!(types.contains(&"artifact-destroyed"));
    assert!(types.contains(&"magic-damage-allocated"));
    assert!(types.contains(&"damage-dealt"));
    assert!(!types.contains(&"artifact-banished"));
    let destroyed = receipt_event(&receipt, "artifact-destroyed");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], relic_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let allocated = receipt_event(&receipt, "magic-damage-allocated");
    assert_eq!(allocated.payload["amount"], 3);
    assert_eq!(allocated.payload["targetInstanceId"], enemy_id);
    assert_eq!(
        allocated.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    let dealt = receipt_event(&receipt, "damage-dealt");
    assert_eq!(dealt.payload["amount"], 3);
    assert_eq!(dealt.payload["instanceId"], enemy_id);
    assert_eq!(dealt.payload["seat"], "south");
    let after = state(&session);
    assert!(realm_artifact(&after, &relic_id).is_none());
    assert_eq!(unit(&after, &enemy_id)["location"], "C3");
    assert_eq!(unit(&after, &enemy_id)["damage"], 3);
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 20);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == relic_id)
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0578_detonate_is_unoffered_without_an_own_artifact() {
    let encoded = seed_with(&["north-detonate"]);
    let mut session = opening_main(&encoded);
    south_plays_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "south-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C1"
    });
    end_and_draw(&mut session);
    assert!(
        realm_artifact(
            &state(&session),
            &artifact_at(&session, "south-relic", "C1")
        )
        .is_some()
    );
    assert!(
        state(&session)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("north Spellbook hand")
            .iter()
            .any(|card| card["cardId"] == "north-detonate")
    );
    assert!(detonate_casts(&session).is_empty());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1040_detonate_deathrite_draws_for_controller_on_kill() {
    let encoded = seed_with_deathrite(1040);
    let mut session = opening_main(&encoded);
    south_ends_after_c1(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "north-relic"
            && descriptor["bearer"].is_null()
            && descriptor["cell"] == "C3"
    });
    let relic_id = artifact_at(&session, "north-relic", "C3");
    end_and_draw(&mut session);
    let (summoned, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C3"
            && descriptor["region"].is_null()
    });
    let enemy_id = summoned["cardInstanceId"]
        .as_str()
        .expect("enemy instance identity")
        .to_owned();
    end_and_draw(&mut session);

    let before = state(&session);
    let north_atlas = atlas_len(&before, "north");
    let south_atlas = atlas_len(&before, "south");

    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-detonate"
            && descriptor["targetArtifactInstanceId"] == relic_id
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        [
            "magic-cast",
            "artifact-destroyed",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(receipt.events[2].payload["targetInstanceId"], enemy_id);
    let drawn = receipt
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], enemy_id);
    let types = event_types(&receipt);
    let damage_dealt = types
        .iter()
        .position(|event_type| *event_type == "damage-dealt")
        .expect("damage-dealt index");
    let site_drawn = types
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let minion_died = types
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("minion-died index");
    let magic_resolved = types
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        damage_dealt < site_drawn && site_drawn < minion_died && minion_died < magic_resolved,
        "expected damage-dealt, deathrite site-drawn, minion-died, then magic-resolved; got {types:?}"
    );
    assert_eq!(types.last(), Some(&"magic-resolved"));

    let finished = state(&session);
    assert!(
        finished["players"]["south"]["cemetery"]
            .as_array()
            .expect("south cemetery")
            .iter()
            .any(|card| card["instanceId"] == enemy_id)
    );
    assert_eq!(atlas_len(&finished, "north"), north_atlas);
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_1100_detonate_withheld_during_pending_deathrite_order() {
    let encoded = deathrite_detonate_seed_with(1100);
    let mut setup = try_pending_deathrite_with_own_relic(&encoded)
        .expect("complete Detonate Deathrite withheld setup");
    let relic_id = setup.relic_id.clone();
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
    assert_eq!(
        realm_artifact(&paused, &relic_id).expect("own relic")["location"],
        "C3"
    );
    assert!(
        session
            .legal_actions()
            .expect("paused legal actions")
            .iter()
            .all(|action| action.descriptor["kind"] != "cast-magic")
    );
    assert!(detonate_casts(session).is_empty());

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
    assert_eq!(
        realm_artifact(&resumed, &relic_id).expect("own relic remains")["location"],
        "C3"
    );
    let offered = detonate_casts(session);
    assert!(!offered.is_empty());
    assert!(offered.iter().all(|cast| {
        cast["targetArtifactInstanceId"] == relic_id && cast["targetLocation"]["cell"] == "C3"
    }));

    let (cast, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-detonate"
            && descriptor["targetArtifactInstanceId"] == relic_id
            && descriptor["targetLocation"]["cell"] == "C3"
    });
    assert_eq!(
        event_types(&receipt),
        ["magic-cast", "artifact-destroyed", "magic-resolved"]
    );
    let destroyed = receipt_event(&receipt, "artifact-destroyed");
    assert_eq!(destroyed.payload["cardId"], "north-relic");
    assert_eq!(destroyed.payload["instanceId"], relic_id);
    assert_eq!(destroyed.payload["owner"], "north");
    assert_eq!(
        destroyed.payload["sourceInstanceId"],
        cast["cardInstanceId"]
    );
    assert!(realm_artifact(&state(session), &relic_id).is_none());
    assert_exact_replay(session);
}
