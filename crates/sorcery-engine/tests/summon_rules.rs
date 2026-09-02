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
