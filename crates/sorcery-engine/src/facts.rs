//! Validated, normalized card facts used by the authoritative engine.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde_json::{Map, Value};

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_SAFE_INTEGER_F64: f64 = 9_007_199_254_740_991.0;
const MAX_COMBAT_STAT: u64 = 100;
const MAX_DECK_CARDS: u64 = 200;

const ELEMENT_FIELDS: [&str; 4] = ["earth", "fire", "water", "air"];

/// An elemental threshold or affinity in canonical rules order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Element {
    /// Earth.
    Earth,
    /// Fire.
    Fire,
    /// Water.
    Water,
    /// Air.
    Air,
}

impl Element {
    const ALL: [Self; 4] = [Self::Earth, Self::Fire, Self::Water, Self::Air];

    const fn index(self) -> usize {
        self as usize
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "earth" => Some(Self::Earth),
            "fire" => Some(Self::Fire),
            "water" => Some(Self::Water),
            "air" => Some(Self::Air),
            _ => None,
        }
    }
}

/// A compact set of elements.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ElementSet(u8);

impl ElementSet {
    /// Returns whether the set contains `element`.
    #[must_use]
    pub const fn contains(self, element: Element) -> bool {
        self.0 & (1 << element.index()) != 0
    }

    /// Iterates present elements in canonical rules order.
    pub fn iter(self) -> impl Iterator<Item = Element> {
        Element::ALL
            .into_iter()
            .filter(move |element| self.contains(*element))
    }
}

/// Four elemental thresholds stored as Earth, Fire, Water, Air.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Thresholds([u64; 4]);

impl Thresholds {
    /// Returns the threshold for `element`.
    #[must_use]
    pub const fn get(self, element: Element) -> u64 {
        self.0[element.index()]
    }

    /// Returns thresholds in canonical Earth, Fire, Water, Air order.
    #[must_use]
    pub const fn canonical(self) -> [u64; 4] {
        self.0
    }
}

/// A validated card definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardFacts {
    /// Avatar facts.
    Avatar(AvatarFacts),
    /// Artifact facts.
    Artifact(ArtifactFacts),
    /// Aura facts.
    Aura(AuraFacts),
    /// Magic facts.
    Magic(MagicFacts),
    /// Minion facts.
    Minion(MinionFacts),
    /// Site facts.
    Site(SiteFacts),
}

/// Avatar facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "named normalized rule facts avoid invalid Option<bool> states"
)]
pub struct AvatarFacts {
    pub attack: u8,
    pub defense: u8,
    pub draw_spell: bool,
    pub earth_site_play_creates_adjacent_rubble: bool,
    pub life: u8,
    pub replace_adjacent_rubble_with_top_atlas_site: bool,
    pub tap_damage_random_other_unit_at_nearby_location_per_air_threshold_cast_this_turn: bool,
}

/// Site facts.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "named rule facts are clearer and safer than positional bits"
)]
pub struct SiteFacts {
    pub airborne_minions_atop_move_freely_away: bool,
    pub blocks_ground_minion_entry_while_minion_atop: bool,
    pub cannot_be_moved_destroyed_or_modified: bool,
    pub connects_burrowed_allies: bool,
    pub elements: ElementSet,
    pub fly_to_nearby_void_once_per_turn_at_air_threshold: bool,
    pub genesis_discard_top_spells: bool,
    pub genesis_draw_spell_per_adjacent_same_card: bool,
    pub genesis_enemies_lose_stealth: bool,
    pub genesis_gain_mana: Option<u8>,
    pub genesis_gain_mana_if_only_controlled_copy: bool,
    pub genesis_heal_nearby_avatars: bool,
    pub genesis_immobilize_nearby_until_next_turn: bool,
    pub genesis_may_bottom_next_spell: bool,
    pub genesis_pay_one_mana_to_summon_token: Option<String>,
    pub genesis_reorder_next_spells: bool,
    pub is_tower: bool,
    pub minions_here_gain_voidwalk_until_leaving_void: bool,
    pub ordinary_minion_mana_discount: bool,
    pub prevents_units_with_power_at_least_from_entering: Option<u8>,
    pub ranged_units_here_range_bonus: bool,
    pub sacrifice_to_destroy_nearby_site: bool,
}

/// The single supported effect carried by an Artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactEffect {
    AtEndOfEachTurnSiteControllerLosesLife(u8),
    BearerControllerChoosesExtraRandomOutcome,
    GrantsBearerLethal,
    GrantsBearerPowerTwo,
    TapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps,
    TapBearerAndAnotherAllyHereToDamageTargetWithinTwoStepsThree,
    TapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPathFour,
}

/// Artifact facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactFacts {
    pub effect: ArtifactEffect,
    pub mana_cost: u64,
    pub thresholds: Thresholds,
}

/// The single supported effect carried by an Aura.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuraEffect {
    AtEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStepThree,
    ImmobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns,
}

/// Aura facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuraFacts {
    pub effect: AuraEffect,
    pub mana_cost: u64,
    pub thresholds: Thresholds,
}

/// The single supported effect carried by Magic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MagicEffect {
    BurrowAllMinionsAndArtifactsAtTargetLandSite,
    BurrowTargetMinionOrArtifact,
    DamageChainNearbyUnits,
    DamageEachAbovegroundMinionOne,
    DamageEachUnitAtLocationWithinTwoSteps(u8),
    DamageRandomUnitAtLocation(u8),
    DamageTargetUnit {
        amount: u8,
        target_nearby: bool,
        untap_target_minion_after_damage: bool,
    },
    DestroyTargetSiteWithDamageGrid([u8; 5]),
    DisableTargetNearbyMinionUntilNextTurn,
    FightAllyWithAdjacentEnemy,
    GainControlOfTargetNearbyMinion,
    GrantChargeToAllyThisTurn,
    GrantPowerTwoToAllyThisTurn,
    HealController(u8),
    KillTargetWoundedMinion,
    LeapAttackAlly,
    LureEnemyMinionOneStepCloser,
    ReturnMinionFromOwnCemetery,
    SubmergeTargetMinion,
    SummonRandomMinionFromAnyCemetery,
    SummonTokenToEachControlledSiteBorderingEnemySite(String),
    TeleportAllyToTargetSite,
    TeleportNearbyAllyThenDrawCard,
}

/// Magic facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MagicFacts {
    pub effect: MagicEffect,
    pub mana_cost: u64,
    pub thresholds: Thresholds,
}

/// A minion's mutually exclusive Genesis effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinionGenesis {
    DamageEachOtherUnitHereOne,
    DisableSelfUntilDamaged,
    DrawSite,
    DrawSpells(u8),
    HealControllerTwo,
    LoseControllerLifeTwo,
    MayDamageTargetAdjacentUnitTwo,
    StrikeEachEnemyHere,
}

/// A minion's alternative summon payment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlternativeSummonPayment {
    DiscardRandomCardInsteadOfMana,
    SacrificeMinionAtSummoningLocationForManaDiscountTwo,
}

/// A minion's end-turn Stealth rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndTurnStealth {
    Always,
    IfNoEnemiesNearby,
}

/// A minion's basic movement restriction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicMovementRestriction {
    ForwardOnly,
    SidewaysOnly,
}

/// A minion's required cast region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequiredCastRegion {
    Underground,
    Underwater,
}

/// A minion's single supported damage-prevention rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DamagePrevention {
    TakesOneLessDamage,
    PreventsDamageFromUnitsWithPowerAtLeast(u8),
    Ward,
}

/// Minion facts.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "named normalized rule facts avoid invalid Option<bool> states"
)]
pub struct MinionFacts {
    pub airborne: bool,
    pub alternative_summon_payment: Option<AlternativeSummonPayment>,
    pub at_start_of_controller_turn_teleport_to_random_site_or_void: bool,
    pub attack: u8,
    pub burrowing: bool,
    pub cannot_attack_sites: bool,
    pub cannot_defend: bool,
    pub cannot_defend_or_intercept: bool,
    pub charge: bool,
    pub connects_top_bottom: bool,
    pub damage_prevention: Option<DamagePrevention>,
    pub deathrite_damage_each_unit_here: Option<u8>,
    pub deathrite_draw_site: bool,
    pub deathrite_heal: Option<u8>,
    pub deathrite_lose_life_per_nearby_site_controlled: bool,
    pub defense: u8,
    pub dies_at_end_of_controller_turn: bool,
    pub discard_spell_to_damage_random_other_unit_here: Option<u8>,
    pub end_turn_stealth: Option<EndTurnStealth>,
    pub gains_power_ranged_and_spellcaster_atop_tower: bool,
    pub genesis: Option<MinionGenesis>,
    pub immobile: bool,
    pub lance_count: Option<u8>,
    pub lethal: bool,
    pub mana_cost: u64,
    pub may_ranged_strike_once_during_basic_movement: bool,
    pub may_step_after_ranged_strike: bool,
    pub mortal: bool,
    pub movement_bonus: Option<u8>,
    pub movement_restriction: Option<BasicMovementRestriction>,
    pub must_be_cast_to_outer_column: bool,
    pub must_be_cast_to_water_site: bool,
    pub nearby_enemies_permanently_lose_stealth: bool,
    pub occupies_square_area_two: bool,
    pub ordinary: bool,
    pub other_controlled_mortals_power_bonus: bool,
    pub other_nearby_allies_power_bonus: bool,
    pub provides: Option<Element>,
    pub ranged: bool,
    pub required_cast_region: Option<RequiredCastRegion>,
    pub shoots_drag_projectile: bool,
    pub site_provides_no_threshold: bool,
    pub spellcaster: bool,
    pub stealth: bool,
    pub strikes_first_while_attacking: bool,
    pub submerge: bool,
    pub summon_to_any_site: bool,
    pub tap_for_mana: Option<u8>,
    pub tap_to_damage_each_unit_at_adjacent_location: bool,
    pub tap_to_shoot_projectile_damage: Option<u8>,
    pub thresholds: Thresholds,
    pub token: bool,
    pub untaps_at_end_of_controller_turn: bool,
    pub voidwalk: bool,
    pub waterbound: bool,
}

/// A rejected card-fact definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactError {
    path: String,
    problem: String,
}

impl FactError {
    fn new(path: impl Into<String>, problem: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            problem: problem.into(),
        }
    }

    /// Returns the rejected input path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the reason the input was rejected.
    #[must_use]
    pub fn problem(&self) -> &str {
        &self.problem
    }
}

impl Display for FactError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.path, self.problem)
    }
}

impl Error for FactError {}

fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

/// Validates an engine identifier using the TypeScript contract's UTF-16 and
/// ECMAScript-whitespace rules.
///
/// # Errors
///
/// Returns [`FactError`] when the identifier is blank or exceeds 256 UTF-16 units.
pub fn validate_identifier(value: &str, path: &str) -> Result<(), FactError> {
    if value.chars().all(is_ecmascript_whitespace) || value.encode_utf16().count() > 256 {
        return Err(FactError::new(path, "must be 1-256 UTF-16 code units"));
    }
    Ok(())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, FactError> {
    value
        .as_object()
        .ok_or_else(|| FactError::new(path, "must be an object"))
}

fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), FactError> {
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(FactError::new(format!("{path}.{field}"), "is unsupported"));
    }
    Ok(())
}

fn required_bool(object: &Map<String, Value>, field: &str, path: &str) -> Result<bool, FactError> {
    object
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| FactError::new(format!("{path}.{field}"), "must be boolean"))
}

fn optional_bool(object: &Map<String, Value>, field: &str, path: &str) -> Result<bool, FactError> {
    object.get(field).map_or(Ok(false), |value| {
        value
            .as_bool()
            .ok_or_else(|| FactError::new(format!("{path}.{field}"), "must be boolean"))
    })
}

fn true_only(object: &Map<String, Value>, field: &str, path: &str) -> Result<bool, FactError> {
    match object.get(field) {
        None => Ok(false),
        Some(Value::Bool(true)) => Ok(true),
        Some(_) => Err(FactError::new(
            format!("{path}.{field}"),
            "must be true when defined",
        )),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the finite integral JS-safe bounds make the f64-to-i64 conversion exact"
)]
fn safe_integer(value: &Value) -> Option<i64> {
    if let Some(value) = value.as_i64() {
        return (value.unsigned_abs() <= MAX_SAFE_INTEGER).then_some(value);
    }
    if let Some(value) = value.as_u64() {
        return (value <= MAX_SAFE_INTEGER)
            .then(|| i64::try_from(value).ok())
            .flatten();
    }
    value.as_f64().and_then(|value| {
        (value.is_finite() && value.fract() == 0.0 && value.abs() <= MAX_SAFE_INTEGER_F64)
            .then_some(value as i64)
    })
}

fn compact_u8(value: u64) -> u8 {
    match u8::try_from(value) {
        Ok(value) => value,
        Err(_) => unreachable!("value was already bounded to u8"),
    }
}

fn required_nonnegative_integer(
    object: &Map<String, Value>,
    field: &str,
    maximum: u64,
    path: &str,
) -> Result<u64, FactError> {
    let value = object.get(field).and_then(safe_integer).ok_or_else(|| {
        FactError::new(
            format!("{path}.{field}"),
            "must be a supported safe integer",
        )
    })?;
    let value = u64::try_from(value).map_err(|_| {
        FactError::new(
            format!("{path}.{field}"),
            format!("must be between 0 and {maximum}"),
        )
    })?;
    if value > maximum {
        return Err(FactError::new(
            format!("{path}.{field}"),
            format!("must be between 0 and {maximum}"),
        ));
    }
    Ok(value)
}

fn optional_bounded_integer(
    object: &Map<String, Value>,
    field: &str,
    minimum: u64,
    maximum: u64,
    path: &str,
) -> Result<Option<u64>, FactError> {
    let Some(raw) = object.get(field) else {
        return Ok(None);
    };
    let Some(value) = safe_integer(raw) else {
        return Err(FactError::new(
            format!("{path}.{field}"),
            "must be a supported safe integer",
        ));
    };
    let value = u64::try_from(value).map_err(|_| {
        FactError::new(
            format!("{path}.{field}"),
            format!("must be between {minimum} and {maximum}"),
        )
    })?;
    if !(minimum..=maximum).contains(&value) {
        return Err(FactError::new(
            format!("{path}.{field}"),
            format!("must be between {minimum} and {maximum}"),
        ));
    }
    Ok(Some(value))
}

fn fixed_integer(
    object: &Map<String, Value>,
    field: &str,
    expected: u64,
    path: &str,
) -> Result<bool, FactError> {
    match object.get(field) {
        None => Ok(false),
        Some(value) if safe_integer(value) == i64::try_from(expected).ok() => Ok(true),
        Some(_) => Err(FactError::new(
            format!("{path}.{field}"),
            format!("must be {expected}"),
        )),
    }
}

fn parse_thresholds(object: &Map<String, Value>, path: &str) -> Result<Thresholds, FactError> {
    let threshold_path = format!("{path}.thresholds");
    let thresholds = object
        .get("thresholds")
        .and_then(Value::as_object)
        .ok_or_else(|| FactError::new(&threshold_path, "must be an object"))?;
    reject_unknown(thresholds, &ELEMENT_FIELDS, &threshold_path)?;
    let mut values = [0; 4];
    for (index, field) in ELEMENT_FIELDS.into_iter().enumerate() {
        values[index] =
            required_nonnegative_integer(thresholds, field, MAX_SAFE_INTEGER, &threshold_path)?;
    }
    Ok(Thresholds(values))
}

fn parse_elements(object: &Map<String, Value>, path: &str) -> Result<ElementSet, FactError> {
    let field_path = format!("{path}.elements");
    let values = object
        .get("elements")
        .and_then(Value::as_array)
        .ok_or_else(|| FactError::new(&field_path, "must be an array"))?;
    let mut set = ElementSet::default();
    let mut previous = None;
    for value in values {
        let element = value
            .as_str()
            .and_then(Element::parse)
            .ok_or_else(|| FactError::new(&field_path, "contains an unsupported element"))?;
        if previous.is_some_and(|index| element.index() <= index) {
            return Err(FactError::new(
                &field_path,
                "must contain unique elements in canonical order",
            ));
        }
        set.0 |= 1 << element.index();
        previous = Some(element.index());
    }
    Ok(set)
}

fn parse_reference(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
) -> Result<Option<String>, FactError> {
    let Some(value) = object.get(field) else {
        return Ok(None);
    };
    let reference = value
        .as_str()
        .ok_or_else(|| FactError::new(format!("{path}.{field}"), "must be a card ID"))?;
    validate_identifier(reference, &format!("{path}.{field}"))?;
    Ok(Some(reference.to_owned()))
}

fn one_effect<T>(effects: impl IntoIterator<Item = Option<T>>, path: &str) -> Result<T, FactError> {
    let mut effects = effects.into_iter().flatten();
    let Some(effect) = effects.next() else {
        return Err(FactError::new(
            path,
            "must define exactly one supported effect",
        ));
    };
    if effects.next().is_some() {
        return Err(FactError::new(
            path,
            "must define exactly one supported effect",
        ));
    }
    Ok(effect)
}

const AVATAR_FIELDS: &[&str] = &[
    "attack",
    "cardType",
    "defense",
    "drawSpell",
    "earthSitePlayCreatesAdjacentRubble",
    "life",
    "replaceAdjacentRubbleWithTopAtlasSite",
    "tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn",
];

const SITE_FIELDS: &[&str] = &[
    "airborneMinionsAtopMoveFreelyAway",
    "blocksGroundMinionEntryWhileMinionAtop",
    "cannotBeMovedDestroyedOrModified",
    "cardType",
    "connectsBurrowedAllies",
    "elements",
    "flyToNearbyVoidOncePerTurnAtAirThreshold",
    "genesisDiscardTopSpells",
    "genesisDrawSpellPerAdjacentSameCard",
    "genesisEnemiesLoseStealth",
    "genesisGainMana",
    "genesisGainManaIfOnlyControlledCopy",
    "genesisHealNearbyAvatars",
    "genesisImmobilizeNearbyUntilNextTurn",
    "genesisMayBottomNextSpell",
    "genesisPayOneManaToSummonToken",
    "genesisReorderNextSpells",
    "isTower",
    "minionsHereGainVoidwalkUntilLeavingVoid",
    "ordinaryMinionManaDiscount",
    "preventsUnitsWithPowerAtLeastFromEntering",
    "rangedUnitsHereRangeBonus",
    "sacrificeToDestroyNearbySite",
];

const ARTIFACT_FIELDS: &[&str] = &[
    "atEndOfEachTurnSiteControllerLosesLife",
    "bearerControllerChoosesExtraRandomOutcome",
    "cardType",
    "grantsBearerLethal",
    "grantsBearerPower",
    "manaCost",
    "tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps",
    "tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps",
    "tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath",
    "thresholds",
];

const AURA_FIELDS: &[&str] = &[
    "atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep",
    "cardType",
    "immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns",
    "manaCost",
    "thresholds",
];

const MAGIC_FIELDS: &[&str] = &[
    "burrowAllMinionsAndArtifactsAtTargetLandSite",
    "burrowTargetMinionOrArtifact",
    "cardType",
    "damageChainNearbyUnits",
    "damageEachAbovegroundMinion",
    "damageEachUnitAtLocationWithinTwoSteps",
    "damageRandomUnitAtLocation",
    "damageTargetUnit",
    "damageUnitsAboveAndBelowTargetSiteByManhattanDistance",
    "destroyTargetSite",
    "disableTargetNearbyMinionUntilNextTurn",
    "discardSiteAsAdditionalCost",
    "fightAllyWithAdjacentEnemy",
    "gainControlOfTargetNearbyMinion",
    "grantChargeToAllyThisTurn",
    "grantPowerToAllyThisTurn",
    "healController",
    "killTargetWoundedMinion",
    "leapAttackAlly",
    "lureEnemyMinionOneStepCloser",
    "manaCost",
    "returnMinionFromOwnCemetery",
    "submergeTargetMinion",
    "summonRandomMinionFromAnyCemetery",
    "summonTokenToEachControlledSiteBorderingEnemySite",
    "targetNearby",
    "teleportAllyToTargetSite",
    "teleportNearbyAllyThenDrawCard",
    "thresholds",
    "untapTargetMinionAfterDamage",
];

const MINION_FIELDS: &[&str] = &[
    "airborne",
    "atStartOfControllerTurnTeleportToRandomSiteOrVoid",
    "attack",
    "burrowing",
    "cannotAttackSites",
    "cannotDefend",
    "cannotDefendOrIntercept",
    "cardType",
    "charge",
    "connectsTopBottom",
    "deathriteDamageEachUnitHere",
    "deathriteDrawSite",
    "deathriteHeal",
    "deathriteLoseLifePerNearbySiteControlled",
    "defense",
    "diesAtEndOfControllerTurn",
    "discardRandomCardInsteadOfMana",
    "discardSpellToDamageRandomOtherUnitHere",
    "gainsPowerRangedAndSpellcasterAtopTower",
    "gainsStealthAtEndOfTurn",
    "gainsStealthAtEndOfTurnIfNoEnemiesNearby",
    "genesisDamageEachOtherUnitHere",
    "genesisDisableSelfUntilDamaged",
    "genesisDrawSite",
    "genesisDrawSpells",
    "genesisHealController",
    "genesisLoseControllerLife",
    "genesisMayDamageTargetAdjacentUnit",
    "genesisStrikeEachEnemyHere",
    "immobile",
    "lanceCount",
    "lethal",
    "manaCost",
    "mayRangedStrikeOnceDuringBasicMovement",
    "mayStepAfterRangedStrike",
    "mortal",
    "movementBonus",
    "movesOnlyForward",
    "movesOnlySideways",
    "mustBeCastBurrowed",
    "mustBeCastSubmerged",
    "mustBeCastToOuterColumn",
    "mustBeCastToWaterSite",
    "nearbyEnemiesPermanentlyLoseStealth",
    "occupiesSquareArea",
    "ordinary",
    "otherControlledMortalsPowerBonus",
    "otherNearbyAlliesPowerBonus",
    "preventsDamageFromUnitsWithPowerAtLeast",
    "provides",
    "ranged",
    "sacrificeMinionAtSummoningLocationForManaDiscount",
    "shootsDragProjectile",
    "siteProvidesNoThreshold",
    "spellcaster",
    "stealth",
    "strikesFirstWhileAttacking",
    "submerge",
    "summonToAnySite",
    "takesLessDamage",
    "tapForMana",
    "tapToDamageEachUnitAtAdjacentLocation",
    "tapToShootProjectileDamage",
    "thresholds",
    "token",
    "untapsAtEndOfControllerTurn",
    "voidwalk",
    "ward",
    "waterbound",
];

/// Parses one public card definition into a type that cannot retain invalid raw states.
///
/// # Errors
/// Returns [`FactError`] when the card ID, fields, values, or supported fact combination does not
/// match the authoritative contract.
pub fn parse_card_definition(card_id: &str, value: &Value) -> Result<CardFacts, FactError> {
    validate_identifier(card_id, "cardId")?;
    let path = format!("cards.{card_id}");
    let object = object(value, &path)?;
    if object.contains_key("genesisDrawSpell") {
        return Err(FactError::new(
            format!("{path}.genesisDrawSpell"),
            "is obsolete; use genesisDrawSpells",
        ));
    }
    match object.get("cardType").and_then(Value::as_str) {
        Some("avatar") => parse_avatar(object, &path).map(CardFacts::Avatar),
        Some("artifact") => parse_artifact(object, &path).map(CardFacts::Artifact),
        Some("aura") => parse_aura(object, &path).map(CardFacts::Aura),
        Some("magic") => parse_magic(object, &path).map(CardFacts::Magic),
        Some("minion") => parse_minion(object, &path).map(CardFacts::Minion),
        Some("site") => parse_site(object, &path).map(CardFacts::Site),
        _ => Err(FactError::new(format!("{path}.cardType"), "is unsupported")),
    }
}

fn parse_avatar(object: &Map<String, Value>, path: &str) -> Result<AvatarFacts, FactError> {
    reject_unknown(object, AVATAR_FIELDS, path)?;
    Ok(AvatarFacts {
        attack: compact_u8(required_nonnegative_integer(
            object,
            "attack",
            MAX_COMBAT_STAT,
            path,
        )?),
        defense: compact_u8(required_nonnegative_integer(
            object,
            "defense",
            MAX_COMBAT_STAT,
            path,
        )?),
        draw_spell: required_bool(object, "drawSpell", path)?,
        earth_site_play_creates_adjacent_rubble: true_only(
            object,
            "earthSitePlayCreatesAdjacentRubble",
            path,
        )?,
        life: compact_u8(
            optional_bounded_integer(object, "life", 1, MAX_COMBAT_STAT, path)?
                .ok_or_else(|| FactError::new(format!("{path}.life"), "is required"))?,
        ),
        replace_adjacent_rubble_with_top_atlas_site: true_only(
            object,
            "replaceAdjacentRubbleWithTopAtlasSite",
            path,
        )?,
        tap_damage_random_other_unit_at_nearby_location_per_air_threshold_cast_this_turn:
            true_only(
                object,
                "tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn",
                path,
            )?,
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "the flat site schema mirrors one external validation contract"
)]
fn parse_site(object: &Map<String, Value>, path: &str) -> Result<SiteFacts, FactError> {
    reject_unknown(object, SITE_FIELDS, path)?;
    let genesis_gain_mana =
        optional_bounded_integer(object, "genesisGainMana", 1, MAX_COMBAT_STAT, path)?
            .map(compact_u8);
    let genesis_gain_mana_if_only_controlled_copy =
        fixed_integer(object, "genesisGainManaIfOnlyControlledCopy", 1, path)?;
    if genesis_gain_mana.is_some() && genesis_gain_mana_if_only_controlled_copy {
        return Err(FactError::new(
            path,
            "simultaneous unconditional and conditional Genesis mana are unsupported",
        ));
    }

    let genesis_discard_top_spells = fixed_integer(object, "genesisDiscardTopSpells", 2, path)?;
    let genesis_draw_spell_per_adjacent_same_card =
        optional_bool(object, "genesisDrawSpellPerAdjacentSameCard", path)?;
    if genesis_discard_top_spells && genesis_draw_spell_per_adjacent_same_card {
        return Err(FactError::new(
            path,
            "simultaneous Genesis spell discard and draw are unsupported",
        ));
    }

    let genesis_enemies_lose_stealth = true_only(object, "genesisEnemiesLoseStealth", path)?;
    let genesis_heal_nearby_avatars = fixed_integer(object, "genesisHealNearbyAvatars", 3, path)?;
    let genesis_immobilize_nearby_until_next_turn =
        true_only(object, "genesisImmobilizeNearbyUntilNextTurn", path)?;
    let genesis_may_bottom_next_spell = true_only(object, "genesisMayBottomNextSpell", path)?;
    let genesis_pay_one_mana_to_summon_token =
        parse_reference(object, "genesisPayOneManaToSummonToken", path)?;
    let genesis_reorder_next_spells = fixed_integer(object, "genesisReorderNextSpells", 3, path)?;
    let other_genesis = genesis_discard_top_spells
        || genesis_draw_spell_per_adjacent_same_card
        || genesis_enemies_lose_stealth
        || genesis_gain_mana.is_some()
        || genesis_gain_mana_if_only_controlled_copy
        || genesis_heal_nearby_avatars
        || genesis_immobilize_nearby_until_next_turn;
    if genesis_pay_one_mana_to_summon_token.is_some()
        && (other_genesis || genesis_may_bottom_next_spell || genesis_reorder_next_spells)
    {
        return Err(FactError::new(
            path,
            "simultaneous paid-token and another site Genesis are unsupported",
        ));
    }
    if genesis_may_bottom_next_spell
        && (other_genesis
            || genesis_pay_one_mana_to_summon_token.is_some()
            || genesis_reorder_next_spells)
    {
        return Err(FactError::new(
            path,
            "simultaneous next-spell and another site Genesis are unsupported",
        ));
    }
    if genesis_reorder_next_spells
        && (other_genesis
            || genesis_may_bottom_next_spell
            || genesis_pay_one_mana_to_summon_token.is_some())
    {
        return Err(FactError::new(
            path,
            "simultaneous spell-order and another site Genesis are unsupported",
        ));
    }

    Ok(SiteFacts {
        airborne_minions_atop_move_freely_away: true_only(
            object,
            "airborneMinionsAtopMoveFreelyAway",
            path,
        )?,
        blocks_ground_minion_entry_while_minion_atop: true_only(
            object,
            "blocksGroundMinionEntryWhileMinionAtop",
            path,
        )?,
        cannot_be_moved_destroyed_or_modified: true_only(
            object,
            "cannotBeMovedDestroyedOrModified",
            path,
        )?,
        connects_burrowed_allies: optional_bool(object, "connectsBurrowedAllies", path)?,
        elements: parse_elements(object, path)?,
        fly_to_nearby_void_once_per_turn_at_air_threshold: fixed_integer(
            object,
            "flyToNearbyVoidOncePerTurnAtAirThreshold",
            3,
            path,
        )?,
        genesis_discard_top_spells,
        genesis_draw_spell_per_adjacent_same_card,
        genesis_enemies_lose_stealth,
        genesis_gain_mana,
        genesis_gain_mana_if_only_controlled_copy,
        genesis_heal_nearby_avatars,
        genesis_immobilize_nearby_until_next_turn,
        genesis_may_bottom_next_spell,
        genesis_pay_one_mana_to_summon_token,
        genesis_reorder_next_spells,
        is_tower: true_only(object, "isTower", path)?,
        minions_here_gain_voidwalk_until_leaving_void: true_only(
            object,
            "minionsHereGainVoidwalkUntilLeavingVoid",
            path,
        )?,
        ordinary_minion_mana_discount: fixed_integer(
            object,
            "ordinaryMinionManaDiscount",
            1,
            path,
        )?,
        prevents_units_with_power_at_least_from_entering: optional_bounded_integer(
            object,
            "preventsUnitsWithPowerAtLeastFromEntering",
            1,
            MAX_COMBAT_STAT,
            path,
        )?
        .map(compact_u8),
        ranged_units_here_range_bonus: fixed_integer(object, "rangedUnitsHereRangeBonus", 1, path)?,
        sacrifice_to_destroy_nearby_site: true_only(object, "sacrificeToDestroyNearbySite", path)?,
    })
}

fn parse_artifact(object: &Map<String, Value>, path: &str) -> Result<ArtifactFacts, FactError> {
    reject_unknown(object, ARTIFACT_FIELDS, path)?;
    let life_loss = optional_bounded_integer(
        object,
        "atEndOfEachTurnSiteControllerLosesLife",
        1,
        MAX_COMBAT_STAT,
        path,
    )?
    .map(|value| ArtifactEffect::AtEndOfEachTurnSiteControllerLosesLife(compact_u8(value)));
    let effect = one_effect(
        [
            life_loss,
            true_only(
                object,
                "bearerControllerChoosesExtraRandomOutcome",
                path,
            )?
            .then_some(ArtifactEffect::BearerControllerChoosesExtraRandomOutcome),
            true_only(object, "grantsBearerLethal", path)?
                .then_some(ArtifactEffect::GrantsBearerLethal),
            fixed_integer(object, "grantsBearerPower", 2, path)?
                .then_some(ArtifactEffect::GrantsBearerPowerTwo),
            true_only(
                object,
                "tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps",
                path,
            )?
            .then_some(
                ArtifactEffect::TapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps,
            ),
            fixed_integer(
                object,
                "tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps",
                3,
                path,
            )?
            .then_some(
                ArtifactEffect::TapBearerAndAnotherAllyHereToDamageTargetWithinTwoStepsThree,
            ),
            fixed_integer(
                object,
                "tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath",
                4,
                path,
            )?
            .then_some(
                ArtifactEffect::TapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPathFour,
            ),
        ],
        path,
    )?;
    Ok(ArtifactFacts {
        effect,
        mana_cost: required_nonnegative_integer(object, "manaCost", MAX_SAFE_INTEGER, path)?,
        thresholds: parse_thresholds(object, path)?,
    })
}

fn parse_aura(object: &Map<String, Value>, path: &str) -> Result<AuraFacts, FactError> {
    reject_unknown(object, AURA_FIELDS, path)?;
    let effect = one_effect(
        [
            fixed_integer(
                object,
                "atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep",
                3,
                path,
            )?
            .then_some(
                AuraEffect::AtEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStepThree,
            ),
            true_only(
                object,
                "immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns",
                path,
            )?
            .then_some(
                AuraEffect::ImmobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns,
            ),
        ],
        path,
    )?;
    Ok(AuraFacts {
        effect,
        mana_cost: required_nonnegative_integer(object, "manaCost", MAX_SAFE_INTEGER, path)?,
        thresholds: parse_thresholds(object, path)?,
    })
}

fn parse_damage_grid(
    object: &Map<String, Value>,
    path: &str,
) -> Result<Option<[u8; 5]>, FactError> {
    let field = "damageUnitsAboveAndBelowTargetSiteByManhattanDistance";
    let Some(value) = object.get(field) else {
        return Ok(None);
    };
    let values = value.as_array().ok_or_else(|| {
        FactError::new(
            format!("{path}.{field}"),
            "must contain five supported positive damage values",
        )
    })?;
    if values.len() != 5 {
        return Err(FactError::new(
            format!("{path}.{field}"),
            "must contain five supported positive damage values",
        ));
    }
    let mut damage = [0; 5];
    for (index, value) in values.iter().enumerate() {
        let Some(amount) = safe_integer(value) else {
            return Err(FactError::new(
                format!("{path}.{field}"),
                "must contain five supported positive damage values",
            ));
        };
        if !(1..=100).contains(&amount) {
            return Err(FactError::new(
                format!("{path}.{field}"),
                "must contain five supported positive damage values",
            ));
        }
        damage[index] = match u8::try_from(amount) {
            Ok(amount) => amount,
            Err(_) => unreachable!("damage was already bounded to u8"),
        };
    }
    Ok(Some(damage))
}

#[expect(
    clippy::too_many_lines,
    reason = "the effect list intentionally mirrors the fail-closed Magic contract"
)]
fn parse_magic(object: &Map<String, Value>, path: &str) -> Result<MagicFacts, FactError> {
    reject_unknown(object, MAGIC_FIELDS, path)?;
    let target_nearby = optional_bool(object, "targetNearby", path)?;
    let untap_target_minion_after_damage = true_only(object, "untapTargetMinionAfterDamage", path)?;
    let damage_target =
        optional_bounded_integer(object, "damageTargetUnit", 1, MAX_COMBAT_STAT, path)?;
    if object.contains_key("targetNearby") && damage_target.is_none() {
        return Err(FactError::new(
            format!("{path}.targetNearby"),
            "requires damageTargetUnit",
        ));
    }
    if untap_target_minion_after_damage && damage_target.is_none() {
        return Err(FactError::new(
            format!("{path}.untapTargetMinionAfterDamage"),
            "requires damageTargetUnit",
        ));
    }

    let discard_site = true_only(object, "discardSiteAsAdditionalCost", path)?;
    let destroy_site = true_only(object, "destroyTargetSite", path)?;
    let damage_grid = parse_damage_grid(object, path)?;
    let target_site_fact_count =
        u8::from(discard_site) + u8::from(destroy_site) + u8::from(damage_grid.is_some());
    if target_site_fact_count != 0 && target_site_fact_count != 3 {
        return Err(FactError::new(
            path,
            "site-destruction grid damage facts must be defined together",
        ));
    }

    let token_reference = parse_reference(
        object,
        "summonTokenToEachControlledSiteBorderingEnemySite",
        path,
    )?;
    let effect = one_effect(
        [
            true_only(object, "burrowAllMinionsAndArtifactsAtTargetLandSite", path)?
                .then_some(MagicEffect::BurrowAllMinionsAndArtifactsAtTargetLandSite),
            true_only(object, "burrowTargetMinionOrArtifact", path)?
                .then_some(MagicEffect::BurrowTargetMinionOrArtifact),
            true_only(object, "damageChainNearbyUnits", path)?
                .then_some(MagicEffect::DamageChainNearbyUnits),
            fixed_integer(object, "damageEachAbovegroundMinion", 1, path)?
                .then_some(MagicEffect::DamageEachAbovegroundMinionOne),
            optional_bounded_integer(
                object,
                "damageEachUnitAtLocationWithinTwoSteps",
                1,
                MAX_COMBAT_STAT,
                path,
            )?
            .map(|amount| MagicEffect::DamageEachUnitAtLocationWithinTwoSteps(compact_u8(amount))),
            optional_bounded_integer(
                object,
                "damageRandomUnitAtLocation",
                1,
                MAX_COMBAT_STAT,
                path,
            )?
            .map(|amount| MagicEffect::DamageRandomUnitAtLocation(compact_u8(amount))),
            damage_target.map(|amount| MagicEffect::DamageTargetUnit {
                amount: compact_u8(amount),
                target_nearby,
                untap_target_minion_after_damage,
            }),
            damage_grid.map(MagicEffect::DestroyTargetSiteWithDamageGrid),
            true_only(object, "disableTargetNearbyMinionUntilNextTurn", path)?
                .then_some(MagicEffect::DisableTargetNearbyMinionUntilNextTurn),
            true_only(object, "fightAllyWithAdjacentEnemy", path)?
                .then_some(MagicEffect::FightAllyWithAdjacentEnemy),
            true_only(object, "gainControlOfTargetNearbyMinion", path)?
                .then_some(MagicEffect::GainControlOfTargetNearbyMinion),
            true_only(object, "grantChargeToAllyThisTurn", path)?
                .then_some(MagicEffect::GrantChargeToAllyThisTurn),
            fixed_integer(object, "grantPowerToAllyThisTurn", 2, path)?
                .then_some(MagicEffect::GrantPowerTwoToAllyThisTurn),
            optional_bounded_integer(object, "healController", 1, MAX_COMBAT_STAT, path)?
                .map(|amount| MagicEffect::HealController(compact_u8(amount))),
            true_only(object, "killTargetWoundedMinion", path)?
                .then_some(MagicEffect::KillTargetWoundedMinion),
            true_only(object, "leapAttackAlly", path)?.then_some(MagicEffect::LeapAttackAlly),
            true_only(object, "lureEnemyMinionOneStepCloser", path)?
                .then_some(MagicEffect::LureEnemyMinionOneStepCloser),
            true_only(object, "returnMinionFromOwnCemetery", path)?
                .then_some(MagicEffect::ReturnMinionFromOwnCemetery),
            true_only(object, "submergeTargetMinion", path)?
                .then_some(MagicEffect::SubmergeTargetMinion),
            true_only(object, "summonRandomMinionFromAnyCemetery", path)?
                .then_some(MagicEffect::SummonRandomMinionFromAnyCemetery),
            token_reference.map(MagicEffect::SummonTokenToEachControlledSiteBorderingEnemySite),
            true_only(object, "teleportAllyToTargetSite", path)?
                .then_some(MagicEffect::TeleportAllyToTargetSite),
            true_only(object, "teleportNearbyAllyThenDrawCard", path)?
                .then_some(MagicEffect::TeleportNearbyAllyThenDrawCard),
        ],
        path,
    )?;
    Ok(MagicFacts {
        effect,
        mana_cost: required_nonnegative_integer(object, "manaCost", MAX_SAFE_INTEGER, path)?,
        thresholds: parse_thresholds(object, path)?,
    })
}

fn at_most_one<T>(
    effects: impl IntoIterator<Item = Option<T>>,
    path: &str,
    problem: &str,
) -> Result<Option<T>, FactError> {
    let mut effects = effects.into_iter().flatten();
    let effect = effects.next();
    if effects.next().is_some() {
        return Err(FactError::new(path, problem));
    }
    Ok(effect)
}

fn parse_minion_genesis(
    object: &Map<String, Value>,
    path: &str,
) -> Result<Option<MinionGenesis>, FactError> {
    at_most_one(
        [
            fixed_integer(object, "genesisDamageEachOtherUnitHere", 1, path)?
                .then_some(MinionGenesis::DamageEachOtherUnitHereOne),
            true_only(object, "genesisDisableSelfUntilDamaged", path)?
                .then_some(MinionGenesis::DisableSelfUntilDamaged),
            optional_bool(object, "genesisDrawSite", path)?.then_some(MinionGenesis::DrawSite),
            optional_bounded_integer(object, "genesisDrawSpells", 1, MAX_DECK_CARDS, path)?
                .map(|count| MinionGenesis::DrawSpells(compact_u8(count))),
            fixed_integer(object, "genesisHealController", 2, path)?
                .then_some(MinionGenesis::HealControllerTwo),
            fixed_integer(object, "genesisLoseControllerLife", 2, path)?
                .then_some(MinionGenesis::LoseControllerLifeTwo),
            fixed_integer(object, "genesisMayDamageTargetAdjacentUnit", 2, path)?
                .then_some(MinionGenesis::MayDamageTargetAdjacentUnitTwo),
            true_only(object, "genesisStrikeEachEnemyHere", path)?
                .then_some(MinionGenesis::StrikeEachEnemyHere),
        ],
        path,
        "simultaneous Genesis effects are unsupported",
    )
}

fn parse_alternative_summon_payment(
    object: &Map<String, Value>,
    path: &str,
) -> Result<Option<AlternativeSummonPayment>, FactError> {
    at_most_one(
        [
            true_only(object, "discardRandomCardInsteadOfMana", path)?
                .then_some(AlternativeSummonPayment::DiscardRandomCardInsteadOfMana),
            fixed_integer(
                object,
                "sacrificeMinionAtSummoningLocationForManaDiscount",
                2,
                path,
            )?
            .then_some(
                AlternativeSummonPayment::SacrificeMinionAtSummoningLocationForManaDiscountTwo,
            ),
        ],
        path,
        "competing alternative summon payments are unsupported",
    )
}

fn parse_end_turn_stealth(
    object: &Map<String, Value>,
    path: &str,
) -> Result<Option<EndTurnStealth>, FactError> {
    at_most_one(
        [
            optional_bool(object, "gainsStealthAtEndOfTurn", path)?
                .then_some(EndTurnStealth::Always),
            optional_bool(object, "gainsStealthAtEndOfTurnIfNoEnemiesNearby", path)?
                .then_some(EndTurnStealth::IfNoEnemiesNearby),
        ],
        path,
        "simultaneous unconditional and conditional end-turn Stealth are unsupported",
    )
}

fn parse_movement_restriction(
    object: &Map<String, Value>,
    path: &str,
) -> Result<Option<BasicMovementRestriction>, FactError> {
    at_most_one(
        [
            optional_bool(object, "movesOnlyForward", path)?
                .then_some(BasicMovementRestriction::ForwardOnly),
            optional_bool(object, "movesOnlySideways", path)?
                .then_some(BasicMovementRestriction::SidewaysOnly),
        ],
        path,
        "cannot move only forward and only sideways",
    )
}

fn parse_required_cast_region(
    object: &Map<String, Value>,
    path: &str,
    burrowing: bool,
    submerge: bool,
) -> Result<Option<RequiredCastRegion>, FactError> {
    let underground = optional_bool(object, "mustBeCastBurrowed", path)?;
    let underwater = optional_bool(object, "mustBeCastSubmerged", path)?;
    if underground && !burrowing {
        return Err(FactError::new(
            format!("{path}.mustBeCastBurrowed"),
            "requires Burrowing",
        ));
    }
    if underwater && !submerge {
        return Err(FactError::new(
            format!("{path}.mustBeCastSubmerged"),
            "requires Submerge",
        ));
    }
    at_most_one(
        [
            underground.then_some(RequiredCastRegion::Underground),
            underwater.then_some(RequiredCastRegion::Underwater),
        ],
        path,
        "cannot require both burrowed and submerged casting",
    )
}

fn parse_damage_prevention(
    object: &Map<String, Value>,
    path: &str,
) -> Result<Option<DamagePrevention>, FactError> {
    at_most_one(
        [
            fixed_integer(object, "takesLessDamage", 1, path)?
                .then_some(DamagePrevention::TakesOneLessDamage),
            optional_bounded_integer(
                object,
                "preventsDamageFromUnitsWithPowerAtLeast",
                1,
                MAX_COMBAT_STAT,
                path,
            )?
            .map(|power| {
                DamagePrevention::PreventsDamageFromUnitsWithPowerAtLeast(compact_u8(power))
            }),
            optional_bool(object, "ward", path)?.then_some(DamagePrevention::Ward),
        ],
        path,
        "competing damage prevention effects are unsupported",
    )
}

fn parse_provides(object: &Map<String, Value>, path: &str) -> Result<Option<Element>, FactError> {
    let Some(value) = object.get("provides") else {
        return Ok(None);
    };
    value
        .as_str()
        .and_then(Element::parse)
        .map(Some)
        .ok_or_else(|| FactError::new(format!("{path}.provides"), "must be a supported element"))
}

#[expect(
    clippy::too_many_lines,
    reason = "the flat minion schema mirrors one external validation contract"
)]
fn parse_minion(object: &Map<String, Value>, path: &str) -> Result<MinionFacts, FactError> {
    reject_unknown(object, MINION_FIELDS, path)?;
    let airborne = optional_bool(object, "airborne", path)?;
    let alternative_summon_payment = parse_alternative_summon_payment(object, path)?;
    let at_start_of_controller_turn_teleport_to_random_site_or_void = true_only(
        object,
        "atStartOfControllerTurnTeleportToRandomSiteOrVoid",
        path,
    )?;
    let burrowing = optional_bool(object, "burrowing", path)?;
    let connects_top_bottom = optional_bool(object, "connectsTopBottom", path)?;
    let damage_prevention = parse_damage_prevention(object, path)?;
    let end_turn_stealth = parse_end_turn_stealth(object, path)?;
    let gains_power_ranged_and_spellcaster_atop_tower =
        fixed_integer(object, "gainsPowerRangedAndSpellcasterAtopTower", 2, path)?;
    let genesis = parse_minion_genesis(object, path)?;
    let may_ranged_strike_once_during_basic_movement =
        true_only(object, "mayRangedStrikeOnceDuringBasicMovement", path)?;
    let may_step_after_ranged_strike = true_only(object, "mayStepAfterRangedStrike", path)?;
    let movement_restriction = parse_movement_restriction(object, path)?;
    let occupies_square_area_two = fixed_integer(object, "occupiesSquareArea", 2, path)?;
    let ordinary = true_only(object, "ordinary", path)?;
    let ranged = optional_bool(object, "ranged", path)?;
    let shoots_drag_projectile = optional_bool(object, "shootsDragProjectile", path)?;
    let site_provides_no_threshold = true_only(object, "siteProvidesNoThreshold", path)?;
    let spellcaster = optional_bool(object, "spellcaster", path)?;
    let stealth = optional_bool(object, "stealth", path)?;
    let submerge = optional_bool(object, "submerge", path)?;
    let required_cast_region = parse_required_cast_region(object, path, burrowing, submerge)?;
    let summon_to_any_site = optional_bool(object, "summonToAnySite", path)?;
    let must_be_cast_to_outer_column = optional_bool(object, "mustBeCastToOuterColumn", path)?;
    let must_be_cast_to_water_site = optional_bool(object, "mustBeCastToWaterSite", path)?;
    let tap_to_shoot_projectile_damage = optional_bounded_integer(
        object,
        "tapToShootProjectileDamage",
        1,
        MAX_COMBAT_STAT,
        path,
    )?
    .map(compact_u8);
    let token = true_only(object, "token", path)?;
    let voidwalk = optional_bool(object, "voidwalk", path)?;
    let waterbound = optional_bool(object, "waterbound", path)?;

    if matches!(genesis, Some(MinionGenesis::DisableSelfUntilDamaged)) && stealth {
        return Err(FactError::new(
            path,
            "Genesis disable with Stealth is unsupported",
        ));
    }
    if matches!(genesis, Some(MinionGenesis::MayDamageTargetAdjacentUnitTwo))
        && alternative_summon_payment.is_some()
    {
        return Err(FactError::new(
            path,
            "targeted Genesis with alternative summon payment is unsupported",
        ));
    }
    if may_ranged_strike_once_during_basic_movement && !ranged {
        return Err(FactError::new(
            format!("{path}.mayRangedStrikeOnceDuringBasicMovement"),
            "requires ranged",
        ));
    }
    if may_ranged_strike_once_during_basic_movement && may_step_after_ranged_strike {
        return Err(FactError::new(
            path,
            "simultaneous during-movement and post-Ranged movement is unsupported",
        ));
    }
    if at_start_of_controller_turn_teleport_to_random_site_or_void && !voidwalk {
        return Err(FactError::new(
            format!("{path}.atStartOfControllerTurnTeleportToRandomSiteOrVoid"),
            "requires voidwalk",
        ));
    }
    if at_start_of_controller_turn_teleport_to_random_site_or_void && occupies_square_area_two {
        return Err(FactError::new(
            path,
            "oversized start-turn random teleport is unsupported",
        ));
    }

    let deathrite_damage_each_unit_here = optional_bounded_integer(
        object,
        "deathriteDamageEachUnitHere",
        1,
        MAX_COMBAT_STAT,
        path,
    )?
    .map(compact_u8);
    let discard_spell_to_damage_random_other_unit_here = optional_bounded_integer(
        object,
        "discardSpellToDamageRandomOtherUnitHere",
        1,
        MAX_COMBAT_STAT,
        path,
    )?
    .map(compact_u8);
    if occupies_square_area_two
        && (ordinary
            || connects_top_bottom
            || matches!(
                alternative_summon_payment,
                Some(
                    AlternativeSummonPayment::SacrificeMinionAtSummoningLocationForManaDiscountTwo
                )
            )
            || required_cast_region.is_some()
            || must_be_cast_to_water_site
            || burrowing
            || submerge
            || voidwalk
            || waterbound
            || ranged
            || tap_to_shoot_projectile_damage.is_some()
            || shoots_drag_projectile
            || site_provides_no_threshold
            || gains_power_ranged_and_spellcaster_atop_tower
            || must_be_cast_to_outer_column
            || token)
    {
        return Err(FactError::new(
            format!("{path}.occupiesSquareArea"),
            "has an unsupported ability combination",
        ));
    }
    if token && genesis.is_some() {
        return Err(FactError::new(
            path,
            "token Genesis effects are unsupported",
        ));
    }

    Ok(MinionFacts {
        airborne,
        alternative_summon_payment,
        at_start_of_controller_turn_teleport_to_random_site_or_void,
        attack: compact_u8(required_nonnegative_integer(
            object,
            "attack",
            MAX_COMBAT_STAT,
            path,
        )?),
        burrowing,
        cannot_attack_sites: optional_bool(object, "cannotAttackSites", path)?,
        cannot_defend: optional_bool(object, "cannotDefend", path)?,
        cannot_defend_or_intercept: optional_bool(object, "cannotDefendOrIntercept", path)?,
        charge: optional_bool(object, "charge", path)?,
        connects_top_bottom,
        damage_prevention,
        deathrite_damage_each_unit_here,
        deathrite_draw_site: optional_bool(object, "deathriteDrawSite", path)?,
        deathrite_heal: optional_bounded_integer(
            object,
            "deathriteHeal",
            1,
            MAX_COMBAT_STAT,
            path,
        )?
        .map(compact_u8),
        deathrite_lose_life_per_nearby_site_controlled: fixed_integer(
            object,
            "deathriteLoseLifePerNearbySiteControlled",
            1,
            path,
        )?,
        defense: compact_u8(required_nonnegative_integer(
            object,
            "defense",
            MAX_COMBAT_STAT,
            path,
        )?),
        dies_at_end_of_controller_turn: true_only(object, "diesAtEndOfControllerTurn", path)?,
        discard_spell_to_damage_random_other_unit_here,
        end_turn_stealth,
        gains_power_ranged_and_spellcaster_atop_tower,
        genesis,
        immobile: optional_bool(object, "immobile", path)?,
        lance_count: optional_bounded_integer(object, "lanceCount", 1, 3, path)?.map(compact_u8),
        lethal: optional_bool(object, "lethal", path)?,
        mana_cost: required_nonnegative_integer(object, "manaCost", MAX_SAFE_INTEGER, path)?,
        may_ranged_strike_once_during_basic_movement,
        may_step_after_ranged_strike,
        mortal: true_only(object, "mortal", path)?,
        movement_bonus: optional_bounded_integer(object, "movementBonus", 1, 2, path)?
            .map(compact_u8),
        movement_restriction,
        must_be_cast_to_outer_column,
        must_be_cast_to_water_site,
        nearby_enemies_permanently_lose_stealth: true_only(
            object,
            "nearbyEnemiesPermanentlyLoseStealth",
            path,
        )?,
        occupies_square_area_two,
        ordinary,
        other_controlled_mortals_power_bonus: fixed_integer(
            object,
            "otherControlledMortalsPowerBonus",
            1,
            path,
        )?,
        other_nearby_allies_power_bonus: fixed_integer(
            object,
            "otherNearbyAlliesPowerBonus",
            1,
            path,
        )?,
        provides: parse_provides(object, path)?,
        ranged,
        required_cast_region,
        shoots_drag_projectile,
        site_provides_no_threshold,
        spellcaster,
        stealth,
        strikes_first_while_attacking: optional_bool(object, "strikesFirstWhileAttacking", path)?,
        submerge,
        summon_to_any_site,
        tap_for_mana: optional_bounded_integer(object, "tapForMana", 1, MAX_COMBAT_STAT, path)?
            .map(compact_u8),
        tap_to_damage_each_unit_at_adjacent_location: fixed_integer(
            object,
            "tapToDamageEachUnitAtAdjacentLocation",
            2,
            path,
        )?,
        tap_to_shoot_projectile_damage,
        thresholds: parse_thresholds(object, path)?,
        token,
        untaps_at_end_of_controller_turn: true_only(object, "untapsAtEndOfControllerTurn", path)?,
        voidwalk,
        waterbound,
    })
}
