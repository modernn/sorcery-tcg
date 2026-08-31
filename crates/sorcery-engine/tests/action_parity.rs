use std::collections::BTreeSet;

use serde_json::{Value, json};
use sorcery_engine::action::{ActionDescriptor, CombatTarget, DeckZone};
use sorcery_engine::canonical::{IdentityHash, canonical_json};
use sorcery_engine::contract::Seat;

const FIXTURE: &str = include_str!("../../../tests/engine/fixtures/typescript-parity-v1.json");
const NORTH_AVATAR: &str =
    "sha256:310a489a62739a8b1a6a13bf949daa8dc42ab0995619e5288691a0ac86a2472e";

fn descriptor_kind(descriptor: &ActionDescriptor) -> &'static str {
    match descriptor {
        ActionDescriptor::ActivateMana { .. } => "activate-mana",
        ActionDescriptor::Mulligan { .. } => "mulligan",
        ActionDescriptor::Draw { .. } => "draw",
        ActionDescriptor::DrawSite => "draw-site",
        ActionDescriptor::DrawSpell => "draw-spell",
        ActionDescriptor::PlaySite { .. } => "play-site",
        ActionDescriptor::ReplaceRubbleWithTopAtlasSite { .. } => {
            "replace-rubble-with-top-atlas-site"
        }
        ActionDescriptor::ResolveGenesisToken { .. } => "resolve-genesis-token",
        ActionDescriptor::SummonMinion { .. } => "summon-minion",
        ActionDescriptor::MoveAndAttack { .. } => "move-and-attack",
        ActionDescriptor::DeclineAttack => "decline-attack",
        ActionDescriptor::DeclareAttack { .. } => "declare-attack",
        ActionDescriptor::CloseDefend { .. } => "close-defend",
        ActionDescriptor::EndTurn => "end-turn",
    }
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
