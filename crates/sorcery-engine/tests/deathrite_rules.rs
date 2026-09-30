use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, Seat};
use sorcery_engine::game::{Game, GameError};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

#[path = "common/marked_death.rs"]
mod marked_death;

struct AttackSetup {
    attacker_id: String,
    defender_id: String,
    session: Session,
    target_id: String,
}

fn finish_manifest(mut manifest: Value, revision: &str) -> String {
    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("manifestId");
    manifest["authority"]["revisionId"] = json!(revision);
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("synthetic manifest identity"));
    canonical_json(&manifest).expect("canonical synthetic manifest")
}

fn base_manifest(seed: u32) -> Value {
    serde_json::from_str(&synthetic_demo_manifest_json(seed).expect("public synthetic manifest"))
        .expect("manifest value")
}

fn minion(definition: &Value) -> Value {
    let mut facts = json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": 0,
        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
    });
    facts
        .as_object_mut()
        .expect("minion facts")
        .extend(definition.as_object().expect("fact overrides").clone());
    facts
}

fn set_all_minions(manifest: &mut Value, seat: &str, definition: &Value) {
    let prefix = format!("{seat}-spell-");
    for (card_id, card) in manifest["cards"].as_object_mut().expect("manifest cards") {
        if card_id.starts_with(&prefix) {
            *card = minion(definition);
        }
    }
}

fn combat_manifest(seed: u32, north: &Value, south: &Value, avatar_life: u8) -> String {
    let mut manifest = base_manifest(seed);
    set_all_minions(&mut manifest, "north", north);
    set_all_minions(&mut manifest, "south", south);
    manifest["cards"]["north-avatar"]["life"] = json!(avatar_life);
    manifest["cards"]["south-avatar"]["life"] = json!(avatar_life);
    finish_manifest(manifest, "synthetic-deathrite-combat-v1")
}

fn empty_atlas_combat_manifest(seed: u32, facts: &Value) -> String {
    let mut manifest = base_manifest(seed);
    set_all_minions(&mut manifest, "north", facts);
    set_all_minions(&mut manifest, "south", facts);
    for seat in ["north", "south"] {
        manifest["decks"][seat]["atlas"]
            .as_array_mut()
            .expect("Atlas array")
            .truncate(3);
    }
    let mut referenced = Vec::new();
    for seat in ["north", "south"] {
        referenced.push(
            manifest["decks"][seat]["avatar"]
                .as_str()
                .expect("Avatar card ID")
                .to_owned(),
        );
        for zone in ["atlas", "spellbook"] {
            referenced.extend(
                manifest["decks"][seat][zone]
                    .as_array()
                    .expect("deck zone")
                    .iter()
                    .map(|card_id| card_id.as_str().expect("deck card ID").to_owned()),
            );
        }
    }
    manifest["cards"]
        .as_object_mut()
        .expect("manifest cards")
        .retain(|card_id, _| referenced.contains(card_id));
    finish_manifest(manifest, "synthetic-empty-atlas-deathrite-v1")
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
    session.replay_value().expect("authoritative replay value")["state"].clone()
}

fn assert_exact_replay(session: &Session) {
    let action_ids: Vec<IdentityHash> = session
        .transcript()
        .iter()
        .map(|receipt| receipt.action_id.clone())
        .collect();
    let replayed = Session::replay(session.manifest_json(), &action_ids).expect("exact replay");
    assert_eq!(replayed.transcript(), session.transcript());
    assert_eq!(
        replayed.replay_value().expect("replayed value"),
        session.replay_value().expect("session value")
    );
    assert!(session.verify_replay().expect("verified replay"));
}

fn assert_checkpoint_round_trip(session: &Session) {
    let checkpoint = create_game_checkpoint(session).expect("captured Deathrite checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let parsed = parse_game_checkpoint(&serialized).expect("parsed checkpoint");
    let restored = resume_game_checkpoint(&parsed).expect("restored checkpoint");
    assert_eq!(state(&restored), state(session));
    assert_eq!(
        restored.legal_actions().expect("restored actions"),
        session.legal_actions().expect("source actions")
    );
}

fn event_types(receipt: &Receipt) -> Vec<&str> {
    receipt
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "fixed terminal matrix keeps explicit seat, life, deck, and body parameters"
)]
fn fixed_terminal_manifest(
    first_seat: &str,
    north_life: u64,
    south_life: u64,
    north_atlas: usize,
    south_atlas: usize,
    body_facts: &Value,
    area_kind: Option<&str>,
    include_primer: bool,
) -> String {
    let mut area_query = json!({"area":{"realm":{}},"controller":"any","excludeSource":false});
    if let Some(kind) = area_kind {
        area_query["kind"] = json!(kind);
    }
    let mut cards = json!({
        "north-avatar": {"attack":1,"cardType":"avatar","defense":1,"drawSpell":false,"life":north_life},
        "south-avatar": {"attack":1,"cardType":"avatar","defense":1,"drawSpell":false,"life":south_life},
        "site": {"cardType":"site","elements":["earth"]},
        "victim": minion(body_facts),
        "area": {"cardType":"magic","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"effectProgram":{"effects":[
            {"op":"damage","amount":1,"recipients":{"query":area_query}},
            {"op":"draw","zone":"atlas","count":1}
        ]}},
        "filler": minion(&json!({})),
    });
    if include_primer {
        cards["primer"] = json!({"cardType":"magic","manaCost":0,"thresholds":{"air":0,"earth":0,"fire":0,"water":0},"effectProgram":{"effects":[
            {"op":"damage","amount":1,"recipients":{"query":{"area":{"realm":{}},"controller":"any","excludeSource":false,"kind":"avatar"}}}
        ]}});
    }
    let first_cards = if include_primer {
        vec!["victim", "primer", "area"]
    } else {
        vec!["victim", "victim", "area"]
    };
    let (north_spells, south_spells) = if first_seat == "north" {
        (first_cards, vec!["filler", "filler", "filler"])
    } else {
        (vec!["filler", "filler", "filler"], first_cards)
    };
    finish_manifest(
        json!({
            "authority":{"contentHash":identity_hash(&json!({"fixture":"accepted-terminal-current-event-proof"})).expect("fixture identity"),"mode":"synthetic","revisionId":"synthetic-terminal-current-event-proof-v1"},
            "cards":cards,
            "decks":{
                "north":{"atlas":vec!["site";north_atlas],"avatar":"north-avatar","spellbook":north_spells},
                "south":{"atlas":vec!["site";south_atlas],"avatar":"south-avatar","spellbook":south_spells}
            },
            "engineVersion":"sorcery-core-v1","firstSeat":first_seat,"schemaVersion":1,"seed":9401
        }),
        "synthetic-terminal-current-event-proof-v1",
    )
}

fn prepare_terminal_area(
    encoded: &str,
    first_seat: &str,
    body_count: usize,
    primer: bool,
    later_turn: bool,
) -> Session {
    let mut session = Session::new(encoded).expect("valid accepted terminal fixture");
    keep(&mut session);
    keep(&mut session);
    let first_cell = if first_seat == "north" { "C4" } else { "C1" };
    let other_cell = if first_seat == "north" { "C1" } else { "C4" };
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == first_cell
    });
    for _ in 0..body_count {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "victim"
                && descriptor["cell"] == first_cell
        });
    }
    if primer {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic" && descriptor["cardId"] == "primer"
        });
        if later_turn {
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
            });
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "play-site" && descriptor["cell"] == other_cell
            });
            accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
            accept_where(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
            });
        }
    }
    assert_eq!(
        session.acting_controller(),
        if first_seat == "north" {
            Seat::North
        } else {
            Seat::South
        }
    );
    session
}

fn replay_game_for_session(session: &Session) -> Game {
    let mut game = Game::from_manifest_json(session.manifest_json()).expect("replay Game fixture");
    for receipt in session.transcript() {
        let action = game
            .legal_actions()
            .expect("Ignore-path legal actions")
            .into_iter()
            .find(|action| {
                action
                    .to_legal_action()
                    .expect("Ignore-path action")
                    .action_id
                    == receipt.action_id
            })
            .expect("accepted Session action exists in Ignore path");
        game.apply_action(&action)
            .expect("Ignore-path accepted action");
    }
    game
}

fn north_attacks_at_c2(manifest: &str) -> AttackSetup {
    let mut session = Session::new(manifest).expect("valid Deathrite combat scenario");
    keep(&mut session);
    keep(&mut session);

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let attacker_id = summon["cardInstanceId"]
        .as_str()
        .expect("attacker identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
    });
    let defender_id = summon["cardInstanceId"]
        .as_str()
        .expect("distant defender identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["to"]["cell"] == "C3"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "decline-attack"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    let (summon, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C2"
    });
    let target_id = summon["cardInstanceId"]
        .as_str()
        .expect("target identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");

    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == attacker_id
            && descriptor["from"]["cell"] == "C3"
            && descriptor["to"]["cell"] == "C2"
    });

    AttackSetup {
        attacker_id,
        defender_id,
        session,
        target_id,
    }
}

fn resolve_minion_fight(manifest: &str) -> (AttackSetup, Receipt) {
    let mut setup = north_attacks_at_c2(manifest);
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.target_id
    });
    let (_, receipt) = accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    (setup, receipt)
}

fn card_ids_in_hand(session: &Session, seat: &str, count: usize) -> Vec<String> {
    state(session)["players"][seat]["hand"]["spellbook"]
        .as_array()
        .expect("Spellbook hand")
        .iter()
        .take(count)
        .map(|card| card["cardId"].as_str().expect("card identity").to_owned())
        .collect()
}

fn unit_ids_for_cards(session: &Session, card_ids: &[String]) -> Vec<String> {
    let mut ids: Vec<_> = state(session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| card_ids.iter().any(|card_id| unit["cardId"] == *card_id))
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("unit identity")
                .to_owned()
        })
        .collect();
    ids.sort();
    ids
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one simultaneous Deathrite order and cemetery sequence proof"
)]
fn rule_catalog_0776_simultaneous_deathrites_resolve_nap_then_ap_before_cemetery() {
    let facts = json!({ "deathriteDrawSite": true });
    let manifest = combat_manifest(48, &facts, &facts, 20);
    let (setup, receipt) = resolve_minion_fight(&manifest);
    let draws: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "site-drawn")
        .map(|event| event.payload.clone())
        .collect();
    let first_death = event_types(&receipt)
        .iter()
        .position(|kind| *kind == "minion-died")
        .expect("simultaneous deaths");
    let last_draw = event_types(&receipt)
        .iter()
        .rposition(|kind| *kind == "site-drawn")
        .expect("Deathrite draws");

    assert_eq!(
        draws,
        [
            json!({ "seat": "south", "sourceInstanceId": setup.target_id }),
            json!({ "seat": "north", "sourceInstanceId": setup.attacker_id }),
        ]
    );
    assert!(last_draw < first_death);
    assert_eq!(
        state(&setup.session)["players"]["north"]["cemetery"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        state(&setup.session)["players"]["south"]["cemetery"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_exact_replay(&setup.session);

    let empty_manifest = empty_atlas_combat_manifest(49, &facts);
    let mut deck_out = north_attacks_at_c2(&empty_manifest);
    accept_where(&mut deck_out.session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == deck_out.target_id
    });
    let action = deck_out
        .session
        .legal_actions()
        .expect("final defense actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "close-defend"
                && action.descriptor["originalTargetParticipates"] == true
        })
        .expect("engine-issued lethal attack action");
    let before_view = deck_out
        .session
        .public_view(Seat::North)
        .expect("pre-attempt view");
    let before_hash = deck_out.session.state_hash().expect("pre-attempt hash");
    assert_checkpoint_round_trip(&deck_out.session);
    let before_transcript = deck_out.session.transcript().to_vec();
    let before_attempts = deck_out.session.attempts().to_vec();
    let StepResult::Accepted(receipt) = deck_out
        .session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .expect("the first failed Atlas draw ends the game")
    else {
        panic!("engine-issued Deathrite order must resolve");
    };
    assert_ne!(
        deck_out.session.public_view(Seat::North).unwrap(),
        before_view
    );
    assert_ne!(deck_out.session.state_hash().unwrap(), before_hash);
    assert_ne!(deck_out.session.transcript(), before_transcript);
    assert!(deck_out.session.attempts().len() > before_attempts.len());
    assert!(deck_out.session.unsupported_mechanic().is_none());
    assert_eq!(
        event_types(&receipt),
        [
            "defend-window-closed",
            "fight-started",
            "strike-damage-allocated",
            "damage-dealt",
            "damage-dealt",
            "game-ended",
        ]
    );
    assert_eq!(
        state(&deck_out.session)["terminal"],
        json!({"status":"finished","winner":"north","loser":"south","reason":"deck_empty"})
    );
    marked_death::assert_live_marked_before_cemetery(
        &state(&deck_out.session),
        &[deck_out.attacker_id.clone(), deck_out.target_id.clone()],
    );
    assert!(
        !event_types(&receipt).iter().any(|kind| [
            "site-drawn",
            "minion-died",
            "simultaneous-defeat"
        ]
        .contains(kind))
    );
    assert_checkpoint_round_trip(&deck_out.session);
    assert_exact_replay(&deck_out.session);
}

#[test]
fn rule_catalog_0787_bladderblimp_deathrite_counts_nearby_controlled_sites() {
    let manifest = combat_manifest(
        147,
        &json!({
            "airborne": true,
            "deathriteLoseLifePerNearbySiteControlled": 1,
        }),
        &json!({}),
        2,
    );
    let (setup, receipt) = resolve_minion_fight(&manifest);
    let life_events: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.event_type == "avatar-life-lost")
        .map(|event| event.payload.clone())
        .collect();

    assert_eq!(
        life_events,
        [
            json!({
                "amount": 1,
                "life": 1,
                "seat": "north",
                "sourceInstanceId": setup.attacker_id,
            }),
            json!({
                "amount": 2,
                "life": 0,
                "seat": "south",
                "sourceInstanceId": setup.attacker_id,
            }),
        ]
    );
    assert_eq!(
        state(&setup.session)["terminal"],
        json!({ "status": "active" })
    );
    let deaths_door = receipt
        .events
        .iter()
        .position(|event| event.event_type == "avatar-reached-deaths-door")
        .expect("South reaches Death's Door");
    assert_eq!(
        receipt.events[deaths_door].payload,
        json!({
            "seat": "south",
            "sourceInstanceId": setup.attacker_id,
            "turnNumber": state(&setup.session)["turnNumber"],
        })
    );
    let first_death = receipt
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .expect("combat death");
    assert!(deaths_door < first_death);
    assert_exact_replay(&setup.session);

    let mut invalid = base_manifest(147);
    invalid["cards"]["north-spell-1"]["deathriteLoseLifePerNearbySiteControlled"] = json!(2);
    let error = Session::new(&finish_manifest(invalid, "invalid-bladderblimp"))
        .expect_err("invalid fixed-value fact");
    assert!(
        error
            .to_string()
            .contains("deathriteLoseLifePerNearbySiteControlled")
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves last-location Ward, reduction, and Lethal branches"
)]
fn rule_catalog_0878_deathrite_area_damage_uses_last_location_ward_reduction_and_lethal() {
    let ward_manifest = combat_manifest(
        266,
        &json!({ "attack": 0, "deathriteDamageEachUnitHere": 2 }),
        &json!({ "attack": 2, "defense": 10, "ward": true }),
        20,
    );
    let (warded, ward_receipt) = resolve_minion_fight(&ward_manifest);
    let ward_allocation = ward_receipt
        .events
        .iter()
        .position(|event| event.event_type == "deathrite-damage-allocated")
        .expect("Deathrite allocation");
    assert_eq!(
        ward_receipt.events[ward_allocation].payload,
        json!({
            "amount": 2,
            "sourceInstanceId": warded.attacker_id,
            "targetInstanceId": warded.target_id,
        })
    );
    let ward_damage = ward_receipt.events[ward_allocation + 1..]
        .iter()
        .find(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == warded.target_id
        })
        .expect("Ward prevents Deathrite damage");
    assert_eq!(
        ward_damage.payload,
        json!({
            "amount": 0,
            "attemptedAmount": 2,
            "direct": true,
            "instanceId": warded.target_id,
            "prevented": true,
            "seat": "south",
        })
    );
    assert!(
        ward_receipt
            .events
            .iter()
            .position(|event| event.event_type == "ward-broken")
            .expect("Ward breaks")
            > ward_allocation
    );
    let source_moves: Vec<_> = warded
        .session
        .transcript()
        .iter()
        .flat_map(|receipt| &receipt.events)
        .filter(|event| {
            event.event_type == "move-and-attack-activated"
                && event.payload["unitInstanceId"] == warded.attacker_id
        })
        .collect();
    assert_eq!(source_moves.len(), 2);
    assert_eq!(source_moves[1].payload["to"]["cell"], "C2");
    let ward_target = state(&warded.session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == warded.target_id)
        .cloned()
        .expect("warded target survives");
    let distant = state(&warded.session)["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == warded.defender_id)
        .cloned()
        .expect("distant defender survives");
    assert_eq!(
        json!({
            "targetDamage": ward_target["damage"],
            "targetWard": ward_target["warded"],
            "distantDamage": distant["damage"],
            "distantWard": distant["warded"],
        }),
        json!({
            "targetDamage": 0,
            "targetWard": false,
            "distantDamage": 0,
            "distantWard": true,
        })
    );
    assert_exact_replay(&warded.session);

    let lethal_manifest = combat_manifest(
        267,
        &json!({
            "attack": 0,
            "deathriteDamageEachUnitHere": 2,
            "lethal": true,
        }),
        &json!({ "attack": 2, "defense": 10, "takesLessDamage": 1 }),
        20,
    );
    let (lethal, lethal_receipt) = resolve_minion_fight(&lethal_manifest);
    let lethal_allocation = lethal_receipt
        .events
        .iter()
        .position(|event| event.event_type == "deathrite-damage-allocated")
        .expect("Lethal Deathrite allocation");
    let lethal_damage = lethal_receipt.events[lethal_allocation + 1..]
        .iter()
        .find(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == lethal.target_id
        })
        .expect("reduced Lethal damage");
    assert_eq!(
        lethal_damage.payload,
        json!({
            "accumulated": 1,
            "amount": 1,
            "attemptedAmount": 2,
            "direct": true,
            "instanceId": lethal.target_id,
            "prevented": true,
            "seat": "south",
        })
    );
    assert!(
        lethal_receipt
            .events
            .iter()
            .position(|event| {
                event.event_type == "minion-died" && event.payload["instanceId"] == lethal.target_id
            })
            .expect("Lethal kills target")
            > lethal_allocation
    );
    assert!(
        state(&lethal.session)["players"]["south"]["cemetery"]
            .as_array()
            .expect("South cemetery")
            .iter()
            .any(|card| card["instanceId"] == lethal.target_id)
    );
    assert_exact_replay(&lethal.session);
}

#[test]
fn rule_catalog_0794_deathrite_damage_preserves_source_power_until_resolution() {
    let mut raw = base_manifest(171);
    // The tower turns base 2/0 into 4/2: general power 3 must survive in the Deathrite snapshot.
    set_all_minions(
        &mut raw,
        "north",
        &json!({
            "attack": 2,
            "deathriteDamageEachUnitHere": 1,
            "defense": 0,
            "gainsPowerRangedAndSpellcasterAtopTower": 2,
        }),
    );
    set_all_minions(
        &mut raw,
        "south",
        &json!({
            "attack": 2,
            "defense": 10,
            "preventsDamageFromUnitsWithPowerAtLeast": 3,
        }),
    );
    for card in raw["cards"]
        .as_object_mut()
        .expect("manifest cards")
        .values_mut()
    {
        if card["cardType"] == "site" {
            card["isTower"] = json!(true);
        }
    }
    let manifest = finish_manifest(raw, "synthetic-derived-power-deathrite-v1");
    let (setup, receipt) = resolve_minion_fight(&manifest);
    let target_damage: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == setup.target_id
        })
        .map(|event| event.payload.clone())
        .collect();

    assert_eq!(
        target_damage,
        [
            json!({
                "accumulated": 0,
                "amount": 0,
                "attemptedAmount": 4,
                "direct": true,
                "instanceId": setup.target_id,
                "prevented": true,
                "seat": "south",
            }),
            json!({
                "accumulated": 0,
                "amount": 0,
                "attemptedAmount": 1,
                "direct": true,
                "instanceId": setup.target_id,
                "prevented": true,
                "seat": "south",
            }),
        ]
    );
    assert!(
        state(&setup.session)["players"]["north"]["cemetery"]
            .as_array()
            .expect("North cemetery")
            .iter()
            .any(|card| card["instanceId"] == setup.attacker_id)
    );
    assert_exact_replay(&setup.session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves stable target snapshots across an awakening AOE"
)]
fn rule_catalog_0745_simultaneous_area_damage_snapshots_status_before_awakening_aura() {
    let seed = 289;
    let mut manifest = base_manifest(seed);
    let preview_manifest = finish_manifest(manifest.clone(), "synthetic-area-snapshot-preview-v1");
    let mut preview = Session::new(&preview_manifest).expect("preview session");
    keep(&mut preview);
    keep(&mut preview);
    let source_card_id = state(&preview)["players"]["north"]["hand"]["spellbook"][0]["cardId"]
        .as_str()
        .expect("Genesis source card")
        .to_owned();
    let mut south_cards = state(&preview)["players"]["south"]["hand"]["spellbook"]
        .as_array()
        .expect("South hand")
        .iter()
        .map(|card| {
            (
                card["instanceId"]
                    .as_str()
                    .expect("card instance")
                    .to_owned(),
                card["cardId"].as_str().expect("card ID").to_owned(),
            )
        })
        .collect::<Vec<_>>();
    south_cards.sort_unstable();
    let aura_card_id = south_cards[0].1.clone();
    let target_card_id = south_cards[1].1.clone();
    manifest["cards"][&source_card_id] = minion(&json!({
        "defense": 10,
        "genesisDamageEachOtherUnitHere": 1,
        "summonToAnySite": true,
    }));
    manifest["cards"][&aura_card_id] = minion(&json!({
        "defense": 10,
        "genesisDisableSelfUntilDamaged": true,
        "otherNearbyAlliesPowerBonus": 1,
        "summonToAnySite": true,
    }));
    manifest["cards"][&target_card_id] = minion(&json!({
        "defense": 1,
        "summonToAnySite": true,
    }));
    let manifest = finish_manifest(manifest, "synthetic-area-status-snapshot-v1");
    let mut session = Session::new(&manifest).expect("valid status snapshot scenario");
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
    let (aura, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == aura_card_id
            && descriptor["cell"] == "C1"
    });
    let aura_id = aura["cardInstanceId"]
        .as_str()
        .expect("aura instance")
        .to_owned();
    let (target, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == target_card_id
            && descriptor["cell"] == "C1"
    });
    let target_id = target["cardInstanceId"]
        .as_str()
        .expect("target instance")
        .to_owned();
    assert!(aura_id < target_id);
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == source_card_id
            && descriptor["cell"] == "C1"
    });
    let awakened = receipt
        .events
        .iter()
        .position(|event| {
            event.event_type == "minion-awakened" && event.payload["instanceId"] == aura_id
        })
        .expect("aura awakens");
    let target_damage = receipt
        .events
        .iter()
        .position(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == target_id
        })
        .expect("later target takes damage");
    assert!(awakened < target_damage);
    assert!(receipt.events.iter().any(|event| {
        event.event_type == "minion-died" && event.payload["instanceId"] == target_id
    }));
    assert_exact_replay(&session);
}

fn resolve_healing_fight(life: u8, seed: u32) -> (AttackSetup, Receipt) {
    let healing = json!({ "deathriteHeal": 3 });
    let manifest = combat_manifest(seed, &healing, &healing, life);
    let mut setup = north_attacks_at_c2(&manifest);
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack" && descriptor["target"]["kind"] == "site"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == false
    });
    assert_eq!(
        state(&setup.session)["players"]["south"]["avatar"]["life"],
        life - 1
    );
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "end-turn"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "move-and-attack"
            && descriptor["unitInstanceId"] == setup.defender_id
            && descriptor["to"]["cell"] == "C2"
    });
    accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "declare-attack"
            && descriptor["target"]["kind"] == "minion"
            && descriptor["target"]["instanceId"] == setup.attacker_id
    });
    let (_, receipt) = accept_where(&mut setup.session, |descriptor| {
        descriptor["kind"] == "close-defend" && descriptor["originalTargetParticipates"] == true
    });
    (setup, receipt)
}

#[test]
fn rule_catalog_0803_deathrite_healing_caps_skips_deaths_door_precedes_cemetery() {
    let (capped, capped_receipt) = resolve_healing_fight(5, 56);
    let healed = capped_receipt
        .events
        .iter()
        .position(|event| event.event_type == "avatar-healed")
        .expect("capped Deathrite heal");
    let first_death = capped_receipt
        .events
        .iter()
        .position(|event| event.event_type == "minion-died")
        .expect("combat deaths");
    assert_eq!(
        capped_receipt.events[healed].payload,
        json!({
            "amount": 1,
            "attemptedAmount": 3,
            "life": 5,
            "seat": "south",
            "sourceInstanceId": capped.defender_id,
        })
    );
    assert!(healed < first_death);
    assert_exact_replay(&capped.session);

    let (death_door, death_door_receipt) = resolve_healing_fight(1, 57);
    assert_eq!(
        state(&death_door.session)["players"]["south"]["avatar"]["life"],
        0
    );
    assert!(
        death_door_receipt
            .events
            .iter()
            .all(|event| event.event_type != "avatar-healed")
    );
    assert_exact_replay(&death_door.session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves the complete AP-then-NAP ordering transaction"
)]
fn rule_catalog_0876_ap_commits_deathrite_order_before_nap_resolves_first() {
    let seed = 281;
    let mut manifest = base_manifest(seed);
    let preview_manifest = finish_manifest(manifest.clone(), "synthetic-ap-nap-preview-v1");
    let preview = Session::new(&preview_manifest).expect("preview session");
    let ap_card_ids = card_ids_in_hand(&preview, "north", 2);
    let genesis_card_id = card_ids_in_hand(&preview, "north", 3)
        .pop()
        .expect("Genesis card");
    let nap_card_ids = card_ids_in_hand(&preview, "south", 2);
    for card_id in ap_card_ids.iter().chain(&nap_card_ids) {
        manifest["cards"][card_id] = minion(&json!({
            "deathriteDrawSite": true,
            "summonToAnySite": true,
        }));
    }
    manifest["cards"][&genesis_card_id] = minion(&json!({
        "defense": 5,
        "genesisDamageEachOtherUnitHere": 1,
        "summonToAnySite": true,
    }));
    let manifest = finish_manifest(manifest, "synthetic-ap-nap-deathrites-v1");
    let mut session = Session::new(&manifest).expect("valid AP/NAP Deathrite scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    for card_id in &ap_card_ids {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == *card_id
                && descriptor["cell"] == "C4"
        });
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    for card_id in &nap_card_ids {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == *card_id
                && descriptor["cell"] == "C4"
        });
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let ap_ids = unit_ids_for_cards(&session, &ap_card_ids);
    let nap_ids = unit_ids_for_cards(&session, &nap_card_ids);
    let (_, trigger) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == genesis_card_id
            && descriptor["cell"] == "C4"
    });
    assert!(
        trigger
            .events
            .iter()
            .all(|event| event.event_type != "site-drawn" && event.event_type != "minion-died")
    );
    assert_eq!(state(&session)["phase"], "trigger-order");
    assert_eq!(state(&session)["decisionSeat"], "north");
    let ap_state = state(&session);
    let ap_pending = &ap_state["pendingDeathrites"];
    assert_eq!(ap_pending["batches"][0]["stage"], "active-order");
    assert_eq!(ap_pending["batches"][0]["activeOrder"], json!([]));
    assert_eq!(
        ap_pending["batches"][0]["activeRemaining"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        ap_pending["batches"][0]["activeRemaining"][0]
            .as_object()
            .map(|source| source.keys().cloned().collect::<Vec<_>>()),
        Some(vec![
            "controller".to_owned(),
            "currentPower".to_owned(),
            "instanceId".to_owned(),
            "lethal".to_owned(),
            "unit".to_owned(),
        ])
    );
    assert_checkpoint_round_trip(&session);

    let ap_actions: Vec<_> = session
        .legal_actions()
        .expect("AP ordering actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .collect();
    for action in &ap_actions {
        let source = ap_pending["batches"][0]["activeRemaining"]
            .as_array()
            .expect("AP sources")
            .iter()
            .find(|source| source["instanceId"] == action.descriptor["sourceInstanceId"])
            .expect("issued AP source");
        assert_eq!(
            action.label,
            format!(
                "Order {} first within your triggers",
                source["unit"]["cardId"].as_str().expect("source card ID")
            )
        );
    }
    let ap_first = ap_actions.first().expect("AP first choice");
    let ap_first_id = ap_first.descriptor["sourceInstanceId"]
        .as_str()
        .expect("AP source")
        .to_owned();
    let ap_order = [
        ap_first_id.clone(),
        ap_ids
            .iter()
            .find(|id| **id != ap_first_id)
            .expect("other AP source")
            .clone(),
    ];
    let (_, committed) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == ap_first_id
    });
    assert_eq!(event_types(&committed), ["trigger-order-committed"]);
    assert_eq!(
        committed.events[0].payload,
        json!({ "seat": "north", "sourceInstanceId": ap_first_id })
    );
    assert_eq!(state(&session)["decisionSeat"], "south");
    assert_eq!(
        state(&session)["pendingDeathrites"]["batches"][0]["activeOrder"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_checkpoint_round_trip(&session);

    let nap_actions: Vec<_> = session
        .legal_actions()
        .expect("NAP ordering actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-triggers")
        .collect();
    let nap_first_id = nap_actions[0].descriptor["sourceInstanceId"]
        .as_str()
        .expect("NAP source")
        .to_owned();
    let nap_order = [
        nap_first_id.clone(),
        nap_ids
            .iter()
            .find(|id| **id != nap_first_id)
            .expect("other NAP source")
            .clone(),
    ];
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == nap_first_id
    });
    let source_order: Vec<_> = resolved
        .events
        .iter()
        .filter(|event| event.event_type == "site-drawn")
        .map(|event| event.payload["sourceInstanceId"].clone())
        .collect();
    assert_eq!(
        source_order,
        nap_order
            .iter()
            .chain(&ap_order)
            .map(|id| json!(id))
            .collect::<Vec<_>>()
    );
    let types = event_types(&resolved);
    assert!(
        types
            .iter()
            .rposition(|kind| *kind == "site-drawn")
            .expect("last Deathrite draw")
            < types
                .iter()
                .position(|kind| *kind == "minion-died")
                .expect("first cemetery entry")
    );
    assert_eq!(
        types.iter().filter(|kind| **kind == "minion-died").count(),
        4
    );
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one direct scenario proves ordered nested Deathrite damage batches"
)]
fn rule_catalog_0877_deathrite_area_damage_chains_in_ordered_simultaneous_batches() {
    let seed = 263;
    let mut manifest = base_manifest(seed);
    let preview_manifest = finish_manifest(manifest.clone(), "synthetic-area-preview-v1");
    let preview = Session::new(&preview_manifest).expect("preview session");
    let source_card_id = card_ids_in_hand(&preview, "north", 1)
        .pop()
        .expect("Genesis source");
    let south_ids = card_ids_in_hand(&preview, "south", 3);
    let scarab_card_ids = south_ids[..2].to_vec();
    let chained_card_id = south_ids[2].clone();
    manifest["cards"][&source_card_id] = minion(&json!({
        "defense": 4,
        "genesisDamageEachOtherUnitHere": 1,
        "summonToAnySite": true,
    }));
    for card_id in &scarab_card_ids {
        manifest["cards"][card_id] = minion(&json!({
            "deathriteDamageEachUnitHere": 1,
            "summonToAnySite": true,
        }));
    }
    manifest["cards"][&chained_card_id] = minion(&json!({
        "deathriteDamageEachUnitHere": 1,
        "defense": 2,
        "summonToAnySite": true,
    }));

    let mut invalid_zero = manifest.clone();
    invalid_zero["cards"][&scarab_card_ids[0]]["deathriteDamageEachUnitHere"] = json!(0);
    assert!(Session::new(&finish_manifest(invalid_zero, "invalid-zero-deathrite")).is_err());
    let mut valid_area = manifest.clone();
    valid_area["cards"][&scarab_card_ids[0]]["occupiesSquareArea"] = json!(2);
    Session::new(&finish_manifest(valid_area, "valid-area-deathrite"))
        .expect("oversized Deathrite with summonToAnySite is admitted");
    let mut valid_three = manifest.clone();
    valid_three["cards"][&scarab_card_ids[0]]["deathriteDamageEachUnitHere"] = json!(3);
    Session::new(&finish_manifest(valid_three, "valid-three-deathrite"))
        .expect("bounded Deathrite damage accepts three");

    let manifest = finish_manifest(manifest, "synthetic-deathrite-area-damage-v1");
    let mut session = Session::new(&manifest).expect("valid chained Deathrite scenario");
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
    for card_id in scarab_card_ids.iter().chain([&chained_card_id]) {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == *card_id
                && descriptor["cell"] == "C1"
        });
    }
    let scarab_ids = unit_ids_for_cards(&session, &scarab_card_ids);
    let chained_id = unit_ids_for_cards(&session, std::slice::from_ref(&chained_card_id))
        .pop()
        .expect("chained source identity");
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (summon, trigger) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == source_card_id
            && descriptor["cell"] == "C1"
    });
    let source_id = summon["cardInstanceId"]
        .as_str()
        .expect("Genesis source identity")
        .to_owned();
    assert!(trigger.events.iter().all(|event| {
        event.event_type != "deathrite-damage-allocated" && event.event_type != "minion-died"
    }));
    assert_eq!(state(&session)["phase"], "trigger-order");
    assert_eq!(state(&session)["decisionSeat"], "south");
    assert_eq!(
        state(&session)["pendingDeathrites"]["batches"][0]["stage"],
        "non-active-order"
    );
    assert_checkpoint_round_trip(&session);
    let paused_state = state(&session);
    let paused_units = paused_state["realm"]["units"]
        .as_array()
        .expect("live marked wave units");
    let paused_unit = |instance_id: &str| {
        paused_units
            .iter()
            .find(|unit| unit["instanceId"] == instance_id)
            .expect("live occurrence")
    };
    assert!(
        scarab_ids
            .iter()
            .all(|id| paused_unit(id)["deathMarked"] == true)
    );
    assert_eq!(paused_unit(&chained_id)["deathMarked"], Value::Null);
    assert_eq!(paused_unit(&source_id)["deathMarked"], Value::Null);
    assert!(trigger.events.iter().all(|event| {
        event.event_type != "deathrite-damage-allocated" && event.event_type != "minion-died"
    }));
    let first_id = session
        .legal_actions()
        .expect("Deathrite ordering actions")[0]
        .descriptor["sourceInstanceId"]
        .as_str()
        .expect("first Deathrite source")
        .to_owned();
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-triggers" && descriptor["sourceInstanceId"] == first_id
    });
    let allocations: Vec<_> = resolved
        .events
        .iter()
        .filter(|event| event.event_type == "deathrite-damage-allocated")
        .collect();
    assert_eq!(allocations.len(), 12);
    let mut source_order = Vec::new();
    for event in &allocations {
        let source = event.payload["sourceInstanceId"]
            .as_str()
            .expect("allocation source");
        if source_order.last().is_none_or(|last| last != source) {
            source_order.push(source.to_owned());
        }
    }
    let other_id = scarab_ids
        .iter()
        .find(|id| **id != first_id)
        .expect("other initial Deathrite")
        .clone();
    assert_eq!(
        source_order,
        [first_id.clone(), chained_id.clone(), other_id.clone()]
    );
    let mut all_recipients = allocations
        .iter()
        .map(|event| {
            event.payload["targetInstanceId"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    all_recipients.extend([source_id.clone(), chained_id.clone()]);
    all_recipients.extend(scarab_ids.iter().cloned());
    all_recipients.sort();
    all_recipients.dedup();
    assert_eq!(
        all_recipients.len(),
        5,
        "four minions plus the nearby avatar"
    );
    for source in &source_order {
        let mut actual_targets = allocations
            .iter()
            .filter(|event| event.payload["sourceInstanceId"] == *source)
            .map(|event| {
                assert_eq!(event.payload["amount"], 1);
                event.payload["targetInstanceId"]
                    .as_str()
                    .expect("allocated recipient")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        actual_targets.sort();
        let mut expected_targets = all_recipients
            .iter()
            .filter(|target| *target != source)
            .cloned()
            .collect::<Vec<_>>();
        expected_targets.sort();
        assert_eq!(actual_targets, expected_targets, "recipients for {source}");
    }
    let mut departed = resolved
        .events
        .iter()
        .filter(|event| event.event_type == "minion-died")
        .map(|event| event.payload["instanceId"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    departed.sort();
    let mut expected_departed = source_order.clone();
    expected_departed.sort();
    assert_eq!(departed, expected_departed);
    let final_state = state(&session);
    let final_units = final_state["realm"]["units"]
        .as_array()
        .expect("final realm units");
    assert!(expected_departed.iter().all(|id| {
        !final_units
            .iter()
            .any(|unit| unit["instanceId"] == id.as_str())
    }));
    let south_cemetery = final_state["players"]["south"]["cemetery"]
        .as_array()
        .expect("owner cemetery");
    assert!(expected_departed.iter().all(|id| {
        south_cemetery
            .iter()
            .any(|card| card["instanceId"] == id.as_str())
    }));
    let types = event_types(&resolved);
    assert!(
        types
            .iter()
            .rposition(|kind| *kind == "deathrite-damage-allocated")
            .expect("last Deathrite allocation")
            < types
                .iter()
                .position(|kind| *kind == "minion-died")
                .expect("first cemetery entry")
    );
    assert_eq!(
        types.iter().filter(|kind| **kind == "minion-died").count(),
        3
    );
    let after = final_state;
    assert_eq!(after["players"]["north"]["avatar"]["life"], 20);
    assert_eq!(after["players"]["south"]["avatar"]["life"], 16);
    let units = after["realm"]["units"].as_array().expect("realm units");
    assert_eq!(units.len(), 1);
    assert_eq!(
        json!({
            "damage": units[0]["damage"],
            "instanceId": units[0]["instanceId"],
            "location": units[0]["location"],
        }),
        json!({ "damage": 3, "instanceId": source_id, "location": "C1" })
    );
    assert!(resolved.random_draws.is_empty());
    assert_exact_replay(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one simultaneous area event pins terminal current-event completion"
)]
fn simultaneous_area_terminal_preserves_marked_harmful_deathrite() {
    let encoded = fixed_terminal_manifest(
        "north",
        20,
        1,
        8,
        8,
        &json!({"deathriteDamageEachUnitHere":1}),
        None,
        true,
    );
    let mut session = prepare_terminal_area(&encoded, "north", 1, true, true);
    let before_cast = state(&session);
    assert_eq!(before_cast["players"]["north"]["avatar"]["life"], 19);
    assert_eq!(before_cast["players"]["south"]["avatar"]["life"], 0);
    let victim_id = before_cast["realm"]["units"][0]["instanceId"]
        .as_str()
        .unwrap()
        .to_owned();
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "area")
        .expect("issued area Magic");
    let mut ignored = replay_game_for_session(&session);
    let ignored_action = ignored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| a.to_legal_action().unwrap().action_id == action.action_id)
        .expect("same issued action in Ignore path");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap()
    else {
        panic!("area action accepted")
    };
    ignored
        .apply_action(&ignored_action)
        .expect("Ignore-path area action");
    let after = state(&session);
    assert_eq!(ignored.authoritative_state(), after);
    assert_eq!(ignored.state_hash().unwrap(), session.state_hash().unwrap());
    assert_eq!(event_types(&receipt).last(), Some(&"game-ended"));
    assert_eq!(
        receipt.events.last().unwrap().payload,
        json!({"loser":"south","reason":"avatar_defeated","winner":"north"})
    );
    let damaged: Vec<_> = receipt
        .events
        .iter()
        .filter(|e| e.event_type == "damage-dealt")
        .map(|e| e.payload["instanceId"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(damaged.iter().filter(|id| *id == &victim_id).count(), 1);
    assert_eq!(
        damaged
            .iter()
            .filter(|id| *id
                == after["players"]["north"]["avatar"]["card"]["instanceId"]
                    .as_str()
                    .unwrap())
            .count(),
        1
    );
    assert_eq!(
        damaged
            .iter()
            .filter(|id| *id
                == after["players"]["south"]["avatar"]["card"]["instanceId"]
                    .as_str()
                    .unwrap())
            .count(),
        1
    );
    assert_eq!(after["players"]["north"]["avatar"]["life"], 18);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|e| e.event_type == "death-blow" && e.payload["seat"] == "south")
            .count(),
        1
    );
    assert!(!event_types(&receipt).iter().any(|kind| {
        [
            "deathrite-damage-allocated",
            "minion-died",
            "site-drawn",
            "magic-resolved",
        ]
        .contains(kind)
    }));
    assert_eq!(
        after["realm"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|unit| unit["instanceId"] == victim_id)
            .count(),
        1
    );
    let victim = after["realm"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["instanceId"] == victim_id)
        .unwrap();
    assert_eq!(victim["deathMarked"], true);
    assert_eq!(victim["damage"], 1);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["instanceId"] != victim_id)
    );
    assert!(
        after["pendingDeathrites"]["marked"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["instanceId"] == victim_id)
    );
    assert!(
        after["pendingDeathrites"]["batches"][0]["resolving"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["instanceId"] == victim_id)
    );
    assert_eq!(after["pendingDeathrites"]["continuation"]["kind"], "effect");
    assert_eq!(after["pendingDeathrites"]["continuation"]["entry"], "magic");
    assert_eq!(after["pendingDeathrites"]["continuation"]["cursor"], 1);
    let magic_id = receipt.events[0].payload["instanceId"].clone();
    assert_eq!(
        after["pendingDeathrites"]["continuation"]["magic"]["instanceId"],
        magic_id
    );
    assert_eq!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|card| card["instanceId"] == magic_id)
            .count(),
        0
    );
    assert_eq!(
        after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|card| card["instanceId"] == magic_id)
            .count(),
        0
    );
    assert!(session.legal_actions().unwrap().is_empty());
    assert_exact_replay(&session);
    assert_checkpoint_round_trip(&session);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "mirrored simultaneous defeat and same-turn Death's Door are one bounded comparison"
)]
fn simultaneous_area_double_death_blow_and_same_turn_control() {
    for (first, area_kind, later, expected_deathblows) in [
        ("north", None, true, 2usize),
        ("south", None, true, 2usize),
        ("north", None, false, 0usize),
    ] {
        let encoded = fixed_terminal_manifest(
            first,
            1,
            1,
            8,
            8,
            &json!({"deathriteDamageEachUnitHere":1}),
            area_kind,
            true,
        );
        let mut session = prepare_terminal_area(&encoded, first, 1, true, later);
        let before = state(&session);
        let victim_id = before["realm"]["units"][0]["instanceId"].clone();
        let action = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "area")
            .unwrap();
        let mut ignored = replay_game_for_session(&session);
        let ignored_action = ignored
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|a| a.to_legal_action().unwrap().action_id == action.action_id)
            .unwrap();
        let StepResult::Accepted(receipt) = session
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .unwrap()
        else {
            panic!("area action accepted")
        };
        ignored.apply_action(&ignored_action).unwrap();
        let after = state(&session);
        assert_eq!(ignored.authoritative_state(), after);
        assert_eq!(ignored.state_hash().unwrap(), session.state_hash().unwrap());
        assert_eq!(
            receipt
                .events
                .iter()
                .filter(|event| event.event_type == "death-blow")
                .count(),
            expected_deathblows
        );
        if later {
            assert_eq!(
                receipt
                    .events
                    .iter()
                    .filter(|event| event.event_type == "game-ended")
                    .count(),
                1
            );
            assert_eq!(
                receipt
                    .events
                    .iter()
                    .find(|event| event.event_type == "game-ended")
                    .unwrap()
                    .payload,
                json!({"reason":"simultaneous_avatar_defeat","result":"draw"})
            );
            for seat in ["north", "south"] {
                let avatar_id = before["players"][seat]["avatar"]["card"]["instanceId"].clone();
                assert_eq!(
                    receipt
                        .events
                        .iter()
                        .filter(|event| event.event_type == "death-blow"
                            && event.payload["instanceId"] == avatar_id)
                        .count(),
                    1
                );
            }
            assert_eq!(after["realm"]["units"][0]["instanceId"], victim_id);
            assert_eq!(after["realm"]["units"][0]["deathMarked"], true);
            assert!(
                event_types(&receipt)
                    .iter()
                    .position(|kind| *kind == "game-ended")
                    .unwrap()
                    > event_types(&receipt)
                        .iter()
                        .position(|kind| *kind == "damage-dealt")
                        .unwrap()
            );
        } else {
            assert_eq!(
                receipt
                    .events
                    .iter()
                    .filter(|event| event.event_type == "game-ended"
                        || event.event_type == "death-blow")
                    .count(),
                0
            );
            for seat in ["north", "south"] {
                let avatar_id = before["players"][seat]["avatar"]["card"]["instanceId"].clone();
                assert!(
                    receipt
                        .events
                        .iter()
                        .any(|event| event.event_type == "damage-dealt"
                            && event.payload["instanceId"] == avatar_id
                            && event.payload["amount"] == 0)
                );
            }
            assert_eq!(
                receipt
                    .events
                    .iter()
                    .filter(|event| event.event_type == "minion-died"
                        && event.payload["instanceId"] == victim_id)
                    .count(),
                1
            );
            assert_eq!(
                receipt
                    .events
                    .iter()
                    .filter(|event| event.event_type == "site-drawn")
                    .count(),
                1
            );
            assert_eq!(
                receipt
                    .events
                    .iter()
                    .filter(|event| event.event_type == "magic-resolved")
                    .count(),
                1
            );
            assert_eq!(
                after["players"]["north"]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|card| card["instanceId"] == victim_id)
                    .count(),
                1
            );
        }
        assert_exact_replay(&session);
        assert_checkpoint_round_trip(&session);
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "terminal unsupported rollback is paired with its identical nonterminal control"
)]
fn mixed_terminal_deathrite_rolls_back_but_same_facts_nonterminal_completes() {
    let body = json!({"deathriteDamageEachUnitHere":1,"deathriteHeal":1});
    let encoded = fixed_terminal_manifest("north", 1, 1, 8, 8, &body, Some("minion"), true);
    let mut session = prepare_terminal_area(&encoded, "north", 1, true, true);
    let checkpoint_bytes =
        serialize_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
    let pre_state = state(&session);
    let pre_hash = session.state_hash().unwrap();
    let pre_version = session.state_version();
    let pre_transcript = session.transcript().to_vec();
    let pre_north = session.public_view(Seat::North).unwrap();
    let pre_south = session.public_view(Seat::South).unwrap();
    let pre_actions = session.legal_actions().unwrap();
    assert_eq!(
        pre_north["players"]["north"]["avatar"]["life"],
        pre_state["players"]["north"]["avatar"]["life"]
    );
    assert_eq!(
        pre_south["players"]["south"]["avatar"]["life"],
        pre_state["players"]["south"]["avatar"]["life"]
    );
    let mut ignored = replay_game_for_session(&session);
    let action = session
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "area")
        .unwrap();
    let ignored_action = ignored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| a.to_legal_action().unwrap().action_id == action.action_id)
        .unwrap();
    let error = session
        .step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        })
        .unwrap_err();
    assert!(
        matches!(error, sorcery_engine::session::SessionError::Game(GameError::UnsupportedMechanic(ref reason)) if reason == "mixed Deathrite clause at terminal damage")
    );
    assert_eq!(
        session.unsupported_mechanic(),
        Some("mixed Deathrite clause at terminal damage")
    );
    assert_eq!(session.state_hash().unwrap(), pre_hash);
    assert_eq!(session.state_version(), pre_version);
    assert_eq!(session.transcript(), pre_transcript);
    assert_eq!(session.public_view(Seat::North).unwrap(), pre_north);
    assert_eq!(session.public_view(Seat::South).unwrap(), pre_south);
    assert!(session.replay_value().is_err());
    assert!(session.session_hash().is_err());
    assert!(session.verify_replay().is_err());
    assert!(session.legal_actions().is_err());
    assert!(create_game_checkpoint(&session).is_err());
    let error = ignored.apply_action(&ignored_action).unwrap_err();
    assert!(
        matches!(error, GameError::UnsupportedMechanic(ref reason) if reason == "mixed Deathrite clause at terminal damage")
    );
    assert_eq!(ignored.authoritative_state(), pre_state);
    let restored =
        resume_game_checkpoint(&parse_game_checkpoint(&checkpoint_bytes).unwrap()).unwrap();
    assert_eq!(state(&restored), pre_state);
    assert_eq!(restored.legal_actions().unwrap(), pre_actions);
    let restored_action = restored
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.action_id == action.action_id)
        .expect("same issued cast after pre-failure checkpoint restore");
    let mut restored = restored;
    let restored_error = restored
        .step(ActionRequest {
            action_id: restored_action.action_id.to_string(),
            seat: restored_action.seat,
            state_version: restored_action.state_version,
        })
        .unwrap_err();
    assert!(
        matches!(restored_error, sorcery_engine::session::SessionError::Game(GameError::UnsupportedMechanic(ref reason)) if reason == "mixed Deathrite clause at terminal damage")
    );

    let control = fixed_terminal_manifest("north", 20, 20, 8, 8, &body, Some("minion"), true);
    let mut positive = prepare_terminal_area(&control, "north", 1, true, true);
    let before = state(&positive);
    let victim_id = before["realm"]["units"][0]["instanceId"].clone();
    let area_action = positive
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "area")
        .unwrap();
    let mut positive_ignore = replay_game_for_session(&positive);
    let ignored_action = positive_ignore
        .legal_actions()
        .unwrap()
        .into_iter()
        .find(|a| a.to_legal_action().unwrap().action_id == area_action.action_id)
        .unwrap();
    let StepResult::Accepted(receipt) = positive
        .step(ActionRequest {
            action_id: area_action.action_id.to_string(),
            seat: area_action.seat,
            state_version: area_action.state_version,
        })
        .unwrap()
    else {
        panic!("nonterminal mixed path completes")
    };
    positive_ignore.apply_action(&ignored_action).unwrap();
    assert_eq!(positive_ignore.authoritative_state(), state(&positive));
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(
                |event| event.event_type == "avatar-life-lost" && event.payload["seat"] == "north"
            )
            .count(),
        1
    );
    assert_eq!(state(&positive)["players"]["north"]["avatar"]["life"], 19);
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "avatar-healed" && event.payload["seat"] == "north")
            .count(),
        1
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "minion-died"
                && event.payload["instanceId"] == victim_id)
            .count(),
        1
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "site-drawn")
            .count(),
        1
    );
    assert_eq!(
        receipt
            .events
            .iter()
            .filter(|event| event.event_type == "magic-resolved")
            .count(),
        1
    );
    assert_exact_replay(&positive);
    assert_checkpoint_round_trip(&positive);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "actual Session compiled-Magic checkpoint is proved through terminal and positive branches"
)]
fn compiled_magic_holds_card_through_terminal_deathrite_and_session_resume() {
    for atlas_len in [3usize, 6usize] {
        let encoded = fixed_terminal_manifest(
            "north",
            20,
            20,
            atlas_len,
            8,
            &json!({"deathriteDrawSite":true}),
            Some("minion"),
            false,
        );
        let mut session = prepare_terminal_area(&encoded, "north", 2, false, false);
        let before_cast = state(&session);
        let action = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .find(|a| a.descriptor["kind"] == "cast-magic" && a.descriptor["cardId"] == "area")
            .unwrap();
        let StepResult::Accepted(cast) = session
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .unwrap()
        else {
            panic!("compiled area cast accepted")
        };
        assert_eq!(
            session.replay_value().unwrap()["state"]["phase"],
            "trigger-order"
        );
        assert_eq!(
            cast.events
                .iter()
                .filter(|event| event.event_type == "damage-dealt")
                .count(),
            2
        );
        let ordered = state(&session);
        let marked: Vec<_> = ordered["pendingDeathrites"]["marked"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["instanceId"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(marked.len(), 2);
        assert_eq!(
            ordered["pendingDeathrites"]["continuation"]["kind"],
            "effect"
        );
        assert_eq!(ordered["pendingDeathrites"]["continuation"]["cursor"], 1);
        assert_eq!(
            ordered["pendingDeathrites"]["continuation"]["entry"],
            "magic"
        );
        let magic_id = cast.events[0].payload["instanceId"].clone();
        assert_eq!(
            ordered["pendingDeathrites"]["continuation"]["magic"]["instanceId"],
            magic_id
        );
        let checkpoint =
            serialize_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
        let parsed = parse_game_checkpoint(&checkpoint).unwrap();
        let mut resumed = resume_game_checkpoint(&parsed).unwrap();
        assert_eq!(state(&resumed), ordered);
        assert_eq!(
            resumed.legal_actions().unwrap(),
            session.legal_actions().unwrap()
        );
        let choices: Vec<_> = session
            .legal_actions()
            .unwrap()
            .into_iter()
            .filter(|a| a.descriptor["kind"] == "order-triggers")
            .collect();
        assert_eq!(choices.len(), 2);
        let chosen = choices[0].descriptor["sourceInstanceId"]
            .as_str()
            .unwrap()
            .to_owned();
        let (_, first_receipt) = accept_where(&mut session, |d| {
            d["kind"] == "order-triggers" && d["sourceInstanceId"] == chosen
        });
        let (_, resumed_receipt) = accept_where(&mut resumed, |d| {
            d["kind"] == "order-triggers" && d["sourceInstanceId"] == chosen
        });
        assert_eq!(first_receipt, resumed_receipt);
        assert_eq!(session.transcript(), resumed.transcript());
        assert_eq!(state(&session), state(&resumed));
        let after = state(&session);
        if atlas_len == 3 {
            assert_eq!(
                event_types(&first_receipt),
                ["trigger-order-committed", "game-ended"]
            );
            assert_eq!(
                first_receipt.events.last().unwrap().payload,
                json!({"loser":"north","reason":"deck_empty","winner":"south"})
            );
            assert!(
                !event_types(&first_receipt).iter().any(|kind| [
                    "site-drawn",
                    "minion-died",
                    "magic-resolved"
                ]
                .contains(kind))
            );
            assert_eq!(after["pendingDeathrites"]["continuation"]["cursor"], 1);
            assert_eq!(
                after["pendingDeathrites"]["continuation"]["magic"]["instanceId"],
                magic_id
            );
            assert!(marked.iter().all(|id| {
                after["realm"]["units"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|unit| unit["instanceId"] == *id && unit["deathMarked"] == true)
            }));
            assert_eq!(
                after["players"]["north"]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|card| card["instanceId"] == magic_id)
                    .count(),
                0
            );
            assert_eq!(session.legal_actions().unwrap(), Vec::new());
            let terminal_bytes =
                serialize_game_checkpoint(&create_game_checkpoint(&session).unwrap()).unwrap();
            let terminal_resume =
                resume_game_checkpoint(&parse_game_checkpoint(&terminal_bytes).unwrap()).unwrap();
            assert_eq!(state(&terminal_resume), after);
            assert_eq!(terminal_resume.legal_actions().unwrap(), Vec::new());
            let mut ignored = replay_game_for_session(&Session::new(&encoded).unwrap());
            // Rebuild the exact pre-order transition through issued transcript actions.
            for receipt in session.transcript() {
                let issued = ignored
                    .legal_actions()
                    .unwrap()
                    .into_iter()
                    .find(|a| a.to_legal_action().unwrap().action_id == receipt.action_id)
                    .unwrap();
                let result = ignored.apply_action(&issued);
                assert!(
                    result.is_ok(),
                    "recorded terminal choice has Ignore-path parity"
                );
            }
            assert_eq!(ignored.authoritative_state(), after);
        } else {
            let drawn: Vec<_> = first_receipt
                .events
                .iter()
                .filter(|event| event.event_type == "site-drawn")
                .map(|event| event.payload["sourceInstanceId"].clone())
                .collect();
            assert_eq!(drawn.len(), 3);
            assert_eq!(
                &drawn[..2],
                &[
                    json!(chosen),
                    json!(choices[1].descriptor["sourceInstanceId"])
                ]
            );
            assert_eq!(drawn[2], magic_id);
            assert_eq!(
                first_receipt
                    .events
                    .iter()
                    .filter(|event| event.event_type == "minion-died")
                    .count(),
                2
            );
            for id in &marked {
                assert_eq!(
                    first_receipt
                        .events
                        .iter()
                        .filter(|event| event.event_type == "minion-died"
                            && event.payload["instanceId"] == *id)
                        .count(),
                    1
                );
            }
            assert_eq!(
                first_receipt
                    .events
                    .iter()
                    .filter(|event| event.event_type == "magic-resolved"
                        && event.payload["instanceId"] == magic_id)
                    .count(),
                1
            );
            assert_eq!(
                after["players"]["north"]["cemetery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|card| card["instanceId"] == magic_id)
                    .count(),
                1
            );
            assert_eq!(after["pendingDeathrites"], Value::Null);
        }
        assert_exact_replay(&session);
        assert_exact_replay(&resumed);
        assert_checkpoint_round_trip(&session);
        assert_checkpoint_round_trip(&resumed);
        assert_eq!(state(&resumed), state(&session));
        assert_eq!(before_cast["realm"]["units"].as_array().unwrap().len(), 2);
    }
}

#[test]
fn rule_catalog_0932_deathrite_area_damage_skips_self_and_hits_avatar_sharing_cell() {
    let seed = 312;
    let mut manifest = base_manifest(seed);
    let preview_manifest = finish_manifest(manifest.clone(), "synthetic-deathrite-self-preview-v1");
    let preview = Session::new(&preview_manifest).expect("preview session");
    let genesis_card_id = card_ids_in_hand(&preview, "north", 1)
        .pop()
        .expect("Genesis source card");
    let deathrite_card_id = card_ids_in_hand(&preview, "south", 1)
        .pop()
        .expect("Deathrite minion card");
    manifest["cards"][&genesis_card_id] = minion(&json!({
        "defense": 10,
        "genesisDamageEachOtherUnitHere": 1,
        "summonToAnySite": true,
    }));
    manifest["cards"][&deathrite_card_id] = minion(&json!({
        "defense": 1,
        "deathriteDamageEachUnitHere": 2,
        "summonToAnySite": true,
    }));
    let manifest = finish_manifest(manifest, "synthetic-deathrite-self-skip-v1");
    let mut session = Session::new(&manifest).expect("valid self-skip Deathrite scenario");
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
    let (deathrite, _) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == deathrite_card_id
            && descriptor["cell"] == "C1"
    });
    let deathrite_id = deathrite["cardInstanceId"]
        .as_str()
        .expect("Deathrite minion identity")
        .to_owned();
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == genesis_card_id
            && descriptor["cell"] == "C1"
    });
    let types = event_types(&resolved);
    let allocations: Vec<_> = resolved
        .events
        .iter()
        .filter(|event| event.event_type == "deathrite-damage-allocated")
        .collect();
    assert!(
        types
            .iter()
            .rposition(|kind| *kind == "deathrite-damage-allocated")
            .expect("Deathrite allocation")
            < types
                .iter()
                .position(|kind| *kind == "minion-died")
                .expect("cemetery entry")
    );
    let source_id = deathrite_id.clone();
    let south_avatar_id =
        state(&session)["players"]["south"]["avatar"]["card"]["instanceId"].clone();
    assert_eq!(allocations.len(), 2);
    for allocation in &allocations {
        assert_eq!(allocation.payload["amount"], 2);
        assert_eq!(allocation.payload["sourceInstanceId"], source_id);
        assert_ne!(
            allocation.payload["sourceInstanceId"],
            allocation.payload["targetInstanceId"]
        );
    }
    let targets: Vec<_> = allocations
        .iter()
        .map(|allocation| allocation.payload["targetInstanceId"].clone())
        .collect();
    assert!(targets.contains(&south_avatar_id));
    assert!(!targets.contains(&json!(deathrite_id)));
    let avatar_damage: Vec<_> = resolved
        .events
        .iter()
        .filter(|event| {
            event.event_type == "damage-dealt" && event.payload["instanceId"] == south_avatar_id
        })
        .map(|event| event.payload["amount"].clone())
        .collect();
    assert_eq!(avatar_damage, [json!(1), json!(2)]);
    assert_eq!(state(&session)["players"]["south"]["avatar"]["life"], 17);
    assert_exact_replay(&session);
}
