//! Strict, self-identifying snapshots for deterministic selector policies.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::action::{ActionDescriptor, DeckZone, GenesisTokenChoice};
use crate::board::Region;
use crate::canonical::{
    CanonicalError, IdentityHash, canonical_json, identity_hash, parse_json_without_duplicate_keys,
};
use crate::contract::Seat;
use crate::game::{IssuedAction, SeatObservation};

/// Policy snapshot schema understood by this module.
pub const POLICY_SCHEMA_VERSION: u8 = 1;
/// Largest accepted lineage generation.
pub const MAX_POLICY_GENERATION: u32 = 1_000_000;
/// Largest accepted Atlas reserve used by the current selector.
pub const MAX_ATLAS_RESERVE: u8 = 8;
/// Maximum accepted canonical policy snapshot byte length.
pub const POLICY_SNAPSHOT_MAX_BYTES: usize = 64 * 1024;

const MAX_ENGINE_VERSION_BYTES: usize = 64;

/// The observation contract consumed by the deterministic selector.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ObservationVersion {
    /// Public information visible to one acting seat.
    #[serde(rename = "seat-observation-v1")]
    SeatObservationV1,
}

impl ObservationVersion {
    /// Returns the canonical contract identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SeatObservationV1 => "seat-observation-v1",
        }
    }
}

/// The deterministic tie-breaking contract applied after feature priority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TieBreak {
    /// Preserve the engine's canonical legal-action order.
    #[serde(rename = "canonical-action-order-v1")]
    CanonicalActionOrderV1,
}

impl TieBreak {
    /// Returns the canonical contract identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalActionOrderV1 => "canonical-action-order-v1",
        }
    }
}

/// A generic, card-independent action-selection feature.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PolicyFeature {
    /// Keep an acceptable opening hand instead of taking another mulligan.
    #[serde(rename = "keep-mulligan")]
    KeepMulligan,
    /// Prefer a legal site play.
    #[serde(rename = "play-site")]
    PlaySite,
    /// Prefer a legal minion summon.
    #[serde(rename = "summon-minion")]
    SummonMinion,
    /// Prefer the configured strategic draw choice.
    #[serde(rename = "preferred-draw")]
    PreferredDraw,
    /// Prefer movement that uses a unit's available power effectively.
    #[serde(rename = "powered-movement")]
    PoweredMovement,
    /// Prefer a currently beneficial legal tactic.
    #[serde(rename = "beneficial-tactic")]
    BeneficialTactic,
    /// Prefer legal movement toward an opposing unit or site.
    #[serde(rename = "move-toward-enemy")]
    MoveTowardEnemy,
    /// End the turn when higher-priority features do not select an action.
    #[serde(rename = "end-turn")]
    EndTurn,
    /// Select the first remaining action in canonical engine order.
    #[serde(rename = "canonical-fallback")]
    CanonicalFallback,
}

impl PolicyFeature {
    /// Every supported feature, in the baseline selector order.
    pub const ALL: [Self; 9] = [
        Self::KeepMulligan,
        Self::PlaySite,
        Self::SummonMinion,
        Self::PreferredDraw,
        Self::PoweredMovement,
        Self::BeneficialTactic,
        Self::MoveTowardEnemy,
        Self::EndTurn,
        Self::CanonicalFallback,
    ];

    const fn index(self) -> usize {
        match self {
            Self::KeepMulligan => 0,
            Self::PlaySite => 1,
            Self::SummonMinion => 2,
            Self::PreferredDraw => 3,
            Self::PoweredMovement => 4,
            Self::BeneficialTactic => 5,
            Self::MoveTowardEnemy => 6,
            Self::EndTurn => 7,
            Self::CanonicalFallback => 8,
        }
    }
}

/// Validated configuration for the current deterministic selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicySelector {
    atlas_reserve: u8,
    feature_priority: [PolicyFeature; 9],
}

impl PolicySelector {
    /// Returns the number of Atlas cards the selector attempts to retain.
    #[must_use]
    pub const fn atlas_reserve(&self) -> u8 {
        self.atlas_reserve
    }

    /// Returns all generic features in deterministic priority order.
    #[must_use]
    pub const fn feature_priority(&self) -> &[PolicyFeature; 9] {
        &self.feature_priority
    }
}

/// A validated, canonical deterministic policy snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicySnapshot {
    authority_hash: IdentityHash,
    deck_id: IdentityHash,
    engine_version: String,
    generation: u32,
    observation_version: ObservationVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_policy_id: Option<IdentityHash>,
    policy_id: IdentityHash,
    schema_version: u8,
    selector: PolicySelector,
    tie_break: TieBreak,
}

impl PolicySnapshot {
    /// Returns the authority revision this policy was evaluated against.
    #[must_use]
    pub const fn authority_hash(&self) -> &IdentityHash {
        &self.authority_hash
    }

    /// Returns the immutable deck identity assigned to this policy.
    #[must_use]
    pub const fn deck_id(&self) -> &IdentityHash {
        &self.deck_id
    }

    /// Returns the engine version required by this policy.
    #[must_use]
    pub fn engine_version(&self) -> &str {
        &self.engine_version
    }

    /// Returns the bounded lineage generation.
    #[must_use]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Returns the observation contract.
    #[must_use]
    pub const fn observation_version(&self) -> ObservationVersion {
        self.observation_version
    }

    /// Returns the optional parent policy identity.
    #[must_use]
    pub const fn parent_policy_id(&self) -> Option<&IdentityHash> {
        self.parent_policy_id.as_ref()
    }

    /// Returns the self-hash over every field except `policyId`.
    #[must_use]
    pub const fn policy_id(&self) -> &IdentityHash {
        &self.policy_id
    }

    /// Returns the policy schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u8 {
        self.schema_version
    }

    /// Returns the validated selector configuration.
    #[must_use]
    pub const fn selector(&self) -> &PolicySelector {
        &self.selector
    }

    /// Returns the tie-breaking contract.
    #[must_use]
    pub const fn tie_break(&self) -> TieBreak {
        self.tie_break
    }

    /// Verifies the immutable authority, deck, and engine binding for a match.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError`] when the policy was evaluated for a different match contract.
    pub fn validate_binding(
        &self,
        authority_hash: &IdentityHash,
        deck_id: &IdentityHash,
        engine_version: &str,
    ) -> Result<(), PolicyError> {
        if self.authority_hash != *authority_hash {
            return Err(PolicyError::Invalid(
                "policy authority binding does not match",
            ));
        }
        if self.deck_id != *deck_id {
            return Err(PolicyError::Invalid("policy deck binding does not match"));
        }
        if self.engine_version != engine_version {
            return Err(PolicyError::Invalid("policy engine binding does not match"));
        }
        Ok(())
    }

    /// Selects one engine-issued action from a seat-scoped observation.
    ///
    /// Ties preserve the engine's canonical legal-action ordering.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError`] for an empty or wrong-seat action set.
    pub fn select_action<'a>(
        &self,
        observation: SeatObservation,
        legal_actions: &'a [IssuedAction],
    ) -> Result<&'a IssuedAction, PolicyError> {
        if legal_actions.is_empty() {
            return Err(PolicyError::Invalid(
                "policy selector requires at least one legal action",
            ));
        }
        if legal_actions
            .iter()
            .any(|action| action.seat() != observation.seat())
        {
            return Err(PolicyError::Invalid(
                "policy observation seat does not match legal actions",
            ));
        }
        for feature in self.selector.feature_priority {
            if let Some(action) = select_feature(
                feature,
                self.selector.atlas_reserve,
                observation,
                legal_actions,
            ) {
                return Ok(action);
            }
        }
        Err(PolicyError::Invalid(
            "policy feature contract omitted canonical fallback",
        ))
    }

    /// Builds the complete deterministic one-step policy neighborhood.
    ///
    /// Children swap one adjacent feature pair or adjust the Atlas reserve by one.
    /// Every child is immutable, self-hashed, and points to this snapshot as its parent.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError`] when this snapshot cannot produce another generation.
    pub fn neighbors(&self) -> Result<Vec<Self>, PolicyError> {
        if self.generation == MAX_POLICY_GENERATION {
            return Err(PolicyError::Invalid(
                "policy generation exceeds the supported bound",
            ));
        }
        let mut children = Vec::with_capacity(10);
        for index in 0..self.selector.feature_priority.len() - 1 {
            let mut selector = self.selector.clone();
            selector.feature_priority.swap(index, index + 1);
            children.push(self.child(selector)?);
        }
        if self.selector.atlas_reserve > 0 {
            let mut selector = self.selector.clone();
            selector.atlas_reserve -= 1;
            children.push(self.child(selector)?);
        }
        if self.selector.atlas_reserve < MAX_ATLAS_RESERVE {
            let mut selector = self.selector.clone();
            selector.atlas_reserve += 1;
            children.push(self.child(selector)?);
        }
        children.sort_unstable_by(|left, right| left.policy_id.cmp(&right.policy_id));
        Ok(children)
    }

    fn child(&self, selector: PolicySelector) -> Result<Self, PolicyError> {
        let mut child = Self {
            authority_hash: self.authority_hash.clone(),
            deck_id: self.deck_id.clone(),
            engine_version: self.engine_version.clone(),
            generation: self.generation + 1,
            observation_version: self.observation_version,
            parent_policy_id: Some(self.policy_id.clone()),
            policy_id: self.policy_id.clone(),
            schema_version: self.schema_version,
            selector,
            tie_break: self.tie_break,
        };
        child.policy_id = identity_hash(&body_value(&child)?)?;
        validate(&child)?;
        Ok(child)
    }
}

fn select_feature(
    feature: PolicyFeature,
    atlas_reserve: u8,
    observation: SeatObservation,
    actions: &[IssuedAction],
) -> Option<&IssuedAction> {
    match feature {
        PolicyFeature::KeepMulligan => actions.iter().find(|action| {
            matches!(
                action.descriptor(),
                ActionDescriptor::Mulligan {
                    atlas_order,
                    spellbook_order,
                } if atlas_order.is_empty() && spellbook_order.is_empty()
            )
        }),
        PolicyFeature::PlaySite => actions
            .iter()
            .find(|action| matches!(action.descriptor(), ActionDescriptor::PlaySite { .. })),
        PolicyFeature::SummonMinion => actions
            .iter()
            .find(|action| matches!(action.descriptor(), ActionDescriptor::SummonMinion { .. })),
        PolicyFeature::PreferredDraw => {
            let zone = if observation.atlas_remaining() > usize::from(atlas_reserve)
                || observation.spellbook_remaining() <= observation.atlas_remaining()
            {
                DeckZone::Atlas
            } else {
                DeckZone::Spellbook
            };
            actions.iter().find(|action| {
                matches!(action.descriptor(), ActionDescriptor::Draw { zone: candidate } if *candidate == zone)
            })
        }
        PolicyFeature::PoweredMovement => None,
        PolicyFeature::BeneficialTactic => beneficial_tactic_index(
            observation.seat(),
            actions.iter().map(IssuedAction::descriptor),
        )
        .and_then(|index| actions.get(index)),
        PolicyFeature::MoveTowardEnemy => select_movement(observation, actions),
        PolicyFeature::EndTurn => actions
            .iter()
            .find(|action| matches!(action.descriptor(), ActionDescriptor::EndTurn)),
        PolicyFeature::CanonicalFallback => actions.first(),
    }
}

fn beneficial_tactic_index<'a>(
    seat: Seat,
    descriptors: impl Iterator<Item = &'a ActionDescriptor>,
) -> Option<usize> {
    descriptors
        .enumerate()
        .filter_map(|(index, descriptor)| {
            beneficial_tactic_rank(seat, descriptor).map(|rank| (rank, index))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, index)| index)
}

fn beneficial_tactic_rank(seat: Seat, descriptor: &ActionDescriptor) -> Option<u8> {
    match descriptor {
        ActionDescriptor::ShootProjectile {
            hit: Some(target), ..
        }
        | ActionDescriptor::ShootDamageProjectile {
            hit: Some(target), ..
        } if target.seat() != seat => Some(0),
        ActionDescriptor::CastMagic {
            cemetery_minion_instance_id: Some(_),
            ..
        } => Some(1),
        ActionDescriptor::ReplaceRubbleWithTopAtlasSite { .. } => Some(2),
        ActionDescriptor::ResolveGenesisToken {
            choice: GenesisTokenChoice::PayOneMana,
        } => Some(3),
        _ => None,
    }
}

fn select_movement(
    observation: SeatObservation,
    actions: &[IssuedAction],
) -> Option<&IssuedAction> {
    let enemy = observation.enemy_avatar();
    actions
        .iter()
        .filter_map(|action| {
            let ActionDescriptor::MoveAndAttack { path, to, .. } = action.descriptor() else {
                return None;
            };
            if to.region != Region::Surface {
                return None;
            }
            let priority = if path.len() == 1 && *to == enemy {
                0
            } else if path.len() > 1 {
                u16::from(to.cell.manhattan_distance(enemy.cell)) + 1
            } else {
                return None;
            };
            Some((priority, action))
        })
        .min_by_key(|(priority, _)| *priority)
        .map(|(_, action)| action)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawPolicySelector {
    atlas_reserve: u64,
    feature_priority: Vec<PolicyFeature>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawPolicySnapshot {
    authority_hash: IdentityHash,
    deck_id: IdentityHash,
    engine_version: String,
    generation: u64,
    observation_version: ObservationVersion,
    parent_policy_id: Option<IdentityHash>,
    policy_id: IdentityHash,
    schema_version: u64,
    selector: RawPolicySelector,
    tie_break: TieBreak,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PolicyBody<'a> {
    authority_hash: &'a IdentityHash,
    deck_id: &'a IdentityHash,
    engine_version: &'a str,
    generation: u32,
    observation_version: ObservationVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_policy_id: Option<&'a IdentityHash>,
    schema_version: u8,
    selector: &'a PolicySelector,
    tie_break: TieBreak,
}

/// Policy parsing, validation, serialization, or hashing failed.
#[derive(Debug)]
pub enum PolicyError {
    /// Canonical serialization or hashing failed.
    Canonical(CanonicalError),
    /// JSON decoding failed.
    Json(serde_json::Error),
    /// A policy contract invariant was violated.
    Invalid(&'static str),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl Error for PolicyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<CanonicalError> for PolicyError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<serde_json::Error> for PolicyError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

fn validate_engine_version(value: &str) -> Result<(), PolicyError> {
    if value.is_empty()
        || value.len() > MAX_ENGINE_VERSION_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(PolicyError::Invalid(
            "engineVersion must be a 1-64 byte ASCII version identifier",
        ));
    }
    Ok(())
}

fn validate_feature_priority(
    features: Vec<PolicyFeature>,
) -> Result<[PolicyFeature; 9], PolicyError> {
    let features: [PolicyFeature; 9] = features.try_into().map_err(|_| {
        PolicyError::Invalid("featurePriority must contain all nine features exactly once")
    })?;
    let mut seen = [false; 9];
    for feature in features {
        let index = feature.index();
        if seen[index] {
            return Err(PolicyError::Invalid(
                "featurePriority must contain all nine features exactly once",
            ));
        }
        seen[index] = true;
    }
    Ok(features)
}

fn body_value(snapshot: &PolicySnapshot) -> Result<Value, PolicyError> {
    let body = PolicyBody {
        authority_hash: &snapshot.authority_hash,
        deck_id: &snapshot.deck_id,
        engine_version: &snapshot.engine_version,
        generation: snapshot.generation,
        observation_version: snapshot.observation_version,
        parent_policy_id: snapshot.parent_policy_id.as_ref(),
        schema_version: snapshot.schema_version,
        selector: &snapshot.selector,
        tie_break: snapshot.tie_break,
    };
    Ok(serde_json::to_value(body)?)
}

fn validate(snapshot: &PolicySnapshot) -> Result<(), PolicyError> {
    if snapshot.schema_version != POLICY_SCHEMA_VERSION {
        return Err(PolicyError::Invalid("policy schemaVersion is unsupported"));
    }
    if snapshot.generation > MAX_POLICY_GENERATION {
        return Err(PolicyError::Invalid(
            "policy generation exceeds the supported bound",
        ));
    }
    validate_engine_version(&snapshot.engine_version)?;
    if snapshot.selector.atlas_reserve > MAX_ATLAS_RESERVE {
        return Err(PolicyError::Invalid(
            "selector atlasReserve exceeds the supported bound",
        ));
    }
    if identity_hash(&body_value(snapshot)?)? != snapshot.policy_id {
        return Err(PolicyError::Invalid("policyId does not match policy body"));
    }
    Ok(())
}

/// Serializes a validated policy snapshot as its one canonical JSON representation.
///
/// # Errors
///
/// Returns [`PolicyError`] when the snapshot identity or another invariant is invalid.
pub fn serialize_policy_snapshot(snapshot: &PolicySnapshot) -> Result<String, PolicyError> {
    validate(snapshot)?;
    Ok(canonical_json(&serde_json::to_value(snapshot)?)?)
}

/// Parses a strict canonical policy snapshot and verifies its self-hash.
///
/// Duplicate keys, insignificant whitespace, unknown fields, extensions, invalid identities,
/// and incomplete feature lists are rejected.
///
/// # Errors
///
/// Returns [`PolicyError`] when the JSON is not the unique valid representation of a policy.
pub fn parse_policy_snapshot(text: &str) -> Result<PolicySnapshot, PolicyError> {
    if text.len() > POLICY_SNAPSHOT_MAX_BYTES {
        return Err(PolicyError::Invalid(
            "policy snapshot exceeds the supported byte bound",
        ));
    }
    let value = parse_json_without_duplicate_keys(text)?;
    if canonical_json(&value)? != text {
        return Err(PolicyError::Invalid(
            "policy snapshot must use canonical JSON",
        ));
    }
    let raw: RawPolicySnapshot = serde_json::from_value(value)?;
    let schema_version = u8::try_from(raw.schema_version)
        .map_err(|_| PolicyError::Invalid("policy schemaVersion is unsupported"))?;
    let generation = u32::try_from(raw.generation)
        .map_err(|_| PolicyError::Invalid("policy generation exceeds the supported bound"))?;
    let atlas_reserve = u8::try_from(raw.selector.atlas_reserve)
        .map_err(|_| PolicyError::Invalid("selector atlasReserve exceeds the supported bound"))?;
    let snapshot = PolicySnapshot {
        authority_hash: raw.authority_hash,
        deck_id: raw.deck_id,
        engine_version: raw.engine_version,
        generation,
        observation_version: raw.observation_version,
        parent_policy_id: raw.parent_policy_id,
        policy_id: raw.policy_id,
        schema_version,
        selector: PolicySelector {
            atlas_reserve,
            feature_priority: validate_feature_priority(raw.selector.feature_priority)?,
        },
        tie_break: raw.tie_break,
    };
    validate(&snapshot)?;
    if serialize_policy_snapshot(&snapshot)? != text {
        return Err(PolicyError::Invalid(
            "policy snapshot must use canonical optional fields",
        ));
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use crate::action::{ActionDescriptor, GenesisTokenChoice, ProjectileDirection, UnitTarget};
    use crate::board::Cell;
    use crate::canonical::IdentityHash;
    use crate::contract::Seat;

    use super::beneficial_tactic_index;

    fn identity(hex: char) -> IdentityHash {
        IdentityHash::parse(&format!("sha256:{}", hex.to_string().repeat(64)))
            .expect("test identity")
    }

    fn target(seat: Seat) -> UnitTarget {
        UnitTarget::Minion {
            instance_id: identity('a'),
            seat,
        }
    }

    fn projectile(fixed: bool, hit: Option<UnitTarget>) -> ActionDescriptor {
        if fixed {
            ActionDescriptor::ShootDamageProjectile {
                direction: ProjectileDirection::North,
                hit,
                path: Vec::new(),
                shooter_instance_id: identity('b'),
            }
        } else {
            ActionDescriptor::ShootProjectile {
                direction: ProjectileDirection::North,
                hit,
                path: Vec::new(),
                shooter_instance_id: identity('b'),
            }
        }
    }

    #[test]
    fn beneficial_tactics_should_be_card_independent_safe_and_canonical() {
        let cemetery_magic = ActionDescriptor::CastMagic {
            ally: None,
            ally_destination: None,
            ally_strike_location: None,
            card_id: "synthetic-magic".to_owned(),
            card_instance_id: identity('c'),
            caster_instance_id: identity('d'),
            cemetery_minion_instance_id: Some(identity('e')),
            discard_site_instance_id: None,
            draw_zone: None,
            target: None,
            target_location: None,
            target_site_instance_id: None,
            tempted_destination: None,
            tempted_enemy: None,
        };
        let enemy_magic = ActionDescriptor::CastMagic {
            ally: None,
            ally_destination: None,
            ally_strike_location: None,
            card_id: "synthetic-unknown-effect".to_owned(),
            card_instance_id: identity('f'),
            caster_instance_id: identity('1'),
            cemetery_minion_instance_id: None,
            discard_site_instance_id: None,
            draw_zone: None,
            target: Some(target(Seat::South)),
            target_location: None,
            target_site_instance_id: None,
            tempted_destination: None,
            tempted_enemy: None,
        };
        let rubble = ActionDescriptor::ReplaceRubbleWithTopAtlasSite {
            target_cell: Cell::parse("A1").expect("cell"),
            target_rubble_instance_id: identity('2'),
        };
        let pay = ActionDescriptor::ResolveGenesisToken {
            choice: GenesisTokenChoice::PayOneMana,
        };
        let decline = ActionDescriptor::ResolveGenesisToken {
            choice: GenesisTokenChoice::Decline,
        };
        let actions = vec![
            projectile(false, None),
            projectile(true, Some(target(Seat::North))),
            enemy_magic,
            decline,
            pay,
            rubble,
            cemetery_magic,
            projectile(false, Some(target(Seat::South))),
            projectile(true, Some(target(Seat::South))),
        ];

        assert_eq!(
            beneficial_tactic_index(Seat::North, actions.iter()),
            Some(7)
        );
        assert_eq!(
            beneficial_tactic_index(Seat::North, actions[..7].iter()),
            Some(6)
        );
        assert_eq!(
            beneficial_tactic_index(Seat::North, actions[..6].iter()),
            Some(5)
        );
        assert_eq!(
            beneficial_tactic_index(Seat::North, actions[..5].iter()),
            Some(4)
        );
        assert_eq!(
            beneficial_tactic_index(Seat::North, actions[..4].iter()),
            None
        );
    }
}
