//! Typed descriptors for the currently supported synthetic game workload.

use std::cmp::Ordering;
use std::str::Bytes;

use serde::{Deserialize, Serialize};

use crate::board::{Cell, Location, Region, SquareArea};
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

/// An engine-issued alternative payment for a minion summon.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SummonPaymentMode {
    /// Discard one other random hand card instead of paying mana.
    RandomCardDiscard,
}

/// An engine-issued resolution of the optional step after a Ranged strike.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RangedStepChoice {
    /// Remain at the shooter's current location.
    Decline,
    /// Follow the accompanying one-step path.
    Step,
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
    pub(crate) const fn as_str(self) -> &'static str {
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
    /// Discard one Spellbook card to damage a hidden random other unit at the source's location.
    ActivateDiscardRandomDamage {
        /// Exact Spellbook card discarded to pay for the ability.
        discard_card_instance_id: IdentityHash,
        /// Authoritative source minion identity.
        source_instance_id: IdentityHash,
    },
    /// Tap the Avatar to damage one hidden random other unit at a nearby location.
    ActivateSparkmage {
        /// Authoritative source Avatar identity.
        source_instance_id: IdentityHash,
        /// Engine-issued nearby location; the random target remains private until resolution.
        target_location: Location,
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
    /// Sacrifice one controlled site to destroy a nearby site or Rubble.
    ActivateSiteDestruction {
        /// Exact controlled source site being sacrificed.
        source_site_instance_id: IdentityHash,
        /// Nearby realm cell selected for destruction.
        target_cell: Cell,
        /// Exact site or Rubble identity that made the action legal.
        target_site_instance_id: IdentityHash,
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
        /// Exact canonical two-by-two footprint, when the minion is oversized.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cells: Option<SquareArea>,
        /// Decline or select the accompanying optional Genesis damage.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        genesis_damage_choice: Option<GenesisDamageChoice>,
        /// Exact Avatar or minion selected for optional Genesis damage.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        genesis_damage_target: Option<UnitTarget>,
        /// Mana paid for the summon.
        mana_cost: u64,
        /// Optional non-mana payment selected by the engine.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payment_mode: Option<SummonPaymentMode>,
        /// Exact local minions sacrificed for the engine-issued mana discount.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sacrificed_minion_instance_ids: Option<Vec<IdentityHash>>,
    },
    /// Cast one supported Magic card from the player's hand.
    CastMagic {
        /// Exact engine-issued ally selected by an ally-buffing Magic.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ally: Option<UnitTarget>,
        /// Optional one-step destination chosen for a Leap Attack ally.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ally_destination: Option<Location>,
        /// Selected strike cell inside an oversized Leap Attack footprint.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ally_strike_location: Option<Location>,
        /// Stable rules card identity.
        card_id: String,
        /// Authoritative card instance identity.
        card_instance_id: IdentityHash,
        /// Authoritative Spellcaster instance identity.
        caster_instance_id: IdentityHash,
        /// Exact own cemetery minion selected by Rescue.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cemetery_minion_instance_id: Option<IdentityHash>,
        /// Deck the caster draws from after a Blink teleport.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        draw_zone: Option<DeckZone>,
        /// Exact engine-issued unit target.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<UnitTarget>,
        /// Exact engine-issued realm location targeted by the Magic.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_location: Option<Location>,
        /// Exact site or Rubble instance that made the location target legal.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_site_instance_id: Option<IdentityHash>,
        /// One-step destination the tempted enemy minion takes for Lure.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tempted_destination: Option<Location>,
        /// Exact enemy minion tempted one step closer by Lure.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tempted_enemy: Option<UnitTarget>,
    },
    /// Commit the first engine-issued target for a staged Chain Magic cast.
    BeginChainMagic {
        /// Stable rules card identity.
        card_id: String,
        /// Authoritative card instance identity.
        card_instance_id: IdentityHash,
        /// Authoritative Spellcaster instance identity.
        caster_instance_id: IdentityHash,
        /// First engine-issued unit target.
        target: UnitTarget,
    },
    /// Add one distinct engine-issued target to a staged Chain Magic cast.
    ExtendChainMagic {
        /// Next engine-issued unit target.
        target: UnitTarget,
    },
    /// Finish target selection and pay for the staged Chain Magic cast.
    ResolveChainMagic,
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
    /// Advance one edge, or finish, an already committed basic movement path.
    ContinueBasicMovement {
        /// Authoritative moving minion identity.
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
    /// Tap a ready minion to shoot its drag projectile, hauling the first visible unit hit back
    /// along the ray and optionally fighting it on arrival.
    ShootDragProjectile {
        /// Cardinal direction of travel.
        direction: ProjectileDirection,
        /// Fight the hauled unit once it arrives beside the shooter.
        fight_on_arrival: bool,
        /// First visible unit hit, or explicit null when the ray is empty.
        #[serde(deserialize_with = "required_nullable_unit_target")]
        hit: Option<UnitTarget>,
        /// Complete ray, including the shooter's origin.
        path: Vec<Location>,
        /// Authoritative source minion identity.
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
    /// Decline or take the engine-issued optional step after a Ranged strike.
    ResolveRangedStep {
        /// Decline or take the accompanying path.
        choice: RangedStepChoice,
        /// Location before the optional step.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<Location>,
        /// Complete one-step path, including the starting location.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<Vec<Location>>,
        /// Location after the optional step.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Location>,
        /// Authoritative Ranged minion identity.
        unit_instance_id: IdentityHash,
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
                ally,
                ally_destination,
                ally_strike_location,
                card_id,
                cemetery_minion_instance_id,
                draw_zone,
                target,
                target_location,
                tempted_destination,
                tempted_enemy,
                ..
            } => Some(
                if let (Some(ally), Some(location), Some(zone)) = (ally, target_location, draw_zone)
                {
                    format!(
                        "Cast {card_id} to blink {} {}… to {} and draw from {}",
                        ally.kind(),
                        short_identity(ally.instance_id()),
                        location.cell,
                        zone.as_str()
                    )
                } else if let (Some(ally), Some(enemy), Some(destination)) =
                    (ally, tempted_enemy, tempted_destination)
                {
                    format!(
                        "Cast {card_id}: {} {}… tempts minion {}… to {}",
                        ally.kind(),
                        short_identity(ally.instance_id()),
                        short_identity(enemy.instance_id()),
                        destination.cell
                    )
                } else if let (Some(ally), Some(target)) = (ally, target) {
                    format!(
                        "Cast {card_id}: {} {}… fights {} {}…",
                        ally.kind(),
                        short_identity(ally.instance_id()),
                        target.kind(),
                        short_identity(target.instance_id())
                    )
                } else if let (Some(ally), Some(destination)) = (ally, ally_destination) {
                    let strike = ally_strike_location.unwrap_or(*destination);
                    format!(
                        "Cast {card_id}: {} {}… steps to {} and strikes enemies at {}",
                        ally.kind(),
                        short_identity(ally.instance_id()),
                        destination.cell,
                        strike.cell
                    )
                } else if let (Some(ally), Some(location)) = (ally, target_location) {
                    format!(
                        "Cast {card_id} to teleport {} {}… to {}",
                        ally.kind(),
                        short_identity(ally.instance_id()),
                        location.cell
                    )
                } else if let Some(ally) = ally {
                    format!(
                        "Cast {card_id} to grant Charge to {} {}…",
                        ally.kind(),
                        short_identity(ally.instance_id())
                    )
                } else if let Some(instance_id) = cemetery_minion_instance_id {
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
                } else if let Some(location) = target_location {
                    format!(
                        "Cast {card_id} at {} {}",
                        location.cell,
                        region_name(location.region)
                    )
                } else {
                    format!("Cast {card_id}")
                },
            ),
            Self::BeginChainMagic {
                card_id, target, ..
            } => Some(format!(
                "Choose {} {}… as the first target for {card_id}",
                target.kind(),
                short_identity(target.instance_id())
            )),
            Self::ExtendChainMagic { target } => Some(format!(
                "Add {} {}… as a chained target (+2 mana)",
                target.kind(),
                short_identity(target.instance_id())
            )),
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
            | Self::ShootDamageProjectile { direction, hit, .. } => Some(format!(
                "Shoot {} at {}",
                direction_name(*direction),
                projectile_target_label(hit.as_ref())
            )),
            Self::ShootDragProjectile {
                direction,
                fight_on_arrival,
                hit,
                ..
            } => Some(format!(
                "Hook {} at {}{}",
                direction_name(*direction),
                projectile_target_label(hit.as_ref()),
                if *fight_on_arrival { " and fight" } else { "" }
            )),
            Self::DeclineAttack => Some("Decline attack".to_owned()),
            Self::ResolveRangedStep {
                choice,
                to,
                unit_instance_id,
                ..
            } => Some(match (choice, to) {
                (RangedStepChoice::Decline, None) => {
                    "Decline the optional step after the Ranged strike".to_owned()
                }
                (RangedStepChoice::Step, Some(to)) => {
                    format!("Step {}… to {}", short_identity(unit_instance_id), to.cell)
                }
                _ => return None,
            }),
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
            Self::ActivateSiteDestruction { target_cell, .. } => {
                Some(format!("Sacrifice site to destroy {target_cell}"))
            }
            Self::ActivateDiscardRandomDamage { .. }
            | Self::ActivateSparkmage { .. }
            | Self::PlaySite { .. }
            | Self::ContinueBasicMovement { .. }
            | Self::OrderDeathrites { .. }
            | Self::ResolveGenesisSpell { .. }
            | Self::ResolveGenesisSpellOrder { .. }
            | Self::ResolveGenesisToken { .. }
            | Self::ResolveChainMagic
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
                ActionDescriptor::BeginChainMagic {
                    card_id: left_card,
                    card_instance_id: left_instance,
                    caster_instance_id: left_caster,
                    target: left_target,
                },
                ActionDescriptor::BeginChainMagic {
                    card_id: right_card,
                    card_instance_id: right_instance,
                    caster_instance_id: right_caster,
                    target: right_target,
                },
            ) => compare_json_strings(left_card, right_card)
                .then_with(|| left_instance.cmp(right_instance))
                .then_with(|| left_caster.cmp(right_caster))
                .then_with(|| compare_unit_targets(left_target, right_target)),
            (
                ActionDescriptor::CastMagic {
                    ally: left_ally,
                    ally_destination: left_destination,
                    ally_strike_location: left_strike,
                    card_id: left_card,
                    card_instance_id: left_instance,
                    caster_instance_id: left_caster,
                    cemetery_minion_instance_id: left_cemetery,
                    draw_zone: left_draw_zone,
                    target: left_target,
                    target_location: left_location,
                    target_site_instance_id: left_site,
                    tempted_destination: left_tempted_destination,
                    tempted_enemy: left_tempted_enemy,
                },
                ActionDescriptor::CastMagic {
                    ally: right_ally,
                    ally_destination: right_destination,
                    ally_strike_location: right_strike,
                    card_id: right_card,
                    card_instance_id: right_instance,
                    caster_instance_id: right_caster,
                    cemetery_minion_instance_id: right_cemetery,
                    draw_zone: right_draw_zone,
                    target: right_target,
                    target_location: right_location,
                    target_site_instance_id: right_site,
                    tempted_destination: right_tempted_destination,
                    tempted_enemy: right_tempted_enemy,
                },
            ) => compare_optional_unit_targets(left_ally.as_ref(), right_ally.as_ref())
                .then_with(|| compare_optional_locations(*left_destination, *right_destination))
                .then_with(|| compare_optional_locations(*left_strike, *right_strike))
                .then_with(|| compare_json_strings(left_card, right_card))
                .then_with(|| left_instance.cmp(right_instance))
                .then_with(|| left_caster.cmp(right_caster))
                .then_with(|| {
                    compare_optional_identities(left_cemetery.as_ref(), right_cemetery.as_ref())
                })
                .then_with(|| compare_optional_deck_zones(*left_draw_zone, *right_draw_zone))
                .then_with(|| {
                    compare_optional_unit_targets(left_target.as_ref(), right_target.as_ref())
                })
                .then_with(|| compare_optional_locations(*left_location, *right_location))
                .then_with(|| compare_optional_identities(left_site.as_ref(), right_site.as_ref()))
                .then_with(|| {
                    compare_optional_locations(
                        *left_tempted_destination,
                        *right_tempted_destination,
                    )
                })
                .then_with(|| {
                    compare_optional_unit_targets(
                        left_tempted_enemy.as_ref(),
                        right_tempted_enemy.as_ref(),
                    )
                }),
            (
                ActionDescriptor::SummonMinion {
                    card_id: left_card,
                    card_instance_id: left_instance,
                    caster_instance_id: left_caster,
                    cell: left_cell,
                    cells: left_cells,
                    genesis_damage_choice: left_choice,
                    genesis_damage_target: left_target,
                    mana_cost: left_mana,
                    payment_mode: left_payment,
                    sacrificed_minion_instance_ids: left_sacrifices,
                },
                ActionDescriptor::SummonMinion {
                    card_id: right_card,
                    card_instance_id: right_instance,
                    caster_instance_id: right_caster,
                    cell: right_cell,
                    cells: right_cells,
                    genesis_damage_choice: right_choice,
                    genesis_damage_target: right_target,
                    mana_cost: right_mana,
                    payment_mode: right_payment,
                    sacrificed_minion_instance_ids: right_sacrifices,
                },
            ) => compare_json_strings(left_card, right_card)
                .then_with(|| left_instance.cmp(right_instance))
                .then_with(|| left_caster.cmp(right_caster))
                .then_with(|| left_cell.cmp(right_cell))
                .then_with(|| compare_optional_square_areas(*left_cells, *right_cells))
                .then_with(|| compare_optional_genesis_damage_choices(*left_choice, *right_choice))
                .then_with(|| {
                    compare_optional_unit_targets(left_target.as_ref(), right_target.as_ref())
                })
                .then_with(|| compare_json_integers(*left_mana, *right_mana))
                .then_with(|| compare_optional_summon_payments(*left_payment, *right_payment))
                .then_with(|| {
                    compare_optional_identity_arrays(
                        left_sacrifices.as_deref(),
                        right_sacrifices.as_deref(),
                    )
                }),
            (ActionDescriptor::BeginChainMagic { .. }, ActionDescriptor::CastMagic { .. })
            | (ActionDescriptor::BeginChainMagic { .. }, ActionDescriptor::PlaySite { .. })
            | (ActionDescriptor::CastMagic { .. }, ActionDescriptor::PlaySite { .. })
            | (ActionDescriptor::SummonMinion { .. }, ActionDescriptor::CastMagic { .. })
            | (ActionDescriptor::SummonMinion { .. }, ActionDescriptor::BeginChainMagic { .. })
            | (ActionDescriptor::SummonMinion { .. }, ActionDescriptor::PlaySite { .. }) => {
                compare_card_prefix(left, right).then(Ordering::Less)
            }
            (ActionDescriptor::CastMagic { .. }, ActionDescriptor::BeginChainMagic { .. })
            | (ActionDescriptor::PlaySite { .. }, ActionDescriptor::BeginChainMagic { .. })
            | (ActionDescriptor::PlaySite { .. }, ActionDescriptor::CastMagic { .. })
            | (ActionDescriptor::CastMagic { .. }, ActionDescriptor::SummonMinion { .. })
            | (ActionDescriptor::BeginChainMagic { .. }, ActionDescriptor::SummonMinion { .. })
            | (ActionDescriptor::PlaySite { .. }, ActionDescriptor::SummonMinion { .. }) => {
                compare_card_prefix(left, right).then(Ordering::Greater)
            }
            (
                ActionDescriptor::ShootProjectile { .. }
                | ActionDescriptor::ShootDamageProjectile { .. }
                | ActionDescriptor::ShootDragProjectile { .. },
                ActionDescriptor::ShootProjectile { .. }
                | ActionDescriptor::ShootDamageProjectile { .. }
                | ActionDescriptor::ShootDragProjectile { .. },
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
                        ActionDescriptor::ActivateSiteDestruction {
                            source_site_instance_id: left_source,
                            target_cell: left_cell,
                            target_site_instance_id: left_target,
                        },
                        ActionDescriptor::ActivateSiteDestruction {
                            source_site_instance_id: right_source,
                            target_cell: right_cell,
                            target_site_instance_id: right_target,
                        },
                    ) => left_source
                        .cmp(right_source)
                        .then_with(|| left_cell.cmp(right_cell))
                        .then_with(|| left_target.cmp(right_target)),
                    (
                        ActionDescriptor::ActivateDiscardRandomDamage {
                            discard_card_instance_id: left_discard,
                            source_instance_id: left_source,
                        },
                        ActionDescriptor::ActivateDiscardRandomDamage {
                            discard_card_instance_id: right_discard,
                            source_instance_id: right_source,
                        },
                    ) => left_discard
                        .cmp(right_discard)
                        .then_with(|| left_source.cmp(right_source)),
                    (
                        ActionDescriptor::ActivateSparkmage {
                            source_instance_id: left_source,
                            target_location: left_location,
                        },
                        ActionDescriptor::ActivateSparkmage {
                            source_instance_id: right_source,
                            target_location: right_location,
                        },
                    ) => left_source
                        .cmp(right_source)
                        .then_with(|| left_location.cmp(right_location)),
                    (
                        ActionDescriptor::CloseDefend {
                            original_target_participates: left,
                        },
                        ActionDescriptor::CloseDefend {
                            original_target_participates: right,
                        },
                    ) => left.cmp(right),
                    (
                        ActionDescriptor::ExtendChainMagic { target: left },
                        ActionDescriptor::ExtendChainMagic { target: right },
                    ) => compare_unit_targets(left, right),
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
                    )
                    | (
                        ActionDescriptor::ContinueBasicMovement {
                            unit_instance_id: left,
                        },
                        ActionDescriptor::ContinueBasicMovement {
                            unit_instance_id: right,
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
                    (
                        ActionDescriptor::ResolveRangedStep {
                            choice: left_choice,
                            path: left_path,
                            unit_instance_id: left_unit,
                            ..
                        },
                        ActionDescriptor::ResolveRangedStep {
                            choice: right_choice,
                            path: right_path,
                            unit_instance_id: right_unit,
                            ..
                        },
                    ) => left_choice
                        .cmp(right_choice)
                        .then_with(|| match (left_path, right_path) {
                            (Some(left), Some(right)) => {
                                compare_json_array(left, right, Location::cmp)
                            }
                            (Some(_), None) => Ordering::Less,
                            (None, Some(_)) => Ordering::Greater,
                            (None, None) => Ordering::Equal,
                        })
                        .then_with(|| left_unit.cmp(right_unit)),
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

fn compare_optional_summon_payments(
    left: Option<SummonPaymentMode>,
    right: Option<SummonPaymentMode>,
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
    let left = projectile_fields(left);
    let right = projectile_fields(right);
    left.direction
        .cmp(&right.direction)
        .then_with(|| compare_optional_flags(left.fight_on_arrival, right.fight_on_arrival))
        .then_with(|| compare_nullable_unit_targets(left.hit, right.hit))
        .then_with(|| compare_json_strings(left.kind, right.kind))
        .then_with(|| compare_json_array(left.path, right.path, Location::cmp))
        .then_with(|| left.shooter_instance_id.cmp(right.shooter_instance_id))
}

/// Orders a present `fightOnArrival` flag ahead of an absent one, matching canonical JSON where the
/// drag projectile's extra key precedes the `hit` key every projectile descriptor shares.
fn compare_optional_flags(left: Option<bool>, right: Option<bool>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

struct ProjectileFields<'a> {
    direction: ProjectileDirection,
    fight_on_arrival: Option<bool>,
    hit: Option<&'a UnitTarget>,
    kind: &'static str,
    path: &'a [Location],
    shooter_instance_id: &'a IdentityHash,
}

fn projectile_fields(action: &ActionDescriptor) -> ProjectileFields<'_> {
    match action {
        ActionDescriptor::ShootDamageProjectile {
            direction,
            hit,
            path,
            shooter_instance_id,
        } => ProjectileFields {
            direction: *direction,
            fight_on_arrival: None,
            hit: hit.as_ref(),
            kind: "shoot-damage-projectile",
            path,
            shooter_instance_id,
        },
        ActionDescriptor::ShootDragProjectile {
            direction,
            fight_on_arrival,
            hit,
            path,
            shooter_instance_id,
        } => ProjectileFields {
            direction: *direction,
            fight_on_arrival: Some(*fight_on_arrival),
            hit: hit.as_ref(),
            kind: "shoot-drag-projectile",
            path,
            shooter_instance_id,
        },
        ActionDescriptor::ShootProjectile {
            direction,
            hit,
            path,
            shooter_instance_id,
        } => ProjectileFields {
            direction: *direction,
            fight_on_arrival: None,
            hit: hit.as_ref(),
            kind: "shoot-projectile",
            path,
            shooter_instance_id,
        },
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

fn compare_optional_identity_arrays(
    left: Option<&[IdentityHash]>,
    right: Option<&[IdentityHash]>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_json_array(left, right, IdentityHash::cmp),
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

fn compare_optional_deck_zones(left: Option<DeckZone>, right: Option<DeckZone>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => deck_zone_order(left).cmp(&deck_zone_order(right)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_locations(left: Option<Location>, right: Option<Location>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_square_areas(left: Option<SquareArea>, right: Option<SquareArea>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_json_array(&left, &right, Cell::cmp),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

const fn descriptor_group(action: &ActionDescriptor) -> u8 {
    match action {
        ActionDescriptor::CastMagic { ally: Some(_), .. } => 0,
        ActionDescriptor::ActivateMana { .. } | ActionDescriptor::AllocateStrike { .. } => 1,
        ActionDescriptor::Mulligan { .. } => 2,
        ActionDescriptor::BeginChainMagic { .. }
        | ActionDescriptor::CastMagic { .. }
        | ActionDescriptor::PlaySite { .. }
        | ActionDescriptor::SummonMinion { .. } => 3,
        ActionDescriptor::ShootDamageProjectile { .. }
        | ActionDescriptor::ShootDragProjectile { .. }
        | ActionDescriptor::ShootProjectile { .. } => 4,
        ActionDescriptor::ActivateDiscardRandomDamage { .. } => 5,
        ActionDescriptor::Defend { .. } | ActionDescriptor::MoveAndAttack { .. } => 6,
        _ => 7,
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
        | ActionDescriptor::BeginChainMagic {
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
        ActionDescriptor::ActivateDiscardRandomDamage { .. } => 0,
        ActionDescriptor::ActivateMana { .. } => 1,
        ActionDescriptor::ActivateSiteDestruction { .. } => 2,
        ActionDescriptor::ActivateSparkmage { .. } => 3,
        ActionDescriptor::AllocateStrike { .. } => 4,
        ActionDescriptor::BeginChainMagic { .. } => 5,
        ActionDescriptor::CastMagic { .. } => 6,
        ActionDescriptor::CloseDefend { .. } => 7,
        ActionDescriptor::CloseIntercept {} => 8,
        ActionDescriptor::ContinueBasicMovement { .. } => 9,
        ActionDescriptor::DeclareAttack { .. } => 10,
        ActionDescriptor::DeclineAttack => 11,
        ActionDescriptor::Defend { .. } => 12,
        ActionDescriptor::Draw { .. } => 13,
        ActionDescriptor::DrawSite => 14,
        ActionDescriptor::DrawSpell => 15,
        ActionDescriptor::EndTurn => 16,
        ActionDescriptor::ExtendChainMagic { .. } => 17,
        ActionDescriptor::Intercept { .. } => 18,
        ActionDescriptor::OrderDeathrites { .. } => 19,
        ActionDescriptor::ReplaceRubbleWithTopAtlasSite { .. } => 20,
        ActionDescriptor::ResolveChainMagic => 21,
        ActionDescriptor::ResolveGenesisSpell { .. } => 22,
        ActionDescriptor::ResolveGenesisSpellOrder { .. } => 23,
        ActionDescriptor::ResolveGenesisToken { .. } => 24,
        ActionDescriptor::ResolveRangedStep { .. } => 25,
        ActionDescriptor::Mulligan { .. } => 26,
        ActionDescriptor::PlaySite { .. } => 27,
        ActionDescriptor::ShootDamageProjectile { .. } => 28,
        ActionDescriptor::ShootDragProjectile { .. } => 29,
        ActionDescriptor::ShootProjectile { .. } => 30,
        ActionDescriptor::SummonMinion { .. } | ActionDescriptor::MoveAndAttack { .. } => 31,
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
    left.cmp(right)
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

const fn direction_name(direction: ProjectileDirection) -> &'static str {
    match direction {
        ProjectileDirection::East => "east",
        ProjectileDirection::North => "north",
        ProjectileDirection::South => "south",
        ProjectileDirection::West => "west",
    }
}

fn projectile_target_label(hit: Option<&UnitTarget>) -> String {
    hit.map_or_else(
        || "nothing".to_owned(),
        |target| {
            format!(
                "{} {}…",
                target.kind(),
                short_identity(target.instance_id())
            )
        },
    )
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
    use crate::canonical::canonical_json;

    const COMBAT_RESPONSE_FIXTURE: &str =
        include_str!("../../../tests/engine/fixtures/combat-response-action-v1.json");
    const SITE_DESTRUCTION_FIXTURE: &str =
        include_str!("../../../tests/engine/fixtures/site-destruction-action-v1.json");
    const RANDOM_CARD_DISCARD_SUMMON_FIXTURE: &str =
        include_str!("../../../tests/engine/fixtures/random-card-discard-summon-action-v1.json");

    #[test]
    fn native_site_destruction_order_should_match_typescript() {
        let fixture: Value =
            serde_json::from_str(SITE_DESTRUCTION_FIXTURE).expect("valid parity fixture");
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
    fn native_random_card_discard_summon_order_should_match_typescript() {
        let fixture: Value =
            serde_json::from_str(RANDOM_CARD_DISCARD_SUMMON_FIXTURE).expect("valid parity fixture");
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
    fn cave_in_cast_magic_boundary_should_match_canonical_typescript_shape() {
        const CARD_INSTANCE_ID: &str =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        const CASTER_INSTANCE_ID: &str =
            "sha256:2222222222222222222222222222222222222222222222222222222222222222";
        const FIRST_SITE_ID: &str =
            "sha256:3333333333333333333333333333333333333333333333333333333333333333";
        const SECOND_SITE_ID: &str =
            "sha256:4444444444444444444444444444444444444444444444444444444444444444";
        let descriptor = |cell: Option<&str>, site_id: &str| {
            let mut value = json!({
                "cardId": "cave-in",
                "cardInstanceId": CARD_INSTANCE_ID,
                "casterInstanceId": CASTER_INSTANCE_ID,
                "kind": "cast-magic",
            });
            if let Some(cell) = cell {
                value["targetLocation"] = json!({ "cell": cell, "region": "surface" });
                value["targetSiteInstanceId"] = json!(site_id);
            }
            serde_json::from_value::<ActionDescriptor>(value).expect("typed Cast Magic descriptor")
        };
        let cave_in = descriptor(Some("A1"), FIRST_SITE_ID);

        assert_eq!(
            serde_json::to_value(&cave_in).expect("serialized Cave-In descriptor"),
            json!({
                "cardId": "cave-in",
                "cardInstanceId": CARD_INSTANCE_ID,
                "casterInstanceId": CASTER_INSTANCE_ID,
                "kind": "cast-magic",
                "targetLocation": { "cell": "A1", "region": "surface" },
                "targetSiteInstanceId": FIRST_SITE_ID,
            })
        );
        assert_eq!(
            cave_in.state_independent_label().as_deref(),
            Some("Cast cave-in at A1 surface")
        );

        let mut descriptors = [
            cave_in,
            descriptor(Some("A1"), SECOND_SITE_ID),
            descriptor(Some("B1"), FIRST_SITE_ID),
            descriptor(None, FIRST_SITE_ID),
        ];
        let mut canonical = descriptors.clone();
        descriptors.sort_unstable_by(compare_canonical);
        canonical.sort_unstable_by_key(|candidate| {
            canonical_json(&serde_json::to_value(candidate).expect("serialized ordering candidate"))
                .expect("canonical ordering candidate")
        });

        assert_eq!(descriptors, canonical);
    }

    #[test]
    fn charge_magic_order_should_match_canonical_typescript_shape() {
        const CARD_ID: &str =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        const CASTER_ID: &str =
            "sha256:2222222222222222222222222222222222222222222222222222222222222222";
        const ALLY_ID: &str =
            "sha256:3333333333333333333333333333333333333333333333333333333333333333";
        let descriptor =
            |value| serde_json::from_value::<ActionDescriptor>(value).expect("typed descriptor");
        let charge = descriptor(json!({
            "ally": { "instanceId": ALLY_ID, "kind": "minion", "seat": "north" },
            "cardId": "charge",
            "cardInstanceId": CARD_ID,
            "casterInstanceId": CASTER_ID,
            "kind": "cast-magic",
        }));
        assert_eq!(
            charge.state_independent_label().as_deref(),
            Some("Cast charge to grant Charge to minion sha256:33333333…")
        );
        let mut descriptors = [
            descriptor(json!({
                "amount": 1,
                "kind": "activate-mana",
                "unitInstanceId": CASTER_ID,
            })),
            descriptor(json!({
                "atlasOrder": [],
                "kind": "mulligan",
                "spellbookOrder": [],
            })),
            descriptor(json!({
                "cardId": "ordinary",
                "cardInstanceId": CARD_ID,
                "casterInstanceId": CASTER_ID,
                "kind": "cast-magic",
            })),
            charge,
        ];
        let mut canonical = descriptors.clone();
        descriptors.sort_unstable_by(compare_canonical);
        canonical.sort_unstable_by_key(|candidate| {
            canonical_json(&serde_json::to_value(candidate).expect("serialized descriptor"))
                .expect("canonical descriptor")
        });

        assert_eq!(descriptors, canonical);
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
    fn native_integer_order_should_match_typescript_locale_prefixes() {
        let values = [0, 1, 2, 9, 10, 11, 20, u64::MAX];
        for left in values {
            for right in values {
                let expected = left.to_string().cmp(&right.to_string());
                assert_eq!(
                    compare_json_integers(left, right),
                    expected,
                    "{left} compared with {right}"
                );
            }
        }
    }
}
