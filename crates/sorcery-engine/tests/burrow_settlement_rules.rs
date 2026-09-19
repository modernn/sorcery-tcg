//! Direct proofs for Bury Deathrite settlement and Cave-In Deathrite order
//! (RULE-CATALOG-0691–0692, 0710, 2423–2428).
//!
//! 0655–0656 prove an ordinary minion dies after a forceful burrow or stays
//! put on Water. 0587–0588 prove Cave-In burrows Burrowing survivors at a land
//! site and skips water-only sites. 0679–0680 prove targeted Bury Artifact
//! occupancy. These slices keep the 0042/0044 leftovers: a single Deathrite
//! minion settles in the same Bury receipt, and Cave-In burrows two Deathrite
//! minions in canonical instance-id order before Deathrite ordering.
//!
//! Supplemental 2423–2428 bind cemetery persistence, empty-repeat, enemy-arrival,
//! multi-minion, far-minion, and a newly summoned Deathrite. Distinct from 0691,
//! which settles the first C1 Deathrite on the same turn, from 0692, which
//! burrows every Deathrite then pauses in deathrite-order, and from 1913–1918 /
//! 2243–2248, which kill ordinary minions without a same-receipt site draw.

use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{create_game_checkpoint, resume_game_checkpoint};
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

fn deathrite_minion() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn burrowing_deathrite_buff() -> Value {
    json!({
        "attack": 1,
        "burrowing": true,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 1,
        "manaCost": 0,
        "otherNearbyAlliesPowerBonus": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn genesis_pinger() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 10,
        "genesisDamageEachOtherUnitHere": 1,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn bury() -> Value {
    json!({
        "burrowTargetMinionOrArtifact": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn cave_in() -> Value {
    json!({
        "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
        "cardType": "magic",
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn settlement_manifest(seed: u32, cave_in_spell: bool) -> String {
    let (north_spell, north_card, fixture) = if cave_in_spell {
        ("north-cave-in", cave_in(), "burrow-settlement-cave-in")
    } else {
        ("north-bury", bury(), "burrow-settlement-bury")
    };
    let mut cards = json!({
        "north-avatar": avatar(),
        "north-site": earth_site(),
        "south-avatar": avatar(),
        "south-minion": deathrite_minion(),
        "south-site": earth_site(),
    });
    cards[north_spell] = north_card;
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": fixture }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": format!("synthetic-{fixture}-v1"),
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec![north_spell; 6],
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

fn keep(session: &mut Session) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    });
}

fn opening_main(encoded: &str) -> Session {
    let mut session = Session::new(encoded).expect("valid burrow-settlement session");
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

fn opening_spell_ids(encoded: &str, seat: &str) -> Vec<String> {
    let preview = Session::new(encoded).expect("candidate session");
    state(&preview)["players"][seat]["hand"]["spellbook"]
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

fn bury_deferral_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "bury-deferral-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-bury-deferral-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-pinger": genesis_pinger(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-buff": burrowing_deathrite_buff(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-bury",
                    "north-bury",
                    "north-pinger",
                    "north-bury",
                    "north-bury",
                    "north-pinger",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec!["south-buff"; 6],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn seed_with_deferral(start: u32) -> String {
    (start..start + 256)
        .map(bury_deferral_manifest)
        .find(|candidate| {
            let preview = Session::new(candidate).expect("ordered Bury candidate");
            let snapshot = state(&preview);
            let hand = snapshot["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand");
            hand.iter().any(|card| card["cardId"] == "north-bury")
                && hand.iter().any(|card| card["cardId"] == "north-pinger")
        })
        .expect("bounded seed with Bury and a pinger")
}

fn seed_with(cave_in_spell: bool, start: u32, required_south: usize) -> String {
    let north_spell = if cave_in_spell {
        "north-cave-in"
    } else {
        "north-bury"
    };
    (start..start + 256)
        .map(|seed| settlement_manifest(seed, cave_in_spell))
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == north_spell)
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with the north spell and required South minions")
}

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()
        .expect("realm units")
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

fn south_plays_c1_and_summons(session: &mut Session, count: usize) -> Vec<String> {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut ids = Vec::new();
    for _ in 0..count {
        let (summoned, _) = accept_where(session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "south-minion"
                && descriptor["cell"] == "C1"
                && descriptor["region"].is_null()
        });
        ids.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("Deathrite occupant identity")
                .to_owned(),
        );
    }
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    ids
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
fn rule_catalog_0691_bury_immediately_settles_a_deathrite_minion() {
    let encoded = seed_with(false, 691, 1);
    let mut session = opening_main(&encoded);
    let target_id = south_plays_c1_and_summons(&mut session, 1)
        .into_iter()
        .next()
        .expect("Bury Deathrite target");
    let before = state(&session);
    let surface = realm_unit(&before, &target_id).expect("surface Deathrite minion");
    assert_eq!(surface["location"], "C1");
    assert_eq!(surface["region"], "surface");

    let (cast, settled) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == target_id
    });
    assert_eq!(
        event_types(&settled),
        [
            "magic-cast",
            "minion-burrowed",
            "site-drawn",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(
        settled.events[1].payload,
        json!({
            "cell": "C1",
            "instanceId": target_id,
            "seat": "south",
            "sourceInstanceId": cast["cardInstanceId"],
        })
    );
    assert_eq!(
        settled.events[2].payload,
        json!({
            "seat": "south",
            "sourceInstanceId": target_id,
        })
    );
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert!(realm_unit(&after, &target_id).is_none());
    assert!(cemetery_has(&after, "south", &target_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0692_cave_in_burrows_then_orders_deathrites() {
    let encoded = seed_with(true, 692, 2);
    let mut session = opening_main(&encoded);
    let mut victim_ids = south_plays_c1_and_summons(&mut session, 2);
    let land_site_id = state(&session)["realm"]["sites"]["C1"]["instanceId"]
        .as_str()
        .expect("Land Site identity")
        .to_owned();

    let (cast, burrowed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-cave-in"
            && descriptor["targetLocation"]["cell"] == "C1"
            && descriptor["targetSiteInstanceId"] == land_site_id
    });
    victim_ids.sort();
    assert_eq!(
        event_types(&burrowed),
        ["magic-cast", "minion-burrowed", "minion-burrowed",]
    );
    assert_eq!(
        burrowed.events[0].payload["targetLocation"],
        json!({ "cell": "C1", "region": "surface" })
    );
    assert_eq!(
        burrowed.events[0].payload["targetSiteInstanceId"],
        land_site_id
    );
    assert_eq!(
        burrowed
            .events
            .iter()
            .filter(|event| event.event_type == "minion-burrowed")
            .map(|event| {
                assert_eq!(event.payload["cell"], "C1");
                assert_eq!(event.payload["seat"], "south");
                assert_eq!(event.payload["sourceInstanceId"], cast["cardInstanceId"]);
                event.payload["instanceId"]
                    .as_str()
                    .expect("burrowed identity")
                    .to_owned()
            })
            .collect::<Vec<_>>(),
        victim_ids
    );

    let pending = state(&session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert!(
        victim_ids
            .iter()
            .all(|instance_id| realm_unit(&pending, instance_id).is_none())
    );
    assert_eq!(
        pending["pendingDeathrites"]["deferredOutcomes"],
        json!([{
            "payload": {
                "cardId": "north-cave-in",
                "instanceId": cast["cardInstanceId"],
                "owner": "north",
            },
            "type": "magic-resolved",
        }])
    );

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
    assert_eq!(order_sources, victim_ids);

    let (_, ordered) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == victim_ids[0]
    });
    assert_eq!(
        event_types(&ordered),
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
            "minion-died",
            "magic-resolved",
        ]
    );
    let completed = state(&session);
    assert_eq!(completed["phase"], "main");
    assert!(
        victim_ids
            .iter()
            .all(|instance_id| cemetery_has(&completed, "south", instance_id))
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0710_bury_defers_until_ordered_static_deathrites_finish() {
    let encoded = seed_with_deferral(710);
    let mut session = opening_main(&encoded);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let mut buff_ids = Vec::new();
    for _ in 0..2 {
        let (summoned, _) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["region"].is_null()
                && descriptor["cardId"] == "south-buff"
                && descriptor["cell"] == "C1"
        });
        buff_ids.push(
            summoned["cardInstanceId"]
                .as_str()
                .expect("buff identity")
                .to_owned(),
        );
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["region"].is_null()
            && descriptor["cardId"] == "north-pinger"
            && descriptor["cell"] == "C1"
    });
    assert!(buff_ids.iter().all(|instance_id| {
        realm_unit(&state(&session), instance_id).is_some_and(|unit| unit["damage"] == 1)
    }));
    let spell_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-bury")
        .expect("Bury in hand")["instanceId"]
        .as_str()
        .expect("Bury identity")
        .to_owned();

    let (_, cast) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["target"]["instanceId"] == buff_ids[0]
    });
    assert_eq!(event_types(&cast), ["magic-cast", "minion-burrowed"]);
    let pending = state(&session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(pending["decisionSeat"], "south");
    assert_eq!(
        pending["pendingDeathrites"]["deferredOutcomes"],
        json!([{
            "payload": {
                "cardId": "north-bury",
                "instanceId": spell_id,
                "owner": "north",
            },
            "type": "magic-resolved",
        }])
    );

    let checkpoint = create_game_checkpoint(&session).expect("ordered Bury checkpoint");
    let restored = resume_game_checkpoint(&checkpoint).expect("restored ordered Bury checkpoint");
    assert_eq!(
        restored.replay_value().expect("restored pending state"),
        session.replay_value().expect("source pending state")
    );
    let (_, ordered) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == buff_ids[0]
    });
    assert_eq!(
        event_types(&ordered),
        [
            "deathrite-order-committed",
            "site-drawn",
            "site-drawn",
            "minion-died",
            "minion-died",
            "magic-resolved",
        ]
    );
    assert_eq!(state(&session)["phase"], "main");
    assert_exact_replay(&session);
}

fn roaming_deathrite() -> Value {
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

fn settlement_supplemental_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "burrow-settlement-deathrite-supplemental" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-burrow-settlement-deathrite-supplemental-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-bury": bury(),
            "north-site": earth_site(),
            "south-avatar": avatar(),
            "south-minion": roaming_deathrite(),
            "south-site": earth_site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 24],
                "avatar": "north-avatar",
                "spellbook": vec!["north-bury"; 8],
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

fn seed_with_start(start: u32, required_south: usize) -> String {
    (start..start + 2048)
        .chain(691..691 + 2048)
        .map(settlement_supplemental_manifest)
        .find(|candidate| {
            opening_spell_ids(candidate, "north")
                .iter()
                .any(|card| card == "north-bury")
                && opening_spell_ids(candidate, "south")
                    .iter()
                    .filter(|card| *card == "south-minion")
                    .count()
                    >= required_south
        })
        .expect("bounded seed with Bury and required South Deathrite minions")
}

fn bury_spells_in_hand(snapshot: &Value) -> usize {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .map(|hand| {
            hand.iter()
                .filter(|card| card["cardId"] == "north-bury")
                .count()
        })
        .unwrap_or_default()
}

fn bury_targets(session: &Session) -> Vec<String> {
    let mut targets: Vec<_> = session
        .legal_actions()
        .expect("bury actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic" && action.descriptor["cardId"] == "north-bury"
        })
        .filter_map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .map(ToOwned::to_owned)
        })
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn offers(session: &Session, predicate: impl Fn(&Value) -> bool) -> bool {
    session
        .legal_actions()
        .ok()
        .is_some_and(|actions| actions.iter().any(|action| predicate(&action.descriptor)))
}

fn bury_settle_events() -> [&'static str; 5] {
    [
        "magic-cast",
        "minion-burrowed",
        "site-drawn",
        "minion-died",
        "magic-resolved",
    ]
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

fn south_plays_c1(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
}

fn north_draws_spellbook(session: &mut Session) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
}

fn summon_south_at(session: &mut Session, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite occupant identity")
        .to_owned()
}

fn setup_c1_with_minions(session: &mut Session, count: usize) -> Vec<String> {
    south_plays_c1(session);
    (0..count).map(|_| summon_south_at(session, "C1")).collect()
}

fn cast_bury_target(session: &mut Session, target_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "north-bury"
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn try_far_minion_prefix(encoded: &str) -> Option<(Session, Vec<String>, String)> {
    let mut session = opening_main(encoded);
    let c1_ids = setup_c1_with_minions(&mut session, 2);
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let far_id = summon_south_at(&mut session, "C4");
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    (!bury_targets(&session).is_empty()).then_some((session, c1_ids, far_id))
}

fn seed_for_far_minion(start: u32) -> String {
    (start..start + 2048)
        .chain(691..691 + 2048)
        .find_map(|seed| {
            let encoded = settlement_supplemental_manifest(seed);
            if opening_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 3
            {
                return None;
            }
            try_far_minion_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching Bury Deathrite far-minion setup")
}

fn try_second_bury_enemy_arrival_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_bury_target(&mut session, &first_id);
    if event_types(&first) != bury_settle_events() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if bury_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "atlas" || descriptor["zone"] == "spellbook")
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let minion_id = summon_south_at(&mut session, "C2");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    bury_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_bury_enemy_arrival(start: u32) -> String {
    (start..start + 8192)
        .chain(691..691 + 8192)
        .find_map(|seed| {
            let encoded = settlement_supplemental_manifest(seed);
            if opening_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_bury_enemy_arrival_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Bury Deathrite enemy-arrival setup")
}

fn try_second_bury_new_summon_prefix(encoded: &str) -> Option<(Session, String)> {
    let mut session = opening_main(encoded);
    let first_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let first = cast_bury_target(&mut session, &first_id);
    if event_types(&first) != bury_settle_events() {
        return None;
    }
    pass_turn_to_north_spellbook(&mut session);
    if bury_spells_in_hand(&state(&session)) < 1 {
        return None;
    }
    end_turn_if_offered(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw"
            && (descriptor["zone"] == "spellbook" || descriptor["zone"] == "atlas")
    })?;
    let minion_id = summon_south_at(&mut session, "C1");
    end_turn_if_offered(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    bury_targets(&session)
        .contains(&minion_id)
        .then_some((session, minion_id))
}

fn seed_for_second_bury_new_summon(start: u32) -> String {
    (start..start + 8192)
        .chain(691..691 + 8192)
        .find_map(|seed| {
            let encoded = settlement_supplemental_manifest(seed);
            if opening_spell_ids(&encoded, "south")
                .iter()
                .filter(|card| *card == "south-minion")
                .count()
                < 2
            {
                return None;
            }
            try_second_bury_new_summon_prefix(&encoded).map(|_| encoded)
        })
        .expect("bounded seed reaching second Bury Deathrite new-summon setup")
}

#[test]
fn rule_catalog_2423_settled_deathrite_minion_stays_in_cemetery_after_turns_pass() {
    let encoded = seed_with_start(2423, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let receipt = cast_bury_target(&mut session, &target_id);
    assert_eq!(event_types(&receipt), bury_settle_events());
    assert_eq!(state(&session)["phase"], "main");
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &target_id));
    pass_turn_to_north_spellbook(&mut session);
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &target_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2424_second_bury_without_a_surface_target_stays_unoffered() {
    let encoded = seed_with_start(2424, 1);
    let mut session = opening_main(&encoded);
    let target_id = setup_c1_with_minions(&mut session, 1)[0].clone();
    north_draws_spellbook(&mut session);
    let receipt = cast_bury_target(&mut session, &target_id);
    assert_eq!(event_types(&receipt), bury_settle_events());
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert!(bury_spells_in_hand(&state(&session)) >= 1);
    assert!(bury_targets(&session).is_empty());
    assert!(!offers(&session, |descriptor| {
        descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-bury"
    }));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2425_second_bury_settles_a_newly_arrived_deathrite_after_enemy_site_placement() {
    let encoded = seed_for_second_bury_enemy_arrival(2425);
    let (mut session, minion_id) = try_second_bury_enemy_arrival_prefix(&encoded)
        .expect("second Bury Deathrite enemy-arrival prefix");
    let receipt = cast_bury_target(&mut session, &minion_id);
    assert_eq!(event_types(&receipt), bury_settle_events());
    assert_eq!(state(&session)["phase"], "main");
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2426_bury_offers_every_surface_deathrite_minion_at_the_target_land_site() {
    let encoded = seed_with_start(2426, 2);
    let mut session = opening_main(&encoded);
    let minion_ids = setup_c1_with_minions(&mut session, 2);
    north_draws_spellbook(&mut session);
    let offered = bury_targets(&session);
    assert_eq!(offered.len(), 2);
    for minion_id in &minion_ids {
        assert!(offered.contains(minion_id));
    }
    assert_eq!(state(&session)["phase"], "main");
    assert!(state(&session)["pendingDeathrites"].is_null());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2427_bury_leaves_a_far_deathrite_minion_untouched() {
    let encoded = seed_for_far_minion(2427);
    let (mut session, c1_ids, far_id) =
        try_far_minion_prefix(&encoded).expect("Bury Deathrite far-minion prefix");
    let buried_id = &c1_ids[0];
    let receipt = cast_bury_target(&mut session, buried_id);
    assert_eq!(event_types(&receipt), bury_settle_events());
    assert!(realm_unit(&state(&session), buried_id).is_none());
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("far Deathrite minion")["location"],
        "C4"
    );
    assert_eq!(
        realm_unit(&state(&session), &far_id).expect("far Deathrite minion")["region"],
        "surface"
    );
    assert_eq!(state(&session)["phase"], "main");
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_2428_second_bury_settles_a_newly_summoned_deathrite_minion() {
    let encoded = seed_for_second_bury_new_summon(2428);
    let (mut session, minion_id) = try_second_bury_new_summon_prefix(&encoded)
        .expect("second Bury Deathrite new-summon prefix");
    let receipt = cast_bury_target(&mut session, &minion_id);
    assert_eq!(event_types(&receipt), bury_settle_events());
    assert_eq!(state(&session)["phase"], "main");
    assert!(realm_unit(&state(&session), &minion_id).is_none());
    assert!(cemetery_has(&state(&session), "south", &minion_id));
    assert_exact_replay(&session);
}
