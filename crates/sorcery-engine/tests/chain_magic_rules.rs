//! Direct proofs for 1×1 Chain Magic hops (RULE-CATALOG-0030, 0696, 0709,
//! 0885–0890, 0893–0894, 0896, 0903–0904, 0913–0914, 0923–0924, 0933–0934,
//! 0943–0944, 0952, 0955, 0970, 0981, 0984–0985).
//!
//! 0385–0386 already cover oversized Spellcaster footprint hops. 0696 keeps
//! the 0030 leftover: a 1×1 caster stages distinct nearby hops, then damages
//! every chosen unit in one resolve. 0709 is the edge slice: paid Chain Magic
//! is suppressed without enough mana, and hops cannot leave the caster region.
//! 0885–0886 cover pay-life additional costs on Chain Magic resolution.
//! 0887–0890 cover chosen-discard additional costs on Chain Magic.
//! 0894 covers chosen-discard resolve gating when the staged discard leaves hand.
//! 0893 covers staged mana gates on resolve-chain-magic and extend-chain-magic.
//! 0904 covers extend withheld for the next hop while resolve stays legal at the
//! current staged count when mana covers resolve but not extend.
//! 0913 covers resolve withheld after a legal extend when staged target count
//! raises total mana cost above the pool while resolve was legal at one target.
//! 0896 covers pay-life resolve gating when life drops after begin.
//! 0903 covers checkpoint resume preserving staged targets, discard choice,
//! and legal resolve/extend actions mid-staged Chain Magic.
//! 0914 covers Chain Magic phase routing: staged chains offer only
//! extend-chain-magic and resolve-chain-magic from `append_chain_magic_actions`,
//! not main-phase cast-magic, end-turn, or move-and-attack.
//! 0924 covers resolve-chain-magic emitting magic-damage-allocated for every
//! staged target before any minion-died in the same receipt.
//! 0923 covers pending.targets dedup: extend-chain-magic never re-lists an
//! already-staged hop. Distinct from 0696 resolve flow, 0903 checkpoint resume,
//! 0913 post-extend mana gating, and 0914 phase routing.
//! 0943 covers extend/resolve withheld when a staged target minion leaves the
//! Realm, including after checkpoint resume.
//! 0933 covers extend/resolve withheld when the staged caster is no longer a
//! legal Spellcaster (checkpoint branch after the printed Spellcaster leaves).
//! 0934 covers extend-chain-magic preserving the staged discardCardInstanceId
//! from begin through resolve when a second hop is added. Distinct from 0889
//! single-hop atlas discard resolve and 0903 checkpoint resume.
//! 0944 covers extend-chain-magic omitting nearby enemy minions with active
//! Stealth while visible nearby enemies remain eligible.
//! 0952 covers begin-chain-magic withheld for a printed Spellcaster with zero
//! legal first hops on an otherwise empty nearby board area.
//! 0955 covers begin-chain-magic targeting a nearby enemy Avatar as the first
//! hop from a printed Spellcaster caster, then resolving avatar life loss.
//! 0970 covers begin-chain-magic omitting a distant enemy Avatar as the first
//! hop while a nearby enemy minion remains eligible.
//! 0981 covers extend-chain-magic targeting a nearby enemy Avatar as the second
//! hop after begin-chain-magic stages a nearby enemy minion, then resolving
//! avatar life loss.
//! 0984 covers resolve-chain-magic killing a Deathrite minion: the controller
//! draws a site and magic-resolved only appears after deathrite settlement.
//! 0985 covers extend-chain-magic omitting a distant enemy Avatar as the second
//! hop after begin-chain-magic stages a nearby enemy minion.

use serde_json::{Value, json};
use sorcery_engine::action::ActionDescriptor;
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::game::Game;
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

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 2,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn chain(mana_cost: u8) -> Value {
    json!({
        "cardType": "magic",
        "damageChainNearbyUnits": true,
        "manaCost": mana_cost,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn avatar_with_life(life: u8) -> Value {
    json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": life,
    })
}

fn pay_life_chain(life_cost: u8) -> Value {
    json!({
        "cardType": "magic",
        "damageChainNearbyUnits": true,
        "manaCost": 0,
        "payLifeAsAdditionalCost": life_cost,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn discard_chain() -> Value {
    json!({
        "cardType": "magic",
        "damageChainNearbyUnits": true,
        "discardCardAsAdditionalCost": true,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn fodder() -> Value {
    json!({
        "cardType": "magic",
        "healController": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn discard_chain_manifest(seed: u32, north_spellbook: &[&str]) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-discard" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-discard-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-chain": discard_chain(),
            "north-fodder": fodder(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
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

fn pay_life_chain_manifest(seed: u32, life: u8) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-pay-life" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-pay-life-v1",
        },
        "cards": {
            "north-avatar": avatar_with_life(life),
            "north-chain": pay_life_chain(2),
            "north-site": site(),
            "south-avatar": avatar_with_life(20),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-chain"; 6],
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

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn discard_hops_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-discard-hops" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-discard-hops-v1",
        },
        "cards": {
            "north-ally-a": minion(json!({})),
            "north-ally-b": minion(json!({})),
            "north-avatar": avatar(),
            "north-chain": discard_chain(),
            "north-fodder": fodder(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-ally-a",
                    "north-ally-b",
                    "north-fodder",
                    "north-chain",
                    "north-fodder",
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

fn isolated_spellcaster_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-isolated-spellcaster" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-isolated-spellcaster-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "burrowing": true, "spellcaster": true })),
            "north-chain": chain(0),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-caster",
                    "north-chain",
                    "north-caster",
                    "north-chain",
                    "north-caster",
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

fn spellcaster_hops_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-spellcaster-hops" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-spellcaster-hops-v1",
        },
        "cards": {
            "north-ally-a": minion(json!({})),
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "spellcaster": true })),
            "north-chain": chain(0),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-caster",
                    "north-ally-a",
                    "north-ally-a",
                    "north-chain",
                    "north-ally-a",
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

fn spellcaster_avatar_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-spellcaster-avatar" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-spellcaster-avatar-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-caster": minion(json!({ "spellcaster": true })),
            "north-chain": chain(0),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-caster",
                    "north-chain",
                    "north-caster",
                    "north-chain",
                    "north-caster",
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

fn stealth_hops_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-stealth-hops" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-stealth-hops-v1",
        },
        "cards": {
            "north-ally-a": minion(json!({})),
            "north-avatar": avatar(),
            "north-chain": chain(0),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-site": site(),
            "south-stealth": minion(json!({ "stealth": true, "summonToAnySite": true })),
            "south-visible": minion(json!({ "summonToAnySite": true })),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-ally-a",
                    "north-chain",
                    "north-ally-a",
                    "north-chain",
                    "north-ally-a",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": vec![
                    "south-visible",
                    "south-stealth",
                    "south-visible",
                    "south-visible",
                    "south-stealth",
                    "south-stealth",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": seed,
    }))
}

fn deathrite() -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "deathriteDrawSite": true,
        "defense": 2,
        "manaCost": 0,
        "summonToAnySite": true,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    })
}

fn deathrite_chain_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-deathrite" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-deathrite-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-chain": chain(0),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-deathrite": deathrite(),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": vec!["north-chain"; 6],
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

fn hops_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-hops" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-hops-v1",
        },
        "cards": {
            "north-ally-a": minion(json!({})),
            "north-ally-b": minion(json!({})),
            "north-avatar": avatar(),
            "north-chain": chain(0),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-ally-a",
                    "north-ally-b",
                    "north-chain",
                    "north-ally-a",
                    "north-ally-b",
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

fn filter_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-filter" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-filter-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-burrower": minion(json!({ "burrowing": true })),
            "north-chain": chain(2),
            "north-free-chain": chain(0),
            "north-site": site(),
            "north-target": minion(json!({})),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-free-chain",
                    "north-target",
                    "north-burrower",
                    "north-chain",
                    "north-free-chain",
                    "north-target",
                    "north-burrower",
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
    let mut session = Session::new(encoded).expect("valid Chain Magic session");
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

fn realm_unit<'a>(snapshot: &'a Value, instance_id: &str) -> Option<&'a Value> {
    snapshot["realm"]["units"]
        .as_array()?
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

fn opening_has_all(encoded: &str, card_ids: &[&str]) -> bool {
    Session::new(encoded).ok().is_some_and(|preview| {
        state(&preview)["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .is_some_and(|hand| {
                card_ids
                    .iter()
                    .all(|card_id| hand.iter().any(|card| card["cardId"] == *card_id))
            })
    })
}

fn hand_instance(snapshot: &Value, card_id: &str) -> String {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == card_id)
        .expect("card in hand")["instanceId"]
        .as_str()
        .expect("card identity")
        .to_owned()
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

fn chain_ids(session: &Session, card_instance_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("Chain Magic actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardInstanceId"] == card_instance_id
        })
        .map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .expect("start target identity")
                .to_owned()
        })
        .collect()
}

fn extend_ids(session: &Session) -> Vec<String> {
    session
        .legal_actions()
        .expect("staged Chain Magic actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "extend-chain-magic")
        .map(|action| {
            action.descriptor["target"]["instanceId"]
                .as_str()
                .expect("extension target identity")
                .to_owned()
        })
        .collect()
}

fn offers_resolve_chain_magic(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("staged Chain Magic actions")
        .iter()
        .any(|action| action.descriptor["kind"] == "resolve-chain-magic")
}

const CHAIN_MAGIC_PHASE_ACTION_KINDS: &[&str] = &[
    "begin-chain-magic",
    "extend-chain-magic",
    "resolve-chain-magic",
];

const MAIN_ACTIONS_FORBIDDEN_WHILE_CHAIN_STAGED: &[&str] =
    &["cast-magic", "end-turn", "move-and-attack"];

fn assert_staged_chain_magic_legal_actions_only(session: &Session) {
    let snapshot = state(session);
    assert_eq!(snapshot["phase"], "chain-magic");
    assert!(snapshot["pendingChainMagic"].is_object());
    let legal = session
        .legal_actions()
        .expect("staged Chain Magic legal_actions");
    assert!(
        !legal.is_empty(),
        "append_chain_magic_actions must issue resolve-chain-magic or extend-chain-magic"
    );
    assert!(
        legal.iter().all(|action| {
            CHAIN_MAGIC_PHASE_ACTION_KINDS
                .contains(&action.descriptor["kind"].as_str().unwrap_or(""))
        }),
        "Phase::ChainMagic legal_actions must only enumerate chain-magic actions"
    );
    assert!(
        !legal.iter().any(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                || MAIN_ACTIONS_FORBIDDEN_WHILE_CHAIN_STAGED
                    .contains(&action.descriptor["kind"].as_str().unwrap_or(""))
        }),
        "staged Chain Magic must issue no cast-magic, end-turn, move-and-attack, or begin-chain-magic"
    );
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort_unstable();
    ids
}

fn atlas_len(snapshot: &Value, seat: &str) -> usize {
    snapshot["players"][seat]["atlas"]
        .as_array()
        .expect("atlas")
        .len()
}

fn cemetery_has(snapshot: &Value, seat: &str, instance_id: &str) -> bool {
    snapshot["players"][seat]["cemetery"]
        .as_array()
        .expect("cemetery")
        .iter()
        .any(|card| card["instanceId"] == instance_id)
}

struct ChainHops {
    avatar_id: String,
    chain_id: String,
    first_id: String,
    mana: u64,
    second_id: String,
    session: Session,
}

fn setup_hops(encoded: &str) -> ChainHops {
    let mut session = opening_main(encoded);
    let (first, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally-a"
            && descriptor["cell"] == "C4"
    });
    let (second, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally-b"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    let before = state(&session);
    ChainHops {
        avatar_id: before["players"]["north"]["avatar"]["card"]["instanceId"]
            .as_str()
            .expect("North Avatar identity")
            .to_owned(),
        chain_id: hand_instance(&before, "north-chain"),
        first_id: first["cardInstanceId"]
            .as_str()
            .expect("first hop identity")
            .to_owned(),
        mana: before["players"]["north"]["mana"]
            .as_u64()
            .expect("North mana"),
        second_id: second["cardInstanceId"]
            .as_str()
            .expect("second hop identity")
            .to_owned(),
        session,
    }
}

#[test]
fn rule_catalog_0696_chain_magic_stages_distinct_nearby_hops_and_resolves_simultaneously() {
    let encoded = (696..696 + 256)
        .map(hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
        })
        .expect("bounded seed with Chain Magic and both nearby allies in the opening hand");
    let mut hops = setup_hops(&encoded);
    assert_eq!(
        sorted(chain_ids(&hops.session, &hops.chain_id)),
        sorted(vec![
            hops.avatar_id.clone(),
            hops.first_id.clone(),
            hops.second_id.clone(),
        ])
    );

    let (_, begin) = accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    assert!(begin.events.is_empty());
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(staged["players"]["north"]["mana"], hops.mana);
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": hops.first_id,
            "kind": "minion",
            "seat": "north",
        }])
    );
    assert_eq!(
        sorted(extend_ids(&hops.session)),
        sorted(vec![hops.avatar_id.clone(), hops.second_id.clone()])
    );
    assert!(!extend_ids(&hops.session).contains(&hops.first_id));

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["instanceId"] == hops.second_id
    });
    let (_, resolved) = accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    let damaged: Vec<_> = resolved
        .events
        .iter()
        .filter(|event| event.event_type == "magic-damage-allocated")
        .map(|event| event.payload.clone())
        .collect();
    assert_eq!(
        damaged,
        [&hops.first_id, &hops.second_id]
            .into_iter()
            .map(|target_instance_id| json!({
                "amount": 2,
                "sourceInstanceId": hops.chain_id,
                "targetInstanceId": target_instance_id,
            }))
            .collect::<Vec<_>>()
    );
    let first_death = event_types(&resolved)
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("first chained death");
    assert!(
        resolved
            .events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.event_type == "damage-dealt")
            .all(|(index, _)| index < first_death)
    );
    assert_eq!(event_types(&resolved).last(), Some(&"magic-resolved"));
    let after = state(&hops.session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingChainMagic"].is_null());
    assert_eq!(after["players"]["north"]["mana"], hops.mana - 2);
    assert!(realm_unit(&after, &hops.first_id).is_none());
    assert!(realm_unit(&after, &hops.second_id).is_none());
    assert_exact_replay(&hops.session);
}

#[test]
fn rule_catalog_0924_resolve_chain_magic_allocates_all_targets_before_any_minion_died() {
    let encoded = (924..924 + 256)
        .map(hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
        })
        .expect("bounded seed with Chain Magic and both nearby allies in the opening hand");
    let mut hops = setup_hops(&encoded);
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["instanceId"] == hops.second_id
    });
    let (_, resolved) = accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    let first_death = event_types(&resolved)
        .iter()
        .position(|event_type| *event_type == "minion-died")
        .expect("at least one staged target must die");
    let allocations: Vec<_> = resolved
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.event_type == "magic-damage-allocated")
        .collect();
    assert_eq!(
        allocations.len(),
        2,
        "each staged hop gets one magic-damage-allocated"
    );
    assert!(
        allocations.iter().all(|(index, _)| *index < first_death),
        "every magic-damage-allocated must precede the first minion-died"
    );
    assert_eq!(
        sorted(
            allocations
                .iter()
                .map(|(_, event)| {
                    event.payload["targetInstanceId"]
                        .as_str()
                        .expect("allocation target")
                        .to_owned()
                })
                .collect()
        ),
        sorted(vec![hops.first_id.clone(), hops.second_id.clone()])
    );
    for (_, event) in &allocations {
        assert_eq!(event.payload["amount"], 2);
        assert_eq!(event.payload["sourceInstanceId"], hops.chain_id);
    }
    assert_exact_replay(&hops.session);
}

fn try_hand_instance(snapshot: &Value, card_id: &str) -> Option<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == card_id)?["instanceId"]
        .as_str()
        .map(str::to_owned)
}

fn try_setup_filter(encoded: &str) -> Option<(Session, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let opening = state(&session);
    let paid_id = try_hand_instance(&opening, "north-chain")?;
    if opening["players"]["north"]["mana"].as_u64() != Some(1)
        || !chain_ids(&session, &paid_id).is_empty()
    {
        return None;
    }
    let target = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-target"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let burrower = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-burrower"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    })?;
    let snapshot = state(&session);
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()?
        .iter()
        .find(|card| card["cardId"] == "north-free-chain")?;
    Some((
        session,
        target.0["cardInstanceId"].as_str()?.to_owned(),
        burrower.0["cardInstanceId"].as_str()?.to_owned(),
    ))
}

#[test]
fn rule_catalog_0709_chain_magic_requires_mana_and_same_region_hops() {
    let encoded = (1696..1696 + 512)
        .map(filter_manifest)
        .find(|candidate| try_setup_filter(candidate).is_some())
        .expect("bounded seed with paid Chain Magic, free hops, and an underground burrower");
    let preview = opening_main(&encoded);
    let paid_id = hand_instance(&state(&preview), "north-chain");
    assert_eq!(state(&preview)["players"]["north"]["mana"], 1);
    assert!(chain_ids(&preview, &paid_id).is_empty());

    let (mut session, target_id, burrower_id) =
        try_setup_filter(&encoded).expect("complete Chain Magic filter setup");
    let avatar_id = state(&session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let free_id = hand_instance(&state(&session), "north-free-chain");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == free_id
            && descriptor["target"]["instanceId"] == target_id
    });
    let extensions = extend_ids(&session);
    assert!(extensions.contains(&avatar_id));
    assert!(!extensions.contains(&burrower_id));
    assert!(!extensions.contains(&target_id));
    assert_exact_replay(&session);
}

fn setup_pay_life_chain(life: u8, seed: u32) -> (Session, String, String) {
    let mut session = Session::new(&pay_life_chain_manifest(seed, life))
        .expect("valid pay-life Chain Magic session");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (target, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let before = state(&session);
    let chain_id = hand_instance(&before, "north-chain");
    let target_id = target["cardInstanceId"]
        .as_str()
        .expect("south minion identity")
        .to_owned();
    (session, chain_id, target_id)
}

fn offers_begin_pay_life_chain(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardId"] == "north-chain"
        })
}

fn replay_game(session: &Session) -> Game {
    let mut game = Game::from_manifest_json(session.manifest_json()).expect("valid replay game");
    for receipt in session.transcript() {
        let action = game
            .legal_actions()
            .expect("replay legal actions")
            .into_iter()
            .find(|action| {
                action
                    .to_legal_action()
                    .is_ok_and(|action| action.action_id == receipt.action_id)
            })
            .expect("recorded engine-issued action");
        game.apply_action(&action).expect("replay action");
    }
    game
}

fn branch_without_staged_discard(serialized: &str, zone: &str, discard_id: &str) -> Game {
    let parsed = parse_game_checkpoint(serialized).expect("parsed chain checkpoint");
    let resumed = resume_game_checkpoint(&parsed).expect("resumed chain session");
    let mut checkpoint_json: Value = serde_json::from_str(serialized).expect("checkpoint JSON");
    let mut branched = replay_game(&resumed);
    let mut edited = branched.authoritative_state();
    edited["players"]["north"]["hand"][zone]
        .as_array_mut()
        .expect("north hand zone")
        .retain(|card| card["instanceId"] != discard_id);
    checkpoint_json["editedState"] = edited.clone();
    assert!(
        branched.test_remove_north_hand_card(zone, discard_id),
        "edited checkpoint branch must drop the staged discard from {zone} hand"
    );
    assert_eq!(
        branched.authoritative_state()["players"]["north"]["hand"][zone],
        edited["players"]["north"]["hand"][zone],
        "checkpoint JSON edit must match the branched hand: {}",
        canonical_json(&checkpoint_json).expect("canonical edited checkpoint JSON")
    );
    branched
}

#[test]
fn rule_catalog_0885_chain_magic_pay_life_is_paid_before_the_cast_resolves() {
    let (mut session, chain_id, target_id) = setup_pay_life_chain(20, 885);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 20);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
    });
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "life-paid",
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(resolved.events[0].payload["amount"], 2);
    assert_eq!(resolved.events[0].payload["life"], 18);
    assert_eq!(resolved.events[1].payload["lifePaid"], 2);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    assert!(realm_unit(&state(&session), &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0886_deaths_door_cannot_begin_pay_life_chain_magic() {
    let blocked = setup_pay_life_chain(1, 886).0;
    assert_eq!(state(&blocked)["players"]["north"]["avatar"]["life"], 1);
    assert!(!offers_begin_pay_life_chain(&blocked));
    assert_exact_replay(&blocked);

    let (mut session, chain_id, target_id) = setup_pay_life_chain(2, 887);
    assert!(offers_begin_pay_life_chain(&session));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
    });
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "life-paid",
            "avatar-reached-deaths-door",
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 0);
    assert!(!state(&session)["players"]["north"]["avatar"]["deathDoorTurn"].is_null());
    assert!(!offers_begin_pay_life_chain(&session));
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0896_chain_magic_resolve_is_unoffered_when_pay_life_cost_exceeds_current_life() {
    let (mut session, chain_id, target_id) = setup_pay_life_chain(3, 896);
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 3);
    assert!(offers_begin_pay_life_chain(&session));
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
    });
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(staged["players"]["north"]["avatar"]["life"], 3);
    assert!(offers_resolve_chain_magic(&session));

    let checkpoint = create_game_checkpoint(&session).expect("staged pay-life checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let resumed = resume_game_checkpoint(&parsed).expect("resumed staged checkpoint");
    assert_eq!(state(&resumed), staged);
    assert!(offers_resolve_chain_magic(&resumed));

    let mut branched = replay_game(&resumed);
    branched.test_set_north_avatar_life(1);
    assert_eq!(
        branched.authoritative_state()["players"]["north"]["avatar"]["life"],
        1
    );
    assert!(
        !branched
            .legal_actions()
            .expect("branched legal actions")
            .iter()
            .any(|action| matches!(action.descriptor(), ActionDescriptor::ResolveChainMagic)),
        "life below the pay-life cost must issue no resolve-chain-magic"
    );
    assert_exact_replay(&session);
}

fn try_setup_discard_chain(encoded: &str) -> Option<(Session, String, String, String)> {
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
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (target, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let before = state(&session);
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let fodder_id = try_hand_instance(&before, "north-fodder")?;
    let target_id = target["cardInstanceId"].as_str()?.to_owned();
    Some((session, chain_id, fodder_id, target_id))
}

fn offers_discard_chain(session: &Session) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardId"] == "north-chain"
        })
}

#[test]
fn rule_catalog_0887_chain_magic_discard_is_paid_before_the_cast_resolves() {
    let spellbook = [
        "north-chain",
        "north-fodder",
        "north-fodder",
        "north-chain",
        "north-fodder",
        "north-fodder",
    ];
    let encoded = (887..887 + 512)
        .map(|seed| discard_chain_manifest(seed, &spellbook))
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-fodder"])
                && try_setup_discard_chain(candidate).is_some()
        })
        .expect("bounded seed with Chain Magic, fodder, and completable setup");
    let (mut session, chain_id, fodder_id, target_id) =
        try_setup_discard_chain(&encoded).expect("discard Chain Magic setup");
    assert!(offers_discard_chain(&session));
    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["discardCardInstanceId"] == fodder_id
    });
    assert_eq!(begin.events.len(), 0);
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "card-discarded",
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(resolved.events[0].payload["instanceId"], fodder_id);
    assert_eq!(resolved.events[0].payload["sourceInstanceId"], chain_id);
    assert_eq!(
        resolved.events[1].payload["discardCardInstanceId"],
        fodder_id
    );
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == fodder_id)
    );
    assert!(realm_unit(&after, &target_id).is_none());
    assert_exact_replay(&session);
}

fn north_spell_card_ids(snapshot: &Value) -> Vec<String> {
    snapshot["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("north Spellbook")
        .iter()
        .map(|card| card["cardId"].as_str().expect("card id").to_owned())
        .collect()
}

fn north_hand_ids(snapshot: &Value, zone: &str) -> Vec<String> {
    snapshot["players"]["north"]["hand"][zone]
        .as_array()
        .expect("north hand zone")
        .iter()
        .map(|card| {
            card["instanceId"]
                .as_str()
                .expect("hand identity")
                .to_owned()
        })
        .collect()
}

fn cast_all_fodder(session: &mut Session) {
    while session
        .legal_actions()
        .expect("fodder actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-fodder"
        })
    {
        accept_where(session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "north-fodder"
        });
    }
}

fn seed_with_discard_chain(north_spellbook: &[&str], required: &[&str], start: u32) -> String {
    (start..start + 256)
        .map(|seed| discard_chain_manifest(seed, north_spellbook))
        .find(|candidate| opening_has_all(candidate, required))
        .expect("bounded seed with required opening cards")
}

#[test]
fn rule_catalog_0888_chain_magic_discard_is_unoffered_without_another_hand_card() {
    let encoded = seed_with_discard_chain(
        &[
            "north-chain",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
        ],
        &["north-chain"],
        888,
    );
    let mut session = opening_main(&encoded);
    assert!(
        offers_discard_chain(&session),
        "Atlas leftovers still pay the discard cost"
    );
    cast_all_fodder(&mut session);
    assert!(offers_discard_chain(&session));
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        if state(&session)["players"]["south"]["domainEstablished"].as_bool() != Some(true) {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
            });
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "play-site");
        cast_all_fodder(&mut session);
    }
    let after = state(&session);
    assert_eq!(north_hand_ids(&after, "atlas").len(), 0);
    assert_eq!(north_spell_card_ids(&after), ["north-chain".to_owned()]);
    assert!(
        !offers_discard_chain(&session),
        "an empty other-hand must issue no discard Chain Magic"
    );
    assert_exact_replay(&session);
}

fn try_setup_atlas_discard_chain(
    encoded: &str,
) -> Option<(Session, String, String, String, String)> {
    let mut session = Session::new(encoded).ok()?;
    keep(&mut session);
    keep(&mut session);
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    })?;
    let opening = state(&session);
    let atlas_id = north_hand_ids(&opening, "atlas").into_iter().next()?;
    let (target, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally"
            && descriptor["cell"] == "C4"
    })?;
    let before = state(&session);
    if north_hand_ids(&before, "atlas").is_empty() {
        return None;
    }
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let target_id = target["cardInstanceId"].as_str()?.to_owned();
    Some((
        session,
        chain_id,
        atlas_id,
        target_id,
        "north-site".to_owned(),
    ))
}

fn atlas_discard_chain_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-atlas-discard" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-atlas-discard-v1",
        },
        "cards": {
            "north-avatar": avatar(),
            "north-chain": discard_chain(),
            "north-fodder": fodder(),
            "north-ally": minion(json!({})),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({ "summonToAnySite": true })),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-ally",
                    "north-fodder",
                    "north-chain",
                    "north-ally",
                    "north-fodder",
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

fn atlas_discard_hops_manifest(seed: u32) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "chain-magic-atlas-discard-hops" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-chain-magic-atlas-discard-hops-v1",
        },
        "cards": {
            "north-ally-a": minion(json!({})),
            "north-ally-b": minion(json!({})),
            "north-avatar": avatar(),
            "north-chain": discard_chain(),
            "north-fodder": fodder(),
            "north-site": site(),
            "south-avatar": avatar(),
            "south-minion": minion(json!({})),
            "south-site": site(),
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-chain",
                    "north-ally-a",
                    "north-ally-b",
                    "north-fodder",
                    "north-chain",
                    "north-fodder",
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

fn chain_discard_begin_ids(session: &Session, chain_id: &str) -> Vec<String> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardInstanceId"] == chain_id
        })
        .filter_map(|action| {
            action.descriptor["discardCardInstanceId"]
                .as_str()
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn rule_catalog_0889_chain_magic_discard_may_discard_an_atlas_card() {
    let encoded = (889..889 + 512)
        .map(atlas_discard_chain_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally"])
                && try_setup_atlas_discard_chain(candidate).is_some()
        })
        .expect("bounded seed with Chain Magic, ally, and Atlas discard setup");
    let (mut session, chain_id, atlas_id, target_id, site_card_id) =
        try_setup_atlas_discard_chain(&encoded).expect("Atlas discard Chain Magic setup");
    let atlas_before = state(&session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    let discard_ids = chain_discard_begin_ids(&session, &chain_id);
    assert!(discard_ids.iter().any(|id| id == &atlas_id));
    assert!(discard_ids.iter().all(|id| id != &chain_id));
    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["discardCardInstanceId"] == atlas_id
    });
    assert_eq!(begin.events.len(), 0);
    let checkpoint = create_game_checkpoint(&session).expect("staged chain checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized chain checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed chain checkpoint");
    let mut session = resume_game_checkpoint(&parsed).expect("resumed chain session");
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "card-discarded",
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(resolved.events[0].payload["cardId"], site_card_id);
    assert_eq!(resolved.events[0].payload["instanceId"], atlas_id);
    assert_eq!(resolved.events[0].payload["zone"], "atlas");
    assert_eq!(resolved.events[0].payload["sourceInstanceId"], chain_id);
    assert_eq!(
        resolved.events[1].payload["discardCardInstanceId"],
        atlas_id
    );
    let after = state(&session);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == atlas_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        atlas_before - 1
    );
    assert!(realm_unit(&after, &target_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0890_chain_magic_issues_no_begin_without_a_chosen_discard_while_atlas_remains() {
    let encoded = seed_with_discard_chain(
        &[
            "north-chain",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
            "north-fodder",
        ],
        &["north-chain"],
        890,
    );
    let session = opening_main(&encoded);
    let chain_id = hand_instance(&state(&session), "north-chain");
    assert!(!north_hand_ids(&state(&session), "atlas").is_empty());
    assert!(offers_discard_chain(&session));
    let discard_ids = chain_discard_begin_ids(&session, &chain_id);
    assert!(
        !discard_ids.is_empty(),
        "Atlas leftovers still pay the discard cost"
    );
    assert!(
        session
            .legal_actions()
            .expect("legal actions")
            .iter()
            .filter(|action| {
                action.descriptor["kind"] == "begin-chain-magic"
                    && action.descriptor["cardInstanceId"] == chain_id
            })
            .all(|action| action.descriptor["discardCardInstanceId"].is_string()),
        "every issued begin must carry the chosen discard identity"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0894_chain_magic_resolve_is_unoffered_without_staged_discard_in_hand() {
    let spellbook = [
        "north-chain",
        "north-fodder",
        "north-fodder",
        "north-chain",
        "north-fodder",
        "north-fodder",
    ];
    let encoded = (894..894 + 512)
        .map(|seed| discard_chain_manifest(seed, &spellbook))
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-fodder"])
                && try_setup_discard_chain(candidate).is_some()
        })
        .expect("bounded seed with Chain Magic discard setup");
    let (mut session, chain_id, fodder_id, target_id) =
        try_setup_discard_chain(&encoded).expect("discard Chain Magic setup");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
            && descriptor["discardCardInstanceId"] == fodder_id
    });
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["discardCardInstanceId"],
        fodder_id
    );
    assert!(offers_resolve_chain_magic(&session));

    let checkpoint = create_game_checkpoint(&session).expect("staged chain checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized chain checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed chain checkpoint");
    let resumed = resume_game_checkpoint(&parsed).expect("resumed chain session");
    assert_eq!(state(&resumed), staged);
    assert!(offers_resolve_chain_magic(&resumed));

    let branched = branch_without_staged_discard(&serialized, "spellbook", &fodder_id);
    assert_eq!(
        branched.authoritative_state()["pendingChainMagic"]["discardCardInstanceId"],
        fodder_id
    );
    assert!(
        !branched
            .legal_actions()
            .expect("branched legal actions")
            .iter()
            .any(|action| matches!(action.descriptor(), ActionDescriptor::ResolveChainMagic)),
        "a staged discard that left hand must issue no resolve-chain-magic"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0893_chain_magic_staged_mana_gates_resolve_and_extend_independently() {
    const EXTRA_TARGET_MANA: u64 = 2;
    let encoded = (893..893 + 256)
        .map(hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
        })
        .expect("bounded seed with Chain Magic and both nearby allies in the opening hand");
    let mut hops = setup_hops(&encoded);
    assert_eq!(hops.mana, EXTRA_TARGET_MANA);
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    assert_eq!(
        state(&hops.session)["pendingChainMagic"]["targets"]
            .as_array()
            .expect("staged targets")
            .len(),
        1
    );
    assert!(offers_resolve_chain_magic(&hops.session));
    assert_eq!(
        sorted(extend_ids(&hops.session)),
        sorted(vec![hops.avatar_id.clone(), hops.second_id.clone()])
    );

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["instanceId"] == hops.second_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["players"]["north"]["mana"], EXTRA_TARGET_MANA);
    assert_eq!(
        staged["pendingChainMagic"]["targets"]
            .as_array()
            .expect("staged targets")
            .len(),
        2
    );
    assert!(
        offers_resolve_chain_magic(&hops.session),
        "two staged hops on a zero-cost chain cost {EXTRA_TARGET_MANA} mana to resolve"
    );
    assert!(
        extend_ids(&hops.session).is_empty(),
        "a third hop would cost {} mana",
        EXTRA_TARGET_MANA * 3
    );

    let mut stuck = replay_game(&hops.session);
    stuck.test_set_north_mana(1);
    assert_eq!(stuck.authoritative_state()["players"]["north"]["mana"], 1);
    assert!(
        !stuck
            .legal_actions()
            .expect("stuck staged chain actions")
            .iter()
            .any(|action| matches!(action.descriptor(), ActionDescriptor::ResolveChainMagic)),
        "two staged hops need {EXTRA_TARGET_MANA} mana to resolve"
    );
    assert!(
        !stuck
            .legal_actions()
            .expect("stuck staged chain actions")
            .iter()
            .any(|action| matches!(
                action.descriptor(),
                ActionDescriptor::ExtendChainMagic { .. }
            )),
        "a third hop would exceed the one-mana pool"
    );

    let mut short = setup_hops_short(&encoded, true);
    assert_eq!(short.mana, 1);
    accept_where(&mut short.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == short.chain_id
            && descriptor["target"]["instanceId"] == short.first_id
    });
    assert!(
        offers_resolve_chain_magic(&short.session),
        "one staged hop on a zero-cost chain resolves for free"
    );
    assert!(
        extend_ids(&short.session).is_empty(),
        "the next hop costs {EXTRA_TARGET_MANA} mana while the caster has one"
    );
    assert_exact_replay(&hops.session);
    assert_exact_replay(&short.session);
}

#[test]
fn rule_catalog_0904_extend_chain_magic_is_withheld_when_next_hop_mana_exceeds_pool_while_resolve_remains_legal()
 {
    const EXTRA_TARGET_MANA: u64 = 2;
    let encoded = (904..904 + 256)
        .map(hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
        })
        .expect("bounded seed with Chain Magic and both nearby allies in the opening hand");
    let mut hops = setup_hops_short(&encoded, true);
    assert_eq!(hops.mana, 1);
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(staged["players"]["north"]["mana"], 1);
    let chosen_count = staged["pendingChainMagic"]["targets"]
        .as_array()
        .expect("staged targets")
        .len() as u64;
    assert_eq!(chosen_count, 1);
    let resolve_mana = EXTRA_TARGET_MANA.saturating_mul(chosen_count.saturating_sub(1));
    let extend_mana = EXTRA_TARGET_MANA.saturating_mul(chosen_count);
    assert_eq!(
        resolve_mana, 0,
        "resolve uses mana_paid for the current count"
    );
    assert_eq!(
        extend_mana, EXTRA_TARGET_MANA,
        "extend uses next_mana for one more hop"
    );
    assert!(
        offers_resolve_chain_magic(&hops.session),
        "resolve-chain-magic stays legal at {resolve_mana} mana with one mana in pool"
    );
    assert!(
        extend_ids(&hops.session).is_empty(),
        "extend-chain-magic needs {extend_mana} mana for the next hop while the caster has one"
    );
    let legal = hops.session.legal_actions().expect("staged chain actions");
    assert!(
        legal
            .iter()
            .any(|action| action.descriptor["kind"] == "resolve-chain-magic")
    );
    assert!(
        !legal
            .iter()
            .any(|action| action.descriptor["kind"] == "extend-chain-magic")
    );
    assert_exact_replay(&hops.session);
}

#[test]
fn rule_catalog_0913_resolve_chain_magic_is_withheld_after_extend_when_staged_target_mana_exceeds_pool_while_resolve_was_legal_at_one_target()
 {
    const EXTRA_TARGET_MANA: u64 = 2;
    let encoded = (913..913 + 256)
        .map(hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
        })
        .expect("bounded seed with Chain Magic and both nearby allies in the opening hand");
    let mut hops = setup_hops(&encoded);
    assert_eq!(hops.mana, EXTRA_TARGET_MANA);
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(staged["players"]["north"]["mana"], EXTRA_TARGET_MANA);
    let chosen_count = staged["pendingChainMagic"]["targets"]
        .as_array()
        .expect("staged targets")
        .len() as u64;
    assert_eq!(chosen_count, 1);
    let resolve_mana = EXTRA_TARGET_MANA.saturating_mul(chosen_count.saturating_sub(1));
    let extend_mana = EXTRA_TARGET_MANA.saturating_mul(chosen_count);
    assert_eq!(
        resolve_mana, 0,
        "resolve uses mana_paid for the current count"
    );
    assert_eq!(
        extend_mana, EXTRA_TARGET_MANA,
        "extend uses next_mana for one more hop"
    );
    assert!(
        offers_resolve_chain_magic(&hops.session),
        "resolve-chain-magic stays legal at {resolve_mana} mana with {EXTRA_TARGET_MANA} mana in pool"
    );
    assert_eq!(
        sorted(extend_ids(&hops.session)),
        sorted(vec![hops.avatar_id.clone(), hops.second_id.clone()]),
        "extend-chain-magic needs exactly {extend_mana} mana for the next hop"
    );

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["instanceId"] == hops.second_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["players"]["north"]["mana"], EXTRA_TARGET_MANA);
    let chosen_count = staged["pendingChainMagic"]["targets"]
        .as_array()
        .expect("staged targets")
        .len() as u64;
    assert_eq!(chosen_count, 2);
    let resolve_mana = EXTRA_TARGET_MANA.saturating_mul(chosen_count.saturating_sub(1));
    let extend_mana = EXTRA_TARGET_MANA.saturating_mul(chosen_count);
    assert_eq!(
        resolve_mana, EXTRA_TARGET_MANA,
        "resolve uses mana_paid for the current count"
    );
    assert_eq!(
        extend_mana,
        EXTRA_TARGET_MANA * 2,
        "extend uses next_mana for one more hop"
    );
    assert!(
        extend_ids(&hops.session).is_empty(),
        "a third hop would cost {extend_mana} mana while the caster has {EXTRA_TARGET_MANA}"
    );

    let mut depleted = replay_game(&hops.session);
    depleted.test_set_north_mana(1);
    assert_eq!(
        depleted.authoritative_state()["players"]["north"]["mana"],
        1
    );
    assert!(
        !depleted
            .legal_actions()
            .expect("depleted staged chain actions")
            .iter()
            .any(|action| matches!(action.descriptor(), ActionDescriptor::ResolveChainMagic)),
        "two staged hops need {resolve_mana} mana to resolve while the caster has one"
    );
    assert!(
        !depleted
            .legal_actions()
            .expect("depleted staged chain actions")
            .iter()
            .any(|action| matches!(
                action.descriptor(),
                ActionDescriptor::ExtendChainMagic { .. }
            )),
        "a third hop would exceed the one-mana pool"
    );
    assert_exact_replay(&hops.session);
}

fn try_setup_discard_hops(encoded: &str) -> Option<(ChainHops, String)> {
    if !opening_has_all(encoded, &["north-chain", "north-ally-a", "north-ally-b"]) {
        return None;
    }
    let hops = setup_hops(encoded);
    let fodder_id = try_hand_instance(&state(&hops.session), "north-fodder")?;
    Some((hops, fodder_id))
}

fn try_setup_atlas_discard_hops(encoded: &str) -> Option<(ChainHops, String, String)> {
    if !opening_has_all(encoded, &["north-chain", "north-ally-a", "north-ally-b"]) {
        return None;
    }
    let hops = setup_hops(encoded);
    let snapshot = state(&hops.session);
    let atlas_id = north_hand_ids(&snapshot, "atlas").into_iter().next()?;
    Some((hops, atlas_id, "north-site".to_owned()))
}

#[test]
fn rule_catalog_0903_chain_magic_checkpoint_resume_preserves_staged_targets_discard_and_actions() {
    let encoded = (903..903 + 512)
        .map(discard_hops_manifest)
        .find(|candidate| try_setup_discard_hops(candidate).is_some())
        .expect("bounded seed with discard Chain Magic, both nearby allies, and fodder in hand");
    let (mut hops, fodder_id) =
        try_setup_discard_hops(&encoded).expect("discard Chain Magic hops setup");
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
            && descriptor["discardCardInstanceId"] == fodder_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": hops.first_id,
            "kind": "minion",
            "seat": "north",
        }])
    );
    assert_eq!(
        staged["pendingChainMagic"]["discardCardInstanceId"],
        fodder_id
    );
    assert!(offers_resolve_chain_magic(&hops.session));
    assert_eq!(
        sorted(extend_ids(&hops.session)),
        sorted(vec![hops.avatar_id.clone(), hops.second_id.clone()])
    );

    let checkpoint = create_game_checkpoint(&hops.session).expect("staged chain checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized chain checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed chain checkpoint");
    let mut session = resume_game_checkpoint(&parsed).expect("resumed chain session");
    assert_eq!(state(&session), staged);
    assert_eq!(
        state(&session)["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": hops.first_id,
            "kind": "minion",
            "seat": "north",
        }])
    );
    assert_eq!(
        state(&session)["pendingChainMagic"]["discardCardInstanceId"],
        fodder_id
    );
    assert!(offers_resolve_chain_magic(&session));
    assert_eq!(
        sorted(extend_ids(&session)),
        sorted(vec![hops.avatar_id.clone(), hops.second_id.clone()])
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["instanceId"] == hops.second_id
    });
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "card-discarded",
            "magic-cast",
            "magic-damage-allocated",
            "magic-damage-allocated",
            "damage-dealt",
            "damage-dealt",
            "minion-died",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(resolved.events[0].payload["instanceId"], fodder_id);
    assert_eq!(
        resolved.events[1].payload["discardCardInstanceId"],
        fodder_id
    );
    let after = state(&session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingChainMagic"].is_null());
    assert!(realm_unit(&after, &hops.first_id).is_none());
    assert!(realm_unit(&after, &hops.second_id).is_none());
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0914_chain_magic_phase_issues_only_chain_actions_while_staged() {
    let encoded = (914..914 + 512)
        .map(discard_hops_manifest)
        .find(|candidate| try_setup_discard_hops(candidate).is_some())
        .expect("bounded seed with discard Chain Magic, both nearby allies, and fodder in hand");
    let (mut hops, fodder_id) =
        try_setup_discard_hops(&encoded).expect("discard Chain Magic hops setup");
    let main_before = hops.session.legal_actions().expect("main legal actions");
    assert_eq!(state(&hops.session)["phase"], "main");
    assert!(
        main_before
            .iter()
            .any(|action| action.descriptor["kind"] == "cast-magic"),
        "main phase must offer cast-magic before Chain Magic is staged"
    );
    assert!(
        main_before
            .iter()
            .any(|action| action.descriptor["kind"] == "end-turn")
    );
    assert!(
        main_before
            .iter()
            .any(|action| action.descriptor["kind"] == "move-and-attack")
    );
    assert!(
        main_before
            .iter()
            .any(|action| action.descriptor["kind"] == "begin-chain-magic")
    );

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
            && descriptor["discardCardInstanceId"] == fodder_id
    });
    assert_staged_chain_magic_legal_actions_only(&hops.session);
    assert!(offers_resolve_chain_magic(&hops.session));
    assert!(!extend_ids(&hops.session).is_empty());
    assert_exact_replay(&hops.session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Atlas discard Chain Magic extend-through-resolve scenario proof keeps assertions inline"
)]
fn rule_catalog_0934_extend_chain_magic_preserves_staged_discard_through_resolve() {
    let encoded = (934..934 + 512)
        .map(atlas_discard_hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
                && try_setup_atlas_discard_hops(candidate).is_some()
        })
        .expect("bounded seed with Chain Magic, both nearby allies, and Atlas discard");
    let (mut hops, atlas_id, site_card_id) =
        try_setup_atlas_discard_hops(&encoded).expect("Atlas discard Chain Magic hops setup");
    let atlas_before = state(&hops.session)["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("north Atlas")
        .len();
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
            && descriptor["discardCardInstanceId"] == atlas_id
    });
    let after_begin = state(&hops.session);
    assert_eq!(after_begin["phase"], "chain-magic");
    assert_eq!(
        after_begin["pendingChainMagic"]["discardCardInstanceId"],
        atlas_id
    );
    assert_eq!(
        after_begin["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": hops.first_id,
            "kind": "minion",
            "seat": "north",
        }])
    );

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["instanceId"] == hops.second_id
    });
    let after_extend = state(&hops.session);
    assert_eq!(after_extend["phase"], "chain-magic");
    assert_eq!(
        after_extend["pendingChainMagic"]["discardCardInstanceId"],
        atlas_id
    );
    assert_eq!(
        after_extend["pendingChainMagic"]["targets"],
        json!([
            {
                "instanceId": hops.first_id,
                "kind": "minion",
                "seat": "north",
            },
            {
                "instanceId": hops.second_id,
                "kind": "minion",
                "seat": "north",
            },
        ])
    );

    let (_, resolved) = accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "card-discarded",
            "magic-cast",
            "magic-damage-allocated",
            "magic-damage-allocated",
            "damage-dealt",
            "damage-dealt",
            "minion-died",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(resolved.events[0].payload["cardId"], site_card_id);
    assert_eq!(resolved.events[0].payload["instanceId"], atlas_id);
    assert_eq!(resolved.events[0].payload["zone"], "atlas");
    assert_eq!(
        resolved.events[0].payload["sourceInstanceId"],
        hops.chain_id
    );
    assert_eq!(
        resolved.events[1].payload["discardCardInstanceId"],
        atlas_id
    );
    let after = state(&hops.session);
    assert_eq!(after["phase"], "main");
    assert!(after["pendingChainMagic"].is_null());
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == atlas_id)
    );
    assert_eq!(
        after["players"]["north"]["hand"]["atlas"]
            .as_array()
            .expect("north Atlas")
            .len(),
        atlas_before - 1
    );
    assert!(realm_unit(&after, &hops.first_id).is_none());
    assert!(realm_unit(&after, &hops.second_id).is_none());
    assert_exact_replay(&hops.session);
}

#[test]
fn rule_catalog_0923_extend_chain_magic_cannot_retarget_already_staged_hop() {
    let encoded = (923..923 + 256)
        .map(hops_manifest)
        .find(|candidate| {
            opening_has_all(candidate, &["north-chain", "north-ally-a", "north-ally-b"])
        })
        .expect("bounded seed with Chain Magic and both nearby allies in the opening hand");
    let mut hops = setup_hops(&encoded);
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": hops.first_id,
            "kind": "minion",
            "seat": "north",
        }])
    );
    assert!(!extend_ids(&hops.session).contains(&hops.first_id));
    assert_eq!(
        sorted(extend_ids(&hops.session)),
        sorted(vec![hops.avatar_id.clone(), hops.second_id.clone()])
    );
    assert_exact_replay(&hops.session);
}

struct SpellcasterChainHops {
    caster_id: String,
    hops: ChainHops,
}

fn try_setup_isolated_spellcaster(encoded: &str) -> Option<(Session, String, String)> {
    if !opening_has_all(encoded, &["north-chain", "north-caster"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    let (caster, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
            && descriptor["region"] == "underground"
    })?;
    let before = state(&session);
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let caster_id = caster["cardInstanceId"]
        .as_str()
        .expect("printed caster identity")
        .to_owned();
    Some((session, chain_id, caster_id))
}

fn offers_begin_spellcaster_chain(session: &Session, chain_id: &str, caster_id: &str) -> bool {
    session
        .legal_actions()
        .expect("legal actions")
        .iter()
        .any(|action| {
            action.descriptor["kind"] == "begin-chain-magic"
                && action.descriptor["cardInstanceId"] == chain_id
                && action.descriptor["casterInstanceId"] == caster_id
        })
}

fn try_setup_spellcaster_avatar_hop(
    encoded: &str,
) -> Option<(Session, String, String, String, u64)> {
    if !opening_has_all(encoded, &["north-chain", "north-caster"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let (caster, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C2"
    })?;
    let before = state(&session);
    let south_avatar_id = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let caster_id = caster["cardInstanceId"]
        .as_str()
        .expect("printed caster identity")
        .to_owned();
    let caster_unit = realm_unit(&before, &caster_id)?;
    if caster_unit["location"] != "C2" || caster_unit["region"] != "surface" {
        return None;
    }
    if before["players"]["south"]["avatar"]["location"] != "C1" {
        return None;
    }
    let offers_enemy_avatar = session.legal_actions().ok()?.iter().any(|action| {
        action.descriptor["kind"] == "begin-chain-magic"
            && action.descriptor["cardInstanceId"] == chain_id
            && action.descriptor["casterInstanceId"] == caster_id
            && action.descriptor["target"]["kind"] == "avatar"
            && action.descriptor["target"]["seat"] == "south"
            && action.descriptor["target"]["instanceId"] == south_avatar_id
    });
    if !offers_enemy_avatar {
        return None;
    }
    let life = before["players"]["south"]["avatar"]["life"]
        .as_u64()
        .expect("South Avatar life");
    Some((session, chain_id, caster_id, south_avatar_id, life))
}

fn try_setup_spellcaster_distant_avatar_hop(
    encoded: &str,
) -> Option<(Session, String, String, String, String)> {
    if !opening_has_all(encoded, &["north-chain", "north-caster"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    let (caster, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (south_minion, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let before = state(&session);
    let south_avatar_id = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    let south_minion_id = south_minion["cardInstanceId"]
        .as_str()
        .expect("South minion identity")
        .to_owned();
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let caster_id = caster["cardInstanceId"]
        .as_str()
        .expect("printed caster identity")
        .to_owned();
    let caster_unit = realm_unit(&before, &caster_id)?;
    let south_minion_unit = realm_unit(&before, &south_minion_id)?;
    if caster_unit["location"] != "C4" || caster_unit["region"] != "surface" {
        return None;
    }
    if south_minion_unit["location"] != "C3" || south_minion_unit["region"] != "surface" {
        return None;
    }
    if before["players"]["south"]["avatar"]["location"] != "C1" {
        return None;
    }
    let begin_targets = chain_ids(&session, &chain_id);
    if !begin_targets.contains(&south_minion_id) {
        return None;
    }
    if begin_targets
        .iter()
        .any(|target_id| target_id == &south_avatar_id)
    {
        return None;
    }
    Some((
        session,
        chain_id,
        caster_id,
        south_minion_id,
        south_avatar_id,
    ))
}

fn try_setup_spellcaster_hops(encoded: &str) -> Option<SpellcasterChainHops> {
    if !opening_has_all(encoded, &["north-chain", "north-caster", "north-ally-a"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    let (caster, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C4"
    })?;
    let caster_id = caster["cardInstanceId"]
        .as_str()
        .expect("printed caster identity")
        .to_owned();
    let (first, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally-a"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    let before = state(&session);
    let avatar_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    Some(SpellcasterChainHops {
        caster_id,
        hops: ChainHops {
            avatar_id: avatar_id.clone(),
            chain_id: hand_instance(&before, "north-chain"),
            first_id: first["cardInstanceId"]
                .as_str()
                .expect("first hop identity")
                .to_owned(),
            mana: before["players"]["north"]["mana"]
                .as_u64()
                .expect("North mana"),
            second_id: avatar_id,
            session,
        },
    })
}

#[test]
fn rule_catalog_0955_begin_chain_magic_may_target_nearby_enemy_avatar_as_first_hop() {
    let encoded = (955..955 + 256)
        .map(spellcaster_avatar_manifest)
        .find(|candidate| try_setup_spellcaster_avatar_hop(candidate).is_some())
        .expect(
            "bounded seed with Chain Magic, printed Spellcaster at C2, and nearby South Avatar",
        );
    let (mut session, chain_id, caster_id, south_avatar_id, life_before) =
        try_setup_spellcaster_avatar_hop(&encoded).expect("spellcaster avatar hop setup");
    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar_id
    });
    assert!(begin.events.is_empty());
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": south_avatar_id,
            "kind": "avatar",
            "seat": "south",
        }])
    );
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        resolved
            .events
            .iter()
            .find(|event| event.event_type == "magic-damage-allocated")
            .expect("avatar hop allocation")
            .payload,
        json!({
            "amount": 2,
            "sourceInstanceId": chain_id,
            "targetInstanceId": south_avatar_id,
        })
    );
    assert!(
        event_types(&resolved).contains(&"avatar-life-lost"),
        "resolve must damage Avatar life"
    );
    assert_eq!(
        state(&session)["players"]["south"]["avatar"]["life"],
        life_before - 2
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0970_begin_chain_magic_omits_distant_enemy_avatar_as_first_hop() {
    let encoded = (955..955 + 512)
        .map(spellcaster_avatar_manifest)
        .find(|candidate| try_setup_spellcaster_distant_avatar_hop(candidate).is_some())
        .expect(
            "bounded seed with Chain Magic, printed Spellcaster at C4, nearby South minion at C3, and distant South Avatar at C1",
        );
    let (mut session, chain_id, caster_id, south_minion_id, south_avatar_id) =
        try_setup_spellcaster_distant_avatar_hop(&encoded)
            .expect("spellcaster distant avatar hop setup");
    let begin_targets = chain_ids(&session, &chain_id);
    assert!(
        begin_targets.contains(&south_minion_id),
        "append_main must offer the nearby enemy minion as a first hop"
    );
    assert!(
        !begin_targets.contains(&south_avatar_id),
        "append_main must omit the distant enemy Avatar as a first hop"
    );
    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_minion_id
    });
    assert!(begin.events.is_empty());
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": south_minion_id,
            "kind": "minion",
            "seat": "south",
        }])
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0952_chain_magic_is_unoffered_when_printed_spellcaster_has_zero_legal_first_hops() {
    let encoded = (952..952 + 256)
        .map(isolated_spellcaster_manifest)
        .find(|candidate| try_setup_isolated_spellcaster(candidate).is_some())
        .expect("bounded seed with Chain Magic, burrowing Spellcaster, and underground setup");
    let (session, chain_id, caster_id) =
        try_setup_isolated_spellcaster(&encoded).expect("isolated Spellcaster setup");
    let snapshot = state(&session);
    let caster = realm_unit(&snapshot, &caster_id).expect("burrowed Spellcaster in realm");
    assert_eq!(caster["location"], "C4");
    assert_eq!(caster["region"], "underground");
    assert!(
        snapshot["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .filter(|unit| unit["instanceId"] != caster_id)
            .all(|unit| unit["region"] != "underground"),
        "the underground caster region must have no other nearby units"
    );
    assert!(
        !offers_begin_spellcaster_chain(&session, &chain_id, &caster_id),
        "append_main must issue no begin-chain-magic for the printed Spellcaster caster"
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0943_chain_magic_withheld_when_staged_target_minion_leaves_realm() {
    let encoded = (943..943 + 256)
        .map(spellcaster_hops_manifest)
        .find(|candidate| try_setup_spellcaster_hops(candidate).is_some())
        .expect("bounded seed with printed Spellcaster, allies, and Chain Magic draw");
    let mut setup = try_setup_spellcaster_hops(&encoded).expect("spellcaster hops setup");
    let hops = &mut setup.hops;

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["casterInstanceId"] == setup.caster_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": hops.first_id,
            "kind": "minion",
            "seat": "north",
        }])
    );
    assert!(offers_resolve_chain_magic(&hops.session));
    assert!(!extend_ids(&hops.session).is_empty());

    let checkpoint = create_game_checkpoint(&hops.session).expect("staged target checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized target checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed target checkpoint");
    let resumed = resume_game_checkpoint(&parsed).expect("resumed staged target session");
    assert_eq!(state(&resumed), staged);
    assert!(offers_resolve_chain_magic(&resumed));

    let mut branched = replay_game(&resumed);
    assert!(
        branched.test_remove_realm_unit(&hops.first_id),
        "checkpoint branch must remove the first staged hop from the Realm"
    );
    assert!(
        realm_unit(&branched.authoritative_state(), &hops.first_id).is_none(),
        "staged target must leave the Realm before legal_actions is reissued"
    );
    match branched.legal_actions() {
        Err(_) => {}
        Ok(legal) => {
            assert!(
                !legal.iter().any(|action| matches!(
                    action.descriptor(),
                    ActionDescriptor::ResolveChainMagic
                )),
                "a staged target that left the Realm must issue no resolve-chain-magic"
            );
            assert!(
                !legal.iter().any(|action| {
                    matches!(
                        action.descriptor(),
                        ActionDescriptor::ExtendChainMagic { .. }
                    )
                }),
                "a staged target that left the Realm must issue no extend-chain-magic"
            );
        }
    }
    assert_exact_replay(&hops.session);
}

#[test]
fn rule_catalog_0933_chain_magic_withheld_when_staged_caster_is_not_a_legal_spellcaster() {
    let encoded = (933..933 + 256)
        .map(spellcaster_hops_manifest)
        .find(|candidate| try_setup_spellcaster_hops(candidate).is_some())
        .expect("bounded seed with printed Spellcaster, allies, and Chain Magic draw");
    let mut setup = try_setup_spellcaster_hops(&encoded).expect("spellcaster hops setup");
    let hops = &mut setup.hops;
    assert!(
        hops.session
            .legal_actions()
            .expect("main legal actions")
            .iter()
            .any(|action| {
                action.descriptor["kind"] == "begin-chain-magic"
                    && action.descriptor["cardInstanceId"] == hops.chain_id
                    && action.descriptor["casterInstanceId"] == setup.caster_id
            }),
        "append_main must issue begin-chain-magic for the printed Spellcaster caster"
    );

    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["casterInstanceId"] == setup.caster_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["casterInstanceId"],
        setup.caster_id
    );
    assert!(offers_resolve_chain_magic(&hops.session));
    assert!(!extend_ids(&hops.session).is_empty());

    let checkpoint = create_game_checkpoint(&hops.session).expect("staged caster checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized caster checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed caster checkpoint");
    let resumed = resume_game_checkpoint(&parsed).expect("resumed staged caster session");
    assert_eq!(state(&resumed), staged);
    assert!(offers_resolve_chain_magic(&resumed));

    let mut branched = replay_game(&resumed);
    assert!(
        branched.test_remove_realm_unit(&setup.caster_id),
        "checkpoint branch must remove the staged Spellcaster from the Realm"
    );
    assert!(
        realm_unit(&branched.authoritative_state(), &setup.caster_id).is_none(),
        "staged caster must leave the Realm before legal_actions is reissued"
    );
    assert!(
        branched.legal_actions().is_err(),
        "append_chain_magic_actions must fail when pending.caster_instance_id is not a legal Spellcaster"
    );
    assert_exact_replay(&hops.session);
}

fn try_setup_stealth_hops(encoded: &str) -> Option<(ChainHops, String, String)> {
    if !opening_has_all(encoded, &["north-chain", "north-ally-a"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    let (first, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally-a"
            && descriptor["cell"] == "C4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (visible, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-visible"
            && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "D4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (stealth, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-stealth"
            && descriptor["cell"] == "D4"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let before = state(&session);
    let visible_id = visible["cardInstanceId"].as_str()?.to_owned();
    let stealth_id = stealth["cardInstanceId"].as_str()?.to_owned();
    let visible_unit = realm_unit(&before, &visible_id)?;
    let stealth_unit = realm_unit(&before, &stealth_id)?;
    if visible_unit["stealthed"] == true || stealth_unit["stealthed"] != true {
        return None;
    }
    Some((
        ChainHops {
            avatar_id: before["players"]["north"]["avatar"]["card"]["instanceId"]
                .as_str()
                .expect("North Avatar identity")
                .to_owned(),
            chain_id: hand_instance(&before, "north-chain"),
            first_id: first["cardInstanceId"]
                .as_str()
                .expect("first hop identity")
                .to_owned(),
            mana: before["players"]["north"]["mana"]
                .as_u64()
                .expect("North mana"),
            second_id: visible_id.clone(),
            session,
        },
        visible_id,
        stealth_id,
    ))
}

#[test]
fn rule_catalog_0944_extend_chain_magic_omits_nearby_stealthed_enemy_minions() {
    let encoded = (944..944 + 512)
        .map(stealth_hops_manifest)
        .find(|candidate| try_setup_stealth_hops(candidate).is_some())
        .expect(
            "bounded seed with Chain Magic, ally hop, and nearby visible and stealthed enemies",
        );
    let (mut hops, visible_id, stealth_id) =
        try_setup_stealth_hops(&encoded).expect("stealth Chain Magic hops setup");
    accept_where(&mut hops.session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == hops.chain_id
            && descriptor["target"]["instanceId"] == hops.first_id
    });
    let staged = state(&hops.session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": hops.first_id,
            "kind": "minion",
            "seat": "north",
        }])
    );
    let extensions = extend_ids(&hops.session);
    assert!(
        extensions.contains(&visible_id),
        "extend-chain-magic must still offer the nearby visible enemy"
    );
    assert!(
        !extensions.contains(&stealth_id),
        "extend-chain-magic must omit the nearby stealthed enemy"
    );
    assert_exact_replay(&hops.session);
}

fn try_setup_spellcaster_extend_enemy_avatar_hop(
    encoded: &str,
) -> Option<(Session, String, String, String, String, u64)> {
    if !opening_has_all(encoded, &["north-chain", "north-caster"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    })?;
    let (south_minion, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-minion"
            && descriptor["cell"] == "C2"
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let (caster, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-caster"
            && descriptor["cell"] == "C3"
    })?;
    let before = state(&session);
    let south_avatar_id = before["players"]["south"]["avatar"]["card"]["instanceId"]
        .as_str()?
        .to_owned();
    let south_minion_id = south_minion["cardInstanceId"]
        .as_str()
        .expect("South minion identity")
        .to_owned();
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let caster_id = caster["cardInstanceId"]
        .as_str()
        .expect("printed caster identity")
        .to_owned();
    let caster_unit = realm_unit(&before, &caster_id)?;
    let south_minion_unit = realm_unit(&before, &south_minion_id)?;
    if caster_unit["location"] != "C3" || caster_unit["region"] != "surface" {
        return None;
    }
    if south_minion_unit["location"] != "C2" || south_minion_unit["region"] != "surface" {
        return None;
    }
    if before["players"]["south"]["avatar"]["location"] != "C1" {
        return None;
    }
    let begin_targets = chain_ids(&session, &chain_id);
    if !begin_targets.contains(&south_minion_id) {
        return None;
    }
    if begin_targets.contains(&south_avatar_id) {
        return None;
    }
    Some((
        session,
        chain_id,
        caster_id,
        south_minion_id,
        south_avatar_id,
        before["players"]["south"]["avatar"]["life"]
            .as_u64()
            .expect("South Avatar life"),
    ))
}

#[test]
fn rule_catalog_0981_extend_chain_magic_may_target_nearby_enemy_avatar_as_second_hop() {
    let encoded = (981..981 + 512)
        .map(spellcaster_avatar_manifest)
        .find(|candidate| try_setup_spellcaster_extend_enemy_avatar_hop(candidate).is_some())
        .expect(
            "bounded seed with Chain Magic, printed Spellcaster at C3, nearby South minion at C2, and nearby South Avatar at C1 for extend",
        );
    let (mut session, chain_id, caster_id, south_minion_id, south_avatar_id, life_before) =
        try_setup_spellcaster_extend_enemy_avatar_hop(&encoded)
            .expect("spellcaster extend enemy avatar hop setup");
    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_minion_id
    });
    assert!(begin.events.is_empty());
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": south_minion_id,
            "kind": "minion",
            "seat": "south",
        }])
    );
    assert!(
        extend_ids(&session).contains(&south_avatar_id),
        "extend-chain-magic must offer the nearby enemy Avatar as a second hop"
    );
    assert!(
        !extend_ids(&session).contains(&south_minion_id),
        "extend-chain-magic must omit the already-staged minion hop"
    );
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "extend-chain-magic"
            && descriptor["target"]["kind"] == "avatar"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_avatar_id
    });
    let extended = state(&session);
    assert_eq!(
        extended["pendingChainMagic"]["targets"],
        json!([
            {
                "instanceId": south_minion_id,
                "kind": "minion",
                "seat": "south",
            },
            {
                "instanceId": south_avatar_id,
                "kind": "avatar",
                "seat": "south",
            }
        ])
    );
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        resolved
            .events
            .iter()
            .find(|event| {
                event.event_type == "magic-damage-allocated"
                    && event.payload["targetInstanceId"] == south_avatar_id
            })
            .expect("avatar hop allocation")
            .payload,
        json!({
            "amount": 2,
            "sourceInstanceId": chain_id,
            "targetInstanceId": south_avatar_id,
        })
    );
    assert!(
        event_types(&resolved).contains(&"avatar-life-lost"),
        "resolve must damage Avatar life"
    );
    assert_eq!(
        state(&session)["players"]["south"]["avatar"]["life"],
        life_before - 2
    );
    assert_exact_replay(&session);
}

#[test]
fn rule_catalog_0985_extend_chain_magic_omits_distant_enemy_avatar_as_second_hop() {
    let encoded = (985..985 + 512)
        .map(spellcaster_avatar_manifest)
        .find(|candidate| try_setup_spellcaster_distant_avatar_hop(candidate).is_some())
        .expect(
            "bounded seed with Chain Magic, printed Spellcaster at C4, nearby South minion at C3, and distant South Avatar at C1 for extend",
        );
    let (mut session, chain_id, caster_id, south_minion_id, south_avatar_id) =
        try_setup_spellcaster_distant_avatar_hop(&encoded)
            .expect("spellcaster distant avatar hop setup");
    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["casterInstanceId"] == caster_id
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["seat"] == "south"
            && descriptor["target"]["instanceId"] == south_minion_id
    });
    assert!(begin.events.is_empty());
    let staged = state(&session);
    assert_eq!(staged["phase"], "chain-magic");
    assert_eq!(
        staged["pendingChainMagic"]["targets"],
        json!([{
            "instanceId": south_minion_id,
            "kind": "minion",
            "seat": "south",
        }])
    );
    let extensions = extend_ids(&session);
    assert!(
        !extensions.contains(&south_avatar_id),
        "extend-chain-magic must omit the distant enemy Avatar as a second hop"
    );
    assert!(
        !extensions.contains(&south_minion_id),
        "extend-chain-magic must omit the already-staged minion hop"
    );
    assert_exact_replay(&session);
}

fn setup_hops_short(encoded: &str, skip_last_site: bool) -> ChainHops {
    let mut session = opening_main(encoded);
    let (first, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally-a"
            && descriptor["cell"] == "C4"
    });
    let (second, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-ally-b"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    if !skip_last_site {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
        });
    }
    let before = state(&session);
    ChainHops {
        avatar_id: before["players"]["north"]["avatar"]["card"]["instanceId"]
            .as_str()
            .expect("North Avatar identity")
            .to_owned(),
        chain_id: hand_instance(&before, "north-chain"),
        first_id: first["cardInstanceId"]
            .as_str()
            .expect("first hop identity")
            .to_owned(),
        mana: before["players"]["north"]["mana"]
            .as_u64()
            .expect("North mana"),
        second_id: second["cardInstanceId"]
            .as_str()
            .expect("second hop identity")
            .to_owned(),
        session,
    }
}

fn try_setup_deathrite_chain(encoded: &str) -> Option<(Session, String, String)> {
    if !opening_has_all(encoded, &["north-chain"]) {
        return None;
    }
    let mut session = opening_main(encoded);
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    })?;
    let (summoned, _) = try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-deathrite"
            && descriptor["cell"] == "C4"
            && descriptor["region"].is_null()
    })?;
    try_accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn")?;
    try_accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    })?;
    let before = state(&session);
    let chain_id = try_hand_instance(&before, "north-chain")?;
    let deathrite_id = summoned["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned();
    if !chain_ids(&session, &chain_id).contains(&deathrite_id) {
        return None;
    }
    Some((session, chain_id, deathrite_id))
}

#[test]
fn rule_catalog_0984_resolve_chain_magic_magic_deathrite_draws_before_magic_resolved() {
    let encoded = (984..984 + 256)
        .map(deathrite_chain_manifest)
        .find(|candidate| try_setup_deathrite_chain(candidate).is_some())
        .expect("bounded seed with Chain Magic and nearby South Deathrite");
    let (mut session, chain_id, deathrite_id) =
        try_setup_deathrite_chain(&encoded).expect("deathrite chain setup");
    let before = state(&session);
    let south_atlas = atlas_len(&before, "south");
    assert_eq!(
        realm_unit(&before, &deathrite_id).expect("Deathrite on board")["controller"],
        "south"
    );

    let (_, begin) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == deathrite_id
    });
    assert!(begin.events.is_empty());
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "resolve-chain-magic"
    });
    assert_eq!(
        event_types(&resolved),
        [
            "magic-cast",
            "magic-damage-allocated",
            "damage-dealt",
            "site-drawn",
            "minion-died",
            "magic-resolved"
        ]
    );
    assert_eq!(
        resolved.events[1].payload,
        json!({
            "amount": 2,
            "sourceInstanceId": chain_id,
            "targetInstanceId": deathrite_id,
        })
    );
    let drawn = resolved
        .events
        .iter()
        .find(|event| event.event_type == "site-drawn")
        .expect("Deathrite site draw");
    assert_eq!(drawn.payload["seat"], "south");
    assert_eq!(drawn.payload["sourceInstanceId"], deathrite_id);
    let site_drawn = event_types(&resolved)
        .iter()
        .position(|event_type| *event_type == "site-drawn")
        .expect("site-drawn index");
    let magic_resolved = event_types(&resolved)
        .iter()
        .position(|event_type| *event_type == "magic-resolved")
        .expect("magic-resolved index");
    assert!(
        site_drawn < magic_resolved,
        "magic-resolved must follow deathrite site-drawn"
    );
    assert_eq!(event_types(&resolved).last(), Some(&"magic-resolved"));

    let finished = state(&session);
    assert_eq!(finished["phase"], "main");
    assert!(finished["pendingChainMagic"].is_null());
    assert!(realm_unit(&finished, &deathrite_id).is_none());
    assert_eq!(atlas_len(&finished, "south"), south_atlas - 1);
    assert!(cemetery_has(&finished, "south", &deathrite_id));
    assert!(!cemetery_has(&finished, "north", &deathrite_id));
    assert_exact_replay(&session);
}
