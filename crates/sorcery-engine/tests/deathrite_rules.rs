use serde_json::{Value, json};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt};
use sorcery_engine::session::{Session, StepResult};
use sorcery_engine::synthetic::synthetic_demo_manifest_json;

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
fn simultaneous_deathrites_should_resolve_nap_then_ap_before_cemetery_entry() {
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
    let (deck_out, receipt) = resolve_minion_fight(&empty_manifest);
    assert!(
        receipt
            .events
            .iter()
            .all(|event| event.event_type != "site-drawn")
    );
    assert_eq!(
        state(&deck_out.session)["terminal"],
        json!({
            "reason": "simultaneous_defeat",
            "result": "draw",
            "status": "finished",
        })
    );
    assert_exact_replay(&deck_out.session);
}

#[test]
fn bladderblimp_deathrite_should_count_each_players_nearby_controlled_sites() {
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
fn deathrite_area_damage_should_use_last_location_ward_reduction_and_lethal() {
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
fn deathrite_damage_should_preserve_source_power_until_resolution() {
    let mut raw = base_manifest(171);
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
            "preventsDamageFromUnitsWithPowerAtLeast": 4,
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
fn simultaneous_area_damage_should_snapshot_status_before_awakening_power_aura() {
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
fn deathrite_healing_should_cap_skip_deaths_door_and_precede_cemetery_entry() {
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
fn ap_should_commit_deathrite_order_before_nap_resolves_first() {
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
    assert_eq!(state(&session)["phase"], "deathrite-order");
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
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
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
                "Order {} first within your Deathrites",
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
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == ap_first_id
    });
    assert_eq!(event_types(&committed), ["deathrite-order-committed"]);
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
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
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
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == nap_first_id
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
fn deathrite_area_damage_should_chain_in_ordered_simultaneous_batches() {
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
    let mut invalid_area = manifest.clone();
    invalid_area["cards"][&scarab_card_ids[0]]["occupiesSquareArea"] = json!(2);
    assert!(Session::new(&finish_manifest(invalid_area, "invalid-area-deathrite")).is_err());
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
    assert_eq!(state(&session)["phase"], "deathrite-order");
    assert_eq!(state(&session)["decisionSeat"], "south");
    assert_eq!(
        state(&session)["pendingDeathrites"]["batches"][0]["stage"],
        "non-active-order"
    );
    assert_checkpoint_round_trip(&session);
    let first_id = session
        .legal_actions()
        .expect("Deathrite ordering actions")[0]
        .descriptor["sourceInstanceId"]
        .as_str()
        .expect("first Deathrite source")
        .to_owned();
    let (_, resolved) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "order-deathrites" && descriptor["sourceInstanceId"] == first_id
    });
    let allocations: Vec<_> = resolved
        .events
        .iter()
        .filter(|event| event.event_type == "deathrite-damage-allocated")
        .collect();
    assert_eq!(allocations.len(), 7);
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
    assert_eq!(source_order, [first_id, chained_id, other_id]);
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
    let after = state(&session);
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
