//! Compact game setup and opening-hand decisions.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::action::{
    ActionDescriptor, CombatTarget, DeckZone, GenesisDamageChoice, GenesisSpellChoice,
    GenesisTokenChoice, ProjectileDirection, RangedStepChoice, UnitTarget, compare_canonical,
};
use crate::board::{Cell, Location, Region, SquareArea, translated_square};
use crate::canonical::{CanonicalError, IdentityHash, identity_hash};
use crate::contract::{LegalAction, Seat, opaque_action_id};
use crate::facts::{
    AvatarFacts, BasicMovementRestriction, CardFacts, DamagePrevention, Element, EndTurnStealth,
    FactError, MagicEffect, MagicFacts, MinionFacts, MinionGenesis, RequiredCastRegion, SiteFacts,
    Thresholds, parse_card_definition, validate_identifier,
};
use crate::prng::PrngState;

const ENGINE_VERSION: &str = "sorcery-core-v1";
const MAX_DECK_CARDS: usize = 200;

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
    decision_seat: Seat,
    pending_basic_movement: PendingField<PendingBasicMovement>,
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
    last_interacted_turn: Option<u64>,
    life: u16,
    location: Cell,
    tapped: bool,
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
    last_interacted_turn: Option<u64>,
    location: Cell,
    occupied_cells: Option<SquareArea>,
    region: Region,
    stealthed: bool,
    summoning_sickness: bool,
    tapped: bool,
    warded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DisableEffect {
    expires_at_seat: Seat,
    source_instance_id: IdentityHash,
}

type MagicChoice = (Option<IdentityHash>, Option<UnitTarget>);

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
enum DeathriteContinuation {
    EndTurn(EndTurnContinuation),
    FirstStrike(FirstStrikeContinuation),
    SiteGenesis(SiteGenesisContinuation),
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
struct MovementProfile {
    airborne: bool,
    connects_top_bottom: bool,
    maximum_cost: Option<usize>,
    moving_minion: bool,
    occupied_cells: Option<SquareArea>,
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
        CardFacts::Artifact(_) => Some("cardType:artifact"),
        CardFacts::Aura(_) => Some("cardType:aura"),
        CardFacts::Magic(facts) => unsupported_selfplay_magic(facts),
        CardFacts::Minion(facts) => unsupported_selfplay_minion(facts),
        CardFacts::Site(facts) => unsupported_selfplay_site(facts),
    }
}

fn unsupported_selfplay_magic(facts: &MagicFacts) -> Option<&'static str> {
    unsupported_magic_effect(&facts.effect)
}

fn unsupported_magic_effect(effect: &MagicEffect) -> Option<&'static str> {
    match effect {
        MagicEffect::HealController(_)
        | MagicEffect::BurrowTargetMinionOrArtifact
        | MagicEffect::ReturnMinionFromOwnCemetery
        | MagicEffect::DamageTargetUnit { .. }
        | MagicEffect::DisableTargetNearbyMinionUntilNextTurn
        | MagicEffect::SummonTokenToEachControlledSiteBorderingEnemySite(_) => None,
        MagicEffect::BurrowAllMinionsAndArtifactsAtTargetLandSite => {
            Some("burrowAllMinionsAndArtifactsAtTargetLandSite")
        }
        MagicEffect::DamageChainNearbyUnits => Some("damageChainNearbyUnits"),
        MagicEffect::DamageEachAbovegroundMinionOne => Some("damageEachAbovegroundMinionOne"),
        MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(_) => {
            Some("damageEachUnitAtLocationWithinTwoSteps")
        }
        MagicEffect::DamageRandomUnitAtLocation(_) => Some("damageRandomUnitAtLocation"),
        MagicEffect::DestroyTargetSiteWithDamageGrid(_) => Some("destroyTargetSiteWithDamageGrid"),
        MagicEffect::FightAllyWithAdjacentEnemy => Some("fightAllyWithAdjacentEnemy"),
        MagicEffect::GainControlOfTargetNearbyMinion => Some("gainControlOfTargetNearbyMinion"),
        MagicEffect::GrantChargeToAllyThisTurn => Some("grantChargeToAllyThisTurn"),
        MagicEffect::GrantPowerTwoToAllyThisTurn => Some("grantPowerTwoToAllyThisTurn"),
        MagicEffect::KillTargetWoundedMinion => Some("killTargetWoundedMinion"),
        MagicEffect::LeapAttackAlly => Some("leapAttackAlly"),
        MagicEffect::LureEnemyMinionOneStepCloser => Some("lureEnemyMinionOneStepCloser"),
        MagicEffect::SubmergeTargetMinion => Some("submergeTargetMinion"),
        MagicEffect::SummonRandomMinionFromAnyCemetery => Some("summonRandomMinionFromAnyCemetery"),
        MagicEffect::TeleportAllyToTargetSite => Some("teleportAllyToTargetSite"),
        MagicEffect::TeleportNearbyAllyThenDrawCard => Some("teleportNearbyAllyThenDrawCard"),
    }
}

fn unsupported_selfplay_minion(facts: &MinionFacts) -> Option<&'static str> {
    account_for_selfplay_minion_fields(facts);
    if facts.alternative_summon_payment.is_some() {
        Some("alternativeSummonPayment")
    } else if facts.at_start_of_controller_turn_teleport_to_random_site_or_void {
        Some("atStartOfControllerTurnTeleportToRandomSiteOrVoid")
    } else if facts.burrowing {
        Some("burrowing")
    } else if facts
        .discard_spell_to_damage_random_other_unit_here
        .is_some()
    {
        Some("discardSpellToDamageRandomOtherUnitHere")
    } else if facts.end_turn_stealth == Some(EndTurnStealth::IfNoEnemiesNearby) {
        Some("gainStealthAtEndOfTurnIfNoEnemiesNearby")
    } else if facts.gains_power_ranged_and_spellcaster_atop_tower {
        Some("gainsPowerRangedAndSpellcasterAtopTower")
    } else if let Some(field) = unsupported_selfplay_minion_genesis(facts.genesis) {
        Some(field)
    } else if facts.must_be_cast_to_outer_column {
        Some("mustBeCastToOuterColumn")
    } else if facts.shoots_drag_projectile {
        Some("shootsDragProjectile")
    } else if facts.submerge {
        Some("submerge")
    } else if facts.tap_to_damage_each_unit_at_adjacent_location {
        Some("tapToDamageEachUnitAtAdjacentLocation")
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
    if facts.cannot_be_moved_destroyed_or_modified {
        Some("cannotBeMovedDestroyedOrModified")
    } else if facts.connects_burrowed_allies {
        Some("connectsBurrowedAllies")
    } else if facts.fly_to_nearby_void_once_per_turn_at_air_threshold {
        Some("flyToNearbyVoidOncePerTurnAtAirThreshold")
    } else if facts.genesis_immobilize_nearby_until_next_turn {
        Some("genesisImmobilizeNearbyUntilNextTurn")
    } else if facts.is_tower {
        Some("isTower")
    } else if facts.minions_here_gain_voidwalk_until_leaving_void {
        Some("minionsHereGainVoidwalkUntilLeavingVoid")
    } else if facts
        .prevents_units_with_power_at_least_from_entering
        .is_some()
    {
        Some("preventsUnitsWithPowerAtLeastFromEntering")
    } else if facts.sacrifice_to_destroy_nearby_site {
        Some("sacrificeToDestroyNearbySite")
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
            | MinionGenesis::MayDamageTargetAdjacentUnitTwo,
        ) => None,
        Some(MinionGenesis::StrikeEachEnemyHere) => Some("genesisStrikeEachEnemyHere"),
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
                decision_seat: Seat::North,
                pending_basic_movement: PendingField::Absent,
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
            connects_top_bottom: facts.connects_top_bottom,
            maximum_cost: (!facts.immobile).then_some(1),
            moving_minion: true,
            occupied_cells: unit.occupied_cells,
            restriction: facts.movement_restriction,
            seat: pending.seat,
        };
        for path in self
            .surface_movement_paths(unit.location, profile)
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
                .surface_movement_paths(start, profile)
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
                || unit.summoning_sickness && !facts.charge
                || attacker_airborne && !facts.airborne && !facts.ranged
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
                    connects_top_bottom: false,
                    maximum_cost: Some(1),
                    moving_minion: false,
                    occupied_cells: None,
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
            if unit.summoning_sickness && !facts.charge {
                continue;
            }
            candidates.push((
                UnitKind::Minion,
                unit.card.instance_id.clone(),
                unit.location,
                MovementProfile {
                    airborne: facts.airborne,
                    connects_top_bottom: facts.connects_top_bottom,
                    maximum_cost: if facts.cannot_defend || facts.immobile {
                        None
                    } else {
                        Some(1 + usize::from(facts.movement_bonus.unwrap_or(0)))
                    },
                    moving_minion: true,
                    occupied_cells: unit.occupied_cells,
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
            for (_, caster_instance_id) in &spellcasters {
                for (cemetery_minion_instance_id, target) in
                    self.magic_choices(seat, caster_instance_id, &facts.effect)?
                {
                    let descriptor = ActionDescriptor::CastMagic {
                        card_id: definition.id.clone(),
                        card_instance_id: card.instance_id.clone(),
                        caster_instance_id: caster_instance_id.clone(),
                        cemetery_minion_instance_id,
                        target,
                    };
                    let label = descriptor
                        .state_independent_label()
                        .ok_or_else(|| invalid("cast-magic action requires a label"))?
                        + &self.minion_caster_suffix(seat, caster_instance_id);
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
            for destination in self
                .summon_destinations(seat, facts)
                .into_iter()
                .filter(|destination| destination.mana_cost <= u64::from(player.mana))
            {
                let genesis_choices = if facts.genesis
                    == Some(MinionGenesis::MayDamageTargetAdjacentUnitTwo)
                {
                    std::iter::once((Some(GenesisDamageChoice::Decline), None))
                        .chain(
                            self.genesis_damage_targets(seat, &card.instance_id, destination.cell)
                                .into_iter()
                                .map(|target| (Some(GenesisDamageChoice::Target), Some(target))),
                        )
                        .collect::<Vec<_>>()
                } else {
                    vec![(None, None)]
                };
                for (caster_kind, caster_instance_id) in &spellcasters {
                    for (genesis_damage_choice, genesis_damage_target) in &genesis_choices {
                        let genesis_suffix = match (genesis_damage_choice, genesis_damage_target) {
                            (Some(GenesisDamageChoice::Decline), None) => {
                                "; decline Genesis".to_owned()
                            }
                            (Some(GenesisDamageChoice::Target), Some(target)) => format!(
                                "; Genesis targets {} {}…",
                                target.kind(),
                                &target.instance_id().as_str()[..15]
                            ),
                            _ => String::new(),
                        };
                        let caster_suffix = if *caster_kind == UnitKind::Minion {
                            self.minion_caster_suffix(seat, caster_instance_id)
                        } else {
                            String::new()
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
                                mana_cost: destination.mana_cost,
                            },
                            format!(
                                "Summon {} at {} ({} mana){genesis_suffix}{caster_suffix}",
                                definition.id, destination.cell, destination.mana_cost
                            ),
                        );
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
        }
        if !player.avatar.tapped {
            self.append_unit_move_actions(
                actions,
                &player.avatar.card.instance_id,
                player.avatar.location,
                MovementProfile {
                    airborne: false,
                    connects_top_bottom: false,
                    maximum_cost: Some(1),
                    moving_minion: false,
                    occupied_cells: None,
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
                    connects_top_bottom: facts.connects_top_bottom,
                    maximum_cost: if facts.immobile {
                        None
                    } else {
                        Some(1 + usize::from(facts.movement_bonus.unwrap_or(0)))
                    },
                    moving_minion: true,
                    occupied_cells: unit.occupied_cells,
                    restriction: facts.movement_restriction,
                    seat,
                },
            )?;
        }
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
        if !facts.ranged
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
                facts
                    .spellcaster
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
        facts.spellcaster.then_some(UnitKind::Minion)
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
        let mut bonus = 0_u16;
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
        if !disabled
            && unit.region == Region::Surface
            && facts.gains_power_ranged_and_spellcaster_atop_tower
        {
            let atop_tower = self.position.sites[unit.location.index()]
                .as_ref()
                .is_some_and(|site| {
                    matches!(
                        &self.rules.cards[usize::from(site.card.card_id.0)].facts,
                        CardFacts::Site(site_facts) if site_facts.is_tower
                    )
                });
            if atop_tower {
                bonus = bonus.checked_add(2).ok_or(GameError::IllegalAction)?;
            }
        }
        Ok((
            u16::from(facts.attack)
                .checked_add(bonus)
                .ok_or(GameError::IllegalAction)?,
            u16::from(facts.defense)
                .checked_add(bonus)
                .ok_or(GameError::IllegalAction)?,
            !disabled && facts.lethal,
        ))
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
                targets.push((
                    None,
                    Some(UnitTarget::Avatar {
                        instance_id: player.avatar.card.instance_id.clone(),
                        seat: target_seat,
                    }),
                ));
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
                    .map(|unit| {
                        (
                            None,
                            Some(UnitTarget::Minion {
                                instance_id: unit.card.instance_id.clone(),
                                seat: target_seat,
                            }),
                        )
                    }),
            );
        }
        Ok(targets)
    }

    fn magic_choices(
        &self,
        seat: Seat,
        caster_instance_id: &IdentityHash,
        effect: &MagicEffect,
    ) -> Result<Vec<MagicChoice>, GameError> {
        Ok(match effect {
            MagicEffect::HealController(_)
            | MagicEffect::SummonTokenToEachControlledSiteBorderingEnemySite(_) => {
                vec![(None, None)]
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
                    .map(|card| (Some(card.instance_id.clone()), None))
                    .collect();
                if choices.is_empty() {
                    vec![(None, None)]
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
            MagicEffect::BurrowTargetMinionOrArtifact => {
                self.targeted_magic_choices(seat, caster_instance_id, false, true)?
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
                    .map(|unit| {
                        (
                            None,
                            Some(UnitTarget::Minion {
                                instance_id: unit.card.instance_id.clone(),
                                seat: unit.controller,
                            }),
                        )
                    })
                    .collect();
                targets.sort_unstable_by(|left, right| {
                    left.1
                        .as_ref()
                        .expect("Freeze target")
                        .instance_id()
                        .cmp(right.1.as_ref().expect("Freeze target").instance_id())
                });
                targets
            }
            _ => {
                return Err(GameError::UnsupportedManifestFact(
                    unsupported_magic_effect(effect)
                        .unwrap_or("Magic effect")
                        .to_owned(),
                ));
            }
        })
    }

    fn append_unit_move_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
        instance_id: &IdentityHash,
        start: Cell,
        profile: MovementProfile,
    ) -> Result<(), GameError> {
        for path in self.surface_movement_paths(start, profile) {
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

    fn surface_movement_paths(&self, start: Cell, profile: MovementProfile) -> Vec<Vec<Location>> {
        let starting_footprint_exists = profile.occupied_cells.map_or_else(
            || self.surface_location_exists(start),
            |area| {
                area.into_iter()
                    .all(|cell| self.surface_location_exists(cell))
            },
        );
        if !starting_footprint_exists {
            return Vec::new();
        }
        let start = Location {
            cell: start,
            region: Region::Surface,
        };
        let mut paths = vec![vec![start]];
        let Some(maximum_cost) = profile.maximum_cost else {
            return paths;
        };
        let mut frontier = vec![(0_usize, vec![start])];
        while !frontier.is_empty() {
            let mut next_frontier = Vec::new();
            for (cost, path) in frontier {
                let current = *path.last().expect("movement path starts nonempty");
                let step_cost = self.surface_movement_step_cost(current.cell, profile);
                if cost == maximum_cost && step_cost != 0 {
                    continue;
                }
                for cell in current.cell.bordering(profile.connects_top_bottom).chain(
                    profile
                        .airborne
                        .then(|| current.cell.diagonals(profile.connects_top_bottom))
                        .into_iter()
                        .flatten(),
                ) {
                    let candidate = Location {
                        cell,
                        region: Region::Surface,
                    };
                    let footprint_allowed = profile.occupied_cells.map_or_else(
                        || {
                            self.surface_location_exists(cell)
                                && self.surface_entry_allowed(current.cell, cell, profile)
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
                                self.surface_location_exists(entered)
                                    && (current_area.contains(&entered)
                                        || self.surface_entry_allowed(
                                            current.cell,
                                            entered,
                                            profile,
                                        ))
                            })
                        },
                    );
                    if !footprint_allowed
                        || !Self::movement_restriction_allows(profile, current.cell, cell)
                        || path
                            .windows(2)
                            .any(|edge| edge[0] == current && edge[1] == candidate)
                    {
                        continue;
                    }
                    let next_cost = cost + step_cost;
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

    fn surface_movement_step_cost(&self, current: Cell, profile: MovementProfile) -> usize {
        if !profile.airborne || !profile.moving_minion {
            return 1;
        }
        let Some(site) = &self.position.sites[current.index()] else {
            return 1;
        };
        let CardFacts::Site(facts) = &self.rules.cards[usize::from(site.card.card_id.0)].facts
        else {
            return 1;
        };
        usize::from(!facts.airborne_minions_atop_move_freely_away)
    }

    fn surface_entry_allowed(
        &self,
        current: Cell,
        candidate: Cell,
        profile: MovementProfile,
    ) -> bool {
        if !profile.moving_minion || profile.airborne || current == candidate {
            return true;
        }
        let Some(site) = &self.position.sites[candidate.index()] else {
            return true;
        };
        let CardFacts::Site(facts) = &self.rules.cards[usize::from(site.card.card_id.0)].facts
        else {
            return true;
        };
        !facts.blocks_ground_minion_entry_while_minion_atop
            || !self.position.units.iter().any(|unit| {
                unit.region == Region::Surface && Self::unit_occupies_cell(unit, candidate)
            })
    }

    fn movement_restriction_allows(profile: MovementProfile, from: Cell, to: Cell) -> bool {
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

    fn underground_location_exists(&self, cell: Cell) -> bool {
        self.surface_location_exists(cell)
            && !self.position.sites[cell.index()]
                .as_ref()
                .is_some_and(|site| {
                    matches!(
                        &self.rules.cards[usize::from(site.card.card_id.0)].facts,
                        CardFacts::Site(facts) if facts.elements.contains(Element::Water)
                    )
                })
    }

    fn location_exists_in_region(&self, cell: Cell, region: Region) -> bool {
        match region {
            Region::Surface => self.surface_location_exists(cell),
            Region::Underground => self.underground_location_exists(cell),
            Region::Underwater => self.position.sites[cell.index()]
                .as_ref()
                .is_some_and(|site| {
                    matches!(
                        &self.rules.cards[usize::from(site.card.card_id.0)].facts,
                        CardFacts::Site(facts) if facts.elements.contains(Element::Water)
                    )
                }),
            Region::Void => !self.surface_location_exists(cell),
        }
    }

    fn move_underground_units_to_underwater(&mut self, cell: Cell) {
        for unit in &mut self.position.units {
            if unit.location == cell && unit.region == Region::Underground {
                unit.region = Region::Underwater;
            }
        }
    }

    fn minion_can_move_and_attack(&self, unit: &UnitPosition, seat: Seat) -> bool {
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return false;
        };
        unit.controller == seat
            && unit.region == Region::Surface
            && !self.minion_is_disabled(unit)
            && !unit.tapped
            && (!unit.summoning_sickness || facts.charge)
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
            ActionDescriptor::CastMagic { .. } => self.apply_cast_magic_action(action, outcomes),
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
            ActionDescriptor::SummonMinion { .. } => {
                self.apply_summon_minion_action(action, outcomes)
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
            ActionDescriptor::Intercept { unit_instance_id } => {
                self.apply_intercept_action(action.seat, unit_instance_id, outcomes)
            }
            ActionDescriptor::DrawSite => {
                self.apply_draw_action(action.seat, DeckZone::Atlas, true, outcomes)
            }
            ActionDescriptor::DrawSpell => {
                self.apply_draw_action(action.seat, DeckZone::Spellbook, true, outcomes)
            }
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
        if let (Some(completion), Some(pending)) =
            (magic_completion, &mut self.position.pending_deathrites)
            && pending.deferred_magic_resolved.is_none()
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
                .surface_movement_paths(start, profile)
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
        outcomes.push("defender-joined", || {
            json!({
                "from": from,
                "instanceId": unit_instance_id,
                "path": path,
                "seat": seat,
                "steps": path.len() - 1,
                "to": final_anchor,
            })
        });
        match kind {
            UnitKind::Avatar => {
                let avatar = &mut self.position.players[seat_index(seat)].avatar;
                avatar.location = final_anchor.cell;
                avatar.tapped = true;
            }
            UnitKind::Minion => {
                for location in path.iter().skip(1) {
                    let unit = self
                        .position
                        .units
                        .iter_mut()
                        .find(|unit| {
                            unit.card.instance_id == *unit_instance_id && unit.controller == seat
                        })
                        .ok_or(GameError::IllegalAction)?;
                    Self::move_minion_to(unit, location.cell)?;
                    self.settle_nearby_enemy_stealth(outcomes);
                }
                self.position
                    .units
                    .iter_mut()
                    .find(|unit| {
                        unit.card.instance_id == *unit_instance_id && unit.controller == seat
                    })
                    .ok_or(GameError::IllegalAction)?
                    .tapped = true;
            }
        }
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
                Ok((u16::from(facts.attack), false))
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
                "accumulated": accumulated,
                "amount": dealt,
                "direct": true,
                "instanceId": instance_id,
                "seat": seat,
            });
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
                        "accumulated": accumulated,
                        "amount": dealt,
                        "direct": true,
                        "instanceId": instance_id,
                        "seat": seat,
                    });
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
        let deaths = self
            .position
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
            if !token {
                self.position.players[seat_index(owner)]
                    .cemetery
                    .push(corpse.card);
            }
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
        let defeated = pending.defeated_avatars;
        let deck_losers = pending.deck_losers;
        let losers: Vec<_> = [Seat::North, Seat::South]
            .into_iter()
            .filter(|seat| defeated.contains(seat) || deck_losers.contains(seat))
            .collect();
        if losers.len() == 2 {
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
        } else {
            if let Some(continuation) = pending.continuation {
                return match continuation {
                    DeathriteContinuation::EndTurn(continuation) => self.continue_end_turn_deaths(
                        continuation.seat,
                        &continuation.remaining_instance_ids,
                        outcomes,
                    ),
                    DeathriteContinuation::FirstStrike(continuation) => {
                        self.continue_after_first_strike(continuation, outcomes)
                    }
                    DeathriteContinuation::SiteGenesis(continuation) => {
                        self.finish_site_genesis(continuation, outcomes)
                    }
                };
            }
            self.position.phase = pending.return_phase;
            self.position.decision_seat = pending.return_decision_seat;
        }
        if self.position.terminal.is_some() && ordered_resolution {
            self.position.pending_basic_movement = PendingField::Absent;
            self.position.pending_ranged_step = PendingField::Absent;
            self.position.pending_combat = None;
            self.position.phase = Phase::Terminal;
        } else if self.position.terminal.is_some()
            || self.position.pending_basic_movement.is_pending()
            || self.position.pending_ranged_step.is_pending()
        {
            self.reconcile_projectile_continuations()?;
        }
        Ok(())
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
                        connects_top_bottom: false,
                        maximum_cost: Some(1),
                        moving_minion: false,
                        occupied_cells: None,
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
                        connects_top_bottom: facts.connects_top_bottom,
                        maximum_cost: if facts.immobile {
                            None
                        } else {
                            Some(1 + usize::from(facts.movement_bonus.unwrap_or(0)))
                        },
                        moving_minion: true,
                        occupied_cells: unit.occupied_cells,
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
                .surface_movement_paths(current_location, profile)
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
            last_interacted_turn: None,
            location: cell,
            occupied_cells: None,
            region: Region::Surface,
            stealthed: facts.stealth,
            summoning_sickness: true,
            tapped: false,
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

    #[expect(
        clippy::too_many_lines,
        reason = "the closed random activation keeps validation, draw, damage, and deaths atomic"
    )]
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

        let mut candidates = Vec::new();
        for candidate_seat in [Seat::North, Seat::South] {
            let avatar = &self.position.players[seat_index(candidate_seat)].avatar;
            if avatar.location == target_location.cell
                && avatar.card.instance_id != *source_instance_id
            {
                candidates.push((
                    avatar.card.instance_id.clone(),
                    UnitKind::Avatar,
                    candidate_seat,
                ));
            }
        }
        candidates.extend(
            self.position
                .units
                .iter()
                .filter(|unit| {
                    unit.card.instance_id != *source_instance_id
                        && unit.region == target_location.region
                        && Self::unit_occupies_cell(unit, target_location.cell)
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

        let selected = if candidates.is_empty() {
            None
        } else {
            let index = draw_index(
                &mut self.position.prng,
                candidates.len(),
                "sparkmage_random_other_unit_at_nearby_location",
                "unit_index_candidate",
                random_draws,
            )?;
            Some(candidates[index].clone())
        };
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

        if let Some((target_instance_id, target_kind, target_seat)) = selected
            && amount > 0
        {
            let result = self.apply_simple_damage(
                target_kind,
                target_seat,
                &target_instance_id,
                amount,
                UnitDamageSource {
                    current_power,
                    lethal,
                },
                outcomes,
            )?;
            if result.minion_died || result.avatar_defeated {
                self.begin_minion_deaths(
                    if result.minion_died {
                        std::slice::from_ref(&target_instance_id)
                    } else {
                        &[]
                    },
                    if result.avatar_defeated {
                        std::slice::from_ref(&target_seat)
                    } else {
                        &[]
                    },
                    Phase::Main,
                    self.position.active_seat,
                    outcomes,
                )?;
            }
        }
        self.position.state_version += 1;
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the closed Magic transaction keeps validation, payment, and effects atomic"
    )]
    fn apply_cast_magic_action(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let ActionDescriptor::CastMagic {
            card_id,
            card_instance_id,
            caster_instance_id,
            cemetery_minion_instance_id,
            target,
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
                .contains(&(cemetery_minion_instance_id.clone(), target.clone()))
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
            if let Some(target) = target {
                payload["targetInstanceId"] = json!(target.instance_id());
                payload["targetSeat"] = json!(target.seat());
            }
            payload
        });
        self.record_unit_interaction(
            caster_kind.ok_or(GameError::IllegalAction)?,
            seat,
            caster_instance_id,
            outcomes,
        )?;
        match effect {
            MagicEffect::HealController(amount) => {
                self.heal_avatar(seat, u16::from(amount), card_instance_id, outcomes)?;
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
            MagicEffect::BurrowTargetMinionOrArtifact => {
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
                    let can_move = self.position.units[target_index].region == Region::Surface
                        && Self::unit_occupied_cells(&self.position.units[target_index])
                            .iter()
                            .all(|cell| self.underground_location_exists(*cell));
                    if can_move {
                        self.position.units[target_index].region = Region::Underground;
                        let cell = self.position.units[target_index].location;
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
            _ => return Err(GameError::IllegalAction),
        }
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
        self.position.state_version += 1;
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the closed summon transaction keeps caster validation, payment, and Genesis atomic"
    )]
    fn apply_summon_minion_action(
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
        let CardFacts::Minion(facts) = &definition.facts else {
            return Err(GameError::IllegalAction);
        };
        let genesis = facts.genesis;
        let lance_count = facts.lance_count;
        let starts_stealthed = facts.stealth;
        let starts_warded = facts.damage_prevention == Some(DamagePrevention::Ward);
        if genesis == Some(MinionGenesis::StrikeEachEnemyHere) {
            return Err(GameError::UnsupportedManifestFact(
                "minion Genesis effect".to_owned(),
            ));
        }
        if !self
            .summon_destinations(seat, facts)
            .into_iter()
            .any(|destination| {
                destination.cell == *cell
                    && destination.cells == *cells
                    && destination.mana_cost == *mana_cost
            })
            || *mana_cost > u64::from(player.mana)
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
        let paid_mana = u16::try_from(*mana_cost).map_err(|_| GameError::IllegalAction)?;
        let card = self.position.players[player_index]
            .hand_spellbook
            .remove(hand_index);
        let player = &mut self.position.players[player_index];
        player.mana -= paid_mana;
        player.air_thresholds_cast_this_turn = next_air_thresholds_cast_this_turn;
        self.record_unit_interaction(
            caster_kind.ok_or(GameError::IllegalAction)?,
            seat,
            caster_instance_id,
            outcomes,
        )?;
        self.position.units.push(UnitPosition {
            card,
            carried_lance_count: lance_count.unwrap_or(0),
            controller: seat,
            damage: 0,
            disable_effects: Vec::new(),
            disabled_until_damaged: false,
            last_interacted_turn: None,
            location: *cell,
            occupied_cells: *cells,
            region: Region::Surface,
            stealthed: starts_stealthed,
            summoning_sickness: true,
            tapped: false,
            warded: starts_warded,
        });
        self.position.state_version += 1;
        outcomes.push("minion-summoned", || {
            let mut payload = json!({
                "cardId": card_id,
                "casterInstanceId": caster_instance_id,
                "cell": cell,
                "instanceId": card_instance_id,
                "manaPaid": mana_cost,
                "seat": seat,
            });
            if let Some(cells) = cells {
                payload["cells"] = json!(cells);
            }
            payload
        });
        if let Some(count) = lance_count {
            outcomes.push("lance-gained", || {
                json!({
                    "bearerInstanceId": card_instance_id,
                    "count": count,
                    "sourceInstanceId": card_instance_id,
                })
            });
        }
        self.apply_minion_genesis(
            seat,
            card_instance_id,
            genesis,
            *genesis_damage_choice,
            genesis_damage_target.as_ref(),
            outcomes,
        )?;
        Ok(())
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
                self.apply_genesis_area_damage(source_instance_id, outcomes)?;
            }
            Some(MinionGenesis::StrikeEachEnemyHere) => {
                return Err(GameError::UnsupportedManifestFact(
                    "minion Genesis effect".to_owned(),
                ));
            }
        }
        Ok(())
    }

    fn apply_genesis_area_damage(
        &mut self,
        source_instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let source = self
            .position
            .units
            .iter()
            .find(|unit| unit.card.instance_id == *source_instance_id)
            .cloned()
            .ok_or(GameError::IllegalAction)?;
        let (current_power, lethal) = self.combatant_attack_and_lethal(
            UnitKind::Minion,
            source.controller,
            source_instance_id,
        )?;
        let mut targets = Vec::new();
        for seat in [Seat::North, Seat::South] {
            let avatar = &self.position.players[seat_index(seat)].avatar;
            if source.region == Region::Surface
                && Self::unit_occupies_cell(&source, avatar.location)
            {
                targets.push((avatar.card.instance_id.clone(), UnitKind::Avatar, seat));
            }
        }
        targets.extend(
            self.position
                .units
                .iter()
                .filter(|unit| {
                    unit.region == source.region
                        && Self::unit_occupied_cells(unit)
                            .iter()
                            .any(|cell| Self::unit_occupies_cell(&source, *cell))
                        && unit.card.instance_id != *source_instance_id
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
            outcomes.push("genesis-damage-allocated", || {
                json!({
                    "amount": 1,
                    "sourceInstanceId": source_instance_id,
                    "targetInstanceId": target_instance_id,
                })
            });
        }
        let mut dead_minions = Vec::new();
        let mut defeated_avatars = Vec::new();
        for (target_instance_id, kind, seat, status) in targets {
            let result = self.apply_simple_damage_with_status(
                kind,
                seat,
                &target_instance_id,
                1,
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
        for (unit, disabled) in self.position.units.iter_mut().zip(disabled_units) {
            unit.damage = 0;
            if unit.controller == seat {
                let gains_stealth = matches!(
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts,
                    CardFacts::Minion(facts)
                        if !disabled
                            && facts.end_turn_stealth == Some(EndTurnStealth::Always)
                );
                if gains_stealth && !unit.stealthed {
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
        let ended_turn = self.position.turn_number;
        self.position.turn_number += 1;
        self.position.active_seat = next_seat;
        self.position.decision_seat = next_seat;
        self.position.phase = Phase::Draw;
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
        }
        value
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
                match continuation {
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
                    DeathriteContinuation::SiteGenesis(continuation) => {
                        self.site_genesis_continuation_value(continuation)
                    }
                },
            );
        }
        value
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
        if let (Some(turn), Some(avatar)) = (
            player.avatar.last_interacted_turn,
            value.get_mut("avatar").and_then(Value::as_object_mut),
        ) {
            avatar.insert("lastInteractedTurn".to_owned(), json!(turn));
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
        if let Some(turn) = unit.last_interacted_turn {
            object.insert("lastInteractedTurn".to_owned(), json!(turn));
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
        value
    }

    fn pending_combat_value(pending: &PendingCombat) -> Value {
        json!({
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
        })
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
        last_interacted_turn: None,
        life: u16::from(avatar_facts.life),
        location: Cell::parse(if seat == Seat::North { "C4" } else { "C1" })
            .map_err(|_| invalid("avatar start cell must be valid"))?,
        tapped: false,
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
    fn unsupported_magic_diagnostics_should_name_the_concrete_fact() {
        assert_eq!(
            unsupported_magic_effect(&MagicEffect::DamageChainNearbyUnits),
            Some("damageChainNearbyUnits")
        );
    }

    #[test]
    fn bury_selfplay_should_admit_the_minion_slice_and_reject_unmodeled_cards() {
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

        let artifacts = selfplay_manifest_with(31, |manifest| {
            manifest["cards"]["south-spell-1"] = json!({
                "cardType": "artifact",
                "grantsBearerPower": 2,
                "manaCost": 0,
                "thresholds": { "air": 0, "earth": 0, "fire": 0, "water": 0 },
            });
        });
        assert!(matches!(
            Game::from_manifest_json(&artifacts)
                .expect("valid Artifact manifest")
                .ensure_selfplay_supported(),
            Err(GameError::UnsupportedManifestFact(field)) if field == "cardType:artifact"
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
            last_interacted_turn: None,
            location: c4,
            occupied_cells: None,
            region: Region::Surface,
            stealthed: false,
            summoning_sickness: false,
            tapped: false,
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
            last_interacted_turn: None,
            location,
            occupied_cells,
            region: Region::Surface,
            stealthed: false,
            summoning_sickness: false,
            tapped: true,
            warded: false,
        }
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
                last_interacted_turn: None,
                location: Cell::parse(location).expect("fixture cell"),
                occupied_cells: None,
                region: Region::Surface,
                stealthed,
                summoning_sickness: false,
                tapped: false,
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
}
