//! Compact game setup and opening-hand decisions.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::action::{
    ActionDescriptor, CombatTarget, DeckZone, GenesisDamageChoice, GenesisSpellChoice,
    GenesisTokenChoice, ProjectileDirection, RangedStepChoice, SummonPaymentMode, UnitTarget,
    compare_canonical,
};
use crate::board::{Cell, Location, Region, SquareArea, translated_square};
use crate::canonical::{CanonicalError, IdentityHash, identity_hash};
use crate::contract::{LegalAction, Seat, opaque_action_id};
use crate::facts::{
    AlternativeSummonPayment, ArtifactEffect, ArtifactFacts, AvatarFacts, BasicMovementRestriction,
    CardFacts, DamagePrevention, Element, EndTurnStealth, FactError, MagicEffect, MagicFacts,
    MinionFacts, MinionGenesis, RequiredCastRegion, SiteFacts, Thresholds, parse_card_definition,
    validate_identifier,
};
use crate::prng::PrngState;

const ENGINE_VERSION: &str = "sorcery-core-v1";
const MAX_DECK_CARDS: usize = 200;
const CHAIN_MAGIC_DAMAGE: u16 = 2;
const CHAIN_MAGIC_EXTRA_TARGET_MANA: u64 = 2;
/// The one damage amount `tapToDamageEachUnitAtAdjacentLocation` is admitted with.
const AREA_DAMAGE_AMOUNT: u8 = 2;
/// The one damage amount `tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps` is admitted with.
const ARTIFACT_DAMAGE_AMOUNT: u8 = 3;
const ARTIFACT_ROLL_DAMAGE_AMOUNT: u8 = 4;

/// Immutable manifest facts shared by cloned game positions.
#[derive(Debug)]
pub struct RulesContext {
    authority_hash: IdentityHash,
    cards: Box<[CardDefinition]>,
    first_seat: Seat,
    manifest_id: IdentityHash,
    seed: u32,
}

/// Cloneable dynamic game data used by speculative branches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    active_seat: Seat,
    artifacts: Vec<ArtifactPosition>,
    decision_seat: Seat,
    immobile_areas: Vec<ImmobileArea>,
    pending_basic_movement: PendingField<PendingBasicMovement>,
    pending_cemetery_summon: Option<PendingCemeterySummon>,
    pending_chain_magic: PendingField<PendingChainMagic>,
    pending_combat: Option<PendingCombat>,
    pending_deathrites: Option<PendingDeathrites>,
    pending_genesis_spell: PendingField<PendingGenesisSpell>,
    pending_genesis_spell_order: PendingField<PendingGenesisSpellOrder>,
    pending_genesis_token: PendingField<PendingGenesisToken>,
    pending_ranged_step: PendingField<PendingRangedStep>,
    phase: Phase,
    players: [PlayerPosition; 2],
    prng: PrngState,
    rubble: [Option<IdentityHash>; 20],
    sites: [Option<SitePosition>; 20],
    state_version: u64,
    terminal: Option<TerminalResult>,
    turn_number: u64,
    units: Vec<UnitPosition>,
}

/// Public information required by a deterministic policy for one acting seat.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SeatObservation {
    atlas_remaining: usize,
    enemy_avatar: Location,
    seat: Seat,
    spellbook_remaining: usize,
}

impl SeatObservation {
    /// Returns the observing seat.
    #[must_use]
    pub const fn seat(self) -> Seat {
        self.seat
    }

    /// Returns the observing player's remaining Atlas count.
    #[must_use]
    pub const fn atlas_remaining(self) -> usize {
        self.atlas_remaining
    }

    /// Returns the observing player's remaining Spellbook count.
    #[must_use]
    pub const fn spellbook_remaining(self) -> usize {
        self.spellbook_remaining
    }

    /// Returns the opposing Avatar's public surface location.
    #[must_use]
    pub const fn enemy_avatar(self) -> Location {
        self.enemy_avatar
    }
}

/// A game with immutable rules and independently cloneable dynamic state.
#[derive(Clone, Debug)]
pub struct Game {
    initial_random_draws: Arc<[EngineRandomDraw]>,
    rules: Arc<RulesContext>,
    position: Position,
}

/// One authoritative setup PRNG draw with its accepted domain and state hashes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineRandomDraw {
    domain: RandomDomain,
    draw_sequence: u64,
    post_prng_state_hash: IdentityHash,
    pre_prng_state_hash: IdentityHash,
    purpose: String,
    result: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RandomDomain {
    accepted: bool,
    exclusive_maximum: usize,
    kind: &'static str,
}

/// A typed action issued for one exact game position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuedAction {
    descriptor: ActionDescriptor,
    label: String,
    seat: Seat,
    state_version: u64,
}

/// The setup or opening-hand request was invalid.
#[derive(Debug)]
pub enum GameError {
    /// Canonical serialization or hashing failed.
    Canonical(CanonicalError),
    /// JSON could not be decoded.
    Json(serde_json::Error),
    /// A card rule fact violated the supported contract.
    Fact(FactError),
    /// The manifest violated the frozen game contract.
    InvalidManifest(String),
    /// A known rule fact has not yet been admitted by this Rust slice.
    UnsupportedManifestFact(String),
    /// The action was not issued for the current position.
    IllegalAction,
}

impl fmt::Display for GameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::Fact(error) => error.fmt(formatter),
            Self::InvalidManifest(message) => formatter.write_str(message),
            Self::UnsupportedManifestFact(field) => {
                write!(
                    formatter,
                    "manifest fact is not yet supported by Rust: {field}"
                )
            }
            Self::IllegalAction => formatter.write_str("action is not legal here"),
        }
    }
}

impl Error for GameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Fact(error) => Some(error),
            Self::InvalidManifest(_) | Self::UnsupportedManifestFact(_) | Self::IllegalAction => {
                None
            }
        }
    }
}

impl From<CanonicalError> for GameError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<serde_json::Error> for GameError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<FactError> for GameError {
    fn from(error: FactError) -> Self {
        Self::Fact(error)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    authority: Authority,
    cards: BTreeMap<String, Value>,
    decks: Decks,
    engine_version: String,
    first_seat: Seat,
    #[serde(rename = "manifestId")]
    identity: IdentityHash,
    schema_version: u8,
    seed: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Authority {
    content_hash: IdentityHash,
    mode: AuthorityMode,
    revision_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum AuthorityMode {
    PrivateLocal,
    Synthetic,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decks {
    north: Deck,
    south: Deck,
}

impl Decks {
    fn get(&self, seat: Seat) -> &Deck {
        match seat {
            Seat::North => &self.north,
            Seat::South => &self.south,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Deck {
    atlas: Vec<String>,
    avatar: String,
    spellbook: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CardId(u16);

#[derive(Debug)]
struct CardDefinition {
    definition_hash: IdentityHash,
    facts: CardFacts,
    id: String,
    value: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CardKind {
    Artifact,
    Aura,
    Avatar,
    Magic,
    Minion,
    Site,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CardSource {
    Atlas,
    Avatar,
    Spellbook,
    Token,
}

impl CardSource {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Atlas => "atlas",
            Self::Avatar => "avatar",
            Self::Spellbook => "spellbook",
            Self::Token => "token",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CardInstance {
    card_id: CardId,
    instance_id: IdentityHash,
    owner: Seat,
    source: CardSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AvatarPosition {
    card: CardInstance,
    death_door_turn: Option<u64>,
    last_dropped_artifacts_turn: Option<u64>,
    last_interacted_turn: Option<u64>,
    last_picked_up_artifacts_turn: Option<u64>,
    life: u16,
    location: Cell,
    tapped: bool,
    temporary_power_sources: Vec<IdentityHash>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlayerPosition {
    air_thresholds_cast_this_turn: Option<u16>,
    atlas: Vec<CardInstance>,
    avatar: AvatarPosition,
    cemetery: Vec<CardInstance>,
    domain_established: bool,
    hand_atlas: Vec<CardInstance>,
    hand_spellbook: Vec<CardInstance>,
    mana: u16,
    mulligan_complete: bool,
    spellbook: Vec<CardInstance>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SitePosition {
    card: CardInstance,
    controller: Seat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "named dynamic flags preserve distinct authoritative unit state"
)]
struct UnitPosition {
    card: CardInstance,
    carried_lance_count: u8,
    controller: Seat,
    damage: u16,
    disable_effects: Vec<DisableEffect>,
    disabled_until_damaged: bool,
    last_dropped_artifacts_turn: Option<u64>,
    last_interacted_turn: Option<u64>,
    last_picked_up_artifacts_turn: Option<u64>,
    location: Cell,
    occupied_cells: Option<SquareArea>,
    region: Region,
    stealthed: bool,
    summoning_sickness: bool,
    tapped: bool,
    temporary_charge_sources: Vec<IdentityHash>,
    temporary_power_sources: Vec<IdentityHash>,
    warded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DisableEffect {
    expires_at_seat: Seat,
    source_instance_id: IdentityHash,
}

/// Where a realm Artifact currently sits: carried by one unit, or loose in the realm.
///
/// Oversized bearers would need the exact carried cell inside their footprint; manifests that mix
/// Artifacts with oversized minions are refused for self-play instead of guessing that cell.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ArtifactPlacement {
    Carried { bearer: UnitTarget },
    Loose { location: Cell, region: Region },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ArtifactPosition {
    card: CardInstance,
    placement: ArtifactPlacement,
}

impl ArtifactPosition {
    /// The bearer carrying this Artifact, when it is not lying loose in the realm.
    const fn bearer(&self) -> Option<&UnitTarget> {
        match &self.placement {
            ArtifactPlacement::Carried { bearer } => Some(bearer),
            ArtifactPlacement::Loose { .. } => None,
        }
    }

    fn carried_by(&self, kind: UnitKind, seat: Seat, instance_id: &IdentityHash) -> bool {
        self.bearer().is_some_and(|bearer| {
            unit_target_kind(bearer) == kind
                && bearer.seat() == seat
                && bearer.instance_id() == instance_id
        })
    }
}

/// The turns a unit last used its once-per-turn Artifact interactions on.
#[derive(Clone, Copy)]
struct UnitTurns {
    dropped_artifacts: Option<u64>,
    interacted: Option<u64>,
    picked_up_artifacts: Option<u64>,
}

const fn unit_target_kind(target: &UnitTarget) -> UnitKind {
    match target {
        UnitTarget::Avatar { .. } => UnitKind::Avatar,
        UnitTarget::Minion { .. } => UnitKind::Minion,
    }
}

/// The damage a card discarded as payload deals: its printed mana cost, and nothing for a site.
///
/// Authority mana costs are far below the damage width, and a cost that did reach it would already
/// exceed every printed defense, so the saturating conversion cannot change an outcome.
fn payload_damage_amount(card: &CardDefinition) -> Result<u16, GameError> {
    let mana_cost = match &card.facts {
        CardFacts::Artifact(facts) => facts.mana_cost,
        CardFacts::Aura(facts) => facts.mana_cost,
        CardFacts::Magic(facts) => facts.mana_cost,
        CardFacts::Minion(facts) => facts.mana_cost,
        CardFacts::Site(_) => 0,
        CardFacts::Avatar(_) => return Err(GameError::IllegalAction),
    };
    Ok(u16::try_from(mana_cost).unwrap_or(u16::MAX))
}

/// A realm area whose occupants cannot depart until the recorded seat's next turn.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ImmobileArea {
    cells: BTreeSet<Cell>,
    expires_at_seat: Seat,
    source_instance_id: IdentityHash,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MagicChoice {
    ally: Option<UnitTarget>,
    ally_destination: Option<Location>,
    ally_strike_location: Option<Location>,
    cemetery_minion_instance_id: Option<IdentityHash>,
    discard_site_instance_id: Option<IdentityHash>,
    draw_zone: Option<DeckZone>,
    target: Option<UnitTarget>,
    target_location: Option<Location>,
    target_site_instance_id: Option<IdentityHash>,
    tempted_destination: Option<Location>,
    tempted_enemy: Option<UnitTarget>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UnitKind {
    Avatar,
    Minion,
}

impl UnitKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Avatar => "avatar",
            Self::Minion => "minion",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingCombat {
    allocations: Vec<StrikeAllocation>,
    attacker_instance_id: IdentityHash,
    attacker_kind: UnitKind,
    attacking_seat: Seat,
    cell: Cell,
    combatants: Vec<UnitTarget>,
    defenders: Vec<UnitTarget>,
    original_target: Option<CombatTarget>,
    region: Region,
    target_removed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BasicMovementPurpose {
    Defend,
    MoveAndAttack,
}

impl BasicMovementPurpose {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Defend => "defend",
            Self::MoveAndAttack => "move-and-attack",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingBasicMovement {
    path: Vec<Location>,
    path_index: usize,
    purpose: BasicMovementPurpose,
    ranged_strike_used: bool,
    seat: Seat,
    source_instance_id: IdentityHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingRangedStep {
    seat: Seat,
    source_instance_id: IdentityHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingChainMagic {
    card_id: CardId,
    card_instance_id: IdentityHash,
    caster_instance_id: IdentityHash,
    seat: Seat,
    targets: Vec<UnitTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingDeathriteSource {
    controller: Seat,
    current_power: u16,
    instance_id: IdentityHash,
    lethal: bool,
    unit: UnitPosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeathriteStage {
    ActiveOrder,
    NonActiveOrder,
    Resolve,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingDeathriteBatch {
    active_order: Vec<PendingDeathriteSource>,
    active_remaining: Vec<PendingDeathriteSource>,
    non_active_order: Vec<PendingDeathriteSource>,
    non_active_remaining: Vec<PendingDeathriteSource>,
    resolving: Vec<PendingDeathriteSource>,
    stage: DeathriteStage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingDeathrites {
    batches: Vec<PendingDeathriteBatch>,
    continuation: Option<DeathriteContinuation>,
    corpses: Vec<UnitPosition>,
    deck_losers: Vec<Seat>,
    deferred_magic_resolved: Option<DeferredMagicResolved>,
    defeated_avatars: Vec<Seat>,
    return_decision_seat: Seat,
    return_phase: Phase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeferredMagicResolved {
    card_id: CardId,
    instance_id: IdentityHash,
    owner: Seat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndTurnContinuation {
    remaining_instance_ids: Vec<IdentityHash>,
    seat: Seat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SiteGenesisContinuation {
    card_id: CardId,
    card_instance_id: IdentityHash,
    cell: Cell,
    create_rubble_at: Option<Cell>,
    defer_token: bool,
    from_top_atlas: bool,
    genesis_gain_mana: Option<u8>,
    genesis_spell_draw_count: usize,
    genesis_token_choice: Option<GenesisTokenChoice>,
    origin_state_version: u64,
    seat: Seat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PaidSummonContinuation {
    caster: UnitTarget,
    genesis_damage_choice: Option<GenesisDamageChoice>,
    genesis_damage_target: Option<UnitTarget>,
    mana_paid: u16,
    sacrificed_minion_instance_ids: Vec<IdentityHash>,
    unit: UnitPosition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DeathriteContinuation {
    Blink(BlinkContinuation),
    DragProjectile(DragProjectileContinuation),
    EndTurn(EndTurnContinuation),
    FirstStrike(FirstStrikeContinuation),
    LeapAttack(LeapAttackContinuation),
    PaidSummon(PaidSummonContinuation),
    SiteGenesis(SiteGenesisContinuation),
}

/// The private draw Blink still owes its caster once interrupting Deathrites finish.
#[derive(Clone, Debug, Eq, PartialEq)]
struct BlinkContinuation {
    card_id: String,
    instance_id: IdentityHash,
    owner: Seat,
    seat: Seat,
    zone: DeckZone,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DragProjectileContinuation {
    fight_on_arrival: bool,
    path: Vec<Location>,
    path_index: usize,
    shooter: UnitTarget,
    target: UnitTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LeapAttackContinuation {
    ally: UnitTarget,
    card_id: String,
    instance_id: IdentityHash,
    owner: Seat,
    strike_location: Location,
}

#[derive(Clone, Copy)]
struct LeapAttackRequest<'a> {
    ally: &'a UnitTarget,
    card_id: &'a str,
    card_instance_id: &'a IdentityHash,
    destination: Location,
    owner: Seat,
    seat: Seat,
    strike_location: Option<Location>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FirstStrikeContinuation {
    attacker_struck: bool,
    first_combatant_instance_ids: Vec<IdentityHash>,
    pending: PendingCombat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrikeAllocation {
    amount: u16,
    target_instance_id: IdentityHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingGenesisToken {
    cell: Cell,
    seat: Seat,
    source_instance_id: IdentityHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingGenesisSpell {
    seat: Seat,
    source_instance_id: IdentityHash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingGenesisSpellOrder {
    count: u8,
    seat: Seat,
    source_instance_id: IdentityHash,
}

/// The free placement a cemetery summon owes after its public random selection.
#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingCemeterySummon {
    card_instance_id: IdentityHash,
    card_owner: Seat,
    caster_instance_id: IdentityHash,
    seat: Seat,
    source_magic_card_id: CardId,
    source_magic_instance_id: IdentityHash,
    source_magic_owner: Seat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingField<T> {
    Absent,
    Pending(T),
    Resolved,
}

impl<T> PendingField<T> {
    const fn as_pending(&self) -> Option<&T> {
        match self {
            Self::Pending(value) => Some(value),
            Self::Absent | Self::Resolved => None,
        }
    }

    fn as_pending_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Pending(value) => Some(value),
            Self::Absent | Self::Resolved => None,
        }
    }

    const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Allocate,
    Attack,
    CemeterySummon,
    ChainMagic,
    DeathriteOrder,
    Defend,
    Draw,
    Genesis,
    Intercept,
    Main,
    Movement,
    Mulligan,
    RangedStep,
    Terminal,
}

impl Phase {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Allocate => "allocate",
            Self::Attack => "attack",
            Self::CemeterySummon => "cemetery-summon",
            Self::ChainMagic => "chain-magic",
            Self::DeathriteOrder => "deathrite-order",
            Self::Defend => "defend",
            Self::Draw => "draw",
            Self::Genesis => "genesis",
            Self::Intercept => "intercept",
            Self::Main => "main",
            Self::Movement => "movement",
            Self::Mulligan => "mulligan",
            Self::RangedStep => "ranged-step",
            Self::Terminal => "terminal",
        }
    }
}

/// The public result of a finished authoritative game.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameOutcome {
    /// Both Avatars were defeated by one simultaneous damage batch.
    Draw,
    /// Exactly one seat won the game.
    Win { loser: Seat, winner: Seat },
}

/// Why an authoritative game finished.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameEndReason {
    /// Exactly one Avatar was defeated.
    AvatarDefeated,
    /// A player attempted to draw from an empty deck.
    DeckEmpty,
    /// Both Avatars were defeated by one simultaneous damage batch.
    SimultaneousAvatarDefeat,
    /// Both players lost through a combined Avatar/deck defeat batch.
    SimultaneousDefeat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalResult {
    Draw {
        reason: DrawReason,
    },
    Win {
        loser: Seat,
        reason: WinReason,
        winner: Seat,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DrawReason {
    SimultaneousAvatarDefeat,
    SimultaneousDefeat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WinReason {
    AvatarDefeated,
    DeckEmpty,
}

struct DamageResult {
    minion_died: bool,
    avatar_defeated: bool,
}

#[derive(Clone, Copy)]
struct MinionDamageStatus {
    damage_prevention: Option<DamagePrevention>,
    defense: u16,
    disabled: bool,
    index: usize,
}

#[derive(Clone, Copy)]
struct UnitDamageSource {
    current_power: u16,
    lethal: bool,
}

#[derive(Clone, Copy)]
struct StrikeStats {
    amount: u16,
    current_power: u16,
    lance_count: u8,
    lethal: bool,
}

#[derive(Clone, Copy)]
struct SummonDestination {
    cell: Cell,
    cells: Option<SquareArea>,
    mana_cost: u64,
}

#[derive(Clone, Copy)]
struct CemeterySummonRequest<'a> {
    card_instance_id: &'a IdentityHash,
    caster_instance_id: &'a IdentityHash,
    seat: Seat,
    source_magic_card_id: CardId,
    source_magic_owner: Seat,
}

fn minimum_cardinal_distance(source: &[Cell], target: &[Cell]) -> u8 {
    source
        .iter()
        .flat_map(|left| {
            target.iter().map(move |right| {
                left.file_index().abs_diff(right.file_index())
                    + left.rank_index().abs_diff(right.rank_index())
            })
        })
        .min()
        .unwrap_or(u8::MAX)
}

fn nonempty_identity_combinations(
    instance_ids: &[IdentityHash],
    maximum_count: usize,
) -> Vec<Vec<IdentityHash>> {
    fn choose(
        instance_ids: &[IdentityHash],
        start: usize,
        remaining: usize,
        chosen: &mut Vec<IdentityHash>,
        combinations: &mut Vec<Vec<IdentityHash>>,
    ) {
        if remaining == 0 {
            combinations.push(chosen.clone());
            return;
        }
        for index in start..=instance_ids.len() - remaining {
            chosen.push(instance_ids[index].clone());
            choose(instance_ids, index + 1, remaining - 1, chosen, combinations);
            chosen.pop();
        }
    }

    let mut combinations = Vec::new();
    let mut chosen = Vec::with_capacity(maximum_count);
    for count in 1..=maximum_count.min(instance_ids.len()) {
        choose(instance_ids, 0, count, &mut chosen, &mut combinations);
    }
    combinations
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MovementCause {
    BasicMovement,
    CardEffect,
}

/// The lower realm layers a mover may enter and cross under its own power.
#[derive(Clone, Copy, Default)]
struct RegionAbilities {
    burrowing: bool,
    submerge: bool,
    voidwalk: bool,
}

impl RegionAbilities {
    const fn of_minion(minion: &MinionFacts) -> Self {
        Self {
            burrowing: minion.burrowing,
            submerge: minion.submerge,
            voidwalk: minion.voidwalk,
        }
    }
}

#[derive(Clone, Copy)]
struct MovementProfile {
    airborne: bool,
    cause: MovementCause,
    connects_top_bottom: bool,
    maximum_cost: Option<usize>,
    moving_minion: bool,
    occupied_cells: Option<SquareArea>,
    regions: RegionAbilities,
    restriction: Option<BasicMovementRestriction>,
    seat: Seat,
}

struct ProjectileOption {
    direction: ProjectileDirection,
    hit: Option<UnitTarget>,
    path: Vec<Location>,
}

enum OutcomeLog<'a> {
    Ignore,
    Record(&'a mut Vec<(String, Value)>),
}

impl OutcomeLog<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Ignore => 0,
            Self::Record(outcomes) => outcomes.len(),
        }
    }

    fn move_tail_before_completion(&mut self, tail_start: usize) {
        let Self::Record(outcomes) = self else {
            return;
        };
        let completion = outcomes[..tail_start].iter().position(|(kind, _)| {
            matches!(
                kind.as_str(),
                "game-ended" | "magic-resolved" | "turn-ended"
            )
        });
        let Some(completion) = completion else {
            return;
        };
        let mut tail = outcomes.drain(tail_start..).collect::<Vec<_>>();
        let terminal = tail
            .iter()
            .position(|(kind, _)| kind == "game-ended")
            .map(|index| tail.split_off(index))
            .unwrap_or_default();
        outcomes.splice(completion..completion, tail);
        outcomes.extend(terminal);
    }

    fn remove_first(&mut self, event_type: &str) {
        let Self::Record(outcomes) = self else {
            return;
        };
        if let Some(index) = outcomes
            .iter()
            .position(|(candidate, _)| candidate == event_type)
        {
            outcomes.remove(index);
        }
    }

    fn push(&mut self, kind: &'static str, payload: impl FnOnce() -> Value) {
        if let Self::Record(outcomes) = self {
            outcomes.push((kind.to_owned(), payload()));
        }
    }

    fn insert(&mut self, index: usize, kind: &'static str, payload: impl FnOnce() -> Value) {
        if let Self::Record(outcomes) = self {
            outcomes.insert(index, (kind.to_owned(), payload()));
        }
    }
}

fn invalid(message: impl Into<String>) -> GameError {
    GameError::InvalidManifest(message.into())
}

const fn card_kind(facts: &CardFacts) -> CardKind {
    match facts {
        CardFacts::Artifact(_) => CardKind::Artifact,
        CardFacts::Aura(_) => CardKind::Aura,
        CardFacts::Avatar(_) => CardKind::Avatar,
        CardFacts::Magic(_) => CardKind::Magic,
        CardFacts::Minion(_) => CardKind::Minion,
        CardFacts::Site(_) => CardKind::Site,
    }
}

fn token_reference(facts: &CardFacts) -> Option<&str> {
    match facts {
        CardFacts::Site(site) => site.genesis_pay_one_mana_to_summon_token.as_deref(),
        CardFacts::Magic(magic) => match &magic.effect {
            MagicEffect::SummonTokenToEachControlledSiteBorderingEnemySite(card_id) => {
                Some(card_id)
            }
            _ => None,
        },
        _ => None,
    }
}

fn has_unsupported_site_genesis_after_rubble_replacement(facts: &SiteFacts) -> bool {
    unsupported_site_genesis_after_rubble_replacement(facts).is_some()
}

fn unsupported_site_genesis_after_rubble_replacement(facts: &SiteFacts) -> Option<&'static str> {
    if facts.genesis_discard_top_spells {
        Some("genesisDiscardTopSpells")
    } else if facts.genesis_draw_spell_per_adjacent_same_card {
        Some("genesisDrawSpellPerAdjacentSameCard")
    } else if facts.genesis_enemies_lose_stealth {
        Some("genesisEnemiesLoseStealth")
    } else if facts.genesis_gain_mana.is_some() {
        Some("genesisGainMana")
    } else if facts.genesis_gain_mana_if_only_controlled_copy {
        Some("genesisGainManaIfOnlyControlledCopy")
    } else if facts.genesis_heal_nearby_avatars {
        Some("genesisHealNearbyAvatars")
    } else if facts.genesis_immobilize_nearby_until_next_turn {
        Some("genesisImmobilizeNearbyUntilNextTurn")
    } else {
        None
    }
}

fn unsupported_selfplay_fact(facts: &CardFacts) -> Option<&'static str> {
    match facts {
        CardFacts::Avatar(facts) => {
            account_for_selfplay_avatar_fields(*facts);
            None
        }
        CardFacts::Artifact(facts) => unsupported_selfplay_artifact(facts),
        CardFacts::Aura(_) => Some("cardType:aura"),
        CardFacts::Magic(facts) => unsupported_selfplay_magic(facts),
        CardFacts::Minion(facts) => unsupported_selfplay_minion(facts),
        CardFacts::Site(facts) => unsupported_selfplay_site(facts),
    }
}

fn unsupported_selfplay_magic(facts: &MagicFacts) -> Option<&'static str> {
    unsupported_magic_effect(&facts.effect)
}

fn unsupported_selfplay_artifact(facts: &ArtifactFacts) -> Option<&'static str> {
    unsupported_artifact_effect(facts.effect)
}

/// The Artifact effects the realm cannot yet honor, named by their authoring field.
const fn unsupported_artifact_effect(effect: ArtifactEffect) -> Option<&'static str> {
    match effect {
        ArtifactEffect::AtEndOfEachTurnSiteControllerLosesLife(_)
        | ArtifactEffect::GrantsBearerLethal
        | ArtifactEffect::GrantsBearerPowerTwo
        | ArtifactEffect::TapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
        | ArtifactEffect::TapBearerAndAnotherAllyHereToDamageTargetWithinTwoStepsThree
        | ArtifactEffect::TapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPathFour => None,
        ArtifactEffect::BearerControllerChoosesExtraRandomOutcome => {
            Some("bearerControllerChoosesExtraRandomOutcome")
        }
    }
}

/// Artifact effects the realm honors in full once the Artifact reaches play.
const fn artifact_effect_supported(effect: ArtifactEffect) -> bool {
    matches!(
        effect,
        ArtifactEffect::AtEndOfEachTurnSiteControllerLosesLife(_)
            | ArtifactEffect::GrantsBearerLethal
            | ArtifactEffect::GrantsBearerPowerTwo
            | ArtifactEffect::TapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
            | ArtifactEffect::TapBearerAndAnotherAllyHereToDamageTargetWithinTwoStepsThree
            | ArtifactEffect::TapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPathFour
    )
}

/// Facts that reach realm Artifacts through paths the engine does not implement yet.
fn unsupported_alongside_artifacts(facts: &CardFacts) -> Option<&'static str> {
    match facts {
        CardFacts::Magic(facts) => match facts.effect {
            MagicEffect::BurrowAllMinionsAndArtifactsAtTargetLandSite => {
                Some("burrowAllMinionsAndArtifactsAtTargetLandSite")
            }
            MagicEffect::BurrowTargetMinionOrArtifact => Some("burrowTargetMinionOrArtifact"),
            _ => None,
        },
        // An oversized bearer carries each Artifact at one exact cell of its footprint.
        CardFacts::Minion(facts) => facts
            .occupies_square_area_two
            .then_some("occupiesSquareArea"),
        _ => None,
    }
}

fn unsupported_magic_effect(effect: &MagicEffect) -> Option<&'static str> {
    match effect {
        MagicEffect::HealController(_)
        | MagicEffect::BurrowAllMinionsAndArtifactsAtTargetLandSite
        | MagicEffect::BurrowTargetMinionOrArtifact
        | MagicEffect::DamageChainNearbyUnits
        | MagicEffect::DamageEachAbovegroundMinionOne
        | MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(_)
        | MagicEffect::DamageRandomUnitAtLocation(_)
        | MagicEffect::ReturnMinionFromOwnCemetery
        | MagicEffect::DamageTargetUnit { .. }
        | MagicEffect::DestroyTargetSiteWithDamageGrid(_)
        | MagicEffect::DisableTargetNearbyMinionUntilNextTurn
        | MagicEffect::FightAllyWithAdjacentEnemy
        | MagicEffect::LeapAttackAlly
        | MagicEffect::SummonTokenToEachControlledSiteBorderingEnemySite(_)
        | MagicEffect::GrantChargeToAllyThisTurn
        | MagicEffect::GrantPowerTwoToAllyThisTurn
        | MagicEffect::GainControlOfTargetNearbyMinion
        | MagicEffect::KillTargetWoundedMinion
        | MagicEffect::LureEnemyMinionOneStepCloser
        | MagicEffect::SubmergeTargetMinion
        | MagicEffect::SummonRandomMinionFromAnyCemetery
        | MagicEffect::TeleportAllyToTargetSite
        | MagicEffect::TeleportNearbyAllyThenDrawCard => None,
    }
}

fn unsupported_selfplay_minion(facts: &MinionFacts) -> Option<&'static str> {
    account_for_selfplay_minion_fields(facts);
    if facts.at_start_of_controller_turn_teleport_to_random_site_or_void {
        Some("atStartOfControllerTurnTeleportToRandomSiteOrVoid")
    } else if facts.burrowing {
        Some("burrowing")
    } else if let Some(field) = unsupported_selfplay_minion_genesis(facts.genesis) {
        Some(field)
    } else if facts.must_be_cast_to_outer_column {
        Some("mustBeCastToOuterColumn")
    } else if facts.submerge {
        Some("submerge")
    } else if facts.voidwalk {
        Some("voidwalk")
    } else if facts.waterbound {
        Some("waterbound")
    } else {
        None
    }
}

fn unsupported_selfplay_site(facts: &SiteFacts) -> Option<&'static str> {
    account_for_selfplay_site_fields(facts);
    if facts.connects_burrowed_allies {
        Some("connectsBurrowedAllies")
    } else if facts.fly_to_nearby_void_once_per_turn_at_air_threshold {
        Some("flyToNearbyVoidOncePerTurnAtAirThreshold")
    } else if facts.minions_here_gain_voidwalk_until_leaving_void {
        Some("minionsHereGainVoidwalkUntilLeavingVoid")
    } else if facts
        .prevents_units_with_power_at_least_from_entering
        .is_some()
    {
        Some("preventsUnitsWithPowerAtLeastFromEntering")
    } else {
        None
    }
}

fn unsupported_selfplay_minion_genesis(genesis: Option<MinionGenesis>) -> Option<&'static str> {
    match genesis {
        None
        | Some(
            MinionGenesis::DamageEachOtherUnitHereOne
            | MinionGenesis::DisableSelfUntilDamaged
            | MinionGenesis::DrawSite
            | MinionGenesis::DrawSpells(_)
            | MinionGenesis::HealControllerTwo
            | MinionGenesis::LoseControllerLifeTwo
            | MinionGenesis::MayDamageTargetAdjacentUnitTwo
            | MinionGenesis::StrikeEachEnemyHere,
        ) => None,
    }
}

fn account_for_selfplay_avatar_fields(facts: AvatarFacts) {
    let AvatarFacts {
        attack: _,
        defense: _,
        draw_spell: _,
        earth_site_play_creates_adjacent_rubble: _,
        life: _,
        replace_adjacent_rubble_with_top_atlas_site: _,
        tap_damage_random_other_unit_at_nearby_location_per_air_threshold_cast_this_turn: _,
    } = facts;
}

fn account_for_selfplay_site_fields(facts: &SiteFacts) {
    let SiteFacts {
        airborne_minions_atop_move_freely_away: _,
        blocks_ground_minion_entry_while_minion_atop: _,
        cannot_be_moved_destroyed_or_modified: _,
        connects_burrowed_allies: _,
        elements: _,
        fly_to_nearby_void_once_per_turn_at_air_threshold: _,
        genesis_discard_top_spells: _,
        genesis_draw_spell_per_adjacent_same_card: _,
        genesis_enemies_lose_stealth: _,
        genesis_gain_mana: _,
        genesis_gain_mana_if_only_controlled_copy: _,
        genesis_heal_nearby_avatars: _,
        genesis_immobilize_nearby_until_next_turn: _,
        genesis_may_bottom_next_spell: _,
        genesis_pay_one_mana_to_summon_token: _,
        genesis_reorder_next_spells: _,
        is_tower: _,
        minions_here_gain_voidwalk_until_leaving_void: _,
        ordinary_minion_mana_discount: _,
        prevents_units_with_power_at_least_from_entering: _,
        ranged_units_here_range_bonus: _,
        sacrifice_to_destroy_nearby_site: _,
    } = facts;
}

fn account_for_selfplay_minion_fields(facts: &MinionFacts) {
    let MinionFacts {
        airborne: _,
        alternative_summon_payment: _,
        at_start_of_controller_turn_teleport_to_random_site_or_void: _,
        attack: _,
        burrowing: _,
        cannot_attack_sites: _,
        cannot_defend: _,
        cannot_defend_or_intercept: _,
        charge: _,
        connects_top_bottom: _,
        damage_prevention: _,
        deathrite_damage_each_unit_here: _,
        deathrite_draw_site: _,
        deathrite_heal: _,
        deathrite_lose_life_per_nearby_site_controlled: _,
        defense: _,
        dies_at_end_of_controller_turn: _,
        discard_spell_to_damage_random_other_unit_here: _,
        end_turn_stealth: _,
        gains_power_ranged_and_spellcaster_atop_tower: _,
        genesis: _,
        immobile: _,
        lance_count: _,
        lethal: _,
        mana_cost: _,
        may_ranged_strike_once_during_basic_movement: _,
        may_step_after_ranged_strike: _,
        mortal: _,
        movement_bonus: _,
        movement_restriction: _,
        must_be_cast_to_outer_column: _,
        must_be_cast_to_water_site: _,
        nearby_enemies_permanently_lose_stealth: _,
        occupies_square_area_two: _,
        ordinary: _,
        other_controlled_mortals_power_bonus: _,
        other_nearby_allies_power_bonus: _,
        provides: _,
        ranged: _,
        required_cast_region: _,
        shoots_drag_projectile: _,
        site_provides_no_threshold: _,
        spellcaster: _,
        stealth: _,
        strikes_first_while_attacking: _,
        submerge: _,
        summon_to_any_site: _,
        tap_for_mana: _,
        tap_to_damage_each_unit_at_adjacent_location: _,
        tap_to_shoot_projectile_damage: _,
        thresholds: _,
        token: _,
        untaps_at_end_of_controller_turn: _,
        voidwalk: _,
        waterbound: _,
    } = facts;
}

fn validate_deck(deck: &Deck, cards: &BTreeMap<String, CardFacts>) -> Result<(), GameError> {
    if cards.get(&deck.avatar).map(card_kind) != Some(CardKind::Avatar) {
        return Err(invalid("deck.avatar must reference an avatar"));
    }
    for (zone, expected) in [(&deck.atlas, None), (&deck.spellbook, Some(()))] {
        if !(3..=MAX_DECK_CARDS).contains(&zone.len()) {
            return Err(invalid("deck zone must contain 3-200 cards"));
        }
        for card_id in zone {
            let facts = cards
                .get(card_id)
                .ok_or_else(|| invalid("deck references a missing card"))?;
            let kind = card_kind(facts);
            if expected.is_none() && kind != CardKind::Site {
                return Err(invalid("atlas must contain only sites"));
            }
            if expected.is_some()
                && !matches!(
                    kind,
                    CardKind::Artifact | CardKind::Aura | CardKind::Magic | CardKind::Minion
                )
            {
                return Err(invalid("spellbook references an unsupported spell"));
            }
        }
    }
    Ok(())
}

fn seat_index(seat: Seat) -> usize {
    match seat {
        Seat::North => 0,
        Seat::South => 1,
    }
}

const fn other_seat(seat: Seat) -> Seat {
    match seat {
        Seat::North => Seat::South,
        Seat::South => Seat::North,
    }
}

impl Game {
    /// Validates a canonical manifest and creates its deterministic opening position.
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] for malformed, noncanonical, or unsupported manifests.
    pub fn from_manifest_json(manifest_json: &str) -> Result<Self, GameError> {
        let raw: Value = serde_json::from_str(manifest_json)?;
        if crate::canonical::canonical_json(&raw)? != manifest_json.trim_end() {
            return Err(invalid("game manifest JSON is not canonical"));
        }
        let manifest: Manifest = serde_json::from_value(raw.clone())?;
        let mut parsed_facts = validate_manifest(&raw, &manifest)?;
        if let Some(field) = parsed_facts.values().find_map(|facts| {
            let CardFacts::Minion(facts) = facts else {
                return None;
            };
            match facts.required_cast_region {
                Some(RequiredCastRegion::Underground) => Some("mustBeCastBurrowed"),
                Some(RequiredCastRegion::Underwater) => Some("mustBeCastSubmerged"),
                None => None,
            }
        }) {
            return Err(GameError::UnsupportedManifestFact(field.to_owned()));
        }

        let mut cards = Vec::with_capacity(manifest.cards.len());
        let mut card_ids = BTreeMap::new();
        for (index, (id, value)) in manifest.cards.iter().enumerate() {
            let id_number = u16::try_from(index)
                .map_err(|_| invalid("manifest contains too many card definitions"))?;
            let facts = parsed_facts
                .remove(id)
                .ok_or_else(|| invalid("validated manifest lacks parsed card facts"))?;
            cards.push(CardDefinition {
                definition_hash: identity_hash(value)?,
                facts,
                id: id.clone(),
                value: value.clone(),
            });
            card_ids.insert(id.clone(), CardId(id_number));
        }
        let rules = Arc::new(RulesContext {
            authority_hash: manifest.authority.content_hash.clone(),
            cards: cards.into_boxed_slice(),
            first_seat: manifest.first_seat,
            manifest_id: manifest.identity,
            seed: manifest.seed,
        });

        let mut prng = PrngState::new(manifest.seed);
        let mut initial_random_draws = Vec::new();
        let north = create_player(
            &rules,
            manifest.decks.get(Seat::North),
            Seat::North,
            &card_ids,
            &mut prng,
            &mut initial_random_draws,
        )?;
        let south = create_player(
            &rules,
            manifest.decks.get(Seat::South),
            Seat::South,
            &card_ids,
            &mut prng,
            &mut initial_random_draws,
        )?;
        Ok(Self {
            initial_random_draws: initial_random_draws.into(),
            rules,
            position: Position {
                active_seat: Seat::North,
                artifacts: Vec::new(),
                decision_seat: Seat::North,
                immobile_areas: Vec::new(),
                pending_basic_movement: PendingField::Absent,
                pending_cemetery_summon: None,
                pending_chain_magic: PendingField::Absent,
                pending_combat: None,
                pending_deathrites: None,
                pending_genesis_spell: PendingField::Absent,
                pending_genesis_spell_order: PendingField::Absent,
                pending_genesis_token: PendingField::Absent,
                pending_ranged_step: PendingField::Absent,
                phase: Phase::Mulligan,
                players: [north, south],
                prng,
                rubble: std::array::from_fn(|_| None),
                sites: std::array::from_fn(|_| None),
                state_version: 0,
                terminal: None,
                turn_number: 0,
                units: Vec::new(),
            },
        })
    }

    /// Returns the shared immutable rule context.
    #[must_use]
    pub fn rules(&self) -> &Arc<RulesContext> {
        &self.rules
    }

    /// Returns the compact dynamic position.
    #[must_use]
    pub const fn position(&self) -> &Position {
        &self.position
    }

    pub(crate) fn into_position(self) -> Position {
        self.position
    }

    pub(crate) fn ensure_selfplay_supported(&self) -> Result<(), GameError> {
        for card in &self.rules.cards {
            if let Some(field) = unsupported_selfplay_fact(&card.facts) {
                return Err(GameError::UnsupportedManifestFact(field.to_owned()));
            }
        }
        if self
            .rules
            .cards
            .iter()
            .any(|card| matches!(card.facts, CardFacts::Artifact(_)))
        {
            for card in &self.rules.cards {
                if let Some(field) = unsupported_alongside_artifacts(&card.facts) {
                    return Err(GameError::UnsupportedManifestFact(format!(
                        "{field} with cardType:artifact"
                    )));
                }
            }
        }
        for player in &self.position.players {
            let CardFacts::Avatar(avatar) =
                &self.rules.cards[usize::from(player.avatar.card.card_id.0)].facts
            else {
                return Err(invalid("player avatar lacks avatar facts"));
            };
            if !avatar.replace_adjacent_rubble_with_top_atlas_site {
                continue;
            }
            for card in player.atlas.iter().chain(&player.hand_atlas) {
                let CardFacts::Site(site) = &self.rules.cards[usize::from(card.card_id.0)].facts
                else {
                    return Err(invalid("player Atlas card lacks site facts"));
                };
                if let Some(field) = unsupported_site_genesis_after_rubble_replacement(site) {
                    return Err(GameError::UnsupportedManifestFact(field.to_owned()));
                }
            }
        }
        Ok(())
    }

    /// Builds the compact public policy view for `seat` without exposing hidden identities.
    #[must_use]
    pub fn observe(&self, seat: Seat) -> SeatObservation {
        let player = &self.position.players[seat_index(seat)];
        let enemy = &self.position.players[seat_index(other_seat(seat))];
        SeatObservation {
            atlas_remaining: player.atlas.len(),
            enemy_avatar: Location {
                cell: enemy.avatar.location,
                region: Region::Surface,
            },
            seat,
            spellbook_remaining: player.spellbook.len(),
        }
    }

    /// Returns whether the authoritative game has reached a terminal result.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.position.terminal.is_some()
    }

    /// Returns the public result after the game finishes.
    #[must_use]
    pub const fn outcome(&self) -> Option<GameOutcome> {
        match self.position.terminal {
            Some(TerminalResult::Draw { .. }) => Some(GameOutcome::Draw),
            Some(TerminalResult::Win { loser, winner, .. }) => {
                Some(GameOutcome::Win { loser, winner })
            }
            None => None,
        }
    }

    /// Returns why the game finished.
    #[must_use]
    pub const fn terminal_reason(&self) -> Option<GameEndReason> {
        match self.position.terminal {
            Some(TerminalResult::Draw {
                reason: DrawReason::SimultaneousAvatarDefeat,
            }) => Some(GameEndReason::SimultaneousAvatarDefeat),
            Some(TerminalResult::Draw {
                reason: DrawReason::SimultaneousDefeat,
            }) => Some(GameEndReason::SimultaneousDefeat),
            Some(TerminalResult::Win {
                reason: WinReason::AvatarDefeated,
                ..
            }) => Some(GameEndReason::AvatarDefeated),
            Some(TerminalResult::Win {
                reason: WinReason::DeckEmpty,
                ..
            }) => Some(GameEndReason::DeckEmpty),
            None => None,
        }
    }

    /// Returns the current turn number.
    #[must_use]
    pub const fn turn_number(&self) -> u64 {
        self.position.turn_number
    }

    /// Returns immutable setup draw records in authoritative sequence order.
    #[must_use]
    pub fn initial_random_draws(&self) -> &[EngineRandomDraw] {
        &self.initial_random_draws
    }

    /// Hashes the authoritative setup draw records.
    ///
    /// # Errors
    ///
    /// Returns [`CanonicalError`] if the records cannot be canonicalized.
    pub fn initial_random_draws_hash(&self) -> Result<IdentityHash, GameError> {
        Ok(identity_hash(&serde_json::to_value(
            &*self.initial_random_draws,
        )?)?)
    }

    /// Enumerates typed legal actions in canonical order.
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] if the current position cannot issue its required actions.
    pub fn legal_actions(&self) -> Result<Vec<IssuedAction>, GameError> {
        let mut actions = Vec::new();
        match self.position.phase {
            Phase::Allocate => self.append_allocate_actions(&mut actions)?,
            Phase::Attack => self.append_attack_actions(&mut actions)?,
            Phase::CemeterySummon => self.append_cemetery_summon_actions(&mut actions)?,
            Phase::ChainMagic => self.append_chain_magic_actions(&mut actions)?,
            Phase::DeathriteOrder => self.append_deathrite_order_actions(&mut actions)?,
            Phase::Defend => self.append_defend_actions(&mut actions)?,
            Phase::Draw => self.append_draw_actions(&mut actions)?,
            Phase::Genesis => self.append_genesis_actions(&mut actions)?,
            Phase::Intercept => self.append_intercept_actions(&mut actions)?,
            Phase::Mulligan => self.append_mulligan_actions(&mut actions)?,
            Phase::Main => self.append_main_actions(&mut actions)?,
            Phase::Movement => self.append_basic_movement_actions(&mut actions)?,
            Phase::RangedStep => self.append_ranged_step_actions(&mut actions)?,
            Phase::Terminal => {}
        }
        actions
            .sort_unstable_by(|left, right| compare_canonical(&left.descriptor, &right.descriptor));
        Ok(actions)
    }

    fn append_basic_movement_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_basic_movement
            .as_pending()
            .ok_or(GameError::IllegalAction)?;
        if pending.seat != self.position.decision_seat {
            return Err(GameError::IllegalAction);
        }
        let label = pending.path.get(pending.path_index + 1).map_or_else(
            || {
                format!(
                    "Finish {}",
                    match pending.purpose {
                        BasicMovementPurpose::Defend => "Defend",
                        BasicMovementPurpose::MoveAndAttack => "Move and Attack",
                    }
                )
            },
            |destination| {
                format!(
                    "Continue {}… to {}",
                    &pending.source_instance_id.as_str()[..15],
                    destination.cell
                )
            },
        );
        self.push_action(
            actions,
            ActionDescriptor::ContinueBasicMovement {
                unit_instance_id: pending.source_instance_id.clone(),
            },
            label,
        );
        if !pending.ranged_strike_used {
            for descriptor in
                self.ranged_projectile_descriptors(pending.seat, &pending.source_instance_id, true)?
            {
                let label = descriptor
                    .state_independent_label()
                    .ok_or_else(|| invalid("movement Ranged action requires a label"))?;
                self.push_action(actions, descriptor, label);
            }
        }
        Ok(())
    }

    fn append_ranged_step_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_ranged_step
            .as_pending()
            .ok_or(GameError::IllegalAction)?;
        if pending.seat != self.position.decision_seat {
            return Err(GameError::IllegalAction);
        }
        for descriptor in self.ranged_step_descriptors(pending)? {
            let label = descriptor
                .state_independent_label()
                .ok_or(GameError::IllegalAction)?;
            self.push_action(actions, descriptor, label);
        }
        Ok(())
    }

    fn append_chain_magic_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_chain_magic
            .as_pending()
            .ok_or(GameError::IllegalAction)?;
        if pending.seat != self.position.decision_seat || pending.targets.is_empty() {
            return Err(GameError::IllegalAction);
        }
        let player = &self.position.players[seat_index(pending.seat)];
        let card = player
            .hand_spellbook
            .iter()
            .find(|card| {
                card.card_id == pending.card_id && card.instance_id == pending.card_instance_id
            })
            .ok_or(GameError::IllegalAction)?;
        let definition = &self.rules.cards[usize::from(card.card_id.0)];
        let CardFacts::Magic(facts) = &definition.facts else {
            return Err(GameError::IllegalAction);
        };
        if facts.effect != MagicEffect::DamageChainNearbyUnits {
            return Err(GameError::IllegalAction);
        }
        let caster_kind = self
            .spellcasters(pending.seat)
            .into_iter()
            .find_map(|(kind, instance_id)| {
                (instance_id == pending.caster_instance_id).then_some(kind)
            })
            .ok_or(GameError::IllegalAction)?;
        let caster = match caster_kind {
            UnitKind::Avatar => UnitTarget::Avatar {
                instance_id: pending.caster_instance_id.clone(),
                seat: pending.seat,
            },
            UnitKind::Minion => UnitTarget::Minion {
                instance_id: pending.caster_instance_id.clone(),
                seat: pending.seat,
            },
        };
        let chosen_count =
            u64::try_from(pending.targets.len()).map_err(|_| GameError::IllegalAction)?;
        let mana_paid = facts.mana_cost
            + CHAIN_MAGIC_EXTRA_TARGET_MANA.saturating_mul(chosen_count.saturating_sub(1));
        let finish = ActionDescriptor::ResolveChainMagic;
        self.push_action(
            actions,
            finish,
            format!(
                "Cast {} through {} chosen unit{} ({mana_paid} mana)",
                definition.id,
                pending.targets.len(),
                if pending.targets.len() == 1 { "" } else { "s" }
            ),
        );
        let next_mana =
            facts.mana_cost + CHAIN_MAGIC_EXTRA_TARGET_MANA.saturating_mul(chosen_count);
        if u64::from(player.mana) >= next_mana {
            let previous = pending.targets.last().ok_or(GameError::IllegalAction)?;
            for target in
                self.chain_magic_targets(pending.seat, &caster, previous, &pending.targets)?
            {
                let descriptor = ActionDescriptor::ExtendChainMagic { target };
                let label = descriptor
                    .state_independent_label()
                    .ok_or_else(|| invalid("extend-chain-magic action requires a label"))?;
                self.push_action(actions, descriptor, label);
            }
        }
        Ok(())
    }

    fn ranged_step_descriptors(
        &self,
        pending: &PendingRangedStep,
    ) -> Result<Vec<ActionDescriptor>, GameError> {
        let mut descriptors = vec![ActionDescriptor::ResolveRangedStep {
            choice: RangedStepChoice::Decline,
            from: None,
            path: None,
            to: None,
            unit_instance_id: pending.source_instance_id.clone(),
        }];
        let Some(unit) = self.position.units.iter().find(|unit| {
            unit.controller == pending.seat
                && unit.card.instance_id == pending.source_instance_id
                && unit.region == Region::Surface
                && !self.minion_is_disabled(unit)
        }) else {
            return Ok(descriptors);
        };
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        if !facts.may_step_after_ranged_strike {
            return Ok(descriptors);
        }
        let from = Location {
            cell: unit.location,
            region: Region::Surface,
        };
        let profile = MovementProfile {
            airborne: facts.airborne,
            cause: MovementCause::BasicMovement,
            connects_top_bottom: facts.connects_top_bottom,
            maximum_cost: (!facts.immobile).then_some(1),
            moving_minion: true,
            occupied_cells: unit.occupied_cells,
            regions: RegionAbilities::of_minion(facts),
            restriction: facts.movement_restriction,
            seat: pending.seat,
        };
        for path in self
            .movement_paths(from, profile)
            .into_iter()
            .filter(|path| path.len() == 2)
        {
            let to = *path.last().ok_or(GameError::IllegalAction)?;
            descriptors.push(ActionDescriptor::ResolveRangedStep {
                choice: RangedStepChoice::Step,
                from: Some(from),
                path: Some(path),
                to: Some(to),
                unit_instance_id: pending.source_instance_id.clone(),
            });
        }
        Ok(descriptors)
    }

    fn append_allocate_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let target = pending
            .combatants
            .get(pending.allocations.len())
            .ok_or(GameError::IllegalAction)?;
        let attack = self
            .combatant_strike_stats(
                pending.attacker_kind,
                pending.attacking_seat,
                &pending.attacker_instance_id,
            )?
            .amount;
        let assigned = pending
            .allocations
            .iter()
            .try_fold(0_u16, |total, allocation| {
                total.checked_add(allocation.amount)
            })
            .ok_or(GameError::IllegalAction)?;
        let remaining = attack
            .checked_sub(assigned)
            .ok_or(GameError::IllegalAction)?;
        let amounts = if pending.allocations.len() + 1 == pending.combatants.len() {
            remaining..=remaining
        } else {
            0..=remaining
        };
        for amount in amounts {
            let descriptor = ActionDescriptor::AllocateStrike {
                amount: u64::from(amount),
                target_instance_id: target.instance_id().clone(),
            };
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("allocate-strike action requires a label"))?;
            self.push_action(actions, descriptor, label);
        }
        Ok(())
    }

    fn append_deathrite_order_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
    ) -> Result<(), GameError> {
        let sources = self.pending_deathrite_order()?;
        for source in sources {
            let descriptor = ActionDescriptor::OrderDeathrites {
                source_instance_id: source.instance_id.clone(),
            };
            let card_id = &self.rules.cards[usize::from(source.unit.card.card_id.0)].id;
            self.push_action(
                actions,
                descriptor,
                format!("Order {card_id} first within your Deathrites"),
            );
        }
        Ok(())
    }

    fn pending_deathrite_order(&self) -> Result<&[PendingDeathriteSource], GameError> {
        let batch = self
            .position
            .pending_deathrites
            .as_ref()
            .and_then(|pending| pending.batches.first())
            .ok_or(GameError::IllegalAction)?;
        let sources = match batch.stage {
            DeathriteStage::ActiveOrder => &batch.active_remaining,
            DeathriteStage::NonActiveOrder => &batch.non_active_remaining,
            DeathriteStage::Resolve => return Err(GameError::IllegalAction),
        };
        if sources.len() < 2
            || sources
                .first()
                .is_none_or(|source| source.controller != self.position.decision_seat)
        {
            return Err(GameError::IllegalAction);
        }
        Ok(sources)
    }

    fn append_attack_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        for target in self.attack_targets()? {
            let descriptor = ActionDescriptor::DeclareAttack { target };
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("declare-attack action requires a label"))?;
            self.push_action(actions, descriptor, label);
        }
        let descriptor = ActionDescriptor::DeclineAttack;
        let label = descriptor
            .state_independent_label()
            .ok_or_else(|| invalid("decline-attack action requires a label"))?;
        self.push_action(actions, descriptor, label);
        Ok(())
    }

    fn append_defend_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let target = pending
            .original_target
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let destination = Location {
            cell: pending.cell,
            region: Region::Surface,
        };
        for (_kind, instance_id, start, profile) in self.defender_candidates()? {
            for path in self
                .movement_paths(
                    Location {
                        cell: start,
                        region: Region::Surface,
                    },
                    profile,
                )
                .into_iter()
                .filter(|path| {
                    let Some(end) = path.last() else {
                        return false;
                    };
                    profile.occupied_cells.map_or(end == &destination, |area| {
                        translated_square(area, start, end.cell)
                            .is_some_and(|cells| cells.contains(&destination.cell))
                    })
                })
            {
                let descriptor = ActionDescriptor::Defend {
                    from: path[0],
                    path,
                    to: destination,
                    unit_instance_id: instance_id.clone(),
                };
                let label = descriptor
                    .state_independent_label()
                    .ok_or_else(|| invalid("defend action requires a label"))?;
                self.push_action(actions, descriptor, label);
            }
        }
        let choices: &[bool] = if matches!(target, CombatTarget::Site { .. }) {
            &[false]
        } else if pending.defenders.is_empty() {
            &[true]
        } else {
            &[true, false]
        };
        for original_target_participates in choices {
            let descriptor = ActionDescriptor::CloseDefend {
                original_target_participates: *original_target_participates,
            };
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("close-defend action requires a label"))?;
            self.push_action(actions, descriptor, label);
        }
        Ok(())
    }

    fn append_intercept_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        for target in self.interceptor_candidates()? {
            let descriptor = ActionDescriptor::Intercept {
                unit_instance_id: target.instance_id().clone(),
            };
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("intercept action requires a label"))?;
            self.push_action(actions, descriptor, label);
        }
        let descriptor = ActionDescriptor::CloseIntercept {};
        let label = descriptor
            .state_independent_label()
            .ok_or_else(|| invalid("close-intercept action requires a label"))?;
        self.push_action(actions, descriptor, label);
        Ok(())
    }

    fn interceptor_candidates(&self) -> Result<Vec<UnitTarget>, GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let seat = other_seat(pending.attacking_seat);
        if self.combatant_stealthed(
            pending.attacker_kind,
            pending.attacking_seat,
            &pending.attacker_instance_id,
        )? {
            return Ok(Vec::new());
        }
        let attacker_airborne = self.combatant_airborne(
            pending.attacker_kind,
            pending.attacking_seat,
            &pending.attacker_instance_id,
        )?;
        let unavailable: BTreeSet<_> = pending
            .defenders
            .iter()
            .map(|target| target.instance_id().clone())
            .collect();
        let mut candidates = Vec::new();
        let player = &self.position.players[seat_index(seat)];
        if player.avatar.location == pending.cell
            && !player.avatar.tapped
            && !unavailable.contains(&player.avatar.card.instance_id)
            && !attacker_airborne
        {
            candidates.push(UnitTarget::Avatar {
                instance_id: player.avatar.card.instance_id.clone(),
                seat,
            });
        }
        for unit in &self.position.units {
            if unit.controller != seat
                || unit.region != Region::Surface
                || !Self::unit_occupies_cell(unit, pending.cell)
                || unit.tapped
                || self.minion_is_disabled(unit)
                || unavailable.contains(&unit.card.instance_id)
            {
                continue;
            }
            let CardFacts::Minion(facts) =
                &self.rules.cards[usize::from(unit.card.card_id.0)].facts
            else {
                return Err(invalid("realm minion lacks Minion facts"));
            };
            if facts.cannot_defend_or_intercept
                || unit.summoning_sickness && !self.minion_has_active_charge(unit)
                || attacker_airborne && !facts.airborne && !self.minion_is_ranged(unit, facts)
            {
                continue;
            }
            candidates.push(UnitTarget::Minion {
                instance_id: unit.card.instance_id.clone(),
                seat,
            });
        }
        Ok(candidates)
    }

    fn combatant_stealthed(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<bool, GameError> {
        match kind {
            UnitKind::Avatar => Ok(false),
            UnitKind::Minion => self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == seat && unit.card.instance_id == *instance_id)
                .map(|unit| self.minion_has_active_stealth(unit))
                .ok_or(GameError::IllegalAction),
        }
    }

    fn combatant_airborne(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<bool, GameError> {
        if kind == UnitKind::Avatar {
            return Ok(false);
        }
        let unit = self
            .position
            .units
            .iter()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *instance_id)
            .ok_or(GameError::IllegalAction)?;
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        Ok(facts.airborne && !self.minion_is_disabled(unit))
    }

    fn defender_candidates(
        &self,
    ) -> Result<Vec<(UnitKind, IdentityHash, Cell, MovementProfile)>, GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let seat = other_seat(pending.attacking_seat);
        let unavailable: BTreeSet<_> = pending
            .defenders
            .iter()
            .map(|target| target.instance_id().clone())
            .chain(
                pending
                    .original_target
                    .as_ref()
                    .filter(|target| !matches!(target, CombatTarget::Site { .. }))
                    .map(|target| target.instance_id().clone()),
            )
            .collect();
        let player = &self.position.players[seat_index(seat)];
        let mut candidates = Vec::new();
        if !player.avatar.tapped && !unavailable.contains(&player.avatar.card.instance_id) {
            candidates.push((
                UnitKind::Avatar,
                player.avatar.card.instance_id.clone(),
                player.avatar.location,
                MovementProfile {
                    airborne: false,
                    cause: MovementCause::BasicMovement,
                    connects_top_bottom: false,
                    maximum_cost: Some(1),
                    moving_minion: false,
                    occupied_cells: None,
                    regions: RegionAbilities::default(),
                    restriction: None,
                    seat,
                },
            ));
        }
        for unit in &self.position.units {
            if unit.controller != seat
                || unit.region != Region::Surface
                || unit.tapped
                || self.minion_is_disabled(unit)
                || unavailable.contains(&unit.card.instance_id)
            {
                continue;
            }
            let CardFacts::Minion(facts) =
                &self.rules.cards[usize::from(unit.card.card_id.0)].facts
            else {
                return Err(invalid("realm minion lacks Minion facts"));
            };
            if facts.cannot_defend_or_intercept {
                continue;
            }
            if unit.summoning_sickness && !self.minion_has_active_charge(unit) {
                continue;
            }
            candidates.push((
                UnitKind::Minion,
                unit.card.instance_id.clone(),
                unit.location,
                MovementProfile {
                    airborne: facts.airborne,
                    cause: MovementCause::BasicMovement,
                    connects_top_bottom: facts.connects_top_bottom,
                    maximum_cost: if facts.cannot_defend || facts.immobile {
                        None
                    } else {
                        Some(1 + usize::from(facts.movement_bonus.unwrap_or(0)))
                    },
                    moving_minion: true,
                    occupied_cells: unit.occupied_cells,
                    regions: RegionAbilities::of_minion(facts),
                    restriction: facts.movement_restriction,
                    seat,
                },
            ));
        }
        Ok(candidates)
    }

    fn attack_targets(&self) -> Result<Vec<CombatTarget>, GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let attacker_airborne = self.combatant_airborne(
            pending.attacker_kind,
            pending.attacking_seat,
            &pending.attacker_instance_id,
        )?;
        let opposing_seat = other_seat(pending.attacking_seat);
        let opposing_player = &self.position.players[seat_index(opposing_seat)];
        let attacker_cells = self.combatant_occupied_cells(
            pending.attacker_kind,
            pending.attacking_seat,
            &pending.attacker_instance_id,
        )?;
        let mut targets = Vec::new();
        if attacker_cells.contains(&opposing_player.avatar.location) {
            targets.push(CombatTarget::Avatar {
                instance_id: opposing_player.avatar.card.instance_id.clone(),
                seat: opposing_seat,
            });
        }
        for unit in &self.position.units {
            if unit.controller != opposing_seat
                || unit.region != Region::Surface
                || !Self::unit_occupied_cells(unit)
                    .iter()
                    .any(|cell| attacker_cells.contains(cell))
                || self.minion_has_active_stealth(unit)
            {
                continue;
            }
            let CardFacts::Minion(facts) =
                &self.rules.cards[usize::from(unit.card.card_id.0)].facts
            else {
                return Err(invalid("realm minion lacks Minion facts"));
            };
            if !facts.airborne || self.minion_is_disabled(unit) || attacker_airborne {
                targets.push(CombatTarget::Minion {
                    instance_id: unit.card.instance_id.clone(),
                    seat: opposing_seat,
                });
            }
        }
        if self.attacker_can_target_sites(pending)? {
            for cell in attacker_cells {
                if let Some(site) = &self.position.sites[cell.index()]
                    && site.controller == opposing_seat
                {
                    targets.push(CombatTarget::Site {
                        instance_id: site.card.instance_id.clone(),
                        seat: opposing_seat,
                    });
                }
            }
        }
        Ok(targets)
    }

    fn append_draw_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        for zone in [DeckZone::Atlas, DeckZone::Spellbook] {
            let descriptor = ActionDescriptor::Draw { zone };
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("draw action requires a label"))?;
            self.push_action(actions, descriptor, label);
        }
        Ok(())
    }

    /// Issues the free placements owed by an already selected cemetery minion.
    fn append_cemetery_summon_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
    ) -> Result<(), GameError> {
        let seat = self.position.decision_seat;
        let pending = self
            .position
            .pending_cemetery_summon
            .as_ref()
            .ok_or_else(|| invalid("cemetery summon phase lacks its selected minion"))?;
        if pending.seat != seat {
            return Err(invalid("cemetery summon placement belongs to another seat"));
        }
        let card = self.position.players[seat_index(pending.card_owner)]
            .cemetery
            .iter()
            .find(|card| card.instance_id == pending.card_instance_id)
            .ok_or_else(|| invalid("selected cemetery minion left its cemetery"))?;
        let definition = &self.rules.cards[usize::from(card.card_id.0)];
        let CardFacts::Minion(facts) = &definition.facts else {
            return Err(invalid("selected cemetery minion lacks minion facts"));
        };
        for destination in self.free_summon_destinations(facts) {
            for (genesis_damage_choice, genesis_damage_target) in self.genesis_damage_choices(
                seat,
                &card.instance_id,
                destination.cell,
                facts.genesis,
            ) {
                let genesis_suffix = Self::genesis_damage_suffix(
                    genesis_damage_choice,
                    genesis_damage_target.as_ref(),
                );
                self.push_action(
                    actions,
                    ActionDescriptor::SummonMinion {
                        card_id: definition.id.clone(),
                        card_instance_id: card.instance_id.clone(),
                        caster_instance_id: pending.caster_instance_id.clone(),
                        cell: destination.cell,
                        cells: destination.cells,
                        genesis_damage_choice,
                        genesis_damage_target,
                        mana_cost: 0,
                        payment_mode: None,
                        sacrificed_minion_instance_ids: None,
                    },
                    format!(
                        "Raise {} at {} (free){genesis_suffix}",
                        definition.id, destination.cell
                    ),
                );
            }
        }
        Ok(())
    }

    fn append_genesis_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        let seat = self.position.decision_seat;
        if let PendingField::Pending(pending) = &self.position.pending_genesis_spell_order {
            if pending.seat != seat {
                return Err(invalid("Genesis spell order belongs to another seat"));
            }
            let player = &self.position.players[seat_index(seat)];
            let count = usize::from(pending.count);
            if player.spellbook.len() < count {
                return Err(invalid("pending Genesis spell order exceeds its Spellbook"));
            }
            let indices: Vec<_> = (0..pending.count).collect();
            for order in permutations(&indices) {
                let label = order
                    .iter()
                    .map(|index| {
                        let card = &player.spellbook[usize::from(*index)];
                        self.rules.cards[usize::from(card.card_id.0)].id.as_str()
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                self.push_action(
                    actions,
                    ActionDescriptor::ResolveGenesisSpellOrder { order },
                    format!("Order next spells {label}"),
                );
            }
            return Ok(());
        }
        if let PendingField::Pending(pending) = &self.position.pending_genesis_token {
            if pending.seat != seat {
                return Err(invalid("Genesis token choice belongs to another seat"));
            }
            self.push_action(
                actions,
                ActionDescriptor::ResolveGenesisToken {
                    choice: GenesisTokenChoice::Decline,
                },
                "Decline the optional Genesis token".to_owned(),
            );
            if self.position.players[seat_index(pending.seat)].mana > 0 {
                let site = self.position.sites[pending.cell.index()]
                    .as_ref()
                    .ok_or_else(|| invalid("pending Genesis token source is missing"))?;
                let CardFacts::Site(facts) =
                    &self.rules.cards[usize::from(site.card.card_id.0)].facts
                else {
                    return Err(invalid("pending Genesis token source is not a site"));
                };
                let token_card_id = facts
                    .genesis_pay_one_mana_to_summon_token
                    .as_deref()
                    .ok_or_else(|| invalid("pending Genesis site lacks its token fact"))?;
                self.push_action(
                    actions,
                    ActionDescriptor::ResolveGenesisToken {
                        choice: GenesisTokenChoice::PayOneMana,
                    },
                    format!("Pay 1 to summon {token_card_id}"),
                );
            }
            return Ok(());
        }
        let PendingField::Pending(pending) = &self.position.pending_genesis_spell else {
            return Err(invalid("Genesis phase lacks its pending choice"));
        };
        if pending.seat != seat {
            return Err(invalid("Genesis spell choice belongs to another seat"));
        }
        let top = self.position.players[seat_index(seat)]
            .spellbook
            .first()
            .ok_or_else(|| invalid("pending Genesis spell lacks a top card"))?;
        let card_id = &self.rules.cards[usize::from(top.card_id.0)].id;
        for choice in [GenesisSpellChoice::KeepNext, GenesisSpellChoice::BottomNext] {
            let label = match choice {
                GenesisSpellChoice::BottomNext => format!("Put {card_id} on bottom"),
                GenesisSpellChoice::KeepNext => format!("Keep {card_id} on top"),
            };
            self.push_action(
                actions,
                ActionDescriptor::ResolveGenesisSpell { choice },
                label,
            );
        }
        Ok(())
    }

    fn append_mulligan_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        let seat = self.position.decision_seat;
        let player = &self.position.players[seat_index(seat)];
        let hand: Vec<_> = player
            .hand_atlas
            .iter()
            .chain(&player.hand_spellbook)
            .collect();
        actions.reserve(76);
        for mask in 0_u8..(1_u8 << hand.len()) {
            if mask.count_ones() > 3 {
                continue;
            }
            let selected: Vec<_> = hand
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, card)| *card)
                .collect();
            let atlas: Vec<_> = selected
                .iter()
                .filter(|card| card.source == CardSource::Atlas)
                .map(|card| card.instance_id.clone())
                .collect();
            let spellbook: Vec<_> = selected
                .iter()
                .filter(|card| card.source == CardSource::Spellbook)
                .map(|card| card.instance_id.clone())
                .collect();
            for atlas_order in permutations(&atlas) {
                for spellbook_order in permutations(&spellbook) {
                    let descriptor = ActionDescriptor::Mulligan {
                        atlas_order: atlas_order.clone(),
                        spellbook_order,
                    };
                    let label = descriptor
                        .state_independent_label()
                        .ok_or_else(|| invalid("mulligan action requires a label"))?;
                    self.push_action(actions, descriptor, label);
                }
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "main-phase legality keeps each engine-issued action filter together"
    )]
    fn append_main_actions(&self, actions: &mut Vec<IssuedAction>) -> Result<(), GameError> {
        let seat = self.position.decision_seat;
        let player = &self.position.players[seat_index(seat)];
        let CardFacts::Avatar(avatar) =
            &self.rules.cards[usize::from(player.avatar.card.card_id.0)].facts
        else {
            return Err(invalid("player Avatar lacks Avatar facts"));
        };
        if !player.avatar.tapped {
            let cells = self.legal_site_cells(seat);
            for card in &player.hand_atlas {
                let definition = &self.rules.cards[usize::from(card.card_id.0)];
                let card_id = &definition.id;
                let CardFacts::Site(site_facts) = &definition.facts else {
                    return Err(invalid("Atlas hand card lacks Site facts"));
                };
                let paid_token = site_facts.genesis_pay_one_mana_to_summon_token.is_some();
                let creates_rubble = avatar.earth_site_play_creates_adjacent_rubble
                    && site_facts.elements.contains(Element::Earth);
                let token_choices = if paid_token {
                    [
                        Some(GenesisTokenChoice::Decline),
                        Some(GenesisTokenChoice::PayOneMana),
                    ]
                } else {
                    [None, None]
                };
                for cell in &cells {
                    let rubble_choices: Vec<_> = if creates_rubble {
                        let candidates: Vec<_> = player
                            .avatar
                            .location
                            .bordering(false)
                            .filter(|candidate| {
                                candidate != cell
                                    && self.position.sites[candidate.index()].is_none()
                                    && self.position.rubble[candidate.index()].is_none()
                            })
                            .map(Some)
                            .collect();
                        if candidates.is_empty() {
                            vec![None]
                        } else {
                            candidates
                        }
                    } else {
                        vec![None]
                    };
                    for genesis_token_choice in
                        token_choices
                            .into_iter()
                            .take(if paid_token { 2 } else { 1 })
                    {
                        let token_suffix = match genesis_token_choice {
                            Some(GenesisTokenChoice::Decline) => " (decline Genesis)",
                            Some(GenesisTokenChoice::PayOneMana) => " (pay 1 for Genesis)",
                            None => "",
                        };
                        for create_rubble_at in &rubble_choices {
                            let rubble_suffix = create_rubble_at.map_or_else(String::new, |cell| {
                                format!(" — create Rubble at {cell}")
                            });
                            self.push_action(
                                actions,
                                ActionDescriptor::PlaySite {
                                    card_id: card_id.clone(),
                                    card_instance_id: card.instance_id.clone(),
                                    cell: *cell,
                                    create_rubble_at: *create_rubble_at,
                                    genesis_token_choice,
                                },
                                format!("Play {card_id} at {cell}{token_suffix}{rubble_suffix}"),
                            );
                        }
                    }
                }
            }
            if avatar.replace_adjacent_rubble_with_top_atlas_site && !player.atlas.is_empty() {
                for target_cell in player.avatar.location.bordering(false) {
                    let Some(target_rubble_instance_id) =
                        self.position.rubble[target_cell.index()].as_ref()
                    else {
                        continue;
                    };
                    let descriptor = ActionDescriptor::ReplaceRubbleWithTopAtlasSite {
                        target_cell,
                        target_rubble_instance_id: target_rubble_instance_id.clone(),
                    };
                    let label = descriptor
                        .state_independent_label()
                        .ok_or_else(|| invalid("Rubble replacement requires a label"))?;
                    self.push_action(actions, descriptor, label);
                }
            }
            if avatar
                .tap_damage_random_other_unit_at_nearby_location_per_air_threshold_cast_this_turn
            {
                let amount = player.air_thresholds_cast_this_turn.unwrap_or(0);
                let source_instance_id = player.avatar.card.instance_id.clone();
                let cells = std::iter::once(player.avatar.location)
                    .chain(player.avatar.location.bordering(false))
                    .chain(player.avatar.location.diagonals(false))
                    .filter(|cell| self.surface_location_exists(*cell))
                    .collect::<BTreeSet<_>>();
                for cell in cells {
                    self.push_action(
                        actions,
                        ActionDescriptor::ActivateSparkmage {
                            source_instance_id: source_instance_id.clone(),
                            target_location: Location {
                                cell,
                                region: Region::Surface,
                            },
                        },
                        format!("Tap Sparkmage to deal {amount} to a random other unit at {cell}"),
                    );
                }
            }
        }
        if !player.domain_established {
            return Ok(());
        }
        for source_cell in Cell::ALL {
            let Some(source) = self.position.sites[source_cell.index()].as_ref() else {
                continue;
            };
            let CardFacts::Site(source_facts) =
                &self.rules.cards[usize::from(source.card.card_id.0)].facts
            else {
                return Err(invalid("realm site lacks Site facts"));
            };
            if source.controller != seat || !source_facts.sacrifice_to_destroy_nearby_site {
                continue;
            }
            let nearby = std::iter::once(source_cell)
                .chain(source_cell.bordering(false))
                .chain(source_cell.diagonals(false))
                .collect::<BTreeSet<_>>();
            for target_cell in Cell::ALL {
                if !nearby.contains(&target_cell) {
                    continue;
                }
                let target_site_instance_id = self.position.sites[target_cell.index()]
                    .as_ref()
                    .map(|site| site.card.instance_id.clone())
                    .or_else(|| self.position.rubble[target_cell.index()].clone());
                let Some(target_site_instance_id) = target_site_instance_id else {
                    continue;
                };
                let descriptor = ActionDescriptor::ActivateSiteDestruction {
                    source_site_instance_id: source.card.instance_id.clone(),
                    target_cell,
                    target_site_instance_id,
                };
                let label = descriptor
                    .state_independent_label()
                    .ok_or_else(|| invalid("site destruction action requires a label"))?;
                self.push_action(actions, descriptor, label);
            }
        }
        let spellcasters = self.spellcasters(seat);
        for card in &player.hand_spellbook {
            let definition = &self.rules.cards[usize::from(card.card_id.0)];
            let CardFacts::Magic(facts) = &definition.facts else {
                continue;
            };
            if u64::from(player.mana) < facts.mana_cost
                || !self.thresholds_met(seat, facts.thresholds)
            {
                continue;
            }
            for (caster_kind, caster_instance_id) in &spellcasters {
                if facts.effect == MagicEffect::DamageChainNearbyUnits {
                    let caster = match caster_kind {
                        UnitKind::Avatar => UnitTarget::Avatar {
                            instance_id: caster_instance_id.clone(),
                            seat,
                        },
                        UnitKind::Minion => UnitTarget::Minion {
                            instance_id: caster_instance_id.clone(),
                            seat,
                        },
                    };
                    for target in self.chain_magic_targets(seat, &caster, &caster, &[])? {
                        let descriptor = ActionDescriptor::BeginChainMagic {
                            card_id: definition.id.clone(),
                            card_instance_id: card.instance_id.clone(),
                            caster_instance_id: caster_instance_id.clone(),
                            target,
                        };
                        let label = descriptor
                            .state_independent_label()
                            .ok_or_else(|| invalid("begin-chain-magic action requires a label"))?;
                        self.push_action(actions, descriptor, label);
                    }
                    continue;
                }
                for choice in self.magic_choices(seat, caster_instance_id, &facts.effect)? {
                    let descriptor = ActionDescriptor::CastMagic {
                        ally: choice.ally,
                        ally_destination: choice.ally_destination,
                        ally_strike_location: choice.ally_strike_location,
                        card_id: definition.id.clone(),
                        card_instance_id: card.instance_id.clone(),
                        caster_instance_id: caster_instance_id.clone(),
                        cemetery_minion_instance_id: choice.cemetery_minion_instance_id,
                        discard_site_instance_id: choice.discard_site_instance_id,
                        draw_zone: choice.draw_zone,
                        target: choice.target,
                        target_location: choice.target_location,
                        target_site_instance_id: choice.target_site_instance_id,
                        tempted_destination: choice.tempted_destination,
                        tempted_enemy: choice.tempted_enemy,
                    };
                    let label = (if matches!(facts.effect, MagicEffect::GrantPowerTwoToAllyThisTurn)
                    {
                        let ActionDescriptor::CastMagic {
                            ally: Some(ally), ..
                        } = &descriptor
                        else {
                            return Err(invalid("Overpower action requires an ally"));
                        };
                        format!(
                            "Cast {} to grant +2 power to {} {}…",
                            definition.id,
                            ally.kind(),
                            &ally.instance_id().as_str()[..15]
                        )
                    } else if matches!(facts.effect, MagicEffect::LeapAttackAlly) {
                        let ActionDescriptor::CastMagic {
                            ally: Some(ally),
                            ally_destination: Some(destination),
                            ally_strike_location,
                            ..
                        } = &descriptor
                        else {
                            return Err(invalid("Leap Attack action requires an ally destination"));
                        };
                        let from = self.unit_target_location(ally)?;
                        let stays = from == *destination;
                        let strike = ally_strike_location.unwrap_or(*destination);
                        format!(
                            "Cast {}: {} {}… {} and strikes enemies at {}",
                            definition.id,
                            ally.kind(),
                            &ally.instance_id().as_str()[..15],
                            if stays {
                                "stays".to_owned()
                            } else {
                                format!("steps to {}", destination.cell)
                            },
                            strike.cell
                        )
                    } else {
                        descriptor
                            .state_independent_label()
                            .ok_or_else(|| invalid("cast-magic action requires a label"))?
                    }) + &self.minion_caster_suffix(seat, caster_instance_id);
                    self.push_action(actions, descriptor, label);
                }
            }
        }
        for card in &player.hand_spellbook {
            let definition = &self.rules.cards[usize::from(card.card_id.0)];
            let CardFacts::Minion(facts) = &definition.facts else {
                continue;
            };
            if !self.thresholds_met(seat, facts.thresholds) {
                continue;
            }
            for destination in self.summon_destinations(seat, facts) {
                let can_discard_random_card = facts.alternative_summon_payment
                    == Some(AlternativeSummonPayment::DiscardRandomCardInsteadOfMana)
                    && (!player.hand_atlas.is_empty()
                        || player
                            .hand_spellbook
                            .iter()
                            .any(|candidate| candidate.instance_id != card.instance_id));
                let payments = [
                    (destination.mana_cost <= u64::from(player.mana)).then_some((
                        destination.mana_cost,
                        None,
                        None,
                    )),
                    can_discard_random_card.then_some((
                        0,
                        Some(SummonPaymentMode::RandomCardDiscard),
                        None,
                    )),
                ];
                let mut sacrifice_payments = Vec::new();
                if facts.alternative_summon_payment
                    == Some(
                        AlternativeSummonPayment::SacrificeMinionAtSummoningLocationForManaDiscountTwo,
                    )
                {
                    let mut sacrifice_candidates = self
                        .position
                        .units
                        .iter()
                        .filter(|unit| {
                            unit.controller == seat
                                && unit.region == Region::Surface
                                && Self::unit_occupies_cell(unit, destination.cell)
                        })
                        .map(|unit| unit.card.instance_id.clone())
                        .collect::<Vec<_>>();
                    sacrifice_candidates.sort_unstable();
                    let useful_count = usize::try_from(destination.mana_cost.div_ceil(2))
                        .unwrap_or(usize::MAX)
                        .min(sacrifice_candidates.len());
                    for sacrificed in
                        nonempty_identity_combinations(&sacrifice_candidates, useful_count)
                    {
                        let count = u64::try_from(sacrificed.len())
                            .map_err(|_| GameError::IllegalAction)?;
                        let mana_cost = destination
                            .mana_cost
                            .saturating_sub(count.saturating_mul(2));
                        if mana_cost <= u64::from(player.mana) {
                            sacrifice_payments.push((mana_cost, None, Some(sacrificed)));
                        }
                    }
                }
                let genesis_choices = self.genesis_damage_choices(
                    seat,
                    &card.instance_id,
                    destination.cell,
                    facts.genesis,
                );
                for (mana_cost, payment_mode, sacrificed_minion_instance_ids) in
                    payments.into_iter().flatten().chain(sacrifice_payments)
                {
                    for (caster_kind, caster_instance_id) in &spellcasters {
                        for (genesis_damage_choice, genesis_damage_target) in &genesis_choices {
                            let genesis_suffix = Self::genesis_damage_suffix(
                                *genesis_damage_choice,
                                genesis_damage_target.as_ref(),
                            );
                            let caster_suffix = if *caster_kind == UnitKind::Minion {
                                self.minion_caster_suffix(seat, caster_instance_id)
                            } else {
                                String::new()
                            };
                            let payment =
                                if payment_mode == Some(SummonPaymentMode::RandomCardDiscard) {
                                    "discard random card".to_owned()
                                } else if let Some(sacrificed) = &sacrificed_minion_instance_ids {
                                    format!(
                                        "{mana_cost} mana + sacrifice {} minion{}",
                                        sacrificed.len(),
                                        if sacrificed.len() == 1 { "" } else { "s" }
                                    )
                                } else {
                                    format!("{mana_cost} mana")
                                };
                            self.push_action(
                                actions,
                                ActionDescriptor::SummonMinion {
                                    card_id: definition.id.clone(),
                                    card_instance_id: card.instance_id.clone(),
                                    caster_instance_id: caster_instance_id.clone(),
                                    cell: destination.cell,
                                    cells: destination.cells,
                                    genesis_damage_choice: *genesis_damage_choice,
                                    genesis_damage_target: genesis_damage_target.clone(),
                                    mana_cost,
                                    payment_mode,
                                    sacrificed_minion_instance_ids: sacrificed_minion_instance_ids
                                        .clone(),
                                },
                                format!(
                                    "Summon {} at {} ({payment}){genesis_suffix}{caster_suffix}",
                                    definition.id, destination.cell
                                ),
                            );
                        }
                    }
                }
            }
        }
        for unit in &self.position.units {
            let Some(amount) = self.mana_activation_amount(seat, &unit.card.instance_id) else {
                continue;
            };
            let descriptor = ActionDescriptor::ActivateMana {
                amount: u64::from(amount),
                unit_instance_id: unit.card.instance_id.clone(),
            };
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("activate-mana action requires a label"))?;
            self.push_action(actions, descriptor, label);
        }
        for unit in &self.position.units {
            for descriptor in
                self.ranged_projectile_descriptors(seat, &unit.card.instance_id, false)?
            {
                let label = descriptor
                    .state_independent_label()
                    .ok_or_else(|| invalid("shoot-projectile action requires a label"))?;
                self.push_action(actions, descriptor, label);
            }
            for descriptor in self.damage_projectile_descriptors(seat, &unit.card.instance_id)? {
                let label = descriptor
                    .state_independent_label()
                    .ok_or_else(|| invalid("shoot-damage-projectile action requires a label"))?;
                self.push_action(actions, descriptor, label);
            }
            for descriptor in self.drag_projectile_descriptors(seat, &unit.card.instance_id)? {
                let label = descriptor
                    .state_independent_label()
                    .ok_or_else(|| invalid("shoot-drag-projectile action requires a label"))?;
                self.push_action(actions, descriptor, label);
            }
        }
        self.append_area_damage_actions(actions, seat);
        self.append_discard_random_damage_actions(actions, seat);
        if !player.avatar.tapped {
            self.append_unit_move_actions(
                actions,
                &player.avatar.card.instance_id,
                player.avatar.location,
                MovementProfile {
                    airborne: false,
                    cause: MovementCause::BasicMovement,
                    connects_top_bottom: false,
                    maximum_cost: Some(1),
                    moving_minion: false,
                    occupied_cells: None,
                    regions: RegionAbilities::default(),
                    restriction: None,
                    seat,
                },
            )?;
        }
        for unit in self
            .position
            .units
            .iter()
            .filter(|unit| self.minion_can_move_and_attack(unit, seat))
        {
            let CardFacts::Minion(facts) =
                &self.rules.cards[usize::from(unit.card.card_id.0)].facts
            else {
                return Err(invalid("realm minion lacks Minion facts"));
            };
            self.append_unit_move_actions(
                actions,
                &unit.card.instance_id,
                unit.location,
                MovementProfile {
                    airborne: facts.airborne,
                    cause: MovementCause::BasicMovement,
                    connects_top_bottom: facts.connects_top_bottom,
                    maximum_cost: if facts.immobile {
                        None
                    } else {
                        Some(1 + usize::from(facts.movement_bonus.unwrap_or(0)))
                    },
                    moving_minion: true,
                    occupied_cells: unit.occupied_cells,
                    regions: RegionAbilities::of_minion(facts),
                    restriction: facts.movement_restriction,
                    seat,
                },
            )?;
        }
        self.append_artifact_actions(actions, seat)?;
        if !player.avatar.tapped {
            let descriptor = ActionDescriptor::DrawSite;
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("draw-site action requires a label"))?;
            self.push_action(actions, descriptor, label);
            if avatar.draw_spell {
                let descriptor = ActionDescriptor::DrawSpell;
                let label = descriptor
                    .state_independent_label()
                    .ok_or_else(|| invalid("draw-spell action requires a label"))?;
                self.push_action(actions, descriptor, label);
            }
        }
        self.push_action(actions, ActionDescriptor::EndTurn, "End turn".to_owned());
        Ok(())
    }

    fn mana_activation_amount(&self, seat: Seat, instance_id: &IdentityHash) -> Option<u8> {
        if self.position.phase != Phase::Main
            || self.position.active_seat != seat
            || self.position.decision_seat != seat
        {
            return None;
        }
        let unit = self.position.units.iter().find(|unit| {
            unit.card.instance_id == *instance_id
                && unit.controller == seat
                && !unit.tapped
                && !unit.summoning_sickness
        })?;
        if self.minion_is_disabled(unit) {
            return None;
        }
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return None;
        };
        facts.tap_for_mana
    }

    /// Offers each adjacent location a ready area-damage minion can currently blanket.
    fn append_area_damage_actions(&self, actions: &mut Vec<IssuedAction>, seat: Seat) {
        if self.position.phase != Phase::Main
            || self.position.active_seat != seat
            || self.position.decision_seat != seat
        {
            return;
        }
        for unit in &self.position.units {
            if unit.controller != seat
                || unit.tapped
                || unit.summoning_sickness
                || self.minion_is_disabled(unit)
            {
                continue;
            }
            let CardFacts::Minion(facts) =
                &self.rules.cards[usize::from(unit.card.card_id.0)].facts
            else {
                continue;
            };
            if !facts.tap_to_damage_each_unit_at_adjacent_location {
                continue;
            }
            let occupied = Self::unit_occupied_cells(unit);
            let adjacent: BTreeSet<_> = occupied
                .iter()
                .flat_map(|cell| cell.bordering(false))
                .filter(|cell| {
                    !occupied.contains(cell) && self.location_exists_in_region(*cell, unit.region)
                })
                .collect();
            for cell in adjacent {
                self.push_action(
                    actions,
                    ActionDescriptor::ActivateAreaDamage {
                        source_instance_id: unit.card.instance_id.clone(),
                        target_location: Location {
                            cell,
                            region: unit.region,
                        },
                    },
                    format!(
                        "Tap {}… to damage every unit at {cell}",
                        &unit.card.instance_id.as_str()[..15]
                    ),
                );
            }
        }
    }

    /// Offers each discard-funded random damage activation the controller can currently pay for.
    fn append_discard_random_damage_actions(&self, actions: &mut Vec<IssuedAction>, seat: Seat) {
        if self.position.phase != Phase::Main
            || self.position.active_seat != seat
            || self.position.decision_seat != seat
        {
            return;
        }
        let player = &self.position.players[seat_index(seat)];
        let mut discards: Vec<_> = player.hand_spellbook.iter().collect();
        discards.sort_unstable_by(|left, right| left.instance_id.cmp(&right.instance_id));
        if discards.is_empty() {
            return;
        }
        for unit in &self.position.units {
            if unit.controller != seat || self.minion_is_disabled(unit) {
                continue;
            }
            let CardFacts::Minion(facts) =
                &self.rules.cards[usize::from(unit.card.card_id.0)].facts
            else {
                continue;
            };
            if facts
                .discard_spell_to_damage_random_other_unit_here
                .is_none()
            {
                continue;
            }
            for discard in &discards {
                let card_id = &self.rules.cards[usize::from(discard.card_id.0)].id;
                self.push_action(
                    actions,
                    ActionDescriptor::ActivateDiscardRandomDamage {
                        discard_card_instance_id: discard.instance_id.clone(),
                        source_instance_id: unit.card.instance_id.clone(),
                    },
                    format!(
                        "Discard {card_id} to activate {}…",
                        &unit.card.instance_id.as_str()[..15]
                    ),
                );
            }
        }
    }

    fn damage_projectile_descriptors(
        &self,
        seat: Seat,
        shooter_instance_id: &IdentityHash,
    ) -> Result<Vec<ActionDescriptor>, GameError> {
        if self.position.phase != Phase::Main
            || self.position.active_seat != seat
            || self.position.decision_seat != seat
        {
            return Ok(Vec::new());
        }
        let Some(shooter) =
            self.position.units.iter().find(|unit| {
                unit.controller == seat && unit.card.instance_id == *shooter_instance_id
            })
        else {
            return Ok(Vec::new());
        };
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(shooter.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        if facts.tap_to_shoot_projectile_damage.is_none()
            || shooter.region != Region::Surface
            || self.minion_is_disabled(shooter)
            || shooter.tapped
            || shooter.summoning_sickness
        {
            return Ok(Vec::new());
        }

        Ok(self
            .projectile_options(shooter, usize::MAX)
            .into_iter()
            .map(|option| ActionDescriptor::ShootDamageProjectile {
                direction: option.direction,
                hit: option.hit,
                path: option.path,
                shooter_instance_id: shooter_instance_id.clone(),
            })
            .collect())
    }

    fn drag_projectile_descriptors(
        &self,
        seat: Seat,
        shooter_instance_id: &IdentityHash,
    ) -> Result<Vec<ActionDescriptor>, GameError> {
        if self.position.phase != Phase::Main
            || self.position.active_seat != seat
            || self.position.decision_seat != seat
        {
            return Ok(Vec::new());
        }
        let Some(shooter) =
            self.position.units.iter().find(|unit| {
                unit.controller == seat && unit.card.instance_id == *shooter_instance_id
            })
        else {
            return Ok(Vec::new());
        };
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(shooter.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        if !facts.shoots_drag_projectile
            || shooter.region != Region::Surface
            || self.minion_is_disabled(shooter)
            || shooter.tapped
            || shooter.summoning_sickness
        {
            return Ok(Vec::new());
        }
        Ok(self
            .projectile_options(shooter, usize::MAX)
            .into_iter()
            .flat_map(|option| {
                // A miss leaves nothing to haul back, so only a hit offers the fight choice.
                let arrivals: &[bool] = if option.hit.is_some() {
                    &[false, true]
                } else {
                    &[false]
                };
                arrivals
                    .iter()
                    .map(|fight_on_arrival| ActionDescriptor::ShootDragProjectile {
                        direction: option.direction,
                        fight_on_arrival: *fight_on_arrival,
                        hit: option.hit.clone(),
                        path: option.path.clone(),
                        shooter_instance_id: shooter_instance_id.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect())
    }

    fn ranged_projectile_descriptors(
        &self,
        seat: Seat,
        shooter_instance_id: &IdentityHash,
        allow_tapped: bool,
    ) -> Result<Vec<ActionDescriptor>, GameError> {
        let main = self.position.phase == Phase::Main
            && self.position.active_seat == seat
            && self.position.decision_seat == seat;
        let movement = allow_tapped
            && self.position.phase == Phase::Movement
            && self
                .position
                .pending_basic_movement
                .as_pending()
                .is_some_and(|pending| {
                    pending.seat == seat
                        && pending.source_instance_id == *shooter_instance_id
                        && !pending.ranged_strike_used
                });
        if !main && !movement {
            return Ok(Vec::new());
        }
        let Some(shooter) =
            self.position.units.iter().find(|unit| {
                unit.controller == seat && unit.card.instance_id == *shooter_instance_id
            })
        else {
            return Ok(Vec::new());
        };
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(shooter.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        if !self.minion_is_ranged(shooter, facts)
            || shooter.region != Region::Surface
            || movement && !facts.may_ranged_strike_once_during_basic_movement
            || self.minion_is_disabled(shooter)
            || shooter.tapped && !allow_tapped
            || shooter.summoning_sickness
        {
            return Ok(Vec::new());
        }
        let maximum_steps = if self.position.sites[shooter.location.index()]
            .as_ref()
            .is_some_and(|site| {
                matches!(
                    &self.rules.cards[usize::from(site.card.card_id.0)].facts,
                    CardFacts::Site(site_facts) if site_facts.ranged_units_here_range_bonus
                )
            }) {
            2
        } else {
            1
        };
        Ok(self
            .projectile_options(shooter, maximum_steps)
            .into_iter()
            .map(|option| ActionDescriptor::ShootProjectile {
                direction: option.direction,
                hit: option.hit,
                path: option.path,
                shooter_instance_id: shooter_instance_id.clone(),
            })
            .collect())
    }

    fn projectile_options(
        &self,
        shooter: &UnitPosition,
        maximum_steps: usize,
    ) -> Vec<ProjectileOption> {
        let seat = shooter.controller;
        let shooter_instance_id = &shooter.card.instance_id;
        let origin = Location {
            cell: shooter.location,
            region: Region::Surface,
        };
        let mut options = Vec::new();
        for direction in [
            ProjectileDirection::East,
            ProjectileDirection::North,
            ProjectileDirection::South,
            ProjectileDirection::West,
        ] {
            let mut path = vec![origin];
            loop {
                let location = path.last().copied().unwrap_or(origin);
                let mut hits = Vec::new();
                for target_seat in [Seat::North, Seat::South] {
                    let avatar = &self.position.players[seat_index(target_seat)].avatar;
                    if avatar.card.instance_id != *shooter_instance_id
                        && avatar.location == location.cell
                        && (path.len() > 1 || target_seat != seat)
                    {
                        hits.push(UnitTarget::Avatar {
                            instance_id: avatar.card.instance_id.clone(),
                            seat: target_seat,
                        });
                    }
                }
                hits.extend(
                    self.position
                        .units
                        .iter()
                        .filter(|unit| {
                            unit.card.instance_id != *shooter_instance_id
                                && unit.region == Region::Surface
                                && Self::unit_occupies_cell(unit, location.cell)
                                && !self.minion_has_active_stealth(unit)
                                && (path.len() > 1 || unit.controller != seat)
                        })
                        .map(|unit| UnitTarget::Minion {
                            instance_id: unit.card.instance_id.clone(),
                            seat: unit.controller,
                        }),
                );
                if !hits.is_empty() {
                    hits.sort_unstable_by(|left, right| {
                        left.instance_id().cmp(right.instance_id())
                    });
                    options.extend(hits.into_iter().map(|hit| ProjectileOption {
                        direction,
                        hit: Some(hit),
                        path: path.clone(),
                    }));
                    break;
                }
                if path.len() > maximum_steps {
                    options.push(ProjectileOption {
                        direction,
                        hit: None,
                        path,
                    });
                    break;
                }
                let Some(next) = Self::projectile_step(location.cell, direction) else {
                    options.push(ProjectileOption {
                        direction,
                        hit: None,
                        path,
                    });
                    break;
                };
                if !self.surface_location_exists(next) {
                    options.push(ProjectileOption {
                        direction,
                        hit: None,
                        path,
                    });
                    break;
                }
                path.push(Location {
                    cell: next,
                    region: Region::Surface,
                });
            }
        }
        options
    }

    fn projectile_step(cell: Cell, direction: ProjectileDirection) -> Option<Cell> {
        let index = cell.index();
        match direction {
            ProjectileDirection::East if cell.file_index() < 4 => Some(Cell::ALL[index + 4]),
            ProjectileDirection::North if cell.rank_index() < 3 => Some(Cell::ALL[index + 1]),
            ProjectileDirection::South if cell.rank_index() > 0 => Some(Cell::ALL[index - 1]),
            ProjectileDirection::West if cell.file_index() > 0 => Some(Cell::ALL[index - 4]),
            _ => None,
        }
    }

    fn spellcasters(&self, seat: Seat) -> Vec<(UnitKind, IdentityHash)> {
        let player = &self.position.players[seat_index(seat)];
        std::iter::once((UnitKind::Avatar, player.avatar.card.instance_id.clone()))
            .chain(self.position.units.iter().filter_map(|unit| {
                if unit.controller != seat || self.minion_is_disabled(unit) {
                    return None;
                }
                let CardFacts::Minion(facts) =
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts
                else {
                    return None;
                };
                (facts.spellcaster || self.minion_atop_tower(unit))
                    .then(|| (UnitKind::Minion, unit.card.instance_id.clone()))
            }))
            .collect()
    }

    fn spellcaster_kind(&self, seat: Seat, instance_id: &IdentityHash) -> Option<UnitKind> {
        let player = &self.position.players[seat_index(seat)];
        if player.avatar.card.instance_id == *instance_id {
            return Some(UnitKind::Avatar);
        }
        let unit = self.position.units.iter().find(|unit| {
            unit.controller == seat
                && unit.card.instance_id == *instance_id
                && !self.minion_is_disabled(unit)
        })?;
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return None;
        };
        (facts.spellcaster || self.minion_atop_tower(unit)).then_some(UnitKind::Minion)
    }

    fn append_artifact_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
        seat: Seat,
    ) -> Result<(), GameError> {
        let descriptors = self
            .artifact_cast_descriptors(seat)
            .into_iter()
            .chain(self.pick_up_artifact_descriptors(seat)?)
            .chain(self.drop_artifact_descriptors(seat)?)
            .chain(self.artifact_damage_descriptors(seat)?)
            .chain(self.artifact_discard_area_damage_descriptors(seat)?)
            .chain(self.artifact_roll_damage_descriptors(seat)?);
        for descriptor in descriptors {
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("artifact action requires a label"))?;
            self.push_action(actions, descriptor, label);
        }
        Ok(())
    }

    fn artifact_cast_descriptors(&self, seat: Seat) -> Vec<ActionDescriptor> {
        let player = &self.position.players[seat_index(seat)];
        let spellcasters = self.spellcasters(seat);
        let bearers = self.seat_unit_targets(seat);
        let cells: Vec<_> = self.controlled_site_cells(seat).collect();
        let mut descriptors = Vec::new();
        for card in &player.hand_spellbook {
            let definition = &self.rules.cards[usize::from(card.card_id.0)];
            let CardFacts::Artifact(facts) = &definition.facts else {
                continue;
            };
            if !artifact_effect_supported(facts.effect)
                || u64::from(player.mana) < facts.mana_cost
                || !self.thresholds_met(seat, facts.thresholds)
            {
                continue;
            }
            for (_, caster_instance_id) in &spellcasters {
                let conjure = |bearer, cell| ActionDescriptor::CastArtifact {
                    bearer,
                    bearer_cell: None,
                    card_id: definition.id.clone(),
                    card_instance_id: card.instance_id.clone(),
                    caster_instance_id: caster_instance_id.clone(),
                    cell,
                    mana_cost: facts.mana_cost,
                };
                descriptors.extend(cells.iter().map(|cell| conjure(None, Some(*cell))));
                descriptors.extend(
                    bearers
                        .iter()
                        .map(|bearer| conjure(Some(bearer.clone()), None)),
                );
            }
        }
        descriptors
    }

    fn pick_up_artifact_descriptors(&self, seat: Seat) -> Result<Vec<ActionDescriptor>, GameError> {
        let turn = Some(self.position.turn_number);
        let mut descriptors = Vec::new();
        for unit in self.seat_unit_targets(seat) {
            let turns = self.unit_target_artifact_turns(&unit)?;
            if turns.picked_up_artifacts == turn || self.unit_target_is_disabled(&unit)? {
                continue;
            }
            let region = self.unit_target_region(&unit)?;
            let cells = self.unit_target_occupied_cells(&unit)?.to_vec();
            for cell in &cells {
                let mut instance_ids = self.loose_artifact_instance_ids(*cell, region);
                instance_ids.sort_unstable();
                let count = instance_ids.len();
                descriptors.extend(
                    nonempty_identity_combinations(&instance_ids, count)
                        .into_iter()
                        .map(|artifact_instance_ids| ActionDescriptor::PickUpArtifacts {
                            artifact_instance_ids,
                            cell: (cells.len() > 1).then_some(*cell),
                            unit: unit.clone(),
                        }),
                );
            }
        }
        Ok(descriptors)
    }

    fn drop_artifact_descriptors(&self, seat: Seat) -> Result<Vec<ActionDescriptor>, GameError> {
        let turn = Some(self.position.turn_number);
        let mut descriptors = Vec::new();
        for unit in self.seat_unit_targets(seat) {
            let turns = self.unit_target_artifact_turns(&unit)?;
            if turns.dropped_artifacts == turn
                || turns.interacted == turn
                || self.unit_target_is_disabled(&unit)?
            {
                continue;
            }
            let mut instance_ids = self.carried_artifact_instance_ids(&unit);
            instance_ids.sort_unstable();
            let count = instance_ids.len();
            descriptors.extend(
                nonempty_identity_combinations(&instance_ids, count)
                    .into_iter()
                    .map(|artifact_instance_ids| ActionDescriptor::DropArtifacts {
                        artifact_instance_ids,
                        unit: unit.clone(),
                    }),
            );
        }
        Ok(descriptors)
    }

    /// Whether the seat may activate a carried Artifact ability at all right now.
    fn artifact_activation_window(&self, seat: Seat) -> bool {
        self.position.phase == Phase::Main
            && self.position.active_seat == seat
            && self.position.decision_seat == seat
    }

    /// The cell one carried Artifact fires from and every other ready ally standing there that can
    /// pay the second tap alongside its ready bearer.
    ///
    /// A loose Artifact has no bearer to tap, a spent bearer cannot pay the first tap, and a lone
    /// bearer has nobody to pay the second, so each of those yields nothing.
    fn artifact_tap_pair_helpers(
        &self,
        artifact: &ArtifactPosition,
        seat: Seat,
        effect: ArtifactEffect,
        allies: &[UnitTarget],
    ) -> Result<Option<(UnitTarget, Location, Vec<UnitTarget>)>, GameError> {
        let Some(bearer) = artifact.bearer() else {
            return Ok(None);
        };
        if bearer.seat() != seat
            || self.artifact_facts(artifact)?.effect != effect
            || !self.unit_target_is_ready(bearer)?
        {
            return Ok(None);
        }
        let carried_at = self.unit_target_location(bearer)?;
        let mut helpers = Vec::new();
        for helper in allies {
            if helper.instance_id() != bearer.instance_id()
                && self.unit_target_is_ready(helper)?
                && self.unit_target_region(helper)? == carried_at.region
                && self
                    .unit_target_occupied_cells(helper)?
                    .contains(&carried_at.cell)
            {
                helpers.push(helper.clone());
            }
        }
        Ok((!helpers.is_empty()).then(|| (bearer.clone(), carried_at, helpers)))
    }

    /// Offers each measured Artifact damage activation the seat can pay for right now.
    fn artifact_damage_descriptors(&self, seat: Seat) -> Result<Vec<ActionDescriptor>, GameError> {
        if !self.artifact_activation_window(seat) {
            return Ok(Vec::new());
        }
        let allies = self.seat_unit_targets(seat);
        let mut descriptors = Vec::new();
        for artifact in &self.position.artifacts {
            let Some((_, carried_at, helpers)) = self.artifact_tap_pair_helpers(
                artifact,
                seat,
                ArtifactEffect::TapBearerAndAnotherAllyHereToDamageTargetWithinTwoStepsThree,
                &allies,
            )?
            else {
                continue;
            };
            let reachable: BTreeSet<_> = self
                .locations_within_measured_steps(carried_at, 2)
                .into_iter()
                .map(|location| location.cell)
                .collect();
            let mut targets = Vec::new();
            for target in [Seat::North, Seat::South]
                .into_iter()
                .flat_map(|target_seat| self.seat_unit_targets(target_seat))
            {
                let hidden = target.seat() != seat
                    && self.combatant_stealthed(
                        unit_target_kind(&target),
                        target.seat(),
                        target.instance_id(),
                    )?;
                if hidden
                    || self.unit_target_region(&target)? != carried_at.region
                    || !self
                        .unit_target_occupied_cells(&target)?
                        .iter()
                        .any(|cell| reachable.contains(cell))
                {
                    continue;
                }
                targets.push(target);
            }
            for helper in &helpers {
                descriptors.extend(targets.iter().map(|target| {
                    ActionDescriptor::ActivateArtifactDamage {
                        artifact_instance_id: artifact.card.instance_id.clone(),
                        helper: helper.clone(),
                        target: target.clone(),
                    }
                }));
            }
        }
        Ok(descriptors)
    }

    /// Offers each measured Artifact area activation whose discard cost the seat can still pay.
    ///
    /// The chosen location is targeted, not the units on it, so the offer does not depend on who
    /// stands there and never skips a Stealthed occupant.
    fn artifact_discard_area_damage_descriptors(
        &self,
        seat: Seat,
    ) -> Result<Vec<ActionDescriptor>, GameError> {
        if !self.artifact_activation_window(seat) {
            return Ok(Vec::new());
        }
        let player = &self.position.players[seat_index(seat)];
        let discards: Vec<_> = [
            (DeckZone::Atlas, &player.hand_atlas),
            (DeckZone::Spellbook, &player.hand_spellbook),
        ]
        .into_iter()
        .flat_map(|(zone, hand)| {
            hand.iter()
                .map(move |card| (card.instance_id.clone(), zone))
        })
        .collect();
        if discards.is_empty() {
            return Ok(Vec::new());
        }
        let allies = self.seat_unit_targets(seat);
        let mut descriptors = Vec::new();
        for artifact in &self.position.artifacts {
            let Some((_, carried_at, helpers)) = self.artifact_tap_pair_helpers(
                artifact,
                seat,
                ArtifactEffect::TapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps,
                &allies,
            )?
            else {
                continue;
            };
            let target_locations = self.locations_within_measured_steps(carried_at, 3);
            for helper in &helpers {
                for (discard_card_instance_id, discard_zone) in &discards {
                    descriptors.extend(target_locations.iter().map(|target_location| {
                        ActionDescriptor::ActivateArtifactDiscardAreaDamage {
                            artifact_instance_id: artifact.card.instance_id.clone(),
                            discard_card_instance_id: discard_card_instance_id.clone(),
                            discard_zone: *discard_zone,
                            helper: helper.clone(),
                            target_location: *target_location,
                        }
                    }));
                }
            }
        }
        Ok(descriptors)
    }

    /// Offers each Rolling Boulder push the seat can pay for right now.
    ///
    /// Any ready co-located unit may tap to roll the Boulder maximally in one cardinal direction.
    /// No bearer is required, and the pusher is excluded from path damage.
    fn artifact_roll_damage_descriptors(
        &self,
        seat: Seat,
    ) -> Result<Vec<ActionDescriptor>, GameError> {
        if !self.artifact_activation_window(seat) {
            return Ok(Vec::new());
        }
        let pushers = self.seat_unit_targets(seat);
        let directions = [
            ProjectileDirection::East,
            ProjectileDirection::North,
            ProjectileDirection::South,
            ProjectileDirection::West,
        ];
        let mut descriptors = Vec::new();
        for artifact in &self.position.artifacts {
            if self.artifact_facts(artifact)?.effect
                != ArtifactEffect::TapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPathFour
            {
                continue;
            }
            let origin = self.artifact_location(artifact)?;
            for pusher in &pushers {
                if !self.unit_target_is_ready(pusher)?
                    || self.unit_target_region(pusher)? != origin.region
                    || !self
                        .unit_target_occupied_cells(pusher)?
                        .contains(&origin.cell)
                {
                    continue;
                }
                for direction in directions {
                    let path = self.maximal_roll_path(origin, direction);
                    descriptors.push(ActionDescriptor::ActivateArtifactRollDamage {
                        artifact_instance_id: artifact.card.instance_id.clone(),
                        direction,
                        path,
                        pusher: pusher.clone(),
                    });
                }
            }
        }
        Ok(descriptors)
    }

    /// The maximal cardinal roll path from one location without revisiting a cell or leaving the
    /// starting region.
    fn maximal_roll_path(&self, origin: Location, direction: ProjectileDirection) -> Vec<Location> {
        let mut path = vec![origin];
        let mut seen = BTreeSet::from([origin.cell]);
        loop {
            let location = *path.last().expect("roll path always starts at origin");
            let Some(next_cell) = Self::projectile_step(location.cell, direction) else {
                break;
            };
            if seen.contains(&next_cell)
                || !self.location_exists_in_region(next_cell, origin.region)
            {
                break;
            }
            path.push(Location {
                cell: next_cell,
                region: origin.region,
            });
            seen.insert(next_cell);
        }
        path
    }

    /// Whether a unit can still pay a tap cost: awake, untapped, and clear of summoning sickness.
    fn unit_target_is_ready(&self, target: &UnitTarget) -> Result<bool, GameError> {
        match target {
            UnitTarget::Avatar { instance_id, seat } => {
                let avatar = &self.position.players[seat_index(*seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                Ok(!avatar.tapped)
            }
            UnitTarget::Minion { instance_id, seat } => self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                .map(|unit| {
                    !unit.tapped
                        && !self.minion_is_disabled(unit)
                        && (!unit.summoning_sickness || self.minion_has_active_charge(unit))
                })
                .ok_or(GameError::IllegalAction),
        }
    }

    fn tap_unit_target(&mut self, target: &UnitTarget) -> Result<(), GameError> {
        match target {
            UnitTarget::Avatar { instance_id, seat } => {
                let avatar = &mut self.position.players[seat_index(*seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                avatar.tapped = true;
            }
            UnitTarget::Minion { instance_id, seat } => {
                self.position
                    .units
                    .iter_mut()
                    .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                    .ok_or(GameError::IllegalAction)?
                    .tapped = true;
            }
        }
        Ok(())
    }

    fn loose_artifact_instance_ids(&self, cell: Cell, region: Region) -> Vec<IdentityHash> {
        self.position
            .artifacts
            .iter()
            .filter(|artifact| {
                matches!(
                    artifact.placement,
                    ArtifactPlacement::Loose {
                        location,
                        region: artifact_region,
                    } if location == cell && artifact_region == region
                )
            })
            .map(|artifact| artifact.card.instance_id.clone())
            .collect()
    }

    fn carried_artifact_instance_ids(&self, bearer: &UnitTarget) -> Vec<IdentityHash> {
        let kind = unit_target_kind(bearer);
        self.position
            .artifacts
            .iter()
            .filter(|artifact| artifact.carried_by(kind, bearer.seat(), bearer.instance_id()))
            .map(|artifact| artifact.card.instance_id.clone())
            .collect()
    }

    /// Every unit the seat controls, Avatar first, mirroring the authoritative unit reference order.
    fn seat_unit_targets(&self, seat: Seat) -> Vec<UnitTarget> {
        std::iter::once(UnitTarget::Avatar {
            instance_id: self.position.players[seat_index(seat)]
                .avatar
                .card
                .instance_id
                .clone(),
            seat,
        })
        .chain(
            self.position
                .units
                .iter()
                .filter(|unit| unit.controller == seat)
                .map(|unit| UnitTarget::Minion {
                    instance_id: unit.card.instance_id.clone(),
                    seat,
                }),
        )
        .collect()
    }

    fn artifact_facts(&self, artifact: &ArtifactPosition) -> Result<ArtifactFacts, GameError> {
        let CardFacts::Artifact(facts) =
            &self.rules.cards[usize::from(artifact.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        Ok(*facts)
    }

    /// Where an Artifact currently sits: the cell its bearer stands on, or the cell it lies on.
    fn artifact_location(&self, artifact: &ArtifactPosition) -> Result<Location, GameError> {
        match &artifact.placement {
            ArtifactPlacement::Carried { bearer } => self.unit_target_location(bearer),
            ArtifactPlacement::Loose { location, region } => Ok(Location {
                cell: *location,
                region: *region,
            }),
        }
    }

    /// Total power the unit gains from the Artifacts it carries.
    fn carried_power_bonus(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<u16, GameError> {
        let mut bonus = 0_u16;
        for artifact in &self.position.artifacts {
            if !artifact.carried_by(kind, seat, instance_id) {
                continue;
            }
            if self.artifact_facts(artifact)?.effect == ArtifactEffect::GrantsBearerPowerTwo {
                bonus = bonus.checked_add(2).ok_or(GameError::IllegalAction)?;
            }
        }
        Ok(bonus)
    }

    /// Whether any carried Artifact grants the unit Lethal.
    fn carried_lethal(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<bool, GameError> {
        for artifact in &self.position.artifacts {
            if artifact.carried_by(kind, seat, instance_id)
                && self.artifact_facts(artifact)?.effect == ArtifactEffect::GrantsBearerLethal
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn unit_target_is_disabled(&self, target: &UnitTarget) -> Result<bool, GameError> {
        match target {
            UnitTarget::Avatar { .. } => Ok(false),
            UnitTarget::Minion { instance_id, seat } => self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                .map(|unit| self.minion_is_disabled(unit))
                .ok_or(GameError::IllegalAction),
        }
    }

    /// The turns on which this unit last picked up, dropped, and otherwise interacted.
    fn unit_target_artifact_turns(&self, target: &UnitTarget) -> Result<UnitTurns, GameError> {
        match target {
            UnitTarget::Avatar { instance_id, seat } => {
                let avatar = &self.position.players[seat_index(*seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                Ok(UnitTurns {
                    dropped_artifacts: avatar.last_dropped_artifacts_turn,
                    interacted: avatar.last_interacted_turn,
                    picked_up_artifacts: avatar.last_picked_up_artifacts_turn,
                })
            }
            UnitTarget::Minion { instance_id, seat } => self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                .map(|unit| UnitTurns {
                    dropped_artifacts: unit.last_dropped_artifacts_turn,
                    interacted: unit.last_interacted_turn,
                    picked_up_artifacts: unit.last_picked_up_artifacts_turn,
                })
                .ok_or(GameError::IllegalAction),
        }
    }

    fn minion_is_disabled(&self, unit: &UnitPosition) -> bool {
        if unit.disabled_until_damaged || !unit.disable_effects.is_empty() {
            return true;
        }
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return true;
        };
        facts.waterbound
            && !self.position.sites[unit.location.index()]
                .as_ref()
                .and_then(|site| {
                    let CardFacts::Site(facts) =
                        &self.rules.cards[usize::from(site.card.card_id.0)].facts
                    else {
                        return None;
                    };
                    Some(facts.elements.contains(Element::Water))
                })
                .unwrap_or(false)
    }

    fn minion_has_active_stealth(&self, unit: &UnitPosition) -> bool {
        unit.stealthed && !self.minion_is_disabled(unit)
    }

    /// Reports whether an active surface minion stands on a Tower it can draw from.
    fn minion_atop_tower(&self, unit: &UnitPosition) -> bool {
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return false;
        };
        facts.gains_power_ranged_and_spellcaster_atop_tower
            && unit.region == Region::Surface
            && !self.minion_is_disabled(unit)
            && self.position.sites[unit.location.index()]
                .as_ref()
                .is_some_and(|site| {
                    matches!(
                        &self.rules.cards[usize::from(site.card.card_id.0)].facts,
                        CardFacts::Site(site_facts) if site_facts.is_tower
                    )
                })
    }

    fn minion_is_ranged(&self, unit: &UnitPosition, facts: &MinionFacts) -> bool {
        facts.ranged || self.minion_atop_tower(unit)
    }

    fn has_nearby_enemy_minion(&self, unit: &UnitPosition) -> bool {
        self.position.units.iter().any(|enemy| {
            enemy.controller != unit.controller
                && enemy.region == unit.region
                && Self::footprints_nearby(
                    Self::unit_occupied_cells(unit),
                    Self::unit_occupied_cells(enemy),
                )
        })
    }

    fn unit_occupied_cells(unit: &UnitPosition) -> &[Cell] {
        unit.occupied_cells.as_ref().map_or_else(
            || std::slice::from_ref(&unit.location),
            |area| area.as_slice(),
        )
    }

    fn unit_occupies_cell(unit: &UnitPosition, cell: Cell) -> bool {
        Self::unit_occupied_cells(unit).contains(&cell)
    }

    fn footprints_nearby(source: &[Cell], target: &[Cell]) -> bool {
        source.iter().any(|source_cell| {
            target.contains(source_cell)
                || source_cell
                    .bordering(false)
                    .chain(source_cell.diagonals(false))
                    .any(|cell| target.contains(&cell))
        })
    }

    fn footprints_here_or_bordering(source: &[Cell], target: &[Cell]) -> bool {
        source.iter().any(|source_cell| {
            target.contains(source_cell)
                || source_cell
                    .bordering(false)
                    .any(|cell| target.contains(&cell))
        })
    }

    fn chain_magic_targets(
        &self,
        seat: Seat,
        caster: &UnitTarget,
        previous: &UnitTarget,
        selected: &[UnitTarget],
    ) -> Result<Vec<UnitTarget>, GameError> {
        let caster_region = self.unit_target_region(caster)?;
        let previous_kind = match previous {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        let previous_cells =
            self.combatant_occupied_cells(previous_kind, previous.seat(), previous.instance_id())?;
        let mut targets = Vec::new();
        for target_seat in [Seat::North, Seat::South] {
            let avatar = &self.position.players[seat_index(target_seat)].avatar;
            let target = UnitTarget::Avatar {
                instance_id: avatar.card.instance_id.clone(),
                seat: target_seat,
            };
            if caster_region == Region::Surface
                && !selected
                    .iter()
                    .any(|chosen| chosen.instance_id() == target.instance_id())
                && Self::footprints_nearby(previous_cells, std::slice::from_ref(&avatar.location))
            {
                targets.push(target);
            }
        }
        targets.extend(self.position.units.iter().filter_map(|unit| {
            if unit.region != caster_region
                || selected
                    .iter()
                    .any(|chosen| chosen.instance_id() == &unit.card.instance_id)
                || (unit.controller != seat && self.minion_has_active_stealth(unit))
                || !Self::footprints_nearby(previous_cells, Self::unit_occupied_cells(unit))
            {
                return None;
            }
            Some(UnitTarget::Minion {
                instance_id: unit.card.instance_id.clone(),
                seat: unit.controller,
            })
        }));
        targets.sort_unstable_by(|left, right| left.instance_id().cmp(right.instance_id()));
        Ok(targets)
    }

    fn unit_target_region(&self, target: &UnitTarget) -> Result<Region, GameError> {
        match target {
            UnitTarget::Avatar { instance_id, seat } => (self.position.players[seat_index(*seat)]
                .avatar
                .card
                .instance_id
                == *instance_id)
                .then_some(Region::Surface)
                .ok_or(GameError::IllegalAction),
            UnitTarget::Minion { instance_id, seat } => self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                .map(|unit| unit.region)
                .ok_or(GameError::IllegalAction),
        }
    }

    fn unit_target_occupied_cells(&self, target: &UnitTarget) -> Result<&[Cell], GameError> {
        let kind = match target {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        self.combatant_occupied_cells(kind, target.seat(), target.instance_id())
    }

    fn combatant_occupied_cells(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<&[Cell], GameError> {
        match kind {
            UnitKind::Avatar => {
                let avatar = &self.position.players[seat_index(seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                Ok(std::slice::from_ref(&avatar.location))
            }
            UnitKind::Minion => self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == seat && unit.card.instance_id == *instance_id)
                .map(Self::unit_occupied_cells)
                .ok_or(GameError::IllegalAction),
        }
    }

    fn move_minion_to(unit: &mut UnitPosition, cell: Cell) -> Result<(), GameError> {
        if let Some(area) = unit.occupied_cells {
            unit.occupied_cells =
                Some(translated_square(area, unit.location, cell).ok_or(GameError::IllegalAction)?);
        }
        unit.location = cell;
        Ok(())
    }

    fn settle_nearby_enemy_stealth(&mut self, outcomes: &mut OutcomeLog<'_>) {
        if self.position.terminal.is_some() {
            return;
        }
        // ponytail: bounded O(units²) scan keeps speculative nodes allocation-free; index only
        // if profiling shows dense Scent Hound positions are a bottleneck.
        let mut previous_source: Option<usize> = None;
        loop {
            let source_index = self
                .position
                .units
                .iter()
                .enumerate()
                .filter(|(_, source)| {
                    if self.minion_is_disabled(source) {
                        return false;
                    }
                    let CardFacts::Minion(facts) =
                        &self.rules.cards[usize::from(source.card.card_id.0)].facts
                    else {
                        return false;
                    };
                    facts.nearby_enemies_permanently_lose_stealth
                        && previous_source.is_none_or(|previous_index| {
                            source.card.instance_id
                                > self.position.units[previous_index].card.instance_id
                        })
                })
                .min_by(|(_, left), (_, right)| left.card.instance_id.cmp(&right.card.instance_id))
                .map(|(index, _)| index);
            let Some(source_index) = source_index else {
                break;
            };
            let source_controller = self.position.units[source_index].controller;
            let source_location = self.position.units[source_index].location;
            let source_area = self.position.units[source_index].occupied_cells;
            let source_region = self.position.units[source_index].region;
            let source_singleton = [source_location];
            let source_cells = source_area
                .as_ref()
                .map_or(source_singleton.as_slice(), |area| area.as_slice());
            for target_index in 0..self.position.units.len() {
                let target = &self.position.units[target_index];
                if target.controller == source_controller
                    || target.region != source_region
                    || !target.stealthed
                    || !Self::footprints_nearby(source_cells, Self::unit_occupied_cells(target))
                {
                    continue;
                }
                let event = matches!(outcomes, OutcomeLog::Record(_)).then(|| {
                    (
                        target.card.instance_id.clone(),
                        target.controller,
                        self.position.units[source_index].card.instance_id.clone(),
                    )
                });
                self.position.units[target_index].stealthed = false;
                if let Some((instance_id, controller, source_instance_id)) = event {
                    outcomes.push("stealth-lost", || {
                        json!({
                            "instanceId": instance_id,
                            "seat": controller,
                            "sourceInstanceId": source_instance_id,
                        })
                    });
                }
            }
            previous_source = Some(source_index);
        }
    }

    fn minion_current_stats(&self, unit: &UnitPosition) -> Result<(u16, u16, bool), GameError> {
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let disabled = self.minion_is_disabled(unit);
        let nearby = |source: &UnitPosition| {
            source.region == unit.region
                && Self::footprints_nearby(
                    Self::unit_occupied_cells(source),
                    Self::unit_occupied_cells(unit),
                )
        };
        let mut bonus = Self::temporary_power_bonus(&unit.temporary_power_sources)?;
        for source in &self.position.units {
            if source.card.instance_id == unit.card.instance_id
                || source.controller != unit.controller
                || self.minion_is_disabled(source)
            {
                continue;
            }
            let CardFacts::Minion(source_facts) =
                &self.rules.cards[usize::from(source.card.card_id.0)].facts
            else {
                return Err(GameError::IllegalAction);
            };
            if source_facts.other_nearby_allies_power_bonus && nearby(source) {
                bonus = bonus.checked_add(1).ok_or(GameError::IllegalAction)?;
            }
            if facts.mortal && source_facts.other_controlled_mortals_power_bonus {
                bonus = bonus.checked_add(1).ok_or(GameError::IllegalAction)?;
            }
        }
        if self.minion_atop_tower(unit) {
            bonus = bonus.checked_add(2).ok_or(GameError::IllegalAction)?;
        }
        bonus = bonus
            .checked_add(self.carried_power_bonus(
                UnitKind::Minion,
                unit.controller,
                &unit.card.instance_id,
            )?)
            .ok_or(GameError::IllegalAction)?;
        let lethal = facts.lethal
            || self.carried_lethal(UnitKind::Minion, unit.controller, &unit.card.instance_id)?;
        Ok((
            u16::from(facts.attack)
                .checked_add(bonus)
                .ok_or(GameError::IllegalAction)?,
            u16::from(facts.defense)
                .checked_add(bonus)
                .ok_or(GameError::IllegalAction)?,
            !disabled && lethal,
        ))
    }

    fn temporary_power_bonus(sources: &[IdentityHash]) -> Result<u16, GameError> {
        u16::try_from(sources.len())
            .map_err(|_| GameError::IllegalAction)?
            .checked_mul(2)
            .ok_or(GameError::IllegalAction)
    }

    fn minion_caster_suffix(&self, seat: Seat, instance_id: &IdentityHash) -> String {
        if self.spellcaster_kind(seat, instance_id) == Some(UnitKind::Minion) {
            format!(" with minion {}…", &instance_id.as_str()[..15])
        } else {
            String::new()
        }
    }

    fn spellcaster_location(
        &self,
        seat: Seat,
        caster_instance_id: &IdentityHash,
    ) -> Result<Location, GameError> {
        let player = &self.position.players[seat_index(seat)];
        if player.avatar.card.instance_id == *caster_instance_id {
            return Ok(Location {
                cell: player.avatar.location,
                region: Region::Surface,
            });
        }
        self.position
            .units
            .iter()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *caster_instance_id)
            .map(|unit| Location {
                cell: unit.location,
                region: unit.region,
            })
            .ok_or(GameError::IllegalAction)
    }

    fn targeted_magic_choices(
        &self,
        seat: Seat,
        caster_instance_id: &IdentityHash,
        target_nearby: bool,
        minion_only: bool,
    ) -> Result<Vec<MagicChoice>, GameError> {
        let caster_location = self.spellcaster_location(seat, caster_instance_id)?;
        let caster_cells = [caster_location.cell];
        let mut targets = Vec::new();
        for target_seat in [Seat::North, Seat::South] {
            let player = &self.position.players[seat_index(target_seat)];
            if !minion_only
                && caster_location.region == Region::Surface
                && (!target_nearby
                    || Self::footprints_nearby(
                        &caster_cells,
                        std::slice::from_ref(&player.avatar.location),
                    ))
            {
                targets.push(MagicChoice {
                    target: Some(UnitTarget::Avatar {
                        instance_id: player.avatar.card.instance_id.clone(),
                        seat: target_seat,
                    }),
                    ..MagicChoice::default()
                });
            }
            targets.extend(
                self.position
                    .units
                    .iter()
                    .filter(|unit| {
                        unit.controller == target_seat
                            && unit.region == caster_location.region
                            && (target_seat == seat || !self.minion_has_active_stealth(unit))
                            && (!target_nearby
                                || Self::footprints_nearby(
                                    &caster_cells,
                                    Self::unit_occupied_cells(unit),
                                ))
                    })
                    .map(|unit| MagicChoice {
                        target: Some(UnitTarget::Minion {
                            instance_id: unit.card.instance_id.clone(),
                            seat: target_seat,
                        }),
                        ..MagicChoice::default()
                    }),
            );
        }
        Ok(targets)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one closed match keeps every supported Magic choice shape explicit"
    )]
    fn magic_choices(
        &self,
        seat: Seat,
        caster_instance_id: &IdentityHash,
        effect: &MagicEffect,
    ) -> Result<Vec<MagicChoice>, GameError> {
        Ok(match effect {
            MagicEffect::HealController(_)
            | MagicEffect::DamageEachAbovegroundMinionOne
            | MagicEffect::SummonRandomMinionFromAnyCemetery
            | MagicEffect::SummonTokenToEachControlledSiteBorderingEnemySite(_) => {
                vec![MagicChoice::default()]
            }
            MagicEffect::GrantChargeToAllyThisTurn | MagicEffect::GrantPowerTwoToAllyThisTurn => {
                self.controlled_allies(seat)
                    .into_iter()
                    .map(|ally| MagicChoice {
                        ally: Some(ally),
                        ..MagicChoice::default()
                    })
                    .collect()
            }
            MagicEffect::LeapAttackAlly => self.leap_attack_choices(seat)?,
            MagicEffect::TeleportAllyToTargetSite => {
                if self.spellcaster_location(seat, caster_instance_id)?.region != Region::Surface {
                    return Ok(Vec::new());
                }
                // Teleport bypasses the entry gates that only restrict deliberate movement.
                let destinations: Vec<(Location, IdentityHash)> = Cell::ALL
                    .into_iter()
                    .filter_map(|cell| {
                        let instance_id = self.position.sites[cell.index()]
                            .as_ref()
                            .map(|site| &site.card.instance_id)
                            .or(self.position.rubble[cell.index()].as_ref())?
                            .clone();
                        Some((
                            Location {
                                cell,
                                region: Region::Surface,
                            },
                            instance_id,
                        ))
                    })
                    .collect();
                let mut choices = Vec::with_capacity(destinations.len());
                for ally in self.controlled_allies(seat) {
                    if self.unit_target_occupied_cells(&ally)?.len() > 1 {
                        return Err(GameError::UnsupportedManifestFact(
                            "teleportAllyToTargetSite:occupiesSquareArea".to_owned(),
                        ));
                    }
                    for (target_location, target_site_instance_id) in &destinations {
                        choices.push(MagicChoice {
                            ally: Some(ally.clone()),
                            target_location: Some(*target_location),
                            target_site_instance_id: Some(target_site_instance_id.clone()),
                            ..MagicChoice::default()
                        });
                    }
                }
                choices
            }
            MagicEffect::TeleportNearbyAllyThenDrawCard => {
                let mut choices = Vec::new();
                for ally in self.controlled_allies(seat) {
                    if self.unit_target_occupied_cells(&ally)?.len() > 1 {
                        return Err(GameError::UnsupportedManifestFact(
                            "teleportNearbyAllyThenDrawCard:occupiesSquareArea".to_owned(),
                        ));
                    }
                    let from = self.unit_target_location(&ally)?;
                    // Blink relocates without a deliberate step, so only the layer must exist.
                    let destinations = std::iter::once(from.cell)
                        .chain(from.cell.bordering(false))
                        .chain(from.cell.diagonals(false))
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .filter(|cell| self.location_exists_in_region(*cell, from.region));
                    for cell in destinations {
                        for zone in [DeckZone::Atlas, DeckZone::Spellbook] {
                            choices.push(MagicChoice {
                                ally: Some(ally.clone()),
                                draw_zone: Some(zone),
                                target_location: Some(Location {
                                    cell,
                                    region: from.region,
                                }),
                                target_site_instance_id: self.position.sites[cell.index()]
                                    .as_ref()
                                    .map(|site| site.card.instance_id.clone())
                                    .or_else(|| self.position.rubble[cell.index()].clone()),
                                ..MagicChoice::default()
                            });
                        }
                    }
                }
                choices
            }
            MagicEffect::LureEnemyMinionOneStepCloser => {
                let enemy_seat = other_seat(seat);
                let tempted: Vec<UnitTarget> = self
                    .position
                    .units
                    .iter()
                    .filter(|unit| {
                        unit.controller == enemy_seat
                            && unit.region != Region::Void
                            && !self.minion_is_disabled(unit)
                            && Self::unit_occupied_cells(unit)
                                .iter()
                                .any(|cell| self.surface_location_exists(*cell))
                    })
                    .map(|unit| UnitTarget::Minion {
                        instance_id: unit.card.instance_id.clone(),
                        seat: enemy_seat,
                    })
                    .collect();
                let mut choices = Vec::new();
                for ally in self.controlled_allies(seat) {
                    let ally_cells = self.unit_target_occupied_cells(&ally)?;
                    for enemy in &tempted {
                        let enemy_cells = self.unit_target_occupied_cells(enemy)?;
                        if !Self::footprints_nearby(ally_cells, enemy_cells) {
                            continue;
                        }
                        let from = self.unit_target_location(enemy)?;
                        let starting_distance = minimum_cardinal_distance(enemy_cells, ally_cells);
                        let area = self
                            .position
                            .units
                            .iter()
                            .find(|unit| unit.card.instance_id == *enemy.instance_id())
                            .and_then(|unit| unit.occupied_cells);
                        for destination in self.card_effect_step_destinations(enemy, from)? {
                            let destination_cells = match area {
                                Some(area) => translated_square(area, from.cell, destination.cell)
                                    .ok_or(GameError::IllegalAction)?
                                    .to_vec(),
                                None => vec![destination.cell],
                            };
                            if minimum_cardinal_distance(&destination_cells, ally_cells)
                                >= starting_distance
                            {
                                continue;
                            }
                            choices.push(MagicChoice {
                                ally: Some(ally.clone()),
                                tempted_destination: Some(destination),
                                tempted_enemy: Some(enemy.clone()),
                                ..MagicChoice::default()
                            });
                        }
                    }
                }
                if choices.is_empty() {
                    vec![MagicChoice::default()]
                } else {
                    choices
                }
            }
            MagicEffect::FightAllyWithAdjacentEnemy => {
                let unit_targets = |target_seat| {
                    let player = &self.position.players[seat_index(target_seat)];
                    std::iter::once(UnitTarget::Avatar {
                        instance_id: player.avatar.card.instance_id.clone(),
                        seat: target_seat,
                    })
                    .chain(
                        self.position
                            .units
                            .iter()
                            .filter(move |unit| unit.controller == target_seat)
                            .map(move |unit| UnitTarget::Minion {
                                instance_id: unit.card.instance_id.clone(),
                                seat: target_seat,
                            }),
                    )
                    .collect::<Vec<_>>()
                };
                let allies = unit_targets(seat);
                let enemies = unit_targets(other_seat(seat));
                let mut choices = Vec::new();
                // ponytail: the realm is bounded; index only if Duel enumeration profiles hot.
                for ally in allies {
                    let ally_kind = match ally {
                        UnitTarget::Avatar { .. } => UnitKind::Avatar,
                        UnitTarget::Minion { .. } => UnitKind::Minion,
                    };
                    let ally_region = self.unit_target_region(&ally)?;
                    let ally_cells =
                        self.combatant_occupied_cells(ally_kind, ally.seat(), ally.instance_id())?;
                    for target in &enemies {
                        let target_kind = match target {
                            UnitTarget::Avatar { .. } => UnitKind::Avatar,
                            UnitTarget::Minion { .. } => UnitKind::Minion,
                        };
                        if self.unit_target_region(target)? != ally_region
                            || !Self::footprints_here_or_bordering(
                                ally_cells,
                                self.combatant_occupied_cells(
                                    target_kind,
                                    target.seat(),
                                    target.instance_id(),
                                )?,
                            )
                        {
                            continue;
                        }
                        if let UnitTarget::Minion { instance_id, .. } = target {
                            let unit = self
                                .position
                                .units
                                .iter()
                                .find(|unit| unit.card.instance_id == *instance_id)
                                .ok_or(GameError::IllegalAction)?;
                            if self.minion_has_active_stealth(unit) {
                                continue;
                            }
                        }
                        choices.push(MagicChoice {
                            ally: Some(ally.clone()),
                            target: Some(target.clone()),
                            ..MagicChoice::default()
                        });
                    }
                }
                choices
            }
            MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(_) => self
                .locations_within_measured_steps(
                    self.spellcaster_location(seat, caster_instance_id)?,
                    2,
                )
                .into_iter()
                .map(|target_location| MagicChoice {
                    target_location: Some(target_location),
                    ..MagicChoice::default()
                })
                .collect(),
            MagicEffect::DamageRandomUnitAtLocation(_) => {
                let region = self.spellcaster_location(seat, caster_instance_id)?.region;
                Cell::ALL
                    .into_iter()
                    .filter(|cell| self.location_exists_in_region(*cell, region))
                    .map(|cell| MagicChoice {
                        target_location: Some(Location { cell, region }),
                        ..MagicChoice::default()
                    })
                    .collect()
            }
            MagicEffect::ReturnMinionFromOwnCemetery => {
                let choices: Vec<_> = self.position.players[seat_index(seat)]
                    .cemetery
                    .iter()
                    .filter(|card| {
                        matches!(
                            self.rules.cards[usize::from(card.card_id.0)].facts,
                            CardFacts::Minion(_)
                        )
                    })
                    .map(|card| MagicChoice {
                        cemetery_minion_instance_id: Some(card.instance_id.clone()),
                        ..MagicChoice::default()
                    })
                    .collect();
                if choices.is_empty() {
                    vec![MagicChoice::default()]
                } else {
                    choices
                }
            }
            MagicEffect::DamageTargetUnit {
                target_nearby,
                untap_target_minion_after_damage,
                ..
            } => self.targeted_magic_choices(
                seat,
                caster_instance_id,
                *target_nearby,
                *untap_target_minion_after_damage,
            )?,
            MagicEffect::BurrowTargetMinionOrArtifact | MagicEffect::SubmergeTargetMinion => {
                self.targeted_magic_choices(seat, caster_instance_id, false, true)?
            }
            MagicEffect::GainControlOfTargetNearbyMinion => {
                self.targeted_magic_choices(seat, caster_instance_id, true, true)?
            }
            MagicEffect::KillTargetWoundedMinion => self
                .targeted_magic_choices(seat, caster_instance_id, false, true)?
                .into_iter()
                .filter(|choice| {
                    choice
                        .target
                        .as_ref()
                        .and_then(|target| {
                            self.position
                                .units
                                .iter()
                                .find(|unit| unit.card.instance_id == *target.instance_id())
                        })
                        .is_some_and(|unit| unit.damage > 0)
                })
                .collect(),
            MagicEffect::BurrowAllMinionsAndArtifactsAtTargetLandSite => {
                if self.spellcaster_location(seat, caster_instance_id)?.region == Region::Surface {
                    Cell::ALL
                        .into_iter()
                        .filter(|cell| self.underground_location_exists(*cell))
                        .filter_map(|cell| {
                            let instance_id = self.position.sites[cell.index()]
                                .as_ref()
                                .map(|site| &site.card.instance_id)
                                .or(self.position.rubble[cell.index()].as_ref())?
                                .clone();
                            Some(MagicChoice {
                                target_location: Some(Location {
                                    cell,
                                    region: Region::Surface,
                                }),
                                target_site_instance_id: Some(instance_id),
                                ..MagicChoice::default()
                            })
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            MagicEffect::DestroyTargetSiteWithDamageGrid(_) => {
                let caster_location = self.spellcaster_location(seat, caster_instance_id)?;
                if caster_location.region == Region::Void {
                    return Ok(Vec::new());
                }
                let mut discard_site_instance_ids: Vec<_> = self.position.players[seat_index(seat)]
                    .hand_atlas
                    .iter()
                    .map(|card| card.instance_id.clone())
                    .collect();
                discard_site_instance_ids.sort_unstable();
                let mut choices = Vec::new();
                for cell in Cell::ALL {
                    let Some(target_site_instance_id) = self.position.sites[cell.index()]
                        .as_ref()
                        .map(|site| &site.card.instance_id)
                        .or(self.position.rubble[cell.index()].as_ref())
                    else {
                        continue;
                    };
                    // A subsurface caster only reaches the layer its own region occupies.
                    let water = self.location_exists_in_region(cell, Region::Underwater);
                    if caster_location.region == Region::Underwater && !water
                        || caster_location.region == Region::Underground && water
                    {
                        continue;
                    }
                    for discard_site_instance_id in &discard_site_instance_ids {
                        choices.push(MagicChoice {
                            discard_site_instance_id: Some(discard_site_instance_id.clone()),
                            target_location: Some(Location {
                                cell,
                                region: caster_location.region,
                            }),
                            target_site_instance_id: Some(target_site_instance_id.clone()),
                            ..MagicChoice::default()
                        });
                    }
                }
                choices
            }
            MagicEffect::DisableTargetNearbyMinionUntilNextTurn => {
                let caster_location = self.spellcaster_location(seat, caster_instance_id)?;
                let caster_cells = [caster_location.cell];
                let mut targets: Vec<_> = self
                    .position
                    .units
                    .iter()
                    .filter(|unit| {
                        unit.region == caster_location.region
                            && (unit.controller == seat || !self.minion_has_active_stealth(unit))
                            && Self::footprints_nearby(
                                &caster_cells,
                                Self::unit_occupied_cells(unit),
                            )
                    })
                    .map(|unit| MagicChoice {
                        target: Some(UnitTarget::Minion {
                            instance_id: unit.card.instance_id.clone(),
                            seat: unit.controller,
                        }),
                        ..MagicChoice::default()
                    })
                    .collect();
                targets.sort_unstable_by(|left, right| {
                    left.target
                        .as_ref()
                        .expect("Freeze target")
                        .instance_id()
                        .cmp(right.target.as_ref().expect("Freeze target").instance_id())
                });
                targets
            }
            // Chained Magic enumerates its targets through its own pending decision instead.
            MagicEffect::DamageChainNearbyUnits => {
                return Err(GameError::UnsupportedManifestFact(
                    unsupported_magic_effect(effect)
                        .unwrap_or("Magic effect")
                        .to_owned(),
                ));
            }
        })
    }

    fn leap_attack_choices(&self, seat: Seat) -> Result<Vec<MagicChoice>, GameError> {
        let mut choices = Vec::new();
        for ally in self.controlled_allies(seat) {
            let from = self.unit_target_location(&ally)?;
            let occupied = self.combatant_occupied_cells(
                match ally {
                    UnitTarget::Avatar { .. } => UnitKind::Avatar,
                    UnitTarget::Minion { .. } => UnitKind::Minion,
                },
                ally.seat(),
                ally.instance_id(),
            )?;
            let destinations = self.card_effect_step_destinations(&ally, from)?;
            for destination in destinations {
                if occupied.len() == 1 {
                    choices.push(MagicChoice {
                        ally: Some(ally.clone()),
                        ally_destination: Some(destination),
                        ..MagicChoice::default()
                    });
                    continue;
                }
                let area = self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == *ally.instance_id())
                    .and_then(|unit| unit.occupied_cells)
                    .ok_or(GameError::IllegalAction)?;
                let destination_cells = translated_square(area, from.cell, destination.cell)
                    .ok_or(GameError::IllegalAction)?;
                for cell in destination_cells {
                    choices.push(MagicChoice {
                        ally: Some(ally.clone()),
                        ally_destination: Some(destination),
                        ally_strike_location: Some(Location {
                            cell,
                            region: destination.region,
                        }),
                        ..MagicChoice::default()
                    });
                }
            }
        }
        Ok(choices)
    }

    fn controlled_allies(&self, seat: Seat) -> Vec<UnitTarget> {
        let player = &self.position.players[seat_index(seat)];
        std::iter::once(UnitTarget::Avatar {
            instance_id: player.avatar.card.instance_id.clone(),
            seat,
        })
        .chain(
            self.position
                .units
                .iter()
                .filter(move |unit| unit.controller == seat)
                .map(move |unit| UnitTarget::Minion {
                    instance_id: unit.card.instance_id.clone(),
                    seat,
                }),
        )
        .collect()
    }

    fn card_effect_step_destinations(
        &self,
        ally: &UnitTarget,
        from: Location,
    ) -> Result<Vec<Location>, GameError> {
        let (disabled, immobile, profile) = match ally {
            UnitTarget::Avatar { instance_id, seat } => {
                let avatar = &self.position.players[seat_index(*seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                (
                    false,
                    false,
                    MovementProfile {
                        airborne: false,
                        cause: MovementCause::CardEffect,
                        connects_top_bottom: false,
                        maximum_cost: Some(1),
                        moving_minion: false,
                        occupied_cells: None,
                        regions: RegionAbilities::default(),
                        restriction: None,
                        seat: *seat,
                    },
                )
            }
            UnitTarget::Minion { instance_id, seat } => {
                let unit = self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                    .ok_or(GameError::IllegalAction)?;
                let CardFacts::Minion(facts) =
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts
                else {
                    return Err(GameError::IllegalAction);
                };
                let disabled = self.minion_is_disabled(unit);
                (
                    disabled,
                    facts.immobile,
                    MovementProfile {
                        airborne: facts.airborne,
                        cause: MovementCause::CardEffect,
                        connects_top_bottom: facts.connects_top_bottom,
                        maximum_cost: Some(1),
                        moving_minion: true,
                        occupied_cells: unit.occupied_cells,
                        regions: RegionAbilities::of_minion(facts),
                        restriction: facts.movement_restriction,
                        seat: *seat,
                    },
                )
            }
        };
        if disabled || immobile {
            return Ok(vec![from]);
        }
        let mut destinations = Vec::new();
        let mut seen = BTreeSet::new();
        for path in self.movement_paths(from, profile) {
            let Some(&destination) = path.last() else {
                continue;
            };
            if seen.insert((destination.cell, destination.region)) {
                destinations.push(destination);
            }
        }
        if destinations.is_empty() {
            destinations.push(from);
        }
        Ok(destinations)
    }

    fn unit_target_location(&self, target: &UnitTarget) -> Result<Location, GameError> {
        match target {
            UnitTarget::Avatar { instance_id, seat } => {
                let avatar = &self.position.players[seat_index(*seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                Ok(Location {
                    cell: avatar.location,
                    region: Region::Surface,
                })
            }
            UnitTarget::Minion { instance_id, seat } => self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                .map(|unit| Location {
                    cell: unit.location,
                    region: unit.region,
                })
                .ok_or(GameError::IllegalAction),
        }
    }

    fn append_unit_move_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
        instance_id: &IdentityHash,
        start: Cell,
        profile: MovementProfile,
    ) -> Result<(), GameError> {
        for path in self.movement_paths(
            Location {
                cell: start,
                region: Region::Surface,
            },
            profile,
        ) {
            let from = path[0];
            let to = *path
                .last()
                .ok_or_else(|| invalid("movement path is empty"))?;
            let descriptor = ActionDescriptor::MoveAndAttack {
                from,
                path,
                to,
                unit_instance_id: instance_id.clone(),
            };
            let label = descriptor
                .state_independent_label()
                .ok_or_else(|| invalid("move-and-attack action requires a label"))?;
            self.push_action(actions, descriptor, label);
        }
        Ok(())
    }

    fn movement_paths(&self, start: Location, profile: MovementProfile) -> Vec<Vec<Location>> {
        if !self.footprint_location_exists(profile.occupied_cells, start.cell, start) {
            return Vec::new();
        }
        let mut paths = vec![vec![start]];
        let Some(maximum_cost) = profile.maximum_cost else {
            return paths;
        };
        let mut frontier = vec![(0_usize, vec![start])];
        while !frontier.is_empty() {
            let mut next_frontier = Vec::new();
            for (cost, path) in frontier {
                let current = *path.last().expect("movement path starts nonempty");
                if self.footprint_is_immobilized(profile.occupied_cells, start.cell, current.cell) {
                    continue;
                }
                for candidate in self.movement_step_candidates(current, profile) {
                    let footprint_allowed = profile.occupied_cells.map_or_else(
                        || {
                            self.location_exists_in_region(candidate.cell, candidate.region)
                                && self.unit_entry_allowed(current, candidate, profile)
                        },
                        |area| {
                            let Some(current_area) =
                                translated_square(area, start.cell, current.cell)
                            else {
                                return false;
                            };
                            let Some(candidate_area) =
                                translated_square(area, start.cell, candidate.cell)
                            else {
                                return false;
                            };
                            candidate_area.into_iter().all(|entered| {
                                self.location_exists_in_region(entered, candidate.region)
                                    && (current_area.contains(&entered)
                                        || self.unit_entry_allowed(
                                            current,
                                            Location {
                                                cell: entered,
                                                region: candidate.region,
                                            },
                                            profile,
                                        ))
                            })
                        },
                    );
                    if !footprint_allowed
                        || !Self::movement_restriction_allows(profile, current, candidate)
                        || path
                            .windows(2)
                            .any(|edge| edge[0] == current && edge[1] == candidate)
                    {
                        continue;
                    }
                    let next_cost = cost + self.movement_step_cost(current, candidate, profile);
                    if next_cost > maximum_cost {
                        continue;
                    }
                    let mut next = path.clone();
                    next.push(candidate);
                    next_frontier.push((next_cost, next));
                }
            }
            frontier = next_frontier;
            paths.extend(frontier.iter().map(|(_, path)| path.clone()));
        }
        paths
    }

    /// Every location one step reaches from `current`, honoring the mover's region abilities.
    fn movement_step_candidates(
        &self,
        current: Location,
        profile: MovementProfile,
    ) -> Vec<Location> {
        let bordering = |region: Region| {
            current
                .cell
                .bordering(profile.connects_top_bottom)
                .map(move |cell| Location { cell, region })
        };
        let mut candidates = Vec::new();
        match current.region {
            Region::Surface => {
                candidates.extend(bordering(Region::Surface));
                if profile.airborne {
                    candidates.extend(current.cell.diagonals(profile.connects_top_bottom).map(
                        |cell| Location {
                            cell,
                            region: Region::Surface,
                        },
                    ));
                }
                if profile.regions.burrowing && !self.is_water_site(current.cell) {
                    candidates.push(Location {
                        cell: current.cell,
                        region: Region::Underground,
                    });
                }
                if profile.regions.submerge && self.is_water_site(current.cell) {
                    candidates.push(Location {
                        cell: current.cell,
                        region: Region::Underwater,
                    });
                }
                if profile.regions.voidwalk {
                    candidates.extend(bordering(Region::Void));
                }
            }
            Region::Underground if profile.regions.burrowing => {
                candidates.push(Location {
                    cell: current.cell,
                    region: Region::Surface,
                });
                candidates.extend(bordering(Region::Underground));
                if profile.regions.submerge {
                    candidates.extend(bordering(Region::Underwater));
                }
                if profile.regions.voidwalk {
                    candidates.extend(bordering(Region::Void));
                }
            }
            Region::Underwater if profile.regions.submerge => {
                candidates.push(Location {
                    cell: current.cell,
                    region: Region::Surface,
                });
                candidates.extend(bordering(Region::Underwater));
                if profile.regions.burrowing {
                    candidates.extend(bordering(Region::Underground));
                }
                if profile.regions.voidwalk {
                    candidates.extend(bordering(Region::Void));
                }
            }
            Region::Void if profile.regions.voidwalk => {
                candidates.extend(bordering(Region::Void));
                candidates.extend(bordering(Region::Surface));
                if profile.regions.burrowing {
                    candidates.extend(bordering(Region::Underground));
                }
                if profile.regions.submerge {
                    candidates.extend(bordering(Region::Underwater));
                }
            }
            Region::Underground | Region::Underwater | Region::Void => {}
        }
        candidates
    }

    /// Whether the whole footprint translated onto `location` exists in that region.
    fn footprint_location_exists(
        &self,
        occupied_cells: Option<SquareArea>,
        start: Cell,
        location: Location,
    ) -> bool {
        occupied_cells.map_or_else(
            || self.location_exists_in_region(location.cell, location.region),
            |area| {
                translated_square(area, start, location.cell).is_some_and(|translated| {
                    translated
                        .into_iter()
                        .all(|cell| self.location_exists_in_region(cell, location.region))
                })
            },
        )
    }

    fn cell_is_immobilized(&self, cell: Cell) -> bool {
        self.position
            .immobile_areas
            .iter()
            .any(|area| area.cells.contains(&cell))
    }

    fn footprint_is_immobilized(
        &self,
        occupied_cells: Option<SquareArea>,
        start: Cell,
        current: Cell,
    ) -> bool {
        occupied_cells.map_or_else(
            || self.cell_is_immobilized(current),
            |area| {
                translated_square(area, start, current).is_some_and(|translated| {
                    translated
                        .into_iter()
                        .any(|cell| self.cell_is_immobilized(cell))
                })
            },
        )
    }

    fn movement_step_cost(
        &self,
        current: Location,
        candidate: Location,
        profile: MovementProfile,
    ) -> usize {
        if profile.cause == MovementCause::CardEffect
            || !profile.airborne
            || !profile.moving_minion
            || current.region != Region::Surface
            || current.cell == candidate.cell
        {
            return 1;
        }
        let Some(site) = &self.position.sites[current.cell.index()] else {
            return 1;
        };
        let CardFacts::Site(facts) = &self.rules.cards[usize::from(site.card.card_id.0)].facts
        else {
            return 1;
        };
        usize::from(!facts.airborne_minions_atop_move_freely_away)
    }

    fn unit_entry_allowed(
        &self,
        current: Location,
        candidate: Location,
        profile: MovementProfile,
    ) -> bool {
        if !profile.moving_minion
            || profile.airborne
            || current.region != Region::Surface
            || candidate.region != Region::Surface
            || current.cell == candidate.cell
        {
            return true;
        }
        let Some(site) = &self.position.sites[candidate.cell.index()] else {
            return true;
        };
        let CardFacts::Site(facts) = &self.rules.cards[usize::from(site.card.card_id.0)].facts
        else {
            return true;
        };
        !facts.blocks_ground_minion_entry_while_minion_atop
            || !self.position.units.iter().any(|unit| {
                unit.region == Region::Surface && Self::unit_occupies_cell(unit, candidate.cell)
            })
    }

    fn movement_restriction_allows(profile: MovementProfile, from: Location, to: Location) -> bool {
        if profile.restriction.is_some() && from.region != to.region {
            return false;
        }
        let (from, to) = (from.cell, to.cell);
        match profile.restriction {
            None => true,
            Some(BasicMovementRestriction::SidewaysOnly) => from.rank_index() == to.rank_index(),
            Some(BasicMovementRestriction::ForwardOnly) => {
                let forward_rank = match profile.seat {
                    Seat::North => from.rank_index() - 1,
                    Seat::South => from.rank_index() + 1,
                };
                from.file_index() == to.file_index()
                    && (to.rank_index() == forward_rank
                        || profile.connects_top_bottom
                            && match profile.seat {
                                Seat::North => from.rank_index() == 0 && to.rank_index() == 3,
                                Seat::South => from.rank_index() == 3 && to.rank_index() == 0,
                            })
            }
        }
    }

    fn controlled_site_cells(&self, seat: Seat) -> impl Iterator<Item = Cell> + '_ {
        self.position
            .sites
            .iter()
            .enumerate()
            .filter_map(move |(index, site)| {
                site.as_ref()
                    .is_some_and(|site| site.controller == seat)
                    .then_some(Cell::ALL[index])
            })
    }

    fn surface_location_exists(&self, cell: Cell) -> bool {
        self.position.sites[cell.index()].is_some() || self.position.rubble[cell.index()].is_some()
    }

    /// Whether a played Water site currently stands at one cell.
    fn is_water_site(&self, cell: Cell) -> bool {
        self.position.sites[cell.index()]
            .as_ref()
            .is_some_and(|site| {
                matches!(
                    &self.rules.cards[usize::from(site.card.card_id.0)].facts,
                    CardFacts::Site(facts) if facts.elements.contains(Element::Water)
                )
            })
    }

    fn underground_location_exists(&self, cell: Cell) -> bool {
        self.surface_location_exists(cell) && !self.is_water_site(cell)
    }

    /// Every unit standing at one location in canonical identity order, for random selection.
    fn units_at_location(&self, location: Location) -> Vec<(IdentityHash, UnitKind, Seat)> {
        let mut candidates = Vec::new();
        if location.region == Region::Surface {
            for seat in [Seat::North, Seat::South] {
                let avatar = &self.position.players[seat_index(seat)].avatar;
                if avatar.location == location.cell {
                    candidates.push((avatar.card.instance_id.clone(), UnitKind::Avatar, seat));
                }
            }
        }
        candidates.extend(
            self.position
                .units
                .iter()
                .filter(|unit| {
                    unit.region == location.region && Self::unit_occupies_cell(unit, location.cell)
                })
                .map(|unit| {
                    (
                        unit.card.instance_id.clone(),
                        UnitKind::Minion,
                        unit.controller,
                    )
                }),
        );
        candidates.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        candidates
    }

    /// Every unit the printed grid reaches above and below one cell, in canonical target order.
    fn site_grid_damage_targets(
        &self,
        cell: Cell,
        grid: [u8; 5],
    ) -> Vec<(IdentityHash, UnitKind, Seat, u16)> {
        let mut targets = Vec::new();
        for seat in [Seat::North, Seat::South] {
            let avatar = &self.position.players[seat_index(seat)].avatar;
            let amount = Self::grid_damage(grid, cell, std::slice::from_ref(&avatar.location));
            if amount > 0 {
                targets.push((
                    avatar.card.instance_id.clone(),
                    UnitKind::Avatar,
                    seat,
                    amount,
                ));
            }
        }
        for unit in &self.position.units {
            // The Void sits beside the realm rather than above or below any cell.
            if unit.region == Region::Void {
                continue;
            }
            let amount = Self::grid_damage(grid, cell, Self::unit_occupied_cells(unit));
            if amount > 0 {
                targets.push((
                    unit.card.instance_id.clone(),
                    UnitKind::Minion,
                    unit.controller,
                    amount,
                ));
            }
        }
        targets.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        targets
    }

    /// Sums the grid entry for every occupied cell within two files and two ranks of the centre.
    fn grid_damage(grid: [u8; 5], center: Cell, occupied: &[Cell]) -> u16 {
        occupied.iter().fold(0_u16, |total, cell| {
            let files = center.file_index().abs_diff(cell.file_index());
            let ranks = center.rank_index().abs_diff(cell.rank_index());
            if files <= 2 && ranks <= 2 {
                total.saturating_add(u16::from(grid[usize::from(files + ranks)]))
            } else {
                total
            }
        })
    }

    fn location_exists_in_region(&self, cell: Cell, region: Region) -> bool {
        match region {
            Region::Surface => self.surface_location_exists(cell),
            Region::Underground => self.underground_location_exists(cell),
            Region::Underwater => self.is_water_site(cell),
            Region::Void => !self.surface_location_exists(cell),
        }
    }

    /// Every location a measured walk of at most `steps` cardinal steps reaches without leaving the
    /// starting region.
    fn locations_within_measured_steps(&self, start: Location, steps: u8) -> Vec<Location> {
        if !self.location_exists_in_region(start.cell, start.region) {
            return Vec::new();
        }
        let mut distances = [u8::MAX; Cell::ALL.len()];
        distances[start.cell.index()] = 0;
        for distance in 0..steps {
            for cell in Cell::ALL {
                if distances[cell.index()] != distance {
                    continue;
                }
                for bordering in cell.bordering(false) {
                    if distances[bordering.index()] == u8::MAX
                        && self.location_exists_in_region(bordering, start.region)
                    {
                        distances[bordering.index()] = distance + 1;
                    }
                }
            }
        }
        Cell::ALL
            .into_iter()
            .filter(|cell| distances[cell.index()] != u8::MAX)
            .map(|cell| Location {
                cell,
                region: start.region,
            })
            .collect()
    }

    fn move_underground_units_to_underwater(&mut self, cell: Cell) {
        for unit in &mut self.position.units {
            if unit.location == cell && unit.region == Region::Underground {
                unit.region = Region::Underwater;
            }
        }
    }

    fn minion_can_move_and_attack(&self, unit: &UnitPosition, seat: Seat) -> bool {
        unit.controller == seat
            && unit.region == Region::Surface
            && !self.minion_is_disabled(unit)
            && !unit.tapped
            && (!unit.summoning_sickness || self.minion_has_active_charge(unit))
    }

    fn minion_has_active_charge(&self, unit: &UnitPosition) -> bool {
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return false;
        };
        !self.minion_is_disabled(unit)
            && (facts.charge || !unit.temporary_charge_sources.is_empty())
    }

    fn attacker_can_target_sites(&self, pending: &PendingCombat) -> Result<bool, GameError> {
        if pending.attacker_kind == UnitKind::Avatar {
            return Ok(true);
        }
        let unit = self
            .position
            .units
            .iter()
            .find(|unit| {
                unit.card.instance_id == pending.attacker_instance_id
                    && unit.controller == pending.attacking_seat
            })
            .ok_or(GameError::IllegalAction)?;
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        Ok(!facts.cannot_attack_sites)
    }

    fn elemental_affinities(&self, seat: Seat) -> [u64; 4] {
        let mut suppressed_sites = [false; Cell::ALL.len()];
        for unit in self
            .position
            .units
            .iter()
            .filter(|unit| !self.minion_is_disabled(unit))
        {
            if matches!(
                &self.rules.cards[usize::from(unit.card.card_id.0)].facts,
                CardFacts::Minion(minion) if minion.site_provides_no_threshold
            ) {
                suppressed_sites[unit.location.index()] = true;
            }
        }
        let site_elements = Cell::ALL
            .into_iter()
            .filter_map(|cell| {
                let site = self.position.sites[cell.index()].as_ref()?;
                if site.controller != seat {
                    return None;
                }
                let CardFacts::Site(facts) =
                    &self.rules.cards[usize::from(site.card.card_id.0)].facts
                else {
                    return None;
                };
                if suppressed_sites[cell.index()] && !facts.cannot_be_moved_destroyed_or_modified {
                    return None;
                }
                Some(facts.elements)
            })
            .flat_map(crate::facts::ElementSet::iter);
        let provider_elements = self
            .position
            .units
            .iter()
            .filter(|unit| unit.controller == seat && !self.minion_is_disabled(unit))
            .filter_map(|unit| {
                let CardFacts::Minion(facts) =
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts
                else {
                    return None;
                };
                facts.provides
            });
        let mut affinities = [0_u64; 4];
        for element in site_elements.chain(provider_elements) {
            affinities[match element {
                Element::Earth => 0,
                Element::Fire => 1,
                Element::Water => 2,
                Element::Air => 3,
            }] += 1;
        }
        affinities
    }

    fn thresholds_met(&self, seat: Seat, thresholds: Thresholds) -> bool {
        self.elemental_affinities(seat)
            .into_iter()
            .zip(thresholds.canonical())
            .all(|(available, required)| available >= required)
    }

    fn summon_destinations(&self, seat: Seat, minion: &MinionFacts) -> Vec<SummonDestination> {
        let summon_cell = |cell: Cell| {
            let site = self.position.sites[cell.index()].as_ref()?;
            if !minion.summon_to_any_site && site.controller != seat {
                return None;
            }
            let CardFacts::Site(site_facts) =
                &self.rules.cards[usize::from(site.card.card_id.0)].facts
            else {
                return None;
            };
            if minion.must_be_cast_to_water_site && !site_facts.elements.contains(Element::Water) {
                return None;
            }
            let discount = u64::from(minion.ordinary && site_facts.ordinary_minion_mana_discount);
            Some(minion.mana_cost.saturating_sub(discount))
        };
        if minion.occupies_square_area_two {
            Cell::SQUARE_AREAS
                .into_iter()
                .filter_map(|cells| {
                    let mana_cost = cells.into_iter().find_map(summon_cell)?;
                    cells
                        .into_iter()
                        .all(|cell| self.surface_location_exists(cell))
                        .then_some(SummonDestination {
                            cell: cells[0],
                            cells: Some(cells),
                            mana_cost,
                        })
                })
                .collect()
        } else {
            Cell::ALL
                .into_iter()
                .filter_map(|cell| {
                    summon_cell(cell).map(|mana_cost| SummonDestination {
                        cell,
                        cells: None,
                        mana_cost,
                    })
                })
                .collect()
        }
    }

    /// Enumerates the placements a free summon grants: any existing surface location, ignoring
    /// site control, mana, thresholds, and the printed casting restrictions.
    fn free_summon_destinations(&self, minion: &MinionFacts) -> Vec<SummonDestination> {
        if minion.occupies_square_area_two {
            Cell::SQUARE_AREAS
                .into_iter()
                .filter(|cells| cells.iter().all(|cell| self.surface_location_exists(*cell)))
                .map(|cells| SummonDestination {
                    cell: cells[0],
                    cells: Some(cells),
                    mana_cost: 0,
                })
                .collect()
        } else {
            Cell::ALL
                .into_iter()
                .filter(|cell| self.surface_location_exists(*cell))
                .map(|cell| SummonDestination {
                    cell,
                    cells: None,
                    mana_cost: 0,
                })
                .collect()
        }
    }

    fn genesis_damage_targets(
        &self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        cell: Cell,
    ) -> Vec<UnitTarget> {
        let source_cells = [cell];
        let mut targets = vec![UnitTarget::Minion {
            instance_id: source_instance_id.clone(),
            seat,
        }];
        for target_seat in [Seat::North, Seat::South] {
            let player = &self.position.players[seat_index(target_seat)];
            if Self::footprints_here_or_bordering(
                &source_cells,
                std::slice::from_ref(&player.avatar.location),
            ) {
                targets.push(UnitTarget::Avatar {
                    instance_id: player.avatar.card.instance_id.clone(),
                    seat: target_seat,
                });
            }
            targets.extend(
                self.position
                    .units
                    .iter()
                    .filter(|unit| {
                        unit.controller == target_seat
                            && unit.region == Region::Surface
                            && Self::footprints_here_or_bordering(
                                &source_cells,
                                Self::unit_occupied_cells(unit),
                            )
                            && (target_seat == seat || !self.minion_has_active_stealth(unit))
                    })
                    .map(|unit| UnitTarget::Minion {
                        instance_id: unit.card.instance_id.clone(),
                        seat: target_seat,
                    }),
            );
        }
        targets.sort_unstable_by(|left, right| left.instance_id().cmp(right.instance_id()));
        targets
    }

    /// Enumerates the optional Genesis damage decisions one summon at `cell` may issue.
    fn genesis_damage_choices(
        &self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        cell: Cell,
        genesis: Option<MinionGenesis>,
    ) -> Vec<(Option<GenesisDamageChoice>, Option<UnitTarget>)> {
        if genesis == Some(MinionGenesis::MayDamageTargetAdjacentUnitTwo) {
            std::iter::once((Some(GenesisDamageChoice::Decline), None))
                .chain(
                    self.genesis_damage_targets(seat, source_instance_id, cell)
                        .into_iter()
                        .map(|target| (Some(GenesisDamageChoice::Target), Some(target))),
                )
                .collect()
        } else {
            vec![(None, None)]
        }
    }

    fn genesis_damage_suffix(
        choice: Option<GenesisDamageChoice>,
        target: Option<&UnitTarget>,
    ) -> String {
        match (choice, target) {
            (Some(GenesisDamageChoice::Decline), None) => "; decline Genesis".to_owned(),
            (Some(GenesisDamageChoice::Target), Some(target)) => format!(
                "; Genesis targets {} {}…",
                target.kind(),
                &target.instance_id().as_str()[..15]
            ),
            _ => String::new(),
        }
    }

    fn valid_genesis_damage_choice(
        &self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        cell: Cell,
        genesis: Option<MinionGenesis>,
        choice: Option<GenesisDamageChoice>,
        target: Option<&UnitTarget>,
    ) -> bool {
        match (genesis, choice, target) {
            (
                Some(MinionGenesis::MayDamageTargetAdjacentUnitTwo),
                Some(GenesisDamageChoice::Decline),
                None,
            ) => true,
            (
                Some(MinionGenesis::MayDamageTargetAdjacentUnitTwo),
                Some(GenesisDamageChoice::Target),
                Some(target),
            ) => self
                .genesis_damage_targets(seat, source_instance_id, cell)
                .contains(target),
            (Some(MinionGenesis::MayDamageTargetAdjacentUnitTwo), _, _) => false,
            (_, None, None) => true,
            _ => false,
        }
    }

    fn push_action(
        &self,
        actions: &mut Vec<IssuedAction>,
        descriptor: ActionDescriptor,
        label: String,
    ) {
        let seat = self.position.decision_seat;
        actions.push(IssuedAction {
            descriptor,
            label,
            seat,
            state_version: self.position.state_version,
        });
    }

    fn legal_site_cells(&self, seat: Seat) -> Vec<Cell> {
        let player = &self.position.players[seat_index(seat)];
        let controlled_cells: Vec<_> = self.controlled_site_cells(seat).collect();
        if controlled_cells.is_empty() {
            let minimum_distance = Cell::ALL
                .into_iter()
                .filter(|cell| self.position.sites[cell.index()].is_none())
                .map(|cell| player.avatar.location.manhattan_distance(cell))
                .min();
            return Cell::ALL
                .into_iter()
                .filter(|cell| self.position.sites[cell.index()].is_none())
                .filter(|cell| {
                    Some(player.avatar.location.manhattan_distance(*cell)) == minimum_distance
                })
                .collect();
        }
        controlled_cells
            .into_iter()
            .flat_map(|cell| cell.bordering(false))
            .filter(|cell| self.position.sites[cell.index()].is_none())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Applies an engine-issued action without authoritative serialization or hashing.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::IllegalAction`] when the action is stale, belongs to
    /// another decision, or is not valid for the current phase.
    pub fn apply_action(&mut self, action: &IssuedAction) -> Result<(), GameError> {
        self.apply_action_with_log(action, &mut OutcomeLog::Ignore, None)
    }

    #[expect(
        clippy::type_complexity,
        reason = "the internal transition returns its two existing receipt logs together"
    )]
    pub(crate) fn apply_action_recorded(
        &mut self,
        action: &IssuedAction,
    ) -> Result<(Vec<(String, Value)>, Vec<EngineRandomDraw>), GameError> {
        let mut outcomes = Vec::new();
        let mut random_draws = Vec::new();
        self.apply_action_with_log(
            action,
            &mut OutcomeLog::Record(&mut outcomes),
            Some(&mut random_draws),
        )?;
        Ok((outcomes, random_draws))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "closed typed dispatch keeps authoritative action routing explicit"
    )]
    fn apply_action_with_log(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
        random_draws: Option<&mut Vec<EngineRandomDraw>>,
    ) -> Result<(), GameError> {
        if action.seat != self.position.decision_seat
            || action.state_version != self.position.state_version
        {
            return Err(GameError::IllegalAction);
        }
        let magic_completion = match &action.descriptor {
            ActionDescriptor::CastMagic {
                card_instance_id, ..
            } => self.position.players[seat_index(action.seat)]
                .hand_spellbook
                .iter()
                .find(|card| card.instance_id == *card_instance_id)
                .map(|card| DeferredMagicResolved {
                    card_id: card.card_id,
                    instance_id: card.instance_id.clone(),
                    owner: card.owner,
                }),
            ActionDescriptor::ResolveChainMagic => self
                .position
                .pending_chain_magic
                .as_pending()
                .and_then(|pending| {
                    self.position.players[seat_index(action.seat)]
                        .hand_spellbook
                        .iter()
                        .find(|card| card.instance_id == pending.card_instance_id)
                })
                .map(|card| DeferredMagicResolved {
                    card_id: card.card_id,
                    instance_id: card.instance_id.clone(),
                    owner: card.owner,
                }),
            _ => None,
        };
        let post_ranged_step = match &action.descriptor {
            ActionDescriptor::ShootProjectile {
                hit: Some(_),
                shooter_instance_id,
                ..
            } if self.position.phase != Phase::Movement => Some(shooter_instance_id),
            _ => None,
        };
        let applied = match &action.descriptor {
            ActionDescriptor::ActivateAreaDamage { .. } => {
                self.apply_area_damage_action(action, outcomes)
            }
            ActionDescriptor::ActivateArtifactDamage { .. } => {
                self.apply_artifact_damage_action(action, outcomes)
            }
            ActionDescriptor::ActivateArtifactDiscardAreaDamage { .. } => {
                self.apply_artifact_discard_area_damage_action(action, outcomes)
            }
            ActionDescriptor::ActivateArtifactRollDamage { .. } => {
                self.apply_artifact_roll_damage_action(action, outcomes)
            }
            ActionDescriptor::ActivateDiscardRandomDamage { .. } => {
                self.apply_discard_random_damage_action(action, outcomes, random_draws)
            }
            ActionDescriptor::ActivateSparkmage { .. } => {
                self.apply_sparkmage_action(action, outcomes, random_draws)
            }
            ActionDescriptor::AllocateStrike {
                amount,
                target_instance_id,
            } => self.apply_allocate_strike_action(
                action.seat,
                *amount,
                target_instance_id,
                outcomes,
            ),
            ActionDescriptor::ActivateMana {
                amount,
                unit_instance_id,
            } => self.apply_mana_activation(action.seat, *amount, unit_instance_id, outcomes),
            ActionDescriptor::CastArtifact { .. } => {
                self.apply_cast_artifact_action(action, outcomes)
            }
            ActionDescriptor::CastMagic { .. } => {
                self.apply_cast_magic_action(action, outcomes, random_draws)
            }
            ActionDescriptor::DropArtifacts { .. } => {
                self.apply_drop_artifacts_action(action, outcomes)
            }
            ActionDescriptor::PickUpArtifacts { .. } => {
                self.apply_pick_up_artifacts_action(action, outcomes)
            }
            ActionDescriptor::BeginChainMagic { .. } => self.apply_begin_chain_magic(action),
            ActionDescriptor::CloseDefend {
                original_target_participates,
            } => {
                self.apply_close_defend_action(action.seat, *original_target_participates, outcomes)
            }
            ActionDescriptor::CloseIntercept {} => {
                self.apply_close_intercept_action(action.seat, outcomes)
            }
            ActionDescriptor::ContinueBasicMovement { unit_instance_id } => {
                self.apply_continue_basic_movement(action.seat, unit_instance_id, outcomes)
            }
            ActionDescriptor::DeclareAttack { target } => {
                self.apply_declare_attack_action(action.seat, target, outcomes)
            }
            ActionDescriptor::DeclineAttack => {
                self.apply_decline_attack_action(action.seat, outcomes)
            }
            ActionDescriptor::Defend {
                from,
                path,
                to,
                unit_instance_id,
            } => {
                self.apply_defend_action(action.seat, *from, path, *to, unit_instance_id, outcomes)
            }
            ActionDescriptor::Draw { zone } => {
                self.apply_draw_action(action.seat, *zone, false, outcomes)
            }
            ActionDescriptor::Mulligan {
                atlas_order,
                spellbook_order,
            } => self.apply_mulligan_action(action.seat, atlas_order, spellbook_order, outcomes),
            ActionDescriptor::PlaySite { .. } => self.apply_play_site_action(action, outcomes),
            ActionDescriptor::ReplaceRubbleWithTopAtlasSite {
                target_cell,
                target_rubble_instance_id,
            } => self.apply_replace_rubble_action(
                action.seat,
                *target_cell,
                target_rubble_instance_id,
                outcomes,
            ),
            ActionDescriptor::ActivateSiteDestruction {
                source_site_instance_id,
                target_cell,
                target_site_instance_id,
            } => self.apply_site_destruction_action(
                action.seat,
                source_site_instance_id,
                *target_cell,
                target_site_instance_id,
                outcomes,
            ),
            ActionDescriptor::ResolveGenesisSpell { choice } => {
                self.apply_resolve_genesis_spell(action.seat, *choice, outcomes)
            }
            ActionDescriptor::ResolveGenesisSpellOrder { order } => {
                self.apply_resolve_genesis_spell_order(action.seat, order, outcomes)
            }
            ActionDescriptor::ResolveGenesisToken { choice } => {
                self.apply_resolve_genesis_token(action.seat, *choice, outcomes)
            }
            ActionDescriptor::ResolveRangedStep { .. } => {
                self.apply_resolve_ranged_step(action, outcomes)
            }
            ActionDescriptor::ShootProjectile { .. } => {
                self.apply_ranged_projectile_action(action, outcomes)
            }
            ActionDescriptor::ShootDamageProjectile { .. } => {
                self.apply_damage_projectile_action(action, outcomes)
            }
            ActionDescriptor::ShootDragProjectile { .. } => {
                self.apply_drag_projectile_action(action, outcomes)
            }
            ActionDescriptor::SummonMinion { .. } => {
                self.apply_summon_minion_action(action, outcomes, random_draws)
            }
            ActionDescriptor::MoveAndAttack {
                from,
                path,
                to,
                unit_instance_id,
            } => self.apply_move_and_attack_action(
                action.seat,
                *from,
                path,
                *to,
                unit_instance_id,
                outcomes,
            ),
            ActionDescriptor::OrderDeathrites { source_instance_id } => {
                self.apply_deathrite_order_action(action.seat, source_instance_id, outcomes)
            }
            ActionDescriptor::EndTurn => self.apply_end_turn_action(action.seat, outcomes),
            ActionDescriptor::ExtendChainMagic { .. } => self.apply_extend_chain_magic(action),
            ActionDescriptor::Intercept { unit_instance_id } => {
                self.apply_intercept_action(action.seat, unit_instance_id, outcomes)
            }
            ActionDescriptor::DrawSite => {
                self.apply_draw_action(action.seat, DeckZone::Atlas, true, outcomes)
            }
            ActionDescriptor::DrawSpell => {
                self.apply_draw_action(action.seat, DeckZone::Spellbook, true, outcomes)
            }
            ActionDescriptor::ResolveChainMagic => self.apply_resolve_chain_magic(action, outcomes),
        };
        applied?;
        let settlement_start = outcomes.len();
        self.settle_lower_region_minion_deaths(outcomes)?;
        self.settle_nearby_enemy_stealth(outcomes);
        self.settle_static_power_deaths(outcomes)?;
        if let Some(shooter_instance_id) = post_ranged_step {
            self.queue_ranged_step(action.seat, shooter_instance_id)?;
        }
        outcomes.move_tail_before_completion(settlement_start);
        // Magic that owns its own resolution event resumes through its continuation instead.
        let continuing_cast = self.position.pending_cemetery_summon.is_some()
            || matches!(
                action.descriptor,
                ActionDescriptor::CastMagic {
                    ally_destination: Some(_),
                    ..
                } | ActionDescriptor::CastMagic {
                    draw_zone: Some(_),
                    ..
                }
            );
        if let (Some(completion), Some(pending)) =
            (magic_completion, &mut self.position.pending_deathrites)
            && pending.deferred_magic_resolved.is_none()
            && !continuing_cast
            && !matches!(
                pending.continuation,
                Some(DeathriteContinuation::Blink(_) | DeathriteContinuation::LeapAttack(_))
            )
        {
            pending.deferred_magic_resolved = Some(completion);
            outcomes.remove_first("magic-resolved");
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one Ranged transaction keeps strike, death, and continuation ordering explicit"
    )]
    fn apply_ranged_projectile_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ShootProjectile {
            direction,
            hit,
            path,
            shooter_instance_id,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let movement = self.position.pending_basic_movement.as_pending().cloned();
        if !self
            .ranged_projectile_descriptors(
                action.seat,
                shooter_instance_id,
                self.position.phase == Phase::Movement,
            )?
            .iter()
            .any(|candidate| candidate == &action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let shooter_index = self
            .position
            .units
            .iter()
            .position(|unit| {
                unit.controller == action.seat && unit.card.instance_id == *shooter_instance_id
            })
            .ok_or(GameError::IllegalAction)?;
        let strike =
            self.combatant_strike_stats(UnitKind::Minion, action.seat, shooter_instance_id)?;
        self.position.units[shooter_index].tapped = true;
        if let Some(pending) = self.position.pending_basic_movement.as_pending_mut() {
            pending.ranged_strike_used = true;
        }
        outcomes.push("projectile-shot", || {
            json!({
                "direction": direction,
                "hit": hit,
                "path": path,
                "seat": action.seat,
                "shooterInstanceId": shooter_instance_id,
            })
        });
        self.record_unit_interaction(UnitKind::Minion, action.seat, shooter_instance_id, outcomes)?;
        let Some(target) = hit else {
            self.position.state_version += 1;
            return Ok(());
        };
        outcomes.push("strike-damage-allocated", || {
            json!({
                "amount": strike.amount,
                "strikerInstanceId": shooter_instance_id,
                "targetInstanceId": target.instance_id(),
            })
        });
        let target_kind = match target {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        let damage = self.apply_simple_damage(
            target_kind,
            target.seat(),
            target.instance_id(),
            strike.amount,
            UnitDamageSource {
                current_power: strike.current_power,
                lethal: strike.lethal,
            },
            outcomes,
        )?;
        if strike.lance_count > 0 {
            self.break_lance(action.seat, shooter_instance_id, outcomes)?;
        }
        if damage.minion_died || damage.avatar_defeated {
            let target_instance_id = target.instance_id().clone();
            let target_seat = target.seat();
            self.begin_minion_deaths(
                if damage.minion_died {
                    std::slice::from_ref(&target_instance_id)
                } else {
                    &[]
                },
                if damage.avatar_defeated {
                    std::slice::from_ref(&target_seat)
                } else {
                    &[]
                },
                if movement.is_some() {
                    Phase::Movement
                } else {
                    Phase::Main
                },
                action.seat,
                outcomes,
            )?;
        }
        if self.position.pending_deathrites.is_none() {
            self.reconcile_projectile_continuations()?;
        }
        self.position.state_version += 1;
        Ok(())
    }

    fn queue_ranged_step(
        &mut self,
        seat: Seat,
        source_instance_id: &IdentityHash,
    ) -> Result<(), GameError> {
        if self.position.terminal.is_some() {
            return Ok(());
        }
        let pending = PendingRangedStep {
            seat,
            source_instance_id: source_instance_id.clone(),
        };
        if !self
            .ranged_step_descriptors(&pending)?
            .iter()
            .any(|descriptor| {
                matches!(
                    descriptor,
                    ActionDescriptor::ResolveRangedStep {
                        choice: RangedStepChoice::Step,
                        ..
                    }
                )
            })
        {
            return Ok(());
        }
        self.position.pending_ranged_step = PendingField::Pending(pending);
        if let Some(deathrites) = self.position.pending_deathrites.as_mut() {
            deathrites.return_phase = Phase::RangedStep;
            deathrites.return_decision_seat = seat;
        } else {
            self.position.phase = Phase::RangedStep;
            self.position.decision_seat = seat;
        }
        Ok(())
    }

    fn reconcile_projectile_continuations(&mut self) -> Result<(), GameError> {
        if self.position.terminal.is_some() {
            if !matches!(self.position.pending_basic_movement, PendingField::Absent) {
                self.position.pending_basic_movement = PendingField::Resolved;
            }
            self.position.pending_ranged_step = PendingField::Absent;
            self.position.pending_combat = None;
            self.position.phase = Phase::Terminal;
            return Ok(());
        }
        self.reconcile_pending_combat();
        if let Some(pending) = self.position.pending_basic_movement.as_pending().cloned() {
            let source_remains = self.position.units.iter().any(|unit| {
                unit.controller == pending.seat
                    && unit.card.instance_id == pending.source_instance_id
            });
            let combat_remains = pending.purpose == BasicMovementPurpose::MoveAndAttack
                || self.position.pending_combat.is_some();
            if source_remains && combat_remains {
                self.position.phase = Phase::Movement;
                self.position.decision_seat = pending.seat;
            } else {
                self.position.pending_basic_movement = PendingField::Resolved;
                if pending.purpose == BasicMovementPurpose::Defend
                    && self.position.pending_combat.is_some()
                {
                    self.position.phase = Phase::Defend;
                    self.position.decision_seat = pending.seat;
                } else {
                    self.position.phase = Phase::Main;
                    self.position.decision_seat = self.position.active_seat;
                }
            }
            return Ok(());
        }
        if self.position.pending_ranged_step.is_pending() {
            let pending = self
                .position
                .pending_ranged_step
                .as_pending()
                .ok_or(GameError::IllegalAction)?;
            let source_exists = self.position.units.iter().any(|unit| {
                unit.controller == pending.seat
                    && unit.card.instance_id == pending.source_instance_id
            });
            if source_exists {
                self.position.phase = Phase::RangedStep;
                self.position.decision_seat = pending.seat;
            } else {
                self.position.pending_ranged_step = PendingField::Resolved;
                self.position.phase = Phase::Main;
                self.position.decision_seat = self.position.active_seat;
            }
        }
        Ok(())
    }

    fn reconcile_pending_combat(&mut self) {
        let Some(mut pending) = self.position.pending_combat.take() else {
            return;
        };
        let attacker_exists = match pending.attacker_kind {
            UnitKind::Avatar => {
                self.position.players[seat_index(pending.attacking_seat)]
                    .avatar
                    .card
                    .instance_id
                    == pending.attacker_instance_id
            }
            UnitKind::Minion => self.position.units.iter().any(|unit| {
                unit.controller == pending.attacking_seat
                    && unit.card.instance_id == pending.attacker_instance_id
            }),
        };
        let target_exists = pending
            .original_target
            .as_ref()
            .is_none_or(|target| match target {
                CombatTarget::Avatar { instance_id, seat } => {
                    self.position.players[seat_index(*seat)]
                        .avatar
                        .card
                        .instance_id
                        == *instance_id
                }
                CombatTarget::Minion { instance_id, seat } => {
                    self.position.units.iter().any(|unit| {
                        unit.controller == *seat && unit.card.instance_id == *instance_id
                    })
                }
                CombatTarget::Site { instance_id, seat } => {
                    self.position.sites.iter().any(|site| {
                        site.as_ref().is_some_and(|site| {
                            site.controller == *seat && site.card.instance_id == *instance_id
                        })
                    })
                }
            });
        if !attacker_exists || !target_exists {
            return;
        }
        pending.defenders.retain(|target| match target {
            UnitTarget::Avatar { instance_id, seat } => {
                self.position.players[seat_index(*seat)]
                    .avatar
                    .card
                    .instance_id
                    == *instance_id
            }
            UnitTarget::Minion { instance_id, seat } => self
                .position
                .units
                .iter()
                .any(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id),
        });
        self.position.pending_combat = Some(pending);
    }

    fn apply_damage_projectile_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ShootDamageProjectile {
            direction,
            hit,
            path,
            shooter_instance_id,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        if !self
            .damage_projectile_descriptors(action.seat, shooter_instance_id)?
            .iter()
            .any(|candidate| candidate == &action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let shooter_index = self
            .position
            .units
            .iter()
            .position(|unit| {
                unit.controller == action.seat && unit.card.instance_id == *shooter_instance_id
            })
            .ok_or(GameError::IllegalAction)?;
        let CardFacts::Minion(facts) =
            &self.rules.cards[usize::from(self.position.units[shooter_index].card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let amount = u16::from(
            facts
                .tap_to_shoot_projectile_damage
                .ok_or(GameError::IllegalAction)?,
        );
        let (current_power, lethal) =
            self.combatant_attack_and_lethal(UnitKind::Minion, action.seat, shooter_instance_id)?;
        self.position.units[shooter_index].tapped = true;
        outcomes.push("projectile-shot", || {
            json!({
                "direction": direction,
                "hit": hit,
                "path": path,
                "seat": action.seat,
                "shooterInstanceId": shooter_instance_id,
            })
        });
        self.record_unit_interaction(UnitKind::Minion, action.seat, shooter_instance_id, outcomes)?;
        let Some(target) = hit else {
            self.position.state_version += 1;
            return Ok(());
        };
        outcomes.push("projectile-damage-allocated", || {
            json!({
                "amount": amount,
                "sourceInstanceId": shooter_instance_id,
                "targetInstanceId": target.instance_id(),
            })
        });
        let target_kind = match target {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        let damage = self.apply_simple_damage(
            target_kind,
            target.seat(),
            target.instance_id(),
            amount,
            UnitDamageSource {
                current_power,
                lethal,
            },
            outcomes,
        )?;
        if damage.minion_died || damage.avatar_defeated {
            let target_instance_id = target.instance_id().clone();
            let target_seat = target.seat();
            self.begin_minion_deaths(
                if damage.minion_died {
                    std::slice::from_ref(&target_instance_id)
                } else {
                    &[]
                },
                if damage.avatar_defeated {
                    std::slice::from_ref(&target_seat)
                } else {
                    &[]
                },
                Phase::Main,
                self.position.active_seat,
                outcomes,
            )?;
        }
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_drag_projectile_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ShootDragProjectile {
            direction,
            fight_on_arrival,
            hit,
            path,
            shooter_instance_id,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        if !self
            .drag_projectile_descriptors(action.seat, shooter_instance_id)?
            .iter()
            .any(|candidate| candidate == &action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let shooter_index = self
            .position
            .units
            .iter()
            .position(|unit| {
                unit.controller == action.seat && unit.card.instance_id == *shooter_instance_id
            })
            .ok_or(GameError::IllegalAction)?;
        self.position.units[shooter_index].tapped = true;
        outcomes.push("projectile-shot", || {
            json!({
                "direction": direction,
                "hit": hit,
                "path": path,
                "seat": action.seat,
                "shooterInstanceId": shooter_instance_id,
            })
        });
        self.record_unit_interaction(UnitKind::Minion, action.seat, shooter_instance_id, outcomes)?;
        let Some(target) = hit else {
            self.position.state_version += 1;
            return Ok(());
        };
        let continuation = DragProjectileContinuation {
            fight_on_arrival: *fight_on_arrival,
            path: self.hauled_path(target, path)?,
            path_index: 0,
            shooter: UnitTarget::Minion {
                instance_id: shooter_instance_id.clone(),
                seat: action.seat,
            },
            target: target.clone(),
        };
        self.continue_drag_projectile(&continuation, true, outcomes)?;
        self.position.state_version += 1;
        Ok(())
    }

    /// Reverses the ray so the hauled unit walks its own footprint back to the shooter's cell.
    fn hauled_path(
        &self,
        target: &UnitTarget,
        ray: &[Location],
    ) -> Result<Vec<Location>, GameError> {
        let contacted = ray.last().copied().ok_or(GameError::IllegalAction)?;
        let anchor = self.unit_target_location(target)?;
        ray.iter()
            .rev()
            .map(|location| {
                anchor
                    .cell
                    .translated(contacted.cell, location.cell)
                    .map(|cell| Location {
                        cell,
                        region: location.region,
                    })
                    .ok_or(GameError::IllegalAction)
            })
            .collect()
    }

    fn continue_drag_projectile(
        &mut self,
        continuation: &DragProjectileContinuation,
        emit_zero_step: bool,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.terminal.is_some() || !self.ally_remains(&continuation.target) {
            return Ok(());
        }
        let mut path_index = continuation.path_index;
        if path_index + 1 < continuation.path.len() || emit_zero_step {
            path_index = self.haul_along_path(continuation, emit_zero_step, outcomes)?;
            if self.position.pending_deathrites.is_some() {
                if path_index + 1 < continuation.path.len() || continuation.fight_on_arrival {
                    let resumed = DragProjectileContinuation {
                        path_index,
                        ..continuation.clone()
                    };
                    if let Some(pending) = self.position.pending_deathrites.as_mut() {
                        pending.continuation = Some(DeathriteContinuation::DragProjectile(resumed));
                    }
                }
                return Ok(());
            }
        }
        let destination = continuation
            .path
            .last()
            .copied()
            .ok_or(GameError::IllegalAction)?;
        let arrived = self.ally_remains(&continuation.target)
            && self.unit_target_location(&continuation.target)? == destination;
        if !continuation.fight_on_arrival
            || !arrived
            || !self.ally_remains(&continuation.shooter)
            || self.position.terminal.is_some()
        {
            return Ok(());
        }
        self.position.pending_combat = Some(PendingCombat {
            allocations: Vec::new(),
            attacker_instance_id: continuation.shooter.instance_id().clone(),
            attacker_kind: UnitKind::Minion,
            attacking_seat: continuation.shooter.seat(),
            cell: destination.cell,
            combatants: Vec::new(),
            defenders: Vec::new(),
            original_target: Some(match &continuation.target {
                UnitTarget::Avatar { instance_id, seat } => CombatTarget::Avatar {
                    instance_id: instance_id.clone(),
                    seat: *seat,
                },
                UnitTarget::Minion { instance_id, seat } => CombatTarget::Minion {
                    instance_id: instance_id.clone(),
                    seat: *seat,
                },
            }),
            region: destination.region,
            target_removed: false,
        });
        self.begin_fight(vec![continuation.target.clone()], outcomes)
    }

    /// Walks the hauled unit one location at a time, settling after each step so a mid-path death
    /// interrupts the haul exactly where it happened.
    fn haul_along_path(
        &mut self,
        continuation: &DragProjectileContinuation,
        emit_zero_step: bool,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<usize, GameError> {
        let start = continuation
            .path
            .get(continuation.path_index)
            .copied()
            .ok_or(GameError::IllegalAction)?;
        if self.unit_target_location(&continuation.target)? != start {
            return Ok(continuation.path_index);
        }
        let segment_start = outcomes.len();
        let mut path_index = continuation.path_index;
        let mut walked = vec![start];
        while let Some(next) = continuation.path.get(path_index + 1).copied() {
            let from = continuation.path[path_index];
            if self.unit_target_location(&continuation.target)? != from
                || !self.haul_entry_allowed(&continuation.target, from, next)?
            {
                break;
            }
            self.move_unit_target_to(&continuation.target, next)?;
            path_index += 1;
            walked.push(next);
            self.settle_lower_region_minion_deaths(outcomes)?;
            self.settle_nearby_enemy_stealth(outcomes);
            self.settle_static_power_deaths(outcomes)?;
            if self.position.pending_deathrites.is_some()
                || !self.ally_remains(&continuation.target)
                || self.position.terminal.is_some()
            {
                break;
            }
        }
        if walked.len() > 1 || emit_zero_step {
            let from = walked[0];
            let to = *walked.last().expect("hauled path starts nonempty");
            let steps = walked.len() - 1;
            let seat = continuation.shooter.seat();
            let source_instance_id = continuation.shooter.instance_id().clone();
            let target_instance_id = continuation.target.instance_id().clone();
            outcomes.insert(segment_start, "unit-dragged", || {
                json!({
                    "from": from,
                    "path": walked,
                    "seat": seat,
                    "sourceInstanceId": source_instance_id,
                    "steps": steps,
                    "targetInstanceId": target_instance_id,
                    "to": to,
                })
            });
        }
        Ok(path_index)
    }

    fn haul_entry_allowed(
        &self,
        target: &UnitTarget,
        from: Location,
        next: Location,
    ) -> Result<bool, GameError> {
        if next.region != Region::Surface || from.region != Region::Surface {
            return Ok(false);
        }
        let anchor = self.unit_target_location(target)?;
        let footprint = match target {
            UnitTarget::Avatar { .. } => None,
            UnitTarget::Minion { instance_id, seat } => {
                self.position
                    .units
                    .iter()
                    .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                    .ok_or(GameError::IllegalAction)?
                    .occupied_cells
            }
        };
        let occupied = footprint.map_or_else(|| vec![anchor.cell], Vec::from);
        let entered = match footprint {
            None => vec![next.cell],
            Some(area) => match translated_square(area, anchor.cell, next.cell) {
                None => return Ok(false),
                Some(area) => Vec::from(area),
            },
        };
        let profile = self.haul_movement_profile(target)?;
        Ok(entered.into_iter().all(|cell| {
            self.surface_location_exists(cell)
                && (occupied.contains(&cell)
                    || self.unit_entry_allowed(
                        from,
                        Location {
                            cell,
                            region: Region::Surface,
                        },
                        profile,
                    ))
        }))
    }

    fn haul_movement_profile(&self, target: &UnitTarget) -> Result<MovementProfile, GameError> {
        let (airborne, connects_top_bottom, moving_minion) = match target {
            UnitTarget::Avatar { .. } => (false, false, false),
            UnitTarget::Minion { instance_id, seat } => {
                let unit = self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                    .ok_or(GameError::IllegalAction)?;
                let CardFacts::Minion(facts) =
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts
                else {
                    return Err(GameError::IllegalAction);
                };
                (facts.airborne, facts.connects_top_bottom, true)
            }
        };
        Ok(MovementProfile {
            airborne,
            cause: MovementCause::CardEffect,
            connects_top_bottom,
            maximum_cost: None,
            moving_minion,
            occupied_cells: None,
            regions: RegionAbilities::default(),
            restriction: None,
            seat: target.seat(),
        })
    }

    fn apply_decline_attack_action(
        &mut self,
        seat: Seat,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        if self.position.phase != Phase::Attack || pending.attacking_seat != seat {
            return Err(GameError::IllegalAction);
        }
        let intercept_window_opened = !self.interceptor_candidates()?.is_empty();
        let attacker_instance_id = pending.attacker_instance_id.clone();
        if intercept_window_opened {
            self.position.phase = Phase::Intercept;
            self.position.decision_seat = other_seat(seat);
        } else {
            self.position.pending_combat = None;
            self.position.phase = Phase::Main;
            self.position.decision_seat = seat;
        }
        self.position.state_version += 1;
        outcomes.push("attack-declined", || {
            json!({
                "interceptWindowOpened": intercept_window_opened,
                "seat": seat,
                "unitInstanceId": attacker_instance_id,
            })
        });
        Ok(())
    }

    /// Walks a declared defender one cell at a time, settling derived power after each interior
    /// step so an aura the defender carries away kills its allies where it left them instead of
    /// being restored by the rest of the path. Returns the path index actually reached.
    ///
    /// The arrival step is deliberately left for the shared post-action settlement, so a defender
    /// that only strands its allies as it lands still joins the combat before they die.
    fn walk_declared_defend_path(
        &mut self,
        seat: Seat,
        path: &[Location],
        unit_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<usize, GameError> {
        self.position
            .units
            .iter_mut()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *unit_instance_id)
            .ok_or(GameError::IllegalAction)?
            .tapped = true;
        let mut reached = 0;
        while let Some(next) = path.get(reached + 1).copied() {
            let Some(unit) =
                self.position.units.iter_mut().find(|unit| {
                    unit.controller == seat && unit.card.instance_id == *unit_instance_id
                })
            else {
                break;
            };
            Self::move_minion_to(unit, next.cell)?;
            reached += 1;
            self.settle_nearby_enemy_stealth(outcomes);
            if reached + 1 >= path.len() {
                break;
            }
            self.settle_static_power_deaths(outcomes)?;
            if self.position.pending_deathrites.is_some() || self.position.terminal.is_some() {
                break;
            }
        }
        Ok(reached)
    }

    fn begin_basic_movement(
        &mut self,
        seat: Seat,
        path: &[Location],
        unit_instance_id: &IdentityHash,
        purpose: BasicMovementPurpose,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let unit = self
            .position
            .units
            .iter_mut()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *unit_instance_id)
            .ok_or(GameError::IllegalAction)?;
        if path
            .first()
            .is_none_or(|from| from.cell != unit.location || from.region != unit.region)
        {
            return Err(GameError::IllegalAction);
        }
        unit.tapped = true;
        self.position.pending_basic_movement = PendingField::Pending(PendingBasicMovement {
            path: path.to_vec(),
            path_index: 0,
            purpose,
            ranged_strike_used: false,
            seat,
            source_instance_id: unit_instance_id.clone(),
        });
        self.position.phase = Phase::Movement;
        self.position.decision_seat = seat;
        self.position.state_version += 1;
        outcomes.push("basic-movement-started", || {
            json!({
                "from": path[0],
                "path": path,
                "purpose": purpose.as_str(),
                "seat": seat,
                "sourceInstanceId": unit_instance_id,
                "to": path[path.len() - 1],
            })
        });
        Ok(())
    }

    fn apply_continue_basic_movement(
        &mut self,
        seat: Seat,
        unit_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_basic_movement
            .as_pending()
            .cloned()
            .ok_or(GameError::IllegalAction)?;
        if self.position.phase != Phase::Movement
            || pending.seat != seat
            || pending.source_instance_id != *unit_instance_id
        {
            return Err(GameError::IllegalAction);
        }
        let Some(next) = pending.path.get(pending.path_index + 1).copied() else {
            return self.finish_basic_movement(&pending, outcomes);
        };
        let unit = self
            .position
            .units
            .iter_mut()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *unit_instance_id)
            .ok_or(GameError::IllegalAction)?;
        let from = Location {
            cell: unit.location,
            region: unit.region,
        };
        if next.region != unit.region {
            return Err(GameError::IllegalAction);
        }
        Self::move_minion_to(unit, next.cell)?;
        self.position
            .pending_basic_movement
            .as_pending_mut()
            .ok_or(GameError::IllegalAction)?
            .path_index += 1;
        self.position.state_version += 1;
        outcomes.push("basic-movement-continued", || {
            json!({
                "from": from,
                "purpose": pending.purpose.as_str(),
                "seat": seat,
                "sourceInstanceId": unit_instance_id,
                "to": next,
            })
        });
        self.settle_nearby_enemy_stealth(outcomes);
        Ok(())
    }

    fn finish_basic_movement(
        &mut self,
        pending: &PendingBasicMovement,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let unit = self
            .position
            .units
            .iter()
            .find(|unit| {
                unit.controller == pending.seat
                    && unit.card.instance_id == pending.source_instance_id
            })
            .ok_or(GameError::IllegalAction)?;
        let from = *pending.path.first().ok_or(GameError::IllegalAction)?;
        let to = Location {
            cell: unit.location,
            region: unit.region,
        };
        self.position.pending_basic_movement = PendingField::Resolved;
        match pending.purpose {
            BasicMovementPurpose::MoveAndAttack => {
                if pending.path.last() != Some(&to) {
                    return Err(GameError::IllegalAction);
                }
                self.position.pending_combat = Some(PendingCombat {
                    allocations: Vec::new(),
                    attacker_instance_id: pending.source_instance_id.clone(),
                    attacker_kind: UnitKind::Minion,
                    attacking_seat: pending.seat,
                    cell: to.cell,
                    combatants: Vec::new(),
                    defenders: Vec::new(),
                    original_target: None,
                    region: to.region,
                    target_removed: false,
                });
                self.position.phase = Phase::Attack;
                outcomes.push("move-and-attack-activated", || {
                    json!({
                        "from": from,
                        "path": pending.path,
                        "seat": pending.seat,
                        "steps": pending.path.len() - 1,
                        "to": to,
                        "unitInstanceId": pending.source_instance_id,
                    })
                });
            }
            BasicMovementPurpose::Defend => {
                let fight_cell = self
                    .position
                    .pending_combat
                    .as_ref()
                    .ok_or(GameError::IllegalAction)?
                    .cell;
                if !Self::unit_occupies_cell(unit, fight_cell) {
                    return Err(GameError::IllegalAction);
                }
                let defender = UnitTarget::Minion {
                    instance_id: pending.source_instance_id.clone(),
                    seat: pending.seat,
                };
                let removes_site = self.position.pending_combat.as_ref().is_some_and(|combat| {
                    !combat.target_removed
                        && matches!(combat.original_target, Some(CombatTarget::Site { .. }))
                });
                let removed_target = removes_site.then(|| {
                    self.position
                        .pending_combat
                        .as_ref()
                        .and_then(|combat| combat.original_target.as_ref())
                        .map(|target| target.instance_id().clone())
                });
                let combat = self
                    .position
                    .pending_combat
                    .as_mut()
                    .ok_or(GameError::IllegalAction)?;
                combat.defenders.push(defender);
                combat.target_removed |= removes_site;
                self.position.phase = Phase::Defend;
                outcomes.push("defender-joined", || {
                    json!({
                        "from": from,
                        "instanceId": pending.source_instance_id,
                        "path": pending.path,
                        "seat": pending.seat,
                        "steps": pending.path.len() - 1,
                        "to": to,
                    })
                });
                if let Some(Some(instance_id)) = removed_target {
                    outcomes.push(
                        "original-target-removed",
                        || json!({ "instanceId": instance_id, "kind": "site" }),
                    );
                }
            }
        }
        self.position.decision_seat = pending.seat;
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_resolve_ranged_step(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_ranged_step
            .as_pending()
            .cloned()
            .ok_or(GameError::IllegalAction)?;
        if self.position.phase != Phase::RangedStep || pending.seat != action.seat {
            return Err(GameError::IllegalAction);
        }
        let legal = self
            .legal_actions()?
            .into_iter()
            .any(|candidate| candidate.descriptor == action.descriptor);
        if !legal {
            return Err(GameError::IllegalAction);
        }
        let ActionDescriptor::ResolveRangedStep {
            choice,
            from,
            path,
            to,
            unit_instance_id,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        self.position.pending_ranged_step = PendingField::Resolved;
        self.position.phase = Phase::Main;
        self.position.decision_seat = self.position.active_seat;
        if *choice == RangedStepChoice::Decline {
            self.position.state_version += 1;
            return Ok(());
        }
        let (Some(from), Some(path), Some(to)) = (from, path, to) else {
            return Err(GameError::IllegalAction);
        };
        let unit = self
            .position
            .units
            .iter_mut()
            .find(|unit| {
                unit.controller == action.seat && unit.card.instance_id == *unit_instance_id
            })
            .ok_or(GameError::IllegalAction)?;
        Self::move_minion_to(unit, to.cell)?;
        self.position.state_version += 1;
        outcomes.push("unit-stepped", || {
            json!({
                "from": from,
                "instanceId": unit_instance_id,
                "seat": action.seat,
                "sourceInstanceId": unit_instance_id,
                "steps": path.len() - 1,
                "to": to,
            })
        });
        self.settle_nearby_enemy_stealth(outcomes);
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one closed Defend transaction keeps validation, movement, and response state atomic"
    )]
    fn apply_defend_action(
        &mut self,
        seat: Seat,
        from: Location,
        path: &[Location],
        to: Location,
        unit_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.phase != Phase::Defend
            || path.is_empty()
            || path.first() != Some(&from)
            || path
                .iter()
                .any(|location| location.region != Region::Surface)
        {
            return Err(GameError::IllegalAction);
        }
        let pending_cell = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?
            .cell;
        if to
            != (Location {
                cell: pending_cell,
                region: Region::Surface,
            })
        {
            return Err(GameError::IllegalAction);
        }
        let final_anchor = *path.last().ok_or(GameError::IllegalAction)?;
        let (kind, _, start, profile) = self
            .defender_candidates()?
            .into_iter()
            .find(|(_, candidate, _, _)| candidate == unit_instance_id)
            .ok_or(GameError::IllegalAction)?;
        if start != from.cell
            || !self
                .movement_paths(from, profile)
                .iter()
                .any(|candidate| candidate == path)
            || profile
                .occupied_cells
                .map_or(final_anchor.cell != pending_cell, |area| {
                    translated_square(area, start, final_anchor.cell)
                        .is_none_or(|cells| !cells.contains(&pending_cell))
                })
        {
            return Err(GameError::IllegalAction);
        }
        let incremental = kind == UnitKind::Minion
            && self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == seat && unit.card.instance_id == *unit_instance_id)
                .is_some_and(|unit| {
                    matches!(
                        &self.rules.cards[usize::from(unit.card.card_id.0)].facts,
                        CardFacts::Minion(facts)
                            if facts.may_ranged_strike_once_during_basic_movement
                    )
                });
        if incremental {
            return self.begin_basic_movement(
                seat,
                path,
                unit_instance_id,
                BasicMovementPurpose::Defend,
                outcomes,
            );
        }
        let joined_start = outcomes.len();
        match kind {
            UnitKind::Avatar => {
                let avatar = &mut self.position.players[seat_index(seat)].avatar;
                avatar.location = final_anchor.cell;
                avatar.tapped = true;
            }
            UnitKind::Minion => {
                let reached =
                    self.walk_declared_defend_path(seat, path, unit_instance_id, outcomes)?;
                // An interrupted defender owes the rest of its path, so the combat it was joining
                // cannot be restored until that movement finishes.
                if reached + 1 < path.len() {
                    self.position.pending_basic_movement =
                        PendingField::Pending(PendingBasicMovement {
                            path: path.to_vec(),
                            path_index: reached,
                            purpose: BasicMovementPurpose::Defend,
                            ranged_strike_used: false,
                            seat,
                            source_instance_id: unit_instance_id.clone(),
                        });
                    if let Some(pending) = self.position.pending_deathrites.as_mut() {
                        pending.return_decision_seat = seat;
                        pending.return_phase = Phase::Movement;
                    } else {
                        self.position.phase = Phase::Movement;
                        self.position.decision_seat = seat;
                    }
                    self.position.state_version += 1;
                    outcomes.insert(joined_start, "basic-movement-started", || {
                        json!({
                            "from": path[0],
                            "path": path,
                            "purpose": BasicMovementPurpose::Defend.as_str(),
                            "seat": seat,
                            "sourceInstanceId": unit_instance_id,
                            "to": path[path.len() - 1],
                        })
                    });
                    return Ok(());
                }
            }
        }
        outcomes.insert(joined_start, "defender-joined", || {
            json!({
                "from": from,
                "instanceId": unit_instance_id,
                "path": path,
                "seat": seat,
                "steps": path.len() - 1,
                "to": final_anchor,
            })
        });
        let defender = match kind {
            UnitKind::Avatar => UnitTarget::Avatar {
                instance_id: unit_instance_id.clone(),
                seat,
            },
            UnitKind::Minion => UnitTarget::Minion {
                instance_id: unit_instance_id.clone(),
                seat,
            },
        };
        let removes_site = self
            .position
            .pending_combat
            .as_ref()
            .is_some_and(|pending| {
                !pending.target_removed
                    && matches!(pending.original_target, Some(CombatTarget::Site { .. }))
            });
        let removed_target = removes_site
            .then(|| {
                self.position
                    .pending_combat
                    .as_ref()
                    .and_then(|pending| pending.original_target.as_ref())
                    .map(|target| target.instance_id().clone())
            })
            .flatten();
        let pending = self
            .position
            .pending_combat
            .as_mut()
            .ok_or(GameError::IllegalAction)?;
        pending.defenders.push(defender);
        pending.target_removed |= removes_site;
        self.position.state_version += 1;
        if let Some(instance_id) = removed_target {
            outcomes.push(
                "original-target-removed",
                || json!({ "instanceId": instance_id, "kind": "site" }),
            );
        }
        Ok(())
    }

    fn apply_allocate_strike_action(
        &mut self,
        seat: Seat,
        amount: u64,
        target_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let amount = u16::try_from(amount).map_err(|_| GameError::IllegalAction)?;
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        if self.position.phase != Phase::Allocate || seat != pending.attacking_seat {
            return Err(GameError::IllegalAction);
        }
        let target = pending
            .combatants
            .get(pending.allocations.len())
            .ok_or(GameError::IllegalAction)?;
        let attack = self
            .combatant_strike_stats(
                pending.attacker_kind,
                pending.attacking_seat,
                &pending.attacker_instance_id,
            )?
            .amount;
        let assigned = pending
            .allocations
            .iter()
            .try_fold(0_u16, |total, allocation| {
                total.checked_add(allocation.amount)
            })
            .ok_or(GameError::IllegalAction)?;
        let remaining = attack
            .checked_sub(assigned)
            .ok_or(GameError::IllegalAction)?;
        let final_allocation = pending.allocations.len() + 1 == pending.combatants.len();
        if target.instance_id() != target_instance_id
            || amount > remaining
            || final_allocation && amount != remaining
        {
            return Err(GameError::IllegalAction);
        }
        let attacker_instance_id = pending.attacker_instance_id.clone();
        self.position
            .pending_combat
            .as_mut()
            .ok_or(GameError::IllegalAction)?
            .allocations
            .push(StrikeAllocation {
                amount,
                target_instance_id: target_instance_id.clone(),
            });
        self.position.state_version += 1;
        outcomes.push("strike-damage-allocated", || {
            json!({
                "amount": amount,
                "strikerInstanceId": attacker_instance_id,
                "targetInstanceId": target_instance_id,
            })
        });
        if final_allocation {
            self.resolve_pending_fight(outcomes)?;
        }
        Ok(())
    }

    fn apply_intercept_action(
        &mut self,
        seat: Seat,
        unit_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.phase != Phase::Intercept {
            return Err(GameError::IllegalAction);
        }
        let target = self
            .interceptor_candidates()?
            .into_iter()
            .find(|target| target.instance_id() == unit_instance_id)
            .ok_or(GameError::IllegalAction)?;
        let cell = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?
            .cell;
        match &target {
            UnitTarget::Avatar { .. } => {
                self.position.players[seat_index(seat)].avatar.tapped = true;
            }
            UnitTarget::Minion { .. } => {
                self.position
                    .units
                    .iter_mut()
                    .find(|unit| {
                        unit.controller == seat && unit.card.instance_id == *unit_instance_id
                    })
                    .ok_or(GameError::IllegalAction)?
                    .tapped = true;
            }
        }
        self.position
            .pending_combat
            .as_mut()
            .ok_or(GameError::IllegalAction)?
            .defenders
            .push(target);
        self.position.state_version += 1;
        outcomes.push("interceptor-joined", || {
            json!({
                "cell": cell,
                "instanceId": unit_instance_id,
                "seat": seat,
            })
        });
        Ok(())
    }

    fn apply_close_intercept_action(
        &mut self,
        seat: Seat,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        if self.position.phase != Phase::Intercept || seat != other_seat(pending.attacking_seat) {
            return Err(GameError::IllegalAction);
        }
        let interceptor_count = pending.defenders.len();
        let defenders = pending.defenders.clone();
        outcomes.push(
            "intercept-window-closed",
            || json!({ "interceptorCount": interceptor_count }),
        );
        if defenders.is_empty() {
            let attacking_seat = pending.attacking_seat;
            self.position.pending_combat = None;
            self.position.phase = Phase::Main;
            self.position.decision_seat = attacking_seat;
        } else {
            self.begin_fight(defenders, outcomes)?;
        }
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_declare_attack_action(
        &mut self,
        seat: Seat,
        target: &CombatTarget,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        if self.position.phase != Phase::Attack
            || pending.attacking_seat != seat
            || !self.attack_targets()?.contains(target)
        {
            return Err(GameError::IllegalAction);
        }
        let attacker_instance_id = pending.attacker_instance_id.clone();
        let attacker_kind = pending.attacker_kind;
        let attacking_seat = pending.attacking_seat;
        let attacker_stealthed =
            self.combatant_stealthed(attacker_kind, attacking_seat, &attacker_instance_id)?;
        let attacker_cells =
            self.combatant_occupied_cells(attacker_kind, attacking_seat, &attacker_instance_id)?;
        let cell = match target {
            CombatTarget::Site { instance_id, .. } => Cell::ALL.into_iter().find(|cell| {
                attacker_cells.contains(cell)
                    && self.position.sites[cell.index()]
                        .as_ref()
                        .is_some_and(|site| site.card.instance_id == *instance_id)
            }),
            CombatTarget::Avatar { instance_id, seat }
            | CombatTarget::Minion { instance_id, seat } => self
                .combatant_occupied_cells(
                    if matches!(target, CombatTarget::Avatar { .. }) {
                        UnitKind::Avatar
                    } else {
                        UnitKind::Minion
                    },
                    *seat,
                    instance_id,
                )?
                .iter()
                .copied()
                .filter(|cell| attacker_cells.contains(cell))
                .min(),
        }
        .ok_or(GameError::IllegalAction)?;
        let pending = self
            .position
            .pending_combat
            .as_mut()
            .ok_or(GameError::IllegalAction)?;
        pending.cell = cell;
        pending.original_target = Some(target.clone());
        outcomes.push("attack-declared", || {
            json!({
                "attackerInstanceId": attacker_instance_id,
                "cell": cell,
                "seat": seat,
                "target": target,
            })
        });
        if attacker_stealthed {
            match target {
                CombatTarget::Avatar { instance_id, seat } => self.begin_fight(
                    vec![UnitTarget::Avatar {
                        instance_id: instance_id.clone(),
                        seat: *seat,
                    }],
                    outcomes,
                )?,
                CombatTarget::Minion { instance_id, seat } => self.begin_fight(
                    vec![UnitTarget::Minion {
                        instance_id: instance_id.clone(),
                        seat: *seat,
                    }],
                    outcomes,
                )?,
                CombatTarget::Site { .. } => self.resolve_undefended_site_strike(outcomes)?,
            }
        } else {
            self.position.decision_seat = target.seat();
            self.position.phase = Phase::Defend;
        }
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_close_defend_action(
        &mut self,
        seat: Seat,
        original_target_participates: bool,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let target = pending
            .original_target
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let participation_is_legal = if matches!(target, CombatTarget::Site { .. }) {
            !original_target_participates
        } else if pending.defenders.is_empty() {
            original_target_participates
        } else {
            true
        };
        if self.position.phase != Phase::Defend || seat != target.seat() || !participation_is_legal
        {
            return Err(GameError::IllegalAction);
        }
        let defender_count = pending.defenders.len();
        let target = target.clone();
        outcomes.push("defend-window-closed", || {
            json!({
                "defenderCount": defender_count,
                "originalTargetParticipates": original_target_participates,
            })
        });
        match &target {
            CombatTarget::Site { .. } if defender_count == 0 => {
                self.resolve_undefended_site_strike(outcomes)?;
            }
            CombatTarget::Site { .. } => {
                let combatants = self
                    .position
                    .pending_combat
                    .as_ref()
                    .ok_or(GameError::IllegalAction)?
                    .defenders
                    .clone();
                self.begin_fight(combatants, outcomes)?;
            }
            CombatTarget::Avatar { instance_id, seat }
            | CombatTarget::Minion { instance_id, seat } => {
                if !original_target_participates {
                    outcomes.push(
                        "original-target-removed",
                        || json!({ "instanceId": instance_id, "kind": target.kind() }),
                    );
                }
                let mut combatants = self
                    .position
                    .pending_combat
                    .as_ref()
                    .ok_or(GameError::IllegalAction)?
                    .defenders
                    .clone();
                if original_target_participates {
                    combatants.push(match target {
                        CombatTarget::Avatar { .. } => UnitTarget::Avatar {
                            instance_id: instance_id.clone(),
                            seat: *seat,
                        },
                        CombatTarget::Minion { .. } => UnitTarget::Minion {
                            instance_id: instance_id.clone(),
                            seat: *seat,
                        },
                        CombatTarget::Site { .. } => unreachable!(),
                    });
                }
                self.position
                    .pending_combat
                    .as_mut()
                    .ok_or(GameError::IllegalAction)?
                    .target_removed = !original_target_participates;
                self.begin_fight(combatants, outcomes)?;
            }
        }
        self.position.state_version += 1;
        Ok(())
    }

    fn resolve_undefended_site_strike(
        &mut self,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let CombatTarget::Site {
            instance_id: site_instance_id,
            seat: target_seat,
        } = pending
            .original_target
            .as_ref()
            .ok_or(GameError::IllegalAction)?
        else {
            return Err(GameError::IllegalAction);
        };
        let attacker_id = pending.attacker_instance_id.clone();
        let attacker_kind = pending.attacker_kind;
        let attacking_seat = pending.attacking_seat;
        let cell = pending.cell;
        let site_instance_id = site_instance_id.clone();
        let target_seat = *target_seat;
        let site = self.position.sites[cell.index()]
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        if site.card.instance_id != site_instance_id || site.controller != target_seat {
            return Err(GameError::IllegalAction);
        }
        let (attack, _) =
            self.combatant_attack_and_lethal(attacker_kind, attacking_seat, &attacker_id)?;
        let lost_stealth =
            self.mark_unit_interaction(attacker_kind, attacking_seat, &attacker_id)?;

        let avatar = &mut self.position.players[seat_index(target_seat)].avatar;
        let old_life = avatar.life;
        avatar.life = avatar.life.saturating_sub(attack);
        let lost = old_life - avatar.life;
        let reached_deaths_door = old_life > 0 && avatar.life == 0;
        if reached_deaths_door {
            avatar.death_door_turn = Some(self.position.turn_number);
        }
        let life = avatar.life;
        self.position.pending_combat = None;
        self.position.phase = Phase::Main;
        self.position.decision_seat = self.position.active_seat;

        outcomes.push("undefended-site-struck", || {
            json!({
                "amount": attack,
                "attackerInstanceId": attacker_id,
                "cell": cell,
                "siteInstanceId": site_instance_id,
            })
        });
        if lost > 0 {
            outcomes.push(
                "avatar-life-lost",
                || json!({ "amount": lost, "life": life, "seat": target_seat }),
            );
        }
        if reached_deaths_door {
            let turn_number = self.position.turn_number;
            outcomes.push(
                "avatar-reached-deaths-door",
                || json!({ "seat": target_seat, "turnNumber": turn_number }),
            );
        }
        if lost_stealth {
            outcomes.push(
                "stealth-lost",
                || json!({ "instanceId": attacker_id, "seat": attacking_seat }),
            );
        }
        Ok(())
    }

    fn combatant_attack_and_lethal(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<(u16, bool), GameError> {
        match kind {
            UnitKind::Avatar => {
                let avatar = &self.position.players[seat_index(seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                let CardFacts::Avatar(facts) =
                    &self.rules.cards[usize::from(avatar.card.card_id.0)].facts
                else {
                    return Err(GameError::IllegalAction);
                };
                let bonus = Self::temporary_power_bonus(&avatar.temporary_power_sources)?
                    .checked_add(self.carried_power_bonus(UnitKind::Avatar, seat, instance_id)?)
                    .ok_or(GameError::IllegalAction)?;
                Ok((
                    u16::from(facts.attack)
                        .checked_add(bonus)
                        .ok_or(GameError::IllegalAction)?,
                    self.carried_lethal(UnitKind::Avatar, seat, instance_id)?,
                ))
            }
            UnitKind::Minion => {
                let unit = self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == *instance_id && unit.controller == seat)
                    .ok_or(GameError::IllegalAction)?;
                let (attack, _, lethal) = self.minion_current_stats(unit)?;
                Ok((attack, lethal))
            }
        }
    }

    fn combatant_strike_stats(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<StrikeStats, GameError> {
        let (current_power, lethal) = self.combatant_attack_and_lethal(kind, seat, instance_id)?;
        let lance_count = if kind == UnitKind::Minion {
            self.position
                .units
                .iter()
                .find(|unit| unit.controller == seat && unit.card.instance_id == *instance_id)
                .ok_or(GameError::IllegalAction)?
                .carried_lance_count
        } else {
            0
        };
        let amount = current_power
            .checked_add(u16::from(lance_count))
            .ok_or(GameError::IllegalAction)?;
        Ok(StrikeStats {
            amount,
            current_power,
            lance_count,
            lethal,
        })
    }

    fn combatant_strikes_first_while_attacking(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<bool, GameError> {
        if kind == UnitKind::Avatar {
            return Ok(false);
        }
        let unit = self
            .position
            .units
            .iter()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *instance_id)
            .ok_or(GameError::IllegalAction)?;
        Ok(matches!(
            &self.rules.cards[usize::from(unit.card.card_id.0)].facts,
            CardFacts::Minion(facts) if facts.strikes_first_while_attacking
        ))
    }

    fn break_lance(
        &mut self,
        seat: Seat,
        instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let unit = self
            .position
            .units
            .iter_mut()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *instance_id)
            .ok_or(GameError::IllegalAction)?;
        let count = std::mem::take(&mut unit.carried_lance_count);
        if count > 0 {
            outcomes.push("lance-broken", || {
                json!({
                    "bearerInstanceId": instance_id,
                    "count": count,
                    "sourceInstanceId": instance_id,
                })
            });
        }
        Ok(())
    }

    fn record_unit_interaction(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.mark_unit_interaction(kind, seat, instance_id)? {
            outcomes.push(
                "stealth-lost",
                || json!({ "instanceId": instance_id, "seat": seat }),
            );
        }
        Ok(())
    }

    fn mark_unit_interaction(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<bool, GameError> {
        match kind {
            UnitKind::Avatar => {
                let avatar = &mut self.position.players[seat_index(seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                avatar.last_interacted_turn = Some(self.position.turn_number);
                Ok(false)
            }
            UnitKind::Minion => {
                let unit = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| unit.card.instance_id == *instance_id && unit.controller == seat)
                    .ok_or(GameError::IllegalAction)?;
                unit.last_interacted_turn = Some(self.position.turn_number);
                let lost_stealth = unit.stealthed;
                unit.stealthed = false;
                Ok(lost_stealth)
            }
        }
    }

    fn begin_fight(
        &mut self,
        mut combatants: Vec<UnitTarget>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        combatants.sort_unstable_by(|left, right| left.instance_id().cmp(right.instance_id()));
        if combatants.is_empty() {
            return Err(GameError::IllegalAction);
        }
        let attacker_instance_id = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?
            .attacker_instance_id
            .clone();
        let combatant_instance_ids: Vec<_> = combatants
            .iter()
            .map(|target| target.instance_id().clone())
            .collect();
        let pending = self
            .position
            .pending_combat
            .as_mut()
            .ok_or(GameError::IllegalAction)?;
        pending.allocations.clear();
        pending.combatants = combatants;
        outcomes.push("fight-started", || {
            json!({
                "attackerInstanceId": attacker_instance_id,
                "combatantInstanceIds": combatant_instance_ids,
            })
        });
        if self
            .position
            .pending_combat
            .as_ref()
            .is_some_and(|pending| pending.combatants.len() == 1)
        {
            let pending = self
                .position
                .pending_combat
                .as_ref()
                .ok_or(GameError::IllegalAction)?;
            let amount = self
                .combatant_strike_stats(
                    pending.attacker_kind,
                    pending.attacking_seat,
                    &pending.attacker_instance_id,
                )?
                .amount;
            let target_instance_id = pending.combatants[0].instance_id().clone();
            self.position
                .pending_combat
                .as_mut()
                .ok_or(GameError::IllegalAction)?
                .allocations
                .push(StrikeAllocation {
                    amount,
                    target_instance_id: target_instance_id.clone(),
                });
            outcomes.push("strike-damage-allocated", || {
                json!({
                    "amount": amount,
                    "strikerInstanceId": attacker_instance_id,
                    "targetInstanceId": target_instance_id,
                })
            });
            self.resolve_pending_fight(outcomes)
        } else {
            self.position.phase = Phase::Allocate;
            self.position.decision_seat = self
                .position
                .pending_combat
                .as_ref()
                .ok_or(GameError::IllegalAction)?
                .attacking_seat;
            Ok(())
        }
    }

    fn resolve_pending_fight(&mut self, outcomes: &mut OutcomeLog<'_>) -> Result<(), GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        if pending.combatants.is_empty()
            || pending.allocations.len() != pending.combatants.len()
            || pending
                .allocations
                .iter()
                .zip(&pending.combatants)
                .any(|(allocation, target)| allocation.target_instance_id != *target.instance_id())
        {
            return Err(GameError::IllegalAction);
        }
        let pending = pending.clone();
        let attacker = self.combatant_strike_stats(
            pending.attacker_kind,
            pending.attacking_seat,
            &pending.attacker_instance_id,
        )?;
        let attacker_enabled = match pending.attacker_kind {
            UnitKind::Avatar => true,
            UnitKind::Minion => self
                .position
                .units
                .iter()
                .find(|unit| unit.card.instance_id == pending.attacker_instance_id)
                .is_some_and(|unit| !self.minion_is_disabled(unit)),
        };
        let attacker_struck = attacker_enabled
            && (attacker.lance_count > 0
                || self.combatant_strikes_first_while_attacking(
                    pending.attacker_kind,
                    pending.attacking_seat,
                    &pending.attacker_instance_id,
                )?);
        let first_combatant_instance_ids = pending
            .combatants
            .iter()
            .filter_map(|target| {
                let UnitTarget::Minion { .. } = target else {
                    return None;
                };
                self.position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == *target.instance_id())
                    .filter(|unit| !self.minion_is_disabled(unit))
                    .and_then(|unit| {
                        (unit.carried_lance_count > 0).then(|| target.instance_id().clone())
                    })
            })
            .collect::<Vec<_>>();
        if attacker_struck || !first_combatant_instance_ids.is_empty() {
            let continuation = FirstStrikeContinuation {
                attacker_struck,
                first_combatant_instance_ids,
                pending,
            };
            let interrupted = self.resolve_fight_window(
                &continuation.pending,
                attacker_struck,
                &continuation.first_combatant_instance_ids,
                Some(DeathriteContinuation::FirstStrike(continuation.clone())),
                outcomes,
            )?;
            if !interrupted {
                self.continue_after_first_strike(continuation, outcomes)?;
            }
        } else {
            let combatant_ids = pending
                .combatants
                .iter()
                .map(|target| target.instance_id().clone())
                .collect::<Vec<_>>();
            self.resolve_fight_window(&pending, true, &combatant_ids, None, outcomes)?;
        }
        Ok(())
    }

    fn continue_after_first_strike(
        &mut self,
        mut continuation: FirstStrikeContinuation,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let attacker_survived = continuation.pending.attacker_kind == UnitKind::Avatar
            || self
                .position
                .units
                .iter()
                .any(|unit| unit.card.instance_id == continuation.pending.attacker_instance_id);
        continuation.pending.combatants.retain(|target| {
            matches!(target, UnitTarget::Avatar { .. })
                || self
                    .position
                    .units
                    .iter()
                    .any(|unit| unit.card.instance_id == *target.instance_id())
        });
        if self.position.terminal.is_some()
            || !attacker_survived
            || continuation.pending.combatants.is_empty()
        {
            self.position.pending_combat = None;
            if self.position.terminal.is_none() {
                self.position.phase = Phase::Main;
                self.position.decision_seat = self.position.active_seat;
            }
            return Ok(());
        }
        let first_ids = continuation
            .first_combatant_instance_ids
            .into_iter()
            .collect::<BTreeSet<_>>();
        let normal_ids = continuation
            .pending
            .combatants
            .iter()
            .filter(|target| !first_ids.contains(target.instance_id()))
            .map(|target| target.instance_id().clone())
            .collect::<Vec<_>>();
        self.position.pending_combat = Some(continuation.pending.clone());
        self.resolve_fight_window(
            &continuation.pending,
            !continuation.attacker_struck,
            &normal_ids,
            None,
            outcomes,
        )?;
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one strike window keeps simultaneous damage and event order explicit"
    )]
    fn resolve_fight_window(
        &mut self,
        pending: &PendingCombat,
        attacker_strikes: bool,
        striking_combatant_ids: &[IdentityHash],
        continuation: Option<DeathriteContinuation>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<bool, GameError> {
        let attacker_id = pending.attacker_instance_id.clone();
        let attacker_kind = pending.attacker_kind;
        let attacking_seat = pending.attacking_seat;
        let attacker = self.combatant_strike_stats(attacker_kind, attacking_seat, &attacker_id)?;
        let attacker_can_strike = attacker_strikes
            && match attacker_kind {
                UnitKind::Avatar => true,
                UnitKind::Minion => self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == attacker_id)
                    .is_some_and(|unit| !self.minion_is_disabled(unit)),
            };
        let striking_ids = striking_combatant_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let return_sources = pending
            .combatants
            .iter()
            .filter(|target| striking_ids.contains(target.instance_id()))
            .filter_map(|target| {
                let kind = match target {
                    UnitTarget::Avatar { .. } => UnitKind::Avatar,
                    UnitTarget::Minion { .. } => UnitKind::Minion,
                };
                let can_strike = match kind {
                    UnitKind::Avatar => true,
                    UnitKind::Minion => self
                        .position
                        .units
                        .iter()
                        .find(|unit| unit.card.instance_id == *target.instance_id())
                        .is_some_and(|unit| !self.minion_is_disabled(unit)),
                };
                can_strike.then(|| {
                    self.combatant_strike_stats(kind, target.seat(), target.instance_id())
                        .map(|stats| (kind, target.clone(), stats))
                })
            })
            .collect::<Result<Vec<_>, GameError>>()?;
        let combatant_statuses = pending
            .combatants
            .iter()
            .map(|target| match target {
                UnitTarget::Avatar { .. } => Ok(None),
                UnitTarget::Minion { instance_id, .. } => {
                    self.minion_damage_status(instance_id).map(Some)
                }
            })
            .collect::<Result<Vec<_>, GameError>>()?;

        let mut stealth_losses = Vec::new();
        if attacker_can_strike
            && self.mark_unit_interaction(attacker_kind, attacking_seat, &attacker_id)?
        {
            stealth_losses.push((attacker_id.clone(), attacking_seat));
        }
        for (kind, target, _) in &return_sources {
            if self.mark_unit_interaction(*kind, target.seat(), target.instance_id())? {
                stealth_losses.push((target.instance_id().clone(), target.seat()));
            }
        }
        let return_damage_sources = return_sources
            .iter()
            .map(|(_, _, strike)| {
                (
                    strike.amount,
                    UnitDamageSource {
                        current_power: strike.current_power,
                        lethal: strike.lethal,
                    },
                )
            })
            .collect::<Vec<_>>();
        let attacker_damage = if return_damage_sources.is_empty() {
            DamageResult {
                minion_died: false,
                avatar_defeated: false,
            }
        } else {
            self.apply_simultaneous_unit_damage(
                attacker_kind,
                attacking_seat,
                &attacker_id,
                &return_damage_sources,
                outcomes,
            )?
        };
        let mut combatant_results = Vec::with_capacity(pending.combatants.len());
        if attacker_can_strike {
            for (target, minion_status) in pending.combatants.iter().zip(combatant_statuses) {
                let allocation = pending
                    .allocations
                    .iter()
                    .find(|allocation| allocation.target_instance_id == *target.instance_id())
                    .ok_or(GameError::IllegalAction)?;
                let kind = match target {
                    UnitTarget::Avatar { .. } => UnitKind::Avatar,
                    UnitTarget::Minion { .. } => UnitKind::Minion,
                };
                let result = self.apply_simple_damage_with_status(
                    kind,
                    target.seat(),
                    target.instance_id(),
                    allocation.amount,
                    UnitDamageSource {
                        current_power: attacker.current_power,
                        lethal: attacker.lethal,
                    },
                    minion_status,
                    outcomes,
                )?;
                combatant_results.push((target.clone(), result));
            }
        }
        for (instance_id, seat) in stealth_losses {
            outcomes.push(
                "stealth-lost",
                || json!({ "instanceId": instance_id, "seat": seat }),
            );
        }
        if attacker_can_strike && attacker.lance_count > 0 {
            self.break_lance(attacking_seat, &attacker_id, outcomes)?;
        }
        for (_, target, strike) in &return_sources {
            if strike.lance_count > 0 {
                self.break_lance(target.seat(), target.instance_id(), outcomes)?;
            }
        }
        let mut dead_minions = Vec::new();
        if attacker_damage.minion_died {
            dead_minions.push(attacker_id);
        }
        dead_minions.extend(
            combatant_results
                .iter()
                .filter(|(_, result)| result.minion_died)
                .map(|(target, _)| target.instance_id().clone()),
        );
        let mut defeated_avatars = Vec::new();
        if attacker_damage.avatar_defeated {
            defeated_avatars.push(attacking_seat);
        }
        defeated_avatars.extend(
            combatant_results
                .iter()
                .filter_map(|(target, result)| result.avatar_defeated.then_some(target.seat())),
        );
        self.position.pending_combat = None;
        self.position.decision_seat = self.position.active_seat;
        self.position.phase = Phase::Main;
        let interrupted = !dead_minions.is_empty() || !defeated_avatars.is_empty();
        if interrupted {
            self.begin_minion_deaths_with_continuation(
                &dead_minions,
                &defeated_avatars,
                Phase::Main,
                self.position.active_seat,
                continuation,
                outcomes,
            )?;
        }
        Ok(interrupted)
    }

    fn apply_simultaneous_unit_damage(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
        sources: &[(u16, UnitDamageSource)],
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<DamageResult, GameError> {
        let attempted = sources
            .iter()
            .try_fold(0_u16, |total, (amount, _)| total.checked_add(*amount))
            .ok_or(GameError::IllegalAction)?;
        if kind == UnitKind::Avatar {
            return self.apply_simple_damage(
                kind,
                seat,
                instance_id,
                attempted,
                UnitDamageSource {
                    current_power: 0,
                    lethal: false,
                },
                outcomes,
            );
        }
        let (index, _, defense, prevention) = self.simple_minion_combatant(instance_id)?;
        let disabled = self.minion_is_disabled(&self.position.units[index]);
        let mut contributions = sources.iter().map(|(amount, source)| {
            let dealt =
                if disabled {
                    *amount
                } else {
                    match prevention {
                        Some(DamagePrevention::PreventsDamageFromUnitsWithPowerAtLeast(
                            threshold,
                        )) if source.current_power >= u16::from(threshold) => 0,
                        Some(DamagePrevention::TakesOneLessDamage) => amount.saturating_sub(1),
                        _ => *amount,
                    }
                };
            (dealt, source.lethal && dealt > 0)
        });
        let (unwarded, lethal_dealt) = contributions
            .try_fold((0_u16, false), |(total, any_lethal), (amount, lethal)| {
                total
                    .checked_add(amount)
                    .map(|next| (next, any_lethal || lethal))
            })
            .ok_or(GameError::IllegalAction)?;
        let unit = &mut self.position.units[index];
        let ward_broken = unwarded > 0 && unit.warded;
        if ward_broken {
            unit.warded = false;
        }
        let dealt = if ward_broken { 0 } else { unwarded };
        unit.damage = unit.damage.saturating_add(dealt);
        let accumulated = unit.damage;
        let awakened = dealt > 0 && unit.disabled_until_damaged;
        if awakened {
            unit.disabled_until_damaged = false;
        }
        outcomes.push("damage-dealt", || {
            let mut payload = json!({
                "amount": dealt,
                "direct": true,
                "instanceId": instance_id,
                "seat": seat,
            });
            if !ward_broken {
                payload["accumulated"] = json!(accumulated);
            }
            if dealt < attempted {
                payload["attemptedAmount"] = json!(attempted);
                payload["prevented"] = json!(true);
            }
            payload
        });
        if ward_broken {
            outcomes.push(
                "ward-broken",
                || json!({ "instanceId": instance_id, "seat": seat }),
            );
        }
        if awakened {
            outcomes.push(
                "minion-awakened",
                || json!({ "instanceId": instance_id, "seat": seat }),
            );
        }
        Ok(DamageResult {
            minion_died: accumulated > 0 && (accumulated >= defense || lethal_dealt),
            avatar_defeated: false,
        })
    }

    fn apply_simple_damage(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
        amount: u16,
        source: UnitDamageSource,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<DamageResult, GameError> {
        let status = if kind == UnitKind::Minion {
            Some(self.minion_damage_status(instance_id)?)
        } else {
            None
        };
        self.apply_simple_damage_with_status(
            kind,
            seat,
            instance_id,
            amount,
            source,
            status,
            outcomes,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "the damage transaction keeps its typed source, snapshot, and event sink explicit"
    )]
    fn apply_simple_damage_with_status(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
        amount: u16,
        source: UnitDamageSource,
        minion_status: Option<MinionDamageStatus>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<DamageResult, GameError> {
        match kind {
            UnitKind::Minion => {
                let MinionDamageStatus {
                    damage_prevention,
                    defense,
                    disabled,
                    index,
                } = minion_status.ok_or(GameError::IllegalAction)?;
                if self.position.units[index].card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                let unit = &mut self.position.units[index];
                let ward_broken = amount > 0 && unit.warded;
                if ward_broken {
                    unit.warded = false;
                }
                let dealt = if ward_broken {
                    0
                } else if disabled {
                    amount
                } else {
                    match damage_prevention {
                        Some(DamagePrevention::PreventsDamageFromUnitsWithPowerAtLeast(
                            threshold,
                        )) if source.current_power >= u16::from(threshold) => 0,
                        Some(DamagePrevention::TakesOneLessDamage) => amount.saturating_sub(1),
                        _ => amount,
                    }
                };
                let prevented = dealt < amount;
                unit.damage = unit.damage.saturating_add(dealt);
                let accumulated = unit.damage;
                let awakened = dealt > 0 && unit.disabled_until_damaged;
                if awakened {
                    unit.disabled_until_damaged = false;
                }
                outcomes.push("damage-dealt", || {
                    let mut payload = json!({
                        "amount": dealt,
                        "direct": true,
                        "instanceId": instance_id,
                        "seat": seat,
                    });
                    if !ward_broken {
                        payload["accumulated"] = json!(accumulated);
                    }
                    if prevented {
                        payload["attemptedAmount"] = json!(amount);
                        payload["prevented"] = json!(true);
                    }
                    payload
                });
                if ward_broken {
                    outcomes.push(
                        "ward-broken",
                        || json!({ "instanceId": instance_id, "seat": seat }),
                    );
                }
                if awakened {
                    outcomes.push(
                        "minion-awakened",
                        || json!({ "instanceId": instance_id, "seat": seat }),
                    );
                }
                Ok(DamageResult {
                    minion_died: accumulated > 0
                        && (accumulated >= defense || source.lethal && dealt > 0),
                    avatar_defeated: false,
                })
            }
            UnitKind::Avatar => {
                let avatar = &mut self.position.players[seat_index(seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                if avatar.life == 0 {
                    if amount == 0 {
                        return Ok(DamageResult {
                            minion_died: false,
                            avatar_defeated: false,
                        });
                    }
                    if avatar.death_door_turn != Some(self.position.turn_number) {
                        outcomes.push("damage-dealt", || {
                            json!({
                                "amount": amount,
                                "direct": true,
                                "instanceId": instance_id,
                                "seat": seat,
                            })
                        });
                        outcomes.push(
                            "death-blow",
                            || json!({ "instanceId": instance_id, "seat": seat }),
                        );
                        return Ok(DamageResult {
                            minion_died: false,
                            avatar_defeated: true,
                        });
                    }
                    outcomes.push("damage-dealt", || {
                        json!({
                            "amount": 0,
                            "attemptedAmount": amount,
                            "direct": true,
                            "instanceId": instance_id,
                            "prevented": true,
                            "seat": seat,
                        })
                    });
                    return Ok(DamageResult {
                        minion_died: false,
                        avatar_defeated: false,
                    });
                }
                if amount == 0 {
                    return Ok(DamageResult {
                        minion_died: false,
                        avatar_defeated: false,
                    });
                }
                let old_life = avatar.life;
                avatar.life = avatar.life.saturating_sub(amount);
                let lost = old_life - avatar.life;
                let reached_deaths_door = avatar.life == 0;
                if reached_deaths_door {
                    avatar.death_door_turn = Some(self.position.turn_number);
                }
                let life = avatar.life;
                outcomes.push("damage-dealt", || {
                    json!({
                        "amount": amount,
                        "direct": true,
                        "instanceId": instance_id,
                        "seat": seat,
                    })
                });
                outcomes.push(
                    "avatar-life-lost",
                    || json!({ "amount": lost, "life": life, "seat": seat }),
                );
                if reached_deaths_door {
                    let turn_number = self.position.turn_number;
                    outcomes.push(
                        "avatar-reached-deaths-door",
                        || json!({ "seat": seat, "turnNumber": turn_number }),
                    );
                }
                Ok(DamageResult {
                    minion_died: false,
                    avatar_defeated: false,
                })
            }
        }
    }

    fn simple_minion_combatant(
        &self,
        instance_id: &IdentityHash,
    ) -> Result<(usize, u16, u16, Option<DamagePrevention>), GameError> {
        let (index, unit) = self
            .position
            .units
            .iter()
            .enumerate()
            .find(|(_, unit)| unit.card.instance_id == *instance_id)
            .ok_or(GameError::IllegalAction)?;
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let (attack, defense, _) = self.minion_current_stats(unit)?;
        Ok((index, attack, defense, facts.damage_prevention))
    }

    fn minion_damage_status(
        &self,
        instance_id: &IdentityHash,
    ) -> Result<MinionDamageStatus, GameError> {
        let (index, _, defense, damage_prevention) = self.simple_minion_combatant(instance_id)?;
        Ok(MinionDamageStatus {
            damage_prevention,
            defense,
            disabled: self.minion_is_disabled(&self.position.units[index]),
            index,
        })
    }

    fn make_deathrite_batch(
        &self,
        sources: Vec<PendingDeathriteSource>,
    ) -> Option<PendingDeathriteBatch> {
        if sources.is_empty() {
            return None;
        }
        let (active, non_active): (Vec<_>, Vec<_>) = sources
            .into_iter()
            .partition(|source| source.controller == self.position.active_seat);
        let active_needs_order = active.len() > 1;
        let non_active_needs_order = non_active.len() > 1;
        let stage = if active_needs_order {
            DeathriteStage::ActiveOrder
        } else if non_active_needs_order {
            DeathriteStage::NonActiveOrder
        } else {
            DeathriteStage::Resolve
        };
        let resolving = if stage == DeathriteStage::Resolve {
            non_active.iter().chain(&active).cloned().collect()
        } else {
            Vec::new()
        };
        Some(PendingDeathriteBatch {
            active_order: if active_needs_order {
                Vec::new()
            } else {
                active.clone()
            },
            active_remaining: if active_needs_order {
                active
            } else {
                Vec::new()
            },
            non_active_order: if non_active_needs_order {
                Vec::new()
            } else {
                non_active.clone()
            },
            non_active_remaining: if non_active_needs_order {
                non_active
            } else {
                Vec::new()
            },
            resolving,
            stage,
        })
    }

    fn collect_minion_deaths(
        &mut self,
        instance_ids: &[IdentityHash],
    ) -> Result<(Vec<PendingDeathriteSource>, Vec<UnitPosition>), GameError> {
        let mut seen = BTreeSet::new();
        let mut corpses = Vec::new();
        let mut sources = Vec::new();
        let mut pending_ids = instance_ids.to_vec();
        while !pending_ids.is_empty() {
            let mut batch = Vec::new();
            let mut batch_ids = BTreeSet::new();
            for instance_id in &pending_ids {
                if seen.contains(instance_id) || !batch_ids.insert(instance_id.clone()) {
                    continue;
                }
                batch.push(
                    self.position
                        .units
                        .iter()
                        .find(|unit| unit.card.instance_id == *instance_id)
                        .cloned()
                        .ok_or(GameError::IllegalAction)?,
                );
            }
            for unit in &batch {
                let CardFacts::Minion(facts) =
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts
                else {
                    return Err(GameError::IllegalAction);
                };
                let has_deathrite = facts.deathrite_damage_each_unit_here.is_some()
                    || facts.deathrite_draw_site
                    || facts.deathrite_heal.is_some()
                    || facts.deathrite_lose_life_per_nearby_site_controlled;
                if has_deathrite && !self.minion_is_disabled(unit) {
                    let (current_power, _, lethal) = self.minion_current_stats(unit)?;
                    sources.push(PendingDeathriteSource {
                        controller: unit.controller,
                        current_power,
                        instance_id: unit.card.instance_id.clone(),
                        lethal,
                        unit: unit.clone(),
                    });
                }
                seen.insert(unit.card.instance_id.clone());
            }
            corpses.extend(batch);
            self.position
                .units
                .retain(|unit| !seen.contains(&unit.card.instance_id));
            pending_ids = self
                .position
                .units
                .iter()
                .map(|unit| {
                    self.minion_current_stats(unit).map(|(_, defense, _)| {
                        (unit.damage > 0 && unit.damage >= defense)
                            .then(|| unit.card.instance_id.clone())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();
        }
        Ok((sources, corpses))
    }

    fn settle_static_power_deaths(
        &mut self,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.terminal.is_some() || self.position.pending_deathrites.is_some() {
            return Ok(());
        }
        let deaths = self
            .position
            .units
            .iter()
            .map(|unit| {
                self.minion_current_stats(unit).map(|(_, defense, _)| {
                    (unit.damage > 0 && unit.damage >= defense)
                        .then(|| unit.card.instance_id.clone())
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if deaths.is_empty() {
            return Ok(());
        }
        self.begin_minion_deaths(
            &deaths,
            &[],
            self.position.phase,
            self.position.decision_seat,
            outcomes,
        )
    }

    fn settle_lower_region_minion_deaths(
        &mut self,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.terminal.is_some() || self.position.pending_deathrites.is_some() {
            return Ok(());
        }
        let deaths = self.lower_region_minion_deaths();
        if deaths.is_empty() {
            return Ok(());
        }
        self.begin_minion_deaths(
            &deaths,
            &[],
            self.position.phase,
            self.position.decision_seat,
            outcomes,
        )
    }

    /// Every minion whose current region can no longer keep it alive.
    fn lower_region_minion_deaths(&self) -> Vec<IdentityHash> {
        self.position
            .units
            .iter()
            .filter(|unit| matches!(unit.region, Region::Underground | Region::Underwater))
            .filter(|unit| {
                let CardFacts::Minion(facts) =
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts
                else {
                    return true;
                };
                self.minion_is_disabled(unit)
                    || match unit.region {
                        Region::Underground => !facts.burrowing,
                        Region::Underwater => !facts.submerge,
                        Region::Surface | Region::Void => false,
                    }
                    || Self::unit_occupied_cells(unit)
                        .iter()
                        .any(|cell| !self.location_exists_in_region(*cell, unit.region))
            })
            .map(|unit| unit.card.instance_id.clone())
            .collect()
    }

    fn begin_minion_deaths(
        &mut self,
        instance_ids: &[IdentityHash],
        defeated_avatars: &[Seat],
        return_phase: Phase,
        return_decision_seat: Seat,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        self.begin_minion_deaths_with_continuation(
            instance_ids,
            defeated_avatars,
            return_phase,
            return_decision_seat,
            None,
            outcomes,
        )
    }

    fn begin_minion_deaths_with_continuation(
        &mut self,
        instance_ids: &[IdentityHash],
        defeated_avatars: &[Seat],
        return_phase: Phase,
        return_decision_seat: Seat,
        continuation: Option<DeathriteContinuation>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let (sources, corpses) = self.collect_minion_deaths(instance_ids)?;
        let batch = self.make_deathrite_batch(sources);
        let mut unique_defeated = Vec::new();
        for seat in defeated_avatars {
            if !unique_defeated.contains(seat) {
                unique_defeated.push(*seat);
            }
        }
        let pending = PendingDeathrites {
            batches: batch.into_iter().collect(),
            continuation,
            corpses,
            deck_losers: Vec::new(),
            deferred_magic_resolved: None,
            defeated_avatars: unique_defeated,
            return_decision_seat,
            return_phase,
        };
        self.drive_deathrites(pending, outcomes)
    }

    fn apply_deathrite_order_action(
        &mut self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.phase != Phase::DeathriteOrder
            || seat != self.position.decision_seat
            || !self
                .pending_deathrite_order()?
                .iter()
                .any(|source| source.instance_id == *source_instance_id)
        {
            return Err(GameError::IllegalAction);
        }
        let mut pending = self
            .position
            .pending_deathrites
            .take()
            .ok_or(GameError::IllegalAction)?;
        Self::commit_deathrite_order(&mut pending, source_instance_id)?;
        outcomes.push(
            "deathrite-order-committed",
            || json!({ "seat": seat, "sourceInstanceId": source_instance_id }),
        );
        self.drive_deathrites(pending, outcomes)?;
        self.position.state_version += 1;
        Ok(())
    }

    fn commit_deathrite_order(
        pending: &mut PendingDeathrites,
        source_instance_id: &IdentityHash,
    ) -> Result<(), GameError> {
        let batch = pending
            .batches
            .first_mut()
            .ok_or(GameError::IllegalAction)?;
        let active_stage = batch.stage == DeathriteStage::ActiveOrder;
        let (committed, remaining) = if active_stage {
            (&mut batch.active_order, &mut batch.active_remaining)
        } else if batch.stage == DeathriteStage::NonActiveOrder {
            (&mut batch.non_active_order, &mut batch.non_active_remaining)
        } else {
            return Err(GameError::IllegalAction);
        };
        if remaining.len() < 2 {
            return Err(GameError::IllegalAction);
        }
        let index = remaining
            .iter()
            .position(|source| source.instance_id == *source_instance_id)
            .ok_or(GameError::IllegalAction)?;
        committed.push(remaining.remove(index));
        if remaining.len() == 1 {
            committed.push(remaining.remove(0));
        }
        if remaining.len() > 1 {
            return Ok(());
        }
        if active_stage && batch.non_active_remaining.len() > 1 {
            batch.stage = DeathriteStage::NonActiveOrder;
            return Ok(());
        }
        batch.resolving = batch
            .non_active_order
            .iter()
            .chain(&batch.active_order)
            .cloned()
            .collect();
        batch.stage = DeathriteStage::Resolve;
        Ok(())
    }

    fn drive_deathrites(
        &mut self,
        mut pending: PendingDeathrites,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        loop {
            let Some(batch) = pending.batches.first_mut() else {
                return self.finish_deathrites(pending, outcomes);
            };
            if batch.stage != DeathriteStage::Resolve {
                let sources = match batch.stage {
                    DeathriteStage::ActiveOrder => &batch.active_remaining,
                    DeathriteStage::NonActiveOrder => &batch.non_active_remaining,
                    DeathriteStage::Resolve => unreachable!(),
                };
                let seat = sources
                    .first()
                    .map(|source| source.controller)
                    .ok_or(GameError::IllegalAction)?;
                self.position.pending_deathrites = Some(pending);
                self.position.phase = Phase::DeathriteOrder;
                self.position.decision_seat = seat;
                return Ok(());
            }
            if batch.resolving.is_empty() {
                pending.batches.remove(0);
                continue;
            }
            let source = batch.resolving.remove(0);
            self.apply_deathrite_source(&source, &mut pending, outcomes)?;
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one Deathrite transaction preserves the printed effect and event order"
    )]
    fn apply_deathrite_source(
        &mut self,
        source: &PendingDeathriteSource,
        pending: &mut PendingDeathrites,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let CardFacts::Minion(facts) = self.rules.cards[usize::from(source.unit.card.card_id.0)]
            .facts
            .clone()
        else {
            return Err(GameError::IllegalAction);
        };
        let mut triggered_deaths = Vec::new();
        if let Some(amount) = facts.deathrite_damage_each_unit_here {
            let source_cell = source.unit.location;
            let mut targets = Vec::new();
            for seat in [Seat::North, Seat::South] {
                let avatar = &self.position.players[seat_index(seat)].avatar;
                if source.unit.region == Region::Surface && source_cell == avatar.location {
                    targets.push((avatar.card.instance_id.clone(), UnitKind::Avatar, seat));
                }
            }
            targets.extend(
                self.position
                    .units
                    .iter()
                    .filter(|unit| {
                        unit.region == source.unit.region
                            && Self::unit_occupies_cell(unit, source_cell)
                    })
                    .map(|unit| {
                        (
                            unit.card.instance_id.clone(),
                            UnitKind::Minion,
                            unit.controller,
                        )
                    }),
            );
            targets.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            let targets = targets
                .into_iter()
                .map(|(instance_id, kind, seat)| {
                    let status = if kind == UnitKind::Minion {
                        Some(self.minion_damage_status(&instance_id)?)
                    } else {
                        None
                    };
                    Ok((instance_id, kind, seat, status))
                })
                .collect::<Result<Vec<_>, GameError>>()?;
            for (target_instance_id, _, _, _) in &targets {
                outcomes.push("deathrite-damage-allocated", || {
                    json!({
                        "amount": amount,
                        "sourceInstanceId": source.instance_id,
                        "targetInstanceId": target_instance_id,
                    })
                });
            }
            for (target_instance_id, kind, seat, status) in targets {
                let result = self.apply_deathrite_damage(
                    kind,
                    seat,
                    &target_instance_id,
                    u16::from(amount),
                    UnitDamageSource {
                        current_power: source.current_power,
                        lethal: source.lethal,
                    },
                    status,
                    outcomes,
                )?;
                if result.minion_died {
                    triggered_deaths.push(target_instance_id);
                }
                if result.avatar_defeated && !pending.defeated_avatars.contains(&seat) {
                    pending.defeated_avatars.push(seat);
                }
            }
        }
        if let Some(amount) = facts.deathrite_heal {
            self.heal_avatar(
                source.controller,
                u16::from(amount),
                &source.instance_id,
                outcomes,
            )?;
        }
        if facts.deathrite_lose_life_per_nearby_site_controlled {
            let mut nearby = BTreeSet::new();
            for cell in Self::unit_occupied_cells(&source.unit) {
                nearby.insert(*cell);
                nearby.extend(cell.bordering(false));
                nearby.extend(cell.diagonals(false));
            }
            for seat in [Seat::North, Seat::South] {
                let attempted = Cell::ALL
                    .into_iter()
                    .filter(|cell| {
                        nearby.contains(cell)
                            && self.location_exists_in_region(*cell, source.unit.region)
                            && self.position.sites[cell.index()]
                                .as_ref()
                                .is_some_and(|site| site.controller == seat)
                    })
                    .count();
                let attempted = u16::try_from(attempted).map_err(|_| GameError::IllegalAction)?;
                let avatar = &mut self.position.players[seat_index(seat)].avatar;
                let old_life = avatar.life;
                avatar.life = old_life.saturating_sub(attempted);
                let lost = old_life - avatar.life;
                let reached_deaths_door = old_life > 0 && avatar.life == 0;
                if reached_deaths_door {
                    avatar.death_door_turn = Some(self.position.turn_number);
                }
                let life = avatar.life;
                if lost > 0 {
                    outcomes.push("avatar-life-lost", || {
                        json!({
                            "amount": lost,
                            "life": life,
                            "seat": seat,
                            "sourceInstanceId": source.instance_id,
                        })
                    });
                }
                if reached_deaths_door {
                    let turn_number = self.position.turn_number;
                    outcomes.push("avatar-reached-deaths-door", || {
                        json!({
                            "seat": seat,
                            "sourceInstanceId": source.instance_id,
                            "turnNumber": turn_number,
                        })
                    });
                }
            }
        }
        if facts.deathrite_draw_site {
            if self.draw_private_card(source.controller, DeckZone::Atlas) {
                outcomes.push("site-drawn", || {
                    json!({
                        "seat": source.controller,
                        "sourceInstanceId": source.instance_id,
                    })
                });
            } else if !pending.deck_losers.contains(&source.controller) {
                pending.deck_losers.push(source.controller);
            }
        }
        if !triggered_deaths.is_empty() {
            let (sources, corpses) = self.collect_minion_deaths(&triggered_deaths)?;
            pending.corpses.extend(corpses);
            if let Some(batch) = self.make_deathrite_batch(sources) {
                pending.batches.insert(0, batch);
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Deathrite damage adds one target snapshot to the shared damage transaction"
    )]
    fn apply_deathrite_damage(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
        amount: u16,
        source: UnitDamageSource,
        minion_status: Option<MinionDamageStatus>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<DamageResult, GameError> {
        if kind == UnitKind::Minion {
            let unit = self
                .position
                .units
                .iter_mut()
                .find(|unit| unit.card.instance_id == *instance_id)
                .ok_or(GameError::IllegalAction)?;
            if amount > 0 && unit.warded {
                unit.warded = false;
                outcomes.push("damage-dealt", || {
                    json!({
                        "amount": 0,
                        "attemptedAmount": amount,
                        "direct": true,
                        "instanceId": instance_id,
                        "prevented": true,
                        "seat": seat,
                    })
                });
                outcomes.push(
                    "ward-broken",
                    || json!({ "instanceId": instance_id, "seat": seat }),
                );
                return Ok(DamageResult {
                    minion_died: false,
                    avatar_defeated: false,
                });
            }
        }
        self.apply_simple_damage_with_status(
            kind,
            seat,
            instance_id,
            amount,
            source,
            minion_status,
            outcomes,
        )
    }

    fn emit_deferred_magic_resolved(
        &self,
        deferred: &DeferredMagicResolved,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        let card_id = &self.rules.cards[usize::from(deferred.card_id.0)].id;
        outcomes.push("magic-resolved", || {
            json!({
                "cardId": card_id,
                "instanceId": deferred.instance_id,
                "owner": deferred.owner,
            })
        });
    }

    fn finish_corpses(&mut self, corpses: Vec<UnitPosition>, outcomes: &mut OutcomeLog<'_>) {
        for corpse in corpses {
            let card_id = self.rules.cards[usize::from(corpse.card.card_id.0)]
                .id
                .clone();
            let instance_id = corpse.card.instance_id.clone();
            let owner = corpse.card.owner;
            let token = corpse.card.source == CardSource::Token;
            let fell_at = Location {
                cell: corpse.location,
                region: corpse.region,
            };
            let controller = corpse.controller;
            if !token {
                self.position.players[seat_index(owner)]
                    .cemetery
                    .push(corpse.card);
            }
            self.release_carried_artifacts(
                UnitKind::Minion,
                controller,
                &instance_id,
                fell_at,
                outcomes,
            );
            outcomes.push(
                "minion-died",
                || json!({ "cardId": card_id, "instanceId": instance_id, "owner": owner }),
            );
            if token {
                outcomes.push(
                    "minion-banished",
                    || json!({ "cardId": card_id, "instanceId": instance_id, "owner": owner }),
                );
            }
        }
    }

    /// Restores the interrupted phase and hands control back to whatever the deaths interrupted.
    fn resume_deathrite_continuation(
        &mut self,
        continuation: DeathriteContinuation,
        return_phase: Phase,
        return_decision_seat: Seat,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let mut restore = || {
            self.position.phase = return_phase;
            self.position.decision_seat = return_decision_seat;
        };
        match continuation {
            DeathriteContinuation::Blink(continuation) => {
                restore();
                self.finish_blink(&continuation, outcomes);
                Ok(())
            }
            DeathriteContinuation::DragProjectile(continuation) => {
                restore();
                self.continue_drag_projectile(&continuation, false, outcomes)
            }
            DeathriteContinuation::EndTurn(continuation) => self.continue_end_turn_deaths(
                continuation.seat,
                &continuation.remaining_instance_ids,
                outcomes,
            ),
            DeathriteContinuation::FirstStrike(continuation) => {
                self.continue_after_first_strike(continuation, outcomes)
            }
            DeathriteContinuation::LeapAttack(continuation) => {
                restore();
                self.finish_leap_attack(&continuation, outcomes)
            }
            DeathriteContinuation::PaidSummon(continuation) => {
                restore();
                self.finish_paid_summon(continuation, outcomes)
            }
            DeathriteContinuation::SiteGenesis(continuation) => {
                self.finish_site_genesis(continuation, outcomes)
            }
        }
    }

    fn finish_deathrites(
        &mut self,
        pending: PendingDeathrites,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ordered_resolution = self.position.phase == Phase::DeathriteOrder;
        self.finish_corpses(pending.corpses, outcomes);
        if let Some(deferred) = &pending.deferred_magic_resolved {
            self.emit_deferred_magic_resolved(deferred, outcomes);
        }
        self.position.pending_deathrites = None;
        let continuation = pending.continuation;
        let defeated = pending.defeated_avatars;
        let deck_losers = pending.deck_losers;
        let losers: Vec<_> = [Seat::North, Seat::South]
            .into_iter()
            .filter(|seat| defeated.contains(seat) || deck_losers.contains(seat))
            .collect();
        if losers.len() == 2 {
            Self::emit_interrupted_magic_resolved(continuation.as_ref(), outcomes);
            let reason = if defeated.len() == 2 && deck_losers.is_empty() {
                DrawReason::SimultaneousAvatarDefeat
            } else {
                DrawReason::SimultaneousDefeat
            };
            self.position.phase = Phase::Terminal;
            self.position.terminal = Some(TerminalResult::Draw { reason });
            outcomes.push("game-ended", || {
                json!({
                    "reason": match reason {
                        DrawReason::SimultaneousAvatarDefeat => "simultaneous_avatar_defeat",
                        DrawReason::SimultaneousDefeat => "simultaneous_defeat",
                    },
                    "result": "draw",
                })
            });
        } else if let Some(&loser) = losers.first() {
            Self::emit_interrupted_magic_resolved(continuation.as_ref(), outcomes);
            let winner = other_seat(loser);
            let reason = if defeated.contains(&loser) {
                WinReason::AvatarDefeated
            } else {
                WinReason::DeckEmpty
            };
            self.position.phase = Phase::Terminal;
            self.position.terminal = Some(TerminalResult::Win {
                loser,
                reason,
                winner,
            });
            outcomes.push("game-ended", || {
                json!({
                    "loser": loser,
                    "reason": match reason {
                        WinReason::AvatarDefeated => "avatar_defeated",
                        WinReason::DeckEmpty => "deck_empty",
                    },
                    "winner": winner,
                })
            });
        } else if let Some(continuation) = continuation {
            return self.resume_deathrite_continuation(
                continuation,
                pending.return_phase,
                pending.return_decision_seat,
                outcomes,
            );
        } else {
            self.position.phase = pending.return_phase;
            self.position.decision_seat = pending.return_decision_seat;
        }
        if self.position.terminal.is_some() && ordered_resolution {
            self.clear_ordered_terminal_continuations();
        } else if self.position.terminal.is_some()
            || self.position.pending_basic_movement.is_pending()
            || self.position.pending_ranged_step.is_pending()
        {
            self.reconcile_projectile_continuations()?;
        }
        Ok(())
    }

    fn clear_ordered_terminal_continuations(&mut self) {
        self.position.pending_basic_movement = PendingField::Absent;
        self.position.pending_cemetery_summon = None;
        self.position.pending_chain_magic = PendingField::Absent;
        self.position.pending_ranged_step = PendingField::Absent;
        self.position.pending_combat = None;
        self.position.phase = Phase::Terminal;
    }

    fn apply_draw_action(
        &mut self,
        seat: Seat,
        zone: DeckZone,
        avatar_draw: bool,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let player_index = seat_index(seat);
        let player = &self.position.players[player_index];
        let avatar_can_draw_spell = matches!(
            self.rules.cards[usize::from(player.avatar.card.card_id.0)].facts,
            CardFacts::Avatar(facts) if facts.draw_spell
        );
        if seat != self.position.active_seat
            || avatar_draw
                && (self.position.phase != Phase::Main
                    || player.avatar.tapped
                    || !player.domain_established
                    || zone == DeckZone::Spellbook && !avatar_can_draw_spell)
            || !avatar_draw && self.position.phase != Phase::Draw
        {
            return Err(GameError::IllegalAction);
        }
        let player = &mut self.position.players[player_index];
        if avatar_draw {
            player.avatar.tapped = true;
            if zone == DeckZone::Spellbook {
                player.avatar.last_interacted_turn = Some(self.position.turn_number);
            }
        }
        if !self.draw_private_card(seat, zone) {
            self.finish_deck_empty(seat, outcomes);
            self.position.state_version += 1;
            return Ok(());
        }
        self.position.phase = Phase::Main;
        self.position.state_version += 1;
        if avatar_draw && zone == DeckZone::Atlas {
            outcomes.push("site-drawn", || json!({ "seat": seat }));
        } else if avatar_draw {
            outcomes.push("spell-drawn", || json!({ "seat": seat }));
        } else {
            outcomes.push("card-drawn", || json!({ "seat": seat, "zone": zone }));
        }
        Ok(())
    }

    fn draw_private_card(&mut self, seat: Seat, zone: DeckZone) -> bool {
        let player = &mut self.position.players[seat_index(seat)];
        let card = match zone {
            DeckZone::Atlas => (!player.atlas.is_empty()).then(|| player.atlas.remove(0)),
            DeckZone::Spellbook => {
                (!player.spellbook.is_empty()).then(|| player.spellbook.remove(0))
            }
        };
        let Some(card) = card else {
            return false;
        };
        match zone {
            DeckZone::Atlas => player.hand_atlas.push(card),
            DeckZone::Spellbook => player.hand_spellbook.push(card),
        }
        true
    }

    fn finish_deck_empty(&mut self, loser: Seat, outcomes: &mut OutcomeLog<'_>) {
        let winner = other_seat(loser);
        self.position.phase = Phase::Terminal;
        self.position.terminal = Some(TerminalResult::Win {
            loser,
            reason: WinReason::DeckEmpty,
            winner,
        });
        outcomes.push(
            "game-ended",
            || json!({ "loser": loser, "reason": "deck_empty", "winner": winner }),
        );
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one closed movement transaction revalidates the complete issued path"
    )]
    fn apply_move_and_attack_action(
        &mut self,
        seat: Seat,
        from: Location,
        path: &[Location],
        to: Location,
        unit_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.phase != Phase::Main
            || seat != self.position.active_seat
            || path.is_empty()
            || path
                .iter()
                .any(|location| location.region != Region::Surface)
            || path.first() != Some(&from)
            || path.last() != Some(&to)
        {
            return Err(GameError::IllegalAction);
        }
        let (attacker_kind, current_location, ready, profile, incremental) = {
            let player = &self.position.players[seat_index(seat)];
            if player.avatar.card.instance_id == *unit_instance_id {
                (
                    UnitKind::Avatar,
                    player.avatar.location,
                    !player.avatar.tapped,
                    MovementProfile {
                        airborne: false,
                        cause: MovementCause::BasicMovement,
                        connects_top_bottom: false,
                        maximum_cost: Some(1),
                        moving_minion: false,
                        occupied_cells: None,
                        regions: RegionAbilities::default(),
                        restriction: None,
                        seat,
                    },
                    false,
                )
            } else {
                let unit = self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == *unit_instance_id)
                    .ok_or(GameError::IllegalAction)?;
                let CardFacts::Minion(facts) =
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts
                else {
                    return Err(GameError::IllegalAction);
                };
                (
                    UnitKind::Minion,
                    unit.location,
                    self.minion_can_move_and_attack(unit, seat),
                    MovementProfile {
                        airborne: facts.airborne,
                        cause: MovementCause::BasicMovement,
                        connects_top_bottom: facts.connects_top_bottom,
                        maximum_cost: if facts.immobile {
                            None
                        } else {
                            Some(1 + usize::from(facts.movement_bonus.unwrap_or(0)))
                        },
                        moving_minion: true,
                        occupied_cells: unit.occupied_cells,
                        regions: RegionAbilities::of_minion(facts),
                        restriction: facts.movement_restriction,
                        seat,
                    },
                    facts.may_ranged_strike_once_during_basic_movement,
                )
            }
        };
        if !ready
            || current_location != from.cell
            || !self
                .movement_paths(
                    Location {
                        cell: current_location,
                        region: Region::Surface,
                    },
                    profile,
                )
                .iter()
                .any(|candidate| candidate == path)
        {
            return Err(GameError::IllegalAction);
        }
        if incremental {
            return self.begin_basic_movement(
                seat,
                path,
                unit_instance_id,
                BasicMovementPurpose::MoveAndAttack,
                outcomes,
            );
        }
        outcomes.push("move-and-attack-activated", || {
            json!({
                "from": from,
                "path": path,
                "seat": seat,
                "steps": path.len() - 1,
                "to": to,
                "unitInstanceId": unit_instance_id,
            })
        });
        match attacker_kind {
            UnitKind::Avatar => {
                let avatar = &mut self.position.players[seat_index(seat)].avatar;
                avatar.location = to.cell;
                avatar.tapped = true;
            }
            UnitKind::Minion => {
                for location in path.iter().skip(1) {
                    let unit = self
                        .position
                        .units
                        .iter_mut()
                        .find(|unit| unit.card.instance_id == *unit_instance_id)
                        .ok_or(GameError::IllegalAction)?;
                    Self::move_minion_to(unit, location.cell)?;
                    self.settle_nearby_enemy_stealth(outcomes);
                }
                self.position
                    .units
                    .iter_mut()
                    .find(|unit| unit.card.instance_id == *unit_instance_id)
                    .ok_or(GameError::IllegalAction)?
                    .tapped = true;
            }
        }
        self.position.pending_combat = Some(PendingCombat {
            allocations: Vec::new(),
            attacker_instance_id: unit_instance_id.clone(),
            attacker_kind,
            attacking_seat: seat,
            cell: to.cell,
            combatants: Vec::new(),
            defenders: Vec::new(),
            original_target: None,
            region: to.region,
            target_removed: false,
        });
        self.position.phase = Phase::Attack;
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_mulligan_action(
        &mut self,
        seat: Seat,
        atlas_order: &[IdentityHash],
        spellbook_order: &[IdentityHash],
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.phase != Phase::Mulligan {
            return Err(GameError::IllegalAction);
        }
        let player = &mut self.position.players[seat_index(seat)];
        resolve_mulligan_zone(&mut player.hand_atlas, &mut player.atlas, atlas_order)?;
        resolve_mulligan_zone(
            &mut player.hand_spellbook,
            &mut player.spellbook,
            spellbook_order,
        )?;
        player.mulligan_complete = true;
        self.position.state_version += 1;
        outcomes.push("mulligan-completed", || {
            json!({
                "atlasCount": atlas_order.len(),
                "seat": seat,
                "spellbookCount": spellbook_order.len(),
            })
        });
        if seat == Seat::North {
            self.position.active_seat = Seat::South;
            self.position.decision_seat = Seat::South;
        } else {
            self.position.active_seat = self.rules.first_seat;
            self.position.decision_seat = self.rules.first_seat;
            self.position.phase = Phase::Main;
            self.position.turn_number = 1;
            outcomes.push("turn-started", || {
                json!({
                    "drawSkipped": true,
                    "seat": self.rules.first_seat,
                    "turnNumber": 1,
                })
            });
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "site placement keeps ordered Genesis effects in one authoritative transition"
    )]
    fn apply_play_site_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::PlaySite {
            card_id,
            card_instance_id,
            cell,
            create_rubble_at,
            genesis_token_choice,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        let cell = *cell;
        let create_rubble_at = *create_rubble_at;
        let genesis_token_choice = *genesis_token_choice;
        let origin_state_version = self.position.state_version;
        let player_index = seat_index(seat);
        let player = &self.position.players[player_index];
        if self.position.phase != Phase::Main
            || player.avatar.tapped
            || !self.legal_site_cells(seat).contains(&cell)
        {
            return Err(GameError::IllegalAction);
        }
        let hand_index = player
            .hand_atlas
            .iter()
            .position(|card| {
                card.instance_id == *card_instance_id
                    && self.rules.cards[usize::from(card.card_id.0)].id == card_id.as_str()
            })
            .ok_or(GameError::IllegalAction)?;
        let played_card_id = player.hand_atlas[hand_index].card_id;
        let definition = &self.rules.cards[usize::from(played_card_id.0)];
        let CardFacts::Site(facts) = &definition.facts else {
            return Err(GameError::IllegalAction);
        };
        let CardFacts::Avatar(avatar_facts) =
            &self.rules.cards[usize::from(player.avatar.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let rubble_candidates: Vec<_> = if avatar_facts.earth_site_play_creates_adjacent_rubble
            && facts.elements.contains(Element::Earth)
        {
            player
                .avatar
                .location
                .bordering(false)
                .filter(|candidate| {
                    *candidate != cell
                        && self.position.sites[candidate.index()].is_none()
                        && self.position.rubble[candidate.index()].is_none()
                })
                .collect()
        } else {
            Vec::new()
        };
        if if rubble_candidates.is_empty() {
            create_rubble_at.is_some()
        } else {
            !create_rubble_at.is_some_and(|cell| rubble_candidates.contains(&cell))
        } {
            return Err(GameError::IllegalAction);
        }
        let genesis_token_card_id = facts.genesis_pay_one_mana_to_summon_token.clone();
        match (&genesis_token_card_id, genesis_token_choice) {
            (Some(_), Some(GenesisTokenChoice::Decline | GenesisTokenChoice::PayOneMana))
            | (None, None) => {}
            _ => return Err(GameError::IllegalAction),
        }
        let genesis_gain_mana = facts.genesis_gain_mana.or_else(|| {
            (facts.genesis_gain_mana_if_only_controlled_copy
                && !self
                    .position
                    .sites
                    .iter()
                    .flatten()
                    .any(|site| site.controller == seat && site.card.card_id == played_card_id))
            .then_some(1)
        });
        let genesis_spell_draw_count = if facts.genesis_draw_spell_per_adjacent_same_card {
            cell.bordering(false)
                .filter(|neighbor| {
                    self.position.sites[neighbor.index()]
                        .as_ref()
                        .is_some_and(|site| site.card.card_id == played_card_id)
                })
                .count()
        } else {
            0
        };
        let replacing_rubble_with_water =
            self.position.rubble[cell.index()].is_some() && facts.elements.contains(Element::Water);
        let ordinary_mana = player.mana.checked_add(1).ok_or(GameError::IllegalAction)?;
        let card = self.position.players[player_index]
            .hand_atlas
            .remove(hand_index);
        let replaced_rubble = self.position.rubble[cell.index()].take();
        let player = &mut self.position.players[player_index];
        player.avatar.tapped = true;
        player.domain_established = true;
        player.mana = ordinary_mana;
        self.position.sites[cell.index()] = Some(SitePosition {
            card,
            controller: seat,
        });
        if replacing_rubble_with_water {
            self.move_underground_units_to_underwater(cell);
        }
        self.position.state_version += 1;
        if let Some(rubble_instance_id) = replaced_rubble {
            outcomes.push("rubble-replaced", || {
                json!({
                    "cell": cell,
                    "instanceId": rubble_instance_id,
                    "targetSiteInstanceId": card_instance_id,
                })
            });
        }
        outcomes.push("site-played", || {
            json!({
                "cardId": card_id,
                "cell": cell,
                "instanceId": card_instance_id,
                "seat": seat,
            })
        });
        let continuation = SiteGenesisContinuation {
            card_id: played_card_id,
            card_instance_id: card_instance_id.clone(),
            cell,
            create_rubble_at,
            defer_token: false,
            from_top_atlas: false,
            genesis_gain_mana,
            genesis_spell_draw_count,
            genesis_token_choice,
            origin_state_version,
            seat,
        };
        self.settle_lower_region_minion_deaths(outcomes)?;
        if let Some(pending) = &mut self.position.pending_deathrites {
            pending.continuation = Some(DeathriteContinuation::SiteGenesis(continuation));
            return Ok(());
        }
        if self.position.terminal.is_some() {
            return Ok(());
        }
        self.finish_site_genesis(continuation, outcomes)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one site Genesis continuation preserves settlement, state, and event order"
    )]
    fn finish_site_genesis(
        &mut self,
        continuation: SiteGenesisContinuation,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let SiteGenesisContinuation {
            card_id,
            card_instance_id,
            cell,
            create_rubble_at,
            defer_token,
            from_top_atlas: _,
            genesis_gain_mana,
            genesis_spell_draw_count,
            genesis_token_choice,
            origin_state_version,
            seat,
        } = continuation;
        let CardFacts::Site(facts) = self.rules.cards[usize::from(card_id.0)].facts.clone() else {
            return Err(GameError::IllegalAction);
        };
        self.position.phase = Phase::Main;
        self.position.decision_seat = seat;
        let paid_token = matches!(genesis_token_choice, Some(GenesisTokenChoice::PayOneMana));
        let token = if paid_token {
            Some(
                self.create_token_unit(
                    seat,
                    facts
                        .genesis_pay_one_mana_to_summon_token
                        .as_deref()
                        .ok_or(GameError::IllegalAction)?,
                    &card_instance_id,
                    cell,
                    0,
                    origin_state_version,
                )?,
            )
        } else {
            None
        };
        let player_index = seat_index(seat);
        self.position.players[player_index].mana = self.position.players[player_index]
            .mana
            .checked_add(u16::from(genesis_gain_mana.unwrap_or(0)))
            .and_then(|mana| mana.checked_sub(u16::from(paid_token)))
            .ok_or(GameError::IllegalAction)?;
        if facts.genesis_heal_nearby_avatars {
            for healed_seat in [Seat::North, Seat::South] {
                let avatar_cell = self.position.players[seat_index(healed_seat)]
                    .avatar
                    .location;
                let nearby = avatar_cell == cell
                    || cell
                        .bordering(false)
                        .chain(cell.diagonals(false))
                        .any(|nearby| nearby == avatar_cell);
                if nearby {
                    self.heal_avatar(healed_seat, 3, &card_instance_id, outcomes)?;
                }
            }
        }
        if facts.genesis_immobilize_nearby_until_next_turn {
            let cells: BTreeSet<Cell> = std::iter::once(cell)
                .chain(cell.bordering(false))
                .chain(cell.diagonals(false))
                .filter(|nearby| self.position.sites[nearby.index()].is_some())
                .collect();
            self.position.immobile_areas.push(ImmobileArea {
                cells,
                expires_at_seat: seat,
                source_instance_id: card_instance_id.clone(),
            });
        }
        if let Some(amount) = genesis_gain_mana {
            outcomes.push("mana-gained", || {
                json!({
                    "amount": amount,
                    "seat": seat,
                    "sourceInstanceId": card_instance_id,
                })
            });
        }
        if let Some(token) = token {
            let token_card_id = self.rules.cards[usize::from(token.card.card_id.0)]
                .id
                .clone();
            let token_instance_id = token.card.instance_id.clone();
            let token_owner = token.card.owner;
            self.position.units.push(token);
            outcomes.push("minion-summoned", || {
                json!({
                    "cardId": token_card_id,
                    "cell": cell,
                    "instanceId": token_instance_id,
                    "manaPaid": 1,
                    "owner": token_owner,
                    "seat": seat,
                    "sourceInstanceId": card_instance_id,
                    "token": true,
                })
            });
        }
        if facts.genesis_enemies_lose_stealth {
            let enemy = other_seat(seat);
            for unit in self
                .position
                .units
                .iter_mut()
                .filter(|unit| unit.controller == enemy && unit.stealthed)
            {
                unit.stealthed = false;
                let instance_id = unit.card.instance_id.clone();
                outcomes.push("stealth-lost", || {
                    json!({
                        "instanceId": instance_id,
                        "seat": enemy,
                        "sourceInstanceId": card_instance_id,
                    })
                });
            }
        }
        if genesis_spell_draw_count > 0 {
            let count =
                u8::try_from(genesis_spell_draw_count).map_err(|_| GameError::IllegalAction)?;
            self.apply_genesis_draws(
                seat,
                &card_instance_id,
                DeckZone::Spellbook,
                count,
                outcomes,
            );
        }
        if facts.genesis_discard_top_spells {
            for _ in 0..2 {
                let Some(card) = (!self.position.players[player_index].spellbook.is_empty())
                    .then(|| self.position.players[player_index].spellbook.remove(0))
                else {
                    break;
                };
                let discarded_card_id = self.rules.cards[usize::from(card.card_id.0)].id.clone();
                let instance_id = card.instance_id.clone();
                let owner = card.owner;
                self.position.players[player_index].cemetery.push(card);
                outcomes.push("spell-discarded", || {
                    json!({
                        "cardId": discarded_card_id,
                        "instanceId": instance_id,
                        "owner": owner,
                        "seat": seat,
                        "sourceInstanceId": card_instance_id,
                    })
                });
            }
        }
        if self.position.terminal.is_none() && defer_token {
            self.position.pending_genesis_token = PendingField::Pending(PendingGenesisToken {
                cell,
                seat,
                source_instance_id: card_instance_id.clone(),
            });
            self.position.phase = Phase::Genesis;
        }
        self.begin_hidden_spell_genesis(
            seat,
            &card_instance_id,
            facts.genesis_may_bottom_next_spell,
            facts.genesis_reorder_next_spells,
        );
        if self.position.terminal.is_none()
            && let Some(rubble_cell) = create_rubble_at
        {
            let avatar_instance_id = self.position.players[player_index]
                .avatar
                .card
                .instance_id
                .clone();
            let rubble_instance_id = identity_hash(&json!({
                "cell": rubble_cell,
                "kind": "rubble",
                "sourceInstanceId": avatar_instance_id,
                "stateVersion": origin_state_version,
            }))?;
            self.position.rubble[rubble_cell.index()] = Some(rubble_instance_id.clone());
            outcomes.push("rubble-created", || {
                json!({
                    "cell": rubble_cell,
                    "instanceId": rubble_instance_id,
                    "sourceInstanceId": avatar_instance_id,
                })
            });
        }
        Ok(())
    }

    fn create_token_unit(
        &self,
        owner: Seat,
        token_card_id: &str,
        source_instance_id: &IdentityHash,
        cell: Cell,
        ordinal: usize,
        origin_state_version: u64,
    ) -> Result<UnitPosition, GameError> {
        let (index, definition) = self
            .rules
            .cards
            .iter()
            .enumerate()
            .find(|(_, definition)| definition.id == token_card_id)
            .ok_or_else(|| invalid("token effect lacks its referenced token minion definition"))?;
        let CardFacts::Minion(facts) = &definition.facts else {
            return Err(invalid(
                "token effect lacks its referenced token minion definition",
            ));
        };
        if !facts.token {
            return Err(invalid(
                "token effect lacks its referenced token minion definition",
            ));
        }
        let card_id = CardId(
            u16::try_from(index)
                .map_err(|_| invalid("manifest contains too many card definitions"))?,
        );
        Ok(UnitPosition {
            card: CardInstance {
                card_id,
                instance_id: identity_hash(&json!({
                    "cardId": token_card_id,
                    "cell": cell,
                    "ordinal": ordinal,
                    "owner": owner,
                    "source": "token",
                    "sourceInstanceId": source_instance_id,
                    "stateVersion": origin_state_version,
                }))?,
                owner,
                source: CardSource::Token,
            },
            carried_lance_count: facts.lance_count.unwrap_or(0),
            controller: owner,
            damage: 0,
            disable_effects: Vec::new(),
            disabled_until_damaged: false,
            last_dropped_artifacts_turn: None,
            last_interacted_turn: None,
            last_picked_up_artifacts_turn: None,
            location: cell,
            occupied_cells: None,
            region: Region::Surface,
            stealthed: facts.stealth,
            summoning_sickness: true,
            tapped: false,
            temporary_charge_sources: Vec::new(),
            temporary_power_sources: Vec::new(),
            warded: matches!(facts.damage_prevention, Some(DamagePrevention::Ward)),
        })
    }

    fn apply_replace_rubble_action(
        &mut self,
        seat: Seat,
        target_cell: Cell,
        target_rubble_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let player_index = seat_index(seat);
        let player = &self.position.players[player_index];
        let CardFacts::Avatar(avatar_facts) =
            &self.rules.cards[usize::from(player.avatar.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        if self.position.phase != Phase::Main
            || !avatar_facts.replace_adjacent_rubble_with_top_atlas_site
            || player.avatar.tapped
            || !player
                .avatar
                .location
                .bordering(false)
                .any(|cell| cell == target_cell)
            || self.position.rubble[target_cell.index()].as_ref() != Some(target_rubble_instance_id)
        {
            return Err(GameError::IllegalAction);
        }
        let top = player.atlas.first().ok_or(GameError::IllegalAction)?;
        let definition = &self.rules.cards[usize::from(top.card_id.0)];
        let CardFacts::Site(facts) = &definition.facts else {
            return Err(GameError::IllegalAction);
        };
        if has_unsupported_site_genesis_after_rubble_replacement(facts) {
            return Err(GameError::UnsupportedManifestFact(
                "site Genesis after Rubble replacement".to_owned(),
            ));
        }
        let replacing_with_water = facts.elements.contains(Element::Water);
        let defer_token = facts.genesis_pay_one_mana_to_summon_token.is_some();
        let compact_card_id = top.card_id;
        let origin_state_version = self.position.state_version;
        let next_mana = player.mana.checked_add(1).ok_or(GameError::IllegalAction)?;
        let card = self.position.players[player_index].atlas.remove(0);
        let card_id = self.rules.cards[usize::from(card.card_id.0)].id.clone();
        let card_instance_id = card.instance_id.clone();
        let player = &mut self.position.players[player_index];
        player.avatar.tapped = true;
        player.domain_established = true;
        player.mana = next_mana;
        self.position.rubble[target_cell.index()] = None;
        self.position.sites[target_cell.index()] = Some(SitePosition {
            card,
            controller: seat,
        });
        if replacing_with_water {
            self.move_underground_units_to_underwater(target_cell);
        }
        self.position.state_version += 1;
        outcomes.push("rubble-replaced", || {
            json!({
                "cell": target_cell,
                "instanceId": target_rubble_instance_id,
                "targetSiteInstanceId": card_instance_id,
            })
        });
        outcomes.push("site-played", || {
            json!({
                "cardId": card_id,
                "cell": target_cell,
                "instanceId": card_instance_id,
                "seat": seat,
            })
        });
        let continuation = SiteGenesisContinuation {
            card_id: compact_card_id,
            card_instance_id,
            cell: target_cell,
            create_rubble_at: None,
            defer_token,
            from_top_atlas: true,
            genesis_gain_mana: None,
            genesis_spell_draw_count: 0,
            genesis_token_choice: None,
            origin_state_version,
            seat,
        };
        self.settle_lower_region_minion_deaths(outcomes)?;
        if let Some(pending) = &mut self.position.pending_deathrites {
            pending.continuation = Some(DeathriteContinuation::SiteGenesis(continuation));
            return Ok(());
        }
        if self.position.terminal.is_some() {
            return Ok(());
        }
        self.finish_site_genesis(continuation, outcomes)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one site-destruction transaction keeps validation, terrain, cemetery, and event order atomic"
    )]
    fn apply_site_destruction_action(
        &mut self,
        seat: Seat,
        source_site_instance_id: &IdentityHash,
        target_cell: Cell,
        target_site_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.phase != Phase::Main
            || self.position.active_seat != seat
            || self.position.decision_seat != seat
        {
            return Err(GameError::IllegalAction);
        }
        let (source_cell, source) = Cell::ALL
            .into_iter()
            .find_map(|cell| {
                self.position.sites[cell.index()]
                    .as_ref()
                    .filter(|site| site.card.instance_id == *source_site_instance_id)
                    .cloned()
                    .map(|site| (cell, site))
            })
            .ok_or(GameError::IllegalAction)?;
        let CardFacts::Site(source_facts) =
            &self.rules.cards[usize::from(source.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let target_site = self.position.sites[target_cell.index()].clone();
        let target_rubble = self.position.rubble[target_cell.index()].clone();
        let target_matches = target_site
            .as_ref()
            .is_some_and(|site| site.card.instance_id == *target_site_instance_id)
            || target_rubble.as_ref() == Some(target_site_instance_id);
        let nearby = source_cell == target_cell
            || source_cell
                .bordering(false)
                .chain(source_cell.diagonals(false))
                .any(|cell| cell == target_cell);
        if source.controller != seat
            || !source_facts.sacrifice_to_destroy_nearby_site
            || !target_matches
            || !nearby
        {
            return Err(GameError::IllegalAction);
        }
        let target_protected = if let Some(target) = &target_site {
            let CardFacts::Site(facts) =
                &self.rules.cards[usize::from(target.card.card_id.0)].facts
            else {
                return Err(GameError::IllegalAction);
            };
            facts.cannot_be_moved_destroyed_or_modified
        } else {
            false
        };
        let source_owner = source.card.owner;
        outcomes.push("site-sacrificed", || {
            json!({
                "cell": source_cell,
                "instanceId": source_site_instance_id,
                "owner": source_owner,
                "sourceInstanceId": source_site_instance_id,
            })
        });
        outcomes.push(
            if target_protected {
                "site-destruction-prevented"
            } else {
                "site-destroyed"
            },
            || {
                let mut payload = json!({
                    "cell": target_cell,
                    "instanceId": target_site_instance_id,
                    "sourceInstanceId": source_site_instance_id,
                });
                if let Some(target) = &target_site {
                    payload["owner"] = json!(target.card.owner);
                }
                payload
            },
        );

        let mut destroyed = vec![(source_cell, source)];
        if !target_protected
            && let Some(target) = target_site
            && !destroyed
                .iter()
                .any(|(_, site)| site.card.instance_id == target.card.instance_id)
        {
            destroyed.push((target_cell, target));
        }
        let (destroyed_cards, rubble) =
            self.destroy_sites_into_rubble(destroyed, source_site_instance_id)?;
        self.settle_lower_region_minion_deaths(outcomes)?;
        for card in destroyed_cards {
            self.position.players[seat_index(card.owner)]
                .cemetery
                .push(card);
        }
        let rubble_start = outcomes.len();
        for (cell, instance_id) in rubble {
            outcomes.push("rubble-created", || {
                json!({
                    "cell": cell,
                    "instanceId": instance_id,
                    "sourceInstanceId": source_site_instance_id,
                })
            });
        }
        outcomes.move_tail_before_completion(rubble_start);
        self.position.state_version += 1;
        Ok(())
    }

    /// Replaces every destroyed site with Rubble, drains the units its Water layer supported, and
    /// hands back the destroyed cards with the Rubble identities they left behind.
    #[expect(
        clippy::type_complexity,
        reason = "the caller settles the destroyed cards and the Rubble receipts on separate schedules"
    )]
    fn destroy_sites_into_rubble(
        &mut self,
        mut destroyed: Vec<(Cell, SitePosition)>,
        source_instance_id: &IdentityHash,
    ) -> Result<(Vec<CardInstance>, Vec<(Cell, IdentityHash)>), GameError> {
        destroyed.sort_unstable_by_key(|(cell, _)| *cell);
        let flooded = destroyed
            .iter()
            .filter_map(|(cell, site)| {
                let CardFacts::Site(facts) =
                    &self.rules.cards[usize::from(site.card.card_id.0)].facts
                else {
                    return None;
                };
                facts.elements.contains(Element::Water).then_some(*cell)
            })
            .collect::<BTreeSet<_>>();
        for unit in &mut self.position.units {
            if unit.region == Region::Underwater
                && Self::unit_occupied_cells(unit)
                    .iter()
                    .any(|cell| flooded.contains(cell))
            {
                unit.region = Region::Underground;
            }
        }
        let mut rubble = Vec::with_capacity(destroyed.len());
        let mut destroyed_cards = Vec::with_capacity(destroyed.len());
        for (cell, site) in destroyed {
            self.position.sites[cell.index()] = None;
            let rubble_instance_id = identity_hash(&json!({
                "cell": cell,
                "destroyedSiteInstanceId": site.card.instance_id,
                "kind": "rubble",
                "sourceInstanceId": source_instance_id,
            }))?;
            self.position.rubble[cell.index()] = Some(rubble_instance_id.clone());
            destroyed_cards.push(site.card);
            rubble.push((cell, rubble_instance_id));
        }
        Ok((destroyed_cards, rubble))
    }

    fn begin_hidden_spell_genesis(
        &mut self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        may_bottom_next_spell: bool,
        reorder_next_spells: bool,
    ) {
        if self.position.terminal.is_some() {
            return;
        }
        let spell_count = self.position.players[seat_index(seat)].spellbook.len();
        if may_bottom_next_spell && spell_count > 0 {
            self.position.pending_genesis_spell = PendingField::Pending(PendingGenesisSpell {
                seat,
                source_instance_id: source_instance_id.clone(),
            });
            self.position.phase = Phase::Genesis;
        } else if reorder_next_spells && spell_count > 0 {
            self.position.pending_genesis_spell_order =
                PendingField::Pending(PendingGenesisSpellOrder {
                    count: match spell_count {
                        1 => 1,
                        2 => 2,
                        _ => 3,
                    },
                    seat,
                    source_instance_id: source_instance_id.clone(),
                });
            self.position.phase = Phase::Genesis;
        }
    }

    fn apply_resolve_genesis_spell(
        &mut self,
        seat: Seat,
        choice: GenesisSpellChoice,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let PendingField::Pending(pending) = &self.position.pending_genesis_spell else {
            return Err(GameError::IllegalAction);
        };
        if self.position.phase != Phase::Genesis || pending.seat != seat {
            return Err(GameError::IllegalAction);
        }
        let source_instance_id = pending.source_instance_id.clone();
        let player = &mut self.position.players[seat_index(seat)];
        if player.spellbook.is_empty() {
            return Err(GameError::IllegalAction);
        }
        if choice == GenesisSpellChoice::BottomNext {
            let card = player.spellbook.remove(0);
            player.spellbook.push(card);
        }
        self.position.pending_genesis_spell = PendingField::Resolved;
        self.position.phase = Phase::Main;
        self.position.state_version += 1;
        outcomes.push(
            if choice == GenesisSpellChoice::BottomNext {
                "spell-bottomed"
            } else {
                "spell-kept"
            },
            || {
                json!({
                    "seat": seat,
                    "sourceInstanceId": source_instance_id,
                })
            },
        );
        Ok(())
    }

    fn apply_resolve_genesis_spell_order(
        &mut self,
        seat: Seat,
        order: &[u8],
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let PendingField::Pending(pending) = &self.position.pending_genesis_spell_order else {
            return Err(GameError::IllegalAction);
        };
        let count = usize::from(pending.count);
        if self.position.phase != Phase::Genesis
            || pending.seat != seat
            || order.len() != count
            || self.position.players[seat_index(seat)].spellbook.len() < count
        {
            return Err(GameError::IllegalAction);
        }
        let mut seen = [false; 3];
        for index in order.iter().copied().map(usize::from) {
            let Some(was_seen) = seen.get_mut(index) else {
                return Err(GameError::IllegalAction);
            };
            if *was_seen {
                return Err(GameError::IllegalAction);
            }
            *was_seen = true;
        }
        let source_instance_id = pending.source_instance_id.clone();
        let player = &mut self.position.players[seat_index(seat)];
        let top = player.spellbook[..count].to_vec();
        for (target, source) in order.iter().copied().map(usize::from).enumerate() {
            player.spellbook[target] = top[source].clone();
        }
        self.position.pending_genesis_spell_order = PendingField::Resolved;
        self.position.phase = Phase::Main;
        self.position.state_version += 1;
        outcomes.push("spells-reordered", || {
            json!({
                "count": count,
                "seat": seat,
                "sourceInstanceId": source_instance_id,
            })
        });
        Ok(())
    }

    fn apply_resolve_genesis_token(
        &mut self,
        seat: Seat,
        choice: GenesisTokenChoice,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let PendingField::Pending(pending) = &self.position.pending_genesis_token else {
            return Err(GameError::IllegalAction);
        };
        if self.position.phase != Phase::Genesis || pending.seat != seat {
            return Err(GameError::IllegalAction);
        }
        let site = self.position.sites[pending.cell.index()]
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        if site.card.instance_id != pending.source_instance_id {
            return Err(GameError::IllegalAction);
        }
        let CardFacts::Site(facts) = &self.rules.cards[usize::from(site.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let token_card_id = facts
            .genesis_pay_one_mana_to_summon_token
            .as_deref()
            .ok_or(GameError::IllegalAction)?;
        let paid = choice == GenesisTokenChoice::PayOneMana;
        if paid && self.position.players[seat_index(seat)].mana == 0 {
            return Err(GameError::IllegalAction);
        }
        let token = paid
            .then(|| {
                self.create_token_unit(
                    seat,
                    token_card_id,
                    &pending.source_instance_id,
                    pending.cell,
                    0,
                    self.position.state_version,
                )
            })
            .transpose()?;
        let cell = pending.cell;
        let source_instance_id = pending.source_instance_id.clone();
        self.position.pending_genesis_token = PendingField::Resolved;
        self.position.phase = Phase::Main;
        if let Some(token) = token {
            self.position.players[seat_index(seat)].mana -= 1;
            let card_id = self.rules.cards[usize::from(token.card.card_id.0)]
                .id
                .clone();
            let instance_id = token.card.instance_id.clone();
            let owner = token.card.owner;
            self.position.units.push(token);
            outcomes.push("minion-summoned", || {
                json!({
                    "cardId": card_id,
                    "cell": cell,
                    "instanceId": instance_id,
                    "manaPaid": 1,
                    "owner": owner,
                    "seat": seat,
                    "sourceInstanceId": source_instance_id,
                    "token": true,
                })
            });
        }
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_mana_activation(
        &mut self,
        seat: Seat,
        amount: u64,
        unit_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let printed = self
            .mana_activation_amount(seat, unit_instance_id)
            .ok_or(GameError::IllegalAction)?;
        if amount != u64::from(printed) {
            return Err(GameError::IllegalAction);
        }
        let next_mana = self.position.players[seat_index(seat)]
            .mana
            .checked_add(u16::from(printed))
            .ok_or(GameError::IllegalAction)?;
        self.position
            .units
            .iter_mut()
            .find(|unit| unit.card.instance_id == *unit_instance_id)
            .ok_or(GameError::IllegalAction)?
            .tapped = true;
        self.position.players[seat_index(seat)].mana = next_mana;
        self.position.state_version += 1;
        outcomes.push("mana-activated", || {
            json!({
                "amount": amount,
                "seat": seat,
                "unitInstanceId": unit_instance_id,
            })
        });
        self.record_unit_interaction(UnitKind::Minion, seat, unit_instance_id, outcomes)
    }

    /// Taps one minion to damage every unit at an adjacent location, carrying the source's own
    /// power and Lethal without opening a strike exchange or a return strike.
    fn apply_area_damage_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ActivateAreaDamage {
            source_instance_id,
            target_location,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        let mut offered = Vec::new();
        self.append_area_damage_actions(&mut offered, seat);
        if !offered
            .iter()
            .any(|issued| issued.descriptor == action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let amount = u16::from(AREA_DAMAGE_AMOUNT);
        let target_location = *target_location;
        self.position
            .units
            .iter_mut()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *source_instance_id)
            .ok_or(GameError::IllegalAction)?
            .tapped = true;
        let (current_power, lethal) =
            self.combatant_attack_and_lethal(UnitKind::Minion, seat, source_instance_id)?;
        outcomes.push("area-damage-activated", || {
            json!({
                "cell": target_location.cell,
                "region": target_location.region,
                "seat": seat,
                "sourceInstanceId": source_instance_id,
            })
        });
        self.record_unit_interaction(UnitKind::Minion, seat, source_instance_id, outcomes)?;
        self.damage_each_unit_at_location(
            target_location,
            amount,
            UnitDamageSource {
                current_power,
                lethal,
            },
            ("area-damage-allocated", source_instance_id),
            outcomes,
        )
    }

    /// Damages every unit standing at one location and settles the deaths it caused.
    ///
    /// Every occupant is announced under `allocated`, then damaged against one snapshot, so the
    /// blanket lands simultaneously instead of letting an early death shield a later target.
    fn damage_each_unit_at_location(
        &mut self,
        target_location: Location,
        amount: u16,
        source: UnitDamageSource,
        allocated: (&'static str, &IdentityHash),
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let (allocated_event, source_instance_id) = allocated;
        let targets = self
            .units_at_location(target_location)
            .into_iter()
            .map(|(instance_id, kind, target_seat)| {
                let status = match kind {
                    UnitKind::Avatar => None,
                    UnitKind::Minion => Some(self.minion_damage_status(&instance_id)?),
                };
                Ok((instance_id, kind, target_seat, status))
            })
            .collect::<Result<Vec<_>, GameError>>()?;
        for (target_instance_id, _, _, _) in &targets {
            outcomes.push(allocated_event, || {
                json!({
                    "amount": amount,
                    "sourceInstanceId": source_instance_id,
                    "targetInstanceId": target_instance_id,
                })
            });
        }
        let mut dead_minions = Vec::new();
        let mut defeated_avatars = Vec::new();
        for (target_instance_id, kind, target_seat, status) in targets {
            let result = self.apply_simple_damage_with_status(
                kind,
                target_seat,
                &target_instance_id,
                amount,
                source,
                status,
                outcomes,
            )?;
            if result.minion_died {
                dead_minions.push(target_instance_id);
            }
            if result.avatar_defeated && !defeated_avatars.contains(&target_seat) {
                defeated_avatars.push(target_seat);
            }
        }
        self.position.state_version += 1;
        if !dead_minions.is_empty() || !defeated_avatars.is_empty() {
            self.begin_minion_deaths(
                &dead_minions,
                &defeated_avatars,
                Phase::Main,
                self.position.active_seat,
                outcomes,
            )?;
        }
        Ok(())
    }

    /// Taps a carried Artifact's bearer and one other ally beside it so the Artifact damages one
    /// measured target. The Artifact is the source, so damage prevention keyed to unit power does
    /// not apply and no unit power or Lethal is lent to the shot.
    fn apply_artifact_damage_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ActivateArtifactDamage {
            artifact_instance_id,
            helper,
            target,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        if !self
            .artifact_damage_descriptors(seat)?
            .contains(&action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let bearer = self
            .position
            .artifacts
            .iter()
            .find(|artifact| artifact.card.instance_id == *artifact_instance_id)
            .and_then(ArtifactPosition::bearer)
            .ok_or(GameError::IllegalAction)?
            .clone();
        let amount = u16::from(ARTIFACT_DAMAGE_AMOUNT);
        outcomes.push("artifact-damage-activated", || {
            json!({
                "bearerInstanceId": bearer.instance_id(),
                "helperInstanceId": helper.instance_id(),
                "seat": seat,
                "sourceInstanceId": artifact_instance_id,
                "targetInstanceId": target.instance_id(),
            })
        });
        // Both tap costs are paid before the shot lands, so an overlapping helper is already spent
        // when the damage that kills it is dealt.
        self.tap_unit_target(&bearer)?;
        self.tap_unit_target(helper)?;
        outcomes.push("artifact-damage-allocated", || {
            json!({
                "amount": amount,
                "sourceInstanceId": artifact_instance_id,
                "targetInstanceId": target.instance_id(),
            })
        });
        self.damage_unit_and_settle_deaths(
            &(
                target.instance_id().clone(),
                unit_target_kind(target),
                target.seat(),
            ),
            amount,
            UnitDamageSource {
                current_power: 0,
                lethal: false,
            },
            outcomes,
        )?;
        self.position.state_version += 1;
        Ok(())
    }

    /// Taps a carried Artifact's bearer and one other ally beside it and discards one card in hand
    /// so the Artifact damages every unit at one measured location for the discarded card's mana
    /// cost. The Artifact is the source, so the blanket lends no unit power or Lethal, and the
    /// location is targeted rather than its occupants, so a Stealthed occupant is not skipped.
    fn apply_artifact_discard_area_damage_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ActivateArtifactDiscardAreaDamage {
            artifact_instance_id,
            discard_card_instance_id,
            discard_zone,
            helper,
            target_location,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        if !self
            .artifact_discard_area_damage_descriptors(seat)?
            .contains(&action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let bearer = self
            .position
            .artifacts
            .iter()
            .find(|artifact| artifact.card.instance_id == *artifact_instance_id)
            .and_then(ArtifactPosition::bearer)
            .ok_or(GameError::IllegalAction)?
            .clone();
        let target_location = *target_location;

        // The discard is a cost, so it is paid, and its mana cost read, before the shot lands.
        let player = &mut self.position.players[seat_index(seat)];
        let hand = match discard_zone {
            DeckZone::Atlas => &mut player.hand_atlas,
            DeckZone::Spellbook => &mut player.hand_spellbook,
        };
        let hand_index = hand
            .iter()
            .position(|card| card.instance_id == *discard_card_instance_id)
            .ok_or(GameError::IllegalAction)?;
        let discarded = hand.remove(hand_index);
        let amount = payload_damage_amount(&self.rules.cards[usize::from(discarded.card_id.0)])?;
        outcomes.push("card-discarded", || {
            json!({
                "cardId": self.rules.cards[usize::from(discarded.card_id.0)].id,
                "instanceId": discarded.instance_id,
                "owner": discarded.owner,
                "seat": seat,
                "sourceInstanceId": artifact_instance_id,
                "zone": discard_zone.as_str(),
            })
        });
        self.position.players[seat_index(seat)]
            .cemetery
            .push(discarded);
        self.tap_unit_target(&bearer)?;
        self.tap_unit_target(helper)?;
        outcomes.push("artifact-discard-area-damage-activated", || {
            json!({
                "bearerInstanceId": bearer.instance_id(),
                "discardCardInstanceId": discard_card_instance_id,
                "helperInstanceId": helper.instance_id(),
                "seat": seat,
                "sourceInstanceId": artifact_instance_id,
                "targetCell": target_location.cell,
                "targetRegion": target_location.region,
            })
        });
        self.damage_each_unit_at_location(
            target_location,
            amount,
            UnitDamageSource {
                current_power: 0,
                lethal: false,
            },
            (
                "artifact-discard-area-damage-allocated",
                artifact_instance_id,
            ),
            outcomes,
        )
    }

    /// Every other unit whose footprint overlaps a Rolling Boulder roll path, excluding the pusher.
    fn rolling_boulder_damage_targets(
        &self,
        path: &[Location],
        destination: Location,
        pusher: &UnitTarget,
        amount_per_cell: u16,
    ) -> Result<Vec<(UnitTarget, u16)>, GameError> {
        let path_cells: BTreeSet<Cell> = if path.len() > 1 {
            path.iter().map(|location| location.cell).collect()
        } else {
            BTreeSet::new()
        };
        let mut targets = Vec::new();
        for target_seat in [Seat::North, Seat::South] {
            for target in self.seat_unit_targets(target_seat) {
                if target.instance_id() == pusher.instance_id() {
                    continue;
                }
                if self.unit_target_region(&target)? != destination.region {
                    continue;
                }
                let covered = self
                    .unit_target_occupied_cells(&target)?
                    .iter()
                    .filter(|cell| path_cells.contains(cell))
                    .count();
                if covered > 0 {
                    targets.push((
                        target,
                        amount_per_cell.saturating_mul(u16::try_from(covered).unwrap_or(u16::MAX)),
                    ));
                }
            }
        }
        targets
            .sort_unstable_by(|(left, _), (right, _)| left.instance_id().cmp(right.instance_id()));
        Ok(targets)
    }

    /// Taps one ready co-located unit to push a Rolling Boulder maximally in one cardinal direction,
    /// move the Artifact along the issued path, and deal its printed damage to every other unit
    /// whose footprint overlaps that path. The Artifact is the source, so no bearer power or Lethal
    /// rides along, and the pusher never damages itself.
    #[expect(
        clippy::too_many_lines,
        reason = "one roll keeps tap cost, movement, allocations, and simultaneous damage explicit"
    )]
    fn apply_artifact_roll_damage_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ActivateArtifactRollDamage {
            artifact_instance_id,
            direction,
            path,
            pusher,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        if !self
            .artifact_roll_damage_descriptors(seat)?
            .contains(&action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let destination = *path.last().ok_or(GameError::IllegalAction)?;
        let amount_per_cell = u16::from(ARTIFACT_ROLL_DAMAGE_AMOUNT);
        let damaged_targets =
            self.rolling_boulder_damage_targets(path, destination, pusher, amount_per_cell)?;
        outcomes.push("artifact-roll-damage-activated", || {
            json!({
                "direction": direction,
                "fromCell": path[0].cell,
                "fromRegion": path[0].region,
                "path": path,
                "pusherInstanceId": pusher.instance_id(),
                "pusherKind": pusher.kind(),
                "pusherSeat": pusher.seat(),
                "seat": seat,
                "sourceInstanceId": artifact_instance_id,
                "toCell": destination.cell,
                "toRegion": destination.region,
            })
        });
        self.tap_unit_target(pusher)?;
        let artifact_index = self
            .position
            .artifacts
            .iter()
            .position(|artifact| artifact.card.instance_id == *artifact_instance_id)
            .ok_or(GameError::IllegalAction)?;
        self.position.artifacts[artifact_index].placement = ArtifactPlacement::Loose {
            location: destination.cell,
            region: destination.region,
        };
        if damaged_targets.is_empty() {
            self.position.state_version += 1;
            return Ok(());
        }
        for (target, amount) in &damaged_targets {
            outcomes.push("artifact-roll-damage-allocated", || {
                json!({
                    "amount": amount,
                    "sourceInstanceId": artifact_instance_id,
                    "targetInstanceId": target.instance_id(),
                })
            });
        }
        let source = UnitDamageSource {
            current_power: 0,
            lethal: false,
        };
        let snapshots: Vec<_> = damaged_targets
            .iter()
            .map(|(target, amount)| {
                let kind = unit_target_kind(target);
                let status = if kind == UnitKind::Minion {
                    Some(self.minion_damage_status(target.instance_id())?)
                } else {
                    None
                };
                Ok((target.clone(), *amount, kind, status))
            })
            .collect::<Result<Vec<_>, GameError>>()?;
        let mut dead_minions = Vec::new();
        let mut defeated_avatars = Vec::new();
        for (target, amount, kind, status) in snapshots {
            let result = self.apply_simple_damage_with_status(
                kind,
                target.seat(),
                target.instance_id(),
                amount,
                source,
                status,
                outcomes,
            )?;
            if result.minion_died {
                dead_minions.push(target.instance_id().clone());
            }
            if result.avatar_defeated && !defeated_avatars.contains(&target.seat()) {
                defeated_avatars.push(target.seat());
            }
        }
        self.position.state_version += 1;
        if !dead_minions.is_empty() || !defeated_avatars.is_empty() {
            self.begin_minion_deaths(
                &dead_minions,
                &defeated_avatars,
                Phase::Main,
                self.position.active_seat,
                outcomes,
            )?;
        }
        Ok(())
    }

    /// Discards one Spellbook card so a minion damages a hidden random other unit at its location.
    fn apply_discard_random_damage_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
        random_draws: Option<&mut Vec<EngineRandomDraw>>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ActivateDiscardRandomDamage {
            discard_card_instance_id,
            source_instance_id,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        if self.position.phase != Phase::Main
            || self.position.active_seat != seat
            || self.position.decision_seat != seat
        {
            return Err(GameError::IllegalAction);
        }
        let source = self
            .position
            .units
            .iter()
            .find(|unit| unit.controller == seat && unit.card.instance_id == *source_instance_id)
            .ok_or(GameError::IllegalAction)?;
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(source.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let amount = u16::from(
            facts
                .discard_spell_to_damage_random_other_unit_here
                .ok_or(GameError::IllegalAction)?,
        );
        if self.minion_is_disabled(source) {
            return Err(GameError::IllegalAction);
        }
        let source_location = Location {
            cell: source.location,
            region: source.region,
        };
        let player = &mut self.position.players[seat_index(seat)];
        let hand_index = player
            .hand_spellbook
            .iter()
            .position(|card| card.instance_id == *discard_card_instance_id)
            .ok_or(GameError::IllegalAction)?;
        let discarded = player.hand_spellbook.remove(hand_index);
        outcomes.push("card-discarded", || {
            json!({
                "cardId": self.rules.cards[usize::from(discarded.card_id.0)].id,
                "instanceId": discarded.instance_id,
                "owner": discarded.owner,
                "seat": seat,
                "sourceInstanceId": source_instance_id,
                "zone": "spellbook",
            })
        });
        self.position.players[seat_index(seat)]
            .cemetery
            .push(discarded);

        let (current_power, lethal) =
            self.combatant_attack_and_lethal(UnitKind::Minion, seat, source_instance_id)?;
        let selected = self.draw_random_other_unit_here(
            source_location,
            source_instance_id,
            "discard_spell_random_other_unit_here",
            random_draws,
        )?;
        outcomes.push("discard-random-damage-activated", || {
            let mut payload = json!({
                "amount": amount,
                "discardCardInstanceId": discard_card_instance_id,
                "seat": seat,
                "sourceInstanceId": source_instance_id,
                "sourceLocation": source_location,
            });
            if let Some((target_instance_id, target_kind, target_seat)) = &selected {
                payload["targetInstanceId"] = json!(target_instance_id);
                payload["targetKind"] = json!(target_kind.as_str());
                payload["targetSeat"] = json!(target_seat);
            }
            payload
        });
        self.record_unit_interaction(UnitKind::Minion, seat, source_instance_id, outcomes)?;

        if let Some(target) = selected {
            outcomes.push("discard-random-damage-allocated", || {
                json!({
                    "amount": amount,
                    "sourceInstanceId": source_instance_id,
                    "targetInstanceId": target.0,
                })
            });
            self.damage_unit_and_settle_deaths(
                &target,
                amount,
                UnitDamageSource {
                    current_power,
                    lethal,
                },
                outcomes,
            )?;
        }
        self.position.state_version += 1;
        Ok(())
    }

    /// Draws one hidden random unit sharing a location with an activated source, excluding it.
    fn draw_random_other_unit_here(
        &mut self,
        location: Location,
        source_instance_id: &IdentityHash,
        purpose: &str,
        random_draws: Option<&mut Vec<EngineRandomDraw>>,
    ) -> Result<Option<(IdentityHash, UnitKind, Seat)>, GameError> {
        let mut candidates = self.units_at_location(location);
        candidates.retain(|(instance_id, _, _)| instance_id != source_instance_id);
        if candidates.is_empty() {
            return Ok(None);
        }
        let index = draw_index(
            &mut self.position.prng,
            candidates.len(),
            purpose,
            "unit_index_candidate",
            random_draws,
        )?;
        Ok(Some(candidates[index].clone()))
    }

    /// Applies one activated ability's damage to a resolved unit and settles the deaths it caused.
    fn damage_unit_and_settle_deaths(
        &mut self,
        target: &(IdentityHash, UnitKind, Seat),
        amount: u16,
        source: UnitDamageSource,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let (target_instance_id, target_kind, target_seat) = target;
        let result = self.apply_simple_damage(
            *target_kind,
            *target_seat,
            target_instance_id,
            amount,
            source,
            outcomes,
        )?;
        if result.minion_died || result.avatar_defeated {
            self.begin_minion_deaths(
                if result.minion_died {
                    std::slice::from_ref(target_instance_id)
                } else {
                    &[]
                },
                if result.avatar_defeated {
                    std::slice::from_ref(target_seat)
                } else {
                    &[]
                },
                Phase::Main,
                self.position.active_seat,
                outcomes,
            )?;
        }
        Ok(())
    }

    fn apply_sparkmage_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
        random_draws: Option<&mut Vec<EngineRandomDraw>>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::ActivateSparkmage {
            source_instance_id,
            target_location,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        let player = &self.position.players[seat_index(seat)];
        let CardFacts::Avatar(facts) =
            &self.rules.cards[usize::from(player.avatar.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let target_is_nearby = std::iter::once(player.avatar.location)
            .chain(player.avatar.location.bordering(false))
            .chain(player.avatar.location.diagonals(false))
            .any(|cell| cell == target_location.cell);
        if self.position.phase != Phase::Main
            || self.position.active_seat != seat
            || self.position.decision_seat != seat
            || player.avatar.card.instance_id != *source_instance_id
            || player.avatar.tapped
            || !facts
                .tap_damage_random_other_unit_at_nearby_location_per_air_threshold_cast_this_turn
            || target_location.region != Region::Surface
            || !target_is_nearby
            || !self.surface_location_exists(target_location.cell)
        {
            return Err(GameError::IllegalAction);
        }
        let amount = player.air_thresholds_cast_this_turn.unwrap_or(0);
        let (current_power, lethal) =
            self.combatant_attack_and_lethal(UnitKind::Avatar, seat, source_instance_id)?;
        self.position.players[seat_index(seat)].avatar.tapped = true;

        let selected = self.draw_random_other_unit_here(
            *target_location,
            source_instance_id,
            "sparkmage_random_other_unit_at_nearby_location",
            random_draws,
        )?;
        outcomes.push("sparkmage-activated", || {
            let mut payload = json!({
                "amount": amount,
                "seat": seat,
                "sourceInstanceId": source_instance_id,
                "targetLocation": target_location,
            });
            if let Some((target_instance_id, target_kind, target_seat)) = &selected {
                payload["targetInstanceId"] = json!(target_instance_id);
                payload["targetKind"] = json!(target_kind.as_str());
                payload["targetSeat"] = json!(target_seat);
            }
            payload
        });
        self.record_unit_interaction(UnitKind::Avatar, seat, source_instance_id, outcomes)?;

        if let Some(target) = selected
            && amount > 0
        {
            self.damage_unit_and_settle_deaths(
                &target,
                amount,
                UnitDamageSource {
                    current_power,
                    lethal,
                },
                outcomes,
            )?;
        }
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_begin_chain_magic(&mut self, action: &IssuedAction) -> Result<(), GameError> {
        let ActionDescriptor::BeginChainMagic {
            card_id,
            card_instance_id,
            caster_instance_id,
            target,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        if self.position.phase != Phase::Main
            || !self
                .legal_actions()?
                .iter()
                .any(|candidate| candidate.descriptor == action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let card = self.position.players[seat_index(action.seat)]
            .hand_spellbook
            .iter()
            .find(|card| {
                card.instance_id == *card_instance_id
                    && self.rules.cards[usize::from(card.card_id.0)].id == card_id.as_str()
            })
            .ok_or(GameError::IllegalAction)?;
        self.position.pending_chain_magic = PendingField::Pending(PendingChainMagic {
            card_id: card.card_id,
            card_instance_id: card_instance_id.clone(),
            caster_instance_id: caster_instance_id.clone(),
            seat: action.seat,
            targets: vec![target.clone()],
        });
        self.position.phase = Phase::ChainMagic;
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_extend_chain_magic(&mut self, action: &IssuedAction) -> Result<(), GameError> {
        let ActionDescriptor::ExtendChainMagic { target } = &action.descriptor else {
            return Err(GameError::IllegalAction);
        };
        if self.position.phase != Phase::ChainMagic
            || !self
                .legal_actions()?
                .iter()
                .any(|candidate| candidate.descriptor == action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        self.position
            .pending_chain_magic
            .as_pending_mut()
            .ok_or(GameError::IllegalAction)?
            .targets
            .push(target.clone());
        self.position.state_version += 1;
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one staged Chain Magic transaction keeps payment and simultaneous damage atomic"
    )]
    fn apply_resolve_chain_magic(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if !matches!(action.descriptor, ActionDescriptor::ResolveChainMagic)
            || self.position.phase != Phase::ChainMagic
            || !self
                .legal_actions()?
                .iter()
                .any(|candidate| candidate.descriptor == action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let pending = self
            .position
            .pending_chain_magic
            .as_pending()
            .cloned()
            .ok_or(GameError::IllegalAction)?;
        let player_index = seat_index(action.seat);
        let hand_index = self.position.players[player_index]
            .hand_spellbook
            .iter()
            .position(|card| {
                card.card_id == pending.card_id && card.instance_id == pending.card_instance_id
            })
            .ok_or(GameError::IllegalAction)?;
        let definition = &self.rules.cards[usize::from(pending.card_id.0)];
        let CardFacts::Magic(facts) = &definition.facts else {
            return Err(GameError::IllegalAction);
        };
        if facts.effect != MagicEffect::DamageChainNearbyUnits {
            return Err(GameError::IllegalAction);
        }
        let card_id = definition.id.clone();
        let thresholds = facts.thresholds;
        let extra_targets = u64::try_from(pending.targets.len().saturating_sub(1))
            .map_err(|_| GameError::IllegalAction)?;
        let mana_paid =
            facts.mana_cost + CHAIN_MAGIC_EXTRA_TARGET_MANA.saturating_mul(extra_targets);
        let mana_paid = u16::try_from(mana_paid).map_err(|_| GameError::IllegalAction)?;
        let caster_kind = self
            .spellcaster_kind(action.seat, &pending.caster_instance_id)
            .ok_or(GameError::IllegalAction)?;
        let next_air_thresholds_cast_this_turn = self.position.players[player_index]
            .air_thresholds_cast_this_turn
            .map(|cast_air| {
                let added = u16::try_from(thresholds.get(Element::Air))
                    .map_err(|_| GameError::IllegalAction)?;
                cast_air.checked_add(added).ok_or(GameError::IllegalAction)
            })
            .transpose()?;
        let card = self.position.players[player_index]
            .hand_spellbook
            .remove(hand_index);
        let owner = card.owner;
        let compact_card_id = card.card_id;
        let player = &mut self.position.players[player_index];
        player.mana = player
            .mana
            .checked_sub(mana_paid)
            .ok_or(GameError::IllegalAction)?;
        player.air_thresholds_cast_this_turn = next_air_thresholds_cast_this_turn;
        self.position.players[seat_index(owner)].cemetery.push(card);
        self.position.pending_chain_magic = PendingField::Resolved;
        self.position.decision_seat = self.position.active_seat;
        self.position.phase = Phase::Main;
        outcomes.push("magic-cast", || {
            json!({
                "cardId": card_id,
                "casterInstanceId": pending.caster_instance_id,
                "instanceId": pending.card_instance_id,
                "manaPaid": mana_paid,
                "seat": action.seat,
                "targetInstanceIds": pending.targets.iter().map(UnitTarget::instance_id).collect::<Vec<_>>(),
            })
        });
        self.record_unit_interaction(
            caster_kind,
            action.seat,
            &pending.caster_instance_id,
            outcomes,
        )?;
        let targets = pending
            .targets
            .iter()
            .map(|target| {
                let kind = match target {
                    UnitTarget::Avatar { .. } => UnitKind::Avatar,
                    UnitTarget::Minion { .. } => UnitKind::Minion,
                };
                let status = if kind == UnitKind::Minion {
                    Some(self.minion_damage_status(target.instance_id())?)
                } else {
                    None
                };
                Ok((target.clone(), kind, status))
            })
            .collect::<Result<Vec<_>, GameError>>()?;
        for (target, _, _) in &targets {
            outcomes.push("magic-damage-allocated", || {
                json!({
                    "amount": CHAIN_MAGIC_DAMAGE,
                    "sourceInstanceId": pending.card_instance_id,
                    "targetInstanceId": target.instance_id(),
                })
            });
        }
        let mut dead_minions = Vec::new();
        let mut defeated_avatars = Vec::new();
        for (target, kind, status) in targets {
            let result = self.apply_simple_damage_with_status(
                kind,
                target.seat(),
                target.instance_id(),
                CHAIN_MAGIC_DAMAGE,
                UnitDamageSource {
                    current_power: 0,
                    lethal: false,
                },
                status,
                outcomes,
            )?;
            if result.minion_died {
                dead_minions.push(target.instance_id().clone());
            }
            if result.avatar_defeated && !defeated_avatars.contains(&target.seat()) {
                defeated_avatars.push(target.seat());
            }
        }
        if !dead_minions.is_empty() || !defeated_avatars.is_empty() {
            self.begin_minion_deaths(
                &dead_minions,
                &defeated_avatars,
                Phase::Main,
                self.position.active_seat,
                outcomes,
            )?;
        }
        if let Some(deathrites) = &mut self.position.pending_deathrites {
            deathrites.deferred_magic_resolved = Some(DeferredMagicResolved {
                card_id: compact_card_id,
                instance_id: pending.card_instance_id.clone(),
                owner,
            });
        } else {
            let resolved_start = outcomes.len();
            outcomes.push("magic-resolved", || {
                json!({
                    "cardId": card_id,
                    "instanceId": pending.card_instance_id,
                    "owner": owner,
                })
            });
            outcomes.move_tail_before_completion(resolved_start);
        }
        self.position.state_version += 1;
        Ok(())
    }

    /// Publicly draws one dead minion from either cemetery and opens the free placement it owes.
    ///
    /// Returns whether the caster still owes that placement; an empty cemetery pool or a minion
    /// with no legal location resolves the Magic immediately instead.
    fn begin_cemetery_summon(
        &mut self,
        request: CemeterySummonRequest<'_>,
        outcomes: &mut OutcomeLog<'_>,
        random_draws: Option<&mut Vec<EngineRandomDraw>>,
    ) -> Result<bool, GameError> {
        let CemeterySummonRequest {
            card_instance_id,
            caster_instance_id,
            seat,
            source_magic_card_id,
            source_magic_owner,
        } = request;
        let mut candidates: Vec<(Seat, IdentityHash, CardId)> = Vec::new();
        for owner in [Seat::North, Seat::South] {
            for card in &self.position.players[seat_index(owner)].cemetery {
                if matches!(
                    self.rules.cards[usize::from(card.card_id.0)].facts,
                    CardFacts::Minion(_)
                ) {
                    candidates.push((owner, card.instance_id.clone(), card.card_id));
                }
            }
        }
        candidates.sort_unstable_by(|left, right| left.1.cmp(&right.1));
        if candidates.is_empty() {
            return Ok(false);
        }
        let index = draw_index(
            &mut self.position.prng,
            candidates.len(),
            "magic_random_dead_minion",
            "dead_minion_instance_candidate",
            random_draws,
        )?;
        let (card_owner, dead_instance_id, dead_card_id) = candidates.swap_remove(index);
        let dead_definition = &self.rules.cards[usize::from(dead_card_id.0)];
        let CardFacts::Minion(facts) = &dead_definition.facts else {
            return Err(invalid("cemetery minion candidate lacks minion facts"));
        };
        let dead_card_name = dead_definition.id.clone();
        let placements = self.free_summon_destinations(facts);
        outcomes.push("dead-minion-selected", || {
            json!({
                "cardId": dead_card_name,
                "instanceId": dead_instance_id,
                "owner": card_owner,
                "seat": seat,
                "sourceInstanceId": card_instance_id,
            })
        });
        if placements.is_empty() {
            outcomes.push("minion-summon-failed", || {
                json!({
                    "instanceId": dead_instance_id,
                    "owner": card_owner,
                    "reason": "no-legal-location",
                    "seat": seat,
                    "sourceInstanceId": card_instance_id,
                })
            });
            return Ok(false);
        }
        self.position.pending_cemetery_summon = Some(PendingCemeterySummon {
            card_instance_id: dead_instance_id,
            card_owner,
            caster_instance_id: caster_instance_id.clone(),
            seat,
            source_magic_card_id,
            source_magic_instance_id: card_instance_id.clone(),
            source_magic_owner,
        });
        self.position.phase = Phase::CemeterySummon;
        Ok(true)
    }

    fn apply_cast_artifact_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::CastArtifact {
            bearer,
            card_id,
            card_instance_id,
            caster_instance_id,
            cell,
            mana_cost,
            ..
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        let caster_kind = self
            .spellcaster_kind(seat, caster_instance_id)
            .ok_or(GameError::IllegalAction)?;
        if self.position.phase != Phase::Main
            || !self
                .artifact_cast_descriptors(seat)
                .contains(&action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let placement = match (bearer, cell) {
            (Some(bearer), _) => ArtifactPlacement::Carried {
                bearer: bearer.clone(),
            },
            (None, Some(cell)) => ArtifactPlacement::Loose {
                location: *cell,
                region: Region::Surface,
            },
            (None, None) => return Err(GameError::IllegalAction),
        };
        let player_index = seat_index(seat);
        let hand_index = self.position.players[player_index]
            .hand_spellbook
            .iter()
            .position(|card| card.instance_id == *card_instance_id)
            .ok_or(GameError::IllegalAction)?;
        let compact_card_id =
            self.position.players[player_index].hand_spellbook[hand_index].card_id;
        let CardFacts::Artifact(facts) = &self.rules.cards[usize::from(compact_card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let air = u16::try_from(facts.thresholds.get(Element::Air))
            .map_err(|_| GameError::IllegalAction)?;
        let paid_mana = u16::try_from(*mana_cost).map_err(|_| GameError::IllegalAction)?;
        let player = &mut self.position.players[player_index];
        let card = player.hand_spellbook.remove(hand_index);
        player.mana = player
            .mana
            .checked_sub(paid_mana)
            .ok_or(GameError::IllegalAction)?;
        player.air_thresholds_cast_this_turn = player
            .air_thresholds_cast_this_turn
            .map(|cast_air| cast_air.checked_add(air).ok_or(GameError::IllegalAction))
            .transpose()?;
        self.record_unit_interaction(caster_kind, seat, caster_instance_id, outcomes)?;
        let instance_id = card.instance_id.clone();
        let owner = card.owner;
        self.position
            .artifacts
            .push(ArtifactPosition { card, placement });
        outcomes.push("artifact-conjured", || {
            let mut payload = json!({
                "cardId": card_id,
                "casterInstanceId": caster_instance_id,
                "instanceId": instance_id,
                "manaPaid": paid_mana,
                "owner": owner,
                "seat": seat,
            });
            match (bearer, cell) {
                (Some(bearer), _) => {
                    payload["bearerInstanceId"] = json!(bearer.instance_id());
                    payload["bearerKind"] = json!(bearer.kind());
                    payload["bearerSeat"] = json!(bearer.seat());
                }
                (None, Some(cell)) => {
                    payload["cell"] = json!(cell);
                    payload["region"] = json!(Region::Surface);
                }
                (None, None) => {}
            }
            payload
        });
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_pick_up_artifacts_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::PickUpArtifacts {
            artifact_instance_ids,
            unit,
            ..
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        if self.position.phase != Phase::Main
            || !self
                .pick_up_artifact_descriptors(seat)?
                .contains(&action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let selected: BTreeSet<_> = artifact_instance_ids.iter().cloned().collect();
        for artifact in &mut self.position.artifacts {
            if selected.contains(&artifact.card.instance_id) {
                artifact.placement = ArtifactPlacement::Carried {
                    bearer: unit.clone(),
                };
            }
        }
        let turn = self.position.turn_number;
        self.record_artifact_turn(unit, |tracked| *tracked = Some(turn), true)?;
        outcomes.push("artifacts-picked-up", || {
            json!({
                "artifactInstanceIds": artifact_instance_ids,
                "seat": seat,
                "unitInstanceId": unit.instance_id(),
                "unitKind": unit.kind(),
            })
        });
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_drop_artifacts_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::DropArtifacts {
            artifact_instance_ids,
            unit,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        if self.position.phase != Phase::Main
            || !self
                .drop_artifact_descriptors(seat)?
                .contains(&action.descriptor)
        {
            return Err(GameError::IllegalAction);
        }
        let fell_at = self.unit_target_location(unit)?;
        let selected: BTreeSet<_> = artifact_instance_ids.iter().cloned().collect();
        for artifact in &mut self.position.artifacts {
            if selected.contains(&artifact.card.instance_id) {
                artifact.placement = ArtifactPlacement::Loose {
                    location: fell_at.cell,
                    region: fell_at.region,
                };
            }
        }
        let turn = self.position.turn_number;
        self.record_artifact_turn(unit, |tracked| *tracked = Some(turn), false)?;
        outcomes.push("artifacts-dropped", || {
            json!({
                "artifactInstanceIds": artifact_instance_ids,
                "seat": seat,
                "unitInstanceId": unit.instance_id(),
                "unitKind": unit.kind(),
            })
        });
        // Losing a power Artifact can leave the bearer lethally wounded; shared settlement kills it.
        self.position.state_version += 1;
        Ok(())
    }

    /// Stamps the unit's most recent Artifact Pick Up or Drop turn.
    fn record_artifact_turn(
        &mut self,
        unit: &UnitTarget,
        stamp: impl FnOnce(&mut Option<u64>),
        picked_up: bool,
    ) -> Result<(), GameError> {
        let tracked = match unit {
            UnitTarget::Avatar { instance_id, seat } => {
                let avatar = &mut self.position.players[seat_index(*seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                if picked_up {
                    &mut avatar.last_picked_up_artifacts_turn
                } else {
                    &mut avatar.last_dropped_artifacts_turn
                }
            }
            UnitTarget::Minion { instance_id, seat } => {
                let minion = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                    .ok_or(GameError::IllegalAction)?;
                if picked_up {
                    &mut minion.last_picked_up_artifacts_turn
                } else {
                    &mut minion.last_dropped_artifacts_turn
                }
            }
        };
        stamp(tracked);
        Ok(())
    }

    /// Releases every Artifact the unit carried onto the location where it stood.
    fn release_carried_artifacts(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
        fell_at: Location,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        let mut dropped = Vec::new();
        for artifact in &mut self.position.artifacts {
            if artifact.carried_by(kind, seat, instance_id) {
                artifact.placement = ArtifactPlacement::Loose {
                    location: fell_at.cell,
                    region: fell_at.region,
                };
                dropped.push(artifact.card.clone());
            }
        }
        for card in dropped {
            let card_id = self.rules.cards[usize::from(card.card_id.0)].id.clone();
            outcomes.push("artifact-dropped", || {
                json!({
                    "bearerInstanceId": instance_id,
                    "cardId": card_id,
                    "cell": fell_at.cell,
                    "instanceId": card.instance_id,
                    "owner": card.owner,
                    "region": fell_at.region,
                })
            });
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the closed Magic transaction keeps validation, payment, and effects atomic"
    )]
    fn apply_cast_magic_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
        random_draws: Option<&mut Vec<EngineRandomDraw>>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::CastMagic {
            ally,
            ally_destination,
            ally_strike_location,
            card_id,
            card_instance_id,
            caster_instance_id,
            cemetery_minion_instance_id,
            discard_site_instance_id,
            draw_zone,
            target,
            target_location,
            target_site_instance_id,
            tempted_destination,
            tempted_enemy,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        let player_index = seat_index(seat);
        let player = &self.position.players[player_index];
        let caster_kind = self.spellcaster_kind(seat, caster_instance_id);
        if self.position.phase != Phase::Main || !player.domain_established || caster_kind.is_none()
        {
            return Err(GameError::IllegalAction);
        }
        let hand_index = player
            .hand_spellbook
            .iter()
            .position(|card| {
                card.instance_id == *card_instance_id
                    && self.rules.cards[usize::from(card.card_id.0)].id == card_id.as_str()
            })
            .ok_or(GameError::IllegalAction)?;
        let definition =
            &self.rules.cards[usize::from(player.hand_spellbook[hand_index].card_id.0)];
        let CardFacts::Magic(facts) = &definition.facts else {
            return Err(GameError::IllegalAction);
        };
        if facts.mana_cost > u64::from(player.mana)
            || !self.thresholds_met(seat, facts.thresholds)
            || !self
                .magic_choices(seat, caster_instance_id, &facts.effect)?
                .contains(&MagicChoice {
                    ally: ally.clone(),
                    ally_destination: *ally_destination,
                    ally_strike_location: *ally_strike_location,
                    cemetery_minion_instance_id: cemetery_minion_instance_id.clone(),
                    discard_site_instance_id: discard_site_instance_id.clone(),
                    draw_zone: *draw_zone,
                    target: target.clone(),
                    target_location: *target_location,
                    target_site_instance_id: target_site_instance_id.clone(),
                    tempted_destination: *tempted_destination,
                    tempted_enemy: tempted_enemy.clone(),
                })
        {
            return Err(GameError::IllegalAction);
        }
        let effect = facts.effect.clone();
        let thresholds = facts.thresholds;
        let mana_paid = u16::try_from(facts.mana_cost).map_err(|_| GameError::IllegalAction)?;
        let next_air_thresholds_cast_this_turn = player
            .air_thresholds_cast_this_turn
            .map(|cast_air| {
                let added = u16::try_from(thresholds.get(Element::Air))
                    .map_err(|_| GameError::IllegalAction)?;
                cast_air.checked_add(added).ok_or(GameError::IllegalAction)
            })
            .transpose()?;
        let token_units = match &effect {
            MagicEffect::SummonTokenToEachControlledSiteBorderingEnemySite(token_card_id) => {
                let enemy = other_seat(seat);
                self.controlled_site_cells(seat)
                    .filter(|cell| {
                        cell.bordering(false).any(|bordering| {
                            self.position.sites[bordering.index()]
                                .as_ref()
                                .is_some_and(|site| site.controller == enemy)
                        })
                    })
                    .enumerate()
                    .map(|(ordinal, cell)| {
                        self.create_token_unit(
                            seat,
                            token_card_id,
                            card_instance_id,
                            cell,
                            ordinal,
                            self.position.state_version,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => Vec::new(),
        };
        let duel = if effect == MagicEffect::FightAllyWithAdjacentEnemy {
            let ally = ally.as_ref().ok_or(GameError::IllegalAction)?;
            let target = target.as_ref().ok_or(GameError::IllegalAction)?;
            let attacker_kind = match ally {
                UnitTarget::Avatar { .. } => UnitKind::Avatar,
                UnitTarget::Minion { .. } => UnitKind::Minion,
            };
            let region = self.unit_target_region(ally)?;
            let cell = match ally {
                UnitTarget::Avatar { instance_id, seat } => {
                    let avatar = &self.position.players[seat_index(*seat)].avatar;
                    if avatar.card.instance_id != *instance_id {
                        return Err(GameError::IllegalAction);
                    }
                    avatar.location
                }
                UnitTarget::Minion { instance_id, seat } => self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                    .map(|unit| unit.location)
                    .ok_or(GameError::IllegalAction)?,
            };
            let (original_target, warded_target_index) = match target {
                UnitTarget::Avatar { instance_id, seat } => {
                    let avatar = &self.position.players[seat_index(*seat)].avatar;
                    if avatar.card.instance_id != *instance_id {
                        return Err(GameError::IllegalAction);
                    }
                    (
                        CombatTarget::Avatar {
                            instance_id: instance_id.clone(),
                            seat: *seat,
                        },
                        None,
                    )
                }
                UnitTarget::Minion { instance_id, seat } => {
                    let index = self
                        .position
                        .units
                        .iter()
                        .position(|unit| {
                            unit.controller == *seat && unit.card.instance_id == *instance_id
                        })
                        .ok_or(GameError::IllegalAction)?;
                    (
                        CombatTarget::Minion {
                            instance_id: instance_id.clone(),
                            seat: *seat,
                        },
                        self.position.units[index].warded.then_some(index),
                    )
                }
            };
            Some((
                PendingCombat {
                    allocations: Vec::new(),
                    attacker_instance_id: ally.instance_id().clone(),
                    attacker_kind,
                    attacking_seat: seat,
                    cell,
                    combatants: vec![target.clone()],
                    defenders: Vec::new(),
                    original_target: Some(original_target),
                    region,
                    target_removed: false,
                },
                warded_target_index,
            ))
        } else {
            None
        };
        // The additional site discard is a cost, so it is paid before the cast is announced.
        if let Some(discard_site_instance_id) = discard_site_instance_id {
            let player = &mut self.position.players[player_index];
            let atlas_index = player
                .hand_atlas
                .iter()
                .position(|card| card.instance_id == *discard_site_instance_id)
                .ok_or(GameError::IllegalAction)?;
            let discarded = player.hand_atlas.remove(atlas_index);
            outcomes.push("card-discarded", || {
                json!({
                    "cardId": self.rules.cards[usize::from(discarded.card_id.0)].id,
                    "instanceId": discarded.instance_id,
                    "owner": discarded.owner,
                    "seat": seat,
                    "sourceInstanceId": card_instance_id,
                    "zone": "atlas",
                })
            });
            self.position.players[player_index].cemetery.push(discarded);
        }
        let card = self.position.players[player_index]
            .hand_spellbook
            .remove(hand_index);
        let compact_card_id = card.card_id;
        let owner = card.owner;
        let player = &mut self.position.players[player_index];
        player.mana -= mana_paid;
        player.air_thresholds_cast_this_turn = next_air_thresholds_cast_this_turn;
        self.position.players[seat_index(owner)].cemetery.push(card);
        outcomes.push("magic-cast", || {
            let mut payload = json!({
                "cardId": card_id,
                "casterInstanceId": caster_instance_id,
                "instanceId": card_instance_id,
                "manaPaid": mana_paid,
                "seat": seat,
            });
            if let Some(selected_id) = cemetery_minion_instance_id {
                payload["cemeteryMinionInstanceId"] = json!(selected_id);
            }
            if let Some(discard_site_instance_id) = discard_site_instance_id {
                payload["discardSiteInstanceId"] = json!(discard_site_instance_id);
            }
            if let Some(ally) = ally {
                payload["allyInstanceId"] = json!(ally.instance_id());
                payload["allySeat"] = json!(ally.seat());
            }
            if let Some(destination) = ally_destination {
                payload["allyDestination"] = json!(destination);
            }
            if let Some(strike) = ally_strike_location {
                payload["allyStrikeLocation"] = json!(strike);
            }
            if let Some(target) = target {
                payload["targetInstanceId"] = json!(target.instance_id());
                payload["targetSeat"] = json!(target.seat());
            }
            if let Some(target_location) = target_location {
                payload["targetLocation"] = json!(target_location);
            }
            if let Some(target_site_instance_id) = target_site_instance_id {
                payload["targetSiteInstanceId"] = json!(target_site_instance_id);
            }
            payload
        });
        self.record_unit_interaction(
            caster_kind.ok_or(GameError::IllegalAction)?,
            seat,
            caster_instance_id,
            outcomes,
        )?;
        let leap_attack = effect == MagicEffect::LeapAttackAlly;
        let blink = effect == MagicEffect::TeleportNearbyAllyThenDrawCard;
        // A raised minion resolves its Magic from the free placement it still owes.
        let mut raising = false;
        match effect {
            MagicEffect::HealController(amount) => {
                self.heal_avatar(seat, u16::from(amount), card_instance_id, outcomes)?;
            }
            MagicEffect::SummonRandomMinionFromAnyCemetery => {
                raising = self.begin_cemetery_summon(
                    CemeterySummonRequest {
                        card_instance_id,
                        caster_instance_id,
                        seat,
                        source_magic_card_id: compact_card_id,
                        source_magic_owner: owner,
                    },
                    outcomes,
                    random_draws,
                )?;
            }
            MagicEffect::ReturnMinionFromOwnCemetery => {
                if let Some(selected_id) = cemetery_minion_instance_id {
                    let player = &mut self.position.players[player_index];
                    let selected_index = player
                        .cemetery
                        .iter()
                        .position(|card| card.instance_id == *selected_id)
                        .ok_or(GameError::IllegalAction)?;
                    let selected = player.cemetery.remove(selected_index);
                    let selected_card_id =
                        self.rules.cards[usize::from(selected.card_id.0)].id.clone();
                    let selected_owner = selected.owner;
                    player.hand_spellbook.push(selected);
                    outcomes.push("minion-returned-to-hand", || {
                        json!({
                            "cardId": selected_card_id,
                            "instanceId": selected_id,
                            "owner": selected_owner,
                            "seat": seat,
                            "sourceInstanceId": card_instance_id,
                        })
                    });
                }
            }
            MagicEffect::GrantChargeToAllyThisTurn => {
                let ally = ally.as_ref().ok_or(GameError::IllegalAction)?;
                if let UnitTarget::Minion {
                    instance_id,
                    seat: ally_seat,
                } = ally
                {
                    self.position
                        .units
                        .iter_mut()
                        .find(|unit| {
                            unit.card.instance_id == *instance_id && unit.controller == *ally_seat
                        })
                        .ok_or(GameError::IllegalAction)?
                        .temporary_charge_sources
                        .push(card_instance_id.clone());
                }
                outcomes.push("charge-granted", || {
                    json!({
                        "instanceId": ally.instance_id(),
                        "seat": ally.seat(),
                        "sourceInstanceId": card_instance_id,
                    })
                });
            }
            MagicEffect::GrantPowerTwoToAllyThisTurn => {
                let ally = ally.as_ref().ok_or(GameError::IllegalAction)?;
                match ally {
                    UnitTarget::Avatar {
                        instance_id,
                        seat: ally_seat,
                    } => {
                        let avatar = &mut self.position.players[seat_index(*ally_seat)].avatar;
                        if avatar.card.instance_id != *instance_id {
                            return Err(GameError::IllegalAction);
                        }
                        avatar
                            .temporary_power_sources
                            .push(card_instance_id.clone());
                    }
                    UnitTarget::Minion {
                        instance_id,
                        seat: ally_seat,
                    } => self
                        .position
                        .units
                        .iter_mut()
                        .find(|unit| {
                            unit.card.instance_id == *instance_id && unit.controller == *ally_seat
                        })
                        .ok_or(GameError::IllegalAction)?
                        .temporary_power_sources
                        .push(card_instance_id.clone()),
                }
                outcomes.push("power-granted", || {
                    json!({
                        "amount": 2,
                        "instanceId": ally.instance_id(),
                        "seat": ally.seat(),
                        "sourceInstanceId": card_instance_id,
                    })
                });
            }
            MagicEffect::LeapAttackAlly => {
                self.apply_leap_attack(
                    LeapAttackRequest {
                        ally: ally.as_ref().ok_or(GameError::IllegalAction)?,
                        card_id,
                        card_instance_id,
                        destination: ally_destination.ok_or(GameError::IllegalAction)?,
                        owner,
                        seat,
                        strike_location: *ally_strike_location,
                    },
                    outcomes,
                )?;
            }
            MagicEffect::TeleportAllyToTargetSite => {
                let ally = ally.as_ref().ok_or(GameError::IllegalAction)?;
                let destination = target_location.ok_or(GameError::IllegalAction)?;
                let target_site_instance_id = target_site_instance_id
                    .as_ref()
                    .ok_or(GameError::IllegalAction)?
                    .clone();
                let from = self.unit_target_location(ally)?;
                let to = self.move_unit_target_to(ally, destination)?;
                if to != from {
                    let instance_id = ally.instance_id().clone();
                    let ally_seat = ally.seat();
                    let source_instance_id = card_instance_id.clone();
                    outcomes.push("unit-teleported", || {
                        json!({
                            "from": from,
                            "seat": ally_seat,
                            "sourceInstanceId": source_instance_id,
                            "targetInstanceId": instance_id,
                            "targetSiteInstanceId": target_site_instance_id,
                            "to": to,
                        })
                    });
                    self.settle_lower_region_minion_deaths(outcomes)?;
                    self.settle_static_power_deaths(outcomes)?;
                }
            }
            MagicEffect::TeleportNearbyAllyThenDrawCard => {
                let ally = ally.as_ref().ok_or(GameError::IllegalAction)?;
                let destination = target_location.ok_or(GameError::IllegalAction)?;
                let zone = draw_zone.ok_or(GameError::IllegalAction)?;
                let from = self.unit_target_location(ally)?;
                let to = self.move_unit_target_to(ally, destination)?;
                if to != from {
                    let instance_id = ally.instance_id().clone();
                    let ally_seat = ally.seat();
                    let source_instance_id = card_instance_id.clone();
                    let site_instance_id = target_site_instance_id.clone();
                    outcomes.push("unit-teleported", || {
                        let mut payload = json!({
                            "from": from,
                            "seat": ally_seat,
                            "sourceInstanceId": source_instance_id,
                            "targetInstanceId": instance_id,
                            "to": to,
                        });
                        if let Some(site_instance_id) = site_instance_id {
                            payload["targetSiteInstanceId"] = json!(site_instance_id);
                        }
                        payload
                    });
                    self.settle_lower_region_minion_deaths(outcomes)?;
                    self.settle_static_power_deaths(outcomes)?;
                }
                let continuation = BlinkContinuation {
                    card_id: card_id.clone(),
                    instance_id: card_instance_id.clone(),
                    owner,
                    seat,
                    zone,
                };
                if let Some(pending) = &mut self.position.pending_deathrites {
                    pending.continuation = Some(DeathriteContinuation::Blink(continuation));
                    pending.return_decision_seat = seat;
                    pending.return_phase = Phase::Main;
                } else {
                    self.finish_blink(&continuation, outcomes);
                }
            }
            MagicEffect::LureEnemyMinionOneStepCloser => {
                if let (Some(ally), Some(enemy), Some(destination)) =
                    (ally.as_ref(), tempted_enemy.as_ref(), *tempted_destination)
                {
                    let from = self.unit_target_location(enemy)?;
                    let to = self.move_unit_target_to(enemy, destination)?;
                    let ally_instance_id = ally.instance_id().clone();
                    let enemy_instance_id = enemy.instance_id().clone();
                    let enemy_seat = enemy.seat();
                    let source_instance_id = card_instance_id.clone();
                    let path = if to == from {
                        vec![from]
                    } else {
                        vec![from, to]
                    };
                    outcomes.push("unit-lured", || {
                        json!({
                            "allyInstanceId": ally_instance_id,
                            "from": from,
                            "path": path,
                            "seat": enemy_seat,
                            "sourceInstanceId": source_instance_id,
                            "steps": path.len() - 1,
                            "targetInstanceId": enemy_instance_id,
                            "to": to,
                        })
                    });
                    self.settle_lower_region_minion_deaths(outcomes)?;
                    self.settle_nearby_enemy_stealth(outcomes);
                    self.settle_static_power_deaths(outcomes)?;
                }
            }
            MagicEffect::FightAllyWithAdjacentEnemy => {
                let Some((pending, warded_target_index)) = duel else {
                    return Err(GameError::IllegalAction);
                };
                if let Some(target_index) = warded_target_index {
                    let target = &mut self.position.units[target_index];
                    target.warded = false;
                    outcomes.push("ward-broken", || {
                        json!({
                            "instanceId": target.card.instance_id,
                            "seat": target.controller,
                        })
                    });
                } else {
                    let combatants = pending.combatants.clone();
                    self.position.pending_combat = Some(pending);
                    self.begin_fight(combatants, outcomes)?;
                }
            }
            MagicEffect::SummonTokenToEachControlledSiteBorderingEnemySite(_) => {
                for token in token_units {
                    let token_card_id = self.rules.cards[usize::from(token.card.card_id.0)]
                        .id
                        .clone();
                    let cell = token.location;
                    let instance_id = token.card.instance_id.clone();
                    let token_owner = token.card.owner;
                    self.position.units.push(token);
                    outcomes.push("minion-summoned", || {
                        json!({
                            "cardId": token_card_id,
                            "cell": cell,
                            "instanceId": instance_id,
                            "owner": token_owner,
                            "seat": seat,
                            "sourceInstanceId": card_instance_id,
                            "token": true,
                        })
                    });
                }
            }
            MagicEffect::BurrowAllMinionsAndArtifactsAtTargetLandSite => {
                let Some(Location {
                    cell,
                    region: Region::Surface,
                }) = target_location
                else {
                    return Err(GameError::IllegalAction);
                };
                let mut moved = self
                    .position
                    .units
                    .iter()
                    .filter(|unit| {
                        unit.region == Region::Surface
                            && Self::unit_occupies_cell(unit, *cell)
                            && Self::unit_occupied_cells(unit)
                                .iter()
                                .all(|occupied| self.underground_location_exists(*occupied))
                    })
                    .map(|unit| (unit.card.instance_id.clone(), unit.controller))
                    .collect::<Vec<_>>();
                moved.sort_unstable_by(|left, right| left.0.cmp(&right.0));
                for (instance_id, target_seat) in moved {
                    self.position
                        .units
                        .iter_mut()
                        .find(|unit| unit.card.instance_id == instance_id)
                        .ok_or(GameError::IllegalAction)?
                        .region = Region::Underground;
                    outcomes.push("minion-burrowed", || {
                        json!({
                            "cell": cell,
                            "instanceId": instance_id,
                            "seat": target_seat,
                            "sourceInstanceId": card_instance_id,
                        })
                    });
                }
            }
            MagicEffect::BurrowTargetMinionOrArtifact | MagicEffect::SubmergeTargetMinion => {
                let submerged = matches!(effect, MagicEffect::SubmergeTargetMinion);
                let Some(UnitTarget::Minion {
                    instance_id,
                    seat: target_seat,
                }) = target
                else {
                    return Err(GameError::IllegalAction);
                };
                let target_index = self
                    .position
                    .units
                    .iter()
                    .position(|unit| {
                        unit.card.instance_id == *instance_id && unit.controller == *target_seat
                    })
                    .ok_or(GameError::IllegalAction)?;
                if self.position.units[target_index].warded && *target_seat != seat {
                    self.position.units[target_index].warded = false;
                    outcomes.push(
                        "ward-broken",
                        || json!({ "instanceId": instance_id, "seat": target_seat }),
                    );
                } else {
                    let destination = if submerged {
                        Region::Underwater
                    } else {
                        Region::Underground
                    };
                    let can_move = self.position.units[target_index].region == Region::Surface
                        && Self::unit_occupied_cells(&self.position.units[target_index])
                            .iter()
                            .all(|cell| self.location_exists_in_region(*cell, destination));
                    if can_move {
                        self.position.units[target_index].region = destination;
                        let cell = self.position.units[target_index].location;
                        outcomes.push(
                            if submerged {
                                "minion-submerged"
                            } else {
                                "minion-burrowed"
                            },
                            || {
                                json!({
                                    "cell": cell,
                                    "instanceId": instance_id,
                                    "seat": target_seat,
                                    "sourceInstanceId": card_instance_id,
                                })
                            },
                        );
                    }
                }
            }
            MagicEffect::DamageRandomUnitAtLocation(amount) => {
                let location = target_location.ok_or(GameError::IllegalAction)?;
                let candidates = self.units_at_location(location);
                if !candidates.is_empty() {
                    let index = draw_index(
                        &mut self.position.prng,
                        candidates.len(),
                        "magic_random_unit_at_location",
                        "unit_index_candidate",
                        random_draws,
                    )?;
                    let (target_instance_id, target_kind, target_seat) = candidates[index].clone();
                    let allocated = u16::from(amount);
                    outcomes.push("magic-damage-allocated", || {
                        json!({
                            "amount": allocated,
                            "sourceInstanceId": card_instance_id,
                            "targetInstanceId": target_instance_id,
                        })
                    });
                    self.damage_unit_and_settle_deaths(
                        &(target_instance_id, target_kind, target_seat),
                        allocated,
                        UnitDamageSource {
                            current_power: 0,
                            lethal: false,
                        },
                        outcomes,
                    )?;
                }
            }
            MagicEffect::GainControlOfTargetNearbyMinion => {
                let Some(UnitTarget::Minion {
                    instance_id,
                    seat: target_seat,
                }) = target
                else {
                    return Err(GameError::IllegalAction);
                };
                let unit = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| {
                        unit.card.instance_id == *instance_id && unit.controller == *target_seat
                    })
                    .ok_or(GameError::IllegalAction)?;
                if unit.controller != seat {
                    if unit.warded {
                        unit.warded = false;
                        outcomes.push(
                            "ward-broken",
                            || json!({ "instanceId": instance_id, "seat": target_seat }),
                        );
                    } else {
                        unit.controller = seat;
                        outcomes.push("minion-control-changed", || {
                            json!({
                                "fromSeat": target_seat,
                                "instanceId": instance_id,
                                "seat": seat,
                                "sourceInstanceId": card_instance_id,
                            })
                        });
                        self.settle_static_power_deaths(outcomes)?;
                    }
                }
            }
            MagicEffect::KillTargetWoundedMinion => {
                let Some(UnitTarget::Minion {
                    instance_id,
                    seat: target_seat,
                }) = target
                else {
                    return Err(GameError::IllegalAction);
                };
                let unit = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| {
                        unit.card.instance_id == *instance_id && unit.controller == *target_seat
                    })
                    .ok_or(GameError::IllegalAction)?;
                // A Ward absorbs the kill outright, even when its own controller casts the Magic.
                if unit.warded {
                    unit.warded = false;
                    outcomes.push(
                        "ward-broken",
                        || json!({ "instanceId": instance_id, "seat": target_seat }),
                    );
                } else {
                    let card_id = self.rules.cards[usize::from(unit.card.card_id.0)]
                        .id
                        .clone();
                    let owner = unit.card.owner;
                    outcomes.push("minion-killed", || {
                        json!({
                            "cardId": card_id,
                            "instanceId": instance_id,
                            "owner": owner,
                            "seat": target_seat,
                            "sourceInstanceId": card_instance_id,
                        })
                    });
                    self.begin_minion_deaths(
                        std::slice::from_ref(instance_id),
                        &[],
                        Phase::Main,
                        self.position.active_seat,
                        outcomes,
                    )?;
                }
            }
            MagicEffect::DisableTargetNearbyMinionUntilNextTurn => {
                let Some(UnitTarget::Minion {
                    instance_id,
                    seat: target_seat,
                }) = target
                else {
                    return Err(GameError::IllegalAction);
                };
                let unit = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| {
                        unit.card.instance_id == *instance_id && unit.controller == *target_seat
                    })
                    .ok_or(GameError::IllegalAction)?;
                if unit.warded && *target_seat != seat {
                    unit.warded = false;
                    outcomes.push(
                        "ward-broken",
                        || json!({ "instanceId": instance_id, "seat": target_seat }),
                    );
                } else {
                    let stealth_removed = unit.stealthed;
                    let ward_removed = unit.warded;
                    unit.disable_effects.push(DisableEffect {
                        expires_at_seat: seat,
                        source_instance_id: card_instance_id.clone(),
                    });
                    unit.stealthed = false;
                    unit.warded = false;
                    outcomes.push("minion-disabled", || {
                        json!({
                            "expiresAtSeat": seat,
                            "instanceId": instance_id,
                            "seat": target_seat,
                            "sourceInstanceId": card_instance_id,
                            "stealthRemoved": stealth_removed,
                            "wardRemoved": ward_removed,
                        })
                    });
                }
            }
            MagicEffect::DamageTargetUnit {
                amount,
                untap_target_minion_after_damage,
                ..
            } => {
                let target = target.as_ref().ok_or(GameError::IllegalAction)?;
                let target_instance_id = target.instance_id().clone();
                outcomes.push("magic-damage-allocated", || {
                    json!({
                        "amount": amount,
                        "sourceInstanceId": card_instance_id,
                        "targetInstanceId": target_instance_id,
                    })
                });
                let target_kind = match target {
                    UnitTarget::Avatar { .. } => UnitKind::Avatar,
                    UnitTarget::Minion { .. } => UnitKind::Minion,
                };
                let damage = self.apply_simple_damage(
                    target_kind,
                    target.seat(),
                    &target_instance_id,
                    u16::from(amount),
                    UnitDamageSource {
                        current_power: 0,
                        lethal: false,
                    },
                    outcomes,
                )?;
                if damage.minion_died || damage.avatar_defeated {
                    let defeated_seat = target.seat();
                    self.begin_minion_deaths(
                        if damage.minion_died {
                            std::slice::from_ref(&target_instance_id)
                        } else {
                            &[]
                        },
                        if damage.avatar_defeated {
                            std::slice::from_ref(&defeated_seat)
                        } else {
                            &[]
                        },
                        Phase::Main,
                        self.position.active_seat,
                        outcomes,
                    )?;
                }
                if untap_target_minion_after_damage && !damage.minion_died {
                    let UnitTarget::Minion {
                        seat: target_seat, ..
                    } = target
                    else {
                        return Err(GameError::IllegalAction);
                    };
                    let unit = self
                        .position
                        .units
                        .iter_mut()
                        .find(|unit| unit.card.instance_id == target_instance_id)
                        .ok_or(GameError::IllegalAction)?;
                    if unit.tapped {
                        unit.tapped = false;
                        outcomes.push("minion-untapped", || {
                            json!({
                                "instanceId": target_instance_id,
                                "seat": target_seat,
                                "sourceInstanceId": card_instance_id,
                            })
                        });
                    }
                }
            }
            MagicEffect::DamageEachAbovegroundMinionOne => {
                let mut targets = self
                    .position
                    .units
                    .iter()
                    .filter(|unit| unit.region == Region::Surface)
                    .map(|unit| {
                        Ok((
                            unit.card.instance_id.clone(),
                            unit.controller,
                            self.minion_damage_status(&unit.card.instance_id)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, GameError>>()?;
                targets.sort_unstable_by(|left, right| left.0.cmp(&right.0));
                for (instance_id, _, _) in &targets {
                    outcomes.push("magic-damage-allocated", || {
                        json!({
                            "amount": 1,
                            "sourceInstanceId": card_instance_id,
                            "targetInstanceId": instance_id,
                        })
                    });
                }
                let mut dead_minions = Vec::new();
                for (instance_id, target_seat, status) in targets {
                    if self
                        .apply_simple_damage_with_status(
                            UnitKind::Minion,
                            target_seat,
                            &instance_id,
                            1,
                            UnitDamageSource {
                                current_power: 0,
                                lethal: false,
                            },
                            Some(status),
                            outcomes,
                        )?
                        .minion_died
                    {
                        dead_minions.push(instance_id);
                    }
                }
                if !dead_minions.is_empty() {
                    self.begin_minion_deaths(
                        &dead_minions,
                        &[],
                        Phase::Main,
                        self.position.active_seat,
                        outcomes,
                    )?;
                }
            }
            MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(amount) => {
                let target_location = target_location.ok_or(GameError::IllegalAction)?;
                let mut targets = Vec::new();
                if target_location.region == Region::Surface {
                    for target_seat in [Seat::North, Seat::South] {
                        let avatar = &self.position.players[seat_index(target_seat)].avatar;
                        if avatar.location == target_location.cell {
                            targets.push((
                                avatar.card.instance_id.clone(),
                                UnitKind::Avatar,
                                target_seat,
                                None,
                            ));
                        }
                    }
                }
                targets.extend(
                    self.position
                        .units
                        .iter()
                        .filter(|unit| {
                            unit.region == target_location.region
                                && Self::unit_occupies_cell(unit, target_location.cell)
                        })
                        .map(|unit| {
                            Ok((
                                unit.card.instance_id.clone(),
                                UnitKind::Minion,
                                unit.controller,
                                Some(self.minion_damage_status(&unit.card.instance_id)?),
                            ))
                        })
                        .collect::<Result<Vec<_>, GameError>>()?,
                );
                targets.sort_unstable_by(|left, right| left.0.cmp(&right.0));
                for (instance_id, _, _, _) in &targets {
                    outcomes.push("magic-damage-allocated", || {
                        json!({
                            "amount": amount,
                            "sourceInstanceId": card_instance_id,
                            "targetInstanceId": instance_id,
                        })
                    });
                }
                let mut dead_minions = Vec::new();
                let mut defeated_avatars = Vec::new();
                for (instance_id, kind, target_seat, status) in targets {
                    let damage = self.apply_simple_damage_with_status(
                        kind,
                        target_seat,
                        &instance_id,
                        u16::from(amount),
                        UnitDamageSource {
                            current_power: 0,
                            lethal: false,
                        },
                        status,
                        outcomes,
                    )?;
                    if damage.minion_died {
                        dead_minions.push(instance_id);
                    }
                    if damage.avatar_defeated && !defeated_avatars.contains(&target_seat) {
                        defeated_avatars.push(target_seat);
                    }
                }
                if !dead_minions.is_empty() || !defeated_avatars.is_empty() {
                    self.begin_minion_deaths(
                        &dead_minions,
                        &defeated_avatars,
                        Phase::Main,
                        self.position.active_seat,
                        outcomes,
                    )?;
                }
            }
            MagicEffect::DestroyTargetSiteWithDamageGrid(grid) => {
                let cell = target_location.ok_or(GameError::IllegalAction)?.cell;
                let target_site_instance_id = target_site_instance_id
                    .as_ref()
                    .ok_or(GameError::IllegalAction)?;
                let target_site = self.position.sites[cell.index()]
                    .as_ref()
                    .filter(|site| site.card.instance_id == *target_site_instance_id)
                    .cloned();
                if target_site.is_none()
                    && self.position.rubble[cell.index()].as_ref() != Some(target_site_instance_id)
                {
                    return Err(GameError::IllegalAction);
                }
                // Rubble is already destroyed, and a protected site survives its own crater.
                let target_protected = target_site.as_ref().is_some_and(|site| {
                    matches!(
                        &self.rules.cards[usize::from(site.card.card_id.0)].facts,
                        CardFacts::Site(facts) if facts.cannot_be_moved_destroyed_or_modified
                    )
                });
                let targets = self.site_grid_damage_targets(cell, grid);
                let statuses = targets
                    .iter()
                    .map(|(instance_id, kind, _, _)| match kind {
                        UnitKind::Avatar => Ok(None),
                        UnitKind::Minion => self.minion_damage_status(instance_id).map(Some),
                    })
                    .collect::<Result<Vec<_>, GameError>>()?;
                outcomes.push(
                    if target_protected {
                        "site-destruction-prevented"
                    } else {
                        "site-destroyed"
                    },
                    || {
                        let mut payload = json!({
                            "cell": cell,
                            "instanceId": target_site_instance_id,
                            "sourceInstanceId": card_instance_id,
                        });
                        if let Some(site) = &target_site {
                            payload["owner"] = json!(site.card.owner);
                        }
                        payload
                    },
                );
                for (instance_id, _, _, amount) in &targets {
                    outcomes.push("magic-damage-allocated", || {
                        json!({
                            "amount": amount,
                            "sourceInstanceId": card_instance_id,
                            "targetInstanceId": instance_id,
                        })
                    });
                }
                let mut dead_minions = Vec::new();
                let mut defeated_avatars = Vec::new();
                for ((instance_id, kind, target_seat, amount), status) in
                    targets.into_iter().zip(statuses)
                {
                    let damage = self.apply_simple_damage_with_status(
                        kind,
                        target_seat,
                        &instance_id,
                        amount,
                        UnitDamageSource {
                            current_power: 0,
                            lethal: false,
                        },
                        status,
                        outcomes,
                    )?;
                    if damage.minion_died {
                        dead_minions.push(instance_id);
                    }
                    if damage.avatar_defeated && !defeated_avatars.contains(&target_seat) {
                        defeated_avatars.push(target_seat);
                    }
                }
                let destroyed_cards = match target_site {
                    Some(site) if !target_protected => {
                        let (cards, rubble) =
                            self.destroy_sites_into_rubble(vec![(cell, site)], card_instance_id)?;
                        for (rubble_cell, instance_id) in rubble {
                            outcomes.push("rubble-created", || {
                                json!({
                                    "cell": rubble_cell,
                                    "instanceId": instance_id,
                                    "sourceInstanceId": card_instance_id,
                                })
                            });
                        }
                        cards
                    }
                    _ => Vec::new(),
                };
                // The crater and its damage settle together, so drained minions die alongside
                // the ones the grid killed outright.
                for instance_id in self.lower_region_minion_deaths() {
                    if !dead_minions.contains(&instance_id) {
                        dead_minions.push(instance_id);
                    }
                }
                if !dead_minions.is_empty() || !defeated_avatars.is_empty() {
                    self.begin_minion_deaths(
                        &dead_minions,
                        &defeated_avatars,
                        Phase::Main,
                        self.position.active_seat,
                        outcomes,
                    )?;
                }
                for card in destroyed_cards {
                    self.position.players[seat_index(card.owner)]
                        .cemetery
                        .push(card);
                }
            }
            // Chained Magic resolves through its own pending decision instead.
            MagicEffect::DamageChainNearbyUnits => return Err(GameError::IllegalAction),
        }
        if !leap_attack && !blink && !raising {
            if let Some(pending) = &mut self.position.pending_deathrites {
                pending.deferred_magic_resolved = Some(DeferredMagicResolved {
                    card_id: compact_card_id,
                    instance_id: card_instance_id.clone(),
                    owner,
                });
            } else {
                let resolved_start = outcomes.len();
                outcomes.push("magic-resolved", || {
                    json!({
                        "cardId": card_id,
                        "instanceId": card_instance_id,
                        "owner": owner,
                    })
                });
                outcomes.move_tail_before_completion(resolved_start);
            }
        }
        self.position.state_version += 1;
        Ok(())
    }

    fn apply_leap_attack(
        &mut self,
        leap: LeapAttackRequest<'_>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let LeapAttackRequest {
            ally,
            card_id,
            card_instance_id,
            destination,
            owner,
            seat,
            strike_location,
        } = leap;
        let from = self.unit_target_location(ally)?;
        let stepped_to = self.move_unit_target_to(ally, destination)?;
        if stepped_to != from {
            let instance_id = ally.instance_id().clone();
            let ally_seat = ally.seat();
            let source_instance_id = card_instance_id.clone();
            outcomes.push("unit-stepped", || {
                json!({
                    "from": from,
                    "instanceId": instance_id,
                    "seat": ally_seat,
                    "sourceInstanceId": source_instance_id,
                    "steps": 1,
                    "to": stepped_to,
                })
            });
        }
        self.settle_lower_region_minion_deaths(outcomes)?;
        self.settle_nearby_enemy_stealth(outcomes);
        self.settle_static_power_deaths(outcomes)?;
        let continuation = LeapAttackContinuation {
            ally: ally.clone(),
            card_id: card_id.to_owned(),
            instance_id: card_instance_id.clone(),
            owner,
            strike_location: strike_location.unwrap_or(stepped_to),
        };
        if let Some(pending) = &mut self.position.pending_deathrites {
            pending.continuation = Some(DeathriteContinuation::LeapAttack(continuation));
            pending.return_decision_seat = seat;
            pending.return_phase = Phase::Main;
            return Ok(());
        }
        self.finish_leap_attack(&continuation, outcomes)
    }

    fn move_unit_target_to(
        &mut self,
        ally: &UnitTarget,
        destination: Location,
    ) -> Result<Location, GameError> {
        let from = self.unit_target_location(ally)?;
        if from == destination {
            return Ok(from);
        }
        match ally {
            UnitTarget::Avatar { instance_id, seat } => {
                let avatar = &mut self.position.players[seat_index(*seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                avatar.location = destination.cell;
            }
            UnitTarget::Minion { instance_id, seat } => {
                let unit = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                    .ok_or(GameError::IllegalAction)?;
                Self::move_minion_to(unit, destination.cell)?;
                unit.region = destination.region;
            }
        }
        Ok(destination)
    }

    fn finish_leap_attack(
        &mut self,
        continuation: &LeapAttackContinuation,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ally_remains = self.ally_remains(&continuation.ally);
        if self.position.terminal.is_some() || !ally_remains {
            Self::emit_leap_magic_resolved(continuation, outcomes);
            return Ok(());
        }
        let striker_kind = match continuation.ally {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        let striker_location = self.unit_target_location(&continuation.ally)?;
        let occupied = self.combatant_occupied_cells(
            striker_kind,
            continuation.ally.seat(),
            continuation.ally.instance_id(),
        )?;
        let can_occupy_strike = striker_location.region == continuation.strike_location.region
            && occupied.contains(&continuation.strike_location.cell);
        let disabled = match &continuation.ally {
            UnitTarget::Avatar { .. } => false,
            UnitTarget::Minion { instance_id, seat } => self
                .position
                .units
                .iter()
                .find(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id)
                .is_some_and(|unit| self.minion_is_disabled(unit)),
        };
        let enemies = if can_occupy_strike && !disabled {
            self.enemies_occupying(
                other_seat(continuation.ally.seat()),
                continuation.strike_location,
            )
        } else {
            Vec::new()
        };
        if enemies.is_empty() {
            Self::emit_leap_magic_resolved(continuation, outcomes);
            return Ok(());
        }
        let amount = self
            .combatant_strike_stats(
                striker_kind,
                continuation.ally.seat(),
                continuation.ally.instance_id(),
            )?
            .amount;
        let allocations: Vec<_> = enemies
            .iter()
            .map(|enemy| StrikeAllocation {
                amount,
                target_instance_id: enemy.instance_id().clone(),
            })
            .collect();
        let attacker_id = continuation.ally.instance_id().clone();
        for enemy in &enemies {
            let striker_id = attacker_id.clone();
            let target_id = enemy.instance_id().clone();
            outcomes.push("strike-damage-allocated", || {
                json!({
                    "amount": amount,
                    "strikerInstanceId": striker_id,
                    "targetInstanceId": target_id,
                })
            });
        }
        let pending = PendingCombat {
            allocations,
            attacker_instance_id: continuation.ally.instance_id().clone(),
            attacker_kind: striker_kind,
            attacking_seat: continuation.ally.seat(),
            cell: continuation.strike_location.cell,
            combatants: enemies,
            defenders: Vec::new(),
            original_target: None,
            region: striker_location.region,
            target_removed: false,
        };
        self.resolve_fight_window(&pending, true, &[], None, outcomes)?;
        Self::emit_leap_magic_resolved(continuation, outcomes);
        Ok(())
    }

    fn ally_remains(&self, ally: &UnitTarget) -> bool {
        match ally {
            UnitTarget::Avatar { instance_id, seat } => {
                self.position.players[seat_index(*seat)]
                    .avatar
                    .card
                    .instance_id
                    == *instance_id
            }
            UnitTarget::Minion { instance_id, seat } => self
                .position
                .units
                .iter()
                .any(|unit| unit.controller == *seat && unit.card.instance_id == *instance_id),
        }
    }

    fn enemies_occupying(&self, enemy_seat: Seat, location: Location) -> Vec<UnitTarget> {
        let player = &self.position.players[seat_index(enemy_seat)];
        let mut enemies = Vec::new();
        if location.region == Region::Surface && player.avatar.location == location.cell {
            enemies.push(UnitTarget::Avatar {
                instance_id: player.avatar.card.instance_id.clone(),
                seat: enemy_seat,
            });
        }
        enemies.extend(
            self.position
                .units
                .iter()
                .filter(|unit| {
                    unit.controller == enemy_seat
                        && unit.region == location.region
                        && Self::unit_occupies_cell(unit, location.cell)
                })
                .map(|unit| UnitTarget::Minion {
                    instance_id: unit.card.instance_id.clone(),
                    seat: enemy_seat,
                }),
        );
        enemies.sort_unstable_by(|left, right| left.instance_id().cmp(right.instance_id()));
        enemies
    }

    /// Pays Blink's private draw once the teleport and any interrupting Deathrites are settled.
    fn finish_blink(&mut self, continuation: &BlinkContinuation, outcomes: &mut OutcomeLog<'_>) {
        if self.position.terminal.is_none() {
            self.apply_genesis_draws(
                continuation.seat,
                &continuation.instance_id,
                continuation.zone,
                1,
                outcomes,
            );
        }
        Self::emit_continuation_magic_resolved(
            &continuation.card_id,
            &continuation.instance_id,
            continuation.owner,
            outcomes,
        );
    }

    /// Completes a Magic whose own effect, not the shared cast tail, owns its resolution event.
    fn emit_continuation_magic_resolved(
        card_id: &str,
        instance_id: &IdentityHash,
        owner: Seat,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        let resolved_start = outcomes.len();
        outcomes.push("magic-resolved", || {
            json!({
                "cardId": card_id,
                "instanceId": instance_id,
                "owner": owner,
            })
        });
        outcomes.move_tail_before_completion(resolved_start);
    }

    fn emit_leap_magic_resolved(
        continuation: &LeapAttackContinuation,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        Self::emit_continuation_magic_resolved(
            &continuation.card_id,
            &continuation.instance_id,
            continuation.owner,
            outcomes,
        );
    }

    /// Resolves an interrupted Magic that the terminal result denied its own continuation.
    fn emit_interrupted_magic_resolved(
        continuation: Option<&DeathriteContinuation>,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        let resolution = match continuation {
            Some(DeathriteContinuation::Blink(blink)) => {
                (&blink.card_id, &blink.instance_id, blink.owner)
            }
            Some(DeathriteContinuation::LeapAttack(leap)) => {
                (&leap.card_id, &leap.instance_id, leap.owner)
            }
            _ => return,
        };
        Self::emit_continuation_magic_resolved(resolution.0, resolution.1, resolution.2, outcomes);
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the closed summon transaction keeps caster validation, payment, and Genesis atomic"
    )]
    fn apply_summon_minion_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
        random_draws: Option<&mut Vec<EngineRandomDraw>>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::SummonMinion {
            card_id,
            card_instance_id,
            caster_instance_id,
            cell,
            cells,
            genesis_damage_choice,
            genesis_damage_target,
            mana_cost,
            payment_mode,
            sacrificed_minion_instance_ids,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        if self.position.phase == Phase::CemeterySummon {
            return self.apply_cemetery_summon_action(action, outcomes);
        }
        let player_index = seat_index(seat);
        let player = &self.position.players[player_index];
        let caster_kind = self.spellcaster_kind(seat, caster_instance_id);
        if self.position.phase != Phase::Main || !player.domain_established || caster_kind.is_none()
        {
            return Err(GameError::IllegalAction);
        }
        let hand_index = player
            .hand_spellbook
            .iter()
            .position(|card| {
                card.instance_id == *card_instance_id
                    && self.rules.cards[usize::from(card.card_id.0)].id == card_id.as_str()
            })
            .ok_or(GameError::IllegalAction)?;
        let definition =
            &self.rules.cards[usize::from(player.hand_spellbook[hand_index].card_id.0)];
        let CardFacts::Minion(facts) = &definition.facts else {
            return Err(GameError::IllegalAction);
        };
        let genesis = facts.genesis;
        let lance_count = facts.lance_count;
        let starts_stealthed = facts.stealth;
        let starts_warded = facts.damage_prevention == Some(DamagePrevention::Ward);
        let Some(destination) = self
            .summon_destinations(seat, facts)
            .into_iter()
            .find(|destination| destination.cell == *cell && destination.cells == *cells)
        else {
            return Err(GameError::IllegalAction);
        };
        let sacrifices = sacrificed_minion_instance_ids
            .as_deref()
            .unwrap_or_default();
        if sacrificed_minion_instance_ids
            .as_ref()
            .is_some_and(Vec::is_empty)
        {
            return Err(GameError::IllegalAction);
        }
        let sacrifice_payment_matches = if sacrifices.is_empty() {
            false
        } else {
            let mut sacrifice_candidates = self
                .position
                .units
                .iter()
                .filter(|unit| {
                    unit.controller == seat
                        && unit.region == Region::Surface
                        && Self::unit_occupies_cell(unit, *cell)
                })
                .map(|unit| unit.card.instance_id.clone())
                .collect::<Vec<_>>();
            sacrifice_candidates.sort_unstable();
            let useful_sacrifice_count = usize::try_from(destination.mana_cost.div_ceil(2))
                .unwrap_or(usize::MAX)
                .min(sacrifice_candidates.len());
            payment_mode.is_none()
                && facts.alternative_summon_payment
                    == Some(
                        AlternativeSummonPayment::SacrificeMinionAtSummoningLocationForManaDiscountTwo,
                    )
                && sacrifices.len() <= useful_sacrifice_count
                && sacrifices.windows(2).all(|pair| pair[0] < pair[1])
                && sacrifices
                    .iter()
                    .all(|instance_id| sacrifice_candidates.binary_search(instance_id).is_ok())
                && u64::try_from(sacrifices.len()).is_ok_and(|count| {
                    destination
                        .mana_cost
                        .saturating_sub(count.saturating_mul(2))
                        == *mana_cost
                })
                && *mana_cost <= u64::from(player.mana)
        };
        let payment_matches = if sacrifices.is_empty() {
            match payment_mode {
                None => destination.mana_cost == *mana_cost && *mana_cost <= u64::from(player.mana),
                Some(SummonPaymentMode::RandomCardDiscard) => {
                    *mana_cost == 0
                        && facts.alternative_summon_payment
                            == Some(AlternativeSummonPayment::DiscardRandomCardInsteadOfMana)
                }
            }
        } else {
            sacrifice_payment_matches
        };
        let discard_candidate_count =
            player.hand_atlas.len() + player.hand_spellbook.len().saturating_sub(1);
        if !payment_matches
            || (*payment_mode == Some(SummonPaymentMode::RandomCardDiscard)
                && discard_candidate_count == 0)
            || !self.thresholds_met(seat, facts.thresholds)
            || !self.valid_genesis_damage_choice(
                seat,
                card_instance_id,
                *cell,
                genesis,
                *genesis_damage_choice,
                genesis_damage_target.as_ref(),
            )
        {
            return Err(GameError::IllegalAction);
        }
        let next_air_thresholds_cast_this_turn = player
            .air_thresholds_cast_this_turn
            .map(|cast_air| {
                let added = u16::try_from(facts.thresholds.get(Element::Air))
                    .map_err(|_| GameError::IllegalAction)?;
                cast_air.checked_add(added).ok_or(GameError::IllegalAction)
            })
            .transpose()?;
        let discarded = if *payment_mode == Some(SummonPaymentMode::RandomCardDiscard) {
            let index = draw_index(
                &mut self.position.prng,
                discard_candidate_count,
                "summon_random_card_discard_cost",
                "card_index_candidate",
                random_draws,
            )?;
            Some(if index < player.hand_atlas.len() {
                (DeckZone::Atlas, index)
            } else {
                let spellbook_index = index - player.hand_atlas.len();
                (
                    DeckZone::Spellbook,
                    if spellbook_index >= hand_index {
                        spellbook_index + 1
                    } else {
                        spellbook_index
                    },
                )
            })
        } else {
            None
        };
        let paid_mana = u16::try_from(*mana_cost).map_err(|_| GameError::IllegalAction)?;
        let (card, discarded_card) = {
            let player = &mut self.position.players[player_index];
            let card = player.hand_spellbook.remove(hand_index);
            player.mana -= paid_mana;
            player.air_thresholds_cast_this_turn = next_air_thresholds_cast_this_turn;
            let discarded_card = discarded.map(|(zone, original_index)| {
                let discarded_card = match zone {
                    DeckZone::Atlas => player.hand_atlas.remove(original_index),
                    DeckZone::Spellbook => {
                        let index = if original_index > hand_index {
                            original_index - 1
                        } else {
                            original_index
                        };
                        player.hand_spellbook.remove(index)
                    }
                };
                (zone, discarded_card)
            });
            (card, discarded_card)
        };
        if let Some((zone, discarded_card)) = discarded_card {
            outcomes.push("card-discarded", || {
                json!({
                    "cardId": self.rules.cards[usize::from(discarded_card.card_id.0)].id,
                    "instanceId": discarded_card.instance_id,
                    "owner": discarded_card.owner,
                    "seat": seat,
                    "sourceInstanceId": card_instance_id,
                    "zone": match zone {
                        DeckZone::Atlas => "atlas",
                        DeckZone::Spellbook => "spellbook",
                    },
                })
            });
            self.position.players[player_index]
                .cemetery
                .push(discarded_card);
        }
        let caster_kind = caster_kind.ok_or(GameError::IllegalAction)?;
        let caster = match caster_kind {
            UnitKind::Avatar => UnitTarget::Avatar {
                instance_id: caster_instance_id.clone(),
                seat,
            },
            UnitKind::Minion => UnitTarget::Minion {
                instance_id: caster_instance_id.clone(),
                seat,
            },
        };
        let unit = UnitPosition {
            card,
            carried_lance_count: lance_count.unwrap_or(0),
            controller: seat,
            damage: 0,
            disable_effects: Vec::new(),
            disabled_until_damaged: false,
            last_dropped_artifacts_turn: None,
            last_interacted_turn: None,
            last_picked_up_artifacts_turn: None,
            location: *cell,
            occupied_cells: *cells,
            region: Region::Surface,
            stealthed: starts_stealthed,
            summoning_sickness: true,
            tapped: false,
            temporary_charge_sources: Vec::new(),
            temporary_power_sources: Vec::new(),
            warded: starts_warded,
        };
        let continuation = PaidSummonContinuation {
            caster,
            genesis_damage_choice: *genesis_damage_choice,
            genesis_damage_target: genesis_damage_target.clone(),
            mana_paid: paid_mana,
            sacrificed_minion_instance_ids: sacrifices.to_vec(),
            unit,
        };
        self.position.state_version += 1;
        if !sacrifices.is_empty() {
            for instance_id in sacrifices {
                let sacrificed_unit = self
                    .position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == *instance_id)
                    .ok_or(GameError::IllegalAction)?;
                outcomes.push("minion-sacrificed", || {
                    json!({
                        "cardId": self.rules.cards[usize::from(sacrificed_unit.card.card_id.0)].id,
                        "instanceId": sacrificed_unit.card.instance_id,
                        "owner": sacrificed_unit.card.owner,
                        "seat": sacrificed_unit.controller,
                        "sourceInstanceId": card_instance_id,
                    })
                });
            }
            return self.begin_minion_deaths_with_continuation(
                sacrifices,
                &[],
                Phase::Main,
                seat,
                Some(DeathriteContinuation::PaidSummon(continuation)),
                outcomes,
            );
        }
        self.finish_paid_summon(continuation, outcomes)
    }

    fn finish_paid_summon(
        &mut self,
        continuation: PaidSummonContinuation,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        self.finish_summon(continuation, None, outcomes)
    }

    /// Places one summoned minion and resolves its Genesis.
    ///
    /// A `raised` placement carries the free summon a cemetery Magic still owed, so it skips the
    /// caster interaction a paid summon records and completes that Magic once the minion lands.
    fn finish_summon(
        &mut self,
        continuation: PaidSummonContinuation,
        raised: Option<PendingCemeterySummon>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let PaidSummonContinuation {
            caster,
            genesis_damage_choice,
            genesis_damage_target,
            mana_paid,
            sacrificed_minion_instance_ids: _,
            unit,
        } = continuation;
        let seat = unit.controller;
        let card_id = unit.card.card_id;
        let card_instance_id = unit.card.instance_id.clone();
        let card_owner = unit.card.owner;
        let caster_instance_id = caster.instance_id().clone();
        let caster_kind = match caster {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        if raised.is_none()
            && (caster_kind == UnitKind::Avatar
                || self.position.units.iter().any(|candidate| {
                    candidate.controller == seat && candidate.card.instance_id == caster_instance_id
                }))
        {
            self.record_unit_interaction(caster_kind, seat, &caster_instance_id, outcomes)?;
        }
        let cell = unit.location;
        let cells = unit.occupied_cells;
        let lance_count = unit.carried_lance_count;
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(card_id.0)].facts else {
            return Err(GameError::IllegalAction);
        };
        let genesis = facts.genesis;
        self.position.units.push(unit);
        outcomes.push("minion-summoned", || {
            let mut payload = json!({
                "cardId": self.rules.cards[usize::from(card_id.0)].id,
                "casterInstanceId": caster_instance_id,
                "cell": cell,
                "instanceId": card_instance_id,
                "manaPaid": mana_paid,
                "seat": seat,
            });
            if let Some(cells) = cells {
                payload["cells"] = json!(cells);
            }
            if let Some(raised) = &raised {
                payload["owner"] = json!(card_owner);
                payload["sourceInstanceId"] = json!(raised.source_magic_instance_id);
            }
            payload
        });
        if lance_count > 0 {
            outcomes.push("lance-gained", || {
                json!({
                    "bearerInstanceId": card_instance_id,
                    "count": lance_count,
                    "sourceInstanceId": card_instance_id,
                })
            });
        }
        self.apply_minion_genesis(
            seat,
            &card_instance_id,
            genesis,
            genesis_damage_choice,
            genesis_damage_target.as_ref(),
            outcomes,
        )?;
        if let Some(raised) = raised {
            let source_card_id = self.rules.cards[usize::from(raised.source_magic_card_id.0)]
                .id
                .clone();
            if let Some(pending) = &mut self.position.pending_deathrites {
                pending.deferred_magic_resolved = Some(DeferredMagicResolved {
                    card_id: raised.source_magic_card_id,
                    instance_id: raised.source_magic_instance_id,
                    owner: raised.source_magic_owner,
                });
            } else {
                Self::emit_continuation_magic_resolved(
                    &source_card_id,
                    &raised.source_magic_instance_id,
                    raised.source_magic_owner,
                    outcomes,
                );
            }
        }
        Ok(())
    }

    /// Names the caster a summon event credits.
    ///
    /// A raised minion only labels its caster, so the reference survives a spellcaster that left
    /// the realm between the cast and the placement it owed.
    fn recorded_caster(&self, seat: Seat, caster_instance_id: &IdentityHash) -> UnitTarget {
        if self.position.players[seat_index(seat)]
            .avatar
            .card
            .instance_id
            == *caster_instance_id
        {
            UnitTarget::Avatar {
                instance_id: caster_instance_id.clone(),
                seat,
            }
        } else {
            UnitTarget::Minion {
                instance_id: caster_instance_id.clone(),
                seat,
            }
        }
    }

    /// Applies the free placement a cemetery Magic owes after its public random selection.
    fn apply_cemetery_summon_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::SummonMinion {
            card_id,
            card_instance_id,
            caster_instance_id,
            cell,
            cells,
            genesis_damage_choice,
            genesis_damage_target,
            mana_cost,
            payment_mode,
            sacrificed_minion_instance_ids,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        let pending = self
            .position
            .pending_cemetery_summon
            .clone()
            .ok_or(GameError::IllegalAction)?;
        if pending.seat != seat
            || pending.card_instance_id != *card_instance_id
            || pending.caster_instance_id != *caster_instance_id
            || *mana_cost != 0
            || payment_mode.is_some()
            || sacrificed_minion_instance_ids.is_some()
        {
            return Err(GameError::IllegalAction);
        }
        let owner_index = seat_index(pending.card_owner);
        let cemetery_index = self.position.players[owner_index]
            .cemetery
            .iter()
            .position(|card| {
                card.instance_id == *card_instance_id
                    && self.rules.cards[usize::from(card.card_id.0)].id == card_id.as_str()
            })
            .ok_or(GameError::IllegalAction)?;
        let compact_card_id = self.position.players[owner_index].cemetery[cemetery_index].card_id;
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(compact_card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let genesis = facts.genesis;
        let lance_count = facts.lance_count.unwrap_or(0);
        let starts_stealthed = facts.stealth;
        let starts_warded = facts.damage_prevention == Some(DamagePrevention::Ward);
        let placeable = self
            .free_summon_destinations(facts)
            .into_iter()
            .any(|destination| destination.cell == *cell && destination.cells == *cells);
        if !placeable
            || !self.valid_genesis_damage_choice(
                seat,
                card_instance_id,
                *cell,
                genesis,
                *genesis_damage_choice,
                genesis_damage_target.as_ref(),
            )
        {
            return Err(GameError::IllegalAction);
        }
        let card = self.position.players[owner_index]
            .cemetery
            .remove(cemetery_index);
        let unit = UnitPosition {
            card,
            carried_lance_count: lance_count,
            controller: seat,
            damage: 0,
            disable_effects: Vec::new(),
            disabled_until_damaged: false,
            last_dropped_artifacts_turn: None,
            last_interacted_turn: None,
            last_picked_up_artifacts_turn: None,
            location: *cell,
            occupied_cells: *cells,
            region: Region::Surface,
            stealthed: starts_stealthed,
            summoning_sickness: true,
            tapped: false,
            temporary_charge_sources: Vec::new(),
            temporary_power_sources: Vec::new(),
            warded: starts_warded,
        };
        let continuation = PaidSummonContinuation {
            caster: self.recorded_caster(seat, caster_instance_id),
            genesis_damage_choice: *genesis_damage_choice,
            genesis_damage_target: genesis_damage_target.clone(),
            mana_paid: 0,
            sacrificed_minion_instance_ids: Vec::new(),
            unit,
        };
        self.position.pending_cemetery_summon = None;
        self.position.phase = Phase::Main;
        self.position.state_version += 1;
        self.finish_summon(continuation, Some(pending), outcomes)
    }

    fn apply_minion_genesis(
        &mut self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        genesis: Option<MinionGenesis>,
        genesis_damage_choice: Option<GenesisDamageChoice>,
        genesis_damage_target: Option<&UnitTarget>,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        match genesis {
            None => {}
            Some(MinionGenesis::DrawSite) => {
                self.apply_genesis_draws(seat, source_instance_id, DeckZone::Atlas, 1, outcomes);
            }
            Some(MinionGenesis::DrawSpells(count)) => {
                self.apply_genesis_draws(
                    seat,
                    source_instance_id,
                    DeckZone::Spellbook,
                    count,
                    outcomes,
                );
            }
            Some(MinionGenesis::HealControllerTwo) => {
                self.heal_avatar(seat, 2, source_instance_id, outcomes)?;
            }
            Some(MinionGenesis::LoseControllerLifeTwo) => {
                let player = &mut self.position.players[seat_index(seat)];
                let old_life = player.avatar.life;
                player.avatar.life = old_life.saturating_sub(2);
                let amount = old_life - player.avatar.life;
                if amount > 0 {
                    let life = player.avatar.life;
                    outcomes.push("avatar-life-lost", || {
                        json!({
                            "amount": amount,
                            "life": life,
                            "seat": seat,
                            "sourceInstanceId": source_instance_id,
                        })
                    });
                    if life == 0 {
                        player.avatar.death_door_turn = Some(self.position.turn_number);
                        let turn_number = self.position.turn_number;
                        outcomes.push("avatar-reached-deaths-door", || {
                            json!({
                                "seat": seat,
                                "sourceInstanceId": source_instance_id,
                                "turnNumber": turn_number,
                            })
                        });
                    }
                }
            }
            Some(MinionGenesis::DisableSelfUntilDamaged) => {
                let unit = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| {
                        unit.card.instance_id == *source_instance_id && unit.controller == seat
                    })
                    .ok_or(GameError::IllegalAction)?;
                unit.disabled_until_damaged = true;
                outcomes.push("minion-disabled", || {
                    json!({
                        "instanceId": source_instance_id,
                        "seat": seat,
                        "sourceInstanceId": source_instance_id,
                    })
                });
            }
            Some(MinionGenesis::MayDamageTargetAdjacentUnitTwo) => {
                match (genesis_damage_choice, genesis_damage_target) {
                    (Some(GenesisDamageChoice::Decline), None) => {}
                    (Some(GenesisDamageChoice::Target), Some(target)) => {
                        self.apply_targeted_genesis_damage(source_instance_id, target, outcomes)?;
                    }
                    _ => return Err(GameError::IllegalAction),
                }
            }
            Some(MinionGenesis::DamageEachOtherUnitHereOne) => {
                self.apply_genesis_here_damage(source_instance_id, false, outcomes)?;
            }
            Some(MinionGenesis::StrikeEachEnemyHere) => {
                self.apply_genesis_here_damage(source_instance_id, true, outcomes)?;
            }
        }
        Ok(())
    }

    /// Other units standing anywhere under `source`, in canonical identity order.
    fn units_sharing_footprint(
        &self,
        source: &UnitPosition,
        enemies_only: bool,
    ) -> Vec<(IdentityHash, UnitKind, Seat)> {
        let seats: &[Seat] = if enemies_only {
            &[other_seat(source.controller)]
        } else {
            &[Seat::North, Seat::South]
        };
        let mut targets = Vec::new();
        for seat in seats.iter().copied() {
            let avatar = &self.position.players[seat_index(seat)].avatar;
            if source.region == Region::Surface && Self::unit_occupies_cell(source, avatar.location)
            {
                targets.push((avatar.card.instance_id.clone(), UnitKind::Avatar, seat));
            }
        }
        targets.extend(
            self.position
                .units
                .iter()
                .filter(|unit| {
                    seats.contains(&unit.controller)
                        && unit.region == source.region
                        && unit.card.instance_id != source.card.instance_id
                        && Self::unit_occupied_cells(unit)
                            .iter()
                            .any(|cell| Self::unit_occupies_cell(source, *cell))
                })
                .map(|unit| {
                    (
                        unit.card.instance_id.clone(),
                        UnitKind::Minion,
                        unit.controller,
                    )
                }),
        );
        targets.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        targets
    }

    /// Genesis damage that hits everything sharing the newcomer's own location.
    ///
    /// A strike Genesis only reaches enemies and uses the newcomer's current power, while plain
    /// area damage reaches both seats for a flat point.
    fn apply_genesis_here_damage(
        &mut self,
        source_instance_id: &IdentityHash,
        strike: bool,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let source = self
            .position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == *source_instance_id)
            .cloned()
            .ok_or(GameError::IllegalAction)?;
        if self.minion_is_disabled(&source) {
            return Ok(());
        }
        let (current_power, lethal) = self.combatant_attack_and_lethal(
            UnitKind::Minion,
            source.controller,
            source_instance_id,
        )?;
        let amount = if strike { current_power } else { 1 };
        let targets = self
            .units_sharing_footprint(&source, strike)
            .into_iter()
            .map(|(instance_id, kind, seat)| {
                let status = if kind == UnitKind::Minion {
                    Some(self.minion_damage_status(&instance_id)?)
                } else {
                    None
                };
                Ok((instance_id, kind, seat, status))
            })
            .collect::<Result<Vec<_>, GameError>>()?;
        for (target_instance_id, _, _, _) in &targets {
            outcomes.push(
                if strike {
                    "strike-damage-allocated"
                } else {
                    "genesis-damage-allocated"
                },
                || {
                    let mut payload = json!({
                        "amount": amount,
                        "targetInstanceId": target_instance_id,
                    });
                    payload[if strike {
                        "strikerInstanceId"
                    } else {
                        "sourceInstanceId"
                    }] = json!(source_instance_id);
                    payload
                },
            );
        }
        let mut dead_minions = Vec::new();
        let mut defeated_avatars = Vec::new();
        for (target_instance_id, kind, seat, status) in targets {
            let result = self.apply_simple_damage_with_status(
                kind,
                seat,
                &target_instance_id,
                amount,
                UnitDamageSource {
                    current_power,
                    lethal,
                },
                status,
                outcomes,
            )?;
            if result.minion_died {
                dead_minions.push(target_instance_id);
            }
            if result.avatar_defeated && !defeated_avatars.contains(&seat) {
                defeated_avatars.push(seat);
            }
        }
        if !dead_minions.is_empty() || !defeated_avatars.is_empty() {
            self.begin_minion_deaths(
                &dead_minions,
                &defeated_avatars,
                Phase::Main,
                self.position.active_seat,
                outcomes,
            )?;
        }
        Ok(())
    }

    fn apply_targeted_genesis_damage(
        &mut self,
        source_instance_id: &IdentityHash,
        target: &UnitTarget,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let (_, attack, _, _) = self.simple_minion_combatant(source_instance_id)?;
        let target_instance_id = target.instance_id().clone();
        outcomes.push("genesis-damage-allocated", || {
            json!({
                "amount": 2,
                "sourceInstanceId": source_instance_id,
                "targetInstanceId": target_instance_id,
            })
        });
        let target_kind = match target {
            UnitTarget::Avatar { .. } => UnitKind::Avatar,
            UnitTarget::Minion { .. } => UnitKind::Minion,
        };
        let damage = self.apply_simple_damage(
            target_kind,
            target.seat(),
            &target_instance_id,
            2,
            UnitDamageSource {
                current_power: attack,
                lethal: false,
            },
            outcomes,
        )?;
        if damage.minion_died || damage.avatar_defeated {
            let defeated_seat = target.seat();
            self.begin_minion_deaths(
                if damage.minion_died {
                    std::slice::from_ref(&target_instance_id)
                } else {
                    &[]
                },
                if damage.avatar_defeated {
                    std::slice::from_ref(&defeated_seat)
                } else {
                    &[]
                },
                Phase::Main,
                self.position.active_seat,
                outcomes,
            )?;
        }
        Ok(())
    }

    fn heal_avatar(
        &mut self,
        seat: Seat,
        attempted_amount: u16,
        source_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let player = &mut self.position.players[seat_index(seat)];
        let CardFacts::Avatar(avatar_facts) =
            self.rules.cards[usize::from(player.avatar.card.card_id.0)].facts
        else {
            return Err(GameError::IllegalAction);
        };
        let old_life = player.avatar.life;
        if old_life > 0 {
            player.avatar.life = old_life
                .saturating_add(attempted_amount)
                .min(u16::from(avatar_facts.life));
        }
        let amount = player.avatar.life - old_life;
        if amount > 0 {
            let life = player.avatar.life;
            outcomes.push("avatar-healed", || {
                json!({
                    "amount": amount,
                    "attemptedAmount": attempted_amount,
                    "life": life,
                    "seat": seat,
                    "sourceInstanceId": source_instance_id,
                })
            });
        }
        Ok(())
    }

    fn apply_genesis_draws(
        &mut self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        zone: DeckZone,
        count: u8,
        outcomes: &mut OutcomeLog<'_>,
    ) {
        for _ in 0..count {
            if !self.draw_private_card(seat, zone) {
                self.finish_deck_empty(seat, outcomes);
                break;
            }
            outcomes.push(
                if zone == DeckZone::Atlas {
                    "site-drawn"
                } else {
                    "spell-drawn"
                },
                || json!({ "seat": seat, "sourceInstanceId": source_instance_id }),
            );
        }
    }

    fn apply_end_turn_action(
        &mut self,
        seat: Seat,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if self.position.phase != Phase::Main
            || seat != self.position.active_seat
            || !self.position.players[seat_index(seat)].domain_established
        {
            return Err(GameError::IllegalAction);
        }
        self.resolve_end_of_each_turn_site_controller_life_loss(seat, outcomes)?;
        let triggered: Vec<_> = self
            .position
            .units
            .iter()
            .filter_map(|unit| {
                if unit.controller != seat || self.minion_is_disabled(unit) {
                    return None;
                }
                matches!(
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts,
                    CardFacts::Minion(facts) if facts.dies_at_end_of_controller_turn
                )
                .then(|| unit.card.instance_id.clone())
            })
            .collect();
        self.continue_end_turn_deaths(seat, &triggered, outcomes)?;
        self.position.state_version += 1;
        Ok(())
    }

    /// Charges each Artifact's current site controller the life the Artifact's card names, in the
    /// order the ending turn gives the two seats.
    ///
    /// The Artifact is the source, so a carried Artifact charges the site its bearer stands on and
    /// keeps charging while that bearer is Disabled. An Artifact standing over no site — lying on
    /// Rubble, or beside the realm in the Void — finds no controller to charge. The charge is a
    /// life loss rather than damage, so an Avatar already on Death's Door only records the trigger.
    fn resolve_end_of_each_turn_site_controller_life_loss(
        &mut self,
        active_seat: Seat,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let non_active_seat = other_seat(active_seat);
        let mut triggered = Vec::new();
        for artifact in &self.position.artifacts {
            let ArtifactEffect::AtEndOfEachTurnSiteControllerLosesLife(amount) =
                self.artifact_facts(artifact)?.effect
            else {
                continue;
            };
            let ordering_seat = artifact
                .bearer()
                .map_or(artifact.card.owner, UnitTarget::seat);
            triggered.push((
                ordering_seat != non_active_seat,
                artifact.card.instance_id.clone(),
                u16::from(amount),
            ));
        }
        triggered.sort_unstable();
        for (_, instance_id, amount) in triggered {
            let Some(artifact) = self
                .position
                .artifacts
                .iter()
                .find(|candidate| candidate.card.instance_id == instance_id)
            else {
                continue;
            };
            let cell = self.artifact_location(artifact)?.cell;
            let Some(site) = self.position.sites[cell.index()].as_ref() else {
                continue;
            };
            let seat = site.controller;
            let site_instance_id = site.card.instance_id.clone();
            let avatar = &mut self.position.players[seat_index(seat)].avatar;
            let old_life = avatar.life;
            avatar.life = old_life.saturating_sub(amount);
            let lost = old_life - avatar.life;
            let reached_deaths_door = old_life > 0 && avatar.life == 0;
            if reached_deaths_door {
                avatar.death_door_turn = Some(self.position.turn_number);
            }
            let life = avatar.life;
            outcomes.push("end-turn-site-life-loss-triggered", || {
                json!({
                    "amount": amount,
                    "seat": seat,
                    "siteInstanceId": site_instance_id,
                    "sourceInstanceId": instance_id,
                })
            });
            if lost > 0 {
                outcomes.push("avatar-life-lost", || {
                    json!({
                        "amount": lost,
                        "life": life,
                        "seat": seat,
                        "sourceInstanceId": instance_id,
                    })
                });
            }
            if reached_deaths_door {
                let turn_number = self.position.turn_number;
                outcomes.push("avatar-reached-deaths-door", || {
                    json!({
                        "seat": seat,
                        "sourceInstanceId": instance_id,
                        "turnNumber": turn_number,
                    })
                });
            }
        }
        Ok(())
    }

    fn continue_end_turn_deaths(
        &mut self,
        seat: Seat,
        remaining_instance_ids: &[IdentityHash],
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        for (index, instance_id) in remaining_instance_ids.iter().enumerate() {
            if !self
                .position
                .units
                .iter()
                .any(|unit| unit.card.instance_id == *instance_id)
            {
                continue;
            }
            return self.begin_minion_deaths_with_continuation(
                std::slice::from_ref(instance_id),
                &[],
                Phase::Main,
                seat,
                Some(DeathriteContinuation::EndTurn(EndTurnContinuation {
                    remaining_instance_ids: remaining_instance_ids[index + 1..].to_vec(),
                    seat,
                })),
                outcomes,
            );
        }
        self.finish_end_turn_cleanup(seat, outcomes)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one cleanup transaction preserves exact end-phase state and event ordering"
    )]
    fn finish_end_turn_cleanup(
        &mut self,
        seat: Seat,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let next_seat = other_seat(seat);
        let next_mana = self
            .position
            .sites
            .iter()
            .flatten()
            .filter(|site| site.controller == next_seat)
            .count();
        let next_mana = u16::try_from(next_mana).map_err(|_| GameError::IllegalAction)?;
        for player in &mut self.position.players {
            if player.air_thresholds_cast_this_turn.is_some() {
                player.air_thresholds_cast_this_turn = Some(0);
            }
        }
        self.position.players[seat_index(seat)].mana = 0;
        let next_player = &mut self.position.players[seat_index(next_seat)];
        next_player.avatar.tapped = false;
        next_player.mana = next_mana;
        let disabled_units: Vec<_> = self
            .position
            .units
            .iter()
            .map(|unit| self.minion_is_disabled(unit))
            .collect();
        let end_turn_stealth_gained: Vec<_> = self
            .position
            .units
            .iter()
            .zip(&disabled_units)
            .map(|(unit, disabled)| {
                if unit.controller != seat || *disabled || unit.stealthed {
                    return false;
                }
                let CardFacts::Minion(facts) =
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts
                else {
                    return false;
                };
                match facts.end_turn_stealth {
                    Some(EndTurnStealth::Always) => true,
                    Some(EndTurnStealth::IfNoEnemiesNearby) => !self.has_nearby_enemy_minion(unit),
                    None => false,
                }
            })
            .collect();
        let end_phase_untapped: Vec<_> = self
            .position
            .units
            .iter()
            .zip(&disabled_units)
            .filter_map(|(unit, disabled)| {
                if unit.controller != seat || !unit.tapped || *disabled {
                    return None;
                }
                matches!(
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts,
                    CardFacts::Minion(facts) if facts.untaps_at_end_of_controller_turn
                )
                .then(|| (unit.card.instance_id.clone(), unit.controller))
            })
            .collect();
        let expired_disable_effects: Vec<_> = self
            .position
            .units
            .iter()
            .flat_map(|unit| {
                unit.disable_effects
                    .iter()
                    .filter(|effect| effect.expires_at_seat == next_seat)
                    .map(|effect| {
                        (
                            unit.card.instance_id.clone(),
                            unit.controller,
                            effect.source_instance_id.clone(),
                        )
                    })
            })
            .collect();
        let expired_charge_sources: Vec<_> = self
            .position
            .units
            .iter()
            .flat_map(|unit| {
                unit.temporary_charge_sources.iter().map(|source| {
                    (
                        unit.card.instance_id.clone(),
                        unit.controller,
                        source.clone(),
                    )
                })
            })
            .collect();
        let ending_avatar = &self.position.players[seat_index(seat)].avatar;
        let mut expired_power_sources: Vec<_> = ending_avatar
            .temporary_power_sources
            .iter()
            .map(|source| (ending_avatar.card.instance_id.clone(), seat, source.clone()))
            .collect();
        expired_power_sources.extend(self.position.units.iter().flat_map(|unit| {
            unit.temporary_power_sources.iter().map(|source| {
                (
                    unit.card.instance_id.clone(),
                    unit.controller,
                    source.clone(),
                )
            })
        }));
        self.position.players[seat_index(seat)]
            .avatar
            .temporary_power_sources
            .clear();
        for (instance_id, controller) in &end_phase_untapped {
            let unit = self
                .position
                .units
                .iter_mut()
                .find(|unit| unit.card.instance_id == *instance_id)
                .ok_or(GameError::IllegalAction)?;
            unit.tapped = false;
            outcomes.push("minion-untapped", || {
                json!({
                    "instanceId": instance_id,
                    "seat": controller,
                    "sourceInstanceId": instance_id,
                })
            });
        }
        for (unit, gains_stealth) in self.position.units.iter_mut().zip(end_turn_stealth_gained) {
            unit.damage = 0;
            unit.temporary_charge_sources.clear();
            unit.temporary_power_sources.clear();
            if unit.controller == seat {
                if gains_stealth {
                    unit.stealthed = true;
                    let instance_id = unit.card.instance_id.clone();
                    outcomes.push(
                        "stealth-gained",
                        || json!({ "instanceId": instance_id, "seat": seat }),
                    );
                }
                unit.summoning_sickness = false;
            } else if unit.controller == next_seat {
                unit.tapped = false;
            }
        }
        self.settle_nearby_enemy_stealth(outcomes);
        for unit in &mut self.position.units {
            unit.disable_effects
                .retain(|effect| effect.expires_at_seat != next_seat);
        }
        self.position
            .immobile_areas
            .retain(|area| area.expires_at_seat != next_seat);
        let ended_turn = self.position.turn_number;
        self.position.turn_number += 1;
        self.position.active_seat = next_seat;
        self.position.decision_seat = next_seat;
        self.position.phase = Phase::Draw;
        for (instance_id, controller, source_instance_id) in expired_charge_sources {
            outcomes.push("charge-expired", || {
                json!({
                    "instanceId": instance_id,
                    "seat": controller,
                    "sourceInstanceId": source_instance_id,
                })
            });
        }
        for (instance_id, controller, source_instance_id) in expired_power_sources {
            outcomes.push("power-expired", || {
                json!({
                    "amount": 2,
                    "instanceId": instance_id,
                    "seat": controller,
                    "sourceInstanceId": source_instance_id,
                })
            });
        }
        outcomes.push(
            "turn-ended",
            || json!({ "seat": seat, "turnNumber": ended_turn }),
        );
        for (instance_id, controller, source_instance_id) in expired_disable_effects {
            outcomes.push("minion-disable-expired", || {
                json!({
                    "instanceId": instance_id,
                    "seat": controller,
                    "sourceInstanceId": source_instance_id,
                })
            });
        }
        self.settle_nearby_enemy_stealth(outcomes);
        let turn_number = self.position.turn_number;
        outcomes.push("turn-started", || {
            json!({
                "drawSkipped": false,
                "seat": next_seat,
                "turnNumber": turn_number,
            })
        });
        Ok(())
    }

    /// Materializes the authoritative JSON state used for receipts and replay.
    #[must_use]
    pub fn authoritative_state(&self) -> Value {
        let cards: Map<_, _> = self
            .rules
            .cards
            .iter()
            .map(|card| (card.id.clone(), card.value.clone()))
            .collect();
        let sites: Map<_, _> = Cell::ALL
            .into_iter()
            .filter_map(|cell| {
                self.position.sites[cell.index()].as_ref().map_or_else(
                    || {
                        self.position.rubble[cell.index()]
                            .as_ref()
                            .map(|instance_id| {
                                (
                                    cell.to_string(),
                                    json!({
                                        "controller": Value::Null,
                                        "instanceId": instance_id,
                                        "rubble": true,
                                    }),
                                )
                            })
                    },
                    |site| Some((cell.to_string(), self.site_value(site))),
                )
            })
            .collect();
        let units: Vec<_> = self
            .position
            .units
            .iter()
            .map(|unit| self.unit_value(unit))
            .collect();
        let pending_combat = self
            .position
            .pending_combat
            .as_ref()
            .map_or(Value::Null, Self::pending_combat_value);
        let mut value = json!({
            "activeSeat": self.position.active_seat,
            "cards": cards,
            "decisionSeat": self.position.decision_seat,
            "engine": {
                "prng": self.position.prng,
                "schemaVersion": 1,
                "stateVersion": self.position.state_version,
            },
            "pendingCombat": pending_combat,
            "phase": self.position.phase.as_str(),
            "players": {
                "north": self.player_value(&self.position.players[0]),
                "south": self.player_value(&self.position.players[1]),
            },
            "realm": { "sites": sites, "units": units },
            "schemaVersion": 1,
            "stateVersion": self.position.state_version,
            "terminal": self.terminal_value(),
            "turnNumber": self.position.turn_number,
        });
        if let Value::Object(object) = &mut value {
            self.insert_pending_genesis_state(object);
            if let Some(pending) = &self.position.pending_deathrites {
                object.insert(
                    "pendingDeathrites".to_owned(),
                    self.pending_deathrites_value(pending),
                );
            }
            self.insert_pending_movement_state(object);
            self.insert_pending_chain_magic_state(object);
            if let Some(pending) = &self.position.pending_cemetery_summon {
                object.insert(
                    "pendingCemeterySummon".to_owned(),
                    json!({
                        "cardInstanceId": pending.card_instance_id,
                        "cardOwner": pending.card_owner,
                        "casterInstanceId": pending.caster_instance_id,
                        "seat": pending.seat,
                        "sourceMagicInstanceId": pending.source_magic_instance_id,
                    }),
                );
            }
        }
        self.insert_realm_artifacts(&mut value);
        if !self.position.immobile_areas.is_empty() {
            value["realm"]["immobileAreas"] = self
                .position
                .immobile_areas
                .iter()
                .map(|area| {
                    json!({
                        "cells": area.cells,
                        "expiresAtSeat": area.expires_at_seat,
                        "sourceInstanceId": area.source_instance_id,
                    })
                })
                .collect();
        }
        value
    }

    fn insert_pending_chain_magic_state(&self, object: &mut Map<String, Value>) {
        match &self.position.pending_chain_magic {
            PendingField::Absent => {}
            PendingField::Pending(pending) => {
                let card_id = &self.rules.cards[usize::from(pending.card_id.0)].id;
                object.insert(
                    "pendingChainMagic".to_owned(),
                    json!({
                        "cardId": card_id,
                        "cardInstanceId": pending.card_instance_id,
                        "casterInstanceId": pending.caster_instance_id,
                        "seat": pending.seat,
                        "targets": pending.targets,
                    }),
                );
            }
            PendingField::Resolved => {
                object.insert("pendingChainMagic".to_owned(), Value::Null);
            }
        }
    }

    fn insert_pending_movement_state(&self, object: &mut Map<String, Value>) {
        match &self.position.pending_basic_movement {
            PendingField::Absent => {}
            PendingField::Pending(pending) => {
                object.insert(
                    "pendingBasicMovement".to_owned(),
                    json!({
                        "path": pending.path,
                        "pathIndex": pending.path_index,
                        "purpose": pending.purpose.as_str(),
                        "rangedStrikeUsed": pending.ranged_strike_used,
                        "seat": pending.seat,
                        "sourceInstanceId": pending.source_instance_id,
                    }),
                );
            }
            PendingField::Resolved => {
                object.insert("pendingBasicMovement".to_owned(), Value::Null);
            }
        }
        match &self.position.pending_ranged_step {
            PendingField::Absent => {}
            PendingField::Pending(pending) => {
                object.insert(
                    "pendingRangedStep".to_owned(),
                    json!({
                        "seat": pending.seat,
                        "sourceInstanceId": pending.source_instance_id,
                    }),
                );
            }
            PendingField::Resolved => {
                object.insert("pendingRangedStep".to_owned(), Value::Null);
            }
        }
    }

    fn pending_deathrites_value(&self, pending: &PendingDeathrites) -> Value {
        let source_value = |source: &PendingDeathriteSource| {
            json!({
                "controller": source.controller,
                "currentPower": source.current_power,
                "instanceId": source.instance_id,
                "lethal": source.lethal,
                "unit": self.unit_value(&source.unit),
            })
        };
        let sources_json = |sources: &[PendingDeathriteSource]| {
            Value::Array(sources.iter().map(&source_value).collect())
        };
        let batches = pending
            .batches
            .iter()
            .map(|batch| {
                json!({
                    "activeOrder": sources_json(&batch.active_order),
                    "activeRemaining": sources_json(&batch.active_remaining),
                    "nonActiveOrder": sources_json(&batch.non_active_order),
                    "nonActiveRemaining": sources_json(&batch.non_active_remaining),
                    "resolving": sources_json(&batch.resolving),
                    "stage": match batch.stage {
                        DeathriteStage::ActiveOrder => "active-order",
                        DeathriteStage::NonActiveOrder => "non-active-order",
                        DeathriteStage::Resolve => "resolve",
                    },
                })
            })
            .collect::<Vec<_>>();
        let mut value = json!({
            "batches": batches,
            "corpses": pending
                .corpses
                .iter()
                .map(|corpse| self.unit_value(corpse))
                .collect::<Vec<_>>(),
            "deckLosers": pending.deck_losers,
            "defeatedAvatars": pending.defeated_avatars,
            "returnDecisionSeat": pending.return_decision_seat,
            "returnPhase": pending.return_phase.as_str(),
        });
        if let (Value::Object(object), Some(deferred)) =
            (&mut value, &pending.deferred_magic_resolved)
        {
            let card_id = &self.rules.cards[usize::from(deferred.card_id.0)].id;
            object.insert(
                "deferredOutcomes".to_owned(),
                json!([{
                    "payload": {
                        "cardId": card_id,
                        "instanceId": deferred.instance_id,
                        "owner": deferred.owner,
                    },
                    "type": "magic-resolved",
                }]),
            );
        }
        if let (Value::Object(object), Some(continuation)) = (&mut value, &pending.continuation) {
            object.insert(
                "continuation".to_owned(),
                self.deathrite_continuation_value(continuation),
            );
        }
        value
    }

    fn deathrite_continuation_value(&self, continuation: &DeathriteContinuation) -> Value {
        match continuation {
            DeathriteContinuation::Blink(continuation) => json!({
                "cardId": continuation.card_id,
                "instanceId": continuation.instance_id,
                "kind": "blink",
                "owner": continuation.owner,
                "seat": continuation.seat,
                "zone": continuation.zone.as_str(),
            }),
            DeathriteContinuation::DragProjectile(continuation) => json!({
                "fightOnArrival": continuation.fight_on_arrival,
                "kind": "drag-projectile",
                "path": continuation.path,
                "pathIndex": continuation.path_index,
                "shooter": continuation.shooter,
                "target": continuation.target,
            }),
            DeathriteContinuation::EndTurn(continuation) => json!({
                "kind": "end-turn",
                "remainingInstanceIds": continuation.remaining_instance_ids,
                "seat": continuation.seat,
            }),
            DeathriteContinuation::FirstStrike(continuation) => json!({
                "attackerStrikesFirst": continuation.attacker_struck,
                "firstCombatantInstanceIds": continuation.first_combatant_instance_ids,
                "kind": "first-strike",
                "pending": Self::pending_combat_value(&continuation.pending),
            }),
            DeathriteContinuation::LeapAttack(continuation) => json!({
                "ally": continuation.ally,
                "cardId": continuation.card_id,
                "instanceId": continuation.instance_id,
                "kind": "leap-attack",
                "owner": continuation.owner,
                "strikeLocation": continuation.strike_location,
            }),
            DeathriteContinuation::PaidSummon(continuation) => {
                self.paid_summon_continuation_value(continuation)
            }
            DeathriteContinuation::SiteGenesis(continuation) => {
                self.site_genesis_continuation_value(continuation)
            }
        }
    }

    fn paid_summon_continuation_value(&self, continuation: &PaidSummonContinuation) -> Value {
        let card_id = &self.rules.cards[usize::from(continuation.unit.card.card_id.0)].id;
        let mut descriptor = json!({
            "cardId": card_id,
            "cardInstanceId": continuation.unit.card.instance_id,
            "casterInstanceId": continuation.caster.instance_id(),
            "cell": continuation.unit.location,
            "kind": "summon-minion",
            "manaCost": continuation.mana_paid,
            "sacrificedMinionInstanceIds": continuation.sacrificed_minion_instance_ids,
        });
        let Value::Object(descriptor) = &mut descriptor else {
            unreachable!("summon descriptor is an object");
        };
        if let Some(cells) = continuation.unit.occupied_cells {
            descriptor.insert("cells".to_owned(), json!(cells));
        }
        if let Some(choice) = continuation.genesis_damage_choice {
            descriptor.insert("genesisDamageChoice".to_owned(), json!(choice));
        }
        if let Some(target) = &continuation.genesis_damage_target {
            descriptor.insert("genesisDamageTarget".to_owned(), json!(target));
        }
        json!({
            "caster": continuation.caster,
            "descriptor": descriptor,
            "kind": "paid-summon",
            "unit": self.unit_value(&continuation.unit),
        })
    }

    fn site_genesis_continuation_value(&self, continuation: &SiteGenesisContinuation) -> Value {
        let card_id = &self.rules.cards[usize::from(continuation.card_id.0)].id;
        let mut descriptor = json!({
            "cardId": card_id,
            "cardInstanceId": continuation.card_instance_id,
            "cell": continuation.cell,
            "kind": "play-site",
        });
        let Value::Object(descriptor) = &mut descriptor else {
            unreachable!("site descriptor is an object");
        };
        if let Some(cell) = continuation.create_rubble_at {
            descriptor.insert("createRubbleAt".to_owned(), json!(cell));
        }
        if continuation.from_top_atlas {
            descriptor.insert("fromTopAtlas".to_owned(), json!(true));
        }
        if continuation.defer_token {
            descriptor.insert("genesisTokenChoice".to_owned(), json!("defer"));
        } else if let Some(choice) = continuation.genesis_token_choice {
            descriptor.insert("genesisTokenChoice".to_owned(), json!(choice));
        }
        json!({
            "descriptor": descriptor,
            "genesisGainMana": continuation.genesis_gain_mana.unwrap_or(0),
            "genesisSpellDrawCount": continuation.genesis_spell_draw_count,
            "kind": "site-genesis",
            "originStateVersion": continuation.origin_state_version,
            "seat": continuation.seat,
        })
    }

    fn insert_pending_genesis_state(&self, object: &mut Map<String, Value>) {
        match &self.position.pending_genesis_spell {
            PendingField::Absent => {}
            PendingField::Pending(pending) => {
                object.insert(
                    "pendingGenesisSpell".to_owned(),
                    json!({
                        "seat": pending.seat,
                        "sourceInstanceId": pending.source_instance_id,
                    }),
                );
            }
            PendingField::Resolved => {
                object.insert("pendingGenesisSpell".to_owned(), Value::Null);
            }
        }
        match &self.position.pending_genesis_spell_order {
            PendingField::Absent => {}
            PendingField::Pending(pending) => {
                object.insert(
                    "pendingGenesisSpellOrder".to_owned(),
                    json!({
                        "count": pending.count,
                        "seat": pending.seat,
                        "sourceInstanceId": pending.source_instance_id,
                    }),
                );
            }
            PendingField::Resolved => {
                object.insert("pendingGenesisSpellOrder".to_owned(), Value::Null);
            }
        }
        match &self.position.pending_genesis_token {
            PendingField::Absent => {}
            PendingField::Pending(pending) => {
                object.insert(
                    "pendingGenesisToken".to_owned(),
                    json!({
                        "cell": pending.cell,
                        "seat": pending.seat,
                        "sourceInstanceId": pending.source_instance_id,
                    }),
                );
            }
            PendingField::Resolved => {
                object.insert("pendingGenesisToken".to_owned(), Value::Null);
            }
        }
    }

    /// Hashes the materialized authoritative state.
    ///
    /// # Errors
    ///
    /// Returns [`CanonicalError`] if the boundary state cannot be canonicalized.
    pub fn state_hash(&self) -> Result<IdentityHash, CanonicalError> {
        identity_hash(&self.authoritative_state())
    }

    fn player_value(&self, player: &PlayerPosition) -> Value {
        let mut value = json!({
            "atlas": self.cards_value(&player.atlas),
            "avatar": {
                "card": self.card_value(&player.avatar.card),
                "deathDoorTurn": player.avatar.death_door_turn,
                "life": player.avatar.life,
                "location": player.avatar.location,
                "region": "surface",
                "tapped": player.avatar.tapped,
            },
            "cemetery": self.cards_value(&player.cemetery),
            "domainEstablished": player.domain_established,
            "hand": {
                "atlas": self.cards_value(&player.hand_atlas),
                "spellbook": self.cards_value(&player.hand_spellbook),
            },
            "mana": player.mana,
            "mulliganComplete": player.mulligan_complete,
            "spellbook": self.cards_value(&player.spellbook),
        });
        if let (Some(count), Value::Object(object)) =
            (player.air_thresholds_cast_this_turn, &mut value)
        {
            object.insert("airThresholdsCastThisTurn".to_owned(), json!(count));
        }
        if let Some(avatar) = value.get_mut("avatar").and_then(Value::as_object_mut) {
            for (key, turn) in [
                (
                    "lastDroppedArtifactsTurn",
                    player.avatar.last_dropped_artifacts_turn,
                ),
                ("lastInteractedTurn", player.avatar.last_interacted_turn),
                (
                    "lastPickedUpArtifactsTurn",
                    player.avatar.last_picked_up_artifacts_turn,
                ),
            ] {
                if let Some(turn) = turn {
                    avatar.insert(key.to_owned(), json!(turn));
                }
            }
        }
        if !player.avatar.temporary_power_sources.is_empty() {
            value["avatar"]["temporaryPowerSources"] = json!(player.avatar.temporary_power_sources);
        }
        value
    }

    fn cards_value(&self, cards: &[CardInstance]) -> Vec<Value> {
        cards.iter().map(|card| self.card_value(card)).collect()
    }

    fn card_value(&self, card: &CardInstance) -> Value {
        json!({
            "cardId": self.rules.cards[usize::from(card.card_id.0)].id,
            "instanceId": card.instance_id,
            "owner": card.owner,
            "source": card.source.as_str(),
        })
    }

    fn insert_realm_artifacts(&self, value: &mut Value) {
        if self.position.artifacts.is_empty() {
            return;
        }
        value["realm"]["artifacts"] = self
            .position
            .artifacts
            .iter()
            .map(|artifact| self.artifact_value(artifact))
            .collect();
    }

    fn artifact_value(&self, artifact: &ArtifactPosition) -> Value {
        let mut value = self.card_value(&artifact.card);
        if let Value::Object(object) = &mut value {
            match &artifact.placement {
                ArtifactPlacement::Carried { bearer } => {
                    object.insert("bearer".to_owned(), json!(bearer));
                }
                ArtifactPlacement::Loose { location, region } => {
                    object.insert("location".to_owned(), json!(location));
                    object.insert("region".to_owned(), json!(region));
                }
            }
        }
        value
    }

    fn site_value(&self, site: &SitePosition) -> Value {
        let mut value = self.card_value(&site.card);
        value["controller"] = json!(site.controller);
        value
    }

    fn unit_value(&self, unit: &UnitPosition) -> Value {
        let mut value = self.card_value(&unit.card);
        let Value::Object(object) = &mut value else {
            return value;
        };
        object.extend([
            ("controller".to_owned(), json!(unit.controller)),
            ("damage".to_owned(), json!(unit.damage)),
            ("location".to_owned(), json!(unit.location)),
            ("region".to_owned(), json!(unit.region)),
            ("stealthed".to_owned(), json!(unit.stealthed)),
            (
                "summoningSickness".to_owned(),
                json!(unit.summoning_sickness),
            ),
            ("tapped".to_owned(), json!(unit.tapped)),
            ("warded".to_owned(), json!(unit.warded)),
        ]);
        if let Some(turn) = unit.last_dropped_artifacts_turn {
            object.insert("lastDroppedArtifactsTurn".to_owned(), json!(turn));
        }
        if let Some(turn) = unit.last_interacted_turn {
            object.insert("lastInteractedTurn".to_owned(), json!(turn));
        }
        if let Some(turn) = unit.last_picked_up_artifacts_turn {
            object.insert("lastPickedUpArtifactsTurn".to_owned(), json!(turn));
        }
        if let Some(cells) = unit.occupied_cells {
            object.insert("occupiedCells".to_owned(), json!(cells));
        }
        if !unit.disable_effects.is_empty() {
            object.insert(
                "disableEffects".to_owned(),
                json!(
                    unit.disable_effects
                        .iter()
                        .map(|effect| json!({
                            "expiresAtSeat": effect.expires_at_seat,
                            "sourceInstanceId": effect.source_instance_id,
                        }))
                        .collect::<Vec<_>>()
                ),
            );
        }
        if unit.disabled_until_damaged {
            object.insert("disabledUntilDamaged".to_owned(), json!(true));
        }
        if unit.carried_lance_count > 0 {
            object.insert(
                "carriedLanceCount".to_owned(),
                json!(unit.carried_lance_count),
            );
        }
        if !unit.temporary_charge_sources.is_empty() {
            object.insert(
                "temporaryChargeSources".to_owned(),
                json!(unit.temporary_charge_sources),
            );
        }
        if !unit.temporary_power_sources.is_empty() {
            object.insert(
                "temporaryPowerSources".to_owned(),
                json!(unit.temporary_power_sources),
            );
        }
        value
    }

    fn pending_combat_value(pending: &PendingCombat) -> Value {
        let mut value = json!({
            "allocations": pending.allocations.iter().map(|allocation| json!({
                "amount": allocation.amount,
                "targetInstanceId": allocation.target_instance_id,
            })).collect::<Vec<_>>(),
            "attacker": {
                "instanceId": pending.attacker_instance_id,
                "kind": pending.attacker_kind.as_str(),
                "seat": pending.attacking_seat,
            },
            "attackingSeat": pending.attacking_seat,
            "cell": pending.cell,
            "combatants": pending.combatants,
            "defenders": pending.defenders,
            "originalTarget": pending.original_target,
            "targetRemoved": pending.target_removed,
        });
        if pending.region != Region::Surface {
            value["region"] = json!(pending.region);
        }
        value
    }

    fn terminal_value(&self) -> Value {
        self.position.terminal.map_or_else(
            || json!({ "status": "active" }),
            |terminal| match terminal {
                TerminalResult::Draw { reason } => json!({
                    "reason": match reason {
                        DrawReason::SimultaneousAvatarDefeat => "simultaneous_avatar_defeat",
                        DrawReason::SimultaneousDefeat => "simultaneous_defeat",
                    },
                    "result": "draw",
                    "status": "finished",
                }),
                TerminalResult::Win {
                    loser,
                    reason,
                    winner,
                } => json!({
                    "loser": loser,
                    "reason": match reason {
                        WinReason::AvatarDefeated => "avatar_defeated",
                        WinReason::DeckEmpty => "deck_empty",
                    },
                    "status": "finished",
                    "winner": winner,
                }),
            },
        )
    }
}

impl IssuedAction {
    /// Returns the typed action descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &ActionDescriptor {
        &self.descriptor
    }

    /// Returns the action's human-readable label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the acting seat.
    #[must_use]
    pub const fn seat(&self) -> Seat {
        self.seat
    }

    /// Returns the state version that issued this action.
    #[must_use]
    pub const fn state_version(&self) -> u64 {
        self.state_version
    }

    /// Materializes the typed action at the external JSON boundary.
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] if the typed descriptor cannot be represented or hashed.
    pub fn to_legal_action(&self) -> Result<LegalAction, GameError> {
        let descriptor = serde_json::to_value(&self.descriptor)?;
        Ok(LegalAction {
            action_id: opaque_action_id(
                ENGINE_VERSION,
                self.seat,
                self.state_version,
                &descriptor,
            )?,
            descriptor,
            label: self.label.clone(),
            seat: self.seat,
            state_version: self.state_version,
        })
    }
}

impl RulesContext {
    /// Returns the authority revision hash bound into this game.
    #[must_use]
    pub const fn authority_hash(&self) -> &IdentityHash {
        &self.authority_hash
    }

    /// Returns the engine contract version.
    #[must_use]
    pub const fn engine_version(&self) -> &'static str {
        ENGINE_VERSION
    }

    /// Returns the canonical manifest identity.
    #[must_use]
    pub const fn manifest_id(&self) -> &IdentityHash {
        &self.manifest_id
    }

    /// Returns the manifest-selected first seat.
    #[must_use]
    pub const fn first_seat(&self) -> Seat {
        self.first_seat
    }
}

impl Position {
    /// Returns the current state version.
    #[must_use]
    pub const fn state_version(&self) -> u64 {
        self.state_version
    }

    /// Returns the seat currently making the decision.
    #[must_use]
    pub const fn decision_seat(&self) -> Seat {
        self.decision_seat
    }
}

fn validate_manifest(
    raw: &Value,
    manifest: &Manifest,
) -> Result<BTreeMap<String, CardFacts>, GameError> {
    if manifest.engine_version != ENGINE_VERSION || manifest.schema_version != 1 {
        return Err(invalid("manifest engine or schema version is unsupported"));
    }
    validate_identifier(&manifest.authority.revision_id, "authority.revisionId")?;
    match manifest.authority.mode {
        AuthorityMode::PrivateLocal | AuthorityMode::Synthetic => {}
    }
    let mut body = raw.clone();
    let removed = body
        .as_object_mut()
        .and_then(|object| object.remove("manifestId"));
    if removed.is_none() || identity_hash(&body)? != manifest.identity {
        return Err(invalid("game manifest is not canonical"));
    }
    if manifest.cards.is_empty() || manifest.cards.len() > 5_000 {
        return Err(invalid("cards must contain 1-5000 definitions"));
    }
    let mut facts = BTreeMap::new();
    for (card_id, definition) in &manifest.cards {
        facts.insert(card_id.clone(), parse_card_definition(card_id, definition)?);
    }
    validate_deck(&manifest.decks.north, &facts)?;
    validate_deck(&manifest.decks.south, &facts)?;

    for deck in [&manifest.decks.north, &manifest.decks.south] {
        for card_id in &deck.spellbook {
            if matches!(facts.get(card_id), Some(CardFacts::Minion(minion)) if minion.token) {
                return Err(invalid("spellbook references an unsupported token spell"));
            }
        }
    }
    let mut referenced = BTreeSet::new();
    for deck in [&manifest.decks.north, &manifest.decks.south] {
        referenced.insert(deck.avatar.as_str());
        referenced.extend(deck.atlas.iter().map(String::as_str));
        referenced.extend(deck.spellbook.iter().map(String::as_str));
    }
    let token_sources: Vec<_> = referenced.iter().copied().collect();
    for card_id in token_sources {
        if let Some(token_id) = token_reference(&facts[card_id]) {
            if !matches!(facts.get(token_id), Some(CardFacts::Minion(minion)) if minion.token) {
                return Err(invalid("token effect must reference a token minion"));
            }
            referenced.insert(token_id);
        }
    }
    if referenced.len() != manifest.cards.len()
        || manifest
            .cards
            .keys()
            .any(|card_id| !referenced.contains(card_id.as_str()))
    {
        return Err(invalid(
            "cards must contain exactly the deck-referenced definitions",
        ));
    }
    Ok(facts)
}

fn create_player(
    rules: &RulesContext,
    deck: &Deck,
    seat: Seat,
    card_ids: &BTreeMap<String, CardId>,
    prng: &mut PrngState,
    random_draws: &mut Vec<EngineRandomDraw>,
) -> Result<PlayerPosition, GameError> {
    let mut atlas = instantiate_deck(rules, &deck.atlas, seat, CardSource::Atlas, card_ids)?;
    let mut spellbook = instantiate_deck(
        rules,
        &deck.spellbook,
        seat,
        CardSource::Spellbook,
        card_ids,
    )?;
    shuffle(
        &mut atlas,
        prng,
        &format!("setup_{}_atlas_shuffle", seat_name(seat)),
        random_draws,
    )?;
    shuffle(
        &mut spellbook,
        prng,
        &format!("setup_{}_spellbook_shuffle", seat_name(seat)),
        random_draws,
    )?;
    let avatar_card_id = *card_ids
        .get(&deck.avatar)
        .ok_or_else(|| invalid("validated deck lacks avatar definition"))?;
    let avatar_definition = &rules.cards[usize::from(avatar_card_id.0)];
    let CardFacts::Avatar(avatar_facts) = &avatar_definition.facts else {
        return Err(invalid("validated deck lacks avatar definition"));
    };
    let avatar = AvatarPosition {
        card: card_instance(rules, avatar_card_id, seat, CardSource::Avatar, 0)?,
        death_door_turn: None,
        last_dropped_artifacts_turn: None,
        last_interacted_turn: None,
        last_picked_up_artifacts_turn: None,
        life: u16::from(avatar_facts.life),
        location: Cell::parse(if seat == Seat::North { "C4" } else { "C1" })
            .map_err(|_| invalid("avatar start cell must be valid"))?,
        tapped: false,
        temporary_power_sources: Vec::new(),
    };
    let remaining_atlas = atlas.split_off(3);
    let remaining_spellbook = spellbook.split_off(3);
    Ok(PlayerPosition {
        air_thresholds_cast_this_turn: avatar_facts
            .tap_damage_random_other_unit_at_nearby_location_per_air_threshold_cast_this_turn
            .then_some(0),
        atlas: remaining_atlas,
        avatar,
        cemetery: Vec::new(),
        domain_established: false,
        hand_atlas: atlas,
        hand_spellbook: spellbook,
        mana: 0,
        mulligan_complete: false,
        spellbook: remaining_spellbook,
    })
}

fn instantiate_deck(
    rules: &RulesContext,
    deck: &[String],
    seat: Seat,
    source: CardSource,
    card_ids: &BTreeMap<String, CardId>,
) -> Result<Vec<CardInstance>, GameError> {
    deck.iter()
        .enumerate()
        .map(|(ordinal, card_id)| {
            let card_id = *card_ids
                .get(card_id)
                .ok_or_else(|| invalid("deck references a missing card"))?;
            card_instance(rules, card_id, seat, source, ordinal)
        })
        .collect()
}

fn card_instance(
    rules: &RulesContext,
    card_id: CardId,
    owner: Seat,
    source: CardSource,
    ordinal: usize,
) -> Result<CardInstance, GameError> {
    let definition = &rules.cards[usize::from(card_id.0)];
    Ok(CardInstance {
        card_id,
        instance_id: identity_hash(&json!({
            "authorityHash": rules.authority_hash,
            "cardId": definition.id,
            "definitionHash": definition.definition_hash,
            "engineVersion": ENGINE_VERSION,
            "firstSeat": rules.first_seat,
            "ordinal": ordinal,
            "owner": owner,
            "seed": rules.seed,
            "source": source.as_str(),
        }))?,
        owner,
        source,
    })
}

fn shuffle(
    cards: &mut [CardInstance],
    prng: &mut PrngState,
    purpose: &str,
    random_draws: &mut Vec<EngineRandomDraw>,
) -> Result<(), GameError> {
    for index in (1..cards.len()).rev() {
        let candidate = draw_index(
            prng,
            index + 1,
            purpose,
            "shuffle_index_candidate",
            Some(random_draws),
        )?;
        cards.swap(index, candidate);
    }
    Ok(())
}

fn draw_index(
    prng: &mut PrngState,
    exclusive_maximum: usize,
    purpose: &str,
    domain_kind: &'static str,
    mut random_draws: Option<&mut Vec<EngineRandomDraw>>,
) -> Result<usize, GameError> {
    let maximum = u64::try_from(exclusive_maximum)
        .map_err(|_| invalid("random choice size exceeds the supported range"))?;
    let limit = (1_u64 << 32) / maximum * maximum;
    loop {
        let pre_prng_state_hash = if random_draws.is_some() {
            Some(identity_hash(&serde_json::to_value(*prng)?)?)
        } else {
            None
        };
        let result = prng.draw_u32();
        let draw = u64::from(result);
        let accepted = draw < limit;
        if let Some(random_draws) = random_draws.as_deref_mut() {
            random_draws.push(EngineRandomDraw {
                domain: RandomDomain {
                    accepted,
                    exclusive_maximum,
                    kind: domain_kind,
                },
                draw_sequence: prng.draws,
                post_prng_state_hash: identity_hash(&serde_json::to_value(*prng)?)?,
                pre_prng_state_hash: pre_prng_state_hash
                    .ok_or_else(|| invalid("recorded random draw lacks its pre-state hash"))?,
                purpose: purpose.to_owned(),
                result,
            });
        }
        if accepted {
            return usize::try_from(draw % maximum)
                .map_err(|_| invalid("random choice index exceeds the supported range"));
        }
    }
}

const fn seat_name(seat: Seat) -> &'static str {
    match seat {
        Seat::North => "north",
        Seat::South => "south",
    }
}

fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    if items.len() < 2 {
        return vec![items.to_vec()];
    }
    let mut result = Vec::new();
    for index in 0..items.len() {
        let mut rest = items.to_vec();
        let first = rest.remove(index);
        for mut tail in permutations(&rest) {
            tail.insert(0, first.clone());
            result.push(tail);
        }
    }
    result
}

fn resolve_mulligan_zone(
    hand: &mut Vec<CardInstance>,
    deck: &mut Vec<CardInstance>,
    order: &[IdentityHash],
) -> Result<(), GameError> {
    let mut returned = Vec::with_capacity(order.len());
    for instance_id in order {
        let card = hand
            .iter()
            .find(|card| card.instance_id == *instance_id)
            .cloned()
            .ok_or(GameError::IllegalAction)?;
        if returned
            .iter()
            .any(|returned: &CardInstance| returned.instance_id == *instance_id)
        {
            return Err(GameError::IllegalAction);
        }
        returned.push(card);
    }
    hand.retain(|card| !order.contains(&card.instance_id));
    deck.extend(returned);
    hand.extend(deck.drain(..order.len()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::canonical_json;
    use crate::synthetic::synthetic_demo_manifest_json;

    fn selfplay_manifest_with(seed: u32, mutate: impl FnOnce(&mut Value)) -> String {
        let mut manifest: Value =
            serde_json::from_str(&synthetic_demo_manifest_json(seed).expect("synthetic manifest"))
                .expect("manifest JSON");
        manifest
            .as_object_mut()
            .expect("manifest object")
            .remove("manifestId");
        mutate(&mut manifest);
        manifest["manifestId"] = json!(identity_hash(&manifest).expect("manifest identity"));
        canonical_json(&manifest).expect("canonical manifest")
    }

    fn play_site_actions(game: &Game, card_instance_id: &IdentityHash) -> Vec<IssuedAction> {
        game.legal_actions()
            .expect("legal actions")
            .into_iter()
            .filter(|action| match &action.descriptor {
                ActionDescriptor::PlaySite {
                    card_instance_id: candidate,
                    ..
                } => *candidate == *card_instance_id,
                _ => false,
            })
            .collect()
    }

    #[test]
    fn zero_site_recovery_should_issue_every_nearest_cell_in_canonical_order() {
        let manifest = synthetic_demo_manifest_json(267).expect("synthetic manifest");
        let mut game = Game::from_manifest_json(&manifest).expect("valid game");
        let c3 = Cell::parse("C3").expect("C3");
        let c4 = Cell::parse("C4").expect("C4");
        let south_site = game.position.players[seat_index(Seat::South)]
            .hand_atlas
            .remove(0);
        game.position.sites[c3.index()] = Some(SitePosition {
            card: south_site,
            controller: Seat::South,
        });
        game.position.rubble[c4.index()] = Some(
            identity_hash(&json!({ "fixture": "zero-site-recovery-rubble" }))
                .expect("Rubble identity"),
        );
        let north = &mut game.position.players[seat_index(Seat::North)];
        north.avatar.location = c3;
        north.avatar.tapped = false;
        north.domain_established = true;
        let recovery_card = north.hand_atlas[0].instance_id.clone();
        game.position.active_seat = Seat::North;
        game.position.decision_seat = Seat::North;
        game.position.phase = Phase::Main;

        let recovery_actions = play_site_actions(&game, &recovery_card);
        assert_eq!(
            recovery_actions
                .iter()
                .map(|action| match action.descriptor {
                    ActionDescriptor::PlaySite { cell, .. } => cell,
                    _ => unreachable!("filtered PlaySite action"),
                })
                .collect::<Vec<_>>(),
            ["B3", "C2", "C4", "D3"].map(|cell| Cell::parse(cell).expect("valid cell"))
        );

        let mut forged = recovery_actions[0].clone();
        let ActionDescriptor::PlaySite { cell, .. } = &mut forged.descriptor else {
            unreachable!("filtered PlaySite action");
        };
        *cell = Cell::parse("A1").expect("A1");
        let before_forgery = game.authoritative_state();
        assert!(matches!(
            game.apply_action(&forged),
            Err(GameError::IllegalAction)
        ));
        assert_eq!(game.authoritative_state(), before_forgery);

        game.position.players[seat_index(Seat::North)]
            .avatar
            .location = c4;
        let replacement_actions = play_site_actions(&game, &recovery_card);
        assert_eq!(replacement_actions.len(), 1);
        let (outcomes, random_draws) = game
            .apply_action_recorded(&replacement_actions[0])
            .expect("issued Rubble replacement");
        assert!(random_draws.is_empty());
        assert_eq!(
            outcomes
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            ["rubble-replaced", "site-played"]
        );
        assert!(game.position.rubble[c4.index()].is_none());
        assert!(game.position.sites[c4.index()].is_some());
    }

    #[test]
    fn rubble_replacement_support_should_be_scoped_to_its_owners_atlas() {
        let cross_deck = selfplay_manifest_with(31, |manifest| {
            manifest["cards"]["north-avatar"]["replaceAdjacentRubbleWithTopAtlasSite"] =
                json!(true);
            manifest["cards"]["south-site-1"]["genesisGainMana"] = json!(1);
        });
        Game::from_manifest_json(&cross_deck)
            .expect("valid cross-deck game")
            .ensure_selfplay_supported()
            .expect("unrelated Atlas Genesis is safe");

        let same_deck = selfplay_manifest_with(31, |manifest| {
            manifest["cards"]["north-avatar"]["replaceAdjacentRubbleWithTopAtlasSite"] =
                json!(true);
            manifest["cards"]["north-site-1"]["genesisGainMana"] = json!(1);
        });
        assert!(matches!(
            Game::from_manifest_json(&same_deck)
                .expect("valid same-deck game")
                .ensure_selfplay_supported(),
            Err(GameError::UnsupportedManifestFact(field)) if field == "genesisGainMana"
        ));
    }

    #[test]
    fn supported_magic_effects_should_be_selfplay_supported() {
        for (effect, facts) in [
            (
                MagicEffect::DamageChainNearbyUnits,
                json!({ "damageChainNearbyUnits": true }),
            ),
            (
                MagicEffect::DamageEachAbovegroundMinionOne,
                json!({ "damageEachAbovegroundMinion": 1 }),
            ),
            (
                MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(3),
                json!({ "damageEachUnitAtLocationWithinTwoSteps": 3 }),
            ),
            (
                MagicEffect::DestroyTargetSiteWithDamageGrid([10, 7, 4, 2, 1]),
                json!({
                    "damageUnitsAboveAndBelowTargetSiteByManhattanDistance": [10, 7, 4, 2, 1],
                    "destroyTargetSite": true,
                    "discardSiteAsAdditionalCost": true,
                }),
            ),
            (
                MagicEffect::FightAllyWithAdjacentEnemy,
                json!({ "fightAllyWithAdjacentEnemy": true }),
            ),
            (
                MagicEffect::LeapAttackAlly,
                json!({ "leapAttackAlly": true }),
            ),
        ] {
            assert_eq!(unsupported_magic_effect(&effect), None);
            let manifest = selfplay_manifest_with(31, |manifest| {
                for ordinal in 1..=50 {
                    let card = &mut manifest["cards"][format!("north-spell-{ordinal}")];
                    *card = json!({
                        "cardType": "magic",
                        "manaCost": 0,
                        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                    });
                    for (field, value) in facts.as_object().expect("supported Magic facts") {
                        card[field.as_str()] = value.clone();
                    }
                }
            });
            Game::from_manifest_json(&manifest)
                .expect("valid supported Magic manifest")
                .ensure_selfplay_supported()
                .expect("supported Magic effect is self-play safe");
        }
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one direct payment proof keeps self-play admission and zero/one-card legality together"
    )]
    fn alternate_summon_payments_should_be_admitted_and_require_their_costs() {
        let discard_manifest = selfplay_manifest_with(417, |manifest| {
            for ordinal in 1..=50 {
                manifest["cards"][format!("north-spell-{ordinal}")]["discardRandomCardInsteadOfMana"] =
                    json!(true);
            }
        });
        Game::from_manifest_json(&discard_manifest)
            .expect("valid random-discard manifest")
            .ensure_selfplay_supported()
            .expect("random-discard payment is self-play safe");

        let sacrifice_manifest = selfplay_manifest_with(417, |manifest| {
            manifest["cards"]["north-spell-1"]["sacrificeMinionAtSummoningLocationForManaDiscount"] =
                json!(2);
        });
        Game::from_manifest_json(&sacrifice_manifest)
            .expect("valid sacrifice-payment manifest")
            .ensure_selfplay_supported()
            .expect("sacrifice-discount payment is self-play safe");

        let mut game = Game::from_manifest_json(&discard_manifest).expect("valid Aramos game");
        let cell = Cell::parse("C4").expect("C4");
        let north = &mut game.position.players[seat_index(Seat::North)];
        let site_card = north.hand_atlas.remove(0);
        north.atlas.extend(std::mem::take(&mut north.hand_atlas));
        let aramos = north.hand_spellbook.remove(0);
        north
            .spellbook
            .extend(std::mem::take(&mut north.hand_spellbook));
        let aramos_instance_id = aramos.instance_id.clone();
        north.hand_spellbook.push(aramos);
        north.avatar.location = cell;
        north.domain_established = true;
        north.mana = 0;
        game.position.sites[cell.index()] = Some(SitePosition {
            card: site_card,
            controller: Seat::North,
        });
        game.position.active_seat = Seat::North;
        game.position.decision_seat = Seat::North;
        game.position.phase = Phase::Main;

        assert!(
            !game
                .legal_actions()
                .expect("actions without a payment card")
                .iter()
                .any(|action| matches!(
                    action.descriptor,
                    ActionDescriptor::SummonMinion {
                        payment_mode: Some(SummonPaymentMode::RandomCardDiscard),
                        ..
                    }
                ))
        );

        let only_payment_card = game.position.players[seat_index(Seat::North)]
            .atlas
            .pop()
            .expect("one Atlas payment card");
        let payment_instance_id = only_payment_card.instance_id.clone();
        game.position.players[seat_index(Seat::North)]
            .hand_atlas
            .push(only_payment_card);
        game.position.players[seat_index(Seat::North)].mana = 1;
        let payment_actions = game.legal_actions().expect("actions with one payment card");
        assert!(payment_actions.iter().any(|action| matches!(
            action.descriptor,
            ActionDescriptor::SummonMinion {
                mana_cost: 1,
                payment_mode: None,
                ..
            }
        )));
        let action = payment_actions
            .into_iter()
            .find(|action| {
                matches!(
                    &action.descriptor,
                    ActionDescriptor::SummonMinion {
                        card_instance_id,
                        payment_mode: Some(SummonPaymentMode::RandomCardDiscard),
                        ..
                    } if *card_instance_id == aramos_instance_id
                )
            })
            .expect("random-discard summon");
        let (outcomes, random_draws) = game
            .apply_action_recorded(&action)
            .expect("issued random-discard summon");

        assert_eq!(random_draws.len(), 1);
        assert_eq!(random_draws[0].domain.exclusive_maximum, 1);
        assert_eq!(random_draws[0].domain.kind, "card_index_candidate");
        assert_eq!(random_draws[0].purpose, "summon_random_card_discard_cost");
        assert_eq!(
            outcomes
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            ["card-discarded", "minion-summoned"]
        );
        assert_eq!(outcomes[0].1["instanceId"], payment_instance_id.as_str());
        assert_eq!(outcomes[0].1["zone"], "atlas");
        assert!(
            game.position.players[seat_index(Seat::North)]
                .cemetery
                .iter()
                .any(|card| card.instance_id == payment_instance_id)
        );
        assert_eq!(game.position.players[seat_index(Seat::North)].mana, 1);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one internal proof keeps measured topology, oversized footprints, and region isolation together"
    )]
    fn minor_explosion_should_measure_connected_locations_and_hit_only_the_target_region() {
        let manifest = selfplay_manifest_with(31, |manifest| {
            for ordinal in 1..=50 {
                manifest["cards"][format!("north-spell-{ordinal}")] = json!({
                    "cardType": "magic",
                    "damageEachUnitAtLocationWithinTwoSteps": 3,
                    "manaCost": 0,
                    "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                });
            }
            for ordinal in 1..=30 {
                manifest["cards"][format!("north-site-{ordinal}")]["elements"] = json!(["fire"]);
            }
            manifest["cards"]["south-spell-1"] = json!({
                "attack": 1,
                "cardType": "minion",
                "defense": 10,
                "manaCost": 0,
                "occupiesSquareArea": 2,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            });
            manifest["cards"]["south-spell-2"] = json!({
                "attack": 1,
                "burrowing": true,
                "cardType": "minion",
                "defense": 10,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            });
        });
        let mut game = Game::from_manifest_json(&manifest).expect("valid Minor Explosion game");
        let cells =
            ["B2", "B3", "C2", "C3", "C4"].map(|cell| Cell::parse(cell).expect("fixture cell"));
        let mut site_cards = Vec::new();
        let north = &mut game.position.players[seat_index(Seat::North)];
        while site_cards.len() < cells.len() {
            site_cards.push(if north.hand_atlas.is_empty() {
                north.atlas.remove(0)
            } else {
                north.hand_atlas.remove(0)
            });
        }
        game.position.sites.fill(None);
        for (cell, card) in [cells[1], cells[2], cells[4]]
            .into_iter()
            .zip(site_cards.drain(..3))
        {
            game.position.sites[cell.index()] = Some(SitePosition {
                card,
                controller: Seat::North,
            });
        }
        let start = Location {
            cell: cells[4],
            region: Region::Surface,
        };
        assert_eq!(game.locations_within_measured_steps(start, 2), [start]);

        for (cell, card) in [cells[0], cells[3]].into_iter().zip(site_cards) {
            game.position.sites[cell.index()] = Some(SitePosition {
                card,
                controller: Seat::North,
            });
        }
        assert_eq!(
            game.locations_within_measured_steps(start, 2),
            [cells[1], cells[2], cells[3], cells[4]].map(|cell| Location {
                cell,
                region: Region::Surface,
            })
        );

        let card_id = |id: &str| {
            CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|card| card.id == id)
                        .expect("target card"),
                )
                .expect("target card index"),
            )
        };
        let surface_id = "sha256:7777777777777777777777777777777777777777777777777777777777777777";
        let underground_id =
            "sha256:8888888888888888888888888888888888888888888888888888888888888888";
        let area = Cell::SQUARE_AREAS
            .into_iter()
            .find(|area| area[0] == cells[0])
            .expect("B2 footprint");
        game.position.units.push(test_minion(
            card_id("south-spell-1"),
            surface_id,
            Seat::South,
            area[0],
            Some(area),
        ));
        let mut underground = test_minion(
            card_id("south-spell-2"),
            underground_id,
            Seat::South,
            cells[2],
            None,
        );
        underground.region = Region::Underground;
        game.position.units.push(underground);
        let north = &mut game.position.players[seat_index(Seat::North)];
        north.avatar.location = cells[4];
        north.domain_established = true;
        north.mana = 10;
        game.position.active_seat = Seat::North;
        game.position.decision_seat = Seat::North;
        game.position.phase = Phase::Main;
        let action = game
            .legal_actions()
            .expect("Minor Explosion actions")
            .into_iter()
            .find(|action| {
                matches!(
                    action.descriptor,
                    ActionDescriptor::CastMagic {
                        target_location: Some(Location { cell, region: Region::Surface }),
                        ..
                    } if cell == cells[2]
                )
            })
            .expect("C2 Minor Explosion");
        let (outcomes, random_draws) = game
            .apply_action_recorded(&action)
            .expect("issued Minor Explosion");

        assert!(random_draws.is_empty());
        assert_eq!(
            outcomes
                .iter()
                .filter(|(event_type, _)| event_type == "magic-damage-allocated")
                .map(|(_, payload)| payload["targetInstanceId"].clone())
                .collect::<Vec<_>>(),
            [json!(surface_id)]
        );
        assert_eq!(
            game.position
                .units
                .iter()
                .find(|unit| unit.card.instance_id.as_str() == surface_id)
                .expect("surface oversized minion")
                .damage,
            3
        );
        assert_eq!(
            game.position
                .units
                .iter()
                .find(|unit| unit.card.instance_id.as_str() == underground_id)
                .expect("underground minion")
                .damage,
            0
        );
    }

    #[test]
    fn forceful_burrow_magic_should_admit_minion_slices_and_reject_unmodeled_cards() {
        let bury_manifest = |extra: Option<(&str, Value)>| {
            selfplay_manifest_with(31, |manifest| {
                for ordinal in 1..=50 {
                    manifest["cards"][format!("north-spell-{ordinal}")] = json!({
                        "burrowTargetMinionOrArtifact": true,
                        "cardType": "magic",
                        "manaCost": 0,
                        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                    });
                }
                if let Some((field, value)) = extra {
                    manifest["cards"]["south-spell-1"][field] = value;
                }
            })
        };
        Game::from_manifest_json(&bury_manifest(None))
            .expect("valid Bury manifest")
            .ensure_selfplay_supported()
            .expect("ordinary minion Bury is self-play safe");
        assert!(matches!(
            Game::from_manifest_json(&bury_manifest(Some(("burrowing", json!(true)))))
                .expect("valid Burrowing manifest")
                .ensure_selfplay_supported(),
            Err(GameError::UnsupportedManifestFact(field)) if field == "burrowing"
        ));

        let cave_in = selfplay_manifest_with(31, |manifest| {
            for ordinal in 1..=50 {
                manifest["cards"][format!("north-spell-{ordinal}")] = json!({
                    "burrowAllMinionsAndArtifactsAtTargetLandSite": true,
                    "cardType": "magic",
                    "manaCost": 0,
                    "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                });
            }
        });
        Game::from_manifest_json(&cave_in)
            .expect("valid Cave-In manifest")
            .ensure_selfplay_supported()
            .expect("artifact-free Cave-In is self-play safe");

        let power_artifact = json!({
            "cardType": "artifact",
            "grantsBearerPower": 2,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        });
        let artifact_manifest = |artifact: &Value, burrows: bool| {
            selfplay_manifest_with(31, |manifest| {
                manifest["cards"]["south-spell-1"] = artifact.clone();
                if burrows {
                    for ordinal in 1..=50 {
                        manifest["cards"][format!("north-spell-{ordinal}")] = json!({
                            "burrowTargetMinionOrArtifact": true,
                            "cardType": "magic",
                            "manaCost": 0,
                            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                        });
                    }
                }
            })
        };
        Game::from_manifest_json(&artifact_manifest(&power_artifact, false))
            .expect("valid power Artifact manifest")
            .ensure_selfplay_supported()
            .expect("power Artifacts are self-play safe");
        assert!(matches!(
            Game::from_manifest_json(&artifact_manifest(&power_artifact, true))
                .expect("valid Bury plus Artifact manifest")
                .ensure_selfplay_supported(),
            Err(GameError::UnsupportedManifestFact(field))
                if field == "burrowTargetMinionOrArtifact with cardType:artifact"
        ));

        let lethal_artifact = json!({
            "cardType": "artifact",
            "grantsBearerLethal": true,
            "manaCost": 0,
            "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
        });
        Game::from_manifest_json(&artifact_manifest(&lethal_artifact, false))
            .expect("valid Lethal Artifact manifest")
            .ensure_selfplay_supported()
            .expect("Lethal Artifacts are self-play safe");
        assert!(matches!(
            Game::from_manifest_json(&artifact_manifest(&lethal_artifact, true))
                .expect("valid Bury plus Lethal Artifact manifest")
                .ensure_selfplay_supported(),
            Err(GameError::UnsupportedManifestFact(field))
                if field == "burrowTargetMinionOrArtifact with cardType:artifact"
        ));
    }

    #[test]
    fn post_action_terminal_event_should_follow_magic_completion() {
        let mut events = vec![
            ("magic-cast".to_owned(), Value::Null),
            ("magic-resolved".to_owned(), Value::Null),
        ];
        let mut outcomes = OutcomeLog::Record(&mut events);
        let settlement_start = outcomes.len();
        outcomes.push("minion-died", || Value::Null);
        outcomes.push("game-ended", || Value::Null);
        outcomes.move_tail_before_completion(settlement_start);

        assert_eq!(
            events
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            ["magic-cast", "minion-died", "magic-resolved", "game-ended",]
        );
    }

    #[test]
    fn ordered_terminal_cleanup_should_omit_resolved_chain_magic() {
        let manifest = selfplay_manifest_with(31, |_| {});
        let mut game = Game::from_manifest_json(&manifest).expect("valid game");
        game.position.pending_chain_magic = PendingField::Resolved;
        game.position.phase = Phase::DeathriteOrder;
        game.clear_ordered_terminal_continuations();

        assert_eq!(game.position.pending_chain_magic, PendingField::Absent);
        assert!(
            game.authoritative_state()
                .get("pendingChainMagic")
                .is_none()
        );
        assert_eq!(game.position.phase, Phase::Terminal);
    }

    #[test]
    fn sparkmage_should_issue_and_accept_a_nearby_rubble_target() {
        let manifest = selfplay_manifest_with(31, |manifest| {
            manifest["cards"]["north-avatar"]["tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn"] =
                json!(true);
        });
        let mut game = Game::from_manifest_json(&manifest).expect("valid game");
        let c4 = Cell::parse("C4").expect("C4");
        game.position.rubble[c4.index()] = Some(
            identity_hash(&json!({ "fixture": "sparkmage-rubble" })).expect("Rubble identity"),
        );
        let north = &mut game.position.players[seat_index(Seat::North)];
        north.avatar.tapped = false;
        north.domain_established = true;
        let candidate_card = north.hand_spellbook.remove(0);
        game.position.units.push(UnitPosition {
            card: candidate_card,
            carried_lance_count: 0,
            controller: Seat::North,
            damage: 0,
            disable_effects: Vec::new(),
            disabled_until_damaged: false,
            last_dropped_artifacts_turn: None,
            last_interacted_turn: None,
            last_picked_up_artifacts_turn: None,
            location: c4,
            occupied_cells: None,
            region: Region::Surface,
            stealthed: false,
            summoning_sickness: false,
            tapped: false,
            temporary_charge_sources: Vec::new(),
            temporary_power_sources: Vec::new(),
            warded: false,
        });
        game.position.active_seat = Seat::North;
        game.position.decision_seat = Seat::North;
        game.position.phase = Phase::Main;

        let action = game
            .legal_actions()
            .expect("legal actions")
            .into_iter()
            .find(|action| {
                matches!(
                    action.descriptor,
                    ActionDescriptor::ActivateSparkmage {
                        target_location: Location {
                            cell,
                            region: Region::Surface,
                        },
                        ..
                    } if cell == c4
                )
            })
            .expect("Rubble is an existing surface target");
        let mut speculative = game.clone();
        speculative
            .apply_action(&action)
            .expect("speculative Sparkmage action");
        let (outcomes, random_draws) = game
            .apply_action_recorded(&action)
            .expect("issued Sparkmage action");

        assert_eq!(
            outcomes
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            ["sparkmage-activated"]
        );
        assert!(!random_draws.is_empty());
        assert_eq!(speculative.position, game.position);
        assert!(game.position.players[seat_index(Seat::North)].avatar.tapped);
    }

    fn test_minion(
        card_id: CardId,
        instance_id: &str,
        controller: Seat,
        location: Cell,
        occupied_cells: Option<SquareArea>,
    ) -> UnitPosition {
        UnitPosition {
            card: CardInstance {
                card_id,
                instance_id: IdentityHash::parse(instance_id).expect("fixture identity"),
                owner: controller,
                source: CardSource::Spellbook,
            },
            carried_lance_count: 0,
            controller,
            damage: 0,
            disable_effects: Vec::new(),
            disabled_until_damaged: false,
            last_dropped_artifacts_turn: None,
            last_interacted_turn: None,
            last_picked_up_artifacts_turn: None,
            location,
            occupied_cells,
            region: Region::Surface,
            stealthed: false,
            summoning_sickness: false,
            tapped: true,
            temporary_charge_sources: Vec::new(),
            temporary_power_sources: Vec::new(),
            warded: false,
        }
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one direct terrain proof keeps protected, Rubble, and subsurface Sinkhole branches together"
    )]
    fn site_destruction_should_preserve_protected_rubble_and_relative_subsurface() {
        let admitted = selfplay_manifest_with(244, |manifest| {
            manifest["cards"]["north-site-1"]["sacrificeToDestroyNearbySite"] = json!(true);
            manifest["cards"]["north-site-2"]["cannotBeMovedDestroyedOrModified"] = json!(true);
        });
        Game::from_manifest_json(&admitted)
            .expect("valid Sinkhole manifest")
            .ensure_selfplay_supported()
            .expect("Sinkhole source and protection facts are self-play safe");

        let manifest = selfplay_manifest_with(244, |manifest| {
            manifest["cards"]["north-site-1"]["sacrificeToDestroyNearbySite"] = json!(true);
            manifest["cards"]["north-site-2"]["cannotBeMovedDestroyedOrModified"] = json!(true);
            manifest["cards"]["south-site-1"]["elements"] = json!(["water"]);
            for ordinal in [1, 2] {
                manifest["cards"][format!("south-spell-{ordinal}")]["manaCost"] = json!(0);
                manifest["cards"][format!("south-spell-{ordinal}")]["submerge"] = json!(true);
                manifest["cards"][format!("south-spell-{ordinal}")]["thresholds"] =
                    json!({ "air": 0, "earth": 0, "fire": 0, "water": 0 });
            }
            manifest["cards"]["south-spell-2"]["burrowing"] = json!(true);
            manifest["cards"]["south-spell-1"]["deathriteDrawSite"] = json!(true);
        });
        let mut base = Game::from_manifest_json(&manifest).expect("valid terrain fixture");
        let card_id = |name: &str| {
            CardId(
                u16::try_from(
                    base.rules
                        .cards
                        .iter()
                        .position(|card| card.id == name)
                        .expect("fixture card"),
                )
                .expect("fixture card index"),
            )
        };
        let source_id =
            identity_hash(&json!({ "fixture": "sinkhole-source" })).expect("source identity");
        let protected_id =
            identity_hash(&json!({ "fixture": "sinkhole-protected" })).expect("protected identity");
        let water_id =
            identity_hash(&json!({ "fixture": "sinkhole-water" })).expect("water identity");
        let source = SitePosition {
            card: CardInstance {
                card_id: card_id("north-site-1"),
                instance_id: source_id.clone(),
                owner: Seat::North,
                source: CardSource::Atlas,
            },
            controller: Seat::North,
        };
        let protected = SitePosition {
            card: CardInstance {
                card_id: card_id("north-site-2"),
                instance_id: protected_id.clone(),
                owner: Seat::North,
                source: CardSource::Atlas,
            },
            controller: Seat::North,
        };
        let water = SitePosition {
            card: CardInstance {
                card_id: card_id("south-site-1"),
                instance_id: water_id.clone(),
                owner: Seat::South,
                source: CardSource::Atlas,
            },
            controller: Seat::South,
        };
        let c2 = Cell::parse("C2").expect("C2");
        let c3 = Cell::parse("C3").expect("C3");
        let c4 = Cell::parse("C4").expect("C4");
        base.position.sites = std::array::from_fn(|_| None);
        base.position.rubble = std::array::from_fn(|_| None);
        base.position.sites[c2.index()] = Some(water);
        base.position.sites[c3.index()] = Some(source);
        base.position.sites[c4.index()] = Some(protected.clone());
        let drowned_id = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let survivor_id = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let mut drowned = test_minion(card_id("south-spell-1"), drowned_id, Seat::South, c2, None);
        drowned.region = Region::Underwater;
        let mut survivor =
            test_minion(card_id("south-spell-2"), survivor_id, Seat::South, c2, None);
        survivor.region = Region::Underwater;
        base.position.units = vec![drowned, survivor];
        base.position.active_seat = Seat::North;
        base.position.decision_seat = Seat::North;
        base.position.phase = Phase::Main;
        base.position.players[seat_index(Seat::North)].domain_established = true;

        let source_actions = base
            .legal_actions()
            .expect("site destruction actions")
            .into_iter()
            .filter(|action| {
                matches!(
                    &action.descriptor,
                    ActionDescriptor::ActivateSiteDestruction {
                        source_site_instance_id,
                        ..
                    } if *source_site_instance_id == source_id
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            source_actions
                .iter()
                .map(|action| match action.descriptor {
                    ActionDescriptor::ActivateSiteDestruction { target_cell, .. } => target_cell,
                    _ => unreachable!("filtered site destruction"),
                })
                .collect::<Vec<_>>(),
            [c2, c3, c4]
        );

        let mut protected_branch = base.clone();
        let protected_action = source_actions
            .iter()
            .find(|action| {
                matches!(
                    action.descriptor,
                    ActionDescriptor::ActivateSiteDestruction { target_cell, .. } if target_cell == c4
                )
            })
            .expect("protected target action");
        let (protected_events, protected_random) = protected_branch
            .apply_action_recorded(protected_action)
            .expect("protected target activation");
        assert!(protected_random.is_empty());
        assert_eq!(
            protected_events
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            [
                "site-sacrificed",
                "site-destruction-prevented",
                "rubble-created",
            ]
        );
        assert_eq!(
            protected_branch.position.sites[c4.index()],
            Some(protected.clone())
        );
        assert!(protected_branch.position.sites[c3.index()].is_none());

        let mut rubble_branch = base.clone();
        rubble_branch.position.units.clear();
        rubble_branch.position.sites[c2.index()] = None;
        let existing_rubble =
            identity_hash(&json!({ "fixture": "existing-rubble" })).expect("Rubble identity");
        rubble_branch.position.rubble[c2.index()] = Some(existing_rubble.clone());
        let rubble_action = rubble_branch
            .legal_actions()
            .expect("Rubble target actions")
            .into_iter()
            .find(|action| {
                matches!(
                    &action.descriptor,
                    ActionDescriptor::ActivateSiteDestruction {
                        source_site_instance_id,
                        target_cell,
                        ..
                    } if *source_site_instance_id == source_id && *target_cell == c2
                )
            })
            .expect("existing Rubble target");
        let (rubble_events, _) = rubble_branch
            .apply_action_recorded(&rubble_action)
            .expect("Rubble target activation");
        assert_eq!(
            rubble_branch.position.rubble[c2.index()],
            Some(existing_rubble)
        );
        assert_eq!(
            rubble_events
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            ["site-sacrificed", "site-destroyed", "rubble-created"]
        );
        assert!(rubble_events[1].1.get("owner").is_none());

        let mut terminal_branch = base.clone();
        terminal_branch.position.players[seat_index(Seat::South)]
            .atlas
            .clear();
        let terminal_action = source_actions
            .iter()
            .find(|action| {
                matches!(
                    action.descriptor,
                    ActionDescriptor::ActivateSiteDestruction { target_cell, .. } if target_cell == c2
                )
            })
            .expect("terminal Water target action");
        let (terminal_events, _) = terminal_branch
            .apply_action_recorded(terminal_action)
            .expect("terminal Water target activation");
        let terminal_types = terminal_events
            .iter()
            .map(|(event_type, _)| event_type.as_str())
            .collect::<Vec<_>>();
        assert!(
            terminal_types
                .iter()
                .rposition(|event_type| *event_type == "rubble-created")
                < terminal_types
                    .iter()
                    .position(|event_type| *event_type == "game-ended")
        );

        let mut destroyed = base;
        let normal_action = source_actions
            .iter()
            .find(|action| {
                matches!(
                    action.descriptor,
                    ActionDescriptor::ActivateSiteDestruction { target_cell, .. } if target_cell == c2
                )
            })
            .expect("Water target action");
        let (events, random_draws) = destroyed
            .apply_action_recorded(normal_action)
            .expect("Water target activation");
        assert!(random_draws.is_empty());
        let event_types = events
            .iter()
            .map(|(event_type, _)| event_type.as_str())
            .collect::<Vec<_>>();
        assert_eq!(&event_types[..2], ["site-sacrificed", "site-destroyed"]);
        assert_eq!(
            &event_types[event_types.len() - 2..],
            ["rubble-created", "rubble-created"]
        );
        assert!(
            !destroyed
                .position
                .units
                .iter()
                .any(|unit| unit.card.instance_id.as_str() == drowned_id)
        );
        assert_eq!(
            destroyed
                .position
                .units
                .iter()
                .find(|unit| unit.card.instance_id.as_str() == survivor_id)
                .expect("Burrowing survivor")
                .region,
            Region::Underground
        );
        let south_cemetery = &destroyed.position.players[seat_index(Seat::South)].cemetery;
        assert_eq!(south_cemetery[0].instance_id.as_str(), drowned_id);
        assert_eq!(south_cemetery[1].instance_id, water_id);
        assert!(destroyed.position.sites[c2.index()].is_none());
        assert!(destroyed.position.sites[c3.index()].is_none());
        assert_eq!(destroyed.position.sites[c4.index()], Some(protected));
    }

    #[test]
    fn temporary_power_should_apply_to_avatar_and_disabled_minion_stats() {
        let manifest = synthetic_demo_manifest_json(31).expect("synthetic manifest");
        let mut game = Game::from_manifest_json(&manifest).expect("valid game");
        let source =
            identity_hash(&json!({ "fixture": "temporary-power" })).expect("power source identity");
        let north = seat_index(Seat::North);
        let avatar_card_id = game.position.players[north].avatar.card.card_id;
        let CardFacts::Avatar(avatar_facts) =
            &game.rules.cards[usize::from(avatar_card_id.0)].facts
        else {
            panic!("North Avatar facts");
        };
        let printed_avatar_attack = u16::from(avatar_facts.attack);
        let avatar_id = game.position.players[north].avatar.card.instance_id.clone();
        game.position.players[north]
            .avatar
            .temporary_power_sources
            .push(source.clone());
        assert_eq!(
            game.combatant_attack_and_lethal(UnitKind::Avatar, Seat::North, &avatar_id)
                .expect("powered Avatar stats"),
            (printed_avatar_attack + 2, false)
        );

        let card_id = game.position.players[north]
            .hand_spellbook
            .iter()
            .find_map(|card| {
                matches!(
                    game.rules.cards[usize::from(card.card_id.0)].facts,
                    CardFacts::Minion(_)
                )
                .then_some(card.card_id)
            })
            .expect("North minion card");
        let CardFacts::Minion(facts) = &game.rules.cards[usize::from(card_id.0)].facts else {
            unreachable!("selected minion facts");
        };
        let printed = (u16::from(facts.attack), u16::from(facts.defense));
        let mut unit = test_minion(
            card_id,
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            Seat::North,
            Cell::parse("C4").expect("C4"),
            None,
        );
        unit.disabled_until_damaged = true;
        unit.temporary_power_sources.push(source);
        assert!(game.minion_is_disabled(&unit));
        assert_eq!(
            game.minion_current_stats(&unit)
                .expect("powered disabled minion stats"),
            (printed.0 + 2, printed.1 + 2, false)
        );
        unit.region = Region::Underground;
        unit.stealthed = true;
        unit.warded = true;
        let unit_id = unit.card.instance_id.clone();
        game.position.units.push(unit);
        assert!(
            game.magic_choices(
                Seat::North,
                &avatar_id,
                &MagicEffect::GrantPowerTwoToAllyThisTurn,
            )
            .expect("Overpower ally choices")
            .iter()
            .any(|choice| matches!(
                &choice.ally,
                Some(UnitTarget::Minion {
                    instance_id,
                    seat: Seat::North,
                }) if *instance_id == unit_id
            ))
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one internal footprint proof keeps Land, Rubble, and Water branches together"
    )]
    fn bury_should_check_every_oversized_cell_and_treat_rubble_as_land() {
        let setup = |water: bool, rubble: bool| {
            let manifest = selfplay_manifest_with(31, |manifest| {
                for ordinal in 1..=50 {
                    manifest["cards"][format!("north-spell-{ordinal}")] = json!({
                        "burrowTargetMinionOrArtifact": true,
                        "cardType": "magic",
                        "manaCost": 0,
                        "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                    });
                }
                manifest["cards"]["south-spell-1"] = json!({
                    "attack": 1,
                    "cardType": "minion",
                    "defense": 10,
                    "manaCost": 0,
                    "occupiesSquareArea": 2,
                    "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                });
                if water {
                    for ordinal in 1..=30 {
                        manifest["cards"][format!("north-site-{ordinal}")]["elements"] =
                            json!(["water"]);
                    }
                }
            });
            let mut game = Game::from_manifest_json(&manifest).expect("valid oversized Bury game");
            let area = Cell::SQUARE_AREAS
                .into_iter()
                .find(|area| area[0] == Cell::parse("B3").expect("B3"))
                .expect("B3 footprint");
            let mut site_cards = Vec::new();
            let north = &mut game.position.players[seat_index(Seat::North)];
            while site_cards.len() < area.len() {
                site_cards.push(if north.hand_atlas.is_empty() {
                    north.atlas.remove(0)
                } else {
                    north.hand_atlas.remove(0)
                });
            }
            north.domain_established = true;
            north.mana = 10;
            for (cell, card) in area.into_iter().zip(site_cards) {
                game.position.sites[cell.index()] = Some(SitePosition {
                    card,
                    controller: Seat::North,
                });
            }
            if rubble {
                let cell = Cell::parse("B4").expect("B4");
                game.position.sites[cell.index()] = None;
                game.position.rubble[cell.index()] = Some(
                    identity_hash(&json!({ "fixture": "oversized-bury-rubble" }))
                        .expect("Rubble identity"),
                );
            }
            let target_card_id = CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|card| card.id == "south-spell-1")
                        .expect("target card"),
                )
                .expect("target card index"),
            );
            let target_id = IdentityHash::parse(
                "sha256:7777777777777777777777777777777777777777777777777777777777777777",
            )
            .expect("target identity");
            game.position.units.push(test_minion(
                target_card_id,
                target_id.as_str(),
                Seat::South,
                area[0],
                Some(area),
            ));
            game.position.active_seat = Seat::North;
            game.position.decision_seat = Seat::North;
            game.position.phase = Phase::Main;
            let action = game
                .legal_actions()
                .expect("oversized Bury actions")
                .into_iter()
                .find(|action| {
                    matches!(
                        &action.descriptor,
                        ActionDescriptor::CastMagic {
                            target: Some(UnitTarget::Minion { instance_id, .. }),
                            ..
                        } if *instance_id == target_id
                    )
                })
                .expect("oversized Bury target");
            (game, action, target_id)
        };

        let (mut land, land_action, land_target) = setup(false, true);
        let mut speculative = land.clone();
        speculative
            .apply_action(&land_action)
            .expect("speculative Rubble footprint Bury");
        let (land_outcomes, _) = land
            .apply_action_recorded(&land_action)
            .expect("Rubble footprint Bury");
        assert_eq!(speculative.position, land.position);
        assert_eq!(
            land_outcomes
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            [
                "magic-cast",
                "minion-burrowed",
                "minion-died",
                "magic-resolved",
            ]
        );
        assert!(
            land.position
                .units
                .iter()
                .all(|unit| unit.card.instance_id != land_target)
        );

        let (mut water, water_action, water_target) = setup(true, false);
        let before = water.position.clone();
        let (water_outcomes, _) = water
            .apply_action_recorded(&water_action)
            .expect("Water footprint Bury");
        assert_eq!(
            water_outcomes
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            ["magic-cast", "magic-resolved"]
        );
        let unit = water
            .position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == water_target)
            .expect("blocked giant");
        assert_eq!(unit.region, Region::Surface);
        assert_eq!(unit.occupied_cells, before.units[0].occupied_cells);
    }

    #[test]
    fn water_replacement_should_preserve_the_underwater_deathrite_source_region() {
        let manifest = selfplay_manifest_with(31, |manifest| {
            manifest["cards"]["north-spell-1"] = json!({
                "attack": 1,
                "burrowing": true,
                "cardType": "minion",
                "deathriteDamageEachUnitHere": 1,
                "defense": 1,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            });
            manifest["cards"]["north-spell-2"] = json!({
                "attack": 1,
                "cardType": "minion",
                "defense": 10,
                "manaCost": 0,
                "submerge": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            });
            for ordinal in 1..=30 {
                manifest["cards"][format!("north-site-{ordinal}")]["elements"] = json!(["water"]);
            }
        });
        let mut game = Game::from_manifest_json(&manifest).expect("valid region fixture");
        let cell = Cell::parse("C4").expect("C4");
        let site = game.position.players[seat_index(Seat::North)]
            .hand_atlas
            .remove(0);
        game.position.sites[cell.index()] = Some(SitePosition {
            card: site,
            controller: Seat::North,
        });
        let card_id = |name: &str| {
            CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|card| card.id == name)
                        .expect("fixture card"),
                )
                .expect("fixture card index"),
            )
        };
        let source_id = "sha256:8888888888888888888888888888888888888888888888888888888888888888";
        let target_id = "sha256:9999999999999999999999999999999999999999999999999999999999999999";
        let mut source = test_minion(card_id("north-spell-1"), source_id, Seat::North, cell, None);
        source.region = Region::Underground;
        let mut target = test_minion(card_id("north-spell-2"), target_id, Seat::North, cell, None);
        target.region = Region::Underground;
        game.position.units = vec![source, target];

        game.move_underground_units_to_underwater(cell);
        let mut events = Vec::new();
        game.settle_lower_region_minion_deaths(&mut OutcomeLog::Record(&mut events))
            .expect("underwater settlement");

        assert!(game.position.units.iter().all(|unit| {
            unit.card.instance_id.as_str() != source_id
                && unit.region == Region::Underwater
                && unit.damage == 1
        }));
        assert!(events.iter().any(|(event_type, payload)| {
            event_type == "deathrite-damage-allocated" && payload["targetInstanceId"] == target_id
        }));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one direct continuation proof keeps Water replacement, ordered Deathrites, and Genesis together"
    )]
    fn water_replacement_should_resume_site_genesis_after_ordered_deathrites() {
        let manifest = selfplay_manifest_with(31, |manifest| {
            for ordinal in 1..=30 {
                manifest["cards"][format!("north-site-{ordinal}")] = json!({
                    "cardType": "site",
                    "elements": ["water"],
                    "genesisGainMana": 1,
                });
            }
            for ordinal in 1..=2 {
                manifest["cards"][format!("north-spell-{ordinal}")] = json!({
                    "attack": 1,
                    "burrowing": true,
                    "cardType": "minion",
                    "deathriteDrawSite": true,
                    "defense": 1,
                    "manaCost": 0,
                    "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                });
            }
        });
        let mut game = Game::from_manifest_json(&manifest).expect("valid continuation fixture");
        let cell = Cell::parse("C4").expect("C4");
        let rubble_id =
            identity_hash(&json!({ "fixture": "water-genesis-rubble" })).expect("Rubble identity");
        game.position.rubble[cell.index()] = Some(rubble_id);
        let north = &mut game.position.players[seat_index(Seat::North)];
        north.avatar.location = cell;
        north.avatar.tapped = false;
        north.domain_established = false;
        let card_id = |name: &str| {
            CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|card| card.id == name)
                        .expect("fixture card"),
                )
                .expect("fixture card index"),
            )
        };
        let source_ids = [
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ];
        game.position.units = source_ids
            .iter()
            .enumerate()
            .map(|(index, instance_id)| {
                let mut unit = test_minion(
                    card_id(&format!("north-spell-{}", index + 1)),
                    instance_id,
                    Seat::North,
                    cell,
                    None,
                );
                unit.region = Region::Underground;
                unit
            })
            .collect();
        game.position.active_seat = Seat::North;
        game.position.decision_seat = Seat::North;
        game.position.phase = Phase::Main;
        let play = game
            .legal_actions()
            .expect("Water site actions")
            .into_iter()
            .find(|action| {
                matches!(
                    action.descriptor,
                    ActionDescriptor::PlaySite { cell: target, .. } if target == cell
                )
            })
            .expect("replace Rubble with Water site");
        let (placement, _) = game
            .apply_action_recorded(&play)
            .expect("Water replacement");
        assert_eq!(
            placement
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            ["rubble-replaced", "site-played"]
        );
        assert_eq!(game.position.phase, Phase::DeathriteOrder);
        assert_eq!(game.position.players[seat_index(Seat::North)].mana, 1);
        assert_eq!(
            game.authoritative_state()["pendingDeathrites"]["continuation"]["kind"],
            "site-genesis"
        );
        assert_eq!(
            game.authoritative_state()["pendingDeathrites"]["continuation"]["genesisGainMana"],
            1
        );

        let order = game
            .legal_actions()
            .expect("Deathrite order")
            .into_iter()
            .find(|action| {
                matches!(
                    &action.descriptor,
                    ActionDescriptor::OrderDeathrites { source_instance_id }
                        if source_instance_id.as_str() == source_ids[0]
                )
            })
            .expect("canonical first Deathrite source");
        let (resolved, _) = game
            .apply_action_recorded(&order)
            .expect("ordered Deathrites and Genesis");
        assert_eq!(
            resolved
                .iter()
                .map(|(event_type, _)| event_type.as_str())
                .collect::<Vec<_>>(),
            [
                "deathrite-order-committed",
                "site-drawn",
                "site-drawn",
                "minion-died",
                "minion-died",
                "mana-gained",
            ]
        );
        assert_eq!(game.position.phase, Phase::Main);
        assert_eq!(game.position.players[seat_index(Seat::North)].mana, 2);
    }

    #[test]
    fn oversized_attack_should_choose_the_lowest_shared_contested_cell() {
        let manifest = selfplay_manifest_with(31, |manifest| {
            for card_id in ["north-spell-1", "south-spell-1"] {
                manifest["cards"][card_id] = json!({
                    "attack": 2,
                    "cardType": "minion",
                    "defense": 4,
                    "manaCost": 0,
                    "occupiesSquareArea": 2,
                    "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                });
            }
        });
        let mut game = Game::from_manifest_json(&manifest).expect("valid oversized manifest");
        let card_id = |id: &str| {
            CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|card| card.id == id)
                        .expect("fixture card"),
                )
                .expect("fixture card index"),
            )
        };
        let attacker_id = IdentityHash::parse(
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        )
        .expect("attacker identity");
        let target_id = IdentityHash::parse(
            "sha256:2222222222222222222222222222222222222222222222222222222222222222",
        )
        .expect("target identity");
        game.position.units = vec![
            test_minion(
                card_id("north-spell-1"),
                attacker_id.as_str(),
                Seat::North,
                Cell::SQUARE_AREAS[4][0],
                Some(Cell::SQUARE_AREAS[4]),
            ),
            test_minion(
                card_id("south-spell-1"),
                target_id.as_str(),
                Seat::South,
                Cell::SQUARE_AREAS[5][0],
                Some(Cell::SQUARE_AREAS[5]),
            ),
        ];
        game.position.active_seat = Seat::North;
        game.position.decision_seat = Seat::North;
        game.position.phase = Phase::Attack;
        game.position.pending_combat = Some(PendingCombat {
            allocations: Vec::new(),
            attacker_instance_id: attacker_id,
            attacker_kind: UnitKind::Minion,
            attacking_seat: Seat::North,
            cell: Cell::parse("B2").expect("anchor"),
            combatants: Vec::new(),
            defenders: Vec::new(),
            original_target: None,
            region: Region::Surface,
            target_removed: false,
        });
        let target = CombatTarget::Minion {
            instance_id: target_id,
            seat: Seat::South,
        };
        assert_eq!(
            game.attack_targets().expect("attack targets"),
            std::slice::from_ref(&target)
        );

        let mut outcomes = Vec::new();
        game.apply_declare_attack_action(
            Seat::North,
            &target,
            &mut OutcomeLog::Record(&mut outcomes),
        )
        .expect("declare oversized attack");
        let contested = Cell::parse("B3").expect("lowest shared cell");
        assert_eq!(
            game.position
                .pending_combat
                .as_ref()
                .expect("pending combat")
                .cell,
            contested
        );
        assert_eq!(outcomes[0].1["cell"], json!(contested));
    }

    #[test]
    fn scent_hound_events_should_follow_source_identity_before_target_order() {
        let manifest = selfplay_manifest_with(31, |manifest| {
            for card_id in ["north-spell-1", "north-spell-2"] {
                manifest["cards"][card_id]["nearbyEnemiesPermanentlyLoseStealth"] = json!(true);
            }
            for card_id in ["south-spell-1", "south-spell-2"] {
                manifest["cards"][card_id]["stealth"] = json!(true);
            }
        });
        let mut game = Game::from_manifest_json(&manifest).expect("valid Scent Hound manifest");
        let card_id = |id: &str| {
            CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|card| card.id == id)
                        .expect("fixture card"),
                )
                .expect("fixture card index"),
            )
        };
        let unit =
            |card_id, instance_id: &str, controller, location: &str, stealthed| UnitPosition {
                card: CardInstance {
                    card_id,
                    instance_id: IdentityHash::parse(instance_id).expect("fixture identity"),
                    owner: controller,
                    source: CardSource::Spellbook,
                },
                carried_lance_count: 0,
                controller,
                damage: 0,
                disable_effects: Vec::new(),
                disabled_until_damaged: false,
                last_dropped_artifacts_turn: None,
                last_interacted_turn: None,
                last_picked_up_artifacts_turn: None,
                location: Cell::parse(location).expect("fixture cell"),
                occupied_cells: None,
                region: Region::Surface,
                stealthed,
                summoning_sickness: false,
                tapped: false,
                temporary_charge_sources: Vec::new(),
                temporary_power_sources: Vec::new(),
                warded: false,
            };
        game.position.units = vec![
            unit(
                card_id("south-spell-1"),
                "sha256:3333333333333333333333333333333333333333333333333333333333333333",
                Seat::South,
                "D3",
                true,
            ),
            unit(
                card_id("south-spell-2"),
                "sha256:4444444444444444444444444444444444444444444444444444444444444444",
                Seat::South,
                "A2",
                true,
            ),
            unit(
                card_id("north-spell-1"),
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                Seat::North,
                "A1",
                false,
            ),
            unit(
                card_id("north-spell-2"),
                "sha256:2222222222222222222222222222222222222222222222222222222222222222",
                Seat::North,
                "D4",
                false,
            ),
        ];
        let mut outcomes = Vec::new();
        game.settle_nearby_enemy_stealth(&mut OutcomeLog::Record(&mut outcomes));

        assert_eq!(
            outcomes,
            vec![
                (
                    "stealth-lost".to_owned(),
                    json!({
                        "instanceId": "sha256:4444444444444444444444444444444444444444444444444444444444444444",
                        "seat": "south",
                        "sourceInstanceId": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                    }),
                ),
                (
                    "stealth-lost".to_owned(),
                    json!({
                        "instanceId": "sha256:3333333333333333333333333333333333333333333333333333333333333333",
                        "seat": "south",
                        "sourceInstanceId": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
                    }),
                ),
            ]
        );
    }

    /// One crater board: a Water target at C2, both Avatars inside the grid, and one South minion
    /// per interesting distance, footprint, Ward, Stealth, and Void case.
    #[expect(
        clippy::too_many_lines,
        reason = "one shared crater board keeps every distance, terrain, and status case visible together"
    )]
    fn craterize_fixture(protected: bool) -> (Game, BTreeMap<&'static str, IdentityHash>) {
        let manifest = selfplay_manifest_with(197, |manifest| {
            for ordinal in 1..=50 {
                manifest["cards"][format!("north-spell-{ordinal}")] = json!({
                    "cardType": "magic",
                    "damageUnitsAboveAndBelowTargetSiteByManhattanDistance": [10, 7, 4, 2, 1],
                    "destroyTargetSite": true,
                    "discardSiteAsAdditionalCost": true,
                    "manaCost": 8,
                    "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
                });
            }
            manifest["cards"]["south-site-1"]["elements"] = json!(["water"]);
            if protected {
                manifest["cards"]["south-site-1"]["cannotBeMovedDestroyedOrModified"] = json!(true);
            }
            for (ordinal, facts) in [
                (1, json!({ "burrowing": true, "submerge": true })),
                (2, json!({ "burrowing": true })),
                (3, json!({})),
                (4, json!({ "stealth": true })),
                (5, json!({ "ward": true })),
                (6, json!({ "occupiesSquareArea": 2 })),
                (7, json!({ "voidwalk": true })),
            ] {
                let card = &mut manifest["cards"][format!("south-spell-{ordinal}")];
                card["defense"] = json!(40);
                card["manaCost"] = json!(0);
                for (field, value) in facts.as_object().expect("fixture minion facts") {
                    card[field.as_str()] = value.clone();
                }
            }
        });
        let mut game = Game::from_manifest_json(&manifest).expect("valid Craterize manifest");
        let card_id = |name: &str| {
            CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|card| card.id == name)
                        .expect("fixture card"),
                )
                .expect("fixture card index"),
            )
        };
        let north_land = card_id("north-site-1");
        let south_land = card_id("south-site-2");
        let water = card_id("south-site-1");
        let minions: [CardId; 7] =
            std::array::from_fn(|index| card_id(&format!("south-spell-{}", index + 1)));

        game.position.sites = std::array::from_fn(|_| None);
        game.position.rubble = std::array::from_fn(|_| None);
        for (cell, card, owner) in [
            ("B1", south_land, Seat::South),
            ("B2", south_land, Seat::South),
            ("C1", south_land, Seat::South),
            ("C2", water, Seat::South),
            ("C3", south_land, Seat::South),
            ("C4", north_land, Seat::North),
            ("D3", south_land, Seat::South),
            ("D4", north_land, Seat::North),
            ("E3", south_land, Seat::South),
            ("E4", south_land, Seat::South),
        ] {
            let cell = Cell::parse(cell).expect("fixture cell");
            game.position.sites[cell.index()] = Some(SitePosition {
                card: CardInstance {
                    card_id: card,
                    instance_id: identity_hash(
                        &json!({ "cell": cell, "fixture": "craterize-site" }),
                    )
                    .expect("fixture site identity"),
                    owner,
                    source: CardSource::Atlas,
                },
                controller: owner,
            });
        }

        let mut identities = BTreeMap::new();
        game.position.units = Vec::new();
        for (name, index, cell, region) in [
            ("center", 0, "C2", Region::Underwater),
            ("seven", 1, "C3", Region::Underground),
            ("four", 2, "D3", Region::Surface),
            ("two", 3, "E3", Region::Surface),
            ("one", 4, "E4", Region::Surface),
            ("oversized", 5, "B1", Region::Surface),
            ("void", 6, "A4", Region::Void),
        ] {
            let instance_id =
                identity_hash(&json!({ "fixture": name, "kind": "craterize-minion" }))
                    .expect("fixture minion identity");
            let mut unit = test_minion(
                minions[index],
                instance_id.as_str(),
                Seat::South,
                Cell::parse(cell).expect("fixture cell"),
                (name == "oversized").then_some(Cell::SQUARE_AREAS[3]),
            );
            unit.region = region;
            unit.stealthed = name == "two";
            unit.warded = name == "one";
            game.position.units.push(unit);
            identities.insert(name, instance_id);
        }

        game.position.active_seat = Seat::North;
        game.position.decision_seat = Seat::North;
        game.position.phase = Phase::Main;
        game.position.players[seat_index(Seat::South)]
            .avatar
            .location = Cell::parse("C3").expect("C3");
        let north = &mut game.position.players[seat_index(Seat::North)];
        north.avatar.location = Cell::parse("C4").expect("C4");
        north.domain_established = true;
        north.mana = 8;
        (game, identities)
    }

    fn craterize_casts(game: &Game, card_instance_id: &IdentityHash) -> Vec<IssuedAction> {
        game.legal_actions()
            .expect("Craterize actions")
            .into_iter()
            .filter(|action| match &action.descriptor {
                ActionDescriptor::CastMagic {
                    card_instance_id: candidate,
                    ..
                } => *candidate == *card_instance_id,
                _ => false,
            })
            .collect()
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one direct crater proof keeps the discard cost, protection, grid, and terrain branches together"
    )]
    fn craterize_should_discard_a_site_destroy_its_target_and_apply_its_damage_grid() {
        let (base, identities) = craterize_fixture(false);
        let c2 = Cell::parse("C2").expect("C2");
        let craterize = base.position.players[seat_index(Seat::North)].hand_spellbook[0]
            .instance_id
            .clone();
        let target_site_instance_id = base.position.sites[c2.index()]
            .as_ref()
            .expect("Water target site")
            .card
            .instance_id
            .clone();
        let discard_count = base.position.players[seat_index(Seat::North)]
            .hand_atlas
            .len();
        assert!(discard_count > 1);

        // The additional cost is mandatory, so an empty Atlas hand offers no cast at all.
        let mut costless = base.clone();
        costless.position.players[seat_index(Seat::North)]
            .hand_atlas
            .clear();
        assert!(craterize_casts(&costless, &craterize).is_empty());

        let casts = craterize_casts(&base, &craterize);
        let target_cells = casts
            .iter()
            .filter_map(|action| match &action.descriptor {
                ActionDescriptor::CastMagic {
                    target_location, ..
                } => target_location.map(|location| location.cell),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(target_cells.len(), 10);
        let target_casts: Vec<_> = casts
            .iter()
            .filter(|action| {
                matches!(
                    &action.descriptor,
                    ActionDescriptor::CastMagic {
                        target_site_instance_id: Some(site),
                        ..
                    } if *site == target_site_instance_id
                )
            })
            .collect();
        assert_eq!(target_casts.len(), discard_count);
        let cast = target_casts[0].clone();
        let ActionDescriptor::CastMagic {
            discard_site_instance_id: Some(discarded_site_instance_id),
            ..
        } = &cast.descriptor
        else {
            unreachable!("Craterize issues its discard cost");
        };
        let discarded_site_instance_id = discarded_site_instance_id.clone();

        // Only an Atlas card in hand pays the cost; the target site itself is not a legal payment.
        let mut forged = cast.clone();
        let ActionDescriptor::CastMagic {
            discard_site_instance_id,
            ..
        } = &mut forged.descriptor
        else {
            unreachable!("filtered Craterize cast");
        };
        *discard_site_instance_id = Some(target_site_instance_id.clone());
        let mut forgery = base.clone();
        let before_forgery = forgery.authoritative_state();
        assert!(matches!(
            forgery.apply_action(&forged),
            Err(GameError::IllegalAction)
        ));
        assert_eq!(forgery.authoritative_state(), before_forgery);

        let (protected_base, protected_identities) = craterize_fixture(true);
        let protected_site = protected_base.position.sites[c2.index()].clone();
        let protected_spell = protected_base.position.players[seat_index(Seat::North)]
            .hand_spellbook[0]
            .instance_id
            .clone();
        let protected_cast = craterize_casts(&protected_base, &protected_spell)
            .into_iter()
            .find(|action| {
                matches!(
                    &action.descriptor,
                    ActionDescriptor::CastMagic { target_location: Some(location), .. }
                        if location.cell == c2
                )
            })
            .expect("protected Craterize cast");
        let mut protected = protected_base;
        let (protected_events, protected_random) = protected
            .apply_action_recorded(&protected_cast)
            .expect("protected Craterize cast");
        assert!(protected_random.is_empty());
        let protected_types: Vec<_> = protected_events
            .iter()
            .map(|(event_type, _)| event_type.as_str())
            .collect();
        assert!(protected_types.contains(&"site-destruction-prevented"));
        assert!(!protected_types.contains(&"rubble-created"));
        assert_eq!(protected.position.sites[c2.index()], protected_site);
        let protected_center = protected
            .position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == protected_identities["center"])
            .expect("protected crater centre");
        assert_eq!(
            (protected_center.damage, protected_center.region),
            (10, Region::Underwater)
        );

        let mut destroyed = base.clone();
        let (events, random_draws) = destroyed
            .apply_action_recorded(&cast)
            .expect("Craterize cast");
        assert!(random_draws.is_empty());
        let event_types: Vec<_> = events
            .iter()
            .map(|(event_type, _)| event_type.as_str())
            .collect();
        assert_eq!(
            &event_types[..3],
            ["card-discarded", "magic-cast", "site-destroyed"]
        );
        assert!(event_types.contains(&"rubble-created"));
        assert_eq!(event_types.last(), Some(&"magic-resolved"));
        assert_eq!(events[0].1["zone"], json!("atlas"));
        assert_eq!(
            events[0].1["instanceId"],
            json!(discarded_site_instance_id.as_str())
        );
        assert_eq!(
            events[1].1["discardSiteInstanceId"],
            json!(discarded_site_instance_id.as_str())
        );

        let allocated: BTreeMap<&str, u64> = events
            .iter()
            .filter(|(event_type, _)| event_type == "magic-damage-allocated")
            .map(|(_, payload)| {
                (
                    payload["targetInstanceId"]
                        .as_str()
                        .expect("allocated target"),
                    payload["amount"].as_u64().expect("allocated amount"),
                )
            })
            .collect();
        assert_eq!(
            ["center", "seven", "four", "two", "one", "oversized", "void",]
                .map(|name| allocated.get(identities[name].as_str()).copied()),
            [Some(10), Some(7), Some(4), Some(2), Some(1), Some(28), None,]
        );

        let settled = |name: &str| {
            destroyed
                .position
                .units
                .iter()
                .find(|unit| unit.card.instance_id == identities[name])
                .map(|unit| (unit.damage, unit.region, unit.stealthed, unit.warded))
        };
        assert_eq!(
            ["center", "seven", "four", "two", "one", "oversized", "void"].map(settled),
            [
                // The drained Water layer drops the submerged minion into its Burrowing layer.
                Some((10, Region::Underground, false, false)),
                Some((7, Region::Underground, false, false)),
                Some((4, Region::Surface, false, false)),
                Some((2, Region::Surface, true, false)),
                // A Ward absorbs the grid damage outright and breaks.
                Some((0, Region::Surface, false, false)),
                Some((28, Region::Surface, false, false)),
                Some((0, Region::Void, false, false)),
            ]
        );
        assert_eq!(
            (
                destroyed.position.players[seat_index(Seat::North)]
                    .avatar
                    .life,
                destroyed.position.players[seat_index(Seat::South)]
                    .avatar
                    .life,
                destroyed.position.players[seat_index(Seat::North)].mana,
            ),
            (16, 13, 0)
        );

        let north_cemetery = &destroyed.position.players[seat_index(Seat::North)].cemetery;
        assert!(
            north_cemetery
                .iter()
                .any(|card| card.instance_id == craterize)
        );
        assert!(
            north_cemetery
                .iter()
                .any(|card| card.instance_id == discarded_site_instance_id)
        );
        assert!(
            !destroyed.position.players[seat_index(Seat::North)]
                .hand_atlas
                .iter()
                .any(|card| card.instance_id == discarded_site_instance_id)
        );
        assert!(
            destroyed.position.players[seat_index(Seat::South)]
                .cemetery
                .iter()
                .any(|card| card.instance_id == target_site_instance_id)
        );
        assert!(destroyed.position.sites[c2.index()].is_none());
        assert_eq!(
            destroyed.position.rubble[c2.index()],
            Some(
                identity_hash(&json!({
                    "cell": c2,
                    "destroyedSiteInstanceId": target_site_instance_id,
                    "kind": "rubble",
                    "sourceInstanceId": craterize,
                }))
                .expect("deterministic Rubble identity")
            )
        );

        let mut repeated = base;
        let (repeated_events, repeated_random) = repeated
            .apply_action_recorded(&cast)
            .expect("repeated Craterize cast");
        assert_eq!(repeated_events, events);
        assert!(repeated_random.is_empty());
        assert_eq!(repeated.position, destroyed.position);
    }

    struct RaiseDeadFixture {
        game: Game,
        north_corpse: IdentityHash,
        raise_dead: IdentityHash,
        south_corpse: IdentityHash,
    }

    /// One Raise Dead board: sites only at C1 and C4, so free placement has to cross site
    /// control and cannot host a two-by-two footprint, plus one corpse in each cemetery.
    fn raise_dead_fixture(oversized: bool) -> RaiseDeadFixture {
        let manifest = selfplay_manifest_with(198, |manifest| {
            manifest["cards"]["north-spell-1"] = json!({
                "cardType": "magic",
                "manaCost": 0,
                "summonRandomMinionFromAnyCemetery": true,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            });
            // The corpse is unaffordable and off-threshold, so only a free placement can raise it.
            let corpse = &mut manifest["cards"]["south-spell-1"];
            corpse["defense"] = json!(3);
            corpse["manaCost"] = json!(9);
            corpse["thresholds"] = json!({ "air": 0, "earth": 0, "fire": 0, "water": 4 });
            // An oversized footprint cannot also carry a Genesis, so the branches split here.
            if oversized {
                corpse["occupiesSquareArea"] = json!(2);
            } else {
                corpse["genesisLoseControllerLife"] = json!(2);
            }
        });
        let mut game = Game::from_manifest_json(&manifest).expect("valid Raise Dead manifest");
        let card_id = |name: &str| {
            CardId(
                u16::try_from(
                    game.rules
                        .cards
                        .iter()
                        .position(|card| card.id == name)
                        .expect("fixture card"),
                )
                .expect("fixture card index"),
            )
        };
        let instance = |card_id: CardId, owner: Seat, source: CardSource, ordinal: usize| {
            card_instance(&game.rules, card_id, owner, source, ordinal).expect("fixture instance")
        };
        let raise_dead = instance(
            card_id("north-spell-1"),
            Seat::North,
            CardSource::Spellbook,
            500,
        );
        let south_corpse = instance(
            card_id("south-spell-1"),
            Seat::South,
            CardSource::Spellbook,
            501,
        );
        let north_corpse = instance(
            card_id("north-spell-2"),
            Seat::North,
            CardSource::Spellbook,
            502,
        );
        let sites = [
            (
                Cell::parse("C4").expect("C4"),
                Seat::North,
                instance(card_id("north-site-1"), Seat::North, CardSource::Atlas, 503),
            ),
            (
                Cell::parse("C1").expect("C1"),
                Seat::South,
                instance(card_id("south-site-1"), Seat::South, CardSource::Atlas, 504),
            ),
        ];

        game.position.sites = std::array::from_fn(|_| None);
        game.position.rubble = std::array::from_fn(|_| None);
        for (cell, controller, card) in sites {
            game.position.sites[cell.index()] = Some(SitePosition { card, controller });
        }
        game.position.units = Vec::new();
        game.position.active_seat = Seat::North;
        game.position.decision_seat = Seat::North;
        game.position.phase = Phase::Main;
        let south = &mut game.position.players[seat_index(Seat::South)];
        south.avatar.location = Cell::parse("C1").expect("C1");
        south.cemetery = vec![south_corpse.clone()];
        let north = &mut game.position.players[seat_index(Seat::North)];
        north.avatar.location = Cell::parse("C4").expect("C4");
        north.cemetery = vec![north_corpse.clone()];
        north.domain_established = true;
        north.hand_spellbook = vec![raise_dead.clone()];
        north.mana = 3;
        RaiseDeadFixture {
            game,
            north_corpse: north_corpse.instance_id,
            raise_dead: raise_dead.instance_id,
            south_corpse: south_corpse.instance_id,
        }
    }

    fn only_action(
        actions: &[IssuedAction],
        matches: impl Fn(&ActionDescriptor) -> bool,
    ) -> &IssuedAction {
        let mut found = actions.iter().filter(|action| matches(&action.descriptor));
        let action = found.next().expect("expected one issued action");
        assert!(found.next().is_none(), "expected exactly one issued action");
        action
    }

    fn raise_dead_cast(game: &Game) -> IssuedAction {
        only_action(
            &game.legal_actions().expect("Raise Dead actions"),
            |descriptor| matches!(descriptor, ActionDescriptor::CastMagic { .. }),
        )
        .clone()
    }

    fn event_types(events: &[(String, Value)]) -> Vec<&str> {
        events
            .iter()
            .map(|(event_type, _)| event_type.as_str())
            .collect()
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one direct Raise Dead proof keeps the empty pool, blocked placement, and free summon branches together"
    )]
    fn raise_dead_should_select_a_public_random_cemetery_minion_before_free_placement() {
        let base = raise_dead_fixture(false);
        let c1 = Cell::parse("C1").expect("C1");
        let c4 = Cell::parse("C4").expect("C4");

        // An empty cemetery pool resolves the Magic outright and never touches the PRNG.
        let mut empty = base.game.clone();
        empty.position.players[seat_index(Seat::North)].cemetery = Vec::new();
        empty.position.players[seat_index(Seat::South)].cemetery = Vec::new();
        let empty_cast = raise_dead_cast(&empty);
        let (empty_events, empty_random) = empty
            .apply_action_recorded(&empty_cast)
            .expect("empty cemetery Raise Dead cast");
        assert_eq!(event_types(&empty_events), ["magic-cast", "magic-resolved"]);
        assert!(empty_random.is_empty());
        assert_eq!(empty.position.phase, Phase::Main);
        assert!(empty.position.pending_cemetery_summon.is_none());

        // Both cemeteries feed one public draw over the shared candidate pool.
        let mut both = base.game.clone();
        let both_cast = raise_dead_cast(&both);
        let (both_events, both_random) = both
            .apply_action_recorded(&both_cast)
            .expect("two candidate Raise Dead cast");
        assert_eq!(
            event_types(&both_events),
            ["magic-cast", "dead-minion-selected"]
        );
        assert_eq!(both_random.len(), 1);
        assert_eq!(both_random[0].purpose, "magic_random_dead_minion");
        assert_eq!(both_random[0].domain.kind, "dead_minion_instance_candidate");
        assert_eq!(both_random[0].domain.exclusive_maximum, 2);
        assert_eq!(both.position.phase, Phase::CemeterySummon);
        let selected = both_events[1].1["instanceId"]
            .as_str()
            .expect("selected corpse")
            .to_owned();
        assert!(
            selected == base.north_corpse.as_str() || selected == base.south_corpse.as_str(),
            "the draw stays inside the cemetery pool"
        );
        // Selection alone moves nothing: both corpses wait in their cemeteries.
        assert_eq!(
            both.position.players[seat_index(Seat::North)]
                .cemetery
                .len(),
            2,
        );
        assert_eq!(
            both.position.players[seat_index(Seat::South)]
                .cemetery
                .len(),
            1,
        );

        // A footprint with nowhere to land fails its summon instead of silently vanishing.
        let oversized = raise_dead_fixture(true);
        let mut blocked = oversized.game;
        blocked.position.players[seat_index(Seat::North)].cemetery = Vec::new();
        let blocked_cast = raise_dead_cast(&blocked);
        let (blocked_events, blocked_random) = blocked
            .apply_action_recorded(&blocked_cast)
            .expect("blocked Raise Dead cast");
        assert_eq!(
            event_types(&blocked_events),
            [
                "magic-cast",
                "dead-minion-selected",
                "minion-summon-failed",
                "magic-resolved",
            ]
        );
        assert_eq!(blocked_random.len(), 1);
        assert_eq!(blocked_events[2].1["reason"], json!("no-legal-location"));
        assert_eq!(
            blocked_events[2].1["instanceId"],
            json!(oversized.south_corpse.as_str())
        );
        assert_eq!(blocked.position.phase, Phase::Main);
        assert!(blocked.position.pending_cemetery_summon.is_none());
        assert_eq!(
            blocked.position.players[seat_index(Seat::South)].cemetery[0].instance_id,
            oversized.south_corpse
        );

        // One candidate proves the placement itself: free, anywhere, and enemy owned.
        let mut game = base.game.clone();
        game.position.players[seat_index(Seat::North)].cemetery = Vec::new();
        let cast = raise_dead_cast(&game);
        let (cast_events, cast_random) = game
            .apply_action_recorded(&cast)
            .expect("single candidate Raise Dead cast");
        assert_eq!(
            event_types(&cast_events),
            ["magic-cast", "dead-minion-selected"]
        );
        assert_eq!(cast_random.len(), 1);
        assert_eq!(cast_random[0].domain.exclusive_maximum, 1);
        assert_eq!(
            cast_events[1].1,
            json!({
                "cardId": "south-spell-1",
                "instanceId": base.south_corpse,
                "owner": "south",
                "seat": "north",
                "sourceInstanceId": base.raise_dead,
            })
        );
        assert_eq!(game.position.phase, Phase::CemeterySummon);
        assert_eq!(
            game.authoritative_state()["pendingCemeterySummon"],
            json!({
                "cardInstanceId": base.south_corpse,
                "cardOwner": "south",
                "casterInstanceId": game.position.players[seat_index(Seat::North)]
                    .avatar
                    .card
                    .instance_id,
                "seat": "north",
                "sourceMagicInstanceId": base.raise_dead,
            })
        );

        let placements = game.legal_actions().expect("free placement actions");
        assert!(!placements.is_empty());
        assert!(placements.iter().all(|action| matches!(
            &action.descriptor,
            ActionDescriptor::SummonMinion {
                card_instance_id,
                mana_cost: 0,
                payment_mode: None,
                sacrificed_minion_instance_ids: None,
                ..
            } if *card_instance_id == base.south_corpse
        )));
        let cells: BTreeSet<_> = placements
            .iter()
            .filter_map(|action| match &action.descriptor {
                ActionDescriptor::SummonMinion { cell, .. } => Some(*cell),
                _ => None,
            })
            .collect();
        assert_eq!(cells, BTreeSet::from([c1, c4]));

        // Only engine-issued cells place the corpse; an empty cell is not a legal destination.
        let placement = only_action(&placements, |descriptor| {
            matches!(descriptor, ActionDescriptor::SummonMinion { cell, .. } if *cell == c1)
        })
        .clone();
        let mut forged = placement.clone();
        let ActionDescriptor::SummonMinion { cell, .. } = &mut forged.descriptor else {
            unreachable!("filtered free placement");
        };
        *cell = Cell::parse("A1").expect("A1");
        let mut forgery = game.clone();
        let before_forgery = forgery.authoritative_state();
        assert!(matches!(
            forgery.apply_action(&forged),
            Err(GameError::IllegalAction)
        ));
        assert_eq!(forgery.authoritative_state(), before_forgery);

        let mut placed = game.clone();
        let (placed_events, placed_random) = placed
            .apply_action_recorded(&placement)
            .expect("free cemetery placement");
        assert!(placed_random.is_empty());
        assert_eq!(
            event_types(&placed_events),
            ["minion-summoned", "avatar-life-lost", "magic-resolved"]
        );
        assert_eq!(placed_events[0].1["manaPaid"], json!(0));
        assert_eq!(placed_events[0].1["owner"], json!("south"));
        assert_eq!(
            placed_events[0].1["sourceInstanceId"],
            json!(base.raise_dead.as_str())
        );
        assert_eq!(
            placed_events[2].1["instanceId"],
            json!(base.raise_dead.as_str())
        );
        let raised = placed
            .position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == base.south_corpse)
            .expect("raised minion");
        assert_eq!(
            (
                raised.card.owner,
                raised.controller,
                raised.location,
                raised.region,
                raised.summoning_sickness,
            ),
            (Seat::South, Seat::North, c1, Region::Surface, true)
        );
        assert_eq!(placed.position.phase, Phase::Main);
        assert!(placed.position.pending_cemetery_summon.is_none());
        assert!(
            placed
                .authoritative_state()
                .get("pendingCemeterySummon")
                .is_none()
        );
        // The Genesis bills the raising controller, and the nine-mana corpse still costs nothing.
        assert_eq!(
            (
                placed.position.players[seat_index(Seat::North)].avatar.life,
                placed.position.players[seat_index(Seat::North)].mana,
            ),
            (18, 3)
        );
        assert!(
            placed.position.players[seat_index(Seat::South)]
                .cemetery
                .is_empty()
        );

        let mut repeated = game;
        let (repeated_events, repeated_random) = repeated
            .apply_action_recorded(&placement)
            .expect("repeated free cemetery placement");
        assert_eq!(repeated_events, placed_events);
        assert!(repeated_random.is_empty());
        assert_eq!(repeated.position, placed.position);
    }
}
