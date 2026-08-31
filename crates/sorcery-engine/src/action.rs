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
        /// Issued branch for a site with optional paid-token Genesis.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        genesis_token_choice: Option<GenesisTokenChoice>,
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
        /// Mana paid for the summon.
        mana_cost: u64,
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
    /// Decline to attack after moving or tapping in place.
    DeclineAttack,
    /// Attack one engine-issued target.
    DeclareAttack {
        /// Chosen combat target.
        target: CombatTarget,
    },
    /// Close the defend window.
    CloseDefend {
        /// Whether the original attack target remains a combatant.
        original_target_participates: bool,
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
            Self::DeclineAttack => Some("Decline attack".to_owned()),
            Self::DeclareAttack { target } => Some(format!(
                "Attack {} {}…",
                target.kind(),
                short_identity(target.instance_id())
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
            Self::EndTurn => Some("End turn".to_owned()),
            Self::PlaySite { .. } | Self::SummonMinion { .. } => None,
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
                    genesis_token_choice: left_choice,
                },
                ActionDescriptor::PlaySite {
                    card_id: right_card,
                    card_instance_id: right_instance,
                    cell: right_cell,
                    genesis_token_choice: right_choice,
                },
            ) => compare_json_strings(left_card, right_card)
                .then_with(|| left_instance.cmp(right_instance))
                .then_with(|| left_cell.cmp(right_cell))
                .then_with(|| compare_optional_genesis_choices(*left_choice, *right_choice)),
            (
                ActionDescriptor::SummonMinion {
                    card_id: left_card,
                    card_instance_id: left_instance,
                    caster_instance_id: left_caster,
                    cell: left_cell,
                    mana_cost: left_mana,
                },
                ActionDescriptor::SummonMinion {
                    card_id: right_card,
                    card_instance_id: right_instance,
                    caster_instance_id: right_caster,
                    cell: right_cell,
                    mana_cost: right_mana,
                },
            ) => compare_json_strings(left_card, right_card)
                .then_with(|| left_instance.cmp(right_instance))
                .then_with(|| left_caster.cmp(right_caster))
                .then_with(|| left_cell.cmp(right_cell))
                .then_with(|| compare_json_integers(*left_mana, *right_mana)),
            (ActionDescriptor::SummonMinion { .. }, ActionDescriptor::PlaySite { .. }) => {
                compare_card_prefix(left, right).then(Ordering::Less)
            }
            (ActionDescriptor::PlaySite { .. }, ActionDescriptor::SummonMinion { .. }) => {
                compare_card_prefix(left, right).then(Ordering::Greater)
            }
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
                        ActionDescriptor::DeclareAttack { target: left },
                        ActionDescriptor::DeclareAttack { target: right },
                    ) => compare_targets(left, right),
                    (
                        ActionDescriptor::Draw { zone: left },
                        ActionDescriptor::Draw { zone: right },
                    ) => deck_zone_order(*left).cmp(&deck_zone_order(*right)),
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

const fn descriptor_group(action: &ActionDescriptor) -> u8 {
    match action {
        ActionDescriptor::ActivateMana { .. } => 0,
        ActionDescriptor::Mulligan { .. } => 1,
        ActionDescriptor::PlaySite { .. } | ActionDescriptor::SummonMinion { .. } => 2,
        ActionDescriptor::MoveAndAttack { .. } => 3,
        _ => 4,
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
        ActionDescriptor::CloseDefend { .. } => 1,
        ActionDescriptor::DeclareAttack { .. } => 2,
        ActionDescriptor::DeclineAttack => 3,
        ActionDescriptor::Draw { .. } => 4,
        ActionDescriptor::DrawSite => 5,
        ActionDescriptor::DrawSpell => 6,
        ActionDescriptor::EndTurn => 7,
        ActionDescriptor::Mulligan { .. } => 8,
        ActionDescriptor::PlaySite { .. } => 9,
        ActionDescriptor::SummonMinion { .. } => 10,
        ActionDescriptor::MoveAndAttack { .. } => 11,
    }
}

fn compare_targets(left: &CombatTarget, right: &CombatTarget) -> Ordering {
    left.instance_id()
        .cmp(right.instance_id())
        .then_with(|| target_kind(left).cmp(&target_kind(right)))
        .then_with(|| seat_order(left.seat()).cmp(&seat_order(right.seat())))
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
    use serde_json::json;

    use super::{compare_json_integers, compare_json_strings};

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
