//! Direct proof that a tapped area-damage minion blankets one adjacent location with its own
//! power and its carried Lethal, without becoming a strike (RULE-CATALOG-0076).

use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};

const SEED: u32 = 7;

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 1,
        "thresholds": { "air": 0, "earth": 0, "fire": 1, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn cards() -> Value {
    json!({
        "poisonous-dagger": {
            "cardType": "artifact",
            "grantsBearerLethal": true,
            "manaCost": 2,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "vikings": minion(json!({
            "attack": 4,
            "defense": 4,
            "manaCost": 5,
            "stealth": true,
            "tapToDamageEachUnitAtAdjacentLocation": 2,
            "thresholds": { "air": 0, "earth": 0, "fire": 2, "water": 0 },
        })),
        "vikings-ally": minion(json!({ "summonToAnySite": true })),
        "vikings-avatar": {
            "attack": 1,
            "cardType": "avatar",
            "defense": 1,
            "drawSpell": false,
            "life": 20,
        },
        "vikings-north-site": { "cardType": "site", "elements": ["fire"], "genesisGainMana": 10 },
        "vikings-south-site": {
            "cardType": "site",
            "elements": ["fire", "water"],
            "genesisGainMana": 10,
        },
        "vikings-stealthed-enemy": minion(json!({ "defense": 3, "stealth": true })),
        "vikings-submerged-enemy": minion(json!({
            "submerge": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 1 },
        })),
        "vikings-warded-enemy": minion(json!({ "ward": true })),
        // A Waterbound copy of the same ability proves a disabled source offers no activation.
        "waterbound-vikings": minion(json!({
            "attack": 4,
            "defense": 4,
            "tapToDamageEachUnitAtAdjacentLocation": 2,
            "waterbound": true,
        })),
        "whelm": {
            "cardType": "magic",
            "manaCost": 1,
            "submergeTargetMinion": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 1 },
        },
    })
}

/// Both spellbooks hold exactly the cards their seat casts, so the opening hand plus the first
/// draw is the whole spellbook and the scenario does not depend on the shuffle.
fn manifest() -> String {
    let mut value = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "area-damage-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-area-damage-v1",
        },
        "cards": cards(),
        "decks": {
            "north": {
                "atlas": vec!["vikings-north-site"; 6],
                "avatar": "vikings-avatar",
                "spellbook": [
                    "vikings",
                    "poisonous-dagger",
                    "waterbound-vikings",
                    "vikings-ally",
                ],
            },
            "south": {
                "atlas": vec!["vikings-south-site"; 6],
                "avatar": "vikings-avatar",
                "spellbook": [
                    "vikings-warded-enemy",
                    "vikings-stealthed-enemy",
                    "vikings-submerged-enemy",
                    "whelm",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": SEED,
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

fn end_and_draw(session: &mut Session, zone: &str) {
    accept_where(session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == zone
    });
}

fn play_site(session: &mut Session, cell: &str) {
    accept_where(session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == cell
    });
}

fn summon(session: &mut Session, card_id: &str, cell: &str) -> String {
    let (summoned, _) = accept_where(session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == card_id
            && descriptor["cell"] == cell
            && descriptor["region"].is_null()
    });
    summoned["cardInstanceId"]
        .as_str()
        .expect("summoned identity")
        .to_owned()
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("authoritative replay")["state"].clone()
}

fn realm_unit(current: &Value, instance_id: &str) -> Option<Value> {
    current["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
        .cloned()
}

fn area_damage_descriptors(session: &Session, source_instance_id: &str) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .map(|action| action.descriptor)
        .filter(|descriptor| {
            descriptor["kind"] == "activate-area-damage"
                && descriptor["sourceInstanceId"] == source_instance_id
        })
        .collect()
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
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
    assert!(session.verify_replay().expect("verified replay"));
}

/// Every identity the blanket at C1 either reaches or deliberately leaves alone.
struct Blanket {
    ally: String,
    disabled_source: String,
    source: String,
    stealthed: String,
    submerged: String,
    warded: String,
}

/// Walks both seats up to North's ready area-damage minion at C2, one step from South's Avatar and
/// its three minions at C1. The dagger is conjured either onto that minion or loose beside it,
/// which is the only difference between the two outcomes.
fn blanket_position(session: &mut Session, carried: bool) -> Blanket {
    keep(session);
    keep(session);

    play_site(session, "C4");
    end_and_draw(session, "spellbook");

    play_site(session, "C1");
    let warded = summon(session, "vikings-warded-enemy", "C1");
    let stealthed = summon(session, "vikings-stealthed-enemy", "C1");
    let submerged = summon(session, "vikings-submerged-enemy", "C1");
    // South pulls its own minion under the Water site so only one layer of C1 is exposed.
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardId"] == "whelm"
            && descriptor["target"]["instanceId"] == submerged.as_str()
    });
    assert_eq!(
        realm_unit(&state(session), &submerged).expect("submerged enemy")["region"],
        "underwater"
    );
    end_and_draw(session, "spellbook");

    play_site(session, "C3");
    let ally = summon(session, "vikings-ally", "C1");
    end_and_draw(session, "atlas");
    end_and_draw(session, "atlas");

    play_site(session, "C2");
    let source = summon(session, "vikings", "C2");
    accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-artifact"
            && descriptor["cardId"] == "poisonous-dagger"
            && if carried {
                descriptor["bearer"]["instanceId"] == source.as_str()
            } else {
                descriptor["bearer"].is_null() && descriptor["cell"] == "C2"
            }
    });
    let disabled_source = summon(session, "waterbound-vikings", "C2");
    // A summoning-sick source cannot tap for its ability yet.
    assert!(area_damage_descriptors(session, &source).is_empty());
    end_and_draw(session, "atlas");
    end_and_draw(session, "atlas");

    Blanket {
        ally,
        disabled_source,
        source,
        stealthed,
        submerged,
        warded,
    }
}

#[test]
fn rule_catalog_0076_area_damage_should_use_bearer_lethal_without_becoming_a_strike() {
    let mut session = Session::new(&manifest()).expect("valid area damage scenario");
    let blanket = blanket_position(&mut session, true);

    // The ability reaches only the existing locations bordering C2, in the source's own region.
    let offered = area_damage_descriptors(&session, &blanket.source);
    assert_eq!(
        offered
            .iter()
            .map(|descriptor| descriptor["targetLocation"].clone())
            .collect::<Vec<_>>(),
        [
            json!({ "cell": "C1", "region": "surface" }),
            json!({ "cell": "C3", "region": "surface" }),
        ]
    );
    // The Waterbound copy of the same ability stands on a Fire site, so it offers nothing.
    assert!(
        area_damage_descriptors(&session, &blanket.disabled_source).is_empty(),
        "a disabled source must not offer its area damage"
    );

    let (_, blanketed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-area-damage"
            && descriptor["sourceInstanceId"] == blanket.source.as_str()
            && descriptor["targetLocation"]["cell"] == "C1"
    });
    let types = event_types(&blanketed);
    assert_eq!(
        types[..2],
        ["area-damage-activated", "stealth-lost"],
        "tapping for the ability reveals the source before anything is allocated"
    );
    assert_eq!(types[2..6], ["area-damage-allocated"; 4]);
    assert!(
        !types.iter().any(|event_type| event_type.contains("strike")),
        "area damage is not a strike, so nothing may strike back: {types:?}"
    );
    let allocated: Vec<_> = blanketed
        .events
        .iter()
        .filter(|event| event.event_type == "area-damage-allocated")
        .map(|event| {
            (
                event.payload["amount"].clone(),
                event.payload["sourceInstanceId"].clone(),
                event.payload["targetInstanceId"].clone(),
            )
        })
        .collect();
    let mut targeted: Vec<_> = allocated
        .iter()
        .map(|(amount, source_instance_id, target_instance_id)| {
            assert_eq!(*amount, json!(2));
            assert_eq!(*source_instance_id, json!(blanket.source));
            target_instance_id.clone()
        })
        .collect();
    targeted.sort_unstable_by_key(ToString::to_string);
    let south_avatar = state(&session)["players"]["south"]["avatar"]["card"]["instanceId"].clone();
    let mut expected = vec![
        json!(blanket.ally),
        json!(blanket.stealthed),
        json!(blanket.warded),
        south_avatar.clone(),
    ];
    expected.sort_unstable_by_key(ToString::to_string);
    assert_eq!(targeted, expected);

    // Two damage against three defense only kills through the dagger the source carries.
    assert_eq!(
        blanketed
            .events
            .iter()
            .filter(|event| event.event_type == "minion-died")
            .map(|event| event.payload["instanceId"].clone())
            .collect::<Vec<_>>()
            .len(),
        2
    );
    assert!(blanketed.random_draws.is_empty());

    let settled = state(&session);
    assert!(realm_unit(&settled, &blanket.stealthed).is_none());
    assert!(realm_unit(&settled, &blanket.ally).is_none());
    // Ward absorbs the whole blanket and breaks; Lethal cannot finish a warded minion.
    let warded = realm_unit(&settled, &blanket.warded).expect("warded enemy");
    assert_eq!(
        (&warded["damage"], &warded["warded"]),
        (&json!(0), &json!(false))
    );
    // The Water layer of the same cell is a different location, so the submerged enemy is missed.
    let submerged = realm_unit(&settled, &blanket.submerged).expect("submerged enemy");
    assert_eq!(
        (&submerged["damage"], &submerged["region"]),
        (&json!(0), &json!("underwater"))
    );
    // An Avatar loses life to the blanket and is never finished by Lethal.
    assert_eq!(settled["players"]["south"]["avatar"]["life"], 18);
    assert_eq!(
        realm_unit(&settled, &blanket.source).expect("source")["tapped"],
        true
    );
    assert!(
        area_damage_descriptors(&session, &blanket.source).is_empty(),
        "a tapped source cannot blanket a second location"
    );
    assert_exact_replay(&session);
}

#[test]
fn a_loose_lethal_artifact_should_not_lend_its_lethal_to_area_damage() {
    let mut session = Session::new(&manifest()).expect("valid area damage scenario");
    let blanket = blanket_position(&mut session, false);

    let (_, blanketed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "activate-area-damage"
            && descriptor["sourceInstanceId"] == blanket.source.as_str()
            && descriptor["targetLocation"]["cell"] == "C1"
    });
    assert_eq!(
        blanketed
            .events
            .iter()
            .filter(|event| event.event_type == "minion-died")
            .map(|event| event.payload["instanceId"].clone())
            .collect::<Vec<_>>(),
        vec![json!(blanket.ally)]
    );

    // The same two damage only wounds the three-defense enemy while the dagger lies loose on C2.
    let settled = state(&session);
    assert_eq!(
        realm_unit(&settled, &blanket.stealthed).expect("surviving enemy")["damage"],
        2
    );
    assert_eq!(settled["players"]["south"]["avatar"]["life"], 18);
    assert_exact_replay(&session);
}
