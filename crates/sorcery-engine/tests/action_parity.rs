use std::collections::BTreeSet;

use serde_json::{Value, json};
use sorcery_engine::action::{ActionDescriptor, CombatTarget, DeckZone};
use sorcery_engine::canonical::{IdentityHash, canonical_json, identity_hash};
use sorcery_engine::checkpoint::{
    create_game_checkpoint, parse_game_checkpoint, resume_game_checkpoint,
    serialize_game_checkpoint,
};
use sorcery_engine::contract::{ActionRequest, Seat, opaque_action_id};
use sorcery_engine::session::{Session, StepResult};

const FIXTURE: &str = include_str!("../../../tests/engine/fixtures/typescript-parity-v1.json");
const CAST_MAGIC_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/cast-magic-action-v1.json");
const COMBAT_RESPONSE_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/combat-response-action-v1.json");
const DEATHRITE_ORDER_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/deathrite-order-action-v1.json");
const DUEL_FIXTURE: &str = include_str!("../../../tests/engine/fixtures/duel-action-v1.json");
const SHOOT_PROJECTILE_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/shoot-projectile-action-v1.json");
const SHOOT_DAMAGE_PROJECTILE_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/shoot-damage-projectile-action-v1.json");
const SPARKMAGE_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/sparkmage-action-v1.json");
const SITE_DESTRUCTION_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/site-destruction-action-v1.json");
const RANDOM_CARD_DISCARD_SUMMON_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/random-card-discard-summon-action-v1.json");
const SACRIFICE_SUMMON_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/sacrifice-summon-action-v1.json");
const NORTH_AVATAR: &str =
    "sha256:310a489a62739a8b1a6a13bf949daa8dc42ab0995619e5288691a0ac86a2472e";

fn descriptor_kind(descriptor: &ActionDescriptor) -> &'static str {
    match descriptor {
        ActionDescriptor::ActivateAreaDamage { .. } => "activate-area-damage",
        ActionDescriptor::ActivateDiscardRandomDamage { .. } => "activate-discard-random-damage",
        ActionDescriptor::ActivateMana { .. } => "activate-mana",
        ActionDescriptor::ActivateSparkmage { .. } => "activate-sparkmage",
        ActionDescriptor::AllocateStrike { .. } => "allocate-strike",
        ActionDescriptor::BeginChainMagic { .. } => "begin-chain-magic",
        ActionDescriptor::CastArtifact { .. } => "cast-artifact",
        ActionDescriptor::CastMagic { .. } => "cast-magic",
        ActionDescriptor::DropArtifacts { .. } => "drop-artifacts",
        ActionDescriptor::PickUpArtifacts { .. } => "pick-up-artifacts",
        ActionDescriptor::Mulligan { .. } => "mulligan",
        ActionDescriptor::Draw { .. } => "draw",
        ActionDescriptor::DrawSite => "draw-site",
        ActionDescriptor::DrawSpell => "draw-spell",
        ActionDescriptor::PlaySite { .. } => "play-site",
        ActionDescriptor::ReplaceRubbleWithTopAtlasSite { .. } => {
            "replace-rubble-with-top-atlas-site"
        }
        ActionDescriptor::ActivateSiteDestruction { .. } => "activate-site-destruction",
        ActionDescriptor::ResolveGenesisSpell { .. } => "resolve-genesis-spell",
        ActionDescriptor::ResolveGenesisSpellOrder { .. } => "resolve-genesis-spell-order",
        ActionDescriptor::ResolveGenesisToken { .. } => "resolve-genesis-token",
        ActionDescriptor::ResolveRangedStep { .. } => "resolve-ranged-step",
        ActionDescriptor::SummonMinion { .. } => "summon-minion",
        ActionDescriptor::MoveAndAttack { .. } => "move-and-attack",
        ActionDescriptor::ContinueBasicMovement { .. } => "continue-basic-movement",
        ActionDescriptor::DeclineAttack => "decline-attack",
        ActionDescriptor::DeclareAttack { .. } => "declare-attack",
        ActionDescriptor::Defend { .. } => "defend",
        ActionDescriptor::CloseDefend { .. } => "close-defend",
        ActionDescriptor::Intercept { .. } => "intercept",
        ActionDescriptor::OrderDeathrites { .. } => "order-deathrites",
        ActionDescriptor::ShootProjectile { .. } => "shoot-projectile",
        ActionDescriptor::ShootDamageProjectile { .. } => "shoot-damage-projectile",
        ActionDescriptor::ShootDragProjectile { .. } => "shoot-drag-projectile",
        ActionDescriptor::CloseIntercept {} => "close-intercept",
        ActionDescriptor::EndTurn => "end-turn",
        ActionDescriptor::ExtendChainMagic { .. } => "extend-chain-magic",
        ActionDescriptor::ResolveChainMagic => "resolve-chain-magic",
    }
}

#[test]
fn site_destruction_descriptors_labels_order_and_ids_should_match_typescript() {
    let fixture: Value =
        serde_json::from_str(SITE_DESTRUCTION_FIXTURE).expect("valid site destruction fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");
    let contract = fixture["contract"].as_str().expect("action contract");
    let seat: Seat = serde_json::from_value(fixture["seat"].clone()).expect("fixture seat");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let mut ordered = Vec::new();
    let mut labels = Vec::new();

    for action in fixture["actions"].as_array().expect("fixture actions") {
        let descriptor: ActionDescriptor = serde_json::from_value(action["descriptor"].clone())
            .expect("typed site destruction fixture descriptor");
        let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
        assert_eq!(serialized, action["descriptor"]);
        let expected_id =
            IdentityHash::parse(action["actionId"].as_str().expect("TypeScript action ID"))
                .expect("valid TypeScript action ID");
        assert_eq!(
            opaque_action_id(contract, seat, state_version, &serialized)
                .expect("Rust action identity"),
            expected_id
        );
        if matches!(descriptor, ActionDescriptor::ActivateSiteDestruction { .. }) {
            labels.push(
                descriptor
                    .state_independent_label()
                    .expect("site destruction label"),
            );
        }
        ordered.push((
            canonical_json(&serialized).expect("canonical descriptor"),
            expected_id,
        ));
    }

    ordered.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        ordered
            .into_iter()
            .map(|(_, action_id)| action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical TypeScript order")
            .iter()
            .map(|action_id| action_id.as_str().expect("action ID").to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        labels,
        [
            "Sacrifice site to destroy C4",
            "Sacrifice site to destroy C3",
            "Sacrifice site to destroy C2",
        ]
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one exact cross-engine fixture covers legal enumeration and its selected transition"
)]
fn random_card_discard_summon_descriptors_order_and_ids_should_match_typescript() {
    let fixture: Value = serde_json::from_str(RANDOM_CARD_DISCARD_SUMMON_FIXTURE)
        .expect("valid random-card-discard summon fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");
    let contract = fixture["contract"].as_str().expect("action contract");
    let seat: Seat = serde_json::from_value(fixture["seat"].clone()).expect("fixture seat");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let mut actions = Vec::new();

    for action in fixture["actions"].as_array().expect("fixture actions") {
        let descriptor: ActionDescriptor = serde_json::from_value(action["descriptor"].clone())
            .expect("typed random-card-discard summon descriptor");
        assert!(matches!(descriptor, ActionDescriptor::SummonMinion { .. }));
        let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
        assert_eq!(serialized, action["descriptor"]);
        let expected_id =
            IdentityHash::parse(action["actionId"].as_str().expect("TypeScript action ID"))
                .expect("valid TypeScript action ID");
        assert_eq!(
            opaque_action_id(contract, seat, state_version, &serialized)
                .expect("Rust action identity"),
            expected_id
        );
        actions.push((
            canonical_json(&serialized).expect("canonical summon descriptor"),
            expected_id,
        ));
    }

    actions.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    assert_eq!(
        actions
            .into_iter()
            .map(|(_, action_id)| action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical TypeScript order")
            .iter()
            .map(|action_id| action_id.as_str().expect("action ID").to_owned())
            .collect::<Vec<_>>()
    );

    let zero = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "synthetic-random-discard-action-v1",
            }))
            .expect("synthetic fixture authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-random-discard-action-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-site": { "cardType": "site", "elements": ["fire"] },
            "south-avatar": avatar,
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": zero,
            },
            "south-site": { "cardType": "site", "elements": ["earth"] },
            "synthetic-random-discard-minion": {
                "attack": 7,
                "cardType": "minion",
                "defense": 5,
                "discardRandomCardInsteadOfMana": true,
                "manaCost": 2,
                "thresholds": { "air": 0, "earth": 0, "fire": 1, "water": 0 },
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": vec!["synthetic-random-discard-minion"; 3],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 3],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 417,
    });
    manifest["manifestId"] = json!(identity_hash(&manifest).expect("synthetic manifest identity"));
    assert_eq!(manifest["manifestId"], fixture["manifestId"]);
    let manifest = canonical_json(&manifest).expect("canonical synthetic fixture manifest");
    let mut session = Session::new(&manifest).expect("valid synthetic fixture manifest");
    let mut accept_where = |predicate: &dyn Fn(&Value) -> bool| {
        let action = session
            .legal_actions()
            .expect("fixture legal actions")
            .into_iter()
            .find(|action| predicate(&action.descriptor))
            .expect("fixture setup action");
        let StepResult::Accepted(_) = session
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .expect("fixture setup step")
        else {
            panic!("engine-issued fixture action must be accepted");
        };
    };
    let keep = |descriptor: &Value| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    };
    let end_turn = |descriptor: &Value| descriptor["kind"] == "end-turn";
    accept_where(&keep);
    accept_where(&keep);
    accept_where(&|descriptor| descriptor["kind"] == "play-site" && descriptor["cell"] == "C4");
    accept_where(&end_turn);
    accept_where(&|descriptor| descriptor["kind"] == "draw" && descriptor["zone"] == "atlas");
    accept_where(&|descriptor| descriptor["kind"] == "play-site" && descriptor["cell"] == "C1");
    accept_where(&end_turn);
    accept_where(&|descriptor| descriptor["kind"] == "draw" && descriptor["zone"] == "atlas");
    accept_where(&|descriptor| descriptor["kind"] == "play-site" && descriptor["cell"] == "C3");

    let casting_instance_id = fixture["actions"][0]["descriptor"]["cardInstanceId"]
        .as_str()
        .expect("fixture casting instance identity");
    let issued = session
        .legal_actions()
        .expect("Rust random-discard legal actions")
        .into_iter()
        .filter(|action| action.descriptor["cardInstanceId"] == casting_instance_id)
        .collect::<Vec<_>>();
    assert_eq!(
        issued
            .iter()
            .map(|action| json!({
                "actionId": action.action_id,
                "descriptor": action.descriptor,
                "label": action.label,
            }))
            .collect::<Vec<_>>(),
        fixture["actions"]
            .as_array()
            .expect("fixture random-discard actions")
            .clone()
    );

    let state = session.replay_value().expect("Rust fixture replay state")["state"].clone();
    let hand = &state["players"]["north"]["hand"];
    let eligible_candidates = hand["atlas"]
        .as_array()
        .expect("fixture Atlas hand")
        .iter()
        .map(|card| json!({ "instanceId": card["instanceId"], "zone": "atlas" }))
        .chain(
            hand["spellbook"]
                .as_array()
                .expect("fixture Spellbook hand")
                .iter()
                .filter(|card| card["instanceId"] != casting_instance_id)
                .map(|card| json!({ "instanceId": card["instanceId"], "zone": "spellbook" })),
        )
        .collect::<Vec<_>>();
    assert!(eligible_candidates.len() > 1);
    assert_eq!(
        eligible_candidates,
        fixture["transition"]["eligibleCandidates"]
            .as_array()
            .expect("fixture discard candidates")
            .clone()
    );

    let selected_action_id = fixture["transition"]["selectedActionId"]
        .as_str()
        .expect("fixture selected action identity");
    let selected = issued
        .iter()
        .find(|action| action.action_id.as_str() == selected_action_id)
        .expect("Rust issued selected random-discard action");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: selected_action_id.to_owned(),
            seat: selected.seat,
            state_version: selected.state_version,
        })
        .expect("Rust selected random-discard transition")
    else {
        panic!("Rust issued selected random-discard action must be accepted");
    };
    assert_eq!(
        serde_json::to_value(receipt).expect("serialized Rust random-discard receipt"),
        fixture["transition"]["receipt"]
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one exact cross-engine fixture covers legal enumeration and its selected transition"
)]
fn sacrifice_summon_descriptors_order_and_receipt_should_match_typescript() {
    let fixture: Value =
        serde_json::from_str(SACRIFICE_SUMMON_FIXTURE).expect("valid sacrifice summon fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");

    let zero = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let base = json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "synthetic-sacrifice-summon-action-v1",
            }))
            .expect("synthetic fixture authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-sacrifice-summon-action-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-site": {
                "cardType": "site",
                "elements": ["earth"],
                "genesisGainMana": 5,
            },
            "south-avatar": avatar,
            "south-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": zero,
            },
            "south-site": { "cardType": "site", "elements": ["water"] },
            "synthetic-fodder-minion": {
                "attack": 1,
                "cardType": "minion",
                "defense": 2,
                "manaCost": 0,
                "thresholds": zero,
            },
            "synthetic-tithe-beast": {
                "attack": 8,
                "cardType": "minion",
                "defense": 4,
                "manaCost": 6,
                "sacrificeMinionAtSummoningLocationForManaDiscount": 2,
                "thresholds": zero,
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": [
                    "synthetic-tithe-beast",
                    "synthetic-fodder-minion",
                    "synthetic-fodder-minion",
                    "synthetic-fodder-minion",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": vec!["south-minion"; 3],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
    });
    let expected_manifest_id = fixture["manifestId"]
        .as_str()
        .expect("fixture manifest identity");
    let manifest = (1..=4_096)
        .find_map(|seed| {
            let mut candidate = base.clone();
            candidate["seed"] = json!(seed);
            candidate["manifestId"] =
                json!(identity_hash(&candidate).expect("synthetic manifest identity"));
            (candidate["manifestId"] == expected_manifest_id).then_some(candidate)
        })
        .expect("Rust reconstruction of TypeScript fixture manifest");
    assert_eq!(manifest["manifestId"], fixture["manifestId"]);

    let manifest = canonical_json(&manifest).expect("canonical synthetic fixture manifest");
    let mut session = Session::new(&manifest).expect("valid synthetic fixture manifest");
    let mut accept_where = |predicate: &dyn Fn(&Value) -> bool| {
        let action = session
            .legal_actions()
            .expect("fixture legal actions")
            .into_iter()
            .find(|action| predicate(&action.descriptor))
            .expect("fixture setup action");
        let StepResult::Accepted(_) = session
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .expect("fixture setup step")
        else {
            panic!("engine-issued fixture action must be accepted");
        };
    };
    let keep = |descriptor: &Value| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    };
    let end_turn = |descriptor: &Value| descriptor["kind"] == "end-turn";
    let summon_fodder = |descriptor: &Value| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "synthetic-fodder-minion"
            && descriptor["cell"] == "C4"
    };
    accept_where(&keep);
    accept_where(&keep);
    accept_where(&|descriptor| descriptor["kind"] == "play-site" && descriptor["cell"] == "C4");
    accept_where(&summon_fodder);
    accept_where(&summon_fodder);
    accept_where(&end_turn);
    accept_where(&|descriptor| descriptor["kind"] == "draw" && descriptor["zone"] == "atlas");
    accept_where(&|descriptor| descriptor["kind"] == "play-site" && descriptor["cell"] == "C1");
    accept_where(&end_turn);
    accept_where(&|descriptor| descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook");
    accept_where(&|descriptor| descriptor["kind"] == "play-site" && descriptor["cell"] == "C3");
    accept_where(&summon_fodder);

    let casting_instance_id = fixture["actions"][0]["descriptor"]["cardInstanceId"]
        .as_str()
        .expect("fixture casting instance identity");
    let issued = session
        .legal_actions()
        .expect("Rust sacrifice-summon legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["cardInstanceId"] == casting_instance_id
                && action.descriptor["cell"] == "C4"
        })
        .collect::<Vec<_>>();
    assert_eq!(issued.len(), 8);
    assert_eq!(
        issued
            .iter()
            .map(|action| json!({
                "actionId": action.action_id,
                "descriptor": action.descriptor,
                "label": action.label,
            }))
            .collect::<Vec<_>>(),
        fixture["actions"]
            .as_array()
            .expect("fixture sacrifice-summon actions")
            .clone()
    );

    let state = session.replay_value().expect("Rust fixture replay state")["state"].clone();
    let mut eligible_candidates = state["realm"]["units"]
        .as_array()
        .expect("fixture units")
        .iter()
        .filter(|unit| {
            unit["controller"] == "north" && unit["location"] == "C4" && unit["region"] == "surface"
        })
        .map(|unit| unit["instanceId"].clone())
        .collect::<Vec<_>>();
    eligible_candidates.sort_unstable_by(|left, right| {
        left.as_str()
            .expect("candidate identity")
            .cmp(right.as_str().expect("candidate identity"))
    });
    assert_eq!(
        eligible_candidates,
        fixture["eligibleSacrificeCandidateIds"]
            .as_array()
            .expect("fixture sacrifice candidates")
            .clone()
    );

    let selected_action_id = fixture["transition"]["selectedActionId"]
        .as_str()
        .expect("fixture selected action identity");
    let selected = issued
        .iter()
        .find(|action| action.action_id.as_str() == selected_action_id)
        .expect("Rust issued selected sacrifice-summon action");
    let StepResult::Accepted(receipt) = session
        .step(ActionRequest {
            action_id: selected_action_id.to_owned(),
            seat: selected.seat,
            state_version: selected.state_version,
        })
        .expect("Rust selected sacrifice-summon transition")
    else {
        panic!("Rust issued selected sacrifice-summon action must be accepted");
    };
    assert_eq!(
        serde_json::to_value(receipt).expect("serialized Rust sacrifice-summon receipt"),
        fixture["transition"]["receipt"]
    );

    let mut deathrite_base = base;
    deathrite_base["authority"]["contentHash"] = json!(
        identity_hash(&json!({
            "fixture": "synthetic-sacrifice-summon-deathrite-v1",
        }))
        .expect("synthetic Deathrite authority identity")
    );
    deathrite_base["authority"]["revisionId"] = json!("synthetic-sacrifice-summon-deathrite-v1");
    deathrite_base["cards"]["synthetic-fodder-minion"]["deathriteDrawSite"] = json!(true);
    let expected_deathrite_manifest_id = fixture["deathrite"]["manifestId"]
        .as_str()
        .expect("fixture Deathrite manifest identity");
    let deathrite_manifest = (1..=4_096)
        .find_map(|seed| {
            let mut candidate = deathrite_base.clone();
            candidate["seed"] = json!(seed);
            candidate["manifestId"] =
                json!(identity_hash(&candidate).expect("synthetic Deathrite manifest identity"));
            (candidate["manifestId"] == expected_deathrite_manifest_id).then_some(candidate)
        })
        .expect("Rust reconstruction of TypeScript Deathrite manifest");
    assert_eq!(
        deathrite_manifest["manifestId"],
        fixture["deathrite"]["manifestId"]
    );

    let deathrite_manifest =
        canonical_json(&deathrite_manifest).expect("canonical Deathrite fixture manifest");
    let mut deathrite_session =
        Session::new(&deathrite_manifest).expect("valid Deathrite fixture manifest");
    let mut accept_deathrite_setup = |predicate: &dyn Fn(&Value) -> bool| {
        let action = deathrite_session
            .legal_actions()
            .expect("Deathrite fixture legal actions")
            .into_iter()
            .find(|action| predicate(&action.descriptor))
            .expect("Deathrite fixture setup action");
        let StepResult::Accepted(_) = deathrite_session
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .expect("Deathrite fixture setup step")
        else {
            panic!("engine-issued Deathrite setup action must be accepted");
        };
    };
    let keep = |descriptor: &Value| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    };
    let end_turn = |descriptor: &Value| descriptor["kind"] == "end-turn";
    let summon_fodder = |descriptor: &Value| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "synthetic-fodder-minion"
            && descriptor["cell"] == "C4"
    };
    accept_deathrite_setup(&keep);
    accept_deathrite_setup(&keep);
    accept_deathrite_setup(&|descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_deathrite_setup(&summon_fodder);
    accept_deathrite_setup(&summon_fodder);
    accept_deathrite_setup(&end_turn);
    accept_deathrite_setup(&|descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_deathrite_setup(&|descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    accept_deathrite_setup(&end_turn);
    accept_deathrite_setup(&|descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
    });
    accept_deathrite_setup(&|descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C3"
    });
    accept_deathrite_setup(&summon_fodder);

    let expected_payment = &fixture["deathrite"]["paymentAction"];
    let payment = deathrite_session
        .legal_actions()
        .expect("Deathrite payment actions")
        .into_iter()
        .find(|action| action.action_id.as_str() == expected_payment["actionId"])
        .expect("Rust issued fixture Deathrite payment");
    assert_eq!(
        json!({
            "actionId": payment.action_id,
            "descriptor": payment.descriptor,
            "label": payment.label,
            "seat": payment.seat,
            "stateVersion": payment.state_version,
        }),
        *expected_payment
    );
    let StepResult::Accepted(interrupted) = deathrite_session
        .step(ActionRequest {
            action_id: payment.action_id.to_string(),
            seat: payment.seat,
            state_version: payment.state_version,
        })
        .expect("Rust Deathrite payment transition")
    else {
        panic!("Rust issued Deathrite payment must be accepted");
    };
    assert_eq!(
        serde_json::to_value(interrupted).expect("serialized Rust interrupted receipt"),
        fixture["deathrite"]["pending"]["receipt"]
    );
    let pending_state = deathrite_session
        .replay_value()
        .expect("Rust pending replay state")["state"]
        .clone();
    assert_eq!(
        pending_state["phase"],
        fixture["deathrite"]["pending"]["phase"]
    );
    assert_eq!(
        pending_state["decisionSeat"],
        fixture["deathrite"]["pending"]["decisionSeat"]
    );
    assert_eq!(
        pending_state["stateVersion"],
        fixture["deathrite"]["pending"]["stateVersion"]
    );
    assert!(!pending_state["pendingDeathrites"].is_null());
    assert_eq!(
        identity_hash(&pending_state).expect("Rust pending state identity"),
        IdentityHash::parse(
            fixture["deathrite"]["pending"]["stateHash"]
                .as_str()
                .expect("fixture pending state identity")
        )
        .expect("valid fixture pending state identity")
    );
    let pending_checkpoint =
        create_game_checkpoint(&deathrite_session).expect("Rust pending checkpoint");
    let serialized_pending =
        serialize_game_checkpoint(&pending_checkpoint).expect("serialized Rust pending checkpoint");
    assert_eq!(
        identity_hash(&Value::String(serialized_pending.clone()))
            .expect("serialized Rust pending checkpoint identity"),
        IdentityHash::parse(
            fixture["deathrite"]["pending"]["serializedCheckpointHash"]
                .as_str()
                .expect("fixture serialized pending checkpoint identity")
        )
        .expect("valid fixture serialized pending checkpoint identity")
    );
    assert_eq!(
        pending_checkpoint.checkpoint_id.as_str(),
        fixture["deathrite"]["pending"]["checkpointId"]
    );
    assert_eq!(
        pending_checkpoint.expected_session_hash.as_str(),
        fixture["deathrite"]["pending"]["expectedSessionHash"]
    );

    let parsed =
        parse_game_checkpoint(&serialized_pending).expect("parsed Rust pending checkpoint");
    let mut restored = resume_game_checkpoint(&parsed).expect("restored Rust pending checkpoint");
    assert_eq!(
        restored
            .replay_value()
            .expect("restored Rust pending state"),
        deathrite_session
            .replay_value()
            .expect("source Rust pending state")
    );
    let order_actions = restored
        .legal_actions()
        .expect("Rust Deathrite order actions");
    assert_eq!(
        order_actions
            .iter()
            .map(|action| json!({
                "actionId": action.action_id,
                "descriptor": action.descriptor,
                "label": action.label,
                "seat": action.seat,
                "stateVersion": action.state_version,
            }))
            .collect::<Vec<_>>(),
        fixture["deathrite"]["pending"]["orderActions"]
            .as_array()
            .expect("fixture Deathrite order actions")
            .clone()
    );
    let selected_order_id = fixture["deathrite"]["resolved"]["selectedOrderActionId"]
        .as_str()
        .expect("fixture selected Deathrite order identity");
    let selected_order = order_actions
        .iter()
        .find(|action| action.action_id.as_str() == selected_order_id)
        .expect("Rust issued selected Deathrite order");
    let StepResult::Accepted(resolved) = restored
        .step(ActionRequest {
            action_id: selected_order.action_id.to_string(),
            seat: selected_order.seat,
            state_version: selected_order.state_version,
        })
        .expect("Rust selected Deathrite order transition")
    else {
        panic!("Rust issued Deathrite order must be accepted");
    };
    assert_eq!(
        serde_json::to_value(resolved).expect("serialized Rust resolved receipt"),
        fixture["deathrite"]["resolved"]["receipt"]
    );
    let resolved_state =
        restored.replay_value().expect("Rust resolved replay state")["state"].clone();
    assert_eq!(
        resolved_state["phase"],
        fixture["deathrite"]["resolved"]["phase"]
    );
    assert_eq!(
        resolved_state["decisionSeat"],
        fixture["deathrite"]["resolved"]["decisionSeat"]
    );
    assert_eq!(
        resolved_state["stateVersion"],
        fixture["deathrite"]["resolved"]["stateVersion"]
    );
    assert!(resolved_state["pendingDeathrites"].is_null());
    assert_eq!(
        identity_hash(&resolved_state).expect("Rust resolved state identity"),
        IdentityHash::parse(
            fixture["deathrite"]["resolved"]["stateHash"]
                .as_str()
                .expect("fixture resolved state identity")
        )
        .expect("valid fixture resolved state identity")
    );
    assert!(
        restored
            .verify_replay()
            .expect("verified Rust Deathrite replay")
    );
    let resolved_checkpoint = create_game_checkpoint(&restored).expect("Rust resolved checkpoint");
    let serialized_resolved = serialize_game_checkpoint(&resolved_checkpoint)
        .expect("serialized Rust resolved checkpoint");
    assert_eq!(
        identity_hash(&Value::String(serialized_resolved))
            .expect("serialized Rust resolved checkpoint identity"),
        IdentityHash::parse(
            fixture["deathrite"]["resolved"]["serializedCheckpointHash"]
                .as_str()
                .expect("fixture serialized resolved checkpoint identity")
        )
        .expect("valid fixture serialized resolved checkpoint identity")
    );
    assert_eq!(
        resolved_checkpoint.checkpoint_id.as_str(),
        fixture["deathrite"]["resolved"]["checkpointId"]
    );
    assert_eq!(
        resolved_checkpoint.expected_session_hash.as_str(),
        fixture["deathrite"]["resolved"]["expectedSessionHash"]
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one exact cross-engine fixture covers Duel legality, combat, Ward, and replay"
)]
fn duel_descriptors_order_and_transitions_should_match_typescript() {
    fn accept_underground_setup(session: &mut Session, predicate: &dyn Fn(&Value) -> bool) {
        let action = session
            .legal_actions()
            .expect("underground Duel fixture legal actions")
            .into_iter()
            .find(|action| predicate(&action.descriptor))
            .expect("underground Duel fixture setup action");
        let StepResult::Accepted(_) = session
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .expect("underground Duel fixture setup step")
        else {
            panic!("engine-issued underground Duel setup action must be accepted");
        };
    }

    let fixture: Value = serde_json::from_str(DUEL_FIXTURE).expect("valid Duel fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");
    let zero = json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
    let avatar = json!({
        "attack": 1,
        "cardType": "avatar",
        "defense": 1,
        "drawSpell": false,
        "life": 20,
    });
    let mut manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({ "fixture": "synthetic-duel-action-v1" }))
                .expect("synthetic Duel authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-duel-action-v1",
        },
        "cards": {
            "north-avatar": avatar,
            "north-filler": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": zero,
            },
            "north-site": { "cardType": "site", "elements": ["earth"] },
            "south-avatar": avatar,
            "south-filler": {
                "attack": 1,
                "cardType": "minion",
                "defense": 1,
                "manaCost": 0,
                "thresholds": zero,
            },
            "south-site": { "cardType": "site", "elements": ["water"] },
            "synthetic-duel-ally": {
                "attack": 3,
                "cardType": "minion",
                "defense": 4,
                "manaCost": 0,
                "thresholds": zero,
            },
            "synthetic-duel-target": {
                "attack": 2,
                "cardType": "minion",
                "defense": 3,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": zero,
            },
            "synthetic-forced-duel": {
                "cardType": "magic",
                "fightAllyWithAdjacentEnemy": true,
                "manaCost": 1,
                "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
            },
            "synthetic-warded-target": {
                "attack": 2,
                "cardType": "minion",
                "defense": 3,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": zero,
                "ward": true,
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": [
                    "synthetic-forced-duel",
                    "synthetic-duel-ally",
                    "north-filler",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": [
                    "synthetic-duel-target",
                    "synthetic-warded-target",
                    "south-filler",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 719,
    });
    manifest["manifestId"] =
        json!(identity_hash(&manifest).expect("synthetic Duel manifest identity"));
    assert_eq!(manifest["manifestId"], fixture["manifestId"]);
    let manifest = canonical_json(&manifest).expect("canonical Duel fixture manifest");
    let mut session = Session::new(&manifest).expect("valid Duel fixture manifest");
    let mut accept_where = |predicate: &dyn Fn(&Value) -> bool| {
        let action = session
            .legal_actions()
            .expect("Duel fixture legal actions")
            .into_iter()
            .find(|action| predicate(&action.descriptor))
            .expect("Duel fixture setup action");
        let StepResult::Accepted(_) = session
            .step(ActionRequest {
                action_id: action.action_id.to_string(),
                seat: action.seat,
                state_version: action.state_version,
            })
            .expect("Duel fixture setup step")
        else {
            panic!("engine-issued Duel setup action must be accepted");
        };
    };
    let keep = |descriptor: &Value| {
        descriptor["kind"] == "mulligan"
            && descriptor["atlasOrder"] == json!([])
            && descriptor["spellbookOrder"] == json!([])
    };
    let end_turn = |descriptor: &Value| descriptor["kind"] == "end-turn";
    accept_where(&keep);
    accept_where(&keep);
    accept_where(&|descriptor| descriptor["kind"] == "play-site" && descriptor["cell"] == "C4");
    accept_where(&|descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "synthetic-duel-ally"
            && descriptor["cell"] == "C4"
    });
    accept_where(&end_turn);
    accept_where(&|descriptor| descriptor["kind"] == "draw" && descriptor["zone"] == "atlas");
    accept_where(&|descriptor| descriptor["kind"] == "play-site" && descriptor["cell"] == "C1");
    for card_id in ["synthetic-duel-target", "synthetic-warded-target"] {
        accept_where(&|descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == card_id
                && descriptor["cell"] == "C4"
        });
    }
    accept_where(&end_turn);
    accept_where(&|descriptor| descriptor["kind"] == "draw" && descriptor["zone"] == "atlas");

    let issued = session
        .legal_actions()
        .expect("Rust Duel legal actions")
        .into_iter()
        .filter(|action| {
            action.descriptor["kind"] == "cast-magic"
                && action.descriptor["cardId"] == "synthetic-forced-duel"
                && action.descriptor["ally"]["instanceId"] == fixture["allyInstanceId"]
        })
        .collect::<Vec<_>>();
    assert_eq!(
        issued
            .iter()
            .map(|action| json!({
                "actionId": action.action_id,
                "descriptor": action.descriptor,
                "label": action.label,
            }))
            .collect::<Vec<_>>(),
        fixture["actions"]
            .as_array()
            .expect("fixture Duel actions")
            .clone()
    );
    assert_eq!(
        issued
            .iter()
            .map(|action| action.action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("fixture canonical Duel action IDs")
            .iter()
            .map(|value| value.as_str().expect("Duel action ID").to_owned())
            .collect::<Vec<_>>()
    );

    let start_state = session.replay_value().expect("Rust Duel start state")["state"].clone();
    assert_eq!(
        identity_hash(&start_state).expect("Rust Duel start state identity"),
        IdentityHash::parse(
            fixture["startCheckpoint"]["stateHash"]
                .as_str()
                .expect("fixture Duel start state identity")
        )
        .expect("valid fixture Duel start state identity")
    );
    let start_checkpoint = create_game_checkpoint(&session).expect("Rust Duel start checkpoint");
    let serialized_start = serialize_game_checkpoint(&start_checkpoint)
        .expect("serialized Rust Duel start checkpoint");
    assert_eq!(
        start_checkpoint.checkpoint_id.as_str(),
        fixture["startCheckpoint"]["checkpointId"]
    );
    assert_eq!(
        start_checkpoint.expected_session_hash.as_str(),
        fixture["startCheckpoint"]["expectedSessionHash"]
    );
    assert_eq!(
        identity_hash(&Value::String(serialized_start.clone()))
            .expect("serialized Rust Duel start checkpoint identity"),
        IdentityHash::parse(
            fixture["startCheckpoint"]["serializedCheckpointHash"]
                .as_str()
                .expect("fixture serialized Duel start checkpoint identity")
        )
        .expect("valid fixture serialized Duel start checkpoint identity")
    );

    for (transition_name, target_field) in [
        ("normalTransition", "normalTargetInstanceId"),
        ("wardedTransition", "wardedTargetInstanceId"),
    ] {
        let parsed =
            parse_game_checkpoint(&serialized_start).expect("parsed Duel start checkpoint");
        let mut branch = resume_game_checkpoint(&parsed).expect("restored Duel start checkpoint");
        let transition = &fixture[transition_name];
        let selected_action_id = transition["selectedActionId"]
            .as_str()
            .expect("fixture selected Duel action identity");
        let selected = branch
            .legal_actions()
            .expect("restored Duel actions")
            .into_iter()
            .find(|action| action.action_id.as_str() == selected_action_id)
            .expect("Rust issued selected Duel action");
        let StepResult::Accepted(receipt) = branch
            .step(ActionRequest {
                action_id: selected.action_id.to_string(),
                seat: selected.seat,
                state_version: selected.state_version,
            })
            .expect("Rust selected Duel transition")
        else {
            panic!("Rust issued Duel action must be accepted");
        };
        assert_eq!(
            serde_json::to_value(receipt).expect("serialized Rust Duel receipt"),
            transition["receipt"]
        );
        let state = branch.replay_value().expect("Rust Duel branch state")["state"].clone();
        assert_eq!(
            identity_hash(&state).expect("Rust Duel branch state identity"),
            IdentityHash::parse(
                transition["stateHash"]
                    .as_str()
                    .expect("fixture Duel branch state identity")
            )
            .expect("valid fixture Duel branch state identity")
        );
        let ally = state["realm"]["units"]
            .as_array()
            .expect("Duel branch units")
            .iter()
            .find(|unit| unit["instanceId"] == fixture["allyInstanceId"])
            .expect("Duel ally survives");
        let target = state["realm"]["units"]
            .as_array()
            .expect("Duel branch units")
            .iter()
            .find(|unit| unit["instanceId"] == fixture[target_field]);
        assert_eq!(
            json!({
                "allyDamage": ally["damage"],
                "mana": state["players"]["north"]["mana"],
                "targetPresent": target.is_some(),
                "targetWarded": target.map_or(Value::Null, |unit| unit["warded"].clone()),
            }),
            transition["summary"]
        );
        assert!(branch.verify_replay().expect("verified Rust Duel replay"));
        let checkpoint = create_game_checkpoint(&branch).expect("Rust Duel result checkpoint");
        let serialized =
            serialize_game_checkpoint(&checkpoint).expect("serialized Rust Duel result checkpoint");
        assert_eq!(
            checkpoint.checkpoint_id.as_str(),
            transition["checkpointId"]
        );
        assert_eq!(
            checkpoint.expected_session_hash.as_str(),
            transition["expectedSessionHash"]
        );
        assert_eq!(
            identity_hash(&Value::String(serialized))
                .expect("serialized Rust Duel result checkpoint identity"),
            IdentityHash::parse(
                transition["serializedCheckpointHash"]
                    .as_str()
                    .expect("fixture serialized Duel result checkpoint identity")
            )
            .expect("valid fixture serialized Duel result checkpoint identity")
        );
    }

    let mut underground_manifest = json!({
        "authority": {
            "contentHash": identity_hash(&json!({
                "fixture": "synthetic-underground-duel-action-v1"
            }))
            .expect("synthetic underground Duel authority identity"),
            "mode": "synthetic",
            "revisionId": "synthetic-underground-duel-action-v1",
        },
        "cards": {
            "north-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "north-site": { "cardType": "site", "elements": ["earth"] },
            "south-avatar": {
                "attack": 1,
                "cardType": "avatar",
                "defense": 1,
                "drawSpell": false,
                "life": 20,
            },
            "south-site": { "cardType": "site", "elements": ["water"] },
            "synthetic-forced-duel": {
                "cardType": "magic",
                "fightAllyWithAdjacentEnemy": true,
                "manaCost": 1,
                "thresholds": { "air": 0, "earth": 1, "fire": 0, "water": 0 },
            },
            "synthetic-bury-duelists": {
                "burrowTargetMinionOrArtifact": true,
                "cardType": "magic",
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "synthetic-underground-deathrite": {
                "attack": 2,
                "burrowing": true,
                "cardType": "minion",
                "deathriteDamageEachUnitHere": 1,
                "defense": 3,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "synthetic-underground-duelist": {
                "attack": 3,
                "burrowing": true,
                "cardType": "minion",
                "defense": 6,
                "manaCost": 0,
                "strikesFirstWhileAttacking": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
            "synthetic-underground-fragile": {
                "attack": 0,
                "burrowing": true,
                "cardType": "minion",
                "deathriteDrawSite": true,
                "defense": 1,
                "manaCost": 0,
                "summonToAnySite": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            },
        },
        "decks": {
            "north": {
                "atlas": vec!["north-site"; 9],
                "avatar": "north-avatar",
                "spellbook": [
                    "synthetic-forced-duel",
                    "synthetic-underground-duelist",
                    "synthetic-bury-duelists",
                    "synthetic-bury-duelists",
                    "synthetic-bury-duelists",
                    "synthetic-bury-duelists",
                ],
            },
            "south": {
                "atlas": vec!["south-site"; 9],
                "avatar": "south-avatar",
                "spellbook": [
                    "synthetic-underground-deathrite",
                    "synthetic-underground-fragile",
                    "synthetic-underground-fragile",
                ],
            },
        },
        "engineVersion": "sorcery-core-v1",
        "firstSeat": "north",
        "schemaVersion": 1,
        "seed": 1,
    });
    underground_manifest["manifestId"] = json!(
        identity_hash(&underground_manifest).expect("synthetic underground Duel manifest identity")
    );
    let underground = &fixture["undergroundDeathrite"];
    assert_eq!(
        underground_manifest["manifestId"],
        underground["manifestId"]
    );
    let underground_manifest =
        canonical_json(&underground_manifest).expect("canonical underground Duel fixture manifest");
    let mut underground_session =
        Session::new(&underground_manifest).expect("valid underground Duel fixture manifest");
    accept_underground_setup(&mut underground_session, &keep);
    accept_underground_setup(&mut underground_session, &keep);
    accept_underground_setup(&mut underground_session, &|descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C4"
    });
    accept_underground_setup(&mut underground_session, &|descriptor| {
        descriptor["kind"] == "summon-minion"
            && descriptor["cardId"] == "synthetic-underground-duelist"
            && descriptor["cell"] == "C4"
    });
    accept_underground_setup(&mut underground_session, &end_turn);
    accept_underground_setup(&mut underground_session, &|descriptor| {
        descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
    });
    accept_underground_setup(&mut underground_session, &|descriptor| {
        descriptor["kind"] == "play-site" && descriptor["cell"] == "C1"
    });
    for card_id in [
        "synthetic-underground-deathrite",
        "synthetic-underground-fragile",
        "synthetic-underground-fragile",
    ] {
        accept_underground_setup(&mut underground_session, &|descriptor| {
            descriptor["kind"] == "summon-minion"
                && descriptor["cardId"] == card_id
                && descriptor["cell"] == "C4"
        });
    }
    accept_underground_setup(&mut underground_session, &end_turn);
    for _ in 0..3 {
        accept_underground_setup(&mut underground_session, &|descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "spellbook"
        });
        loop {
            let current = underground_session
                .replay_value()
                .expect("underground Duel setup state")["state"]
                .clone();
            let has_bury = current["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North underground Duel hand")
                .iter()
                .any(|card| card["cardId"] == "synthetic-bury-duelists");
            let target_id = current["realm"]["units"]
                .as_array()
                .expect("underground Duel units")
                .iter()
                .find(|unit| unit["region"] == "surface")
                .and_then(|unit| unit["instanceId"].as_str())
                .map(str::to_owned);
            let (true, Some(target_id)) = (has_bury, target_id) else {
                break;
            };
            accept_underground_setup(&mut underground_session, &|descriptor| {
                descriptor["kind"] == "cast-magic"
                    && descriptor["cardId"] == "synthetic-bury-duelists"
                    && descriptor["target"]["instanceId"] == target_id
            });
        }
        let current = underground_session
            .replay_value()
            .expect("underground Duel readiness state")["state"]
            .clone();
        let ready = current["realm"]["units"]
            .as_array()
            .expect("underground Duel readiness units")
            .iter()
            .all(|unit| unit["region"] == "underground")
            && current["players"]["north"]["hand"]["spellbook"]
                .as_array()
                .expect("North underground Duel readiness hand")
                .iter()
                .any(|card| card["cardId"] == "synthetic-forced-duel");
        if ready {
            break;
        }
        accept_underground_setup(&mut underground_session, &end_turn);
        accept_underground_setup(&mut underground_session, &|descriptor| {
            descriptor["kind"] == "draw" && descriptor["zone"] == "atlas"
        });
        accept_underground_setup(&mut underground_session, &end_turn);
    }

    let expected_duel = &underground["duelAction"];
    let duel = underground_session
        .legal_actions()
        .expect("underground Duel actions")
        .into_iter()
        .find(|action| action.action_id.as_str() == expected_duel["actionId"])
        .expect("Rust issued underground Duel action");
    assert_eq!(
        json!({
            "actionId": duel.action_id,
            "descriptor": duel.descriptor,
            "label": duel.label,
            "seat": duel.seat,
            "stateVersion": duel.state_version,
        }),
        *expected_duel
    );
    let StepResult::Accepted(interrupted) = underground_session
        .step(ActionRequest {
            action_id: duel.action_id.to_string(),
            seat: duel.seat,
            state_version: duel.state_version,
        })
        .expect("Rust underground Duel transition")
    else {
        panic!("Rust issued underground Duel action must be accepted");
    };
    assert_eq!(
        serde_json::to_value(interrupted).expect("serialized Rust underground Duel receipt"),
        underground["pending"]["receipt"]
    );
    let pending_state = underground_session
        .replay_value()
        .expect("Rust underground Duel pending state")["state"]
        .clone();
    assert_eq!(
        pending_state["pendingDeathrites"]["continuation"],
        underground["pending"]["continuation"]
    );
    assert_eq!(
        pending_state["pendingDeathrites"]["continuation"]["pending"]["region"],
        "underground"
    );
    assert_eq!(pending_state["phase"], underground["pending"]["phase"]);
    assert_eq!(
        pending_state["decisionSeat"],
        underground["pending"]["decisionSeat"]
    );
    assert_eq!(
        pending_state["stateVersion"],
        underground["pending"]["stateVersion"]
    );
    assert_eq!(
        identity_hash(&pending_state).expect("Rust underground Duel pending state identity"),
        IdentityHash::parse(
            underground["pending"]["stateHash"]
                .as_str()
                .expect("fixture underground Duel pending state identity")
        )
        .expect("valid fixture underground Duel pending state identity")
    );
    let pending_checkpoint = create_game_checkpoint(&underground_session)
        .expect("Rust underground Duel pending checkpoint");
    let serialized_pending = serialize_game_checkpoint(&pending_checkpoint)
        .expect("serialized Rust underground Duel pending checkpoint");
    assert_eq!(
        pending_checkpoint.checkpoint_id.as_str(),
        underground["pending"]["checkpointId"]
    );
    assert_eq!(
        pending_checkpoint.expected_session_hash.as_str(),
        underground["pending"]["expectedSessionHash"]
    );
    assert_eq!(
        identity_hash(&Value::String(serialized_pending.clone()))
            .expect("serialized Rust underground Duel pending checkpoint identity"),
        IdentityHash::parse(
            underground["pending"]["serializedCheckpointHash"]
                .as_str()
                .expect("fixture serialized underground Duel pending checkpoint identity")
        )
        .expect("valid fixture serialized underground Duel pending checkpoint identity")
    );

    let parsed = parse_game_checkpoint(&serialized_pending)
        .expect("parsed Rust underground Duel pending checkpoint");
    let mut restored =
        resume_game_checkpoint(&parsed).expect("restored Rust underground Duel pending checkpoint");
    assert_eq!(
        restored
            .replay_value()
            .expect("restored Rust underground Duel pending state"),
        underground_session
            .replay_value()
            .expect("source Rust underground Duel pending state")
    );
    let order_actions = restored
        .legal_actions()
        .expect("Rust underground Deathrite order actions");
    assert_eq!(
        order_actions
            .iter()
            .map(|action| json!({
                "actionId": action.action_id,
                "descriptor": action.descriptor,
                "label": action.label,
                "seat": action.seat,
                "stateVersion": action.state_version,
            }))
            .collect::<Vec<_>>(),
        underground["pending"]["orderActions"]
            .as_array()
            .expect("fixture underground Deathrite order actions")
            .clone()
    );
    let selected_order_id = underground["resolved"]["selectedOrderActionId"]
        .as_str()
        .expect("fixture selected underground Deathrite order identity");
    let selected_order = order_actions
        .iter()
        .find(|action| action.action_id.as_str() == selected_order_id)
        .expect("Rust issued selected underground Deathrite order");
    let StepResult::Accepted(resolved) = restored
        .step(ActionRequest {
            action_id: selected_order.action_id.to_string(),
            seat: selected_order.seat,
            state_version: selected_order.state_version,
        })
        .expect("Rust selected underground Deathrite order transition")
    else {
        panic!("Rust issued underground Deathrite order must be accepted");
    };
    assert_eq!(
        serde_json::to_value(resolved)
            .expect("serialized Rust underground Deathrite resolved receipt"),
        underground["resolved"]["receipt"]
    );
    let resolved_state = restored
        .replay_value()
        .expect("Rust underground Duel resolved state")["state"]
        .clone();
    assert_eq!(resolved_state["phase"], underground["resolved"]["phase"]);
    assert_eq!(
        resolved_state["decisionSeat"],
        underground["resolved"]["decisionSeat"]
    );
    assert_eq!(
        resolved_state["stateVersion"],
        underground["resolved"]["stateVersion"]
    );
    assert!(resolved_state["pendingCombat"].is_null());
    assert!(resolved_state["pendingDeathrites"].is_null());
    assert_eq!(
        identity_hash(&resolved_state).expect("Rust underground Duel resolved state identity"),
        IdentityHash::parse(
            underground["resolved"]["stateHash"]
                .as_str()
                .expect("fixture underground Duel resolved state identity")
        )
        .expect("valid fixture underground Duel resolved state identity")
    );
    assert!(
        restored
            .verify_replay()
            .expect("verified Rust underground Duel replay")
    );
    let resolved_checkpoint =
        create_game_checkpoint(&restored).expect("Rust underground Duel resolved checkpoint");
    let serialized_resolved = serialize_game_checkpoint(&resolved_checkpoint)
        .expect("serialized Rust underground Duel resolved checkpoint");
    assert_eq!(
        resolved_checkpoint.checkpoint_id.as_str(),
        underground["resolved"]["checkpointId"]
    );
    assert_eq!(
        resolved_checkpoint.expected_session_hash.as_str(),
        underground["resolved"]["expectedSessionHash"]
    );
    assert_eq!(
        identity_hash(&Value::String(serialized_resolved))
            .expect("serialized Rust underground Duel resolved checkpoint identity"),
        IdentityHash::parse(
            underground["resolved"]["serializedCheckpointHash"]
                .as_str()
                .expect("fixture serialized underground Duel resolved checkpoint identity")
        )
        .expect("valid fixture serialized underground Duel resolved checkpoint identity")
    );
}

#[test]
fn shoot_projectile_descriptors_labels_order_and_ids_should_match_typescript() {
    let fixture: Value =
        serde_json::from_str(SHOOT_PROJECTILE_FIXTURE).expect("valid ordinary Ranged fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");
    assert_eq!(
        fixture["actions"]
            .as_array()
            .expect("fixture actions")
            .len(),
        4
    );
    assert_eq!(
        fixture["actions"][2]["descriptor"]["path"],
        json!([
            { "cell": "C4", "region": "surface" },
            { "cell": "C3", "region": "surface" },
        ])
    );
    let contract = fixture["contract"].as_str().expect("action contract");
    let seat: Seat = serde_json::from_value(fixture["seat"].clone()).expect("fixture seat");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let mut ordered = Vec::new();
    let mut labels = Vec::new();

    for action in fixture["actions"].as_array().expect("fixture actions") {
        let descriptor: ActionDescriptor = serde_json::from_value(action["descriptor"].clone())
            .expect("typed ordinary Ranged descriptor");
        assert!(matches!(
            descriptor,
            ActionDescriptor::ShootProjectile { .. }
        ));
        assert_eq!(descriptor_kind(&descriptor), "shoot-projectile");
        let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
        assert_eq!(serialized, action["descriptor"]);
        let expected_id =
            IdentityHash::parse(action["actionId"].as_str().expect("TypeScript action ID"))
                .expect("valid TypeScript action ID");
        assert_eq!(
            opaque_action_id(contract, seat, state_version, &serialized)
                .expect("Rust action identity"),
            expected_id
        );
        labels.push(
            descriptor
                .state_independent_label()
                .expect("state-independent Ranged label"),
        );
        ordered.push((
            canonical_json(&serialized).expect("canonical descriptor"),
            expected_id,
        ));
    }

    ordered.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        ordered
            .into_iter()
            .map(|(_, action_id)| action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical TypeScript order")
            .iter()
            .map(|action_id| action_id.as_str().expect("action ID").to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        labels,
        [
            "Shoot east at nothing",
            "Shoot north at nothing",
            "Shoot south at minion sha256:b1bcf07f…",
            "Shoot west at nothing",
        ]
    );

    for invalid in [
        json!({
            "direction": "south",
            "kind": "shoot-projectile",
            "path": [{ "cell": "C4", "region": "surface" }],
            "shooterInstanceId": fixture["shooterInstanceId"],
        }),
        json!({
            "direction": "south",
            "hit": null,
            "kind": "shoot-projectile",
            "path": [],
        }),
        json!({
            "direction": "diagonal",
            "hit": null,
            "kind": "shoot-projectile",
            "path": [{ "cell": "C4", "region": "surface" }],
            "shooterInstanceId": fixture["shooterInstanceId"],
        }),
    ] {
        assert!(serde_json::from_value::<ActionDescriptor>(invalid).is_err());
    }
}

#[test]
fn shoot_damage_projectile_descriptors_labels_order_and_ids_should_match_typescript() {
    let fixture: Value = serde_json::from_str(SHOOT_DAMAGE_PROJECTILE_FIXTURE)
        .expect("valid Shoot Damage Projectile fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");
    let contract = fixture["contract"].as_str().expect("action contract");
    let seat: Seat = serde_json::from_value(fixture["seat"].clone()).expect("fixture seat");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let mut ordered = Vec::new();
    let mut labels = Vec::new();

    for action in fixture["actions"].as_array().expect("fixture actions") {
        let descriptor: ActionDescriptor = serde_json::from_value(action["descriptor"].clone())
            .expect("typed Shoot Damage Projectile descriptor");
        assert!(matches!(
            descriptor,
            ActionDescriptor::ShootDamageProjectile { .. }
        ));
        assert_eq!(descriptor_kind(&descriptor), "shoot-damage-projectile");
        let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
        assert_eq!(serialized, action["descriptor"]);
        let expected_id =
            IdentityHash::parse(action["actionId"].as_str().expect("TypeScript action ID"))
                .expect("valid TypeScript action ID");
        assert_eq!(
            opaque_action_id(contract, seat, state_version, &serialized)
                .expect("Rust action identity"),
            expected_id
        );
        labels.push(
            descriptor
                .state_independent_label()
                .expect("state-independent projectile label"),
        );
        ordered.push((
            canonical_json(&serialized).expect("canonical descriptor"),
            expected_id,
        ));
    }

    ordered.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        ordered
            .into_iter()
            .map(|(_, action_id)| action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical TypeScript order")
            .iter()
            .map(|action_id| action_id.as_str().expect("action ID").to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        labels,
        [
            "Shoot east at nothing",
            "Shoot north at nothing",
            "Shoot south at minion sha256:b58208bd…",
            "Shoot west at nothing",
        ]
    );

    for invalid in [
        json!({
            "direction": "south",
            "kind": "shoot-damage-projectile",
            "path": [{ "cell": "C4", "region": "surface" }],
            "shooterInstanceId": fixture["shooterInstanceId"],
        }),
        json!({
            "direction": "south",
            "hit": null,
            "kind": "shoot-damage-projectile",
            "path": [],
        }),
        json!({
            "direction": "diagonal",
            "hit": null,
            "kind": "shoot-damage-projectile",
            "path": [{ "cell": "C4", "region": "surface" }],
            "shooterInstanceId": fixture["shooterInstanceId"],
        }),
    ] {
        assert!(serde_json::from_value::<ActionDescriptor>(invalid).is_err());
    }
}

#[test]
fn sparkmage_descriptors_order_and_action_ids_should_match_typescript() {
    let fixture: Value =
        serde_json::from_str(SPARKMAGE_FIXTURE).expect("valid Sparkmage action fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");
    assert_eq!(
        fixture["actions"]
            .as_array()
            .expect("fixture actions")
            .len(),
        6
    );
    let contract = fixture["contract"].as_str().expect("action contract");
    let seat: Seat = serde_json::from_value(fixture["seat"].clone()).expect("fixture seat");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let source_instance_id = fixture["avatarInstanceId"]
        .as_str()
        .expect("fixture Avatar identity");
    let mut ordered = Vec::new();
    let mut target_cells = Vec::new();

    for action in fixture["actions"].as_array().expect("fixture actions") {
        let descriptor: ActionDescriptor = serde_json::from_value(action["descriptor"].clone())
            .expect("typed Sparkmage descriptor");
        assert!(matches!(
            descriptor,
            ActionDescriptor::ActivateSparkmage { .. }
        ));
        assert_eq!(descriptor_kind(&descriptor), "activate-sparkmage");
        assert_eq!(descriptor.state_independent_label(), None);
        let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
        assert_eq!(serialized, action["descriptor"]);
        assert_eq!(serialized["sourceInstanceId"], source_instance_id);
        target_cells.push(
            serialized["targetLocation"]["cell"]
                .as_str()
                .expect("target cell")
                .to_owned(),
        );
        let expected_id =
            IdentityHash::parse(action["actionId"].as_str().expect("TypeScript action ID"))
                .expect("valid TypeScript action ID");
        assert_eq!(
            opaque_action_id(contract, seat, state_version, &serialized)
                .expect("Rust action identity"),
            expected_id
        );
        ordered.push((
            canonical_json(&serialized).expect("canonical descriptor"),
            expected_id,
        ));
    }

    assert_eq!(target_cells, ["B3", "B4", "C3", "C4", "D3", "D4"]);
    ordered.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        ordered
            .into_iter()
            .map(|(_, action_id)| action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical TypeScript order")
            .iter()
            .map(|action_id| action_id.as_str().expect("action ID").to_owned())
            .collect::<Vec<_>>()
    );

    for invalid in [
        json!({
            "kind": "activate-sparkmage",
            "sourceInstanceId": source_instance_id,
            "targetLocation": null,
        }),
        json!({
            "kind": "activate-sparkmage",
            "sourceInstanceId": source_instance_id,
            "targetLocation": { "cell": "C4", "region": "surface" },
            "unknown": true,
        }),
        json!({
            "kind": "activate-sparkmage",
            "sourceInstanceId": source_instance_id,
            "targetLocation": { "cell": "C4", "region": "sky" },
        }),
    ] {
        assert!(serde_json::from_value::<ActionDescriptor>(invalid).is_err());
    }
}

#[test]
fn deathrite_order_descriptors_and_action_ids_should_match_typescript() {
    let fixture: Value =
        serde_json::from_str(DEATHRITE_ORDER_FIXTURE).expect("valid Deathrite fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(fixture["source"], "typescript-legality-engine");
    let contract = fixture["contract"].as_str().expect("action contract");
    let seat: Seat = serde_json::from_value(fixture["seat"].clone()).expect("fixture seat");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let descriptors = fixture["actions"]
        .as_array()
        .expect("fixture actions")
        .iter()
        .map(|action| {
            let descriptor: ActionDescriptor =
                serde_json::from_value(action["descriptor"].clone()).expect("valid descriptor");
            assert_eq!(descriptor_kind(&descriptor), "order-deathrites");
            assert_eq!(descriptor.state_independent_label(), None);
            let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
            let expected_id =
                IdentityHash::parse(action["actionId"].as_str().expect("fixture action ID"))
                    .expect("valid fixture action ID");
            assert_eq!(
                opaque_action_id(contract, seat, state_version, &serialized)
                    .expect("opaque action ID"),
                expected_id
            );
            descriptor
        })
        .collect::<Vec<_>>();
    assert!(matches!(
        &descriptors[..],
        [
            ActionDescriptor::OrderDeathrites {
                source_instance_id: first,
            },
            ActionDescriptor::OrderDeathrites {
                source_instance_id: second,
            },
        ] if first < second
    ));
    assert_eq!(
        fixture["actions"]
            .as_array()
            .expect("fixture actions")
            .iter()
            .map(|action| action["actionId"].clone())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical action IDs")
            .clone()
    );
    for invalid in [
        json!({ "kind": "order-deathrites", "sourceInstanceId": null }),
        json!({
            "kind": "order-deathrites",
            "sourceInstanceId": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            "unknown": true,
        }),
    ] {
        assert!(serde_json::from_value::<ActionDescriptor>(invalid).is_err());
    }
}

#[test]
fn combat_response_descriptors_order_ids_and_labels_should_match_typescript() {
    let fixture: Value = serde_json::from_str(COMBAT_RESPONSE_FIXTURE)
        .expect("valid combat response parity fixture");
    let contract = fixture["contract"].as_str().expect("action contract");
    let seat: Seat = serde_json::from_value(fixture["seat"].clone()).expect("fixture seat");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let mut ordered = Vec::new();
    let mut labels = Vec::new();

    for action in fixture["actions"].as_array().expect("fixture actions") {
        let descriptor: ActionDescriptor = serde_json::from_value(action["descriptor"].clone())
            .expect("typed combat response descriptor");
        let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
        assert_eq!(serialized, action["descriptor"]);
        let expected_id =
            IdentityHash::parse(action["actionId"].as_str().expect("TypeScript action ID"))
                .expect("valid TypeScript action ID");
        assert_eq!(
            opaque_action_id(contract, seat, state_version, &serialized)
                .expect("Rust action identity"),
            expected_id
        );
        if matches!(
            descriptor,
            ActionDescriptor::Defend { .. }
                | ActionDescriptor::Intercept { .. }
                | ActionDescriptor::CloseIntercept {}
                | ActionDescriptor::AllocateStrike { .. }
        ) {
            labels.push(
                descriptor
                    .state_independent_label()
                    .expect("state-independent combat label"),
            );
        }
        ordered.push((
            canonical_json(&serialized).expect("canonical descriptor"),
            expected_id,
        ));
    }

    ordered.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        ordered
            .into_iter()
            .map(|(_, action_id)| action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical TypeScript order")
            .iter()
            .map(|action_id| action_id.as_str().expect("action ID").to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        labels,
        [
            "Defend with sha256:22222222… via C2 → D2",
            "Defend with sha256:11111111… via C2 → C3",
            "Assign 2 damage to sha256:11111111…",
            "Assign 10 damage to sha256:33333333…",
            "Intercept with sha256:44444444…",
            "Close intercept window",
        ]
    );
}

#[test]
fn selected_descriptors_should_round_trip_to_identical_canonical_values() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("valid TypeScript parity fixture");
    let games = fixture["games"].as_array().expect("fixture games");
    let mut kinds = BTreeSet::new();
    let mut target_kinds = BTreeSet::new();
    let mut count = 0;

    for game in games {
        for step in game["steps"].as_array().expect("fixture game steps") {
            let expected = &step["selectedAction"]["descriptor"];
            let descriptor: ActionDescriptor =
                serde_json::from_value(expected.clone()).expect("typed selected descriptor");
            let actual = serde_json::to_value(&descriptor).expect("serialize selected descriptor");
            assert_eq!(actual, *expected);
            assert_eq!(
                canonical_json(&actual).expect("canonical Rust descriptor"),
                canonical_json(expected).expect("canonical TypeScript descriptor")
            );
            kinds.insert(descriptor_kind(&descriptor));
            if let ActionDescriptor::DeclareAttack { target } = descriptor {
                target_kinds.insert(match target {
                    CombatTarget::Avatar { .. } => "avatar",
                    CombatTarget::Minion { .. } => "minion",
                    CombatTarget::Site { .. } => "site",
                });
            }
            count += 1;
        }
    }

    assert_eq!(count, 667);
    assert_eq!(
        kinds,
        BTreeSet::from([
            "close-defend",
            "declare-attack",
            "decline-attack",
            "draw",
            "end-turn",
            "move-and-attack",
            "mulligan",
            "play-site",
            "summon-minion",
        ])
    );
    assert_eq!(target_kinds, BTreeSet::from(["avatar", "minion", "site"]));
}

#[test]
fn cast_magic_descriptors_order_and_action_ids_should_match_typescript() {
    let fixture: Value =
        serde_json::from_str(CAST_MAGIC_FIXTURE).expect("valid Cast Magic parity fixture");
    let contract = fixture["contract"].as_str().expect("action contract");
    let state_version = fixture["stateVersion"]
        .as_u64()
        .expect("fixture state version");
    let mut ordered = Vec::new();
    for action in fixture["actions"].as_array().expect("fixture actions") {
        let descriptor: ActionDescriptor = serde_json::from_value(action["descriptor"].clone())
            .expect("typed Cast Magic descriptor");
        let serialized = serde_json::to_value(&descriptor).expect("serialized descriptor");
        assert_eq!(serialized, action["descriptor"]);
        let expected_id =
            IdentityHash::parse(action["actionId"].as_str().expect("TypeScript action ID"))
                .expect("valid TypeScript action ID");
        assert_eq!(
            opaque_action_id(contract, Seat::North, state_version, &serialized)
                .expect("Rust action identity"),
            expected_id
        );
        ordered.push((
            canonical_json(&serialized).expect("canonical descriptor"),
            expected_id,
        ));
    }
    ordered.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        ordered
            .into_iter()
            .map(|(_, action_id)| action_id.to_string())
            .collect::<Vec<_>>(),
        fixture["canonicalActionIds"]
            .as_array()
            .expect("canonical TypeScript order")
            .iter()
            .map(|action_id| action_id.as_str().expect("action ID").to_owned())
            .collect::<Vec<_>>()
    );
}

#[test]
fn draw_zones_should_round_trip_with_lowercase_values() {
    for (zone, expected) in [
        (DeckZone::Atlas, "atlas"),
        (DeckZone::Spellbook, "spellbook"),
    ] {
        let descriptor = ActionDescriptor::Draw { zone };
        let value = serde_json::to_value(&descriptor).expect("serialize draw descriptor");
        assert_eq!(value, json!({ "kind": "draw", "zone": expected }));
        assert_eq!(
            serde_json::from_value::<ActionDescriptor>(value).expect("deserialize draw descriptor"),
            descriptor
        );
    }
}

#[test]
fn combat_targets_should_round_trip_with_typed_kinds() {
    let id = IdentityHash::parse(NORTH_AVATAR).expect("valid identity");
    let descriptors = [
        ActionDescriptor::DeclareAttack {
            target: CombatTarget::Avatar {
                instance_id: id.clone(),
                seat: Seat::North,
            },
        },
        ActionDescriptor::DeclareAttack {
            target: CombatTarget::Minion {
                instance_id: id.clone(),
                seat: Seat::North,
            },
        },
        ActionDescriptor::DeclareAttack {
            target: CombatTarget::Site {
                instance_id: id,
                seat: Seat::North,
            },
        },
    ];

    for descriptor in descriptors {
        let value = serde_json::to_value(&descriptor).expect("serialize attack descriptor");
        assert_eq!(
            serde_json::from_value::<ActionDescriptor>(value)
                .expect("deserialize attack descriptor"),
            descriptor
        );
    }
}

#[test]
fn unsupported_fields_and_null_substitutions_should_be_rejected() {
    let with_unsupported_field = json!({
        "cardId": "north-site-18",
        "cardInstanceId": NORTH_AVATAR,
        "cell": "C4",
        "fromTopAtlas": null,
        "kind": "play-site"
    });
    assert!(serde_json::from_value::<ActionDescriptor>(with_unsupported_field).is_err());

    let null_required_field = json!({
        "kind": "close-defend",
        "originalTargetParticipates": null
    });
    assert!(serde_json::from_value::<ActionDescriptor>(null_required_field).is_err());

    let intercept_with_unsupported_field = json!({
        "kind": "intercept",
        "unitInstanceId": NORTH_AVATAR,
        "from": { "cell": "C4", "region": "surface" }
    });
    assert!(serde_json::from_value::<ActionDescriptor>(intercept_with_unsupported_field).is_err());

    let close_intercept_with_unsupported_field = json!({
        "kind": "close-intercept",
        "unitInstanceId": NORTH_AVATAR
    });
    assert!(
        serde_json::from_value::<ActionDescriptor>(close_intercept_with_unsupported_field).is_err()
    );
}

#[test]
fn labels_should_match_only_state_independent_typescript_labels() {
    let keep = ActionDescriptor::Mulligan {
        atlas_order: Vec::new(),
        spellbook_order: Vec::new(),
    };
    let attack = ActionDescriptor::DeclareAttack {
        target: CombatTarget::Avatar {
            instance_id: IdentityHash::parse(NORTH_AVATAR).expect("valid identity"),
            seat: Seat::North,
        },
    };
    let site: ActionDescriptor = serde_json::from_value(json!({
        "cardId": "north-site-18",
        "cardInstanceId": NORTH_AVATAR,
        "cell": "C4",
        "kind": "play-site"
    }))
    .expect("valid site descriptor");

    assert_eq!(
        keep.state_independent_label().as_deref(),
        Some("Keep opening hand")
    );
    assert_eq!(
        attack.state_independent_label().as_deref(),
        Some("Attack avatar sha256:310a489a…")
    );
    assert_eq!(site.state_independent_label(), None);
}
