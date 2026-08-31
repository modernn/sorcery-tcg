//! Compact game setup and opening-hand decisions.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::canonical::{CanonicalError, IdentityHash, identity_hash};
use crate::contract::{LegalAction, Seat, opaque_action_id};
use crate::prng::PrngState;

const ENGINE_VERSION: &str = "sorcery-core-v1";
const MAX_DECK_CARDS: usize = 200;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

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
    phase: Phase,
    players: [PlayerPosition; 2],
    prng: PrngState,
    state_version: u64,
    turn_number: u64,
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

/// A typed, engine-issued opening-hand action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MulliganAction {
    action_id: IdentityHash,
    descriptor: MulliganDescriptor,
    label: String,
    seat: Seat,
    state_version: u64,
}

/// A typed opening-hand selection. Order controls which returned cards go to
/// the bottom first.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MulliganDescriptor {
    atlas_order: Vec<IdentityHash>,
    kind: MulliganKind,
    spellbook_order: Vec<IdentityHash>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum MulliganKind {
    Mulligan,
}

/// The setup or opening-hand request was invalid.
#[derive(Debug)]
pub enum GameError {
    /// Canonical serialization or hashing failed.
    Canonical(CanonicalError),
    /// JSON could not be decoded.
    Json(serde_json::Error),
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
            Self::InvalidManifest(message) => formatter.write_str(message),
            Self::UnsupportedManifestFact(field) => {
                write!(
                    formatter,
                    "manifest fact is not yet supported by Rust: {field}"
                )
            }
            Self::IllegalAction => formatter.write_str("mulligan action is not legal here"),
        }
    }
}

impl Error for GameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            Self::Json(error) => Some(error),
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
    id: String,
    kind: CardKind,
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
}

impl CardSource {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Atlas => "atlas",
            Self::Avatar => "avatar",
            Self::Spellbook => "spellbook",
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
    life: u16,
    location: &'static str,
}

#[derive(Clone, Debug)]
struct PlayerPosition {
    air_thresholds_cast_this_turn: Option<u16>,
    atlas: Vec<CardInstance>,
    avatar: AvatarPosition,
    hand_atlas: Vec<CardInstance>,
    hand_spellbook: Vec<CardInstance>,
    mana: u16,
    mulligan_complete: bool,
    spellbook: Vec<CardInstance>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Main,
    Mulligan,
}

impl Phase {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Mulligan => "mulligan",
        }
    }
}

fn invalid(message: impl Into<String>) -> GameError {
    GameError::InvalidManifest(message.into())
}

fn card_kind(value: &Value) -> Result<CardKind, GameError> {
    match value.get("cardType").and_then(Value::as_str) {
        Some("artifact") => Ok(CardKind::Artifact),
        Some("aura") => Ok(CardKind::Aura),
        Some("avatar") => Ok(CardKind::Avatar),
        Some("magic") => Ok(CardKind::Magic),
        Some("minion") => Ok(CardKind::Minion),
        Some("site") => Ok(CardKind::Site),
        _ => Err(invalid("cardType is unsupported")),
    }
}

fn require_card_id(value: &str, path: &str) -> Result<(), GameError> {
    if value.trim().is_empty() || value.len() > 256 {
        return Err(invalid(format!("{path} must be 1-256 characters")));
    }
    Ok(())
}

fn validate_integer(
    definition: &Value,
    field: &str,
    minimum: u64,
    maximum: Option<u64>,
) -> Result<u64, GameError> {
    let value = definition
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid(format!("card.{field} must be a nonnegative integer")))?;
    if value < minimum || value > maximum.unwrap_or(MAX_SAFE_INTEGER) {
        return Err(invalid(format!(
            "card.{field} is outside the supported range"
        )));
    }
    Ok(value)
}

fn validate_thresholds(definition: &Value) -> Result<(), GameError> {
    let thresholds = definition
        .get("thresholds")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("card.thresholds must be an object"))?;
    if thresholds.len() != 4
        || ["air", "earth", "fire", "water"].iter().any(|element| {
            thresholds
                .get(*element)
                .and_then(Value::as_u64)
                .is_none_or(|value| value > MAX_SAFE_INTEGER)
        })
    {
        return Err(invalid(
            "card.thresholds must contain nonnegative air, earth, fire, and water integers",
        ));
    }
    Ok(())
}

fn validate_card_definition(definition: &Value) -> Result<CardKind, GameError> {
    let object = definition
        .as_object()
        .ok_or_else(|| invalid("card definition must be an object"))?;
    let kind = card_kind(definition)?;
    if object
        .keys()
        .any(|field| !supported_card_field(kind, field))
    {
        return Err(invalid("card definition contains an unsupported field"));
    }
    for field in object.keys() {
        let admitted = match kind {
            CardKind::Avatar => matches!(
                field.as_str(),
                "attack"
                    | "cardType"
                    | "defense"
                    | "drawSpell"
                    | "life"
                    | "tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn"
            ),
            CardKind::Site => matches!(field.as_str(), "cardType" | "elements"),
            CardKind::Minion => matches!(
                field.as_str(),
                "attack" | "cardType" | "defense" | "manaCost" | "thresholds" | "token"
            ),
            CardKind::Artifact | CardKind::Aura | CardKind::Magic => {
                matches!(field.as_str(), "cardType" | "manaCost" | "thresholds")
            }
        };
        if !admitted {
            return Err(GameError::UnsupportedManifestFact(field.clone()));
        }
    }
    if matches!(kind, CardKind::Artifact | CardKind::Aura | CardKind::Magic) {
        return Err(GameError::UnsupportedManifestFact(format!(
            "{kind:?} card rules"
        )));
    }
    match kind {
        CardKind::Avatar => {
            validate_integer(definition, "attack", 0, Some(100))?;
            validate_integer(definition, "defense", 0, Some(100))?;
            validate_integer(definition, "life", 1, Some(100))?;
            if definition
                .get("drawSpell")
                .and_then(Value::as_bool)
                .is_none()
            {
                return Err(invalid("card.drawSpell must be boolean"));
            }
            if definition
                .get("tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn")
                .is_some_and(|value| value != &Value::Bool(true))
            {
                return Err(invalid(
                    "avatar air-threshold damage fact must be true when defined",
                ));
            }
        }
        CardKind::Site => {
            let elements = definition
                .get("elements")
                .and_then(Value::as_array)
                .ok_or_else(|| invalid("card.elements must be an array"))?;
            let canonical = ["earth", "fire", "water", "air"];
            let mut previous = None;
            for element in elements {
                let index = element
                    .as_str()
                    .and_then(|element| {
                        canonical.iter().position(|candidate| *candidate == element)
                    })
                    .ok_or_else(|| invalid("card.elements contains an unsupported element"))?;
                if previous.is_some_and(|previous| index <= previous) {
                    return Err(invalid(
                        "card.elements must be unique and canonically ordered",
                    ));
                }
                previous = Some(index);
            }
        }
        CardKind::Artifact | CardKind::Aura | CardKind::Magic | CardKind::Minion => {
            validate_integer(definition, "manaCost", 0, None)?;
            validate_thresholds(definition)?;
            if kind == CardKind::Minion {
                validate_integer(definition, "attack", 0, Some(100))?;
                validate_integer(definition, "defense", 0, Some(100))?;
                if definition
                    .get("token")
                    .is_some_and(|value| value != &Value::Bool(true))
                {
                    return Err(invalid("card.token must be true when defined"));
                }
            }
        }
    }
    Ok(kind)
}

fn supported_card_field(kind: CardKind, field: &str) -> bool {
    let fields = match kind {
        CardKind::Artifact => {
            "atEndOfEachTurnSiteControllerLosesLife bearerControllerChoosesExtraRandomOutcome cardType grantsBearerLethal grantsBearerPower manaCost tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath thresholds"
        }
        CardKind::Aura => {
            "atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep cardType immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns manaCost thresholds"
        }
        CardKind::Avatar => {
            "attack cardType defense drawSpell earthSitePlayCreatesAdjacentRubble life replaceAdjacentRubbleWithTopAtlasSite tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn"
        }
        CardKind::Magic => {
            "burrowAllMinionsAndArtifactsAtTargetLandSite burrowTargetMinionOrArtifact cardType damageChainNearbyUnits damageEachAbovegroundMinion damageEachUnitAtLocationWithinTwoSteps damageRandomUnitAtLocation damageTargetUnit damageUnitsAboveAndBelowTargetSiteByManhattanDistance disableTargetNearbyMinionUntilNextTurn discardSiteAsAdditionalCost destroyTargetSite fightAllyWithAdjacentEnemy gainControlOfTargetNearbyMinion grantChargeToAllyThisTurn grantPowerToAllyThisTurn healController killTargetWoundedMinion leapAttackAlly lureEnemyMinionOneStepCloser manaCost returnMinionFromOwnCemetery submergeTargetMinion summonRandomMinionFromAnyCemetery summonTokenToEachControlledSiteBorderingEnemySite targetNearby teleportAllyToTargetSite teleportNearbyAllyThenDrawCard thresholds untapTargetMinionAfterDamage"
        }
        CardKind::Minion => {
            "airborne atStartOfControllerTurnTeleportToRandomSiteOrVoid attack burrowing cannotAttackSites cannotDefend cannotDefendOrIntercept cardType charge connectsTopBottom deathriteDamageEachUnitHere deathriteDrawSite deathriteHeal deathriteLoseLifePerNearbySiteControlled defense diesAtEndOfControllerTurn discardRandomCardInsteadOfMana discardSpellToDamageRandomOtherUnitHere gainsPowerRangedAndSpellcasterAtopTower gainsStealthAtEndOfTurn gainsStealthAtEndOfTurnIfNoEnemiesNearby genesisDamageEachOtherUnitHere genesisDisableSelfUntilDamaged genesisDrawSite genesisDrawSpells genesisHealController genesisLoseControllerLife genesisMayDamageTargetAdjacentUnit genesisStrikeEachEnemyHere immobile lanceCount lethal manaCost mayRangedStrikeOnceDuringBasicMovement mayStepAfterRangedStrike mortal movementBonus movesOnlyForward movesOnlySideways mustBeCastBurrowed mustBeCastSubmerged mustBeCastToOuterColumn mustBeCastToWaterSite nearbyEnemiesPermanentlyLoseStealth occupiesSquareArea ordinary otherControlledMortalsPowerBonus otherNearbyAlliesPowerBonus preventsDamageFromUnitsWithPowerAtLeast provides ranged sacrificeMinionAtSummoningLocationForManaDiscount shootsDragProjectile siteProvidesNoThreshold spellcaster stealth strikesFirstWhileAttacking submerge summonToAnySite takesLessDamage tapForMana tapToDamageEachUnitAtAdjacentLocation tapToShootProjectileDamage thresholds token untapsAtEndOfControllerTurn voidwalk ward waterbound"
        }
        CardKind::Site => {
            "airborneMinionsAtopMoveFreelyAway blocksGroundMinionEntryWhileMinionAtop cannotBeMovedDestroyedOrModified cardType connectsBurrowedAllies elements flyToNearbyVoidOncePerTurnAtAirThreshold genesisDiscardTopSpells genesisDrawSpellPerAdjacentSameCard genesisEnemiesLoseStealth genesisGainMana genesisGainManaIfOnlyControlledCopy genesisHealNearbyAvatars genesisImmobilizeNearbyUntilNextTurn genesisMayBottomNextSpell genesisPayOneManaToSummonToken genesisReorderNextSpells isTower minionsHereGainVoidwalkUntilLeavingVoid ordinaryMinionManaDiscount preventsUnitsWithPowerAtLeastFromEntering rangedUnitsHereRangeBonus sacrificeToDestroyNearbySite"
        }
    };
    fields
        .split_ascii_whitespace()
        .any(|candidate| candidate == field)
}

fn validate_deck(deck: &Deck, cards: &BTreeMap<String, CardKind>) -> Result<(), GameError> {
    require_card_id(&deck.avatar, "deck.avatar")?;
    if cards.get(&deck.avatar) != Some(&CardKind::Avatar) {
        return Err(invalid("deck.avatar must reference an avatar"));
    }
    for (zone, expected) in [(&deck.atlas, None), (&deck.spellbook, Some(()))] {
        if !(3..=MAX_DECK_CARDS).contains(&zone.len()) {
            return Err(invalid("deck zone must contain 3-200 cards"));
        }
        for card_id in zone {
            require_card_id(card_id, "deck card")?;
            let kind = cards
                .get(card_id)
                .ok_or_else(|| invalid("deck references a missing card"))?;
            if expected.is_none() && *kind != CardKind::Site {
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
        validate_manifest(&raw, &manifest)?;

        let mut cards = Vec::with_capacity(manifest.cards.len());
        let mut card_ids = BTreeMap::new();
        for (index, (id, value)) in manifest.cards.iter().enumerate() {
            let id_number = u16::try_from(index)
                .map_err(|_| invalid("manifest contains too many card definitions"))?;
            let kind = card_kind(value)?;
            cards.push(CardDefinition {
                definition_hash: identity_hash(value)?,
                id: id.clone(),
                kind,
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
                phase: Phase::Mulligan,
                players: [north, south],
                prng,
                state_version: 0,
                turn_number: 0,
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

    /// Enumerates typed legal opening-hand actions in canonical order.
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] if action identity generation fails.
    pub fn legal_mulligans(&self) -> Result<Vec<MulliganAction>, GameError> {
        if self.position.phase != Phase::Mulligan {
            return Ok(Vec::new());
        }
        let seat = self.position.decision_seat;
        let player = &self.position.players[seat_index(seat)];
        let hand: Vec<_> = player
            .hand_atlas
            .iter()
            .chain(&player.hand_spellbook)
            .collect();
        let mut actions = Vec::with_capacity(76);
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
                    let descriptor = MulliganDescriptor {
                        atlas_order: atlas_order.clone(),
                        kind: MulliganKind::Mulligan,
                        spellbook_order,
                    };
                    let descriptor_value = serde_json::to_value(&descriptor)?;
                    let count = descriptor.atlas_order.len() + descriptor.spellbook_order.len();
                    actions.push((
                        crate::canonical::canonical_json(&descriptor_value)?,
                        MulliganAction {
                            action_id: opaque_action_id(
                                ENGINE_VERSION,
                                seat,
                                self.position.state_version,
                                &descriptor_value,
                            )?,
                            descriptor,
                            label: if count == 0 {
                                "Keep opening hand".to_owned()
                            } else {
                                format!(
                                    "Mulligan {count} ({} atlas, {} spellbook)",
                                    atlas.len(),
                                    spellbook.len()
                                )
                            },
                            seat,
                            state_version: self.position.state_version,
                        },
                    ));
                }
            }
        }
        actions.sort_unstable_by(|(left_key, left), (right_key, right)| {
            left_key
                .cmp(right_key)
                .then_with(|| left.action_id.cmp(&right.action_id))
        });
        Ok(actions.into_iter().map(|(_, action)| action).collect())
    }

    /// Applies an engine-issued opening-hand action without serialization or hashing.
    ///
    /// # Errors
    ///
    /// Returns [`GameError::IllegalAction`] when the action is stale or belongs to
    /// another decision.
    pub fn apply_mulligan(&mut self, action: &MulliganAction) -> Result<(), GameError> {
        if self.position.phase != Phase::Mulligan
            || action.seat != self.position.decision_seat
            || action.state_version != self.position.state_version
        {
            return Err(GameError::IllegalAction);
        }
        let player = &mut self.position.players[seat_index(action.seat)];
        resolve_mulligan_zone(
            &mut player.hand_atlas,
            &mut player.atlas,
            &action.descriptor.atlas_order,
        )?;
        resolve_mulligan_zone(
            &mut player.hand_spellbook,
            &mut player.spellbook,
            &action.descriptor.spellbook_order,
        )?;
        player.mulligan_complete = true;
        self.position.state_version += 1;
        if action.seat == Seat::North {
            self.position.active_seat = Seat::South;
            self.position.decision_seat = Seat::South;
        } else {
            self.position.active_seat = self.rules.first_seat;
            self.position.decision_seat = self.rules.first_seat;
            self.position.phase = Phase::Main;
            self.position.turn_number = 1;
        }
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
        json!({
            "activeSeat": self.position.active_seat,
            "cards": cards,
            "decisionSeat": self.position.decision_seat,
            "engine": {
                "prng": self.position.prng,
                "schemaVersion": 1,
                "stateVersion": self.position.state_version,
            },
            "pendingCombat": null,
            "phase": self.position.phase.as_str(),
            "players": {
                "north": self.player_value(&self.position.players[0]),
                "south": self.player_value(&self.position.players[1]),
            },
            "realm": { "sites": {}, "units": [] },
            "schemaVersion": 1,
            "stateVersion": self.position.state_version,
            "terminal": { "status": "active" },
            "turnNumber": self.position.turn_number,
        })
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
                "deathDoorTurn": null,
                "life": player.avatar.life,
                "location": player.avatar.location,
                "region": "surface",
                "tapped": false,
            },
            "cemetery": [],
            "domainEstablished": false,
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
}

impl MulliganAction {
    /// Returns the opaque action identity.
    #[must_use]
    pub const fn action_id(&self) -> &IdentityHash {
        &self.action_id
    }

    /// Returns the typed action descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &MulliganDescriptor {
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
    /// Returns [`serde_json::Error`] if the typed descriptor cannot be represented.
    pub fn to_legal_action(&self) -> Result<LegalAction, serde_json::Error> {
        Ok(LegalAction {
            action_id: self.action_id.clone(),
            descriptor: serde_json::to_value(&self.descriptor)?,
            label: self.label.clone(),
            seat: self.seat,
            state_version: self.state_version,
        })
    }
}

impl MulliganDescriptor {
    /// Returns selected Atlas instance IDs in bottom-deck order.
    #[must_use]
    pub fn atlas_order(&self) -> &[IdentityHash] {
        &self.atlas_order
    }

    /// Returns selected Spellbook instance IDs in bottom-deck order.
    #[must_use]
    pub fn spellbook_order(&self) -> &[IdentityHash] {
        &self.spellbook_order
    }
}

impl RulesContext {
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

fn validate_manifest(raw: &Value, manifest: &Manifest) -> Result<(), GameError> {
    if manifest.engine_version != ENGINE_VERSION || manifest.schema_version != 1 {
        return Err(invalid("manifest engine or schema version is unsupported"));
    }
    require_card_id(&manifest.authority.revision_id, "authority.revisionId")?;
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
    let mut kinds = BTreeMap::new();
    for (card_id, definition) in &manifest.cards {
        require_card_id(card_id, "cards key")?;
        kinds.insert(card_id.clone(), validate_card_definition(definition)?);
    }
    validate_deck(&manifest.decks.north, &kinds)?;
    validate_deck(&manifest.decks.south, &kinds)?;

    for deck in [&manifest.decks.north, &manifest.decks.south] {
        for card_id in &deck.spellbook {
            if manifest.cards[card_id].get("token") == Some(&Value::Bool(true)) {
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
        let definition = &manifest.cards[card_id];
        let token_id = definition
            .get("summonTokenToEachControlledSiteBorderingEnemySite")
            .or_else(|| definition.get("genesisPayOneManaToSummonToken"))
            .and_then(Value::as_str);
        if let Some(token_id) = token_id {
            let token = manifest.cards.get(token_id);
            if token
                .and_then(|value| value.get("cardType"))
                .and_then(Value::as_str)
                != Some("minion")
                || token
                    .and_then(|value| value.get("token"))
                    .and_then(Value::as_bool)
                    != Some(true)
            {
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
    Ok(())
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
    if avatar_definition.kind != CardKind::Avatar {
        return Err(invalid("validated deck lacks avatar definition"));
    }
    let life = avatar_definition
        .value
        .get("life")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(|| invalid("validated avatar lacks supported life"))?;
    let avatar = AvatarPosition {
        card: card_instance(rules, avatar_card_id, seat, CardSource::Avatar, 0)?,
        life,
        location: if seat == Seat::North { "C4" } else { "C1" },
    };
    let remaining_atlas = atlas.split_off(3);
    let remaining_spellbook = spellbook.split_off(3);
    Ok(PlayerPosition {
        air_thresholds_cast_this_turn: avatar_definition
            .value
            .get("tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn")
            .is_some_and(|value| value == &Value::Bool(true))
            .then_some(0),
        atlas: remaining_atlas,
        avatar,
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
