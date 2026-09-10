use serde_json::{Value, json};
use sorcery_engine::canonical::{canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Receipt, RejectionCode};
use sorcery_engine::session::{Session, StepResult};

fn thresholds(element: Option<&str>, required: u64) -> Value {
    let mut value = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    if let Some(element) = element {
        value[element] = json!(required);
    }
    value
}

fn site(element: &str, ordinary_discount: bool) -> Value {
    let mut value = json!({ "cardType": "site", "elements": [element] });
    if ordinary_discount {
        value["ordinaryMinionManaDiscount"] = json!(1);
    }
    value
}

fn minion(mana_cost: u64, required: &Value) -> Value {
    json!({
        "attack": 1,
        "cardType": "minion",
        "defense": 1,
        "manaCost": mana_cost,
        "thresholds": required,
    })
}

fn scenario_manifest(
    seed: u32,
    north_site: &Value,
    north_minion: &Value,
    south_site: &Value,
) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "summon-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-summon-rules-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-minion": north_minion,
            "north-site": north_site,
            "south-avatar": avatar,
            "south-minion": minion(0, &thresholds(None, 0)),
            "south-site": south_site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 4],
                "avatar": "north-avatar",
                "spellbook": vec!["north-minion"; 4],
            },
            "south": {
                "atlas": vec!["south-site"; 4],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4],
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

fn wendigo_manifest(mana_cost: u64, genesis_mana: u64, deathrites: bool) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let local = |deathrite| {
        let mut value = minion(0, &thresholds(None, 0));
        if deathrite {
            value["deathriteDrawSite"] = json!(true);
        }
        value
    };
    let mut burrower = local(false);
    burrower["burrowing"] = json!(true);
    let mut local_a = local(deathrites);
    local_a["provides"] = json!("water");
    let mut enemy = local(false);
    enemy["summonToAnySite"] = json!(true);
    let mut wendigo = minion(mana_cost, &thresholds(Some("water"), 1));
    wendigo["attack"] = json!(5);
    wendigo["defense"] = json!(5);
    wendigo["sacrificeMinionAtSummoningLocationForManaDiscount"] = json!(2);
    let mana_site = json!({
        "cardType": "site",
        "elements": ["earth"],
        "genesisGainMana": genesis_mana,
    });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "wendigo-summon-rules" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-wendigo-summon-rules-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-burrower": burrower,
            "north-bury": {
                "burrowTargetMinionOrArtifact": true,
                "cardType": "magic",
                "manaCost": 0,
                "thresholds": thresholds(None, 0),
            },
            "north-fill-1": local(false),
            "north-fill-2": local(false),
            "north-fill-3": local(false),
            "north-local-a": local_a,
            "north-local-b": local(deathrites),
            "north-site": mana_site,
            "north-wendigo": wendigo,
            "south-avatar": avatar,
            "south-enemy": enemy,
            "south-fill": local(false),
            "south-site": mana_site,
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 6],
                "avatar": "north-avatar",
                "spellbook": [
                    "north-wendigo",
                    "north-local-a",
                    "north-burrower",
                    "north-local-b",
                    "north-bury",
                    "north-fill-1",
                    "north-fill-2",
                    "north-fill-3",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 6],
                "avatar": "south-avatar",
                "spellbook": [
                    "south-enemy",
                    "south-fill",
                    "south-fill",
                    "south-fill",
                    "south-fill",
                    "south-fill",
                    "south-fill",
                    "south-fill",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 332,
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

fn first_main(manifest: &str) -> Session {
    let mut session = Session::new(manifest).expect("valid summon scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "north-site"
            && descriptor["cell"] == "C4"
    });
    session
}

fn north_second_main(mut session: Session) -> Session {
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site"
            && descriptor["cardId"] == "south-site"
            && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    session
}

fn wendigo_main(manifest: &str, bury_burrower: bool) -> Session {
    let mut session = Session::new(manifest).expect("valid Wendigo scenario");
    keep(&mut session);
    keep(&mut session);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    for card_id in ["north-local-a", "north-burrower"] {
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == card_id
                && descriptor["cell"] == "C4"
                && descriptor["region"].is_null()
        });
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "south-enemy"
            && descriptor["cell"] == "C4"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    if bury_burrower {
        let burrower_id = state(&session)["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .find(|unit| unit["cardId"] == "north-burrower")
            .expect("burrower")["instanceId"]
            .clone();
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "cast-magic"
                && descriptor["cardId"] == "north-bury"
                && descriptor["target"]["instanceId"] == burrower_id
        });
    }
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C2"
    });
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "north-local-b"
            && descriptor["cell"] == "C4"
    });
    session
}

fn state(session: &Session) -> Value {
    session.replay_value().expect("replay value")["state"].clone()
}

fn summon_descriptors(session: &Session) -> Vec<Value> {
    session
        .legal_actions()
        .expect("legal actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "summon-minion")
        .map(|action| action.descriptor)
        .collect()
}

fn summon_cells(session: &Session) -> Vec<String> {
    let descriptors = summon_descriptors(session);
    let instance_id = descriptors
        .iter()
        .filter_map(|descriptor| descriptor["cardInstanceId"].as_str())
        .min()
        .expect("summon instance identity");
    descriptors
        .iter()
        .filter(|descriptor| descriptor["cardInstanceId"] == instance_id)
        .map(|descriptor| {
            descriptor["cell"]
                .as_str()
                .expect("summon destination")
                .to_owned()
        })
        .collect()
}

#[test]
fn spellcaster_should_pay_mana_and_summon_at_controlled_site() {
    let manifest = scenario_manifest(
        41,
        &site("earth", false),
        &minion(1, &thresholds(Some("earth"), 1)),
        &site("earth", false),
    );
    let mut session = first_main(&manifest);
    let before = state(&session);
    let caster_instance_id = before["players"]["north"]["avatar"]["card"]["instanceId"]
        .as_str()
        .expect("Avatar instance identity");
    let summons = summon_descriptors(&session);

    assert_eq!(summons.len(), 3);
    assert!(summons.iter().all(|descriptor| {
        descriptor["cell"] == "C4"
            && descriptor["casterInstanceId"] == caster_instance_id
            && descriptor["manaCost"] == 1
    }));

    let (summon, receipt) = accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    let after = state(&session);
    assert_eq!(after["players"]["north"]["mana"], 0);
    assert_eq!(after["realm"]["units"][0]["controller"], "north");
    assert_eq!(after["realm"]["units"][0]["location"], "C4");
    assert_eq!(after["realm"]["units"][0]["summoningSickness"], true);
    assert_eq!(receipt.events.len(), 1);
    assert_eq!(receipt.events[0].event_type, "minion-summoned");
    assert_eq!(receipt.events[0].payload["manaPaid"], 1);
    assert_eq!(receipt.events[0].payload["cardId"], summon["cardId"]);
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one catalog proof keeps payment privacy, checkpoint, PRNG, state, stale rejection, and replay together"
)]
fn aramos_should_discard_one_deterministic_random_hand_card_instead_of_mana() {
    let mut aramos = minion(3, &thresholds(Some("earth"), 1));
    aramos["discardRandomCardInsteadOfMana"] = json!(true);
    let manifest = scenario_manifest(417, &site("earth", false), &aramos, &site("earth", false));
    let mut session = first_main(&manifest);
    let before = state(&session);
    assert_eq!(before["players"]["north"]["mana"], 1);

    let actions = session.legal_actions().expect("Aramos legal actions");
    let summons: Vec<_> = actions
        .iter()
        .filter(|action| action.descriptor["kind"] == "summon-minion")
        .collect();
    assert!(!summons.is_empty());
    assert!(summons.iter().all(|action| {
        action.descriptor["manaCost"] == 0
            && action.descriptor["paymentMode"] == "random-card-discard"
            && action.label == "Summon north-minion at C4 (discard random card)"
    }));

    let action = summons[0];
    let cast_instance_id = action.descriptor["cardInstanceId"]
        .as_str()
        .expect("Aramos instance identity");
    let eligible = before["players"]["north"]["hand"]["atlas"]
        .as_array()
        .expect("Atlas hand")
        .iter()
        .map(|card| {
            (
                card["instanceId"]
                    .as_str()
                    .expect("Atlas hand identity")
                    .to_owned(),
                "atlas",
            )
        })
        .chain(
            before["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("Spellbook hand")
                .iter()
                .filter(|card| card["instanceId"] != cast_instance_id)
                .map(|card| {
                    (
                        card["instanceId"]
                            .as_str()
                            .expect("Spellbook hand identity")
                            .to_owned(),
                        "spellbook",
                    )
                }),
        )
        .collect::<Vec<_>>();
    let action_json = canonical_json(&action.descriptor).expect("canonical Aramos descriptor");
    assert!(
        eligible
            .iter()
            .all(|(instance_id, _)| !action_json.contains(instance_id))
    );
    let request = ActionRequest {
        action_id: action.action_id.to_string(),
        seat: action.seat,
        state_version: action.state_version,
    };
    let checkpoint = create_game_checkpoint(&session).expect("captured Aramos checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized Aramos checkpoint");
    let mut repeated = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed Aramos checkpoint"),
    )
    .expect("restored Aramos checkpoint");
    assert_eq!(
        session.session_hash().expect("original session hash"),
        repeated.session_hash().expect("restored session hash")
    );
    let StepResult::Accepted(first) = session.step(request.clone()).expect("first Aramos summon")
    else {
        panic!("engine-issued Aramos action must be accepted");
    };
    let StepResult::Accepted(second) = repeated
        .step(request.clone())
        .expect("repeated Aramos summon")
    else {
        panic!("repeated engine-issued Aramos action must be accepted");
    };
    assert_eq!(first, second);
    assert_eq!(
        first
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["card-discarded", "minion-summoned"]
    );
    assert_eq!(
        first.events[0].payload["sourceInstanceId"],
        cast_instance_id
    );
    assert_eq!(first.events[1].payload["manaPaid"], 0);
    assert_eq!(first.random_draws.len(), 1);
    assert_eq!(
        first.random_draws[0]["purpose"],
        "summon_random_card_discard_cost"
    );
    assert_eq!(
        first.random_draws[0]["domain"]["kind"],
        "card_index_candidate"
    );
    assert_eq!(
        first.random_draws[0]["domain"]["exclusiveMaximum"],
        eligible.len()
    );

    let discarded_instance_id = first.events[0].payload["instanceId"].clone();
    let accepted_draw = first
        .random_draws
        .iter()
        .find(|draw| draw["domain"]["accepted"] == true)
        .expect("accepted random discard draw");
    let selected_index = usize::try_from(
        accepted_draw["result"]
            .as_u64()
            .expect("random discard result")
            % u64::try_from(eligible.len()).expect("eligible count"),
    )
    .expect("selected payment index");
    assert_eq!(
        first.events[0].payload["instanceId"],
        eligible[selected_index].0
    );
    assert_eq!(first.events[0].payload["zone"], eligible[selected_index].1);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["mana"], 1);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == discarded_instance_id)
    );
    assert!(
        after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == cast_instance_id)
    );
    let StepResult::Rejected(stale) = session.step(request).expect("stale Aramos request") else {
        panic!("reused Aramos request must be stale");
    };
    assert_eq!(stale.code, RejectionCode::StaleVersion);
    assert!(session.verify_replay().expect("verified Aramos replay"));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one catalog proof keeps sacrifice eligibility, checkpoint, state, stale rejection, and replay together"
)]
fn gnarled_wendigo_should_sacrifice_only_local_surface_allies_before_paying_mana() {
    let manifest = wendigo_manifest(6, 1, false);
    let mut session = wendigo_main(&manifest, true);
    let before = state(&session);
    assert_eq!(before["players"]["north"]["mana"], 4);
    let units = before["realm"]["units"].as_array().expect("realm units");
    let mut local_ids = units
        .iter()
        .filter(|unit| {
            ["north-local-a", "north-local-b"].contains(&unit["cardId"].as_str().expect("card id"))
        })
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("local identity")
                .to_owned()
        })
        .collect::<Vec<_>>();
    local_ids.sort_unstable();
    let buried_id = units
        .iter()
        .find(|unit| unit["cardId"] == "north-burrower")
        .expect("buried ally")["instanceId"]
        .as_str()
        .expect("buried identity");
    let enemy_id = units
        .iter()
        .find(|unit| unit["cardId"] == "south-enemy")
        .expect("enemy minion")["instanceId"]
        .as_str()
        .expect("enemy identity");
    assert_eq!(local_ids.len(), 2);
    assert_eq!(
        units
            .iter()
            .find(|unit| unit["instanceId"] == buried_id)
            .expect("buried ally")["region"],
        "underground"
    );

    let actions = session.legal_actions().expect("Wendigo legal actions");
    let wendigo_actions = actions
        .iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-wendigo"
                && action.descriptor["cell"] == "C4"
        })
        .collect::<Vec<_>>();
    assert!(!wendigo_actions.iter().any(|action| {
        action.descriptor["manaCost"] == 6
            && action.descriptor["sacrificedMinionInstanceIds"].is_null()
    }));
    let discounted = wendigo_actions
        .iter()
        .filter(|action| action.descriptor["manaCost"] == 4)
        .collect::<Vec<_>>();
    assert_eq!(
        discounted
            .iter()
            .map(|action| action.descriptor["sacrificedMinionInstanceIds"].clone())
            .collect::<Vec<_>>(),
        local_ids
            .iter()
            .map(|instance_id| json!([instance_id]))
            .collect::<Vec<_>>()
    );
    let double_discounted = wendigo_actions
        .iter()
        .filter(|action| action.descriptor["manaCost"] == 2)
        .collect::<Vec<_>>();
    assert_eq!(double_discounted.len(), 1);
    assert_eq!(
        double_discounted[0].descriptor["sacrificedMinionInstanceIds"],
        json!(local_ids)
    );
    let descriptors = canonical_json(&Value::Array(
        wendigo_actions
            .iter()
            .map(|action| action.descriptor.clone())
            .collect(),
    ))
    .expect("canonical Wendigo descriptors");
    assert!(!descriptors.contains(buried_id));
    assert!(!descriptors.contains(enemy_id));

    let cast = discounted[0];
    assert_eq!(
        cast.label,
        "Summon north-wendigo at C4 (4 mana + sacrifice 1 minion)"
    );
    let sacrificed_id = cast.descriptor["sacrificedMinionInstanceIds"][0]
        .as_str()
        .expect("sacrifice identity")
        .to_owned();
    let sacrificed_card_id = units
        .iter()
        .find(|unit| unit["instanceId"] == sacrificed_id)
        .expect("sacrificed unit")["cardId"]
        .clone();
    let source_id = cast.descriptor["cardInstanceId"]
        .as_str()
        .expect("Wendigo identity")
        .to_owned();
    let request = ActionRequest {
        action_id: cast.action_id.to_string(),
        seat: cast.seat,
        state_version: cast.state_version,
    };
    let checkpoint = create_game_checkpoint(&session).expect("captured Wendigo checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized checkpoint");
    let mut repeated = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed Wendigo checkpoint"),
    )
    .expect("restored Wendigo checkpoint");
    assert_eq!(
        session.session_hash().expect("source session hash"),
        repeated.session_hash().expect("restored session hash")
    );
    assert_eq!(
        session.legal_actions().expect("source actions"),
        repeated.legal_actions().expect("restored actions")
    );

    let StepResult::Accepted(first) = session.step(request.clone()).expect("Wendigo summon") else {
        panic!("engine-issued Wendigo summon must be accepted");
    };
    let StepResult::Accepted(second) = repeated
        .step(request.clone())
        .expect("repeated Wendigo summon")
    else {
        panic!("repeated Wendigo summon must be accepted");
    };
    assert_eq!(first, second);
    assert!(first.random_draws.is_empty());
    assert_eq!(
        first
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["minion-sacrificed", "minion-died", "minion-summoned"]
    );
    assert_eq!(
        first.events[0].payload,
        json!({
            "cardId": sacrificed_card_id,
            "instanceId": sacrificed_id,
            "owner": "north",
            "seat": "north",
            "sourceInstanceId": source_id,
        })
    );
    assert_eq!(first.events[2].payload["manaPaid"], 4);
    let after = state(&session);
    assert_eq!(after["players"]["north"]["mana"], 0);
    assert!(
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("north cemetery")
            .iter()
            .any(|card| card["instanceId"] == sacrificed_id)
    );
    assert!(
        !after["realm"]["units"]
            .as_array()
            .expect("realm units")
            .iter()
            .any(|unit| unit["instanceId"] == sacrificed_id)
    );
    let wendigo = after["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .find(|unit| unit["instanceId"] == source_id)
        .expect("summoned Wendigo");
    assert_eq!(wendigo["location"], "C4");
    assert_eq!(wendigo["region"], "surface");
    let StepResult::Rejected(stale) = session.step(request).expect("stale Wendigo request") else {
        panic!("reused Wendigo request must be stale");
    };
    assert_eq!(stale.code, RejectionCode::StaleVersion);
    assert!(session.verify_replay().expect("verified Wendigo replay"));
}

#[test]
fn gnarled_wendigo_should_offer_normal_and_only_useful_sacrifice_payments() {
    let manifest = wendigo_manifest(4, 1, false);
    let session = wendigo_main(&manifest, false);
    let before = state(&session);
    let units = before["realm"]["units"].as_array().expect("realm units");
    let mut eligible = units
        .iter()
        .filter(|unit| unit["controller"] == "north" && unit["location"] == "C4")
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("ally identity")
                .to_owned()
        })
        .collect::<Vec<_>>();
    eligible.sort_unstable();
    assert_eq!(eligible.len(), 3);
    let actions = session
        .legal_actions()
        .expect("Wendigo legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-wendigo"
                && action.descriptor["cell"] == "C4"
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actions
            .iter()
            .filter(|action| {
                action.descriptor["manaCost"] == 4
                    && action.descriptor["sacrificedMinionInstanceIds"].is_null()
            })
            .count(),
        1
    );
    assert_eq!(
        actions
            .iter()
            .filter(|action| {
                action.descriptor["manaCost"] == 2
                    && action.descriptor["sacrificedMinionInstanceIds"]
                        .as_array()
                        .is_some_and(|ids| ids.len() == 1)
            })
            .count(),
        3
    );
    let zero_cost = actions
        .iter()
        .filter(|action| action.descriptor["manaCost"] == 0)
        .collect::<Vec<_>>();
    assert_eq!(zero_cost.len(), 3);
    let expected_pairs = (0..eligible.len())
        .flat_map(|left| {
            let eligible = &eligible;
            ((left + 1)..eligible.len()).map(move |right| json!([eligible[left], eligible[right]]))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        zero_cost
            .iter()
            .map(|action| action.descriptor["sacrificedMinionInstanceIds"].clone())
            .collect::<Vec<_>>(),
        expected_pairs
    );
    assert!(actions.iter().all(|action| {
        action.descriptor["sacrificedMinionInstanceIds"]
            .as_array()
            .is_none_or(|ids| ids.len() <= 2)
    }));
    assert!(session.verify_replay().expect("verified Wendigo setup"));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the ordered payment proof keeps the pending checkpoint and both converging Deathrite branches together"
)]
fn gnarled_wendigo_payment_deathrites_should_resume_one_summon_and_converge() {
    let manifest = wendigo_manifest(6, 1, true);
    let mut session = wendigo_main(&manifest, true);
    let before = state(&session);
    let mut local_ids = before["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| {
            ["north-local-a", "north-local-b"].contains(&unit["cardId"].as_str().expect("card id"))
        })
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("local identity")
                .to_owned()
        })
        .collect::<Vec<_>>();
    local_ids.sort_unstable();
    let payment = session
        .legal_actions()
        .expect("Wendigo payment actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-wendigo"
                && action.descriptor["cell"] == "C4"
                && action.descriptor["manaCost"] == 2
                && action.descriptor["sacrificedMinionInstanceIds"] == json!(local_ids)
        })
        .expect("double-sacrifice payment");
    let StepResult::Accepted(interrupted) = session
        .step(ActionRequest {
            action_id: payment.action_id.to_string(),
            seat: payment.seat,
            state_version: payment.state_version,
        })
        .expect("ordered Wendigo payment")
    else {
        panic!("engine-issued ordered Wendigo payment must be accepted");
    };
    assert_eq!(
        interrupted
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["minion-sacrificed", "minion-sacrificed"]
    );
    let pending = state(&session);
    assert_eq!(pending["phase"], "deathrite-order");
    assert_eq!(pending["decisionSeat"], "north");
    assert_eq!(pending["players"]["north"]["mana"], 2);
    assert!(
        !pending["realm"]["units"]
            .as_array()
            .expect("pending units")
            .iter()
            .any(|unit| unit["cardId"] == "north-wendigo")
    );
    assert!(local_ids.iter().all(|instance_id| {
        !pending["players"]["north"]["cemetery"]
            .as_array()
            .expect("pending cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id.as_str())
    }));

    let checkpoint = create_game_checkpoint(&session).expect("pending Wendigo checkpoint");
    let serialized = serialize_game_checkpoint(&checkpoint).expect("serialized pending checkpoint");
    let restored = resume_game_checkpoint(
        &parse_game_checkpoint(&serialized).expect("parsed pending checkpoint"),
    )
    .expect("restored pending checkpoint");
    assert_eq!(
        restored.replay_value().expect("restored pending state"),
        session.replay_value().expect("source pending state")
    );
    let order_actions = restored
        .legal_actions()
        .expect("Deathrite order actions")
        .into_iter()
        .filter(|action| action.descriptor["kind"] == "order-deathrites")
        .collect::<Vec<_>>();
    assert_eq!(
        order_actions
            .iter()
            .map(|action| action.descriptor["sourceInstanceId"]
                .as_str()
                .expect("Deathrite source")
                .to_owned())
            .collect::<Vec<_>>(),
        local_ids
    );

    let mut branch_hashes = Vec::new();
    for order_action in order_actions {
        let chosen_id = order_action.descriptor["sourceInstanceId"]
            .as_str()
            .expect("chosen Deathrite source")
            .to_owned();
        let other_id = local_ids
            .iter()
            .find(|instance_id| instance_id.as_str() != chosen_id)
            .expect("other Deathrite source")
            .to_owned();
        let mut branch = resume_game_checkpoint(
            &parse_game_checkpoint(&serialized).expect("parsed branch checkpoint"),
        )
        .expect("restored branch checkpoint");
        let StepResult::Accepted(resolved) = branch
            .step(ActionRequest {
                action_id: order_action.action_id.to_string(),
                seat: order_action.seat,
                state_version: order_action.state_version,
            })
            .expect("resolve Deathrite order")
        else {
            panic!("engine-issued Deathrite order must be accepted");
        };
        assert_eq!(
            resolved
                .events
                .iter()
                .map(|event| event.event_type.as_str())
                .collect::<Vec<_>>(),
            [
                "deathrite-order-committed",
                "site-drawn",
                "site-drawn",
                "minion-died",
                "minion-died",
                "minion-summoned",
            ]
        );
        assert_eq!(
            resolved
                .events
                .iter()
                .filter(|event| event.event_type == "site-drawn")
                .map(|event| event.payload["sourceInstanceId"]
                    .as_str()
                    .expect("draw source")
                    .to_owned())
                .collect::<Vec<_>>(),
            [chosen_id, other_id]
        );
        let after = state(&branch);
        assert_eq!(after["phase"], "main");
        assert_eq!(after["decisionSeat"], "north");
        assert_eq!(after["pendingDeathrites"], Value::Null);
        assert_eq!(after["players"]["north"]["mana"], 2);
        assert_eq!(
            after["players"]["north"]["atlas"]
                .as_array()
                .expect("Atlas")
                .len(),
            1
        );
        assert_eq!(
            after["players"]["north"]["hand"]["atlas"]
                .as_array()
                .expect("Atlas hand")
                .len(),
            2
        );
        assert!(local_ids.iter().all(|instance_id| {
            after["players"]["north"]["cemetery"]
                .as_array()
                .expect("resolved cemetery")
                .iter()
                .any(|card| card["instanceId"] == instance_id.as_str())
        }));
        assert_eq!(
            after["realm"]["units"]
                .as_array()
                .expect("resolved units")
                .iter()
                .filter(|unit| unit["cardId"] == "north-wendigo")
                .count(),
            1
        );
        assert!(branch.verify_replay().expect("verified Deathrite branch"));
        branch_hashes.push(identity_hash(&after).expect("resolved branch identity"));
    }
    assert_eq!(branch_hashes[0], branch_hashes[1]);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the terminal payment proof keeps deck exhaustion, deferred summon cancellation, cemetery state, and replay together"
)]
fn gnarled_wendigo_terminal_deathrite_should_end_before_deferred_summon() {
    let manifest = wendigo_manifest(6, 1, true);
    let mut session = wendigo_main(&manifest, true);
    for _ in 0..2 {
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        accept_where(&mut session, |descriptor| descriptor["kind"] == "end-turn");
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
    }
    let before = state(&session);
    assert_eq!(
        before["players"]["north"]["atlas"]
            .as_array()
            .expect("North Atlas")
            .len(),
        1
    );
    let mana_before = before["players"]["north"]["mana"]
        .as_u64()
        .expect("North mana");
    let mut local_ids = before["realm"]["units"]
        .as_array()
        .expect("realm units")
        .iter()
        .filter(|unit| {
            ["north-local-a", "north-local-b"].contains(&unit["cardId"].as_str().expect("card id"))
        })
        .map(|unit| {
            unit["instanceId"]
                .as_str()
                .expect("local identity")
                .to_owned()
        })
        .collect::<Vec<_>>();
    local_ids.sort_unstable();
    let payment = session
        .legal_actions()
        .expect("terminal Wendigo payment actions")
        .into_iter()
        .find(|action| {
            action.descriptor["kind"] == "summon-minion"
                && action.descriptor["cardId"] == "north-wendigo"
                && action.descriptor["cell"] == "C4"
                && action.descriptor["manaCost"] == 2
                && action.descriptor["sacrificedMinionInstanceIds"] == json!(local_ids)
        })
        .expect("terminal double-sacrifice payment");
    let StepResult::Accepted(interrupted) = session
        .step(ActionRequest {
            action_id: payment.action_id.to_string(),
            seat: payment.seat,
            state_version: payment.state_version,
        })
        .expect("terminal Wendigo payment")
    else {
        panic!("engine-issued terminal Wendigo payment must be accepted");
    };
    assert_eq!(
        interrupted
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        ["minion-sacrificed", "minion-sacrificed"]
    );
    assert_eq!(state(&session)["phase"], "deathrite-order");
    let order = session
        .legal_actions()
        .expect("terminal Deathrite order actions")
        .into_iter()
        .find(|action| action.descriptor["kind"] == "order-deathrites")
        .expect("terminal Deathrite order");
    let StepResult::Accepted(terminal) = session
        .step(ActionRequest {
            action_id: order.action_id.to_string(),
            seat: order.seat,
            state_version: order.state_version,
        })
        .expect("terminal Deathrite resolution")
    else {
        panic!("engine-issued terminal Deathrite order must be accepted");
    };
    assert_eq!(
        terminal
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "deathrite-order-committed",
            "site-drawn",
            "minion-died",
            "minion-died",
            "game-ended",
        ]
    );
    assert!(
        !terminal
            .events
            .iter()
            .any(|event| event.event_type == "minion-summoned")
    );
    let after = state(&session);
    assert_eq!(after["phase"], "terminal");
    assert_eq!(after["pendingDeathrites"], Value::Null);
    assert_eq!(
        after["terminal"],
        json!({
            "loser": "north",
            "reason": "deck_empty",
            "status": "finished",
            "winner": "south",
        })
    );
    assert_eq!(after["players"]["north"]["mana"], mana_before - 2);
    assert!(local_ids.iter().all(|instance_id| {
        after["players"]["north"]["cemetery"]
            .as_array()
            .expect("terminal cemetery")
            .iter()
            .any(|card| card["instanceId"] == instance_id.as_str())
    }));
    assert!(
        !after["realm"]["units"]
            .as_array()
            .expect("terminal units")
            .iter()
            .any(|unit| unit["cardId"] == "north-wendigo")
    );
    assert!(
        !after["players"]["north"]["hand"]["spellbook"]
            .as_array()
            .expect("terminal Spellbook hand")
            .iter()
            .any(|card| card["cardId"] == "north-wendigo")
    );
    assert!(
        !after["players"]["north"]["cemetery"]
            .as_array()
            .expect("terminal cemetery")
            .iter()
            .any(|card| card["cardId"] == "north-wendigo")
    );
    assert!(session.verify_replay().expect("verified terminal replay"));
}

#[test]
fn hamlet_should_discount_only_ordinary_minions_at_that_site() {
    let mut ordinary = minion(1, &thresholds(None, 0));
    ordinary["ordinary"] = json!(true);
    let manifest = scenario_manifest(17, &site("earth", true), &ordinary, &site("earth", false));
    let mut discounted = first_main(&manifest);

    assert!(
        summon_descriptors(&discounted)
            .iter()
            .all(|descriptor| descriptor["manaCost"] == 0)
    );
    let (_, receipt) = accept_where(&mut discounted, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["manaCost"] == 0
    });
    assert_eq!(state(&discounted)["players"]["north"]["mana"], 1);
    assert_eq!(receipt.events[0].payload["manaPaid"], 0);

    let manifest = scenario_manifest(
        18,
        &site("earth", true),
        &minion(1, &thresholds(None, 0)),
        &site("earth", false),
    );
    let full_price = first_main(&manifest);
    assert!(
        summon_descriptors(&full_price)
            .iter()
            .all(|descriptor| descriptor["manaCost"] == 1)
    );
    assert!(discounted.verify_replay().expect("verified replay"));
}

fn hamlet_board_manifest(seed: u32, north_minion: &Value, helpers: bool) -> String {
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let mut ordinary_site = site("earth", false);
    if helpers {
        ordinary_site["genesisGainMana"] = json!(6);
    }
    let mut cards = json!({
        "hamlet": site("earth", true),
        "north-avatar": avatar,
        "north-minion": north_minion,
        "ordinary-site": ordinary_site,
        "south-avatar": avatar,
        "south-minion": minion(0, &thresholds(None, 0)),
    });
    let north_spellbook = if helpers {
        cards["helper"] = minion(0, &thresholds(None, 0));
        vec!["helper", "helper", "helper", "north-minion"]
    } else {
        vec!["north-minion"; 4]
    };
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "hamlet-payment-matrix" }))
                .expect("synthetic authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-hamlet-payment-matrix-v1",
        },
        "cards": cards,
        "decks": {
            "north": {
                "atlas": ["hamlet", "ordinary-site", "hamlet", "ordinary-site"],
                "avatar": "north-avatar",
                "spellbook": north_spellbook,
            },
            "south": {
                "atlas": vec!["hamlet"; 4],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 4],
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

fn try_accept(session: &mut Session, predicate: impl Fn(&Value) -> bool) -> bool {
    let Ok(actions) = session.legal_actions() else {
        return false;
    };
    let Some(action) = actions
        .into_iter()
        .find(|action| predicate(&action.descriptor))
    else {
        return false;
    };
    matches!(
        session.step(ActionRequest {
            action_id: action.action_id.to_string(),
            seat: action.seat,
            state_version: action.state_version,
        }),
        Ok(StepResult::Accepted(_))
    )
}

fn hamlet_three_sites(north_minion: &Value, helpers: bool) -> Session {
    (1..=512)
        .find_map(|seed| {
            let manifest = hamlet_board_manifest(seed, north_minion, helpers);
            let mut session = Session::new(&manifest).ok()?;
            keep(&mut session);
            keep(&mut session);
            if !try_accept(&mut session, |descriptor| {
                descriptor["kind"] == "play-site"
                    && descriptor["cardId"] == "hamlet"
                    && descriptor["cell"] == "C4"
            }) {
                return None;
            }
            if !try_accept(&mut session, |descriptor| descriptor["kind"] == "end-turn") {
                return None;
            }
            if !try_accept(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
            }) {
                return None;
            }
            if !try_accept(&mut session, |descriptor| {
                descriptor["kind"] == "play-site"
                    && descriptor["cardId"] == "hamlet"
                    && descriptor["cell"] == "C1"
            }) {
                return None;
            }
            if !try_accept(&mut session, |descriptor| descriptor["kind"] == "end-turn") {
                return None;
            }
            if !try_accept(&mut session, |descriptor| {
                descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
            }) {
                return None;
            }
            if !try_accept(&mut session, |descriptor| {
                descriptor["kind"] == "play-site"
                    && descriptor["cardId"] == "ordinary-site"
                    && descriptor["cell"] == "C3"
            }) {
                return None;
            }
            Some(session)
        })
        .expect("bounded seed with Hamlet, enemy Hamlet, and an ordinary site")
}

fn summon_costs(session: &Session, card_id: &str) -> Vec<(String, u64, String)> {
    summon_descriptors(session)
        .into_iter()
        .filter(|descriptor| descriptor["cardId"] == card_id)
        .map(|descriptor| {
            (
                descriptor["cell"].as_str().expect("summon cell").to_owned(),
                descriptor["manaCost"].as_u64().expect("mana cost"),
                descriptor["paymentMode"]
                    .as_str()
                    .unwrap_or("mana")
                    .to_owned(),
            )
        })
        .collect()
}

#[test]
fn hamlet_should_discount_only_ordinary_payments_across_sites_and_payment_modes() {
    let mut ordinary = minion(1, &thresholds(None, 0));
    ordinary["ordinary"] = json!(true);
    let ordinary_board = hamlet_three_sites(&ordinary, false);
    let mut ordinary_costs = summon_costs(&ordinary_board, "north-minion");
    ordinary_costs.sort();
    ordinary_costs.dedup();
    assert_eq!(
        ordinary_costs,
        [
            ("C3".to_owned(), 1, "mana".to_owned()),
            ("C4".to_owned(), 0, "mana".to_owned()),
        ]
    );

    let nonordinary_board = hamlet_three_sites(&minion(1, &thresholds(None, 0)), false);
    let mut nonordinary_costs = summon_costs(&nonordinary_board, "north-minion");
    nonordinary_costs.sort();
    nonordinary_costs.dedup();
    assert_eq!(
        nonordinary_costs,
        [
            ("C3".to_owned(), 1, "mana".to_owned()),
            ("C4".to_owned(), 1, "mana".to_owned()),
        ]
    );

    let mut roaming = minion(1, &thresholds(None, 0));
    roaming["ordinary"] = json!(true);
    roaming["summonToAnySite"] = json!(true);
    let roaming_board = hamlet_three_sites(&roaming, false);
    assert!(
        summon_costs(&roaming_board, "north-minion")
            .iter()
            .any(|(cell, mana_cost, mode)| cell == "C1" && *mana_cost == 0 && mode == "mana")
    );

    let mut aramos = minion(3, &thresholds(None, 0));
    aramos["discardRandomCardInsteadOfMana"] = json!(true);
    aramos["ordinary"] = json!(true);
    let aramos_board = hamlet_three_sites(&aramos, false);
    let mut aramos_costs = summon_costs(&aramos_board, "north-minion")
        .into_iter()
        .map(|(cell, mana_cost, mode)| format!("{cell}:{mana_cost}:{mode}"))
        .collect::<Vec<_>>();
    aramos_costs.sort();
    aramos_costs.dedup();
    assert_eq!(
        aramos_costs,
        [
            "C3:0:random-card-discard".to_owned(),
            "C4:0:random-card-discard".to_owned(),
            "C4:2:mana".to_owned(),
        ]
    );

    let mut gnarled = minion(6, &thresholds(None, 0));
    gnarled["sacrificeMinionAtSummoningLocationForManaDiscount"] = json!(2);
    let mut gnarled_board = hamlet_three_sites(&gnarled, true);
    for _ in 0..3 {
        assert!(try_accept(&mut gnarled_board, |descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == "helper"
                && descriptor["cell"] == "C4"
        }));
    }
    let mut gnarled_costs = summon_descriptors(&gnarled_board)
        .into_iter()
        .filter(|descriptor| descriptor["cardId"] == "north-minion" && descriptor["cell"] == "C4")
        .map(|descriptor| {
            format!(
                "{}:{}",
                descriptor["sacrificedMinionInstanceIds"]
                    .as_array()
                    .map_or(0, Vec::len),
                descriptor["manaCost"].as_u64().expect("gnarled mana cost")
            )
        })
        .collect::<Vec<_>>();
    gnarled_costs.sort();
    gnarled_costs.dedup();
    assert_eq!(
        gnarled_costs,
        [
            "0:6".to_owned(),
            "1:4".to_owned(),
            "2:2".to_owned(),
            "3:0".to_owned()
        ]
    );
    assert!(
        ordinary_board
            .verify_replay()
            .expect("verified ordinary Hamlet replay")
    );
}

#[test]
fn explicit_permission_should_allow_summoning_to_any_site() {
    let ordinary_manifest = scenario_manifest(
        47,
        &site("earth", false),
        &minion(1, &thresholds(Some("earth"), 1)),
        &site("earth", false),
    );
    let ordinary = north_second_main(first_main(&ordinary_manifest));
    assert_eq!(summon_cells(&ordinary), ["C4"]);

    let mut roaming = minion(1, &thresholds(Some("earth"), 1));
    roaming["summonToAnySite"] = json!(true);
    let roaming_manifest =
        scenario_manifest(48, &site("earth", false), &roaming, &site("earth", false));
    let mut unrestricted = north_second_main(first_main(&roaming_manifest));
    assert_eq!(summon_cells(&unrestricted), ["C1", "C4"]);
    accept_where(&mut unrestricted, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C1"
    });
    assert_eq!(state(&unrestricted)["realm"]["units"][0]["location"], "C1");
    assert!(unrestricted.verify_replay().expect("verified replay"));
}

#[test]
fn water_site_restriction_should_filter_unrestricted_summons_by_terrain() {
    let mut water_only = minion(1, &thresholds(Some("water"), 1));
    water_only["mustBeCastToWaterSite"] = json!(true);
    water_only["summonToAnySite"] = json!(true);
    let manifest = scenario_manifest(
        78,
        &site("water", false),
        &water_only,
        &site("earth", false),
    );
    let mut session = north_second_main(first_main(&manifest));

    assert_eq!(summon_cells(&session), ["C4"]);
    accept_where(&mut session, |descriptor| {
        descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
    });
    assert!(session.verify_replay().expect("verified replay"));
}

#[test]
fn mana_and_each_elemental_threshold_should_gate_without_spending_affinity() {
    let mana_gated = scenario_manifest(
        79,
        &site("earth", false),
        &minion(2, &thresholds(Some("earth"), 1)),
        &site("earth", false),
    );
    assert!(summon_descriptors(&first_main(&mana_gated)).is_empty());

    let threshold_gated = scenario_manifest(
        80,
        &site("earth", false),
        &minion(1, &thresholds(Some("earth"), 2)),
        &site("earth", false),
    );
    assert!(summon_descriptors(&first_main(&threshold_gated)).is_empty());

    for (seed, element) in (81..).zip(["air", "earth", "fire", "water"]) {
        let manifest = scenario_manifest(
            seed,
            &site(element, false),
            &minion(2, &thresholds(Some(element), 2)),
            &site("earth", false),
        );
        let mut session = north_second_main(first_main(&manifest));
        assert!(summon_descriptors(&session).is_empty());
        accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
        });
        assert_eq!(summon_cells(&session), ["C3", "C4"]);
        let (_, receipt) = accept_where(&mut session, |descriptor| {
            descriptor["kind"] == "summon-minion" && descriptor["cell"] == "C4"
        });
        let after = state(&session);
        assert_eq!(after["players"]["north"]["mana"], 0);
        assert_eq!(after["realm"]["sites"]["C3"]["controller"], "north");
        assert_eq!(after["realm"]["sites"]["C4"]["controller"], "north");
        assert_eq!(receipt.events[0].payload["manaPaid"], 2);
        assert!(session.verify_replay().expect("verified replay"));
    }
}
