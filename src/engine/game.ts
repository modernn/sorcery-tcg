import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  deepFreeze,
  type EngineActionRequest,
  type EngineAttempt,
  type EngineLegalAction,
  type EngineRandomDraw,
  type EngineReceipt,
  type EngineRejection,
  type EngineSeat,
  type StateHash,
} from './contract.ts';
import { createEngineState, type EngineState } from './determinism.ts';
import {
  createRustGameSession,
  rustLegalGameActions,
  rustObserveGame,
  rustReplayGame,
  rustStepGame,
  rustVerifyGameReplay,
} from './rust-legality-sync.ts';

const MAX_DECK_CARDS = 200;
const MAX_COMBAT_STAT = 100;

export type GameSeat = EngineSeat;
export type DeckZone = 'atlas' | 'spellbook';
export type RealmCell = `${'A' | 'B' | 'C' | 'D' | 'E'}${1 | 2 | 3 | 4}`;
export type GameElement = 'air' | 'earth' | 'fire' | 'water';
export type GameThresholds = Readonly<Record<GameElement, number>>;
export type GameRegion = 'surface' | 'underground' | 'underwater' | 'void';
export type TwoByTwoArea = readonly [RealmCell, RealmCell, RealmCell, RealmCell];

export type GameCardDefinition =
  | Readonly<{
    attack: number;
    cardType: 'avatar';
    defense: number;
    drawSpell: boolean;
    earthSitePlayCreatesAdjacentRubble?: true;
    life: number;
    replaceAdjacentRubbleWithTopAtlasSite?: true;
    tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn?: true;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower: 2;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: never;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal: true;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: never;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: never;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 3;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: never;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps: true;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: never;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 4;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife: number;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: never;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome: true;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: never;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble: true;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: true;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage: true;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn: number;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    nearbyMinionsMustAttackIfAble?: never;
    nearbyStrikesAgainstUnitsDealDoubleDamage?: never;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    airborneMinionsAtopMoveFreelyAway?: true;
    blocksGroundMinionEntryWhileMinionAtop?: true;
    cannotBeMovedDestroyedOrModified?: true;
    cardType: 'site';
    connectsBurrowedAllies?: boolean;
    elements: readonly GameElement[];
    flyToNearbyVoidOncePerTurnAtAirThreshold?: 3;
    genesisDiscardTopSpells?: 2;
    genesisDrawSpellPerAdjacentSameCard?: boolean;
    genesisEnemiesLoseStealth?: true;
    genesisGainMana?: number;
    genesisGainManaIfOnlyControlledCopy?: 1;
    genesisHealNearbyAvatars?: 3;
    genesisImmobilizeNearbyUntilNextTurn?: true;
    genesisMayBottomNextSpell?: true;
    genesisPayOneManaToSummonToken?: string;
    genesisReorderNextSpells?: 3;
    isTower?: true;
    minionsHereGainVoidwalkUntilLeavingVoid?: true;
    ordinaryMinionManaDiscount?: 1;
    preventsUnitsWithPowerAtLeastFromEntering?: number;
    rangedUnitsHereRangeBonus?: 1;
    sacrificeToDestroyNearbySite?: true;
    uniqueOrLegendary?: true;
  }>
  | Readonly<{
    affectedSitesAreFlooded?: never;
    affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold?: never;
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep?: never;
    atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent?: never;
    atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf?: never;
    cardType: 'aura';
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: true;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    affectedSitesAreFlooded?: never;
    affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold?: never;
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep: 3;
    atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent?: never;
    atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf?: never;
    cardType: 'aura';
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns?: never;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    affectedSitesAreFlooded?: never;
    affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold?: never;
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep?: never;
    atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent?: never;
    atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf: true;
    cardType: 'aura';
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns?: never;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    affectedSitesAreFlooded?: never;
    affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold?: never;
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep?: never;
    atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent: 3;
    atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf?: never;
    cardType: 'aura';
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns?: never;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    affectedSitesAreFlooded: true;
    affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold?: never;
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep?: never;
    atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent?: never;
    atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf?: never;
    cardType: 'aura';
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns?: never;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    affectedSitesAreFlooded?: never;
    affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold: true;
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep?: never;
    atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent?: never;
    atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf?: never;
    cardType: 'aura';
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns?: never;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    burrowAllMinionsAndArtifactsAtTargetLandSite?: true;
    burrowTargetMinionOrArtifact?: true;
    cardType: 'magic';
    damageUnitsAboveAndBelowTargetSiteByManhattanDistance?: readonly [number, number, number, number, number];
    damageChainNearbyUnits?: true;
    damageEachAbovegroundMinion?: 1;
    damageEachUnitAtLocationWithinTwoSteps?: number;
    damageRandomUnitAtLocation?: number;
    damageTargetUnit?: number;
    discardCardAsAdditionalCost?: true;
    discardSiteAsAdditionalCost?: true;
    disableTargetNearbyMinionUntilNextTurn?: true;
    drawSites?: number;
    drawSpells?: number;
    fightAllyWithAdjacentEnemy?: true;
    gainControlOfTargetNearbyMinion?: true;
    grantAirborneToAllyThisTurn?: true;
    grantChargeToAllyThisTurn?: true;
    grantFirstStrikeToAllyThisTurn?: true;
    grantLethalToAllyThisTurn?: true;
    grantPowerToAllyThisTurn?: 2;
    grantRangedToAllyThisTurn?: true;
    healController?: number;
    healTargetMinion?: number;
    killTargetMinion?: true;
    killTargetWoundedMinion?: true;
    leapAttackAlly?: true;
    lureEnemyMinionOneStepCloser?: true;
    manaCost: number;
    millSites?: number;
    millSpells?: number;
    targetPlayerDiscardsCards?: number;
    targetPlayerDrawsSites?: number;
    targetPlayerDrawsSpells?: number;
    returnMinionFromOwnCemetery?: true;
    returnTargetArtifactFromOwnCemetery?: true;
    returnTargetAuraFromOwnCemetery?: true;
    returnTargetMagicFromOwnCemetery?: true;
    returnTargetArtifactToOwnerHand?: true;
    returnTargetAuraToOwnerHand?: true;
    returnTargetMinionToOwnerHand?: true;
    returnTargetSiteFromOwnCemetery?: true;
    returnTargetSiteToOwnerHand?: true;
    submergeTargetMinion?: true;
    summonRandomMinionFromAnyCemetery?: true;
    summonTokenToEachControlledSiteBorderingEnemySite?: string;
    destroyTargetArtifact?: true;
    destroyTargetAura?: true;
    destroyTargetSite?: true;
    grantStealthToTargetMinion?: true;
    grantWardToTargetMinion?: true;
    tapTargetMinion?: true;
    targetNearby?: boolean;
    targetPlayerGainsLife?: number;
    targetPlayerLosesLife?: number;
    teleportAllyToTargetSite?: true;
    teleportNearbyAllyThenDrawCard?: true;
    thresholds: GameThresholds;
    untapTargetMinion?: true;
    untapTargetMinionAfterDamage?: true;
  }>
  | Readonly<{
    airborne?: boolean;
    attack: number;
    burrowing?: boolean;
    cardType: 'minion';
    cannotAttackSites?: boolean;
    charge?: boolean;
    cannotDefend?: boolean;
    cannotDefendOrIntercept?: boolean;
    connectsTopBottom?: boolean;
    deathriteDamageEachUnitHere?: number;
    deathriteHeal?: number;
    deathriteDrawSite?: boolean;
    deathriteDrawSpells?: boolean;
    deathriteLoseLifePerNearbySiteControlled?: 1;
    deathriteMillSites?: boolean;
    deathriteMillSpells?: boolean;
    defense: number;
    discardSpellToDamageRandomOtherUnitHere?: number;
    discardRandomCardInsteadOfMana?: true;
    diesAtEndOfControllerTurn?: true;
    enemiesMustAttackThisIfAble?: true;
    genesisDamageEachOtherUnitHere?: 1;
    genesisDisableSelfUntilDamaged?: true;
    genesisStrikeEachEnemyHere?: true;
    genesisMayDamageTargetAdjacentUnit?: 2;
    genesisDrawSpells?: number;
    genesisDrawSite?: boolean;
    genesisHealController?: 2;
    genesisLoseControllerLife?: 2;
    gainsPowerRangedAndSpellcasterAtopTower?: 2;
    gainsStealthAtEndOfTurn?: boolean;
    gainsStealthAtEndOfTurnIfNoEnemiesNearby?: boolean;
    immobile?: boolean;
    lanceCount?: 1 | 2 | 3;
    lethal?: boolean;
    manaCost: number;
    atEndOfControllerTurnControllerGainsLife?: number;
    atEndOfControllerTurnControllerLosesLife?: number;
    atEndOfControllerTurnDamageEachOtherUnitHere?: number;
    atStartOfControllerTurnControllerGainsLife?: number;
    atStartOfControllerTurnControllerGainsMana?: number;
    atStartOfControllerTurnControllerLosesLife?: number;
    atStartOfControllerTurnDamageEachOtherUnitHere?: number;
    atStartOfControllerTurnDrawSites?: number;
    atStartOfControllerTurnDrawSpells?: number;
    atStartOfControllerTurnMillSites?: number;
    atStartOfControllerTurnMillSpells?: number;
    atStartOfControllerTurnLureNearbyEnemyMinion?: true;
    atStartOfControllerTurnTeleportToRandomSiteOrVoid?: true;
    mayRangedStrikeOnceDuringBasicMovement?: true;
    mayStepAfterRangedStrike?: true;
    mortal?: true;
    movementBonus?: 1 | 2;
    movesOnlyForward?: boolean;
    movesOnlySideways?: boolean;
    mustAttackAUnitIfAble?: true;
    mustBeCastBurrowed?: boolean;
    mustBeCastSubmerged?: boolean;
    mustBeCastToWaterSite?: boolean;
    nearbyEnemiesPermanentlyLoseStealth?: true;
    ordinary?: true;
    occupiesSquareArea?: 2;
    otherControlledMortalsPowerBonus?: 1;
    otherNearbyAlliesPowerBonus?: 1;
    preventsDamageFromUnitsWithPowerAtLeast?: number;
    provides?: GameElement;
    ranged?: boolean;
    sacrificeMinionAtSummoningLocationForManaDiscount?: 2;
    tapToShootProjectileDamage?: number;
    shootsDragProjectile?: boolean;
    siteProvidesNoThreshold?: true;
    spellcaster?: boolean;
    stealth?: boolean;
    strikesFirstWhileAttacking?: boolean;
    strikesFirstWhileDefending?: boolean;
    submerge?: boolean;
    summonToAnySite?: boolean;
    mustBeCastToOuterColumn?: boolean;
    tapToDamageEachUnitAtAdjacentLocation?: 2;
    tapForMana?: number;
    takesLessDamage?: 1;
    thresholds: GameThresholds;
    token?: true;
    doesNotUntapDuringControllersStartPhase?: true;
    untapsAtEndOfControllerTurn?: true;
    voidwalk?: boolean;
    waterbound?: boolean;
    ward?: boolean;
  }>;

export type GameDeckSpec = Readonly<{
  atlas: readonly string[];
  avatar: string;
  spellbook: readonly string[];
}>;

export type GameManifestInput = Readonly<{
  authority: Readonly<{
    contentHash: StateHash;
    mode: 'private-local' | 'synthetic';
    revisionId: string;
  }>;
  cards: Readonly<Record<string, GameCardDefinition>>;
  decks: Readonly<Record<GameSeat, GameDeckSpec>>;
  firstSeat: GameSeat;
  seed: number;
}>;

export type GameManifest = Readonly<GameManifestInput & {
  engineVersion: 'sorcery-core-v1';
  manifestId: StateHash;
  schemaVersion: 1;
}>;

type CardInstance = Readonly<{
  cardId: string;
  instanceId: StateHash;
  owner: GameSeat;
  source: 'atlas' | 'avatar' | 'spellbook' | 'token';
}>;

type SiteInstance = Readonly<CardInstance & {
  controller: GameSeat;
  lastFlightTurn?: number;
}>;

type AuraInstance = Readonly<CardInstance & {
  cells: readonly RealmCell[];
  controller: GameSeat;
  turnCounters: number;
  visitedCells?: readonly RealmCell[];
}>;

type RubbleInstance = Readonly<{
  controller: null;
  instanceId: StateHash;
  rubble: true;
}>;

type RealmSiteInstance = SiteInstance | RubbleInstance;

type DisableEffect = Readonly<{
  expiresAtSeat: GameSeat;
  sourceInstanceId: StateHash;
}>;

type ImmobileArea = Readonly<{
  cells: readonly RealmCell[];
  expiresAtSeat?: GameSeat;
  minionsAtSitesOnly?: true;
  sourceInstanceId: StateHash;
  suppressesAirborne?: true;
}>;

type UnitInstance = Readonly<CardInstance & {
  // ponytail: intrinsic Lance marks omit Artifact transfer/drop; promote them to realm Artifacts when a supported card needs it.
  carriedLanceCount?: number;
  controller: GameSeat;
  damage: number;
  disableEffects?: readonly DisableEffect[];
  disabledUntilDamaged?: true;
  lastDroppedArtifactsTurn?: number;
  lastInteractedTurn?: number;
  lastPickedUpArtifactsTurn?: number;
  location: RealmCell;
  occupiedCells?: TwoByTwoArea;
  planarGateVoidwalk?: true;
  region: GameRegion;
  stealthed: boolean;
  summoningSickness: boolean;
  tapped: boolean;
  temporaryAirborneSources?: readonly StateHash[];
  temporaryChargeSources?: readonly StateHash[];
  temporaryFirstStrikeSources?: readonly StateHash[];
  temporaryLethalSources?: readonly StateHash[];
  temporaryPowerSources?: readonly StateHash[];
  temporaryRangedSources?: readonly StateHash[];
  warded: boolean;
}>;

type ArtifactInstance = Readonly<CardInstance & (
  | Readonly<{ bearer: GameUnitRef; bearerCell?: RealmCell }>
  | Readonly<{ location: RealmCell; region: GameRegion }>
)>;

type GameUnitRef = Readonly<{
  instanceId: StateHash;
  kind: 'avatar' | 'minion';
  seat: GameSeat;
}>;

type GameLocation = Readonly<{ cell: RealmCell; region: GameRegion }>;

type ProjectileDirection = 'east' | 'north' | 'south' | 'west';

type CombatTarget = GameUnitRef | Readonly<{
  instanceId: StateHash;
  kind: 'site';
  seat: GameSeat;
}>;

type PendingCombat = Readonly<{
  allocations: readonly Readonly<{ amount: number; targetInstanceId: StateHash }>[];
  attacker: GameUnitRef;
  attackingSeat: GameSeat;
  cell: RealmCell;
  combatants: readonly GameUnitRef[];
  defenders: readonly GameUnitRef[];
  originalTarget: CombatTarget | null;
  region?: 'underground' | 'underwater' | 'void';
  targetRemoved: boolean;
}>;

type PendingChainMagic = Readonly<{
  cardId: string;
  cardInstanceId: StateHash;
  casterInstanceId: StateHash;
  seat: GameSeat;
  targets: readonly GameUnitRef[];
}>;

type PendingCemeterySummon = Readonly<{
  cardInstanceId: StateHash;
  cardOwner: GameSeat;
  casterInstanceId: StateHash;
  seat: GameSeat;
  sourceMagicInstanceId: StateHash;
}>;

type PendingRandomOutcome = Readonly<{
  action: GameActionDescriptor;
  outcomeInstanceIds: readonly StateHash[];
  seat: GameSeat;
}>;

type PendingStartTurn = Readonly<{
  remainingTriggerInstanceIds: readonly StateHash[];
  seat: GameSeat;
}>;

type PendingDiscardCards = Readonly<{
  remaining: number;
  seat: GameSeat;
  sourceCardId: string;
  sourceInstanceId: StateHash;
  sourceOwner: GameSeat;
}>;

type PendingDeathriteSource = Readonly<{
  controller: GameSeat;
  currentPower: number;
  instanceId: StateHash;
  lethal: boolean;
  unit: UnitInstance;
}>;

type PendingDeathriteBatch = Readonly<{
  activeOrder: readonly PendingDeathriteSource[];
  activeRemaining: readonly PendingDeathriteSource[];
  nonActiveOrder: readonly PendingDeathriteSource[];
  nonActiveRemaining: readonly PendingDeathriteSource[];
  resolving: readonly PendingDeathriteSource[];
  stage: 'active-order' | 'non-active-order' | 'resolve';
}>;

type GamePhase = 'allocate' | 'attack' | 'cemetery-summon' | 'chain-magic' | 'deathrite-order' | 'defend' | 'discard-card' | 'draw' | 'end-turn-aura' | 'genesis' | 'intercept' | 'main' | 'movement' | 'mulligan' | 'random-choice' | 'ranged-step' | 'start-turn' | 'terminal';

type LeapAttackContinuation = Readonly<{
  ally: GameUnitRef;
  cardId: string;
  instanceId: StateHash;
  kind: 'leap-attack';
  owner: GameSeat;
  strikeLocation: GameLocation;
}>;

type PlaySiteDescriptor = Readonly<{
  cardId: string;
  cardInstanceId: StateHash;
  cell: RealmCell;
  createRubbleAt?: RealmCell;
  fromTopAtlas?: true;
  genesisTokenChoice?: 'decline' | 'defer' | 'pay-one-mana';
  kind: 'play-site';
}>;

type SiteGenesisContinuation = Readonly<{
  descriptor: PlaySiteDescriptor;
  genesisGainMana: number;
  genesisSpellDrawCount: number;
  kind: 'site-genesis';
  originStateVersion: number;
  seat: GameSeat;
}>;

type DragProjectileContinuation = Readonly<{
  fightOnArrival: boolean;
  kind: 'drag-projectile';
  path: readonly GameLocation[];
  pathIndex: number;
  shooter: GameUnitRef;
  target: GameUnitRef;
}>;

type DeathriteContinuation =
  | Readonly<{
    cardId: string;
    instanceId: StateHash;
    kind: 'blink';
    owner: GameSeat;
    seat: GameSeat;
    zone: DeckZone;
  }>
  | Readonly<{
    kind: 'end-turn';
    remainingInstanceIds: readonly StateHash[];
    seat: GameSeat;
  }>
  | Readonly<{
    attackerStrikesFirst: boolean;
    firstCombatantInstanceIds: readonly StateHash[];
    kind: 'first-strike';
    pending: PendingCombat;
  }>
  | DragProjectileContinuation
  | LeapAttackContinuation
  | SiteGenesisContinuation
  | Readonly<{
    caster: GameUnitRef;
    descriptor: SummonMinionDescriptor;
    kind: 'paid-summon';
    unit: UnitInstance;
  }>;

type PendingDeathrites = Readonly<{
  batches: readonly PendingDeathriteBatch[];
  continuation?: DeathriteContinuation;
  corpses: readonly UnitInstance[];
  deckLosers: readonly GameSeat[];
  deferredOutcomes?: readonly GameOutcome[];
  defeatedAvatars: readonly GameSeat[];
  returnDecisionSeat?: GameSeat;
  returnPhase?: GamePhase;
}>;

type PendingEndTurnAura = Readonly<{
  auraInstanceId: StateHash;
  outcomeInstanceIds?: readonly StateHash[];
  remainingAuraInstanceIds: readonly StateHash[];
  seat: GameSeat;
  stage: 'move' | 'random';
}>;

type PendingBasicMovement = Readonly<{
  activationEmitted?: true;
  path: readonly GameLocation[];
  pathIndex: number;
  purpose: 'defend' | 'move-and-attack';
  rangedStrikeUsed: boolean;
  seat: GameSeat;
  sourceInstanceId: StateHash;
}>;

type PendingGenesisSpell = Readonly<{
  seat: GameSeat;
  sourceInstanceId: StateHash;
}>;

type PendingGenesisSpellOrder = Readonly<{
  count: number;
  seat: GameSeat;
  sourceInstanceId: StateHash;
}>;

type PendingGenesisToken = Readonly<{
  cell: RealmCell;
  seat: GameSeat;
  sourceInstanceId: StateHash;
}>;

type PendingRangedStep = Readonly<{
  seat: GameSeat;
  sourceInstanceId: StateHash;
}>;

type PlayerState = Readonly<{
  airThresholdsCastThisTurn?: number;
  atlas: readonly CardInstance[];
  avatar: Readonly<{
    card: CardInstance;
    deathDoorTurn: number | null;
    lastDroppedArtifactsTurn?: number;
    lastInteractedTurn?: number;
    lastPickedUpArtifactsTurn?: number;
    life: number;
    location: RealmCell;
    region: GameRegion;
    tapped: boolean;
    temporaryPowerSources?: readonly StateHash[];
  }>;
  cemetery: readonly CardInstance[];
  domainEstablished: boolean;
  hand: Readonly<Record<DeckZone, readonly CardInstance[]>>;
  mana: number;
  mulliganComplete: boolean;
  spellbook: readonly CardInstance[];
}>;

export type GameTerminal =
  | Readonly<{ status: 'active' }>
  | Readonly<{
    loser: GameSeat;
    reason: 'avatar_defeated' | 'deck_empty';
    status: 'finished';
    winner: GameSeat;
  }>
  | Readonly<{
    reason: 'simultaneous_avatar_defeat' | 'simultaneous_defeat';
    result: 'draw';
    status: 'finished';
  }>;

export type GameState = Readonly<{
  activeSeat: GameSeat;
  cards: Readonly<Record<string, GameCardDefinition>>;
  decisionSeat: GameSeat;
  engine: EngineState;
  pendingBasicMovement?: PendingBasicMovement | null;
  pendingCemeterySummon?: PendingCemeterySummon;
  pendingChainMagic?: PendingChainMagic | null;
  pendingCombat: PendingCombat | null;
  pendingDeathrites?: PendingDeathrites | null;
  pendingDiscardCards?: PendingDiscardCards;
  pendingEndTurnAura?: PendingEndTurnAura | null;
  pendingGenesisSpell?: PendingGenesisSpell | null;
  pendingGenesisSpellOrder?: PendingGenesisSpellOrder | null;
  pendingGenesisToken?: PendingGenesisToken | null;
  pendingRandomOutcome?: PendingRandomOutcome | null;
  pendingRangedStep?: PendingRangedStep | null;
  pendingStartTurn?: PendingStartTurn;
  phase: GamePhase;
  players: Readonly<Record<GameSeat, PlayerState>>;
  realm: Readonly<{
    artifacts?: readonly ArtifactInstance[];
    auras?: readonly AuraInstance[];
    immobileAreas?: readonly ImmobileArea[];
    sites: Readonly<Partial<Record<RealmCell, RealmSiteInstance>>>;
    units: readonly UnitInstance[];
  }>;
  schemaVersion: 1;
  stateVersion: number;
  terminal: GameTerminal;
  turnNumber: number;
}>;

type ObservedPlayer = Readonly<{
  affinity: GameThresholds;
  airThresholdsCastThisTurn?: number;
  atlasCount: number;
  avatar: Readonly<{
    attack: number;
    cardId: string;
    deathDoorTurn: number | null;
    defense: number;
    immobile: boolean;
    instanceId: StateHash;
    life: number;
    location: RealmCell;
    region: GameRegion;
    tapped: boolean;
    temporaryPowerSources?: readonly StateHash[];
  }>;
  cemetery: readonly Readonly<{ cardId: string; instanceId: StateHash }>[];
  domainEstablished: boolean;
  hand: Readonly<{
    atlas: number | readonly Readonly<{ cardId: string; instanceId: StateHash }>[];
    spellbook: number | readonly Readonly<{ cardId: string; instanceId: StateHash }>[];
  }>;
  mana: number;
  mulliganComplete: boolean;
  spellbookCount: number;
}>;

export type GameObservation = Readonly<{
  activeSeat: GameSeat;
  decisionSeat: GameSeat;
  pendingCombat: PendingCombat | null;
  phase: GameState['phase'];
  players: Readonly<Record<GameSeat, ObservedPlayer>>;
  realm: Readonly<{
    artifacts?: readonly Readonly<{
      bearer?: GameUnitRef;
      cardId: string;
      controller: GameSeat | null;
      instanceId: StateHash;
      location: RealmCell;
      owner: GameSeat;
      region: GameRegion;
    }>[];
    auras?: readonly Readonly<{
      cardId: string;
      cells: readonly RealmCell[];
      controller: GameSeat;
      instanceId: StateHash;
      owner: GameSeat;
      turnCounters: number;
      visitedCells?: readonly RealmCell[];
    }>[];
    immobileAreas?: readonly ImmobileArea[];
    sites: Readonly<Partial<Record<RealmCell,
      | Readonly<{
        cardId: 'rubble';
        controller: null;
        elements: readonly [];
        instanceId: StateHash;
        rubble: true;
      }>
      | Readonly<{
        cardId: string;
        controller: GameSeat;
        elements: readonly GameElement[];
        instanceId: StateHash;
        owner: GameSeat;
      }>>>>;
    units: readonly Readonly<{
      airborne: boolean;
      attack: number;
      cardId: string;
      carriedLanceCount?: number;
      controller: GameSeat;
      damage: number;
      defense: number;
      disabled: boolean;
      immobile: boolean;
      instanceId: StateHash;
      location: RealmCell;
      occupiedCells?: TwoByTwoArea;
      owner: GameSeat;
      region: GameRegion;
      stealthed: boolean;
      summoningSickness: boolean;
      tapped: boolean;
      temporaryPowerSources?: readonly StateHash[];
      token?: true;
      warded: boolean;
    }>[];
  }>;
  schemaVersion: 1;
  stateVersion: number;
  terminal: GameTerminal;
  turnNumber: number;
  viewer: GameSeat;
}>;

type MulliganDescriptor = Readonly<{
  atlasOrder: readonly string[];
  kind: 'mulligan';
  spellbookOrder: readonly string[];
}>;

type SummonMinionDescriptor = Readonly<{
  cardId: string;
  cardInstanceId: string;
  casterInstanceId: string;
  cell: RealmCell;
  cells?: TwoByTwoArea;
  kind: 'summon-minion';
  manaCost: number;
  genesisDamageChoice?: 'decline' | 'target';
  genesisDamageTarget?: GameUnitRef;
  paymentMode?: 'random-card-discard';
  region?: 'underground' | 'underwater' | 'void';
  sacrificedMinionInstanceIds?: readonly StateHash[];
}>;

type GameActionDescriptor =
  | MulliganDescriptor
  | Readonly<{ kind: 'draw-site' }>
  | Readonly<{ kind: 'draw-spell' }>
  | PlaySiteDescriptor
  | Readonly<{
    kind: 'replace-rubble-with-top-atlas-site';
    targetCell: RealmCell;
    targetRubbleInstanceId: StateHash;
  }>
  | Readonly<{
    choice: 'decline' | 'pay-one-mana';
    kind: 'resolve-genesis-token';
  }>
  | Readonly<{
    choice: 'bottom-next' | 'keep-next';
    kind: 'resolve-genesis-spell';
  }>
  | Readonly<{
    kind: 'resolve-genesis-spell-order';
    order: readonly number[];
  }>
  | Readonly<{
    kind: 'activate-site-destruction';
    sourceSiteInstanceId: StateHash;
    targetCell: RealmCell;
    targetSiteInstanceId: StateHash;
  }>
  | Readonly<{
    kind: 'fly-site';
    sourceSiteInstanceId: StateHash;
    targetCell: RealmCell;
  }>
  | Readonly<{
    bearer?: GameUnitRef;
    bearerCell?: RealmCell;
    cardId: string;
    cardInstanceId: string;
    casterInstanceId: string;
    cell?: RealmCell;
    kind: 'cast-artifact';
    manaCost: number;
  }>
  | SummonMinionDescriptor
  | Readonly<{
    cardId: string;
    cardInstanceId: string;
    casterInstanceId: StateHash;
    cells: readonly RealmCell[];
    kind: 'cast-aura';
  }>
  | Readonly<{
    cardId: string;
    cardInstanceId: StateHash;
    casterInstanceId: StateHash;
    kind: 'begin-chain-magic';
    target: GameUnitRef;
  }>
  | Readonly<{
    kind: 'extend-chain-magic';
    target: GameUnitRef;
  }>
  | Readonly<{ kind: 'resolve-chain-magic' }>
  | Readonly<{
    cardId: string;
    cardInstanceId: string;
    casterInstanceId: StateHash;
    cemeteryMinionInstanceId?: StateHash;
    discardCardInstanceId?: StateHash;
    discardSiteInstanceId?: StateHash;
    drawZone?: DeckZone;
    kind: 'cast-magic';
    ally?: GameUnitRef;
    allyDestination?: GameLocation;
    allyDestinationCells?: TwoByTwoArea;
    allyStrikeLocation?: GameLocation;
    target?: GameUnitRef;
    targets?: readonly GameUnitRef[];
    targetArtifactInstanceId?: StateHash;
    targetAuraInstanceId?: StateHash;
    targetLocation?: GameLocation;
    targetSiteInstanceId?: StateHash;
    temptedDestination?: GameLocation;
    temptedEnemy?: GameUnitRef;
  }>
  | Readonly<{ kind: 'draw'; zone: DeckZone }>
  | Readonly<{
    from: GameLocation;
    kind: 'move-and-attack';
    path: readonly GameLocation[];
    to: GameLocation;
    unitInstanceId: StateHash;
  }>
  | Readonly<{
    kind: 'continue-basic-movement';
    unitInstanceId: StateHash;
  }>
  | Readonly<{
    direction: ProjectileDirection;
    hit: GameUnitRef | null;
    kind: 'shoot-projectile';
    path: readonly GameLocation[];
    shooterInstanceId: StateHash;
  }>
  | Readonly<{
    direction: ProjectileDirection;
    hit: GameUnitRef | null;
    kind: 'shoot-damage-projectile';
    path: readonly GameLocation[];
    shooterInstanceId: StateHash;
  }>
  | Readonly<{
    choice: 'decline';
    kind: 'resolve-ranged-step';
    unitInstanceId: StateHash;
  }>
  | Readonly<{
    choice: 'step';
    from: GameLocation;
    kind: 'resolve-ranged-step';
    path: readonly GameLocation[];
    to: GameLocation;
    unitInstanceId: StateHash;
  }>
  | Readonly<{
    direction: ProjectileDirection;
    fightOnArrival: boolean;
    hit: GameUnitRef | null;
    kind: 'shoot-drag-projectile';
    path: readonly GameLocation[];
    shooterInstanceId: StateHash;
  }>
  | Readonly<{
    artifactInstanceIds: readonly StateHash[];
    cell?: RealmCell;
    kind: 'pick-up-artifacts';
    unit: GameUnitRef;
  }>
  | Readonly<{
    artifactInstanceIds: readonly StateHash[];
    kind: 'drop-artifacts';
    unit: GameUnitRef;
  }>
  | Readonly<{
    artifactInstanceId: StateHash;
    helper: GameUnitRef;
    kind: 'activate-artifact-damage';
    target: GameUnitRef;
  }>
  | Readonly<{
    artifactInstanceId: StateHash;
    discardCardInstanceId: StateHash;
    discardZone: DeckZone;
    helper: GameUnitRef;
    kind: 'activate-artifact-discard-area-damage';
    targetLocation: GameLocation;
  }>
  | Readonly<{
    artifactInstanceId: StateHash;
    direction: ProjectileDirection;
    kind: 'activate-artifact-roll-damage';
    path: readonly GameLocation[];
    pusher: GameUnitRef;
  }>
  | Readonly<{ kind: 'decline-attack' }>
  | Readonly<{ kind: 'declare-attack'; target: CombatTarget }>
  | Readonly<{
    from: GameLocation;
    kind: 'defend';
    path: readonly GameLocation[];
    to: GameLocation;
    unitInstanceId: StateHash;
  }>
  | Readonly<{ kind: 'close-defend'; originalTargetParticipates: boolean }>
  | Readonly<{ kind: 'intercept'; unitInstanceId: StateHash }>
  | Readonly<{ kind: 'close-intercept' }>
  | Readonly<{ amount: number; kind: 'allocate-strike'; targetInstanceId: StateHash }>
  | Readonly<{
    kind: 'activate-area-damage';
    sourceInstanceId: StateHash;
    targetLocation: GameLocation;
  }>
  | Readonly<{
    discardCardInstanceId: StateHash;
    kind: 'activate-discard-random-damage';
    sourceInstanceId: StateHash;
  }>
  | Readonly<{
    kind: 'activate-sparkmage';
    sourceInstanceId: StateHash;
    targetLocation: GameLocation;
  }>
  | Readonly<{
    kind: 'resolve-random-outcome';
    outcomeInstanceId: StateHash;
  }>
  | Readonly<{
    kind: 'resolve-start-turn-trigger';
    lureDestination?: GameLocation;
    lureTargetInstanceId?: StateHash;
    sourceInstanceId: StateHash;
  }>
  | Readonly<{
    cardInstanceId: StateHash;
    kind: 'discard-card';
    zone: DeckZone;
  }>
  | Readonly<{
    kind: 'order-deathrites';
    sourceInstanceId: StateHash;
  }>
  | Readonly<{
    auraInstanceId: StateHash;
    kind: 'resolve-end-turn-aura-random';
    outcomeInstanceId: StateHash;
  }>
  | Readonly<{
    auraInstanceId: StateHash;
    cells?: readonly RealmCell[];
    kind: 'resolve-end-turn-aura-move';
  }>
  | Readonly<{ amount: number; kind: 'activate-mana'; unitInstanceId: StateHash }>
  | Readonly<{ kind: 'end-turn' }>;

export type GameLegalAction = EngineLegalAction<GameActionDescriptor>;
export type GameActionRequest = EngineActionRequest;
export type GameReceipt = EngineReceipt;

export type GameSession = Readonly<{
  attempts: readonly EngineAttempt[];
  initialRandomDraws: readonly EngineRandomDraw[];
  manifest: GameManifest;
  state: GameState;
  transcript: readonly GameReceipt[];
}>;

export type GameStepResult =
  | Readonly<{ accepted: true; receipt: GameReceipt; session: GameSession }>
  | Readonly<{ accepted: false; reason: EngineRejection; session: GameSession }>;

type GameOutcome = Readonly<{ payload: JsonValue; type: string }>;

function asJson(value: unknown): JsonValue {
  return value as JsonValue;
}

function requireCardId(value: string, path: string): void {
  if (!value.trim() || value.length > 256) throw new RangeError(`${path} must be 1-256 characters`);
}

const SUPPORTED_CARD_FIELDS = {
  artifact: new Set(`
    atEndOfEachTurnSiteControllerLosesLife
    atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn
    bearerControllerChoosesExtraRandomOutcome cardType
    grantsBearerLethal grantsBearerPower manaCost nearbyMinionsMustAttackIfAble
    nearbyStrikesAgainstUnitsDealDoubleDamage
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath thresholds
  `.trim().split(/\s+/)),
  aura: new Set(`
    affectedSitesAreFlooded
    affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep
    atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent
    atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf cardType
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns manaCost thresholds
  `.trim().split(/\s+/)),
  avatar: new Set(`
    attack cardType defense drawSpell earthSitePlayCreatesAdjacentRubble life
    replaceAdjacentRubbleWithTopAtlasSite
    tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn
  `.trim().split(/\s+/)),
  magic: new Set(`
    burrowAllMinionsAndArtifactsAtTargetLandSite burrowTargetMinionOrArtifact cardType
    damageChainNearbyUnits damageEachAbovegroundMinion damageEachUnitAtLocationWithinTwoSteps
    damageRandomUnitAtLocation damageTargetUnit disableTargetNearbyMinionUntilNextTurn
    damageUnitsAboveAndBelowTargetSiteByManhattanDistance discardCardAsAdditionalCost
    discardSiteAsAdditionalCost
    destroyTargetArtifact destroyTargetAura destroyTargetSite
    fightAllyWithAdjacentEnemy gainControlOfTargetNearbyMinion grantAirborneToAllyThisTurn
    grantChargeToAllyThisTurn grantFirstStrikeToAllyThisTurn grantLethalToAllyThisTurn grantRangedToAllyThisTurn
    grantPowerToAllyThisTurn grantStealthToTargetMinion grantWardToTargetMinion healController healTargetMinion killTargetMinion killTargetWoundedMinion leapAttackAlly drawSites drawSpells
    lureEnemyMinionOneStepCloser manaCost millSites millSpells returnMinionFromOwnCemetery
    returnTargetArtifactFromOwnCemetery returnTargetAuraFromOwnCemetery returnTargetMagicFromOwnCemetery
    returnTargetArtifactToOwnerHand
    returnTargetAuraToOwnerHand
    returnTargetMinionToOwnerHand returnTargetSiteFromOwnCemetery returnTargetSiteToOwnerHand
    submergeTargetMinion
    summonRandomMinionFromAnyCemetery summonTokenToEachControlledSiteBorderingEnemySite
    tapTargetMinion targetNearby targetPlayerDiscardsCards targetPlayerDrawsSites targetPlayerDrawsSpells targetPlayerGainsLife targetPlayerLosesLife teleportAllyToTargetSite
    teleportNearbyAllyThenDrawCard thresholds untapTargetMinion untapTargetMinionAfterDamage
  `.trim().split(/\s+/)),
  minion: new Set(`
    airborne atEndOfControllerTurnControllerGainsLife atEndOfControllerTurnControllerLosesLife atEndOfControllerTurnDamageEachOtherUnitHere atStartOfControllerTurnControllerGainsLife atStartOfControllerTurnControllerGainsMana atStartOfControllerTurnControllerLosesLife atStartOfControllerTurnDamageEachOtherUnitHere atStartOfControllerTurnDrawSites atStartOfControllerTurnDrawSpells atStartOfControllerTurnLureNearbyEnemyMinion atStartOfControllerTurnMillSites atStartOfControllerTurnMillSpells atStartOfControllerTurnTeleportToRandomSiteOrVoid attack burrowing cardType
    cannotAttackSites cannotDefend cannotDefendOrIntercept
    charge connectsTopBottom deathriteDamageEachUnitHere deathriteDrawSite deathriteDrawSpells deathriteHeal deathriteMillSites deathriteMillSpells
    deathriteLoseLifePerNearbySiteControlled defense discardRandomCardInsteadOfMana
    discardSpellToDamageRandomOtherUnitHere diesAtEndOfControllerTurn enemiesMustAttackThisIfAble genesisDamageEachOtherUnitHere
    genesisDisableSelfUntilDamaged genesisDrawSite genesisDrawSpells genesisHealController
    genesisLoseControllerLife genesisMayDamageTargetAdjacentUnit genesisStrikeEachEnemyHere
    gainsPowerRangedAndSpellcasterAtopTower gainsStealthAtEndOfTurn
    gainsStealthAtEndOfTurnIfNoEnemiesNearby immobile lanceCount lethal
    manaCost mayRangedStrikeOnceDuringBasicMovement mayStepAfterRangedStrike mortal movementBonus
    movesOnlyForward movesOnlySideways mustAttackAUnitIfAble
    mustBeCastBurrowed mustBeCastSubmerged mustBeCastToOuterColumn mustBeCastToWaterSite
    nearbyEnemiesPermanentlyLoseStealth occupiesSquareArea ordinary otherControlledMortalsPowerBonus
    otherNearbyAlliesPowerBonus preventsDamageFromUnitsWithPowerAtLeast provides ranged
    sacrificeMinionAtSummoningLocationForManaDiscount shootsDragProjectile siteProvidesNoThreshold
    spellcaster stealth strikesFirstWhileAttacking strikesFirstWhileDefending submerge summonToAnySite
    tapToDamageEachUnitAtAdjacentLocation tapToShootProjectileDamage tapForMana takesLessDamage thresholds token
    doesNotUntapDuringControllersStartPhase untapsAtEndOfControllerTurn voidwalk ward waterbound
  `.trim().split(/\s+/)),
  site: new Set(`
    airborneMinionsAtopMoveFreelyAway blocksGroundMinionEntryWhileMinionAtop
    cannotBeMovedDestroyedOrModified cardType connectsBurrowedAllies elements
    flyToNearbyVoidOncePerTurnAtAirThreshold
    genesisDiscardTopSpells genesisDrawSpellPerAdjacentSameCard genesisEnemiesLoseStealth
    genesisGainMana genesisGainManaIfOnlyControlledCopy genesisHealNearbyAvatars
    genesisImmobilizeNearbyUntilNextTurn genesisMayBottomNextSpell genesisPayOneManaToSummonToken
    genesisReorderNextSpells
    isTower ordinaryMinionManaDiscount rangedUnitsHereRangeBonus sacrificeToDestroyNearbySite
    minionsHereGainVoidwalkUntilLeavingVoid
    preventsUnitsWithPowerAtLeastFromEntering uniqueOrLegendary
  `.trim().split(/\s+/)),
} satisfies Readonly<Record<GameCardDefinition['cardType'], ReadonlySet<string>>>;

function rejectUnknownCardFields(card: GameCardDefinition, path: string): void {
  const fields = SUPPORTED_CARD_FIELDS[card.cardType as keyof typeof SUPPORTED_CARD_FIELDS];
  if (!fields) throw new RangeError(`${path}.cardType is unsupported`);
  const unknown = Object.keys(card).find((field) => !fields.has(field));
  if (unknown) throw new RangeError(`${path}.${unknown} is unsupported`);
}

function rejectUnknownThresholds(
  thresholds: GameThresholds,
  path: string,
  elements: readonly GameElement[],
): void {
  const unknown = Object.keys(thresholds).find((field) => !elements.includes(field as GameElement));
  if (unknown) throw new RangeError(`${path}.thresholds.${unknown} is unsupported`);
}

function validateCardDefinition(card: GameCardDefinition, path: string): void {
  const elements: readonly GameElement[] = ['earth', 'fire', 'water', 'air'];
  if (Object.prototype.hasOwnProperty.call(card, 'genesisDrawSpell')) {
    throw new RangeError(`${path}.genesisDrawSpell is obsolete; use genesisDrawSpells`);
  }
  rejectUnknownCardFields(card, path);
  if (card.cardType === 'avatar') {
    if (typeof card.drawSpell !== 'boolean') throw new RangeError(`${path}.drawSpell must be boolean`);
    if (card.earthSitePlayCreatesAdjacentRubble !== undefined
      && card.earthSitePlayCreatesAdjacentRubble !== true) {
      throw new RangeError(`${path}.earthSitePlayCreatesAdjacentRubble must be true when defined`);
    }
    if (card.replaceAdjacentRubbleWithTopAtlasSite !== undefined
      && card.replaceAdjacentRubbleWithTopAtlasSite !== true) {
      throw new RangeError(`${path}.replaceAdjacentRubbleWithTopAtlasSite must be true when defined`);
    }
    if (card.tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn !== undefined
      && card.tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn !== true) {
      throw new RangeError(
        `${path}.tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn must be true when defined`,
      );
    }
    for (const field of ['attack', 'defense', 'life'] as const) {
      if (!Number.isSafeInteger(card[field])
        || card[field] < (field === 'life' ? 1 : 0)
        || card[field] > MAX_COMBAT_STAT) {
        throw new RangeError(`${path}.${field} must be a safe integer between ${field === 'life' ? 1 : 0} and ${MAX_COMBAT_STAT}`);
      }
    }
    return;
  }
  if (card.cardType === 'site') {
    if (card.airborneMinionsAtopMoveFreelyAway !== undefined
      && card.airborneMinionsAtopMoveFreelyAway !== true) {
      throw new RangeError(
        `${path}.airborneMinionsAtopMoveFreelyAway must be true when defined`,
      );
    }
    if (card.blocksGroundMinionEntryWhileMinionAtop !== undefined
      && card.blocksGroundMinionEntryWhileMinionAtop !== true) {
      throw new RangeError(
        `${path}.blocksGroundMinionEntryWhileMinionAtop must be true when defined`,
      );
    }
    if (card.cannotBeMovedDestroyedOrModified !== undefined
      && card.cannotBeMovedDestroyedOrModified !== true) {
      throw new RangeError(
        `${path}.cannotBeMovedDestroyedOrModified must be true when defined`,
      );
    }
    if (card.flyToNearbyVoidOncePerTurnAtAirThreshold !== undefined
      && card.flyToNearbyVoidOncePerTurnAtAirThreshold !== 3) {
      throw new RangeError(
        `${path}.flyToNearbyVoidOncePerTurnAtAirThreshold must be 3`,
      );
    }
    if (card.preventsUnitsWithPowerAtLeastFromEntering !== undefined
      && (!Number.isSafeInteger(card.preventsUnitsWithPowerAtLeastFromEntering)
        || card.preventsUnitsWithPowerAtLeastFromEntering < 1
        || card.preventsUnitsWithPowerAtLeastFromEntering > MAX_COMBAT_STAT)) {
      throw new RangeError(
        `${path}.preventsUnitsWithPowerAtLeastFromEntering must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
      );
    }
    if (!Array.isArray(card.elements)
      || card.elements.some((element) => !elements.includes(element))
      || new Set(card.elements).size !== card.elements.length
      || card.elements.some((element, index) => elements.indexOf(element) <= elements.indexOf(card.elements[index - 1]!))) {
      throw new RangeError(`${path}.elements must contain unique elements in canonical order`);
    }
    if (card.genesisGainMana !== undefined
      && (!Number.isSafeInteger(card.genesisGainMana)
        || card.genesisGainMana < 1
        || card.genesisGainMana > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.genesisGainMana must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
    }
    if (card.genesisGainManaIfOnlyControlledCopy !== undefined
      && card.genesisGainManaIfOnlyControlledCopy !== 1) {
      throw new RangeError(`${path}.genesisGainManaIfOnlyControlledCopy must be 1`);
    }
    if (card.genesisGainMana !== undefined
      && card.genesisGainManaIfOnlyControlledCopy !== undefined) {
      throw new RangeError(`${path} simultaneous unconditional and conditional Genesis mana are unsupported`);
    }
    if (card.genesisHealNearbyAvatars !== undefined
      && card.genesisHealNearbyAvatars !== 3) {
      throw new RangeError(`${path}.genesisHealNearbyAvatars must be 3`);
    }
    if (card.genesisImmobilizeNearbyUntilNextTurn !== undefined
      && card.genesisImmobilizeNearbyUntilNextTurn !== true) {
      throw new RangeError(
        `${path}.genesisImmobilizeNearbyUntilNextTurn must be true when defined`,
      );
    }
    if (card.isTower !== undefined && card.isTower !== true) {
      throw new RangeError(`${path}.isTower must be true when defined`);
    }
    if (card.minionsHereGainVoidwalkUntilLeavingVoid !== undefined
      && card.minionsHereGainVoidwalkUntilLeavingVoid !== true) {
      throw new RangeError(
        `${path}.minionsHereGainVoidwalkUntilLeavingVoid must be true when defined`,
      );
    }
    if (card.genesisPayOneManaToSummonToken !== undefined) {
      requireCardId(
        card.genesisPayOneManaToSummonToken,
        `${path}.genesisPayOneManaToSummonToken`,
      );
    }
    if (card.genesisPayOneManaToSummonToken !== undefined
      && (card.genesisDiscardTopSpells !== undefined
        || card.genesisDrawSpellPerAdjacentSameCard
        || card.genesisEnemiesLoseStealth
        || card.genesisGainMana !== undefined
        || card.genesisGainManaIfOnlyControlledCopy !== undefined
        || card.genesisHealNearbyAvatars !== undefined
        || card.genesisImmobilizeNearbyUntilNextTurn !== undefined
        || card.genesisMayBottomNextSpell !== undefined
        || card.genesisReorderNextSpells !== undefined)) {
      throw new RangeError(`${path} simultaneous paid-token and another site Genesis are unsupported`);
    }
    if (card.genesisDrawSpellPerAdjacentSameCard !== undefined
      && typeof card.genesisDrawSpellPerAdjacentSameCard !== 'boolean') {
      throw new RangeError(`${path}.genesisDrawSpellPerAdjacentSameCard must be boolean`);
    }
    if (card.genesisDiscardTopSpells !== undefined && card.genesisDiscardTopSpells !== 2) {
      throw new RangeError(`${path}.genesisDiscardTopSpells must be 2`);
    }
    if (card.genesisDiscardTopSpells !== undefined && card.genesisDrawSpellPerAdjacentSameCard) {
      throw new RangeError(`${path} simultaneous Genesis spell discard and draw are unsupported`);
    }
    if (card.genesisEnemiesLoseStealth !== undefined && card.genesisEnemiesLoseStealth !== true) {
      throw new RangeError(`${path}.genesisEnemiesLoseStealth must be true`);
    }
    if (card.genesisMayBottomNextSpell !== undefined
      && card.genesisMayBottomNextSpell !== true) {
      throw new RangeError(`${path}.genesisMayBottomNextSpell must be true`);
    }
    if (card.genesisMayBottomNextSpell === true
      && (card.genesisDiscardTopSpells !== undefined
        || card.genesisDrawSpellPerAdjacentSameCard
        || card.genesisEnemiesLoseStealth
        || card.genesisGainMana !== undefined
        || card.genesisGainManaIfOnlyControlledCopy !== undefined
        || card.genesisHealNearbyAvatars !== undefined
        || card.genesisImmobilizeNearbyUntilNextTurn !== undefined
        || card.genesisPayOneManaToSummonToken !== undefined
        || card.genesisReorderNextSpells !== undefined)) {
      throw new RangeError(`${path} simultaneous next-spell and another site Genesis are unsupported`);
    }
    if (card.genesisReorderNextSpells !== undefined
      && card.genesisReorderNextSpells !== 3) {
      throw new RangeError(`${path}.genesisReorderNextSpells must be 3`);
    }
    if (card.genesisReorderNextSpells === 3
      && (card.genesisDiscardTopSpells !== undefined
        || card.genesisDrawSpellPerAdjacentSameCard
        || card.genesisEnemiesLoseStealth
        || card.genesisGainMana !== undefined
        || card.genesisGainManaIfOnlyControlledCopy !== undefined
        || card.genesisHealNearbyAvatars !== undefined
        || card.genesisImmobilizeNearbyUntilNextTurn !== undefined
        || card.genesisMayBottomNextSpell !== undefined
        || card.genesisPayOneManaToSummonToken !== undefined)) {
      throw new RangeError(`${path} simultaneous spell-order and another site Genesis are unsupported`);
    }
    if (card.connectsBurrowedAllies !== undefined && typeof card.connectsBurrowedAllies !== 'boolean') {
      throw new RangeError(`${path}.connectsBurrowedAllies must be boolean`);
    }
    if (card.sacrificeToDestroyNearbySite !== undefined
      && card.sacrificeToDestroyNearbySite !== true) {
      throw new RangeError(`${path}.sacrificeToDestroyNearbySite must be true when defined`);
    }
    if (card.ordinaryMinionManaDiscount !== undefined
      && card.ordinaryMinionManaDiscount !== 1) {
      throw new RangeError(`${path}.ordinaryMinionManaDiscount must be 1`);
    }
    if (card.rangedUnitsHereRangeBonus !== undefined
      && card.rangedUnitsHereRangeBonus !== 1) {
      throw new RangeError(`${path}.rangedUnitsHereRangeBonus must be 1`);
    }
    if (card.uniqueOrLegendary !== undefined && card.uniqueOrLegendary !== true) {
      throw new RangeError(`${path}.uniqueOrLegendary must be true when defined`);
    }
    return;
  }
  if (card.cardType === 'artifact') {
    if (card.atEndOfEachTurnSiteControllerLosesLife !== undefined
      && (!Number.isSafeInteger(card.atEndOfEachTurnSiteControllerLosesLife)
        || card.atEndOfEachTurnSiteControllerLosesLife < 1
        || card.atEndOfEachTurnSiteControllerLosesLife > MAX_COMBAT_STAT)) {
      throw new RangeError(
        `${path}.atEndOfEachTurnSiteControllerLosesLife must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
      );
    }
    if (card.atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn !== undefined
      && (!Number.isSafeInteger(card.atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn)
        || card.atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn < 1
        || card.atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn > MAX_COMBAT_STAT)) {
      throw new RangeError(
        `${path}.atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
      );
    }
    if (card.grantsBearerPower !== undefined && card.grantsBearerPower !== 2) {
      throw new RangeError(`${path}.grantsBearerPower must be 2`);
    }
    if (card.grantsBearerLethal !== undefined && card.grantsBearerLethal !== true) {
      throw new RangeError(`${path}.grantsBearerLethal must be true`);
    }
    if (card.nearbyMinionsMustAttackIfAble !== undefined
      && card.nearbyMinionsMustAttackIfAble !== true) {
      throw new RangeError(`${path}.nearbyMinionsMustAttackIfAble must be true`);
    }
    if (card.nearbyStrikesAgainstUnitsDealDoubleDamage !== undefined
      && card.nearbyStrikesAgainstUnitsDealDoubleDamage !== true) {
      throw new RangeError(`${path}.nearbyStrikesAgainstUnitsDealDoubleDamage must be true`);
    }
    if (card.bearerControllerChoosesExtraRandomOutcome !== undefined
      && card.bearerControllerChoosesExtraRandomOutcome !== true) {
      throw new RangeError(
        `${path}.bearerControllerChoosesExtraRandomOutcome must be true`,
      );
    }
    if (card.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps !== undefined
      && card.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps !== 3) {
      throw new RangeError(
        `${path}.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps must be 3`,
      );
    }
    if (card
      .tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
        !== undefined
      && card
        .tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
          !== true) {
      throw new RangeError(
        `${path}.tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps must be true`,
      );
    }
    if (card.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath !== undefined
      && card.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath !== 4) {
      throw new RangeError(
        `${path}.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath must be 4`,
      );
    }
    const exclusiveArtifactEffects = Number(card.atEndOfEachTurnSiteControllerLosesLife !== undefined)
      + Number(card.atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn !== undefined)
      + Number(card.bearerControllerChoosesExtraRandomOutcome === true)
      + Number(card.grantsBearerPower === 2)
      + Number(card.grantsBearerLethal === true)
      + Number(card.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps === 3)
      + Number(card
        .tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
          === true)
      + Number(card.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath === 4);
    const maskArtifactEffects = Number(card.nearbyMinionsMustAttackIfAble === true)
      + Number(card.nearbyStrikesAgainstUnitsDealDoubleDamage === true);
    if (exclusiveArtifactEffects > 1
      || (exclusiveArtifactEffects === 1 && maskArtifactEffects > 0)
      || (exclusiveArtifactEffects === 0 && maskArtifactEffects === 0)) {
      throw new RangeError(`${path} must define exactly one supported Artifact effect`);
    }
    if (!Number.isSafeInteger(card.manaCost) || card.manaCost < 0) {
      throw new RangeError(`${path}.manaCost must be a supported nonnegative safe integer`);
    }
    rejectUnknownThresholds(card.thresholds, path, elements);
    for (const element of elements) {
      if (!Number.isSafeInteger(card.thresholds[element]) || card.thresholds[element] < 0) {
        throw new RangeError(`${path}.thresholds.${element} must be a nonnegative safe integer`);
      }
    }
    return;
  }
  if (card.cardType === 'aura') {
    if (card.immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns !== undefined
      && card.immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns !== true) {
      throw new RangeError(
        `${path}.immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns must be true`,
      );
    }
    if (card.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep
      !== undefined
      && card.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep !== 3) {
      throw new RangeError(
        `${path}.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep must be 3`,
      );
    }
    if (card.atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf !== undefined
      && card.atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf !== true) {
      throw new RangeError(
        `${path}.atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf must be true`,
      );
    }
    if (card.atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent !== undefined
      && card.atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent !== 3) {
      throw new RangeError(
        `${path}.atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent must be 3`,
      );
    }
    if (card.affectedSitesAreFlooded !== undefined && card.affectedSitesAreFlooded !== true) {
      throw new RangeError(`${path}.affectedSitesAreFlooded must be true`);
    }
    if (card.affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold !== undefined
      && card.affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold !== true) {
      throw new RangeError(
        `${path}.affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold must be true`,
      );
    }
    if (Number(card.immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns === true)
      + Number(card.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep === 3)
      + Number(card.atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf === true)
      + Number(card.atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent === 3)
      + Number(card.affectedSitesAreFlooded === true)
      + Number(card.affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold === true)
        !== 1) {
      throw new RangeError(`${path} must define exactly one supported Aura effect`);
    }
    if (!Number.isSafeInteger(card.manaCost) || card.manaCost < 0) {
      throw new RangeError(`${path}.manaCost must be a supported nonnegative safe integer`);
    }
    rejectUnknownThresholds(card.thresholds, path, elements);
    for (const element of elements) {
      if (!Number.isSafeInteger(card.thresholds[element]) || card.thresholds[element] < 0) {
        throw new RangeError(`${path}.thresholds.${element} must be a nonnegative safe integer`);
      }
    }
    return;
  }
  if (card.cardType === 'magic') {
    if (card.burrowAllMinionsAndArtifactsAtTargetLandSite !== undefined
      && card.burrowAllMinionsAndArtifactsAtTargetLandSite !== true) {
      throw new RangeError(
        `${path}.burrowAllMinionsAndArtifactsAtTargetLandSite must be true when defined`,
      );
    }
    if (card.burrowTargetMinionOrArtifact !== undefined
      && card.burrowTargetMinionOrArtifact !== true) {
      throw new RangeError(`${path}.burrowTargetMinionOrArtifact must be true when defined`);
    }
    if (card.submergeTargetMinion !== undefined && card.submergeTargetMinion !== true) {
      throw new RangeError(`${path}.submergeTargetMinion must be true when defined`);
    }
    if (card.teleportAllyToTargetSite !== undefined && card.teleportAllyToTargetSite !== true) {
      throw new RangeError(`${path}.teleportAllyToTargetSite must be true when defined`);
    }
    if (card.teleportNearbyAllyThenDrawCard !== undefined
      && card.teleportNearbyAllyThenDrawCard !== true) {
      throw new RangeError(`${path}.teleportNearbyAllyThenDrawCard must be true when defined`);
    }
    if (card.returnMinionFromOwnCemetery !== undefined
      && card.returnMinionFromOwnCemetery !== true) {
      throw new RangeError(`${path}.returnMinionFromOwnCemetery must be true when defined`);
    }
    if (card.returnTargetArtifactFromOwnCemetery !== undefined
      && card.returnTargetArtifactFromOwnCemetery !== true) {
      throw new RangeError(`${path}.returnTargetArtifactFromOwnCemetery must be true when defined`);
    }
    if (card.returnTargetAuraFromOwnCemetery !== undefined
      && card.returnTargetAuraFromOwnCemetery !== true) {
      throw new RangeError(`${path}.returnTargetAuraFromOwnCemetery must be true when defined`);
    }
    if (card.returnTargetMagicFromOwnCemetery !== undefined
      && card.returnTargetMagicFromOwnCemetery !== true) {
      throw new RangeError(`${path}.returnTargetMagicFromOwnCemetery must be true when defined`);
    }
    if (card.returnTargetMinionToOwnerHand !== undefined
      && card.returnTargetMinionToOwnerHand !== true) {
      throw new RangeError(`${path}.returnTargetMinionToOwnerHand must be true when defined`);
    }
    if (card.returnTargetArtifactToOwnerHand !== undefined
      && card.returnTargetArtifactToOwnerHand !== true) {
      throw new RangeError(`${path}.returnTargetArtifactToOwnerHand must be true when defined`);
    }
    if (card.returnTargetAuraToOwnerHand !== undefined
      && card.returnTargetAuraToOwnerHand !== true) {
      throw new RangeError(`${path}.returnTargetAuraToOwnerHand must be true when defined`);
    }
    if (card.returnTargetSiteFromOwnCemetery !== undefined
      && card.returnTargetSiteFromOwnCemetery !== true) {
      throw new RangeError(`${path}.returnTargetSiteFromOwnCemetery must be true when defined`);
    }
    if (card.returnTargetSiteToOwnerHand !== undefined
      && card.returnTargetSiteToOwnerHand !== true) {
      throw new RangeError(`${path}.returnTargetSiteToOwnerHand must be true when defined`);
    }
    if (card.summonRandomMinionFromAnyCemetery !== undefined
      && card.summonRandomMinionFromAnyCemetery !== true) {
      throw new RangeError(
        `${path}.summonRandomMinionFromAnyCemetery must be true when defined`,
      );
    }
    if (card.summonTokenToEachControlledSiteBorderingEnemySite !== undefined) {
      requireCardId(
        card.summonTokenToEachControlledSiteBorderingEnemySite,
        `${path}.summonTokenToEachControlledSiteBorderingEnemySite`,
      );
    }
    if (card.disableTargetNearbyMinionUntilNextTurn !== undefined
      && card.disableTargetNearbyMinionUntilNextTurn !== true) {
      throw new RangeError(`${path}.disableTargetNearbyMinionUntilNextTurn must be true when defined`);
    }
    if (card.grantAirborneToAllyThisTurn !== undefined
      && card.grantAirborneToAllyThisTurn !== true) {
      throw new RangeError(`${path}.grantAirborneToAllyThisTurn must be true when defined`);
    }
    if (card.grantChargeToAllyThisTurn !== undefined
      && card.grantChargeToAllyThisTurn !== true) {
      throw new RangeError(`${path}.grantChargeToAllyThisTurn must be true when defined`);
    }
    if (card.grantFirstStrikeToAllyThisTurn !== undefined
      && card.grantFirstStrikeToAllyThisTurn !== true) {
      throw new RangeError(`${path}.grantFirstStrikeToAllyThisTurn must be true when defined`);
    }
    if (card.grantLethalToAllyThisTurn !== undefined
      && card.grantLethalToAllyThisTurn !== true) {
      throw new RangeError(`${path}.grantLethalToAllyThisTurn must be true when defined`);
    }
    if (card.grantRangedToAllyThisTurn !== undefined
      && card.grantRangedToAllyThisTurn !== true) {
      throw new RangeError(`${path}.grantRangedToAllyThisTurn must be true when defined`);
    }
    if (card.grantPowerToAllyThisTurn !== undefined
      && card.grantPowerToAllyThisTurn !== 2) {
      throw new RangeError(`${path}.grantPowerToAllyThisTurn must be 2`);
    }
    if (card.leapAttackAlly !== undefined && card.leapAttackAlly !== true) {
      throw new RangeError(`${path}.leapAttackAlly must be true when defined`);
    }
    if (card.fightAllyWithAdjacentEnemy !== undefined
      && card.fightAllyWithAdjacentEnemy !== true) {
      throw new RangeError(`${path}.fightAllyWithAdjacentEnemy must be true when defined`);
    }
    if (card.gainControlOfTargetNearbyMinion !== undefined
      && card.gainControlOfTargetNearbyMinion !== true) {
      throw new RangeError(`${path}.gainControlOfTargetNearbyMinion must be true when defined`);
    }
    if (card.killTargetMinion !== undefined
      && card.killTargetMinion !== true) {
      throw new RangeError(`${path}.killTargetMinion must be true when defined`);
    }
    if (card.killTargetWoundedMinion !== undefined
      && card.killTargetWoundedMinion !== true) {
      throw new RangeError(`${path}.killTargetWoundedMinion must be true when defined`);
    }
    if (card.lureEnemyMinionOneStepCloser !== undefined
      && card.lureEnemyMinionOneStepCloser !== true) {
      throw new RangeError(`${path}.lureEnemyMinionOneStepCloser must be true when defined`);
    }
    if (card.grantStealthToTargetMinion !== undefined
      && card.grantStealthToTargetMinion !== true) {
      throw new RangeError(`${path}.grantStealthToTargetMinion must be true when defined`);
    }
    if (card.grantWardToTargetMinion !== undefined && card.grantWardToTargetMinion !== true) {
      throw new RangeError(`${path}.grantWardToTargetMinion must be true when defined`);
    }
    if (card.tapTargetMinion !== undefined && card.tapTargetMinion !== true) {
      throw new RangeError(`${path}.tapTargetMinion must be true when defined`);
    }
    if (card.untapTargetMinion !== undefined && card.untapTargetMinion !== true) {
      throw new RangeError(`${path}.untapTargetMinion must be true when defined`);
    }
    if (card.untapTargetMinionAfterDamage !== undefined
      && card.untapTargetMinionAfterDamage !== true) {
      throw new RangeError(`${path}.untapTargetMinionAfterDamage must be true when defined`);
    }
    if (card.damageEachAbovegroundMinion !== undefined
      && card.damageEachAbovegroundMinion !== 1) {
      throw new RangeError(`${path}.damageEachAbovegroundMinion must be 1`);
    }
    if (card.damageChainNearbyUnits !== undefined && card.damageChainNearbyUnits !== true) {
      throw new RangeError(`${path}.damageChainNearbyUnits must be true when defined`);
    }
    if (card.discardCardAsAdditionalCost !== undefined
      && card.discardCardAsAdditionalCost !== true) {
      throw new RangeError(`${path}.discardCardAsAdditionalCost must be true when defined`);
    }
    if (card.discardSiteAsAdditionalCost !== undefined
      && card.discardSiteAsAdditionalCost !== true) {
      throw new RangeError(`${path}.discardSiteAsAdditionalCost must be true when defined`);
    }
    if (card.discardCardAsAdditionalCost === true && card.discardSiteAsAdditionalCost === true) {
      throw new RangeError(`${path} competing additional discard costs are unsupported`);
    }
    if (card.destroyTargetArtifact !== undefined && card.destroyTargetArtifact !== true) {
      throw new RangeError(`${path}.destroyTargetArtifact must be true when defined`);
    }
    if (card.destroyTargetAura !== undefined && card.destroyTargetAura !== true) {
      throw new RangeError(`${path}.destroyTargetAura must be true when defined`);
    }
    if (card.destroyTargetSite !== undefined && card.destroyTargetSite !== true) {
      throw new RangeError(`${path}.destroyTargetSite must be true when defined`);
    }
    const targetSiteDamage = card.damageUnitsAboveAndBelowTargetSiteByManhattanDistance;
    if (targetSiteDamage !== undefined
      && (!Array.isArray(targetSiteDamage)
        || targetSiteDamage.length !== 5
        || targetSiteDamage.some((amount) => !Number.isSafeInteger(amount)
          || amount < 1
          || amount > MAX_COMBAT_STAT))) {
      throw new RangeError(
        `${path}.damageUnitsAboveAndBelowTargetSiteByManhattanDistance must contain five supported positive damage values`,
      );
    }
    const targetSiteEffectFacts = Number(card.discardSiteAsAdditionalCost === true)
      + Number(card.destroyTargetSite === true)
      + Number(targetSiteDamage !== undefined);
    const simpleDestroyTargetSite = card.destroyTargetSite === true
      && card.discardSiteAsAdditionalCost !== true
      && targetSiteDamage === undefined;
    if (targetSiteEffectFacts !== 0 && targetSiteEffectFacts !== 3 && !simpleDestroyTargetSite) {
      throw new RangeError(
        `${path} site-destruction grid damage facts must be defined together`,
      );
    }
    const effectCount = Number(card.burrowAllMinionsAndArtifactsAtTargetLandSite === true)
      + Number(card.burrowTargetMinionOrArtifact === true)
      + Number(card.submergeTargetMinion === true)
      + Number(card.damageChainNearbyUnits === true)
      + Number(card.damageEachAbovegroundMinion === 1)
      + Number(card.damageEachUnitAtLocationWithinTwoSteps !== undefined)
      + Number(card.damageRandomUnitAtLocation !== undefined)
      + Number(card.damageTargetUnit !== undefined)
      + Number(targetSiteEffectFacts === 3)
      + Number(simpleDestroyTargetSite)
      + Number(card.destroyTargetArtifact === true)
      + Number(card.destroyTargetAura === true)
      + Number(card.disableTargetNearbyMinionUntilNextTurn === true)
      + Number(card.fightAllyWithAdjacentEnemy === true)
      + Number(card.gainControlOfTargetNearbyMinion === true)
      + Number(card.grantAirborneToAllyThisTurn === true)
      + Number(card.grantChargeToAllyThisTurn === true)
      + Number(card.grantFirstStrikeToAllyThisTurn === true)
      + Number(card.grantLethalToAllyThisTurn === true)
      + Number(card.grantPowerToAllyThisTurn === 2)
      + Number(card.grantRangedToAllyThisTurn === true)
      + Number(card.grantStealthToTargetMinion === true)
      + Number(card.grantWardToTargetMinion === true)
      + Number(card.healController !== undefined)
      + Number(card.healTargetMinion !== undefined)
      + Number(card.drawSites !== undefined)
      + Number(card.drawSpells !== undefined)
      + Number(card.killTargetMinion === true)
      + Number(card.killTargetWoundedMinion === true)
      + Number(card.leapAttackAlly === true)
      + Number(card.lureEnemyMinionOneStepCloser === true)
      + Number(card.millSites !== undefined)
      + Number(card.millSpells !== undefined)
      + Number(card.targetPlayerDrawsSites !== undefined)
      + Number(card.targetPlayerDrawsSpells !== undefined)
      + Number(card.returnMinionFromOwnCemetery === true)
      + Number(card.returnTargetArtifactFromOwnCemetery === true)
      + Number(card.returnTargetAuraFromOwnCemetery === true)
      + Number(card.returnTargetMagicFromOwnCemetery === true)
      + Number(card.returnTargetArtifactToOwnerHand === true)
      + Number(card.returnTargetAuraToOwnerHand === true)
      + Number(card.returnTargetMinionToOwnerHand === true)
      + Number(card.returnTargetSiteFromOwnCemetery === true)
      + Number(card.returnTargetSiteToOwnerHand === true)
      + Number(card.summonRandomMinionFromAnyCemetery === true)
      + Number(card.summonTokenToEachControlledSiteBorderingEnemySite !== undefined)
      + Number(card.targetPlayerDiscardsCards !== undefined)
      + Number(card.targetPlayerGainsLife !== undefined)
      + Number(card.targetPlayerLosesLife !== undefined)
      + Number(card.teleportAllyToTargetSite === true)
      + Number(card.tapTargetMinion === true)
      + Number(card.teleportNearbyAllyThenDrawCard === true)
      + Number(card.untapTargetMinion === true);
    if (effectCount !== 1) {
      throw new RangeError(`${path} must define exactly one supported Magic effect`);
    }
    if (card.targetNearby !== undefined && typeof card.targetNearby !== 'boolean') {
      throw new RangeError(`${path}.targetNearby must be boolean`);
    }
    if (card.targetNearby !== undefined && card.damageTargetUnit === undefined) {
      throw new RangeError(`${path}.targetNearby requires damageTargetUnit`);
    }
    if (card.untapTargetMinionAfterDamage === true && card.damageTargetUnit === undefined) {
      throw new RangeError(`${path}.untapTargetMinionAfterDamage requires damageTargetUnit`);
    }
    if (card.damageTargetUnit !== undefined && (!Number.isSafeInteger(card.damageTargetUnit)
      || card.damageTargetUnit < 1
      || card.damageTargetUnit > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.damageTargetUnit must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
    }
    if (card.damageRandomUnitAtLocation !== undefined
      && (!Number.isSafeInteger(card.damageRandomUnitAtLocation)
        || card.damageRandomUnitAtLocation < 1
        || card.damageRandomUnitAtLocation > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.damageRandomUnitAtLocation must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
    }
    if (card.damageEachUnitAtLocationWithinTwoSteps !== undefined
      && (!Number.isSafeInteger(card.damageEachUnitAtLocationWithinTwoSteps)
        || card.damageEachUnitAtLocationWithinTwoSteps < 1
        || card.damageEachUnitAtLocationWithinTwoSteps > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.damageEachUnitAtLocationWithinTwoSteps must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
    }
    if (card.healController !== undefined && (!Number.isSafeInteger(card.healController)
      || card.healController < 1
      || card.healController > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.healController must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
    }
    if (card.healTargetMinion !== undefined && (!Number.isSafeInteger(card.healTargetMinion)
      || card.healTargetMinion < 1
      || card.healTargetMinion > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.healTargetMinion must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
    }
    if (card.targetPlayerLosesLife !== undefined && (!Number.isSafeInteger(card.targetPlayerLosesLife)
      || card.targetPlayerLosesLife < 1
      || card.targetPlayerLosesLife > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.targetPlayerLosesLife must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
    }
    if (card.targetPlayerGainsLife !== undefined && (!Number.isSafeInteger(card.targetPlayerGainsLife)
      || card.targetPlayerGainsLife < 1
      || card.targetPlayerGainsLife > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.targetPlayerGainsLife must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
    }
    if (card.drawSites !== undefined && (!Number.isSafeInteger(card.drawSites)
      || card.drawSites < 1
      || card.drawSites > MAX_DECK_CARDS)) {
      throw new RangeError(`${path}.drawSites must be a safe integer between 1 and ${MAX_DECK_CARDS}`);
    }
    if (card.drawSpells !== undefined && (!Number.isSafeInteger(card.drawSpells)
      || card.drawSpells < 1
      || card.drawSpells > MAX_DECK_CARDS)) {
      throw new RangeError(`${path}.drawSpells must be a safe integer between 1 and ${MAX_DECK_CARDS}`);
    }
    if (card.millSites !== undefined && (!Number.isSafeInteger(card.millSites)
      || card.millSites < 1
      || card.millSites > MAX_DECK_CARDS)) {
      throw new RangeError(`${path}.millSites must be a safe integer between 1 and ${MAX_DECK_CARDS}`);
    }
    if (card.millSpells !== undefined && (!Number.isSafeInteger(card.millSpells)
      || card.millSpells < 1
      || card.millSpells > MAX_DECK_CARDS)) {
      throw new RangeError(`${path}.millSpells must be a safe integer between 1 and ${MAX_DECK_CARDS}`);
    }
    if (card.targetPlayerDrawsSites !== undefined
      && (!Number.isSafeInteger(card.targetPlayerDrawsSites)
        || card.targetPlayerDrawsSites < 1
        || card.targetPlayerDrawsSites > MAX_DECK_CARDS)) {
      throw new RangeError(
        `${path}.targetPlayerDrawsSites must be a safe integer between 1 and ${MAX_DECK_CARDS}`,
      );
    }
    if (card.targetPlayerDrawsSpells !== undefined
      && (!Number.isSafeInteger(card.targetPlayerDrawsSpells)
        || card.targetPlayerDrawsSpells < 1
        || card.targetPlayerDrawsSpells > MAX_DECK_CARDS)) {
      throw new RangeError(
        `${path}.targetPlayerDrawsSpells must be a safe integer between 1 and ${MAX_DECK_CARDS}`,
      );
    }
    if (card.targetPlayerDiscardsCards !== undefined
      && (!Number.isSafeInteger(card.targetPlayerDiscardsCards)
        || card.targetPlayerDiscardsCards < 1
        || card.targetPlayerDiscardsCards > MAX_DECK_CARDS)) {
      throw new RangeError(
        `${path}.targetPlayerDiscardsCards must be a safe integer between 1 and ${MAX_DECK_CARDS}`,
      );
    }
    if (!Number.isSafeInteger(card.manaCost) || card.manaCost < 0) {
      throw new RangeError(`${path}.manaCost must be a supported nonnegative safe integer`);
    }
    rejectUnknownThresholds(card.thresholds, path, elements);
    for (const element of elements) {
      if (!Number.isSafeInteger(card.thresholds[element]) || card.thresholds[element] < 0) {
        throw new RangeError(`${path}.thresholds.${element} must be a nonnegative safe integer`);
      }
    }
    return;
  }
  if (card.cardType !== 'minion') throw new RangeError(`${path}.cardType is unsupported`);
  if (card.airborne !== undefined && typeof card.airborne !== 'boolean') {
    throw new RangeError(`${path}.airborne must be boolean`);
  }
  if (card.charge !== undefined && typeof card.charge !== 'boolean') {
    throw new RangeError(`${path}.charge must be boolean`);
  }
  if (card.cannotAttackSites !== undefined && typeof card.cannotAttackSites !== 'boolean') {
    throw new RangeError(`${path}.cannotAttackSites must be boolean`);
  }
  if (card.cannotDefend !== undefined && typeof card.cannotDefend !== 'boolean') {
    throw new RangeError(`${path}.cannotDefend must be boolean`);
  }
  if (card.cannotDefendOrIntercept !== undefined && typeof card.cannotDefendOrIntercept !== 'boolean') {
    throw new RangeError(`${path}.cannotDefendOrIntercept must be boolean`);
  }
  if (card.connectsTopBottom !== undefined && typeof card.connectsTopBottom !== 'boolean') {
    throw new RangeError(`${path}.connectsTopBottom must be boolean`);
  }
  if (card.deathriteDrawSite !== undefined && typeof card.deathriteDrawSite !== 'boolean') {
    throw new RangeError(`${path}.deathriteDrawSite must be boolean`);
  }
  if (card.deathriteDrawSpells !== undefined && typeof card.deathriteDrawSpells !== 'boolean') {
    throw new RangeError(`${path}.deathriteDrawSpells must be boolean`);
  }
  if (card.deathriteMillSites !== undefined && typeof card.deathriteMillSites !== 'boolean') {
    throw new RangeError(`${path}.deathriteMillSites must be boolean`);
  }
  if (card.deathriteMillSpells !== undefined && typeof card.deathriteMillSpells !== 'boolean') {
    throw new RangeError(`${path}.deathriteMillSpells must be boolean`);
  }
  if (card.deathriteDamageEachUnitHere !== undefined
    && (!Number.isSafeInteger(card.deathriteDamageEachUnitHere)
      || card.deathriteDamageEachUnitHere < 1
      || card.deathriteDamageEachUnitHere > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.deathriteDamageEachUnitHere must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.deathriteHeal !== undefined
    && (!Number.isSafeInteger(card.deathriteHeal)
      || card.deathriteHeal < 1
      || card.deathriteHeal > MAX_COMBAT_STAT)) {
    throw new RangeError(`${path}.deathriteHeal must be a safe integer between 1 and ${MAX_COMBAT_STAT}`);
  }
  if (card.deathriteLoseLifePerNearbySiteControlled !== undefined
    && card.deathriteLoseLifePerNearbySiteControlled !== 1) {
    throw new RangeError(`${path}.deathriteLoseLifePerNearbySiteControlled must be 1`);
  }
  if (card.discardSpellToDamageRandomOtherUnitHere !== undefined
    && (!Number.isSafeInteger(card.discardSpellToDamageRandomOtherUnitHere)
      || card.discardSpellToDamageRandomOtherUnitHere < 1
      || card.discardSpellToDamageRandomOtherUnitHere > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.discardSpellToDamageRandomOtherUnitHere must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.discardRandomCardInsteadOfMana !== undefined
    && card.discardRandomCardInsteadOfMana !== true) {
    throw new RangeError(`${path}.discardRandomCardInsteadOfMana must be true when defined`);
  }
  if (card.ordinary !== undefined && card.ordinary !== true) {
    throw new RangeError(`${path}.ordinary must be true when defined`);
  }
  if (card.sacrificeMinionAtSummoningLocationForManaDiscount !== undefined
    && card.sacrificeMinionAtSummoningLocationForManaDiscount !== 2) {
    throw new RangeError(
      `${path}.sacrificeMinionAtSummoningLocationForManaDiscount must be 2`,
    );
  }
  if (card.discardRandomCardInsteadOfMana === true
    && card.sacrificeMinionAtSummoningLocationForManaDiscount === 2) {
    throw new RangeError(`${path} competing alternative summon payments are unsupported`);
  }
  if (card.lethal !== undefined && typeof card.lethal !== 'boolean') {
    throw new RangeError(`${path}.lethal must be boolean`);
  }
  if (card.burrowing !== undefined && typeof card.burrowing !== 'boolean') {
    throw new RangeError(`${path}.burrowing must be boolean`);
  }
  if (card.genesisDrawSite !== undefined && typeof card.genesisDrawSite !== 'boolean') {
    throw new RangeError(`${path}.genesisDrawSite must be boolean`);
  }
  if (card.genesisDrawSpells !== undefined
    && (!Number.isSafeInteger(card.genesisDrawSpells)
      || card.genesisDrawSpells < 1
      || card.genesisDrawSpells > MAX_DECK_CARDS)) {
    throw new RangeError(
      `${path}.genesisDrawSpells must be a safe integer between 1 and ${MAX_DECK_CARDS}`,
    );
  }
  if (card.genesisDamageEachOtherUnitHere !== undefined
    && card.genesisDamageEachOtherUnitHere !== 1) {
    throw new RangeError(`${path}.genesisDamageEachOtherUnitHere must be 1`);
  }
  if (card.genesisDisableSelfUntilDamaged !== undefined
    && card.genesisDisableSelfUntilDamaged !== true) {
    throw new RangeError(`${path}.genesisDisableSelfUntilDamaged must be true when defined`);
  }
  if (card.genesisStrikeEachEnemyHere !== undefined
    && card.genesisStrikeEachEnemyHere !== true) {
    throw new RangeError(`${path}.genesisStrikeEachEnemyHere must be true when defined`);
  }
  if (card.genesisMayDamageTargetAdjacentUnit !== undefined
    && card.genesisMayDamageTargetAdjacentUnit !== 2) {
    throw new RangeError(`${path}.genesisMayDamageTargetAdjacentUnit must be 2`);
  }
  if (card.genesisHealController !== undefined && card.genesisHealController !== 2) {
    throw new RangeError(`${path}.genesisHealController must be 2`);
  }
  if (card.genesisLoseControllerLife !== undefined && card.genesisLoseControllerLife !== 2) {
    throw new RangeError(`${path}.genesisLoseControllerLife must be 2`);
  }
  if (card.gainsPowerRangedAndSpellcasterAtopTower !== undefined
    && card.gainsPowerRangedAndSpellcasterAtopTower !== 2) {
    throw new RangeError(`${path}.gainsPowerRangedAndSpellcasterAtopTower must be 2`);
  }
  if (card.diesAtEndOfControllerTurn !== undefined
    && card.diesAtEndOfControllerTurn !== true) {
    throw new RangeError(`${path}.diesAtEndOfControllerTurn must be true when defined`);
  }
  if (card.genesisDrawSite && card.genesisDrawSpells !== undefined) {
    throw new RangeError(`${path} simultaneous Genesis site and spell draws are unsupported`);
  }
  if (card.genesisLoseControllerLife !== undefined
    && (card.genesisDrawSite || card.genesisDrawSpells !== undefined)) {
    throw new RangeError(`${path} simultaneous Genesis life loss and draw are unsupported`);
  }
  if (card.genesisHealController !== undefined
    && (card.genesisDrawSite || card.genesisDrawSpells !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} simultaneous Genesis healing and another effect are unsupported`);
  }
  if (card.genesisDamageEachOtherUnitHere === 1
    && (card.genesisDrawSite || card.genesisDrawSpells !== undefined
      || card.genesisMayDamageTargetAdjacentUnit !== undefined
      || card.genesisStrikeEachEnemyHere === true
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} simultaneous Genesis damage and another effect are unsupported`);
  }
  if (card.genesisMayDamageTargetAdjacentUnit === 2
    && (card.genesisDrawSite || card.genesisDrawSpells !== undefined
      || card.genesisStrikeEachEnemyHere === true
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} simultaneous Genesis damage and another effect are unsupported`);
  }
  if (card.genesisStrikeEachEnemyHere === true
    && (card.genesisDrawSite || card.genesisDrawSpells !== undefined
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} simultaneous Genesis strikes and another effect are unsupported`);
  }
  if (card.genesisDisableSelfUntilDamaged === true
    && (card.genesisDrawSite || card.genesisDrawSpells !== undefined
      || card.genesisDamageEachOtherUnitHere === 1
      || card.genesisMayDamageTargetAdjacentUnit === 2
      || card.genesisStrikeEachEnemyHere === true
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} Genesis disable with another effect is unsupported`);
  }
  if (card.genesisDisableSelfUntilDamaged === true && card.stealth === true) {
    throw new RangeError(`${path} Genesis disable with Stealth is unsupported`);
  }
  if (card.genesisMayDamageTargetAdjacentUnit === 2
    && (card.discardRandomCardInsteadOfMana === true
      || card.sacrificeMinionAtSummoningLocationForManaDiscount === 2)) {
    throw new RangeError(`${path} targeted Genesis with alternative summon payment is unsupported`);
  }
  if (card.gainsStealthAtEndOfTurn !== undefined && typeof card.gainsStealthAtEndOfTurn !== 'boolean') {
    throw new RangeError(`${path}.gainsStealthAtEndOfTurn must be boolean`);
  }
  if (card.gainsStealthAtEndOfTurnIfNoEnemiesNearby !== undefined
    && typeof card.gainsStealthAtEndOfTurnIfNoEnemiesNearby !== 'boolean') {
    throw new RangeError(
      `${path}.gainsStealthAtEndOfTurnIfNoEnemiesNearby must be boolean`,
    );
  }
  if (card.gainsStealthAtEndOfTurn === true
    && card.gainsStealthAtEndOfTurnIfNoEnemiesNearby === true) {
    throw new RangeError(
      `${path} simultaneous unconditional and conditional end-turn Stealth are unsupported`,
    );
  }
  if (card.immobile !== undefined && typeof card.immobile !== 'boolean') {
    throw new RangeError(`${path}.immobile must be boolean`);
  }
  if (card.lanceCount !== undefined
    && (!Number.isSafeInteger(card.lanceCount) || card.lanceCount < 1 || card.lanceCount > 3)) {
    throw new RangeError(`${path}.lanceCount must be a safe integer between 1 and 3`);
  }
  if (card.mayStepAfterRangedStrike !== undefined
    && card.mayStepAfterRangedStrike !== true) {
    throw new RangeError(`${path}.mayStepAfterRangedStrike must be true when defined`);
  }
  if (card.mayRangedStrikeOnceDuringBasicMovement !== undefined
    && card.mayRangedStrikeOnceDuringBasicMovement !== true) {
    throw new RangeError(
      `${path}.mayRangedStrikeOnceDuringBasicMovement must be true when defined`,
    );
  }
  if (card.mayRangedStrikeOnceDuringBasicMovement === true && card.ranged !== true) {
    throw new RangeError(`${path}.mayRangedStrikeOnceDuringBasicMovement requires ranged`);
  }
  if (card.mayRangedStrikeOnceDuringBasicMovement === true
    && card.mayStepAfterRangedStrike === true) {
    throw new RangeError(`${path} simultaneous during-movement and post-Ranged movement is unsupported`);
  }
  if (card.atStartOfControllerTurnDrawSites !== undefined
    && (!Number.isSafeInteger(card.atStartOfControllerTurnDrawSites)
      || card.atStartOfControllerTurnDrawSites < 1
      || card.atStartOfControllerTurnDrawSites > MAX_DECK_CARDS)) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnDrawSites must be a safe integer between 1 and ${MAX_DECK_CARDS}`,
    );
  }
  if (card.atStartOfControllerTurnDrawSpells !== undefined
    && (!Number.isSafeInteger(card.atStartOfControllerTurnDrawSpells)
      || card.atStartOfControllerTurnDrawSpells < 1
      || card.atStartOfControllerTurnDrawSpells > MAX_DECK_CARDS)) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnDrawSpells must be a safe integer between 1 and ${MAX_DECK_CARDS}`,
    );
  }
  if (card.atStartOfControllerTurnMillSites !== undefined
    && (!Number.isSafeInteger(card.atStartOfControllerTurnMillSites)
      || card.atStartOfControllerTurnMillSites < 1
      || card.atStartOfControllerTurnMillSites > MAX_DECK_CARDS)) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnMillSites must be a safe integer between 1 and ${MAX_DECK_CARDS}`,
    );
  }
  if (card.atStartOfControllerTurnMillSpells !== undefined
    && (!Number.isSafeInteger(card.atStartOfControllerTurnMillSpells)
      || card.atStartOfControllerTurnMillSpells < 1
      || card.atStartOfControllerTurnMillSpells > MAX_DECK_CARDS)) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnMillSpells must be a safe integer between 1 and ${MAX_DECK_CARDS}`,
    );
  }
  if (card.atStartOfControllerTurnControllerGainsLife !== undefined
    && (!Number.isSafeInteger(card.atStartOfControllerTurnControllerGainsLife)
      || card.atStartOfControllerTurnControllerGainsLife < 1
      || card.atStartOfControllerTurnControllerGainsLife > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnControllerGainsLife must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.atStartOfControllerTurnControllerGainsMana !== undefined
    && (!Number.isSafeInteger(card.atStartOfControllerTurnControllerGainsMana)
      || card.atStartOfControllerTurnControllerGainsMana < 1
      || card.atStartOfControllerTurnControllerGainsMana > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnControllerGainsMana must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.atStartOfControllerTurnControllerLosesLife !== undefined
    && (!Number.isSafeInteger(card.atStartOfControllerTurnControllerLosesLife)
      || card.atStartOfControllerTurnControllerLosesLife < 1
      || card.atStartOfControllerTurnControllerLosesLife > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnControllerLosesLife must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.atEndOfControllerTurnDamageEachOtherUnitHere !== undefined
    && (!Number.isSafeInteger(card.atEndOfControllerTurnDamageEachOtherUnitHere)
      || card.atEndOfControllerTurnDamageEachOtherUnitHere < 1
      || card.atEndOfControllerTurnDamageEachOtherUnitHere > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.atEndOfControllerTurnDamageEachOtherUnitHere must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.atEndOfControllerTurnControllerGainsLife !== undefined
    && (!Number.isSafeInteger(card.atEndOfControllerTurnControllerGainsLife)
      || card.atEndOfControllerTurnControllerGainsLife < 1
      || card.atEndOfControllerTurnControllerGainsLife > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.atEndOfControllerTurnControllerGainsLife must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.atEndOfControllerTurnControllerLosesLife !== undefined
    && (!Number.isSafeInteger(card.atEndOfControllerTurnControllerLosesLife)
      || card.atEndOfControllerTurnControllerLosesLife < 1
      || card.atEndOfControllerTurnControllerLosesLife > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.atEndOfControllerTurnControllerLosesLife must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  const endTurnPulseCount = Number(card.atEndOfControllerTurnDamageEachOtherUnitHere !== undefined)
    + Number(card.atEndOfControllerTurnControllerGainsLife !== undefined)
    + Number(card.atEndOfControllerTurnControllerLosesLife !== undefined);
  if (endTurnPulseCount > 1) {
    throw new RangeError(`${path} competing end-turn pulses are unsupported`);
  }
  if (card.atStartOfControllerTurnDamageEachOtherUnitHere !== undefined
    && (!Number.isSafeInteger(card.atStartOfControllerTurnDamageEachOtherUnitHere)
      || card.atStartOfControllerTurnDamageEachOtherUnitHere < 1
      || card.atStartOfControllerTurnDamageEachOtherUnitHere > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnDamageEachOtherUnitHere must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.atStartOfControllerTurnLureNearbyEnemyMinion !== undefined
    && card.atStartOfControllerTurnLureNearbyEnemyMinion !== true) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnLureNearbyEnemyMinion must be true when defined`,
    );
  }
  if (card.atStartOfControllerTurnTeleportToRandomSiteOrVoid !== undefined
    && card.atStartOfControllerTurnTeleportToRandomSiteOrVoid !== true) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnTeleportToRandomSiteOrVoid must be true when defined`,
    );
  }
  if (card.atStartOfControllerTurnTeleportToRandomSiteOrVoid === true
    && card.voidwalk !== true) {
    throw new RangeError(
      `${path}.atStartOfControllerTurnTeleportToRandomSiteOrVoid requires voidwalk`,
    );
  }
  if (card.atStartOfControllerTurnTeleportToRandomSiteOrVoid === true
    && card.occupiesSquareArea === 2) {
    throw new RangeError(
      `${path} oversized start-turn random teleport is unsupported`,
    );
  }
  const startTurnTriggerCount = [
    card.atStartOfControllerTurnControllerGainsLife !== undefined,
    card.atStartOfControllerTurnControllerGainsMana !== undefined,
    card.atStartOfControllerTurnControllerLosesLife !== undefined,
    card.atStartOfControllerTurnDamageEachOtherUnitHere !== undefined,
    card.atStartOfControllerTurnDrawSites !== undefined,
    card.atStartOfControllerTurnDrawSpells !== undefined,
    card.atStartOfControllerTurnMillSites !== undefined,
    card.atStartOfControllerTurnMillSpells !== undefined,
    card.atStartOfControllerTurnLureNearbyEnemyMinion === true,
    card.atStartOfControllerTurnTeleportToRandomSiteOrVoid === true,
  ].filter(Boolean).length;
  if (startTurnTriggerCount > 1) {
    throw new RangeError(`${path} competing start-turn triggers are unsupported`);
  }
  if (card.movementBonus !== undefined
    && (!Number.isSafeInteger(card.movementBonus)
      || card.movementBonus < 1
      || card.movementBonus > 2)) {
    throw new RangeError(`${path}.movementBonus must be a safe integer between 1 and 2`);
  }
  if (card.mortal !== undefined && card.mortal !== true) {
    throw new RangeError(`${path}.mortal must be true when defined`);
  }
  if (card.nearbyEnemiesPermanentlyLoseStealth !== undefined
    && card.nearbyEnemiesPermanentlyLoseStealth !== true) {
    throw new RangeError(`${path}.nearbyEnemiesPermanentlyLoseStealth must be true when defined`);
  }
  if (card.otherNearbyAlliesPowerBonus !== undefined
    && card.otherNearbyAlliesPowerBonus !== 1) {
    throw new RangeError(`${path}.otherNearbyAlliesPowerBonus must be 1`);
  }
  if (card.otherControlledMortalsPowerBonus !== undefined
    && card.otherControlledMortalsPowerBonus !== 1) {
    throw new RangeError(`${path}.otherControlledMortalsPowerBonus must be 1`);
  }
  if (card.preventsDamageFromUnitsWithPowerAtLeast !== undefined
    && (!Number.isSafeInteger(card.preventsDamageFromUnitsWithPowerAtLeast)
      || card.preventsDamageFromUnitsWithPowerAtLeast < 1
      || card.preventsDamageFromUnitsWithPowerAtLeast > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.preventsDamageFromUnitsWithPowerAtLeast must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.occupiesSquareArea !== undefined && card.occupiesSquareArea !== 2) {
    throw new RangeError(`${path}.occupiesSquareArea must be 2`);
  }
  if (card.occupiesSquareArea === 2
    && (card.connectsTopBottom === true
      || card.voidwalk === true
      || card.mustBeCastToOuterColumn === true
      || card.token === true)) {
    throw new RangeError(
      `${path}.occupiesSquareArea has an unsupported ability combination`,
    );
  }
  if (card.movesOnlySideways !== undefined && typeof card.movesOnlySideways !== 'boolean') {
    throw new RangeError(`${path}.movesOnlySideways must be boolean`);
  }
  if (card.movesOnlyForward !== undefined && typeof card.movesOnlyForward !== 'boolean') {
    throw new RangeError(`${path}.movesOnlyForward must be boolean`);
  }
  if (card.movesOnlyForward && card.movesOnlySideways) {
    throw new RangeError(`${path} cannot move only forward and only sideways`);
  }
  if (card.mustBeCastBurrowed !== undefined && typeof card.mustBeCastBurrowed !== 'boolean') {
    throw new RangeError(`${path}.mustBeCastBurrowed must be boolean`);
  }
  if (card.mustBeCastBurrowed && !card.burrowing) {
    throw new RangeError(`${path}.mustBeCastBurrowed requires Burrowing`);
  }
  if (card.mustBeCastSubmerged !== undefined && typeof card.mustBeCastSubmerged !== 'boolean') {
    throw new RangeError(`${path}.mustBeCastSubmerged must be boolean`);
  }
  if (card.mustBeCastSubmerged && !card.submerge) {
    throw new RangeError(`${path}.mustBeCastSubmerged requires Submerge`);
  }
  if (card.mustBeCastBurrowed && card.mustBeCastSubmerged) {
    throw new RangeError(`${path} cannot require both burrowed and submerged casting`);
  }
  if (card.mustAttackAUnitIfAble !== undefined && card.mustAttackAUnitIfAble !== true) {
    throw new RangeError(`${path}.mustAttackAUnitIfAble must be true when defined`);
  }
  if (card.enemiesMustAttackThisIfAble !== undefined && card.enemiesMustAttackThisIfAble !== true) {
    throw new RangeError(`${path}.enemiesMustAttackThisIfAble must be true when defined`);
  }
  if (card.mustBeCastToWaterSite !== undefined && typeof card.mustBeCastToWaterSite !== 'boolean') {
    throw new RangeError(`${path}.mustBeCastToWaterSite must be boolean`);
  }
  if (card.ranged !== undefined && typeof card.ranged !== 'boolean') {
    throw new RangeError(`${path}.ranged must be boolean`);
  }
  if (card.tapToShootProjectileDamage !== undefined
    && (!Number.isSafeInteger(card.tapToShootProjectileDamage)
      || card.tapToShootProjectileDamage < 1
      || card.tapToShootProjectileDamage > MAX_COMBAT_STAT)) {
    throw new RangeError(
      `${path}.tapToShootProjectileDamage must be a safe integer between 1 and ${MAX_COMBAT_STAT}`,
    );
  }
  if (card.shootsDragProjectile !== undefined && typeof card.shootsDragProjectile !== 'boolean') {
    throw new RangeError(`${path}.shootsDragProjectile must be boolean`);
  }
  if (card.siteProvidesNoThreshold !== undefined && card.siteProvidesNoThreshold !== true) {
    throw new RangeError(`${path}.siteProvidesNoThreshold must be true when defined`);
  }
  if (card.spellcaster !== undefined && typeof card.spellcaster !== 'boolean') {
    throw new RangeError(`${path}.spellcaster must be boolean`);
  }
  if (card.stealth !== undefined && typeof card.stealth !== 'boolean') {
    throw new RangeError(`${path}.stealth must be boolean`);
  }
  if (card.strikesFirstWhileAttacking !== undefined && typeof card.strikesFirstWhileAttacking !== 'boolean') {
    throw new RangeError(`${path}.strikesFirstWhileAttacking must be boolean`);
  }
  if (card.strikesFirstWhileDefending !== undefined && typeof card.strikesFirstWhileDefending !== 'boolean') {
    throw new RangeError(`${path}.strikesFirstWhileDefending must be boolean`);
  }
  if (card.submerge !== undefined && typeof card.submerge !== 'boolean') {
    throw new RangeError(`${path}.submerge must be boolean`);
  }
  if (card.takesLessDamage !== undefined && card.takesLessDamage !== 1) {
    throw new RangeError(`${path}.takesLessDamage must be 1`);
  }
  if (card.token !== undefined && card.token !== true) {
    throw new RangeError(`${path}.token must be true when defined`);
  }
  if (card.token === true
    && (card.genesisDrawSite || card.genesisDrawSpells !== undefined
      || card.genesisDamageEachOtherUnitHere === 1
      || card.genesisDisableSelfUntilDamaged === true
      || card.genesisMayDamageTargetAdjacentUnit === 2
      || card.genesisStrikeEachEnemyHere === true
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} token Genesis effects are unsupported`);
  }
  if (card.ward !== undefined && typeof card.ward !== 'boolean') {
    throw new RangeError(`${path}.ward must be boolean`);
  }
  if (Number(card.takesLessDamage === 1)
      + Number(card.ward === true)
      + Number(card.preventsDamageFromUnitsWithPowerAtLeast !== undefined) > 1) {
    throw new RangeError(`${path} competing damage prevention effects are unsupported`);
  }
  if (card.voidwalk !== undefined && typeof card.voidwalk !== 'boolean') {
    throw new RangeError(`${path}.voidwalk must be boolean`);
  }
  if (card.waterbound !== undefined && typeof card.waterbound !== 'boolean') {
    throw new RangeError(`${path}.waterbound must be boolean`);
  }
  if (card.summonToAnySite !== undefined && typeof card.summonToAnySite !== 'boolean') {
    throw new RangeError(`${path}.summonToAnySite must be boolean`);
  }
  if (card.mustBeCastToOuterColumn !== undefined
    && typeof card.mustBeCastToOuterColumn !== 'boolean') {
    throw new RangeError(`${path}.mustBeCastToOuterColumn must be boolean`);
  }
  if (card.tapForMana !== undefined
    && (!Number.isSafeInteger(card.tapForMana) || card.tapForMana < 1 || card.tapForMana > MAX_COMBAT_STAT)) {
    throw new RangeError(path + '.tapForMana must be a safe integer between 1 and ' + MAX_COMBAT_STAT);
  }
  if (card.doesNotUntapDuringControllersStartPhase !== undefined
    && card.doesNotUntapDuringControllersStartPhase !== true) {
    throw new RangeError(
      `${path}.doesNotUntapDuringControllersStartPhase must be true when defined`,
    );
  }
  if (card.untapsAtEndOfControllerTurn !== undefined
    && card.untapsAtEndOfControllerTurn !== true) {
    throw new RangeError(`${path}.untapsAtEndOfControllerTurn must be true when defined`);
  }
  if (card.tapToDamageEachUnitAtAdjacentLocation !== undefined
    && card.tapToDamageEachUnitAtAdjacentLocation !== 2) {
    throw new RangeError(`${path}.tapToDamageEachUnitAtAdjacentLocation must be 2`);
  }
  if (card.provides !== undefined && !elements.includes(card.provides)) {
    throw new RangeError(`${path}.provides must be a supported element`);
  }
  for (const field of ['attack', 'defense', 'manaCost'] as const) {
    if (!Number.isSafeInteger(card[field])
      || card[field] < 0
      || ((field === 'attack' || field === 'defense') && card[field] > MAX_COMBAT_STAT)) {
      throw new RangeError(`${path}.${field} must be a supported nonnegative safe integer`);
    }
  }
  rejectUnknownThresholds(card.thresholds, path, elements);
  for (const element of elements) {
    if (!Number.isSafeInteger(card.thresholds[element]) || card.thresholds[element] < 0) {
      throw new RangeError(`${path}.thresholds.${element} must be a nonnegative safe integer`);
    }
  }
}

function validateDeck(
  deck: GameDeckSpec,
  path: string,
  cards: Readonly<Record<string, GameCardDefinition>>,
): void {
  requireCardId(deck.avatar, `${path}.avatar`);
  for (const zone of ['atlas', 'spellbook'] as const) {
    if (deck[zone].length < 3 || deck[zone].length > MAX_DECK_CARDS) {
      throw new RangeError(`${path}.${zone} must contain 3-${MAX_DECK_CARDS} cards`);
    }
    deck[zone].forEach((cardId, index) => requireCardId(cardId, `${path}.${zone}[${index}]`));
  }
  if (cards[deck.avatar]?.cardType !== 'avatar') throw new RangeError(`${path}.avatar must reference an avatar`);
  deck.atlas.forEach((cardId, index) => {
    if (cards[cardId]?.cardType !== 'site') throw new RangeError(`${path}.atlas[${index}] must reference a site`);
  });
  deck.spellbook.forEach((cardId, index) => {
    const definition = cards[cardId];
    if ((definition?.cardType !== 'artifact'
      && definition?.cardType !== 'aura'
      && definition?.cardType !== 'minion'
      && definition?.cardType !== 'magic')
      || definition?.cardType === 'minion' && definition.token === true) {
      throw new RangeError(`${path}.spellbook[${index}] references an unsupported spell`);
    }
  });
}

export function createGameManifest(input: GameManifestInput): GameManifest {
  createEngineState(input.seed);
  if (input.firstSeat !== 'north' && input.firstSeat !== 'south') {
    throw new RangeError('firstSeat is unsupported');
  }
  if (input.authority.mode !== 'private-local' && input.authority.mode !== 'synthetic') {
    throw new RangeError('authority.mode is unsupported');
  }
  const cardEntries = Object.entries(input.cards).sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0);
  if (cardEntries.length === 0 || cardEntries.length > 5_000) {
    throw new RangeError('cards must contain 1-5000 definitions');
  }
  cardEntries.forEach(([cardId, card]) => {
    requireCardId(cardId, 'cards key');
    validateCardDefinition(card, `cards.${cardId}`);
  });
  const referencedCardIds = new Set((['north', 'south'] as const).flatMap((seat) => {
    const deck = input.decks[seat];
    return [deck.avatar, ...deck.atlas, ...deck.spellbook];
  }));
  for (const cardId of referencedCardIds) {
    const definition = input.cards[cardId];
    const tokenCardId = definition?.cardType === 'magic'
      ? definition.summonTokenToEachControlledSiteBorderingEnemySite
      : definition?.cardType === 'site'
        ? definition.genesisPayOneManaToSummonToken
        : undefined;
    if (tokenCardId === undefined) continue;
    const token = input.cards[tokenCardId];
    if (token?.cardType !== 'minion' || token.token !== true) {
      throw new RangeError(`cards.${cardId} token effect must reference a token minion`);
    }
    referencedCardIds.add(tokenCardId);
  }
  if (cardEntries.length !== referencedCardIds.size
    || cardEntries.some(([cardId]) => !referencedCardIds.has(cardId))) {
    throw new RangeError('cards must contain exactly the deck-referenced definitions');
  }
  validateDeck(input.decks.north, 'decks.north', input.cards);
  validateDeck(input.decks.south, 'decks.south', input.cards);
  requireCardId(input.authority.revisionId, 'authority.revisionId');
  if (!/^sha256:[0-9a-f]{64}$/.test(input.authority.contentHash)) {
    throw new RangeError('authority.contentHash must be a SHA-256 identity');
  }

  const body = deepFreeze({
    authority: { ...input.authority },
    cards: Object.fromEntries(cardEntries.map(([cardId, card]) => [cardId,
      card.cardType === 'avatar'
        ? {
          attack: card.attack,
          cardType: 'avatar' as const,
          defense: card.defense,
          drawSpell: card.drawSpell,
          ...(card.earthSitePlayCreatesAdjacentRubble === true
            ? { earthSitePlayCreatesAdjacentRubble: true as const }
            : {}),
          life: card.life,
          ...(card.replaceAdjacentRubbleWithTopAtlasSite === true
            ? { replaceAdjacentRubbleWithTopAtlasSite: true as const }
            : {}),
          ...(card.tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn === true
            ? { tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true as const }
            : {}),
        }
        : card.cardType === 'artifact'
          ? {
            cardType: 'artifact' as const,
            ...(card.atEndOfEachTurnSiteControllerLosesLife !== undefined
              ? {
                atEndOfEachTurnSiteControllerLosesLife:
                  card.atEndOfEachTurnSiteControllerLosesLife,
                }
              : card.atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn !== undefined
                ? {
                  atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn:
                    card.atStartOfSiteControllerTurnLoseLifeAndGainManaThisTurn,
                }
              : card.bearerControllerChoosesExtraRandomOutcome === true
                ? { bearerControllerChoosesExtraRandomOutcome: true as const }
              : card.grantsBearerPower === 2
              ? { grantsBearerPower: 2 as const }
              : card.grantsBearerLethal === true
                ? { grantsBearerLethal: true as const }
              : card.nearbyMinionsMustAttackIfAble === true
                ? {
                  nearbyMinionsMustAttackIfAble: true as const,
                  ...(card.nearbyStrikesAgainstUnitsDealDoubleDamage === true
                    ? { nearbyStrikesAgainstUnitsDealDoubleDamage: true as const }
                    : {}),
                }
              : card.nearbyStrikesAgainstUnitsDealDoubleDamage === true
                ? { nearbyStrikesAgainstUnitsDealDoubleDamage: true as const }
                : card.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps === 3
                  ? {
                    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 3 as const,
                  }
                  : card
                    .tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
                      === true
                    ? {
                    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps:
                      true as const,
                    }
                    : {
                      tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 4 as const,
                  }),
            manaCost: card.manaCost,
            thresholds: { ...card.thresholds },
          }
          : card.cardType === 'site'
          ? {
            ...(card.airborneMinionsAtopMoveFreelyAway === true
              ? { airborneMinionsAtopMoveFreelyAway: true as const }
              : {}),
            ...(card.blocksGroundMinionEntryWhileMinionAtop === true
              ? { blocksGroundMinionEntryWhileMinionAtop: true as const }
              : {}),
            cardType: 'site' as const,
            ...(card.connectsBurrowedAllies === true ? { connectsBurrowedAllies: true } : {}),
            elements: [...card.elements],
            ...(card.flyToNearbyVoidOncePerTurnAtAirThreshold === 3
              ? { flyToNearbyVoidOncePerTurnAtAirThreshold: 3 as const }
              : {}),
            ...(card.genesisDiscardTopSpells === 2 ? { genesisDiscardTopSpells: 2 as const } : {}),
            ...(card.cannotBeMovedDestroyedOrModified === true
              ? { cannotBeMovedDestroyedOrModified: true as const }
              : {}),
            ...(card.genesisDrawSpellPerAdjacentSameCard === true
              ? { genesisDrawSpellPerAdjacentSameCard: true }
              : {}),
            ...(card.genesisEnemiesLoseStealth === true
              ? { genesisEnemiesLoseStealth: true as const }
              : {}),
            ...(card.genesisGainMana ? { genesisGainMana: card.genesisGainMana } : {}),
            ...(card.genesisGainManaIfOnlyControlledCopy === 1
              ? { genesisGainManaIfOnlyControlledCopy: 1 as const }
              : {}),
            ...(card.genesisHealNearbyAvatars === 3
              ? { genesisHealNearbyAvatars: 3 as const }
              : {}),
            ...(card.genesisImmobilizeNearbyUntilNextTurn === true
              ? { genesisImmobilizeNearbyUntilNextTurn: true as const }
              : {}),
            ...(card.genesisMayBottomNextSpell === true
              ? { genesisMayBottomNextSpell: true as const }
              : {}),
            ...(card.genesisPayOneManaToSummonToken
              ? { genesisPayOneManaToSummonToken: card.genesisPayOneManaToSummonToken }
              : {}),
            ...(card.genesisReorderNextSpells === 3
              ? { genesisReorderNextSpells: 3 as const }
              : {}),
            ...(card.isTower === true ? { isTower: true as const } : {}),
            ...(card.minionsHereGainVoidwalkUntilLeavingVoid === true
              ? { minionsHereGainVoidwalkUntilLeavingVoid: true as const }
              : {}),
            ...(card.ordinaryMinionManaDiscount === 1
              ? { ordinaryMinionManaDiscount: 1 as const }
              : {}),
            ...(card.preventsUnitsWithPowerAtLeastFromEntering !== undefined
              ? {
                preventsUnitsWithPowerAtLeastFromEntering:
                  card.preventsUnitsWithPowerAtLeastFromEntering,
                }
              : {}),
            ...(card.rangedUnitsHereRangeBonus === 1
              ? { rangedUnitsHereRangeBonus: 1 as const }
              : {}),
            ...(card.sacrificeToDestroyNearbySite === true
              ? { sacrificeToDestroyNearbySite: true as const }
              : {}),
            ...(card.uniqueOrLegendary === true
              ? { uniqueOrLegendary: true as const }
              : {}),
          }
          : card.cardType === 'aura'
            ? {
              ...(card.atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf === true
                ? {
                  atStartOfControllerTurnDestroyOccupiedSiteMinionsAndSelf: true as const,
                }
                : card.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep === 3
                  ? {
                    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep:
                      3 as const,
                  }
                  : card.atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent === 3
                    ? {
                      atEndOfEachTurnDamageEachUnitHereThenMoveToUnvisitedAdjacent: 3 as const,
                    }
                  : card.affectedSitesAreFlooded === true
                    ? { affectedSitesAreFlooded: true as const }
                    : card.affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold === true
                      ? {
                        affectedSitesAreNotWaterSitesAndProvideNoWaterThreshold: true as const,
                      }
                    : {
                      immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns:
                        true as const,
                    }),
              cardType: 'aura' as const,
              manaCost: card.manaCost,
              thresholds: { ...card.thresholds },
            }
          : card.cardType === 'magic'
            ? {
              cardType: 'magic' as const,
              ...(card.burrowAllMinionsAndArtifactsAtTargetLandSite === true
                ? { burrowAllMinionsAndArtifactsAtTargetLandSite: true as const }
                : card.discardSiteAsAdditionalCost === true
                  ? {
                    damageUnitsAboveAndBelowTargetSiteByManhattanDistance:
                      [...card.damageUnitsAboveAndBelowTargetSiteByManhattanDistance!] as [number, number, number, number, number],
                    destroyTargetSite: true as const,
                    discardSiteAsAdditionalCost: true as const,
                  }
                : card.destroyTargetSite === true
                  ? { destroyTargetSite: true as const }
                : card.destroyTargetArtifact === true
                  ? { destroyTargetArtifact: true as const }
                : card.destroyTargetAura === true
                  ? { destroyTargetAura: true as const }
                : card.burrowTargetMinionOrArtifact === true
                  ? { burrowTargetMinionOrArtifact: true as const }
                : card.submergeTargetMinion === true
                  ? { submergeTargetMinion: true as const }
                : card.damageChainNearbyUnits === true
                  ? { damageChainNearbyUnits: true as const }
                : card.damageEachAbovegroundMinion === 1
                  ? { damageEachAbovegroundMinion: 1 as const }
                : card.damageEachUnitAtLocationWithinTwoSteps !== undefined
                  ? { damageEachUnitAtLocationWithinTwoSteps: card.damageEachUnitAtLocationWithinTwoSteps }
                : card.damageRandomUnitAtLocation !== undefined
                  ? { damageRandomUnitAtLocation: card.damageRandomUnitAtLocation }
                : card.damageTargetUnit !== undefined
                  ? { damageTargetUnit: card.damageTargetUnit }
                  : card.disableTargetNearbyMinionUntilNextTurn === true
                    ? { disableTargetNearbyMinionUntilNextTurn: true as const }
                  : card.fightAllyWithAdjacentEnemy === true
                    ? { fightAllyWithAdjacentEnemy: true as const }
                  : card.gainControlOfTargetNearbyMinion === true
                    ? { gainControlOfTargetNearbyMinion: true as const }
                  : card.grantAirborneToAllyThisTurn === true
                    ? { grantAirborneToAllyThisTurn: true as const }
                  : card.grantChargeToAllyThisTurn === true
                    ? { grantChargeToAllyThisTurn: true as const }
                  : card.grantFirstStrikeToAllyThisTurn === true
                    ? { grantFirstStrikeToAllyThisTurn: true as const }
                  : card.grantLethalToAllyThisTurn === true
                    ? { grantLethalToAllyThisTurn: true as const }
                  : card.grantRangedToAllyThisTurn === true
                    ? { grantRangedToAllyThisTurn: true as const }
                  : card.grantPowerToAllyThisTurn === 2
                    ? { grantPowerToAllyThisTurn: 2 as const }
                  : card.killTargetMinion === true
                    ? { killTargetMinion: true as const }
                  : card.killTargetWoundedMinion === true
                    ? { killTargetWoundedMinion: true as const }
                  : card.leapAttackAlly === true
                    ? { leapAttackAlly: true as const }
                  : card.lureEnemyMinionOneStepCloser === true
                    ? { lureEnemyMinionOneStepCloser: true as const }
                  : card.healController !== undefined
                    ? { healController: card.healController }
                  : card.healTargetMinion !== undefined
                    ? { healTargetMinion: card.healTargetMinion }
                  : card.drawSites !== undefined
                    ? { drawSites: card.drawSites }
                  : card.drawSpells !== undefined
                    ? { drawSpells: card.drawSpells }
                  : card.millSites !== undefined
                    ? { millSites: card.millSites }
                  : card.millSpells !== undefined
                    ? { millSpells: card.millSpells }
                  : card.targetPlayerDrawsSites !== undefined
                    ? { targetPlayerDrawsSites: card.targetPlayerDrawsSites }
                  : card.targetPlayerDrawsSpells !== undefined
                    ? { targetPlayerDrawsSpells: card.targetPlayerDrawsSpells }
                    : card.returnTargetMinionToOwnerHand === true
                      ? { returnTargetMinionToOwnerHand: true as const }
                    : card.returnTargetArtifactToOwnerHand === true
                      ? { returnTargetArtifactToOwnerHand: true as const }
                    : card.returnTargetAuraToOwnerHand === true
                      ? { returnTargetAuraToOwnerHand: true as const }
                    : card.returnTargetSiteToOwnerHand === true
                      ? { returnTargetSiteToOwnerHand: true as const }
                    : card.returnTargetSiteFromOwnCemetery === true
                      ? { returnTargetSiteFromOwnCemetery: true as const }
                    : card.returnMinionFromOwnCemetery === true
                      ? { returnMinionFromOwnCemetery: true as const }
                    : card.returnTargetArtifactFromOwnCemetery === true
                      ? { returnTargetArtifactFromOwnCemetery: true as const }
                    : card.returnTargetAuraFromOwnCemetery === true
                      ? { returnTargetAuraFromOwnCemetery: true as const }
                    : card.returnTargetMagicFromOwnCemetery === true
                      ? { returnTargetMagicFromOwnCemetery: true as const }
                      : card.summonRandomMinionFromAnyCemetery === true
                        ? { summonRandomMinionFromAnyCemetery: true as const }
                        : card.summonTokenToEachControlledSiteBorderingEnemySite !== undefined
                        ? {
                          summonTokenToEachControlledSiteBorderingEnemySite:
                            card.summonTokenToEachControlledSiteBorderingEnemySite,
                        }
                        : card.targetPlayerDiscardsCards !== undefined
                          ? { targetPlayerDiscardsCards: card.targetPlayerDiscardsCards }
                        : card.targetPlayerGainsLife !== undefined
                          ? { targetPlayerGainsLife: card.targetPlayerGainsLife }
                        : card.targetPlayerLosesLife !== undefined
                          ? { targetPlayerLosesLife: card.targetPlayerLosesLife }
                        : card.grantStealthToTargetMinion === true
                          ? { grantStealthToTargetMinion: true as const }
                        : card.grantWardToTargetMinion === true
                          ? { grantWardToTargetMinion: true as const }
                        : card.tapTargetMinion === true
                          ? { tapTargetMinion: true as const }
                        : card.untapTargetMinion === true
                          ? { untapTargetMinion: true as const }
                        : card.teleportNearbyAllyThenDrawCard === true
                          ? { teleportNearbyAllyThenDrawCard: true as const }
                          : { teleportAllyToTargetSite: true as const }),
              manaCost: card.manaCost,
              ...(card.targetNearby === true ? { targetNearby: true } : {}),
              thresholds: { ...card.thresholds },
              ...(card.untapTargetMinionAfterDamage === true
                ? { untapTargetMinionAfterDamage: true as const }
                : {}),
              ...(card.discardCardAsAdditionalCost === true
                ? { discardCardAsAdditionalCost: true as const }
                : {}),
            }
          : {
            ...(card.airborne === true ? { airborne: true } : {}),
            ...(card.atStartOfControllerTurnControllerGainsLife !== undefined
              ? {
                atStartOfControllerTurnControllerGainsLife:
                  card.atStartOfControllerTurnControllerGainsLife,
              }
              : {}),
            ...(card.atStartOfControllerTurnControllerGainsMana !== undefined
              ? {
                atStartOfControllerTurnControllerGainsMana:
                  card.atStartOfControllerTurnControllerGainsMana,
              }
              : {}),
            ...(card.atStartOfControllerTurnControllerLosesLife !== undefined
              ? {
                atStartOfControllerTurnControllerLosesLife:
                  card.atStartOfControllerTurnControllerLosesLife,
              }
              : {}),
            ...(card.atEndOfControllerTurnDamageEachOtherUnitHere !== undefined
              ? {
                atEndOfControllerTurnDamageEachOtherUnitHere:
                  card.atEndOfControllerTurnDamageEachOtherUnitHere,
              }
              : {}),
            ...(card.atEndOfControllerTurnControllerGainsLife !== undefined
              ? {
                atEndOfControllerTurnControllerGainsLife:
                  card.atEndOfControllerTurnControllerGainsLife,
              }
              : {}),
            ...(card.atEndOfControllerTurnControllerLosesLife !== undefined
              ? {
                atEndOfControllerTurnControllerLosesLife:
                  card.atEndOfControllerTurnControllerLosesLife,
              }
              : {}),
            ...(card.atStartOfControllerTurnDamageEachOtherUnitHere !== undefined
              ? {
                atStartOfControllerTurnDamageEachOtherUnitHere:
                  card.atStartOfControllerTurnDamageEachOtherUnitHere,
              }
              : {}),
            ...(card.atStartOfControllerTurnDrawSites !== undefined
              ? { atStartOfControllerTurnDrawSites: card.atStartOfControllerTurnDrawSites }
              : {}),
            ...(card.atStartOfControllerTurnDrawSpells !== undefined
              ? { atStartOfControllerTurnDrawSpells: card.atStartOfControllerTurnDrawSpells }
              : {}),
            ...(card.atStartOfControllerTurnMillSites !== undefined
              ? { atStartOfControllerTurnMillSites: card.atStartOfControllerTurnMillSites }
              : {}),
            ...(card.atStartOfControllerTurnMillSpells !== undefined
              ? { atStartOfControllerTurnMillSpells: card.atStartOfControllerTurnMillSpells }
              : {}),
            ...(card.atStartOfControllerTurnLureNearbyEnemyMinion === true
              ? { atStartOfControllerTurnLureNearbyEnemyMinion: true as const }
              : {}),
            ...(card.atStartOfControllerTurnTeleportToRandomSiteOrVoid === true
              ? { atStartOfControllerTurnTeleportToRandomSiteOrVoid: true as const }
              : {}),
            attack: card.attack,
            ...(card.burrowing === true ? { burrowing: true } : {}),
            cardType: 'minion' as const,
            ...(card.cannotAttackSites === true ? { cannotAttackSites: true } : {}),
            ...(card.charge === true ? { charge: true } : {}),
            ...(card.mustAttackAUnitIfAble === true ? { mustAttackAUnitIfAble: true as const } : {}),
            ...(card.enemiesMustAttackThisIfAble === true
              ? { enemiesMustAttackThisIfAble: true as const }
              : {}),
            ...(card.cannotDefend === true ? { cannotDefend: true } : {}),
            ...(card.cannotDefendOrIntercept === true ? { cannotDefendOrIntercept: true } : {}),
            ...(card.connectsTopBottom === true ? { connectsTopBottom: true } : {}),
            ...(card.deathriteDamageEachUnitHere
              ? { deathriteDamageEachUnitHere: card.deathriteDamageEachUnitHere }
              : {}),
            ...(card.deathriteDrawSite === true ? { deathriteDrawSite: true } : {}),
            ...(card.deathriteDrawSpells === true ? { deathriteDrawSpells: true } : {}),
            ...(card.deathriteHeal ? { deathriteHeal: card.deathriteHeal } : {}),
            ...(card.deathriteMillSites === true ? { deathriteMillSites: true } : {}),
            ...(card.deathriteMillSpells === true ? { deathriteMillSpells: true } : {}),
            ...(card.deathriteLoseLifePerNearbySiteControlled === 1
              ? { deathriteLoseLifePerNearbySiteControlled: 1 as const }
              : {}),
            defense: card.defense,
            ...(card.discardSpellToDamageRandomOtherUnitHere !== undefined
              ? {
                discardSpellToDamageRandomOtherUnitHere:
                  card.discardSpellToDamageRandomOtherUnitHere,
              }
              : {}),
            ...(card.diesAtEndOfControllerTurn === true
              ? { diesAtEndOfControllerTurn: true as const }
              : {}),
            ...(card.discardRandomCardInsteadOfMana === true
              ? { discardRandomCardInsteadOfMana: true as const }
              : {}),
            ...(card.genesisDamageEachOtherUnitHere === 1
              ? { genesisDamageEachOtherUnitHere: 1 as const }
              : {}),
            ...(card.genesisDisableSelfUntilDamaged === true
              ? { genesisDisableSelfUntilDamaged: true as const }
              : {}),
            ...(card.genesisMayDamageTargetAdjacentUnit === 2
              ? { genesisMayDamageTargetAdjacentUnit: 2 as const }
              : {}),
            ...(card.genesisStrikeEachEnemyHere === true
              ? { genesisStrikeEachEnemyHere: true as const }
              : {}),
            ...(card.genesisDrawSpells !== undefined
              ? { genesisDrawSpells: card.genesisDrawSpells }
              : {}),
            ...(card.genesisDrawSite === true ? { genesisDrawSite: true } : {}),
            ...(card.genesisHealController === 2 ? { genesisHealController: 2 as const } : {}),
            ...(card.genesisLoseControllerLife === 2 ? { genesisLoseControllerLife: 2 as const } : {}),
            ...(card.gainsPowerRangedAndSpellcasterAtopTower === 2
              ? { gainsPowerRangedAndSpellcasterAtopTower: 2 as const }
              : {}),
            ...(card.gainsStealthAtEndOfTurn === true ? { gainsStealthAtEndOfTurn: true } : {}),
            ...(card.gainsStealthAtEndOfTurnIfNoEnemiesNearby === true
              ? { gainsStealthAtEndOfTurnIfNoEnemiesNearby: true }
              : {}),
            ...(card.immobile === true ? { immobile: true } : {}),
            ...(card.lanceCount !== undefined ? { lanceCount: card.lanceCount } : {}),
            ...(card.lethal === true ? { lethal: true } : {}),
            manaCost: card.manaCost,
            ...(card.mayRangedStrikeOnceDuringBasicMovement === true
              ? { mayRangedStrikeOnceDuringBasicMovement: true as const }
              : {}),
            ...(card.mayStepAfterRangedStrike === true
              ? { mayStepAfterRangedStrike: true as const }
              : {}),
            ...(card.mortal === true ? { mortal: true as const } : {}),
            ...(card.movementBonus ? { movementBonus: card.movementBonus } : {}),
            ...(card.movesOnlyForward === true ? { movesOnlyForward: true } : {}),
            ...(card.movesOnlySideways === true ? { movesOnlySideways: true } : {}),
            ...(card.mustBeCastBurrowed === true ? { mustBeCastBurrowed: true } : {}),
            ...(card.mustBeCastSubmerged === true ? { mustBeCastSubmerged: true } : {}),
            ...(card.mustBeCastToWaterSite === true ? { mustBeCastToWaterSite: true } : {}),
            ...(card.nearbyEnemiesPermanentlyLoseStealth === true
              ? { nearbyEnemiesPermanentlyLoseStealth: true as const }
              : {}),
            ...(card.ordinary === true ? { ordinary: true as const } : {}),
            ...(card.occupiesSquareArea === 2 ? { occupiesSquareArea: 2 as const } : {}),
            ...(card.otherControlledMortalsPowerBonus === 1
              ? { otherControlledMortalsPowerBonus: 1 as const }
              : {}),
            ...(card.otherNearbyAlliesPowerBonus === 1
              ? { otherNearbyAlliesPowerBonus: 1 as const }
              : {}),
            ...(card.preventsDamageFromUnitsWithPowerAtLeast !== undefined
              ? {
                preventsDamageFromUnitsWithPowerAtLeast:
                  card.preventsDamageFromUnitsWithPowerAtLeast,
              }
              : {}),
            ...(card.provides ? { provides: card.provides } : {}),
            ...(card.ranged === true ? { ranged: true } : {}),
            ...(card.sacrificeMinionAtSummoningLocationForManaDiscount === 2
              ? { sacrificeMinionAtSummoningLocationForManaDiscount: 2 as const }
              : {}),
            ...(card.tapToShootProjectileDamage !== undefined
              ? { tapToShootProjectileDamage: card.tapToShootProjectileDamage }
              : {}),
            ...(card.shootsDragProjectile === true ? { shootsDragProjectile: true } : {}),
            ...(card.siteProvidesNoThreshold === true
              ? { siteProvidesNoThreshold: true as const }
              : {}),
            ...(card.spellcaster === true ? { spellcaster: true } : {}),
            ...(card.stealth === true ? { stealth: true } : {}),
            ...(card.strikesFirstWhileAttacking === true ? { strikesFirstWhileAttacking: true } : {}),
            ...(card.strikesFirstWhileDefending === true ? { strikesFirstWhileDefending: true } : {}),
            ...(card.submerge === true ? { submerge: true } : {}),
            ...(card.summonToAnySite === true ? { summonToAnySite: true } : {}),
            ...(card.mustBeCastToOuterColumn === true ? { mustBeCastToOuterColumn: true } : {}),
            ...(card.tapToDamageEachUnitAtAdjacentLocation === 2
              ? { tapToDamageEachUnitAtAdjacentLocation: 2 as const }
              : {}),
            ...(card.tapForMana ? { tapForMana: card.tapForMana } : {}),
            ...(card.takesLessDamage === 1 ? { takesLessDamage: 1 as const } : {}),
            thresholds: { ...card.thresholds },
            ...(card.token === true ? { token: true as const } : {}),
            ...(card.doesNotUntapDuringControllersStartPhase === true
              ? { doesNotUntapDuringControllersStartPhase: true as const }
              : {}),
            ...(card.untapsAtEndOfControllerTurn === true
              ? { untapsAtEndOfControllerTurn: true as const }
              : {}),
            ...(card.voidwalk === true ? { voidwalk: true } : {}),
            ...(card.waterbound === true ? { waterbound: true } : {}),
            ...(card.ward === true ? { ward: true } : {}),
          },
    ])),
    decks: {
      north: {
        atlas: [...input.decks.north.atlas],
        avatar: input.decks.north.avatar,
        spellbook: [...input.decks.north.spellbook],
      },
      south: {
        atlas: [...input.decks.south.atlas],
        avatar: input.decks.south.avatar,
        spellbook: [...input.decks.south.spellbook],
      },
    },
    engineVersion: 'sorcery-core-v1' as const,
    firstSeat: input.firstSeat,
    schemaVersion: 1 as const,
    seed: input.seed,
  });
  return deepFreeze({ ...body, manifestId: identityHash(asJson(body)) });
}

export function assertCanonicalGameManifest(manifest: GameManifest): void {
  const rebuilt = createGameManifest({
    authority: manifest.authority,
    cards: manifest.cards,
    decks: manifest.decks,
    firstSeat: manifest.firstSeat,
    seed: manifest.seed,
  });
  if (canonicalJson(asJson(rebuilt)) !== canonicalJson(asJson(manifest))) {
    throw new RangeError('game manifest is not canonical');
  }
}

export function createGameSession(manifest: GameManifest): GameSession {
  assertCanonicalGameManifest(manifest);
  return createRustGameSession(manifest);
}

export function hashGameState(state: GameState): StateHash {
  return identityHash(asJson(state));
}

export function observeGame(state: GameState, viewer: GameSeat): GameObservation {
  return rustObserveGame(state, viewer);
}

export function legalGameActions(state: GameState, seat: GameSeat): readonly GameLegalAction[] {
  return rustLegalGameActions(state, seat);
}

export function stepGame(session: GameSession, request: GameActionRequest): GameStepResult {
  return rustStepGame(session, request);
}

export function replayGame(manifest: GameManifest, actionIds: readonly string[]): GameSession {
  return rustReplayGame(manifest, actionIds);
}

export function verifyGameReplay(expected: GameSession): boolean {
  return rustVerifyGameReplay(expected);
}

