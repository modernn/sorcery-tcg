use std::collections::BTreeSet;

use serde_json::{Value, json};
use sorcery_engine::action::{ActionDescriptor, CombatTarget, DeckZone};
use sorcery_engine::canonical::{IdentityHash, canonical_json};
use sorcery_engine::contract::{Seat, opaque_action_id};

const FIXTURE: &str = include_str!("../../../tests/engine/fixtures/typescript-parity-v1.json");
const CAST_MAGIC_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/cast-magic-action-v1.json");
const COMBAT_RESPONSE_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/combat-response-action-v1.json");
const DEATHRITE_ORDER_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/deathrite-order-action-v1.json");
const SHOOT_PROJECTILE_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/shoot-projectile-action-v1.json");
const SHOOT_DAMAGE_PROJECTILE_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/shoot-damage-projectile-action-v1.json");
const SPARKMAGE_FIXTURE: &str =
    include_str!("../../../tests/engine/fixtures/sparkmage-action-v1.json");
const NORTH_AVATAR: &str =
    "sha256:310a489a62739a8b1a6a13bf949daa8dc42ab0995619e5288691a0ac86a2472e";

fn descriptor_kind(descriptor: &ActionDescriptor) -> &'static str {
    match descriptor {
        ActionDescriptor::ActivateMana { .. } => "activate-mana",
        ActionDescriptor::ActivateSparkmage { .. } => "activate-sparkmage",
        ActionDescriptor::AllocateStrike { .. } => "allocate-strike",
        ActionDescriptor::BeginChainMagic { .. } => "begin-chain-magic",
        ActionDescriptor::CastMagic { .. } => "cast-magic",
        ActionDescriptor::Mulligan { .. } => "mulligan",
        ActionDescriptor::Draw { .. } => "draw",
        ActionDescriptor::DrawSite => "draw-site",
        ActionDescriptor::DrawSpell => "draw-spell",
        ActionDescriptor::PlaySite { .. } => "play-site",
        ActionDescriptor::ReplaceRubbleWithTopAtlasSite { .. } => {
            "replace-rubble-with-top-atlas-site"
        }
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
        ActionDescriptor::CloseIntercept {} => "close-intercept",
        ActionDescriptor::EndTurn => "end-turn",
        ActionDescriptor::ExtendChainMagic { .. } => "extend-chain-magic",
        ActionDescriptor::ResolveChainMagic => "resolve-chain-magic",
    }
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
