//! Deterministic deck validation and local price accounting.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::canonical::{CanonicalError, IdentityHash, identity_hash};

const ENGINE_MIN_ZONE_CARDS: u32 = 3;
const ENGINE_MAX_ZONE_CARDS: u32 = 200;
const MAX_CARD_ID_BYTES: usize = 256;

/// A card's deck-building category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CardType {
    /// A player's single avatar.
    Avatar,
    /// A site placed in the Atlas.
    Site,
    /// A summoned minion.
    Minion,
    /// An attached artifact.
    Artifact,
    /// A persistent aura.
    Aura,
    /// A one-shot magic card.
    Magic,
}

/// A rarity with a modeled Constructed copy limit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rarity {
    /// Up to four copies.
    Ordinary,
    /// Up to three copies.
    Exceptional,
    /// Up to two copies.
    Elite,
    /// Up to one copy.
    Unique,
}

/// The zone containing a deck entry.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeckZone {
    /// The avatar slot.
    Avatar,
    /// The Atlas.
    Atlas,
    /// The Spellbook.
    Spellbook,
}

/// A card and positive intended copy count.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardCount {
    /// Stable authority card identity.
    pub card_id: String,
    /// Number of copies.
    pub copies: u32,
}

/// A count-based candidate deck. Atlas and Spellbook row order is not meaningful.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDeck {
    /// The avatar's stable card identity.
    pub avatar: String,
    /// Counted Atlas entries.
    pub atlas: Vec<CardCount>,
    /// Counted Spellbook entries.
    pub spellbook: Vec<CardCount>,
}

/// A candidate deck normalized into canonical stable-ID order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalDeck {
    /// The avatar's stable card identity.
    pub avatar: String,
    /// Canonically ordered Atlas entries.
    pub atlas: Vec<CardCount>,
    /// Canonically ordered Spellbook entries.
    pub spellbook: Vec<CardCount>,
}

/// The sole currently modeled deck format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatContext {
    stable_id: &'static str,
    atlas_minimum: u32,
    avatar_count: u32,
    spellbook_minimum: u32,
}

static CONSTRUCTED: FormatContext = FormatContext {
    stable_id: "format:modeled-constructed-v1",
    atlas_minimum: 30,
    avatar_count: 1,
    spellbook_minimum: 60,
};

impl FormatContext {
    /// Returns the immutable currently modeled Constructed format.
    #[must_use]
    pub const fn constructed() -> &'static Self {
        &CONSTRUCTED
    }

    /// Returns the format identity included in canonical deck identities.
    #[must_use]
    pub const fn stable_id(&self) -> &'static str {
        self.stable_id
    }

    /// Returns the required Atlas size.
    #[must_use]
    pub const fn atlas_minimum(&self) -> u32 {
        self.atlas_minimum
    }

    /// Returns the required number of avatars.
    #[must_use]
    pub const fn avatar_count(&self) -> u32 {
        self.avatar_count
    }

    /// Returns the required Spellbook size.
    #[must_use]
    pub const fn spellbook_minimum(&self) -> u32 {
        self.spellbook_minimum
    }

    /// Returns the copy limit for `rarity`.
    #[must_use]
    pub const fn copy_limit(&self, rarity: Rarity) -> u32 {
        match rarity {
            Rarity::Ordinary => 4,
            Rarity::Exceptional => 3,
            Rarity::Elite => 2,
            Rarity::Unique => 1,
        }
    }
}

/// A deck card's relationship to the official pricing identity space.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OfficialCardMapping {
    /// Authority has no official pricing identity for the card.
    Unavailable,
    /// Authority has one exact official card identity.
    Exact(String),
    /// Authority has multiple possible official card identities and cannot guess.
    Ambiguous(Vec<String>),
}

/// Static catalog information required by deck validation and pricing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardCatalogEntry {
    /// Stable authority card identity used in manifests.
    pub stable_id: String,
    /// Card category.
    pub card_type: CardType,
    /// Rarity, when authority supplies one.
    pub rarity: Option<Rarity>,
    /// Whether the authoritative Rust engine supports all bound mechanics.
    pub engine_supported: bool,
    /// Mapping to the official pricing identity space.
    pub official_mapping: OfficialCardMapping,
    /// Whether this minion is a generated token rather than a deck card.
    pub token: bool,
}

/// A deterministic deck validation diagnostic.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DeckDiagnostic {
    /// A card ID is blank or exceeds the engine boundary.
    InvalidCardId { card_id: String, zone: DeckZone },
    /// A counted row has no copies.
    ZeroCopies { card_id: String, zone: DeckZone },
    /// A zone repeats a counted card row.
    DuplicateCard { card_id: String, zone: DeckZone },
    /// The authority catalog has no such stable card identity.
    UnknownCard { card_id: String, zone: DeckZone },
    /// The card category cannot occur in this zone.
    WrongCardType {
        actual: CardType,
        card_id: String,
        zone: DeckZone,
    },
    /// A token minion was included in the Spellbook.
    TokenInSpellbook { card_id: String },
    /// A deck card has no rarity and therefore no modeled copy limit.
    MissingRarity { card_id: String },
    /// The deck exceeds a modeled rarity copy limit.
    CopyLimitExceeded {
        actual: u32,
        card_id: String,
        limit: u32,
        rarity: Rarity,
    },
    /// The Atlas is smaller than the modeled format minimum.
    AtlasMinimum { actual: u32, required: u32 },
    /// The Spellbook is smaller than the modeled format minimum.
    SpellbookMinimum { actual: u32, required: u32 },
    /// An engine zone contains fewer than three cards.
    EngineZoneMinimum {
        actual: u32,
        required: u32,
        zone: DeckZone,
    },
    /// An engine zone contains more than 200 cards.
    EngineZoneMaximum {
        actual: u32,
        limit: u32,
        zone: DeckZone,
    },
    /// The card is format-legal but has unsupported authoritative mechanics.
    UnsupportedCard { card_id: String },
}

impl DeckDiagnostic {
    fn invalidates_format(&self) -> bool {
        !matches!(
            self,
            Self::EngineZoneMinimum { .. }
                | Self::EngineZoneMaximum { .. }
                | Self::UnsupportedCard { .. }
        )
    }

    fn invalidates_engine_support(&self) -> bool {
        !matches!(
            self,
            Self::AtlasMinimum { .. }
                | Self::SpellbookMinimum { .. }
                | Self::MissingRarity { .. }
                | Self::CopyLimitExceeded { .. }
        )
    }
}

/// Canonical composition, identity, and independent eligibility decisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeckValidation {
    /// Canonical counted deck.
    deck: CanonicalDeck,
    /// Identity of the format plus canonical composition.
    deck_id: IdentityHash,
    /// Whether all currently modeled Constructed rules pass.
    format_legal: bool,
    /// Whether the current engine can instantiate every card and zone.
    engine_supported: bool,
    /// Whether the candidate is both format-legal and engine-supported.
    ranked_eligible: bool,
    /// Deterministically ordered validation findings.
    diagnostics: Vec<DeckDiagnostic>,
}

impl DeckValidation {
    /// Returns the canonical counted deck.
    #[must_use]
    pub const fn deck(&self) -> &CanonicalDeck {
        &self.deck
    }

    /// Returns the identity of the format plus canonical composition.
    #[must_use]
    pub const fn deck_id(&self) -> &IdentityHash {
        &self.deck_id
    }

    /// Returns whether all currently modeled format rules pass.
    #[must_use]
    pub const fn format_legal(&self) -> bool {
        self.format_legal
    }

    /// Returns whether the engine can instantiate every card and zone.
    #[must_use]
    pub const fn engine_supported(&self) -> bool {
        self.engine_supported
    }

    /// Returns whether the deck is both format-legal and engine-supported.
    #[must_use]
    pub const fn ranked_eligible(&self) -> bool {
        self.ranked_eligible
    }

    /// Returns deterministically ordered validation findings.
    #[must_use]
    pub fn diagnostics(&self) -> &[DeckDiagnostic] {
        &self.diagnostics
    }
}

/// Deck validation could not produce a trustworthy result.
#[derive(Debug)]
pub enum DeckError {
    /// Count aggregation overflowed.
    CopyCountOverflow { card_id: String, zone: DeckZone },
    /// A zone's aggregate copy count overflowed.
    ZoneCountOverflow(DeckZone),
    /// Authority supplied the same catalog card identity more than once.
    DuplicateCatalogCard(String),
    /// Canonical identity generation failed.
    Canonical(CanonicalError),
}

impl fmt::Display for DeckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CopyCountOverflow { card_id, zone } => {
                write!(formatter, "copy count overflow for {card_id} in {zone:?}")
            }
            Self::ZoneCountOverflow(zone) => {
                write!(formatter, "total copy count overflow in {zone:?}")
            }
            Self::DuplicateCatalogCard(card_id) => {
                write!(formatter, "duplicate catalog card identity: {card_id}")
            }
            Self::Canonical(error) => error.fmt(formatter),
        }
    }
}

impl Error for DeckError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            Self::CopyCountOverflow { .. }
            | Self::ZoneCountOverflow(_)
            | Self::DuplicateCatalogCard(_) => None,
        }
    }
}

impl From<CanonicalError> for DeckError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

fn compare_ids(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn valid_card_id(card_id: &str) -> bool {
    !card_id.trim().is_empty() && card_id.len() <= MAX_CARD_ID_BYTES
}

fn canonicalize_rows(
    rows: Vec<CardCount>,
    zone: DeckZone,
    diagnostics: &mut Vec<DeckDiagnostic>,
) -> Result<Vec<CardCount>, DeckError> {
    let mut counts = BTreeMap::<String, u32>::new();
    for row in rows {
        if !valid_card_id(&row.card_id) {
            diagnostics.push(DeckDiagnostic::InvalidCardId {
                card_id: row.card_id.clone(),
                zone,
            });
        }
        if row.copies == 0 {
            diagnostics.push(DeckDiagnostic::ZeroCopies {
                card_id: row.card_id,
                zone,
            });
            continue;
        }
        if let Some(existing) = counts.get_mut(&row.card_id) {
            diagnostics.push(DeckDiagnostic::DuplicateCard {
                card_id: row.card_id.clone(),
                zone,
            });
            *existing = existing
                .checked_add(row.copies)
                .ok_or(DeckError::CopyCountOverflow {
                    card_id: row.card_id,
                    zone,
                })?;
        } else {
            counts.insert(row.card_id, row.copies);
        }
    }
    let mut canonical: Vec<_> = counts
        .into_iter()
        .map(|(card_id, copies)| CardCount { card_id, copies })
        .collect();
    canonical.sort_unstable_by(|left, right| compare_ids(&left.card_id, &right.card_id));
    Ok(canonical)
}

fn catalog_by_id(
    cards: &[CardCatalogEntry],
) -> Result<BTreeMap<&str, &CardCatalogEntry>, DeckError> {
    let mut catalog = BTreeMap::new();
    for card in cards {
        if catalog.insert(card.stable_id.as_str(), card).is_some() {
            return Err(DeckError::DuplicateCatalogCard(card.stable_id.clone()));
        }
    }
    Ok(catalog)
}

fn zone_total(rows: &[CardCount], zone: DeckZone) -> Result<u32, DeckError> {
    rows.iter().try_fold(0_u32, |total, row| {
        total
            .checked_add(row.copies)
            .ok_or(DeckError::ZoneCountOverflow(zone))
    })
}

fn validate_zone_shape(
    rows: &[CardCount],
    zone: DeckZone,
    catalog: &BTreeMap<&str, &CardCatalogEntry>,
    diagnostics: &mut Vec<DeckDiagnostic>,
) {
    for row in rows {
        let Some(card) = catalog.get(row.card_id.as_str()) else {
            diagnostics.push(DeckDiagnostic::UnknownCard {
                card_id: row.card_id.clone(),
                zone,
            });
            continue;
        };
        let valid_type = match zone {
            DeckZone::Avatar => card.card_type == CardType::Avatar,
            DeckZone::Atlas => card.card_type == CardType::Site,
            DeckZone::Spellbook => matches!(
                card.card_type,
                CardType::Artifact | CardType::Aura | CardType::Magic | CardType::Minion
            ),
        };
        if !valid_type {
            diagnostics.push(DeckDiagnostic::WrongCardType {
                actual: card.card_type,
                card_id: row.card_id.clone(),
                zone,
            });
        }
        if zone == DeckZone::Spellbook && card.card_type == CardType::Minion && card.token {
            diagnostics.push(DeckDiagnostic::TokenInSpellbook {
                card_id: row.card_id.clone(),
            });
        }
        if !card.engine_supported {
            diagnostics.push(DeckDiagnostic::UnsupportedCard {
                card_id: row.card_id.clone(),
            });
        }
    }
}

fn validate_copy_limits(
    deck: &CanonicalDeck,
    format: &FormatContext,
    catalog: &BTreeMap<&str, &CardCatalogEntry>,
    diagnostics: &mut Vec<DeckDiagnostic>,
) {
    for row in deck.atlas.iter().chain(&deck.spellbook) {
        let Some(card) = catalog.get(row.card_id.as_str()) else {
            continue;
        };
        let Some(rarity) = card.rarity else {
            diagnostics.push(DeckDiagnostic::MissingRarity {
                card_id: row.card_id.clone(),
            });
            continue;
        };
        let limit = format.copy_limit(rarity);
        if row.copies > limit {
            diagnostics.push(DeckDiagnostic::CopyLimitExceeded {
                actual: row.copies,
                card_id: row.card_id.clone(),
                limit,
                rarity,
            });
        }
    }
}

/// Canonicalizes and validates a candidate against modeled Constructed rules and engine support.
///
/// # Errors
///
/// Returns [`DeckError`] for authority catalog duplicates, copy-count overflow,
/// or canonical identity failure.
pub fn validate_deck(
    candidate: CandidateDeck,
    cards: &[CardCatalogEntry],
    format: &FormatContext,
) -> Result<DeckValidation, DeckError> {
    let mut diagnostics = Vec::new();
    if !valid_card_id(&candidate.avatar) {
        diagnostics.push(DeckDiagnostic::InvalidCardId {
            card_id: candidate.avatar.clone(),
            zone: DeckZone::Avatar,
        });
    }
    let deck = CanonicalDeck {
        avatar: candidate.avatar,
        atlas: canonicalize_rows(candidate.atlas, DeckZone::Atlas, &mut diagnostics)?,
        spellbook: canonicalize_rows(candidate.spellbook, DeckZone::Spellbook, &mut diagnostics)?,
    };
    let catalog = catalog_by_id(cards)?;
    validate_zone_shape(
        &[CardCount {
            card_id: deck.avatar.clone(),
            copies: format.avatar_count(),
        }],
        DeckZone::Avatar,
        &catalog,
        &mut diagnostics,
    );
    validate_zone_shape(&deck.atlas, DeckZone::Atlas, &catalog, &mut diagnostics);
    validate_zone_shape(
        &deck.spellbook,
        DeckZone::Spellbook,
        &catalog,
        &mut diagnostics,
    );

    let atlas_total = zone_total(&deck.atlas, DeckZone::Atlas)?;
    let spellbook_total = zone_total(&deck.spellbook, DeckZone::Spellbook)?;
    if atlas_total < format.atlas_minimum() {
        diagnostics.push(DeckDiagnostic::AtlasMinimum {
            actual: atlas_total,
            required: format.atlas_minimum(),
        });
    }
    if spellbook_total < format.spellbook_minimum() {
        diagnostics.push(DeckDiagnostic::SpellbookMinimum {
            actual: spellbook_total,
            required: format.spellbook_minimum(),
        });
    }
    for (zone, total) in [
        (DeckZone::Atlas, atlas_total),
        (DeckZone::Spellbook, spellbook_total),
    ] {
        if total < ENGINE_MIN_ZONE_CARDS {
            diagnostics.push(DeckDiagnostic::EngineZoneMinimum {
                actual: total,
                required: ENGINE_MIN_ZONE_CARDS,
                zone,
            });
        }
        if total > ENGINE_MAX_ZONE_CARDS {
            diagnostics.push(DeckDiagnostic::EngineZoneMaximum {
                actual: total,
                limit: ENGINE_MAX_ZONE_CARDS,
                zone,
            });
        }
    }
    validate_copy_limits(&deck, format, &catalog, &mut diagnostics);
    diagnostics.sort_unstable();

    let deck_id = identity_hash(&json!({
        "deck": deck,
        "formatId": format.stable_id(),
    }))?;
    let format_legal = diagnostics
        .iter()
        .all(|diagnostic| !diagnostic.invalidates_format());
    let engine_supported = diagnostics
        .iter()
        .all(|diagnostic| !diagnostic.invalidates_engine_support());

    Ok(DeckValidation {
        deck,
        deck_id,
        format_legal,
        engine_supported,
        ranked_eligible: format_legal && engine_supported,
        diagnostics,
    })
}

/// A complete market identity for one printing price.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceKey {
    /// Official card identity.
    pub official_card_id: String,
    /// Official printing identity.
    pub printing_id: String,
    /// Printing variant, such as standard or foil.
    pub variant: String,
    /// Physical condition.
    pub condition: String,
    /// Currency code; no conversion is performed.
    pub currency: String,
    /// Price source identity.
    pub source: String,
}

/// One integer-cents price for a fully qualified printing key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintingPrice {
    /// Fully qualified printing key.
    pub key: PriceKey,
    /// Unit price in the currency's cent-like minor unit.
    pub unit_price_cents: u64,
}

/// One immutable, validated local price snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceSnapshot {
    snapshot_id: String,
    prices: BTreeMap<PriceKey, u64>,
}

/// Exact dimensions eligible for minimum-cost printing selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceScope {
    /// Printing variant.
    pub variant: String,
    /// Physical condition.
    pub condition: String,
    /// Currency code.
    pub currency: String,
    /// Price source identity.
    pub source: String,
}

/// A price snapshot is malformed or arithmetic overflowed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PriceError {
    /// Snapshot identity is blank.
    EmptySnapshotId,
    /// A required key field is blank.
    EmptyKeyField(&'static str),
    /// The snapshot repeats an exact qualified price key.
    DuplicatePriceKey(Box<PriceKey>),
    /// A line or total exceeded integer cents.
    CostOverflow,
    /// Authority supplied the same catalog card identity more than once.
    DuplicateCatalogCard(String),
}

impl fmt::Display for PriceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySnapshotId => {
                formatter.write_str("price snapshot identity must be nonempty")
            }
            Self::EmptyKeyField(field) => write!(formatter, "price key {field} must be nonempty"),
            Self::DuplicatePriceKey(key) => write!(
                formatter,
                "duplicate price key for card {} printing {}",
                key.official_card_id, key.printing_id
            ),
            Self::CostOverflow => formatter.write_str("deck price exceeds integer cents"),
            Self::DuplicateCatalogCard(card_id) => {
                write!(formatter, "duplicate catalog card identity: {card_id}")
            }
        }
    }
}

impl Error for PriceError {}

impl PriceSnapshot {
    /// Validates and indexes one local snapshot without coalescing market dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`PriceError`] for blank identities or duplicate qualified keys.
    pub fn new(snapshot_id: String, prices: Vec<PrintingPrice>) -> Result<Self, PriceError> {
        if snapshot_id.trim().is_empty() {
            return Err(PriceError::EmptySnapshotId);
        }
        let mut indexed = BTreeMap::new();
        for price in prices {
            for (field, value) in [
                ("officialCardId", price.key.official_card_id.as_str()),
                ("printingId", price.key.printing_id.as_str()),
                ("variant", price.key.variant.as_str()),
                ("condition", price.key.condition.as_str()),
                ("currency", price.key.currency.as_str()),
                ("source", price.key.source.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(PriceError::EmptyKeyField(field));
                }
            }
            if indexed
                .insert(price.key.clone(), price.unit_price_cents)
                .is_some()
            {
                return Err(PriceError::DuplicatePriceKey(Box::new(price.key)));
            }
        }
        Ok(Self {
            snapshot_id,
            prices: indexed,
        })
    }

    /// Returns the caller-supplied immutable snapshot identity.
    #[must_use]
    pub fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }

    fn cheapest<'a>(
        &'a self,
        official_card_id: &str,
        scope: &PriceScope,
    ) -> Option<(&'a PriceKey, u64)> {
        self.prices
            .iter()
            .filter(|(key, _)| {
                key.official_card_id == official_card_id
                    && key.variant == scope.variant
                    && key.condition == scope.condition
                    && key.currency == scope.currency
                    && key.source == scope.source
            })
            .min_by(|(left_key, left_price), (right_key, right_price)| {
                left_price
                    .cmp(right_price)
                    .then_with(|| left_key.cmp(right_key))
            })
            .map(|(key, price)| (key, *price))
    }
}

/// Why a deck cost line could not be priced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriceUnavailableReason {
    /// No official card identity exists.
    OfficialMappingUnavailable,
    /// No printing price matches the exact requested market scope.
    NoMatchingPrice,
    /// The card was absent from the supplied authority catalog.
    CardNotInCatalog,
}

/// Pricing result for one canonical deck line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeckCostStatus {
    /// One deterministic minimum-cost printing was selected.
    Priced {
        /// Selected fully qualified printing key.
        key: PriceKey,
        /// Exact unit price.
        unit_price_cents: u64,
        /// Exact unit price multiplied by copies.
        line_total_cents: u64,
    },
    /// The card-to-official identity mapping has multiple unresolved candidates.
    AmbiguousMapping { official_card_ids: Vec<String> },
    /// No price can be selected for the stated reason.
    Unavailable { reason: PriceUnavailableReason },
}

/// One canonically ordered deck cost line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeckCostLine {
    /// Stable authority card identity.
    pub card_id: String,
    /// Deck zone.
    pub zone: DeckZone,
    /// Number of copies priced.
    pub copies: u32,
    /// Explicit pricing outcome.
    pub status: DeckCostStatus,
}

/// Exact local cost calculation for a canonical deck and market scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeckCost {
    /// Canonical deck identity.
    pub deck_id: IdentityHash,
    /// Price snapshot identity.
    pub snapshot_id: String,
    /// Exact currency requested by the caller.
    pub currency: String,
    /// Canonically ordered lines.
    pub lines: Vec<DeckCostLine>,
    /// Full deck total, absent when any line is unavailable or ambiguous.
    pub total_cents: Option<u64>,
}

fn cost_line(
    card_id: &str,
    copies: u32,
    zone: DeckZone,
    catalog: &BTreeMap<&str, &CardCatalogEntry>,
    snapshot: &PriceSnapshot,
    scope: &PriceScope,
) -> Result<DeckCostLine, PriceError> {
    let status = match catalog.get(card_id) {
        None => DeckCostStatus::Unavailable {
            reason: PriceUnavailableReason::CardNotInCatalog,
        },
        Some(card) => match &card.official_mapping {
            OfficialCardMapping::Unavailable => DeckCostStatus::Unavailable {
                reason: PriceUnavailableReason::OfficialMappingUnavailable,
            },
            OfficialCardMapping::Ambiguous(official_card_ids) => {
                let mut ids = official_card_ids.clone();
                ids.sort_unstable_by(|left, right| compare_ids(left, right));
                ids.dedup();
                if ids.is_empty() {
                    DeckCostStatus::Unavailable {
                        reason: PriceUnavailableReason::OfficialMappingUnavailable,
                    }
                } else {
                    DeckCostStatus::AmbiguousMapping {
                        official_card_ids: ids,
                    }
                }
            }
            OfficialCardMapping::Exact(official_card_id) if official_card_id.trim().is_empty() => {
                DeckCostStatus::Unavailable {
                    reason: PriceUnavailableReason::OfficialMappingUnavailable,
                }
            }
            OfficialCardMapping::Exact(official_card_id) => {
                let Some((key, unit_price_cents)) = snapshot.cheapest(official_card_id, scope)
                else {
                    return Ok(DeckCostLine {
                        card_id: card_id.to_owned(),
                        zone,
                        copies,
                        status: DeckCostStatus::Unavailable {
                            reason: PriceUnavailableReason::NoMatchingPrice,
                        },
                    });
                };
                let line_total_cents = unit_price_cents
                    .checked_mul(u64::from(copies))
                    .ok_or(PriceError::CostOverflow)?;
                DeckCostStatus::Priced {
                    key: key.clone(),
                    unit_price_cents,
                    line_total_cents,
                }
            }
        },
    };
    Ok(DeckCostLine {
        card_id: card_id.to_owned(),
        zone,
        copies,
        status,
    })
}

/// Selects deterministic minimum-cost printings and totals exact integer cents.
///
/// Variant, condition, currency, and source must all exactly match `scope`;
/// this function never converts or coalesces market data.
///
/// # Errors
///
/// Returns [`PriceError`] for duplicate catalog identities or integer overflow.
pub fn price_deck(
    validation: &DeckValidation,
    cards: &[CardCatalogEntry],
    snapshot: &PriceSnapshot,
    scope: &PriceScope,
) -> Result<DeckCost, PriceError> {
    let mut catalog = BTreeMap::new();
    for card in cards {
        if catalog.insert(card.stable_id.as_str(), card).is_some() {
            return Err(PriceError::DuplicateCatalogCard(card.stable_id.clone()));
        }
    }
    let mut lines =
        Vec::with_capacity(1 + validation.deck.atlas.len() + validation.deck.spellbook.len());
    lines.push(cost_line(
        &validation.deck.avatar,
        1,
        DeckZone::Avatar,
        &catalog,
        snapshot,
        scope,
    )?);
    for (zone, rows) in [
        (DeckZone::Atlas, validation.deck.atlas.as_slice()),
        (DeckZone::Spellbook, validation.deck.spellbook.as_slice()),
    ] {
        for row in rows {
            lines.push(cost_line(
                &row.card_id,
                row.copies,
                zone,
                &catalog,
                snapshot,
                scope,
            )?);
        }
    }

    let mut total_cents = Some(0_u64);
    for line in &lines {
        let DeckCostStatus::Priced {
            line_total_cents, ..
        } = &line.status
        else {
            total_cents = None;
            break;
        };
        total_cents = Some(
            total_cents
                .and_then(|total| total.checked_add(*line_total_cents))
                .ok_or(PriceError::CostOverflow)?,
        );
    }

    Ok(DeckCost {
        deck_id: validation.deck_id.clone(),
        snapshot_id: snapshot.snapshot_id().to_owned(),
        currency: scope.currency.clone(),
        lines,
        total_cents,
    })
}
