use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
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

fn site(discard_top_spells: bool) -> Value {
    let mut value = json!({
        "cardType": "site",
        "elements": ["earth"],
    });
    if discard_top_spells {
        value["genesisDiscardTopSpells"] = json!(2);
    }
    value
}

fn minion(extra: Value) -> Value {
    let mut value = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    });
    let Value::Object(extra) = extra else {
        panic!("extra minion facts must be an object");
    };
    value.as_object_mut().expect("minion facts").extend(extra);
    value
}

fn magic(effect: (&str, Value), mana_cost: u8) -> Value {
    let mut value = json!({
        "cardType": "magic",
        "manaCost": mana_cost,
        "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
    });
    value
        .as_object_mut()
        .expect("Magic facts")
        .insert(effect.0.to_owned(), effect.1);
    value
}

fn finish_manifest(mut value: Value) -> String {
    value["manifestId"] =
        json!(identity_hash(&value).expect("canonical synthetic manifest identity"));
    canonical_json(&value).expect("canonical synthetic manifest")
}

fn manifest(
    seed: u32,
    cards: &Value,
    north_spellbook: &[&str],
    south_spellbook: &[&str],
) -> String {
    finish_manifest(json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "magic-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-magic-rules-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": south_spellbook,
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

fn opening_main(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid Magic scenario");
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
#[expect(
    clippy::too_many_lines,
    reason = "one Chain Magic filter proof keeps low-mana and underground hop exclusion together"
)]
fn chain_magic_should_require_mana_and_same_region_hops() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-burrower": minion(json!({
            "burrowing": true,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "north-chain": {
            "cardType": "magic",
            "damageChainNearbyUnits": true,
            "manaCost": 2,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        },
        "north-site": site(false),
        "north-target": minion(json!({
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "south-avatar": avatar(20),
        "south-minion": minion(json!({
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-chain",
        "north-target",
        "north-burrower",
        "north-chain",
        "north-target",
        "north-burrower",
    ];
    let chosen = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-minion"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("Chain Magic filter candidate");
            let preview_state = state(&preview);
            let hand = preview_state["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand");
            ["north-chain", "north-target", "north-burrower"]
                .into_iter()
                .all(|card_id| hand.iter().any(|card| card["cardId"] == card_id))
        })
        .expect("seed with Chain Magic and both minions");
    let mut session = opening_main(&chosen);
    let chain_id = state(&session)["players"]["north"]["hand"]["spellbook"]
        .as_array()
        .expect("North hand")
        .iter()
        .find(|card| card["cardId"] == "north-chain")
        .expect("Chain Magic in hand")["instanceId"]
        .as_str()
        .expect("Chain Magic identity")
        .to_owned();
    assert_eq!(state(&session)["players"]["north"]["mana"], 1);
    assert!(
        session
            .legal_actions()
            .expect("low-mana actions")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "begin-chain-magic"
                    || action.descriptor["cardInstanceId"] != chain_id
            })
    );

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-target"
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
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-burrower"
            && descriptor["cell"] == "C3"
            && descriptor["region"] == "underground"
    });

    let target_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-target")
        .expect("surface target")["instanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    let burrower_id = state(&session)["realm"]["units"]
        .as_array()
        .expect("units")
        .iter()
        .find(|unit| unit["cardId"] == "north-burrower")
        .expect("burrower")["instanceId"]
        .as_str()
        .expect("burrower identity")
        .to_owned();
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "begin-chain-magic"
            && descriptor["cardInstanceId"] == chain_id
            && descriptor["target"]["instanceId"] == target_id
    });
    assert!(
        session
            .legal_actions()
            .expect("staged Chain Magic")
            .iter()
            .all(|action| {
                action.descriptor["kind"] != "extend-chain-magic"
                    || action.descriptor["target"]["instanceId"] != burrower_id
            })
    );
}

fn cast_bury(session: &mut Session, target_id: &str, spell_id: &str) -> Receipt {
    let (_, receipt) = accept_where(session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["cardInstanceId"] == spell_id
            && descriptor["target"]["instanceId"] == target_id
    });
    receipt
}

fn realm_unit<'a>(value: &'a Value, instance_id: &str) -> Option<&'a Value> {
    value["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == instance_id)
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one ordered Bury proof retains setup, deferred state, checkpoint, completion, and replay"
)]
fn bury_should_defer_completion_until_ordered_static_deathrites_finish() {
    let cards = json!({
        "north-avatar": avatar(20),
        "north-bury": magic(("burrowTargetMinionOrArtifact", json!(true)), 0),
        "north-pinger": minion(json!({
            "defense": 10,
            "genesisDamageEachOtherUnitHere": 1,
            "summonToAnySite": true,
        })),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-buff": minion(json!({
            "burrowing": true,
            "deathriteDrawSite": true,
            "otherNearbyAlliesPowerBonus": 1,
        })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-bury",
        "north-bury",
        "north-pinger",
        "north-bury",
        "north-bury",
        "north-pinger",
    ];
    let manifest = (1..=512)
        .map(|seed| manifest(seed, &cards, &north_spellbook, &["south-buff"; 6]))
        .find(|candidate| {
            let preview = Session::new(candidate).expect("ordered Bury candidate");
            let hand = state(&preview)["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North opening hand")
                .clone();
            hand.iter().any(|card| card["cardId"] == "north-bury")
                && hand.iter().any(|card| card["cardId"] == "north-pinger")
        })
        .expect("bounded seed with Bury and a pinger");
    let mut session = opening_main(&manifest);
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

    let cast = cast_bury(&mut session, &buff_ids[0], &spell_id);
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

struct DuelSetup {
    normal_id: String,
    session: Session,
}

fn summon_duel_minion(
    session: &mut Session,
    card_id: &str,
    cell: &str,
    region: Option<&str>,
) -> String {
    let (descriptor, _) = accept_where(session, |candidate| {
        candidate["kind"] == "summon-minion"
            && candidate["cardId"] == card_id
            && candidate["cell"] == cell
            && region.map_or_else(
                || candidate["region"].is_null(),
                |expected| candidate["region"] == expected,
            )
    });
    descriptor["cardInstanceId"]
        .as_str()
        .expect("summoned Duel minion identity")
        .to_owned()
}

#[expect(
    clippy::too_many_lines,
    reason = "one staged checkpoint keeps Duel geometry and branch comparisons on the same position"
)]
fn duel_checkpoint() -> DuelSetup {
    let cards = json!({
        "north-ally": minion(json!({ "attack": 3, "defense": 4 })),
        "north-avatar": avatar(20),
        "north-bury": magic(("burrowTargetMinionOrArtifact", json!(true)), 0),
        "north-caster": minion(json!({ "burrowing": true, "spellcaster": true })),
        "north-duel": magic(("fightAllyWithAdjacentEnemy", json!(true)), 1),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-diagonal": minion(json!({ "attack": 2, "defense": 3, "summonToAnySite": true })),
        "south-disabled": minion(json!({
            "attack": 2,
            "defense": 3,
            "summonToAnySite": true,
            "waterbound": true,
        })),
        "south-normal": minion(json!({ "attack": 2, "defense": 3, "summonToAnySite": true })),
        "south-site": site(false),
        "south-stealthed": minion(json!({
            "attack": 2,
            "defense": 3,
            "stealth": true,
            "summonToAnySite": true,
        })),
        "south-warded": minion(json!({
            "attack": 2,
            "defense": 3,
            "summonToAnySite": true,
            "ward": true,
        })),
        "south-wrong-region": minion(json!({
            "attack": 2,
            "burrowing": true,
            "defense": 3,
            "summonToAnySite": true,
        })),
    });
    let north_spellbook = [
        "north-duel",
        "north-ally",
        "north-caster",
        "north-bury",
        "north-bury",
        "north-bury",
    ];
    let south_spellbook = [
        "south-normal",
        "south-warded",
        "south-stealthed",
        "south-disabled",
        "south-wrong-region",
        "south-diagonal",
    ];

    for seed in 1..=128 {
        let manifest = manifest(seed, &cards, &north_spellbook, &south_spellbook);
        let mut session = Session::new(&manifest).expect("valid Duel scenario");
        keep(&mut session);
        keep(&mut session);
        let opening = state(&session);
        let north_hand = opening["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North opening spellbook hand");
        if !["north-duel", "north-ally", "north-caster"]
            .into_iter()
            .all(|card_id| north_hand.iter().any(|card| card["cardId"] == card_id))
        {
            continue;
        }

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
        });
        let _ally_id = summon_duel_minion(&mut session, "north-ally", "C4", None);
        let caster_id = summon_duel_minion(&mut session, "north-caster", "C4", None);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        for south_site in ["C2", "C3"] {
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw"
                    && descriptor["zone"]
                        == if south_site == "C2" {
                            "spellbook"
                        } else {
                            "atlas"
                        }
            });
            if south_site == "C2" {
                accept_where(&mut session, |descriptor| {
                    descriptor["kind"] == "cast-magic"
                        && descriptor["cardId"] == "north-bury"
                        && descriptor["target"]["instanceId"] == caster_id
                });
            }
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == south_site
            });
            if south_site == "C2" {
                accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            }
        }

        let normal_id = summon_duel_minion(&mut session, "south-normal", "C3", None);
        let _warded_id = summon_duel_minion(&mut session, "south-warded", "C4", None);
        let _stealthed_id = summon_duel_minion(&mut session, "south-stealthed", "C4", None);
        let _disabled_id = summon_duel_minion(&mut session, "south-disabled", "C4", None);
        let wrong_region_id = summon_duel_minion(&mut session, "south-wrong-region", "C4", None);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == "north-bury"
                && descriptor["target"]["instanceId"] == wrong_region_id
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "B3"
        });
        let _diagonal_id = summon_duel_minion(&mut session, "south-diagonal", "B3", None);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });

        return DuelSetup { normal_id, session };
    }
    panic!("no deterministic Duel opening found");
}

#[test]
fn duel_should_apply_an_avatar_ally_from_the_existing_checkpoint() {
    let setup = duel_checkpoint();
    let north_avatar_id = state(&setup.session)["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("North Avatar identity")
        .to_owned();
    let mut session = setup.session;
    let (descriptor, fight) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "cast-magic"
            && descriptor["ally"]["instanceId"] == north_avatar_id
            && descriptor["target"]["instanceId"] == setup.normal_id
    });
    assert_eq!(descriptor["ally"]["kind"], "avatar");
    assert_eq!(descriptor["target"]["kind"], "minion");
    assert_eq!(
        event_types(&fight),
        [
            "magic-cast",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "avatar-life-lost",
            "damage-dealt",
            "magic-resolved",
        ]
    );
    assert_eq!(state(&session)["players"]["north"]["avatar"]["life"], 18);
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one pending Duel proof keeps lower-region combat, first strike, ordered Deathrites, terminal completion, checkpoint, and replay together"
)]
fn duel_should_checkpoint_an_underground_first_strike_and_finish_before_terminal() {
    let cards = json!({
        "north-ally": minion(json!({
            "attack": 2,
            "burrowing": true,
            "defense": 10,
            "strikesFirstWhileAttacking": true,
        })),
        "north-avatar": avatar(20),
        "north-burrow": magic(("burrowAllMinionsAndArtifactsAtTargetLandSite", json!(true)), 0),
        "north-duel": magic(("fightAllyWithAdjacentEnemy", json!(true)), 0),
        "north-filler": minion(json!({})),
        "north-pinger": minion(json!({
            "defense": 10,
            "genesisDamageEachOtherUnitHere": 1,
        })),
        "north-site": site(false),
        "south-avatar": avatar(20),
        "south-buff": minion(json!({
            "burrowing": true,
            "deathriteDrawSite": true,
            "otherNearbyAlliesPowerBonus": 1,
            "summonToAnySite": true,
        })),
        "south-site": site(false),
    });
    let north_spellbook = [
        "north-ally",
        "north-pinger",
        "north-burrow",
        "north-duel",
        "north-filler",
        "north-filler",
    ];
    let mut prepared = None;
    for seed in 1..=512 {
        let manifest = manifest(seed, &cards, &north_spellbook, &["south-buff"; 6]);
        let mut session = Session::new(&manifest).expect("valid pending Duel candidate");
        keep(&mut session);
        keep(&mut session);
        let opening = state(&session);
        let hand = opening["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North opening hand");
        if !hand.iter().any(|card| card["cardId"] == "north-ally") {
            continue;
        }
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
        });
        let ally_id = summon_duel_minion(&mut session, "north-ally", "C4", None);
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
        });
        let mut target_ids = Vec::new();
        for _ in 0..2 {
            target_ids.push(summon_duel_minion(&mut session, "south-buff", "C4", None));
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        let drawn = state(&session);
        let north_hand = drawn["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("North hand after one draw")
            .iter()
            .filter_map(|card| card["cardId"].as_str())
            .collect::<Vec<_>>();
        if !["north-pinger", "north-burrow", "north-duel"]
            .into_iter()
            .all(|card_id| north_hand.contains(&card_id))
        {
            continue;
        }
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        summon_duel_minion(&mut session, "north-pinger", "C4", None);
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == "north-burrow"
                && descriptor["targetLocation"]["cell"] == "C4"
        });
        let pre_duel = state(&session);
        assert!(target_ids.iter().all(|instance_id| {
            realm_unit(&pre_duel, instance_id).is_some_and(|unit| unit["damage"] == 1)
        }));
        prepared = Some((session, ally_id, target_ids));
        break;
    }
    let (mut session, ally_id, mut target_ids) =
        prepared.expect("bounded seed with the complete pending Duel setup");
    target_ids.sort_unstable();
    let target_id = target_ids[0].clone();
    let duel_action = session
        .legal_actions()
        .expect("underground Duel actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "north-duel"
                && action.descriptor["ally"]["instanceId"] == ally_id
                && action.descriptor["target"]["instanceId"] == target_id
        })
        .expect("engine-issued underground Duel");
    let StepResult::Accepted(cast) = session
        .step(ActionRequest {
            action_id: duel_action.action_id.to_string(),
            seat: duel_action.seat,
            state_version: duel_action.state_version,
        })
        .expect("underground Duel cast")
    else {
        panic!("engine-issued underground Duel must be accepted");
    };
    assert_eq!(
        event_types(&cast),
        [
            "magic-cast",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
        ]
    );
    let pending = state(&session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(pending["decisionSeat"], "south");
    assert_eq!(
        pending["pendingDeathrites"]["continuation"]["kind"],
        "first-strike"
    );
    assert_eq!(
        pending["pendingDeathrites"]["continuation"]["pending"]["region"],
        "underground"
    );
    assert_eq!(
        pending["pendingDeathrites"]["deferredOutcomes"],
        json!([{
            "payload": {
                "cardId": "north-duel",
                "instanceId": cast.events[0].payload["instanceId"],
                "owner": "north",
            },
            "type": "magic-resolved",
        }])
    );

    let checkpoint = create_game_checkpoint(&session).expect("pending Duel checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized pending Duel");
    session = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed pending Duel checkpoint"),
    )
    .expect("resumed pending Duel checkpoint");
    assert_eq!(state(&session), pending);
    let (_, completed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == target_ids[0]
    });
    let completed_types = event_types(&completed);
    assert_eq!(
        &completed_types[completed_types.len() - 2..],
        ["magic-resolved", "game-ended"]
    );
    assert_eq!(state(&session)["phase"], "terminal");
    assert_exact_replay(&session);
}
