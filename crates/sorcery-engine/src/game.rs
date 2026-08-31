//! Compact game setup and opening-hand decisions.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::action::{
    ActionDescriptor, CombatTarget, DeckZone, GenesisSpellChoice, GenesisTokenChoice,
    compare_canonical,
};
use crate::board::{Cell, Location, Region};
use crate::canonical::{CanonicalError, IdentityHash, identity_hash};
use crate::contract::{LegalAction, Seat, opaque_action_id};
use crate::facts::{
    CardFacts, DamagePrevention, Element, EndTurnStealth, FactError, MagicEffect, MinionFacts,
    MinionGenesis, SiteFacts, Thresholds, parse_card_definition, validate_identifier,
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
#[derive(Clone, Debug)]
pub struct Position {
    active_seat: Seat,
    decision_seat: Seat,
    pending_combat: Option<PendingCombat>,
    pending_genesis_spell: PendingField<PendingGenesisSpell>,
    pending_genesis_spell_order: PendingField<PendingGenesisSpellOrder>,
    pending_genesis_token: PendingField<PendingGenesisToken>,
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

#[derive(Clone, Debug)]
struct CardInstance {
    card_id: CardId,
    instance_id: IdentityHash,
    owner: Seat,
    source: CardSource,
}

#[derive(Clone, Debug)]
struct AvatarPosition {
    card: CardInstance,
    death_door_turn: Option<u64>,
    last_interacted_turn: Option<u64>,
    life: u16,
    location: Cell,
    tapped: bool,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
struct SitePosition {
    card: CardInstance,
    controller: Seat,
}

#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "named dynamic flags preserve distinct authoritative unit state"
)]
struct UnitPosition {
    card: CardInstance,
    controller: Seat,
    damage: u8,
    disabled_until_damaged: bool,
    last_interacted_turn: Option<u64>,
    location: Cell,
    stealthed: bool,
    summoning_sickness: bool,
    tapped: bool,
    warded: bool,
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

#[derive(Clone, Debug)]
struct PendingCombat {
    attacker_instance_id: IdentityHash,
    attacker_kind: UnitKind,
    attacking_seat: Seat,
    cell: Cell,
    original_target: Option<CombatTarget>,
}

#[derive(Clone, Debug)]
struct PendingGenesisToken {
    cell: Cell,
    seat: Seat,
    source_instance_id: IdentityHash,
}

#[derive(Clone, Debug)]
struct PendingGenesisSpell {
    seat: Seat,
    source_instance_id: IdentityHash,
}

#[derive(Clone, Debug)]
struct PendingGenesisSpellOrder {
    count: u8,
    seat: Seat,
    source_instance_id: IdentityHash,
}

#[derive(Clone, Debug)]
enum PendingField<T> {
    Absent,
    Pending(T),
    Resolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Attack,
    Defend,
    Draw,
    Genesis,
    Main,
    Mulligan,
    Terminal,
}

impl Phase {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Attack => "attack",
            Self::Defend => "defend",
            Self::Draw => "draw",
            Self::Genesis => "genesis",
            Self::Main => "main",
            Self::Mulligan => "mulligan",
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
}

#[derive(Clone, Copy, Debug)]
enum TerminalResult {
    Draw,
    Win {
        loser: Seat,
        reason: WinReason,
        winner: Seat,
    },
}

#[derive(Clone, Copy, Debug)]
enum WinReason {
    AvatarDefeated,
    DeckEmpty,
}

struct DamageResult {
    minion_died: bool,
    avatar_defeated: bool,
}

#[derive(Clone, Copy)]
struct UnitDamageSource {
    current_power: u8,
    lethal: bool,
}

#[derive(Clone, Copy)]
struct SummonDestination {
    cell: Cell,
    mana_cost: u64,
}

enum OutcomeLog<'a> {
    Ignore,
    Record(&'a mut Vec<(String, Value)>),
}

impl OutcomeLog<'_> {
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
    facts.genesis_discard_top_spells
        || facts.genesis_draw_spell_per_adjacent_same_card
        || facts.genesis_enemies_lose_stealth
        || facts.genesis_gain_mana.is_some()
        || facts.genesis_gain_mana_if_only_controlled_copy
        || facts.genesis_heal_nearby_avatars
        || facts.genesis_immobilize_nearby_until_next_turn
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
                pending_combat: None,
                pending_genesis_spell: PendingField::Absent,
                pending_genesis_spell_order: PendingField::Absent,
                pending_genesis_token: PendingField::Absent,
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
            Some(TerminalResult::Draw) => Some(GameOutcome::Draw),
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
            Some(TerminalResult::Draw) => Some(GameEndReason::SimultaneousAvatarDefeat),
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
            Phase::Attack => self.append_attack_actions(&mut actions)?,
            Phase::Defend => self.append_defend_actions(&mut actions)?,
            Phase::Draw => self.append_draw_actions(&mut actions)?,
            Phase::Genesis => self.append_genesis_actions(&mut actions)?,
            Phase::Mulligan => self.append_mulligan_actions(&mut actions)?,
            Phase::Main => self.append_main_actions(&mut actions)?,
            Phase::Terminal => {}
        }
        actions
            .sort_unstable_by(|left, right| compare_canonical(&left.descriptor, &right.descriptor));
        Ok(actions)
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
        let target = self
            .position
            .pending_combat
            .as_ref()
            .and_then(|pending| pending.original_target.as_ref())
            .ok_or(GameError::IllegalAction)?;
        let descriptor = ActionDescriptor::CloseDefend {
            original_target_participates: !matches!(target, CombatTarget::Site { .. }),
        };
        let label = descriptor
            .state_independent_label()
            .ok_or_else(|| invalid("close-defend action requires a label"))?;
        self.push_action(actions, descriptor, label);
        Ok(())
    }

    fn attack_targets(&self) -> Result<Vec<CombatTarget>, GameError> {
        let pending = self
            .position
            .pending_combat
            .as_ref()
            .ok_or(GameError::IllegalAction)?;
        let opposing_seat = other_seat(pending.attacking_seat);
        let opposing_player = &self.position.players[seat_index(opposing_seat)];
        let mut targets = Vec::new();
        if opposing_player.avatar.location == pending.cell {
            targets.push(CombatTarget::Avatar {
                instance_id: opposing_player.avatar.card.instance_id.clone(),
                seat: opposing_seat,
            });
        }
        targets.extend(
            self.position
                .units
                .iter()
                .filter(|unit| {
                    unit.controller == opposing_seat
                        && unit.location == pending.cell
                        && !unit.stealthed
                })
                .map(|unit| CombatTarget::Minion {
                    instance_id: unit.card.instance_id.clone(),
                    seat: opposing_seat,
                }),
        );
        if self.attacker_can_target_sites(pending)?
            && let Some(site) = &self.position.sites[pending.cell.index()]
            && site.controller == opposing_seat
        {
            targets.push(CombatTarget::Site {
                instance_id: site.card.instance_id.clone(),
                seat: opposing_seat,
            });
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
        }
        if !player.domain_established {
            return Ok(());
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
                .filter(|destination| destination.mana_cost <= u64::from(player.mana))
            {
                self.push_action(
                    actions,
                    ActionDescriptor::SummonMinion {
                        card_id: definition.id.clone(),
                        card_instance_id: card.instance_id.clone(),
                        caster_instance_id: player.avatar.card.instance_id.clone(),
                        cell: destination.cell,
                        mana_cost: destination.mana_cost,
                    },
                    format!(
                        "Summon {} at {} ({} mana)",
                        definition.id, destination.cell, destination.mana_cost
                    ),
                );
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
        if !player.avatar.tapped {
            self.append_unit_move_actions(
                actions,
                &player.avatar.card.instance_id,
                player.avatar.location,
                false,
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
                facts.connects_top_bottom,
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
                && !unit.disabled_until_damaged
                && !unit.tapped
                && !unit.summoning_sickness
        })?;
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return None;
        };
        facts.tap_for_mana
    }

    fn append_unit_move_actions(
        &self,
        actions: &mut Vec<IssuedAction>,
        instance_id: &IdentityHash,
        start: Cell,
        connects_top_bottom: bool,
    ) -> Result<(), GameError> {
        let from = Location {
            cell: start,
            region: Region::Surface,
        };
        for destination in std::iter::once(start).chain(
            start
                .bordering(connects_top_bottom)
                .filter(|cell| self.surface_location_exists(*cell)),
        ) {
            let to = Location {
                cell: destination,
                region: Region::Surface,
            };
            let mut path = vec![from];
            if destination != start {
                path.push(to);
            }
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

    fn minion_can_move_and_attack(&self, unit: &UnitPosition, seat: Seat) -> bool {
        let CardFacts::Minion(facts) = &self.rules.cards[usize::from(unit.card.card_id.0)].facts
        else {
            return false;
        };
        unit.controller == seat
            && !unit.disabled_until_damaged
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
        let site_elements = self
            .position
            .sites
            .iter()
            .flatten()
            .filter(|site| site.controller == seat)
            .filter_map(|site| {
                let CardFacts::Site(facts) =
                    &self.rules.cards[usize::from(site.card.card_id.0)].facts
                else {
                    return None;
                };
                Some(facts.elements)
            })
            .flat_map(crate::facts::ElementSet::iter);
        let provider_elements = self
            .position
            .units
            .iter()
            .filter(|unit| unit.controller == seat && !unit.disabled_until_damaged)
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

    fn summon_destinations<'a>(
        &'a self,
        seat: Seat,
        minion: &'a MinionFacts,
    ) -> impl Iterator<Item = SummonDestination> + 'a {
        Cell::ALL.into_iter().filter_map(move |cell| {
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
            Some(SummonDestination {
                cell,
                mana_cost: minion.mana_cost.saturating_sub(discount),
            })
        })
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
        if !player.domain_established {
            return self.position.sites[player.avatar.location.index()]
                .is_none()
                .then_some(player.avatar.location)
                .into_iter()
                .collect();
        }
        self.controlled_site_cells(seat)
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
        self.apply_action_with_log(action, &mut OutcomeLog::Ignore)
    }

    pub(crate) fn apply_action_recorded(
        &mut self,
        action: &IssuedAction,
    ) -> Result<Vec<(String, Value)>, GameError> {
        let mut outcomes = Vec::new();
        self.apply_action_with_log(action, &mut OutcomeLog::Record(&mut outcomes))?;
        Ok(outcomes)
    }

    fn apply_action_with_log(
        &mut self,
        action: &IssuedAction,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        if action.seat != self.position.decision_seat
            || action.state_version != self.position.state_version
        {
            return Err(GameError::IllegalAction);
        }
        match &action.descriptor {
            ActionDescriptor::ActivateMana {
                amount,
                unit_instance_id,
            } => self.apply_mana_activation(action.seat, *amount, unit_instance_id, outcomes),
            ActionDescriptor::CloseDefend {
                original_target_participates,
            } => {
                self.apply_close_defend_action(action.seat, *original_target_participates, outcomes)
            }
            ActionDescriptor::DeclareAttack { target } => {
                self.apply_declare_attack_action(action.seat, target, outcomes)
            }
            ActionDescriptor::DeclineAttack => {
                self.apply_decline_attack_action(action.seat, outcomes)
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
            ActionDescriptor::EndTurn => self.apply_end_turn_action(action.seat, outcomes),
            ActionDescriptor::DrawSite => {
                self.apply_draw_action(action.seat, DeckZone::Atlas, true, outcomes)
            }
            ActionDescriptor::DrawSpell => {
                self.apply_draw_action(action.seat, DeckZone::Spellbook, true, outcomes)
            }
        }
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
        if self.position.units.iter().any(|unit| {
            unit.controller != seat
                && unit.location == pending.cell
                && !unit.tapped
                && !unit.summoning_sickness
        }) {
            return Err(GameError::UnsupportedManifestFact(
                "intercept window after declined attack".to_owned(),
            ));
        }
        let attacker_instance_id = pending.attacker_instance_id.clone();
        self.position.pending_combat = None;
        self.position.phase = Phase::Main;
        self.position.decision_seat = seat;
        self.position.state_version += 1;
        outcomes.push("attack-declined", || {
            json!({
                "interceptWindowOpened": false,
                "seat": seat,
                "unitInstanceId": attacker_instance_id,
            })
        });
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
        let cell = pending.cell;
        let defending_seat = target.seat();
        self.position
            .pending_combat
            .as_mut()
            .ok_or(GameError::IllegalAction)?
            .original_target = Some(target.clone());
        self.position.decision_seat = defending_seat;
        self.position.phase = Phase::Defend;
        self.position.state_version += 1;
        outcomes.push("attack-declared", || {
            json!({
                "attackerInstanceId": attacker_instance_id,
                "cell": cell,
                "seat": seat,
                "target": target,
            })
        });
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
        let expected_participation = !matches!(target, CombatTarget::Site { .. });
        if self.position.phase != Phase::Defend
            || seat != target.seat()
            || original_target_participates != expected_participation
        {
            return Err(GameError::IllegalAction);
        }
        outcomes.push("defend-window-closed", || {
            json!({
                "defenderCount": 0,
                "originalTargetParticipates": original_target_participates,
            })
        });
        match target {
            CombatTarget::Site { .. } => self.resolve_undefended_site_strike(outcomes)?,
            CombatTarget::Avatar { .. } | CombatTarget::Minion { .. } => {
                self.resolve_simple_avatar_fight(outcomes)?;
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
        self.record_unit_interaction(attacker_kind, attacking_seat, &attacker_id, outcomes)?;

        let avatar = &mut self.position.players[seat_index(target_seat)].avatar;
        let old_life = avatar.life;
        avatar.life = avatar.life.saturating_sub(u16::from(attack));
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
        Ok(())
    }

    fn combatant_attack_and_lethal(
        &self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
    ) -> Result<(u8, bool), GameError> {
        let card_id = match kind {
            UnitKind::Avatar => {
                let avatar = &self.position.players[seat_index(seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                avatar.card.card_id
            }
            UnitKind::Minion => {
                self.position
                    .units
                    .iter()
                    .find(|unit| unit.card.instance_id == *instance_id && unit.controller == seat)
                    .ok_or(GameError::IllegalAction)?
                    .card
                    .card_id
            }
        };
        match &self.rules.cards[usize::from(card_id.0)].facts {
            CardFacts::Avatar(facts) => Ok((facts.attack, false)),
            CardFacts::Minion(facts) => Ok((facts.attack, facts.lethal)),
            _ => Err(GameError::IllegalAction),
        }
    }

    fn record_unit_interaction(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        match kind {
            UnitKind::Avatar => {
                let avatar = &mut self.position.players[seat_index(seat)].avatar;
                if avatar.card.instance_id != *instance_id {
                    return Err(GameError::IllegalAction);
                }
                avatar.last_interacted_turn = Some(self.position.turn_number);
            }
            UnitKind::Minion => {
                let unit = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| unit.card.instance_id == *instance_id && unit.controller == seat)
                    .ok_or(GameError::IllegalAction)?;
                unit.last_interacted_turn = Some(self.position.turn_number);
                if unit.stealthed {
                    unit.stealthed = false;
                    outcomes.push(
                        "stealth-lost",
                        || json!({ "instanceId": instance_id, "seat": seat }),
                    );
                }
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the compact fight path keeps simultaneous damage and terminal event order explicit"
    )]
    fn resolve_simple_avatar_fight(
        &mut self,
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
        let target_kind = match target {
            CombatTarget::Avatar { .. } => UnitKind::Avatar,
            CombatTarget::Minion { .. } => UnitKind::Minion,
            CombatTarget::Site { .. } => return Err(GameError::IllegalAction),
        };
        let attacker_id = pending.attacker_instance_id.clone();
        let attacker_kind = pending.attacker_kind;
        let attacking_seat = pending.attacking_seat;
        let target_id = target.instance_id().clone();
        let target_seat = target.seat();
        let (attacker_attack, attacker_lethal) =
            self.combatant_attack_and_lethal(attacker_kind, attacking_seat, &attacker_id)?;
        let (target_attack, target_lethal) =
            self.combatant_attack_and_lethal(target_kind, target_seat, &target_id)?;
        let target_can_strike = match target_kind {
            UnitKind::Avatar => true,
            UnitKind::Minion => {
                !self
                    .position
                    .units
                    .iter()
                    .find(|unit| {
                        unit.card.instance_id == target_id && unit.controller == target_seat
                    })
                    .ok_or(GameError::IllegalAction)?
                    .disabled_until_damaged
            }
        };

        self.record_unit_interaction(attacker_kind, attacking_seat, &attacker_id, outcomes)?;
        self.record_unit_interaction(target_kind, target_seat, &target_id, outcomes)?;
        outcomes.push("fight-started", || {
            json!({
                "attackerInstanceId": attacker_id,
                "combatantInstanceIds": [target_id],
            })
        });
        outcomes.push("strike-damage-allocated", || {
            json!({
                "amount": attacker_attack,
                "strikerInstanceId": attacker_id,
                "targetInstanceId": target_id,
            })
        });
        let attacker_damage = if target_can_strike {
            self.apply_simple_damage(
                attacker_kind,
                attacking_seat,
                &attacker_id,
                target_attack,
                UnitDamageSource {
                    current_power: target_attack,
                    lethal: target_lethal,
                },
                outcomes,
            )?
        } else {
            DamageResult {
                minion_died: false,
                avatar_defeated: false,
            }
        };
        let target_damage = self.apply_simple_damage(
            target_kind,
            target_seat,
            &target_id,
            attacker_attack,
            UnitDamageSource {
                current_power: attacker_attack,
                lethal: attacker_lethal,
            },
            outcomes,
        )?;
        if attacker_damage.minion_died {
            self.remove_dead_minion(&attacker_id, outcomes)?;
        }
        if target_damage.minion_died {
            self.remove_dead_minion(&target_id, outcomes)?;
        }
        let defeated = if attacker_damage.avatar_defeated {
            Some(attacking_seat)
        } else if target_damage.avatar_defeated {
            Some(target_seat)
        } else {
            None
        };
        self.position.pending_combat = None;
        self.position.decision_seat = self.position.active_seat;
        if attacker_damage.avatar_defeated && target_damage.avatar_defeated {
            self.position.phase = Phase::Terminal;
            self.position.terminal = Some(TerminalResult::Draw);
            outcomes.push("game-ended", || {
                json!({
                    "reason": "simultaneous_avatar_defeat",
                    "result": "draw",
                })
            });
        } else if let Some(loser) = defeated {
            let winner = other_seat(loser);
            self.position.phase = Phase::Terminal;
            self.position.terminal = Some(TerminalResult::Win {
                loser,
                reason: WinReason::AvatarDefeated,
                winner,
            });
            outcomes.push("game-ended", || {
                json!({
                    "loser": loser,
                    "reason": "avatar_defeated",
                    "winner": winner,
                })
            });
        } else {
            self.position.phase = Phase::Main;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one damage helper preserves exact minion and Avatar event ordering"
    )]
    fn apply_simple_damage(
        &mut self,
        kind: UnitKind,
        seat: Seat,
        instance_id: &IdentityHash,
        amount: u8,
        source: UnitDamageSource,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<DamageResult, GameError> {
        match kind {
            UnitKind::Minion => {
                let (index, _, defense, damage_prevention) =
                    self.simple_minion_combatant(instance_id)?;
                let unit = &mut self.position.units[index];
                let dealt = if !unit.disabled_until_damaged
                    && matches!(
                        damage_prevention,
                        Some(DamagePrevention::PreventsDamageFromUnitsWithPowerAtLeast(
                            threshold
                        )) if source.current_power >= threshold
                    ) {
                    0
                } else {
                    amount
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
                let old_life = avatar.life;
                avatar.life = avatar.life.saturating_sub(u16::from(amount));
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
    ) -> Result<(usize, u8, u8, Option<DamagePrevention>), GameError> {
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
        Ok((index, facts.attack, facts.defense, facts.damage_prevention))
    }

    fn remove_dead_minion(
        &mut self,
        instance_id: &IdentityHash,
        outcomes: &mut OutcomeLog<'_>,
    ) -> Result<(), GameError> {
        let index = self
            .position
            .units
            .iter()
            .position(|unit| unit.card.instance_id == *instance_id)
            .ok_or(GameError::IllegalAction)?;
        let unit = self.position.units.remove(index);
        let card_id = self.rules.cards[usize::from(unit.card.card_id.0)]
            .id
            .clone();
        let owner = unit.card.owner;
        self.position.players[seat_index(owner)]
            .cemetery
            .push(unit.card);
        outcomes.push("minion-died", || {
            json!({
                "cardId": card_id,
                "instanceId": instance_id,
                "owner": owner,
            })
        });
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
            || from.region != Region::Surface
            || to.region != Region::Surface
            || !(1..=2).contains(&path.len())
            || path.first() != Some(&from)
            || path.last() != Some(&to)
            || !self.surface_location_exists(to.cell)
            || (path.len() == 1 && from != to)
        {
            return Err(GameError::IllegalAction);
        }
        let (attacker_kind, current_location, ready, connects_top_bottom) = {
            let player = &self.position.players[seat_index(seat)];
            if player.avatar.card.instance_id == *unit_instance_id {
                (
                    UnitKind::Avatar,
                    player.avatar.location,
                    !player.avatar.tapped,
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
                    facts.connects_top_bottom,
                )
            }
        };
        if !ready
            || current_location != from.cell
            || path.len() == 2
                && (from == to
                    || !from
                        .cell
                        .bordering(connects_top_bottom)
                        .any(|cell| cell == to.cell))
        {
            return Err(GameError::IllegalAction);
        }
        match attacker_kind {
            UnitKind::Avatar => {
                let avatar = &mut self.position.players[seat_index(seat)].avatar;
                avatar.location = to.cell;
                avatar.tapped = true;
            }
            UnitKind::Minion => {
                let unit = self
                    .position
                    .units
                    .iter_mut()
                    .find(|unit| unit.card.instance_id == *unit_instance_id)
                    .ok_or(GameError::IllegalAction)?;
                unit.location = to.cell;
                unit.tapped = true;
            }
        }
        self.position.pending_combat = Some(PendingCombat {
            attacker_instance_id: unit_instance_id.clone(),
            attacker_kind,
            attacking_seat: seat,
            cell: to.cell,
            original_target: None,
        });
        self.position.phase = Phase::Attack;
        self.position.state_version += 1;
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
        let rubble = create_rubble_at
            .map(|rubble_cell| {
                identity_hash(&json!({
                    "cell": rubble_cell,
                    "kind": "rubble",
                    "sourceInstanceId": player.avatar.card.instance_id,
                    "stateVersion": origin_state_version,
                }))
            })
            .transpose()?;
        let avatar_instance_id = player.avatar.card.instance_id.clone();
        let genesis_token_card_id = facts.genesis_pay_one_mana_to_summon_token.clone();
        match (&genesis_token_card_id, genesis_token_choice) {
            (Some(_), Some(GenesisTokenChoice::Decline | GenesisTokenChoice::PayOneMana))
            | (None, None) => {}
            _ => return Err(GameError::IllegalAction),
        }
        let paid_token = matches!(genesis_token_choice, Some(GenesisTokenChoice::PayOneMana));
        let token = if paid_token {
            Some(
                self.create_token_unit(
                    seat,
                    genesis_token_card_id
                        .as_deref()
                        .ok_or(GameError::IllegalAction)?,
                    card_instance_id,
                    cell,
                    origin_state_version,
                )?,
            )
        } else {
            None
        };
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
        let genesis_enemies_lose_stealth = facts.genesis_enemies_lose_stealth;
        let genesis_discard_top_spells = facts.genesis_discard_top_spells;
        let genesis_may_bottom_next_spell = facts.genesis_may_bottom_next_spell;
        let genesis_reorder_next_spells = facts.genesis_reorder_next_spells;
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
        let genesis_heal_nearby_avatars = facts.genesis_heal_nearby_avatars;
        let ordinary_mana = player.mana.checked_add(1).ok_or(GameError::IllegalAction)?;
        let mana_after_token = ordinary_mana
            .checked_sub(u16::from(paid_token))
            .ok_or(GameError::IllegalAction)?;
        let final_mana = mana_after_token
            .checked_add(u16::from(genesis_gain_mana.unwrap_or(0)))
            .ok_or(GameError::IllegalAction)?;
        let card = self.position.players[player_index]
            .hand_atlas
            .remove(hand_index);
        let replaced_rubble = self.position.rubble[cell.index()].take();
        let player = &mut self.position.players[player_index];
        player.avatar.tapped = true;
        player.domain_established = true;
        player.mana = mana_after_token;
        self.position.sites[cell.index()] = Some(SitePosition {
            card,
            controller: seat,
        });
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
        if let Some(amount) = genesis_gain_mana {
            self.position.players[player_index].mana = final_mana;
            outcomes.push("mana-gained", || {
                json!({
                    "amount": amount,
                    "seat": seat,
                    "sourceInstanceId": card_instance_id,
                })
            });
        }
        if genesis_heal_nearby_avatars {
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
                    self.heal_avatar(healed_seat, 3, card_instance_id, outcomes)?;
                }
            }
        }
        if genesis_enemies_lose_stealth {
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
            self.apply_genesis_draws(seat, card_instance_id, DeckZone::Spellbook, count, outcomes);
        }
        if genesis_discard_top_spells {
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
        self.begin_hidden_spell_genesis(
            seat,
            card_instance_id,
            genesis_may_bottom_next_spell,
            genesis_reorder_next_spells,
        );
        if self.position.terminal.is_none()
            && let (Some(rubble_cell), Some(rubble_instance_id)) = (create_rubble_at, rubble)
        {
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
                    "ordinal": 0,
                    "owner": owner,
                    "source": "token",
                    "sourceInstanceId": source_instance_id,
                    "stateVersion": origin_state_version,
                }))?,
                owner,
                source: CardSource::Token,
            },
            controller: owner,
            damage: 0,
            disabled_until_damaged: false,
            last_interacted_turn: None,
            location: cell,
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
        let genesis_may_bottom_next_spell = facts.genesis_may_bottom_next_spell;
        let genesis_reorder_next_spells = facts.genesis_reorder_next_spells;
        let pending_token = facts
            .genesis_pay_one_mana_to_summon_token
            .as_ref()
            .map(|_| top.instance_id.clone());
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
        if let Some(source_instance_id) = pending_token {
            self.position.pending_genesis_token = PendingField::Pending(PendingGenesisToken {
                cell: target_cell,
                seat,
                source_instance_id,
            });
            self.position.phase = Phase::Genesis;
        }
        self.begin_hidden_spell_genesis(
            seat,
            &card_instance_id,
            genesis_may_bottom_next_spell,
            genesis_reorder_next_spells,
        );
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
        Ok(())
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
            mana_cost,
        } = &action.descriptor
        else {
            return Err(GameError::IllegalAction);
        };
        let seat = action.seat;
        let player_index = seat_index(seat);
        let player = &self.position.players[player_index];
        if self.position.phase != Phase::Main
            || !player.domain_established
            || player.avatar.card.instance_id != *caster_instance_id
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
        let starts_stealthed = facts.stealth;
        if matches!(
            genesis,
            Some(
                MinionGenesis::DamageEachOtherUnitHereOne
                    | MinionGenesis::MayDamageTargetAdjacentUnitTwo
                    | MinionGenesis::StrikeEachEnemyHere
            )
        ) {
            return Err(GameError::UnsupportedManifestFact(
                "minion Genesis effect".to_owned(),
            ));
        }
        if !self
            .summon_destinations(seat, facts)
            .any(|destination| destination.cell == *cell && destination.mana_cost == *mana_cost)
            || *mana_cost > u64::from(player.mana)
            || !self.thresholds_met(seat, facts.thresholds)
        {
            return Err(GameError::IllegalAction);
        }
        let paid_mana = u16::try_from(*mana_cost).map_err(|_| GameError::IllegalAction)?;
        let card = self.position.players[player_index]
            .hand_spellbook
            .remove(hand_index);
        let player = &mut self.position.players[player_index];
        player.mana -= paid_mana;
        player.avatar.last_interacted_turn = Some(self.position.turn_number);
        self.position.units.push(UnitPosition {
            card,
            controller: seat,
            damage: 0,
            disabled_until_damaged: false,
            last_interacted_turn: None,
            location: *cell,
            stealthed: starts_stealthed,
            summoning_sickness: true,
            tapped: false,
            warded: false,
        });
        self.position.state_version += 1;
        outcomes.push("minion-summoned", || {
            json!({
                "cardId": card_id,
                "casterInstanceId": caster_instance_id,
                "cell": cell,
                "instanceId": card_instance_id,
                "manaPaid": mana_cost,
                "seat": seat,
            })
        });
        self.apply_minion_genesis(seat, card_instance_id, genesis, outcomes)?;
        Ok(())
    }

    fn apply_minion_genesis(
        &mut self,
        seat: Seat,
        source_instance_id: &IdentityHash,
        genesis: Option<MinionGenesis>,
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
            Some(
                MinionGenesis::DamageEachOtherUnitHereOne
                | MinionGenesis::MayDamageTargetAdjacentUnitTwo
                | MinionGenesis::StrikeEachEnemyHere,
            ) => {
                return Err(GameError::UnsupportedManifestFact(
                    "minion Genesis effect".to_owned(),
                ));
            }
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
        let next_seat = other_seat(seat);
        let next_mana = self
            .position
            .sites
            .iter()
            .flatten()
            .filter(|site| site.controller == next_seat)
            .count();
        let next_mana = u16::try_from(next_mana).map_err(|_| GameError::IllegalAction)?;
        self.position.players[seat_index(seat)].mana = 0;
        let next_player = &mut self.position.players[seat_index(next_seat)];
        next_player.avatar.tapped = false;
        next_player.mana = next_mana;
        for unit in &mut self.position.units {
            unit.damage = 0;
            if unit.controller == seat {
                let gains_stealth = matches!(
                    &self.rules.cards[usize::from(unit.card.card_id.0)].facts,
                    CardFacts::Minion(facts)
                        if !unit.disabled_until_damaged
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
        let ended_turn = self.position.turn_number;
        self.position.turn_number += 1;
        self.position.active_seat = next_seat;
        self.position.decision_seat = next_seat;
        self.position.phase = Phase::Draw;
        self.position.state_version += 1;
        outcomes.push(
            "turn-ended",
            || json!({ "seat": seat, "turnNumber": ended_turn }),
        );
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
        }
        value
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
            ("region".to_owned(), json!("surface")),
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
        if unit.disabled_until_damaged {
            object.insert("disabledUntilDamaged".to_owned(), json!(true));
        }
        value
    }

    fn pending_combat_value(pending: &PendingCombat) -> Value {
        json!({
            "allocations": [],
            "attacker": {
                "instanceId": pending.attacker_instance_id,
                "kind": pending.attacker_kind.as_str(),
                "seat": pending.attacking_seat,
            },
            "attackingSeat": pending.attacking_seat,
            "cell": pending.cell,
            "combatants": [],
            "defenders": [],
            "originalTarget": pending.original_target,
            "targetRemoved": false,
        })
    }

    fn terminal_value(&self) -> Value {
        self.position.terminal.map_or_else(
            || json!({ "status": "active" }),
            |terminal| match terminal {
                TerminalResult::Draw => json!({
                    "reason": "simultaneous_avatar_defeat",
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
        let candidate = draw_index(prng, index + 1, purpose, random_draws)?;
        cards.swap(index, candidate);
    }
    Ok(())
}

fn draw_index(
    prng: &mut PrngState,
    exclusive_maximum: usize,
    purpose: &str,
    random_draws: &mut Vec<EngineRandomDraw>,
) -> Result<usize, GameError> {
    let maximum = u64::try_from(exclusive_maximum)
        .map_err(|_| invalid("random choice size exceeds the supported range"))?;
    let limit = (1_u64 << 32) / maximum * maximum;
    loop {
        let pre_prng_state_hash = identity_hash(&serde_json::to_value(*prng)?)?;
        let result = prng.draw_u32();
        let draw = u64::from(result);
        let accepted = draw < limit;
        random_draws.push(EngineRandomDraw {
            domain: RandomDomain {
                accepted,
                exclusive_maximum,
                kind: "shuffle_index_candidate",
            },
            draw_sequence: prng.draws,
            post_prng_state_hash: identity_hash(&serde_json::to_value(*prng)?)?,
            pre_prng_state_hash,
            purpose: purpose.to_owned(),
            result,
        });
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
