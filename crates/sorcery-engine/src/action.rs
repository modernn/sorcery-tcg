//! Typed descriptors for the currently supported synthetic game workload.

use std::cmp::Ordering;
use std::str::Bytes;

use serde::{Deserialize, Serialize};

use crate::board::{Cell, Location, Region};
use crate::canonical::IdentityHash;
use crate::contract::Seat;

/// A deck from which a player may draw.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeckZone {
    /// The player's site deck.
    Atlas,
    /// The player's spell deck.
    Spellbook,
}

/// An engine-issued choice for an optional paid site Genesis token.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GenesisTokenChoice {
    /// Resolve the site without paying for its token.
    Decline,
    /// Spend the mana gained by playing the site and summon its token.
    PayOneMana,
}

/// An engine-issued choice for the hidden top Spellbook card.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GenesisSpellChoice {
    /// Move the next spell to the bottom of its Spellbook.
    BottomNext,
    /// Leave the next spell on top of its Spellbook.
    KeepNext,
}

/// An engine-issued branch for optional targeted Genesis damage.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GenesisDamageChoice {
    /// Resolve the summon without dealing Genesis damage.
    Decline,
    /// Deal Genesis damage to the accompanying engine-issued target.
    Target,
}

/// A cardinal projectile ray direction in canonical string order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectileDirection {
    /// Increasing file.
    East,
    /// Increasing rank.
    North,
    /// Decreasing rank.
    South,
    /// Decreasing file.
    West,
}

fn required_nullable_unit_target<'de, D>(deserializer: D) -> Result<Option<UnitTarget>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<UnitTarget>::deserialize(deserializer)
}

impl DeckZone {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Atlas => "atlas",
            Self::Spellbook => "spellbook",
        }
    }
}

/// A unit or site that can be declared as an attack target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CombatTarget {
    /// A player's Avatar.
    Avatar {
        /// Authoritative Avatar instance identity.
        instance_id: IdentityHash,
        /// Avatar owner.
        seat: Seat,
    },
    /// A minion in the realm.
    Minion {
        /// Authoritative minion instance identity.
        instance_id: IdentityHash,
        /// Minion owner.
        seat: Seat,
    },
    /// A site in the realm.
    Site {
        /// Authoritative site instance identity.
        instance_id: IdentityHash,
        /// Site owner.
        seat: Seat,
    },
}

/// An Avatar or minion selected by a unit-targeting effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum UnitTarget {
    /// A player's Avatar.
    Avatar {
        /// Authoritative Avatar instance identity.
        instance_id: IdentityHash,
        /// Avatar owner.
        seat: Seat,
    },
    /// A minion in the realm.
    Minion {
        /// Authoritative minion instance identity.
        instance_id: IdentityHash,
        /// Minion owner.
        seat: Seat,
    },
}

impl UnitTarget {
    pub(crate) const fn kind(&self) -> &'static str {
        match self {
            Self::Avatar { .. } => "avatar",
            Self::Minion { .. } => "minion",
        }
    }

    pub(crate) fn instance_id(&self) -> &IdentityHash {
        match self {
            Self::Avatar { instance_id, .. } | Self::Minion { instance_id, .. } => instance_id,
        }
    }

    pub(crate) const fn seat(&self) -> Seat {
        match self {
            Self::Avatar { seat, .. } | Self::Minion { seat, .. } => *seat,
        }
    }
}

impl CombatTarget {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Avatar { .. } => "avatar",
            Self::Minion { .. } => "minion",
            Self::Site { .. } => "site",
        }
    }

    pub(crate) fn instance_id(&self) -> &IdentityHash {
        match self {
            Self::Avatar { instance_id, .. }
            | Self::Minion { instance_id, .. }
            | Self::Site { instance_id, .. } => instance_id,
        }
    }

    pub(crate) const fn seat(&self) -> Seat {
        match self {
            Self::Avatar { seat, .. } | Self::Minion { seat, .. } | Self::Site { seat, .. } => {
                *seat
            }
        }
    }
}

/// An engine-issued action payload for the supported synthetic rules slice.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum ActionDescriptor {
    /// Tap one ready minion to add its printed mana amount.
    ActivateMana {
        /// Printed mana added by the ability.
        amount: u64,
        /// Authoritative source minion identity.
        unit_instance_id: IdentityHash,
    },
    /// Keep an opening hand or return selected cards in the specified order.
    Mulligan {
        /// Atlas instance IDs returned to the deck bottom, in order.
        atlas_order: Vec<IdentityHash>,
        /// Spellbook instance IDs returned to the deck bottom, in order.
        spellbook_order: Vec<IdentityHash>,
    },
    /// Draw the top card from one deck.
    Draw {
        /// Deck selected for the draw.
        zone: DeckZone,
    },
    /// Tap the Avatar to draw the top Atlas card during the main phase.
    DrawSite,
    /// Tap a capable Avatar to draw the top Spellbook card during the main phase.
    DrawSpell,
    /// Play a site from the player's hand.
    PlaySite {
        /// Stable rules card identity.
        card_id: String,
        /// Authoritative card instance identity.
        card_instance_id: IdentityHash,
        /// Empty realm cell receiving the site.
        cell: Cell,
        /// Empty cell receiving mandatory Geomancer Rubble, when issued.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        create_rubble_at: Option<Cell>,
        /// Issued branch for a site with optional paid-token Genesis.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        genesis_token_choice: Option<GenesisTokenChoice>,
    },
    /// Replace adjacent Rubble with the still-hidden top Atlas site.
    ReplaceRubbleWithTopAtlasSite {
        /// Public Rubble cell being replaced.
        target_cell: Cell,
        /// Exact public Rubble identity that made the action legal.
        target_rubble_instance_id: IdentityHash,
    },
    /// Resolve a deferred paid-token Genesis after a hidden site is revealed.
    ResolveGenesisToken {
        /// Decline or pay for the revealed site's token.
        choice: GenesisTokenChoice,
    },
    /// Keep or bottom a still-hidden top Spellbook card.
    ResolveGenesisSpell {
        /// Engine-issued hidden-card operation.
        choice: GenesisSpellChoice,
    },
    /// Reorder the still-hidden top Spellbook cards by prefix index.
    ResolveGenesisSpellOrder {
        /// Permutation of `0..pending_count`.
        order: Vec<u8>,
    },
    /// Summon a minion from the player's hand.
    SummonMinion {
        /// Stable rules card identity.
        card_id: String,
        /// Authoritative card instance identity.
        card_instance_id: IdentityHash,
        /// Authoritative caster instance identity.
        caster_instance_id: IdentityHash,
        /// Realm cell receiving the minion.
        cell: Cell,
        /// Decline or select the accompanying optional Genesis damage.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        genesis_damage_choice: Option<GenesisDamageChoice>,
        /// Exact Avatar or minion selected for optional Genesis damage.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        genesis_damage_target: Option<UnitTarget>,
        /// Mana paid for the summon.
        mana_cost: u64,
    },
    /// Cast one supported Magic card from the player's hand.
    CastMagic {
        /// Stable rules card identity.
        card_id: String,
        /// Authoritative card instance identity.
        card_instance_id: IdentityHash,
        /// Authoritative Spellcaster instance identity.
        caster_instance_id: IdentityHash,
        /// Exact own cemetery minion selected by Rescue.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cemetery_minion_instance_id: Option<IdentityHash>,
        /// Exact engine-issued unit target.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<UnitTarget>,
    },
    /// Tap a unit and follow an issued movement path before choosing an attack.
    MoveAndAttack {
        /// Unit location before movement.
        from: Location,
        /// Complete issued path, including the starting location.
        path: Vec<Location>,
        /// Unit location after movement.
        to: Location,
        /// Authoritative moving unit identity.
        unit_instance_id: IdentityHash,
    },
    /// Tap a ready Ranged unit to strike the first visible unit along one issued ray.
    ShootProjectile {
        /// Cardinal direction of travel.
        direction: ProjectileDirection,
        /// First visible unit hit, or explicit null when the ray is empty.
        #[serde(deserialize_with = "required_nullable_unit_target")]
        hit: Option<UnitTarget>,
        /// Complete ray, including the shooter's origin.
        path: Vec<Location>,
        /// Authoritative source unit identity.
        shooter_instance_id: IdentityHash,
    },
    /// Tap a ready minion to shoot its fixed-damage projectile along one issued ray.
    ShootDamageProjectile {
        /// Cardinal direction of travel.
        direction: ProjectileDirection,
        /// First visible unit hit, or explicit null when the ray is empty.
        #[serde(deserialize_with = "required_nullable_unit_target")]
        hit: Option<UnitTarget>,
        /// Complete ray, including the shooter's origin.
        path: Vec<Location>,
        /// Authoritative source minion identity.
        shooter_instance_id: IdentityHash,
    },
    /// Decline to attack after moving or tapping in place.
    DeclineAttack,
    /// Attack one engine-issued target.
    DeclareAttack {
        /// Chosen combat target.
        target: CombatTarget,
    },
    /// Move one ready unit along an engine-issued path to join a fight.
    Defend {
        /// Unit location before movement.
        from: Location,
        /// Complete issued path, including the starting location.
        path: Vec<Location>,
        /// Fight location after movement.
        to: Location,
        /// Authoritative defending unit identity.
        unit_instance_id: IdentityHash,
    },
    /// Close the defend window.
    CloseDefend {
        /// Whether the original attack target remains a combatant.
        original_target_participates: bool,
    },
    /// Move one ready unit at the fight location into combat.
    Intercept {
        /// Authoritative intercepting unit identity.
        unit_instance_id: IdentityHash,
    },
    /// Close the intercept window without adding another combatant.
    CloseIntercept {},
    /// Assign one striker's damage to a combatant.
    AllocateStrike {
        /// Damage assigned by this action.
        amount: u64,
        /// Authoritative target identity.
        target_instance_id: IdentityHash,
    },
    /// Commit one source first within the acting player's simultaneous Deathrites.
    OrderDeathrites {
        /// Authoritative dead minion source identity.
        source_instance_id: IdentityHash,
    },
    /// End the acting player's turn.
    EndTurn,
}

impl ActionDescriptor {
    /// Produces the existing TypeScript label when no game-state lookup is needed.
    ///
    /// Site and summon labels return `None` because their established labels
    /// depend on rule facts or the current phase and board state.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "closed descriptor labels mirror the TypeScript action contract"
    )]
    pub fn state_independent_label(&self) -> Option<String> {
        match self {
            Self::ActivateMana {
                amount,
                unit_instance_id,
            } => Some(format!(
                "Tap {}… for {amount} mana",
                short_identity(unit_instance_id)
            )),
            Self::Mulligan {
                atlas_order,
                spellbook_order,
            } => {
                let count = atlas_order.len() + spellbook_order.len();
                Some(if count == 0 {
                    "Keep opening hand".to_owned()
                } else {
                    format!(
                        "Mulligan {count} ({} atlas, {} spellbook)",
                        atlas_order.len(),
                        spellbook_order.len()
                    )
                })
            }
            Self::Draw { zone } => Some(format!("Draw from {}", zone.as_str())),
            Self::DrawSite => Some("Draw a site with Avatar".to_owned()),
            Self::DrawSpell => Some("Draw a spell with Avatar".to_owned()),
            Self::CastMagic {
                card_id,
                cemetery_minion_instance_id,
                target,
                ..
            } => Some(if let Some(instance_id) = cemetery_minion_instance_id {
                format!(
                    "Cast {card_id} to return minion {}…",
                    short_identity(instance_id)
                )
            } else if let Some(target) = target {
                format!(
                    "Cast {card_id} on {} {}…",
                    target.kind(),
                    short_identity(target.instance_id())
                )
            } else {
                format!("Cast {card_id}")
            }),
            Self::MoveAndAttack {
                path,
                to,
                unit_instance_id,
                ..
            } => Some(if path.len() == 1 {
                let region = if to.region == Region::Surface {
                    String::new()
                } else {
                    format!(" {}", region_name(to.region))
                };
                format!(
                    "Tap {}… without moving{region}",
                    short_identity(unit_instance_id)
                )
            } else {
                let path = path
                    .iter()
                    .copied()
                    .map(location_label)
                    .collect::<Vec<_>>()
                    .join(" → ");
                format!("Move {}… {path}", short_identity(unit_instance_id))
            }),
            Self::ShootProjectile { direction, hit, .. }
            | Self::ShootDamageProjectile { direction, hit, .. } => {
                let direction = match direction {
                    ProjectileDirection::East => "east",
                    ProjectileDirection::North => "north",
                    ProjectileDirection::South => "south",
                    ProjectileDirection::West => "west",
                };
                let target = hit.as_ref().map_or_else(
                    || "nothing".to_owned(),
                    |target| {
                        format!(
                            "{} {}…",
                            target.kind(),
                            short_identity(target.instance_id())
                        )
                    },
                );
                Some(format!("Shoot {direction} at {target}"))
            }
            Self::DeclineAttack => Some("Decline attack".to_owned()),
            Self::DeclareAttack { target } => Some(format!(
                "Attack {} {}…",
                target.kind(),
                short_identity(target.instance_id())
            )),
            Self::Defend {
                path,
                unit_instance_id,
                ..
            } => Some(format!(
                "Defend with {}… via {}",
                short_identity(unit_instance_id),
                path.iter()
                    .copied()
                    .map(location_label)
                    .collect::<Vec<_>>()
                    .join(" → ")
            )),
            Self::CloseDefend {
                original_target_participates,
            } => Some(
                if *original_target_participates {
                    "Close defend window; keep target"
                } else {
                    "Close defend window; remove target"
                }
                .to_owned(),
            ),
            Self::Intercept { unit_instance_id } => Some(format!(
                "Intercept with {}…",
                short_identity(unit_instance_id)
            )),
            Self::CloseIntercept {} => Some("Close intercept window".to_owned()),
            Self::AllocateStrike {
                amount,
                target_instance_id,
            } => Some(format!(
                "Assign {amount} damage to {}…",
                short_identity(target_instance_id)
            )),
            Self::EndTurn => Some("End turn".to_owned()),
            Self::ReplaceRubbleWithTopAtlasSite { target_cell, .. } => Some(format!(
                "Replace Rubble at {target_cell} with the top site of your Atlas"
            )),
            Self::PlaySite { .. }
            | Self::OrderDeathrites { .. }
            | Self::ResolveGenesisSpell { .. }
            | Self::ResolveGenesisSpellOrder { .. }
            | Self::ResolveGenesisToken { .. }
            | Self::SummonMinion { .. } => None,
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "closed descriptor ordering mirrors canonical JSON field order"
)]
pub(crate) fn compare_canonical(left: &ActionDescriptor, right: &ActionDescriptor) -> Ordering {
    descriptor_group(left)
        .cmp(&descriptor_group(right))
        .then_with(|| match (left, right) {
            (
                ActionDescriptor::ActivateMana {
                    amount: left_amount,
                    unit_instance_id: left_unit,
                },
                ActionDescriptor::ActivateMana {
                    amount: right_amount,
                    unit_instance_id: right_unit,
                },
            ) => compare_mana_activations(*left_amount, left_unit, *right_amount, right_unit),
            (
                ActionDescriptor::AllocateStrike {
                    amount: left_amount,
                    target_instance_id: left_target,
                },
                ActionDescriptor::AllocateStrike {
                    amount: right_amount,
                    target_instance_id: right_target,
                },
            ) => compare_json_integers(*left_amount, *right_amount)
                .then_with(|| left_target.cmp(right_target)),
            (
                ActionDescriptor::ActivateMana {
                    amount: left_amount,
                    ..
                },
                ActionDescriptor::AllocateStrike {
                    amount: right_amount,
                    ..
                },
            ) => compare_json_integers(*left_amount, *right_amount).then(Ordering::Less),
            (
                ActionDescriptor::AllocateStrike {
                    amount: left_amount,
                    ..
                },
                ActionDescriptor::ActivateMana {
                    amount: right_amount,
                    ..
                },
            ) => compare_json_integers(*left_amount, *right_amount).then(Ordering::Greater),
            (
                ActionDescriptor::Mulligan {
                    atlas_order: left_atlas,
                    spellbook_order: left_spellbook,
                },
                ActionDescriptor::Mulligan {
                    atlas_order: right_atlas,
                    spellbook_order: right_spellbook,
                },
            ) => compare_json_array(left_atlas, right_atlas, IdentityHash::cmp).then_with(|| {
                compare_json_array(left_spellbook, right_spellbook, IdentityHash::cmp)
            }),
            (
                ActionDescriptor::PlaySite {
                    card_id: left_card,
                    card_instance_id: left_instance,
                    cell: left_cell,
                    create_rubble_at: left_rubble,
                    genesis_token_choice: left_choice,
                },
                ActionDescriptor::PlaySite {
                    card_id: right_card,
                    card_instance_id: right_instance,
                    cell: right_cell,
                    create_rubble_at: right_rubble,
                    genesis_token_choice: right_choice,
                },
            ) => compare_json_strings(left_card, right_card)
                .then_with(|| left_instance.cmp(right_instance))
                .then_with(|| left_cell.cmp(right_cell))
                .then_with(|| compare_optional_cells(*left_rubble, *right_rubble))
                .then_with(|| compare_optional_genesis_choices(*left_choice, *right_choice)),
            (
                ActionDescriptor::CastMagic {
                    card_id: left_card,
                    card_instance_id: left_instance,
                    caster_instance_id: left_caster,
                    cemetery_minion_instance_id: left_cemetery,
                    target: left_target,
                },
                ActionDescriptor::CastMagic {
                    card_id: right_card,
                    card_instance_id: right_instance,
                    caster_instance_id: right_caster,
                    cemetery_minion_instance_id: right_cemetery,
                    target: right_target,
                },
            ) => compare_json_strings(left_card, right_card)
                .then_with(|| left_instance.cmp(right_instance))
                .then_with(|| left_caster.cmp(right_caster))
                .then_with(|| {
                    compare_optional_identities(left_cemetery.as_ref(), right_cemetery.as_ref())
                })
                .then_with(|| {
                    compare_optional_unit_targets(left_target.as_ref(), right_target.as_ref())
                }),
            (
                ActionDescriptor::SummonMinion {
                    card_id: left_card,
                    card_instance_id: left_instance,
                    caster_instance_id: left_caster,
                    cell: left_cell,
                    genesis_damage_choice: left_choice,
                    genesis_damage_target: left_target,
                    mana_cost: left_mana,
                },
                ActionDescriptor::SummonMinion {
                    card_id: right_card,
                    card_instance_id: right_instance,
                    caster_instance_id: right_caster,
                    cell: right_cell,
                    genesis_damage_choice: right_choice,
                    genesis_damage_target: right_target,
                    mana_cost: right_mana,
                },
            ) => compare_json_strings(left_card, right_card)
                .then_with(|| left_instance.cmp(right_instance))
                .then_with(|| left_caster.cmp(right_caster))
                .then_with(|| left_cell.cmp(right_cell))
                .then_with(|| compare_optional_genesis_damage_choices(*left_choice, *right_choice))
                .then_with(|| {
                    compare_optional_unit_targets(left_target.as_ref(), right_target.as_ref())
                })
                .then_with(|| compare_json_integers(*left_mana, *right_mana)),
            (ActionDescriptor::CastMagic { .. }, ActionDescriptor::PlaySite { .. })
            | (ActionDescriptor::SummonMinion { .. }, ActionDescriptor::CastMagic { .. })
            | (ActionDescriptor::SummonMinion { .. }, ActionDescriptor::PlaySite { .. }) => {
                compare_card_prefix(left, right).then(Ordering::Less)
            }
            (ActionDescriptor::PlaySite { .. }, ActionDescriptor::CastMagic { .. })
            | (ActionDescriptor::CastMagic { .. }, ActionDescriptor::SummonMinion { .. })
            | (ActionDescriptor::PlaySite { .. }, ActionDescriptor::SummonMinion { .. }) => {
                compare_card_prefix(left, right).then(Ordering::Greater)
            }
            (
                ActionDescriptor::ShootProjectile { .. }
                | ActionDescriptor::ShootDamageProjectile { .. },
                ActionDescriptor::ShootProjectile { .. }
                | ActionDescriptor::ShootDamageProjectile { .. },
            ) => compare_projectiles(left, right),
            (
                ActionDescriptor::MoveAndAttack {
                    from: left_from,
                    path: left_path,
                    to: left_to,
                    unit_instance_id: left_unit,
                },
                ActionDescriptor::MoveAndAttack {
                    from: right_from,
                    path: right_path,
                    to: right_to,
                    unit_instance_id: right_unit,
                },
            ) => left_from
                .cmp(right_from)
                .then_with(|| compare_json_array(left_path, right_path, Location::cmp))
                .then_with(|| left_to.cmp(right_to))
                .then_with(|| left_unit.cmp(right_unit)),
            (
                ActionDescriptor::Defend {
                    from: left_from,
                    path: left_path,
                    to: left_to,
                    unit_instance_id: left_unit,
                },
                ActionDescriptor::Defend {
                    from: right_from,
                    path: right_path,
                    to: right_to,
                    unit_instance_id: right_unit,
                },
            ) => left_from
                .cmp(right_from)
                .then_with(|| compare_json_array(left_path, right_path, Location::cmp))
                .then_with(|| left_to.cmp(right_to))
                .then_with(|| left_unit.cmp(right_unit)),
            (
                ActionDescriptor::Defend {
                    from: left_from, ..
                },
                ActionDescriptor::MoveAndAttack {
                    from: right_from, ..
                },
            ) => left_from.cmp(right_from).then(Ordering::Less),
            (
                ActionDescriptor::MoveAndAttack {
                    from: left_from, ..
                },
                ActionDescriptor::Defend {
                    from: right_from, ..
                },
            ) => left_from.cmp(right_from).then(Ordering::Greater),
            _ => action_kind(left)
                .cmp(&action_kind(right))
                .then_with(|| match (left, right) {
                    (
                        ActionDescriptor::CloseDefend {
                            original_target_participates: left,
                        },
                        ActionDescriptor::CloseDefend {
                            original_target_participates: right,
                        },
                    ) => left.cmp(right),
                    (
                        ActionDescriptor::Intercept {
                            unit_instance_id: left,
                        },
                        ActionDescriptor::Intercept {
                            unit_instance_id: right,
                        },
                    )
                    | (
                        ActionDescriptor::OrderDeathrites {
                            source_instance_id: left,
                        },
                        ActionDescriptor::OrderDeathrites {
                            source_instance_id: right,
                        },
                    ) => left.cmp(right),
                    (
                        ActionDescriptor::DeclareAttack { target: left },
                        ActionDescriptor::DeclareAttack { target: right },
                    ) => compare_targets(left, right),
                    (
                        ActionDescriptor::Draw { zone: left },
                        ActionDescriptor::Draw { zone: right },
                    ) => deck_zone_order(*left).cmp(&deck_zone_order(*right)),
                    (
                        ActionDescriptor::ReplaceRubbleWithTopAtlasSite {
                            target_cell: left_cell,
                            target_rubble_instance_id: left_id,
                        },
                        ActionDescriptor::ReplaceRubbleWithTopAtlasSite {
                            target_cell: right_cell,
                            target_rubble_instance_id: right_id,
                        },
                    ) => left_cell
                        .cmp(right_cell)
                        .then_with(|| left_id.cmp(right_id)),
                    (
                        ActionDescriptor::ResolveGenesisSpell { choice: left },
                        ActionDescriptor::ResolveGenesisSpell { choice: right },
                    ) => left.cmp(right),
                    (
                        ActionDescriptor::ResolveGenesisSpellOrder { order: left },
                        ActionDescriptor::ResolveGenesisSpellOrder { order: right },
                    ) => compare_json_array(left, right, u8::cmp),
                    (
                        ActionDescriptor::ResolveGenesisToken { choice: left },
                        ActionDescriptor::ResolveGenesisToken { choice: right },
                    ) => left.cmp(right),
                    _ => Ordering::Equal,
                }),
        })
}

fn compare_mana_activations(
    left_amount: u64,
    left_unit: &IdentityHash,
    right_amount: u64,
    right_unit: &IdentityHash,
) -> Ordering {
    compare_json_integers(left_amount, right_amount).then_with(|| left_unit.cmp(right_unit))
}

fn compare_optional_genesis_choices(
    left: Option<GenesisTokenChoice>,
    right: Option<GenesisTokenChoice>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_genesis_damage_choices(
    left: Option<GenesisDamageChoice>,
    right: Option<GenesisDamageChoice>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_unit_targets(
    left: Option<&UnitTarget>,
    right: Option<&UnitTarget>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_unit_targets(left, right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_nullable_unit_targets(
    left: Option<&UnitTarget>,
    right: Option<&UnitTarget>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_unit_targets(left, right),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn compare_projectiles(left: &ActionDescriptor, right: &ActionDescriptor) -> Ordering {
    let (left_direction, left_hit, left_kind, left_path, left_shooter) = projectile_fields(left);
    let (right_direction, right_hit, right_kind, right_path, right_shooter) =
        projectile_fields(right);
    left_direction
        .cmp(&right_direction)
        .then_with(|| compare_nullable_unit_targets(left_hit, right_hit))
        .then_with(|| compare_json_strings(left_kind, right_kind))
        .then_with(|| compare_json_array(left_path, right_path, Location::cmp))
        .then_with(|| left_shooter.cmp(right_shooter))
}

fn projectile_fields(
    action: &ActionDescriptor,
) -> (
    ProjectileDirection,
    Option<&UnitTarget>,
    &'static str,
    &[Location],
    &IdentityHash,
) {
    match action {
        ActionDescriptor::ShootDamageProjectile {
            direction,
            hit,
            path,
            shooter_instance_id,
        } => (
            *direction,
            hit.as_ref(),
            "shoot-damage-projectile",
            path,
            shooter_instance_id,
        ),
        ActionDescriptor::ShootProjectile {
            direction,
            hit,
            path,
            shooter_instance_id,
        } => (
            *direction,
            hit.as_ref(),
            "shoot-projectile",
            path,
            shooter_instance_id,
        ),
        _ => unreachable!("projectile fields are used only for projectile actions"),
    }
}

fn compare_optional_identities(
    left: Option<&IdentityHash>,
    right: Option<&IdentityHash>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_cells(left: Option<Cell>, right: Option<Cell>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

const fn descriptor_group(action: &ActionDescriptor) -> u8 {
    match action {
        ActionDescriptor::ActivateMana { .. } | ActionDescriptor::AllocateStrike { .. } => 0,
        ActionDescriptor::Mulligan { .. } => 1,
        ActionDescriptor::CastMagic { .. }
        | ActionDescriptor::PlaySite { .. }
        | ActionDescriptor::SummonMinion { .. } => 2,
        ActionDescriptor::ShootDamageProjectile { .. }
        | ActionDescriptor::ShootProjectile { .. } => 3,
        ActionDescriptor::Defend { .. } | ActionDescriptor::MoveAndAttack { .. } => 4,
        _ => 5,
    }
}

fn compare_card_prefix(left: &ActionDescriptor, right: &ActionDescriptor) -> Ordering {
    let (left_card, left_instance) = card_prefix(left);
    let (right_card, right_instance) = card_prefix(right);
    compare_json_strings(left_card, right_card).then_with(|| left_instance.cmp(right_instance))
}

fn card_prefix(action: &ActionDescriptor) -> (&str, &IdentityHash) {
    match action {
        ActionDescriptor::PlaySite {
            card_id,
            card_instance_id,
            ..
        }
        | ActionDescriptor::CastMagic {
            card_id,
            card_instance_id,
            ..
        }
        | ActionDescriptor::SummonMinion {
            card_id,
            card_instance_id,
            ..
        } => (card_id, card_instance_id),
        _ => unreachable!("card prefix is used only for card actions"),
    }
}

const fn action_kind(action: &ActionDescriptor) -> u8 {
    match action {
        ActionDescriptor::ActivateMana { .. } => 0,
        ActionDescriptor::AllocateStrike { .. } => 1,
        ActionDescriptor::CastMagic { .. } => 2,
        ActionDescriptor::CloseDefend { .. } => 3,
        ActionDescriptor::CloseIntercept {} => 4,
        ActionDescriptor::DeclareAttack { .. } => 5,
        ActionDescriptor::DeclineAttack => 6,
        ActionDescriptor::Defend { .. } => 7,
        ActionDescriptor::Draw { .. } => 8,
        ActionDescriptor::DrawSite => 9,
        ActionDescriptor::DrawSpell => 10,
        ActionDescriptor::EndTurn => 11,
        ActionDescriptor::Intercept { .. } => 12,
        ActionDescriptor::OrderDeathrites { .. } => 13,
        ActionDescriptor::ReplaceRubbleWithTopAtlasSite { .. } => 14,
        ActionDescriptor::ResolveGenesisSpell { .. } => 15,
        ActionDescriptor::ResolveGenesisSpellOrder { .. } => 16,
        ActionDescriptor::ResolveGenesisToken { .. } => 17,
        ActionDescriptor::Mulligan { .. } => 18,
        ActionDescriptor::PlaySite { .. } => 19,
        ActionDescriptor::ShootDamageProjectile { .. } => 20,
        ActionDescriptor::ShootProjectile { .. } => 21,
        ActionDescriptor::SummonMinion { .. } | ActionDescriptor::MoveAndAttack { .. } => 22,
    }
}

fn compare_targets(left: &CombatTarget, right: &CombatTarget) -> Ordering {
    left.instance_id()
        .cmp(right.instance_id())
        .then_with(|| target_kind(left).cmp(&target_kind(right)))
        .then_with(|| seat_order(left.seat()).cmp(&seat_order(right.seat())))
}

fn compare_unit_targets(left: &UnitTarget, right: &UnitTarget) -> Ordering {
    left.instance_id()
        .cmp(right.instance_id())
        .then_with(|| unit_target_kind(left).cmp(&unit_target_kind(right)))
        .then_with(|| seat_order(left.seat()).cmp(&seat_order(right.seat())))
}

const fn unit_target_kind(target: &UnitTarget) -> u8 {
    match target {
        UnitTarget::Avatar { .. } => 0,
        UnitTarget::Minion { .. } => 1,
    }
}

const fn target_kind(target: &CombatTarget) -> u8 {
    match target {
        CombatTarget::Avatar { .. } => 0,
        CombatTarget::Minion { .. } => 1,
        CombatTarget::Site { .. } => 2,
    }
}

const fn seat_order(seat: Seat) -> u8 {
    match seat {
        Seat::North => 0,
        Seat::South => 1,
    }
}

const fn deck_zone_order(zone: DeckZone) -> u8 {
    match zone {
        DeckZone::Atlas => 0,
        DeckZone::Spellbook => 1,
    }
}

fn compare_json_array<T>(left: &[T], right: &[T], compare: fn(&T, &T) -> Ordering) -> Ordering {
    left.iter()
        .zip(right)
        .map(|(left, right)| compare(left, right))
        .find(|ordering| *ordering != Ordering::Equal)
        .unwrap_or_else(|| right.len().cmp(&left.len()))
}

fn compare_json_integers(left: u64, right: u64) -> Ordering {
    let (left_digits, left_start) = decimal_digits(left);
    let (right_digits, right_start) = decimal_digits(right);
    let left = &left_digits[left_start..];
    let right = &right_digits[right_start..];
    left.iter()
        .zip(right)
        .map(|(left, right)| left.cmp(right))
        .find(|ordering| *ordering != Ordering::Equal)
        .unwrap_or_else(|| right.len().cmp(&left.len()))
}

fn decimal_digits(mut value: u64) -> ([u8; 20], usize) {
    let mut digits = [0; 20];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            return (digits, start);
        }
    }
}

fn compare_json_strings(left: &str, right: &str) -> Ordering {
    JsonStringBytes::new(left).cmp(JsonStringBytes::new(right))
}

struct JsonStringBytes<'a> {
    bytes: Bytes<'a>,
    escaped: [u8; 6],
    escaped_index: usize,
    escaped_len: usize,
    finished: bool,
}

impl<'a> JsonStringBytes<'a> {
    fn new(value: &'a str) -> Self {
        Self {
            bytes: value.bytes(),
            escaped: [0; 6],
            escaped_index: 0,
            escaped_len: 0,
            finished: false,
        }
    }
}

impl Iterator for JsonStringBytes<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.escaped_index < self.escaped_len {
            let byte = self.escaped[self.escaped_index];
            self.escaped_index += 1;
            return Some(byte);
        }
        let Some(byte) = self.bytes.next() else {
            return (!std::mem::replace(&mut self.finished, true)).then_some(b'"');
        };
        let escaped: &[u8] = match byte {
            b'"' => br#"\""#,
            b'\\' => br"\\",
            0x08 => br"\b",
            b'\t' => br"\t",
            b'\n' => br"\n",
            0x0c => br"\f",
            b'\r' => br"\r",
            0x00..=0x1f => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                self.escaped = [
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    HEX[(byte >> 4) as usize],
                    HEX[(byte & 0xf) as usize],
                ];
                self.escaped_index = 1;
                self.escaped_len = self.escaped.len();
                return Some(self.escaped[0]);
            }
            _ => return Some(byte),
        };
        self.escaped[..escaped.len()].copy_from_slice(escaped);
        self.escaped_index = 1;
        self.escaped_len = escaped.len();
        Some(self.escaped[0])
    }
}

fn short_identity(identity: &IdentityHash) -> &str {
    &identity.as_str()[..15]
}

fn region_name(region: Region) -> &'static str {
    match region {
        Region::Surface => "surface",
        Region::Underground => "underground",
        Region::Underwater => "underwater",
        Region::Void => "void",
    }
}

fn location_label(location: Location) -> String {
    if location.region == Region::Surface {
        location.cell.to_string()
    } else {
        format!("{} {}", location.cell, region_name(location.region))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{ActionDescriptor, compare_canonical, compare_json_integers, compare_json_strings};

    const COMBAT_RESPONSE_FIXTURE: &str =
        include_str!("../../../tests/engine/fixtures/combat-response-action-v1.json");

    #[test]
    fn native_combat_response_order_should_match_typescript() {
        let fixture: Value =
            serde_json::from_str(COMBAT_RESPONSE_FIXTURE).expect("valid parity fixture");
        let mut actions = fixture["actions"]
            .as_array()
            .expect("fixture actions")
            .iter()
            .map(|action| {
                (
                    serde_json::from_value::<ActionDescriptor>(action["descriptor"].clone())
                        .expect("typed descriptor"),
                    action["actionId"].as_str().expect("action ID").to_owned(),
                )
            })
            .collect::<Vec<_>>();

        actions.sort_unstable_by(|(left, _), (right, _)| compare_canonical(left, right));

        assert_eq!(
            actions
                .into_iter()
                .map(|(_, action_id)| action_id)
                .collect::<Vec<_>>(),
            fixture["canonicalActionIds"]
                .as_array()
                .expect("canonical action IDs")
                .iter()
                .map(|action_id| action_id.as_str().expect("action ID").to_owned())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn native_string_order_should_match_canonical_json_escaping() {
        let values = ["", " ", "a", "a ", "a\n", "a\"", "a\\", "é"];
        for left in values {
            for right in values {
                let left_json =
                    serde_json::to_string(&json!({ "cardId": left })).expect("left canonical JSON");
                let right_json = serde_json::to_string(&json!({ "cardId": right }))
                    .expect("right canonical JSON");
                assert_eq!(
                    compare_json_strings(left, right),
                    left_json.cmp(&right_json),
                    "{left:?} compared with {right:?}"
                );
            }
        }
    }

    #[test]
    fn native_integer_order_should_match_canonical_json_delimiters() {
        let values = [0, 1, 2, 9, 10, 11, 20, u64::MAX];
        for left in values {
            for right in values {
                let expected =
                    format!("{{\"manaCost\":{left}}}").cmp(&format!("{{\"manaCost\":{right}}}"));
                assert_eq!(
                    compare_json_integers(left, right),
                    expected,
                    "{left} compared with {right}"
                );
            }
        }
    }
}
